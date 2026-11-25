//! ffprobe JSON -> MediaSpec plus what the decoder needs.

use std::ffi::OsStr;

use adit_core::{AudioInfo, FLICKS_PER_SECOND, MediaSpec, Rational, Time, VideoInfo};
use serde_json::Value;

use crate::MediaError;
use crate::proc::{command, input, run};

pub(crate) struct Info {
    pub spec: MediaSpec,
    pub still: bool,
    /// Video stream start time in seconds (frame 0's pts).
    pub start: f64,
    /// Video stream duration when the container states it (more precise than the container's).
    pub video_duration: Option<Time>,
}

pub(crate) fn check_file(path: &str) -> Result<(), MediaError> {
    match std::fs::metadata(path) {
        Ok(m) if m.is_file() => Ok(()),
        Ok(_) => Err(MediaError::Unsupported(format!("not a file: {path}"))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(MediaError::NotFound(path.to_string()))
        }
        Err(e) => Err(MediaError::Io(format!("{path}: {e}"))),
    }
}

pub(crate) fn probe(ffprobe: &OsStr, path: &str) -> Result<Info, MediaError> {
    check_file(path)?;
    let mut c = command(ffprobe);
    c.args([
        "-v",
        "error",
        "-show_format",
        "-show_streams",
        "-of",
        "json",
    ])
    .arg(input(path));
    let (out, _) = run(c)?;
    let v: Value = serde_json::from_slice(&out)
        .map_err(|e| MediaError::Ffmpeg(format!("unreadable ffprobe output: {e}")))?;
    parse(&v, path)
}

fn parse(v: &Value, path: &str) -> Result<Info, MediaError> {
    let none = Vec::new();
    let streams = v["streams"].as_array().unwrap_or(&none);
    let format = &v["format"];
    let format_name = format["format_name"].as_str().unwrap_or("");
    // Single images come through the image demuxers (`png_pipe`, `jpeg_pipe`, ..., `image2`).
    let still = format_name.ends_with("_pipe") || format_name == "image2";
    // Cover art in audio files is a video stream flagged attached_pic.
    let vs = streams
        .iter()
        .find(|s| s["codec_type"] == "video" && s["disposition"]["attached_pic"] != 1);
    let audio_stream = streams.iter().find(|s| s["codec_type"] == "audio");
    if vs.is_none() && audio_stream.is_none() {
        return Err(MediaError::Unsupported(format!(
            "no audio or video stream: {path}"
        )));
    }

    let video = match vs {
        None => None,
        Some(s) => {
            let dim = |k: &str| {
                s[k].as_u64()
                    .and_then(|x| u32::try_from(x).ok())
                    .filter(|x| *x > 0)
                    .ok_or_else(|| MediaError::Unsupported(format!("video {k} unknown: {path}")))
            };
            let fps = if still {
                Rational { num: 25, den: 1 }
            } else {
                rational(&s["avg_frame_rate"])
                    .or_else(|| rational(&s["r_frame_rate"]))
                    .ok_or_else(|| MediaError::Unsupported(format!("frame rate unknown: {path}")))?
            };
            Some(VideoInfo {
                width: dim("width")?,
                height: dim("height")?,
                fps,
            })
        }
    };
    let audio = match audio_stream {
        None => None,
        Some(s) => {
            let sample_rate = s["sample_rate"]
                .as_str()
                .and_then(|x| x.parse::<u32>().ok())
                .filter(|x| *x > 0);
            let channels = s["channels"].as_u64().and_then(|x| u16::try_from(x).ok());
            match (sample_rate, channels) {
                (Some(sample_rate), Some(channels)) if channels > 0 => Some(AudioInfo {
                    sample_rate,
                    channels,
                }),
                _ => {
                    return Err(MediaError::Unsupported(format!(
                        "audio format unknown: {path}"
                    )));
                }
            }
        }
    };

    let video_duration = vs.and_then(|s| flicks(&s["duration"]));
    // Matroska states its end time as the duration (12 s for 2 s of video starting at 10 s);
    // the other demuxers already subtract the start time (H-7).
    let offset = if format_name.starts_with("matroska") {
        secs(&format["start_time"]).unwrap_or(0.0).max(0.0)
    } else {
        0.0
    };
    let container = secs(&format["duration"]).and_then(|d| to_flicks(d - offset));
    let duration = if still {
        None
    } else {
        container
            .or(video_duration)
            .or_else(|| audio_stream.and_then(|s| flicks(&s["duration"])))
    };
    let name = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string());
    Ok(Info {
        spec: MediaSpec {
            name,
            path: path.to_string(),
            duration,
            video,
            audio,
        },
        still,
        start: vs.and_then(|s| secs(&s["start_time"])).unwrap_or(0.0),
        video_duration,
    })
}

/// "30000/1001" -> Rational; `None` for "0/0" and junk.
fn rational(v: &Value) -> Option<Rational> {
    let (n, d) = v.as_str()?.split_once('/')?;
    let r = Rational {
        num: n.parse().ok()?,
        den: d.parse().ok()?,
    };
    (r.num > 0 && r.den > 0).then_some(r)
}

/// Finite seconds string -> f64.
fn secs(v: &Value) -> Option<f64> {
    v.as_str()?.parse().ok().filter(|s: &f64| s.is_finite())
}

fn to_flicks(s: f64) -> Option<Time> {
    (s >= 0.0).then(|| (s * FLICKS_PER_SECOND as f64).round() as Time)
}

/// Seconds string -> flicks.
fn flicks(v: &Value) -> Option<Time> {
    secs(v).and_then(to_flicks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_streams_and_skips_cover_art() {
        let v: Value = serde_json::from_str(
            r#"{"streams":[
                {"codec_type":"video","width":600,"height":600,"avg_frame_rate":"0/0",
                 "r_frame_rate":"90000/1","disposition":{"attached_pic":1}},
                {"codec_type":"audio","sample_rate":"44100","channels":2,"duration":"3.5"}],
               "format":{"format_name":"mp3","duration":"3.500000"}}"#,
        )
        .unwrap();
        let i = parse(&v, "dir/a.mp3").unwrap();
        assert_eq!(i.spec.name, "a.mp3");
        assert_eq!(i.spec.video, None);
        assert_eq!(i.spec.duration, Some(7 * FLICKS_PER_SECOND / 2));
        assert_eq!(i.spec.audio.unwrap().channels, 2);
        assert_eq!(rational(&Value::from("0/0")), None);
        assert_eq!(
            rational(&Value::from("30000/1001")),
            Some(Rational {
                num: 30000,
                den: 1001
            })
        );
    }
}
