//! adit-media: everything that touches media files, through `ffmpeg` / `ffprobe` subprocesses.
//! Contract: docs/SPEC.md Part 2. Signatures here are fixed; bodies and private items are free.

use adit_core::{AudioInfo, FLICKS_PER_SECOND, MediaSpec, Rational, Time, VideoInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsString;
use std::sync::{Arc, Mutex, MutexGuard};

mod decode;
mod encode;
mod probe;
mod proc;

use proc::{bin, command, input, run};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MediaError {
    #[error("file not found: {0}")]
    NotFound(String),
    #[error("ffmpeg not available: {0}")]
    FfmpegMissing(String),
    #[error("ffmpeg failed: {0}")]
    Ffmpeg(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("io: {0}")]
    Io(String),
}

/// RGBA8, straight alpha, sRGB-encoded, rows top to bottom, `rgba.len() == width * height * 4`.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Where the renderer gets pictures and sound from.
pub trait MediaBackend: Send + Sync {
    fn probe(&self, path: &str) -> Result<MediaSpec, MediaError>;

    /// The frame shown at source time `time` (clamped into the media).
    fn video_frame(&self, path: &str, time: Time) -> Result<Arc<Frame>, MediaError>;

    /// Hint that `video_frame(path, time)` will be asked for soon (SPEC M-FRAME-5). Best effort.
    fn prefetch(&self, _path: &str, _time: Time) {}

    /// Interleaved stereo f32 at `sample_rate` for the source range `[start, start + duration)`,
    /// passed through the ffmpeg audio filter chain `filter` (empty = none). The length is
    /// whatever the filter chain yields (a tempo filter changes it).
    fn audio(
        &self,
        path: &str,
        start: Time,
        duration: Time,
        sample_rate: u32,
        filter: &str,
    ) -> Result<Vec<f32>, MediaError>;

    /// Silent stretches `[start, end)`. Backends without analysis return `Unsupported` (SPEC H-14).
    fn detect_silence(
        &self,
        _path: &str,
        _noise_db: f64,
        _min_duration: Time,
    ) -> Result<Vec<(Time, Time)>, MediaError> {
        Err(MediaError::Unsupported("silence detection".into()))
    }

    /// Times of scene cuts. Backends without analysis return `Unsupported` (SPEC H-14).
    fn detect_scenes(&self, _path: &str, _threshold: f64) -> Result<Vec<Time>, MediaError> {
        Err(MediaError::Unsupported("scene detection".into()))
    }
}

/// Accepts only a plain chain of allow-listed ffmpeg audio filters (SPEC H-6). `Err` names the
/// rejected part.
pub fn check_audio_filter(filter: &str) -> Result<(), String> {
    if filter.is_empty() {
        return Ok(());
    }
    if let Some(c) = filter.chars().find(|c| matches!(c, ';' | '[' | ']')) {
        return Err(format!(
            "audio filter must be a plain chain without {c:?}: {filter:?}"
        ));
    }
    // A naive split on ',' can only over-reject: ffmpeg's quoting/escaping merges parts, never
    // splits them further, and every part must start with an allowed name.
    for part in filter.split(',') {
        let name = part.split_once('=').map_or(part, |(n, _)| n);
        if !AUDIO_FILTERS.contains(&name) {
            return Err(format!("audio filter {name:?} is not allowed"));
        }
    }
    Ok(())
}

/// SPEC H-6 allow-list: filters that only transform samples (no files, sockets or plugins).
const AUDIO_FILTERS: &[&str] = &[
    "volume",
    "atempo",
    "areverse",
    "highpass",
    "lowpass",
    "bandpass",
    "bandreject",
    "equalizer",
    "bass",
    "treble",
    "acompressor",
    "alimiter",
    "agate",
    "aecho",
    "chorus",
    "flanger",
    "aphaser",
    "tremolo",
    "vibrato",
    "afade",
    "adelay",
    "loudnorm",
    "dynaudnorm",
    "compand",
    "anequalizer",
    "afftdn",
    "anlmdn",
    "deesser",
    "crystalizer",
    "stereowiden",
    "extrastereo",
    "pan",
    "aresample",
    "asetrate",
    "silenceremove",
    "apad",
    "atrim",
    "asetpts",
    "acontrast",
    "asoftclip",
    "speechnorm",
    "channelmap",
    "aformat",
];

/// Real files through ffmpeg subprocesses.
pub struct Ffmpeg {
    ffmpeg: OsString,
    ffprobe: OsString,
    // shortcut: probe results and frames are cached by path for the backend's lifetime; a file
    // replaced on disk keeps its old data until a new `Ffmpeg`. Key on mtime if that matters.
    infos: Mutex<HashMap<String, Arc<probe::Info>>>,
    frames: decode::Frames,
}

impl Ffmpeg {
    /// Finds `ffmpeg` and `ffprobe` (env `ADIT_FFMPEG` / `ADIT_FFPROBE`, else PATH) and checks they run.
    pub fn new() -> Result<Self, MediaError> {
        let ffmpeg = bin("ADIT_FFMPEG", "ffmpeg");
        let ffprobe = bin("ADIT_FFPROBE", "ffprobe");
        for b in [&ffmpeg, &ffprobe] {
            let mut c = command(b);
            c.arg("-version");
            run(c).map_err(|e| match e {
                MediaError::FfmpegMissing(m) => MediaError::FfmpegMissing(m),
                e => MediaError::FfmpegMissing(format!("{} -version: {e}", b.to_string_lossy())),
            })?;
        }
        Ok(Ffmpeg {
            ffmpeg,
            ffprobe,
            infos: Mutex::new(HashMap::new()),
            frames: decode::Frames::new(),
        })
    }

    fn info(&self, path: &str) -> Result<Arc<probe::Info>, MediaError> {
        if let Some(i) = lock(&self.infos).get(path) {
            return Ok(i.clone());
        }
        self.reprobe(path)
    }

    fn reprobe(&self, path: &str) -> Result<Arc<probe::Info>, MediaError> {
        let i = Arc::new(probe::probe(&self.ffprobe, path)?);
        lock(&self.infos).insert(path.to_string(), i.clone());
        Ok(i)
    }

    /// `ffmpeg -i path <args> -f null -` with info-level logging; returns stderr.
    fn analyse(&self, path: &str, args: &[&str]) -> Result<String, MediaError> {
        let mut c = command(&self.ffmpeg);
        c.args(["-hide_banner", "-nostdin", "-nostats", "-v", "info", "-i"])
            .arg(input(path))
            .args(args)
            .args(["-f", "null", "-"]);
        Ok(run(c)?.1)
    }

    /// Silent stretches `[start, end)`, via ffmpeg `silencedetect`.
    pub fn detect_silence(
        &self,
        path: &str,
        noise_db: f64,
        min_duration: Time,
    ) -> Result<Vec<(Time, Time)>, MediaError> {
        if !noise_db.is_finite() || min_duration <= 0 {
            return Err(MediaError::Unsupported(format!(
                "silence detection needs a finite noise level and min_duration > 0, got {noise_db} dB, {min_duration}"
            )));
        }
        let info = self.info(path)?;
        if info.spec.audio.is_none() {
            return Err(MediaError::Unsupported(format!("no audio stream: {path}")));
        }
        let af = format!("silencedetect=n={noise_db}dB:d={}", seconds(min_duration));
        let log = self.analyse(path, &["-map", "0:a:0", "-af", &af])?;
        Ok(parse_silence(&log, info.spec.duration))
    }

    /// Times of scene cuts, via ffmpeg scene score > `threshold` (0..1).
    pub fn detect_scenes(&self, path: &str, threshold: f64) -> Result<Vec<Time>, MediaError> {
        if !threshold.is_finite() {
            return Err(MediaError::Unsupported(format!(
                "scene threshold {threshold}"
            )));
        }
        let info = self.info(path)?;
        if info.spec.video.is_none() || info.still {
            return Err(MediaError::Unsupported(format!("no video stream: {path}")));
        }
        let vf = format!("select='gt(scene,{threshold})',showinfo");
        let log = self.analyse(path, &["-map", "0:v:0", "-vf", &vf])?;
        Ok(parse_scenes(&log))
    }
}

/// First number after `key` in `line`.
fn number_after(line: &str, key: &str) -> Option<f64> {
    let rest = line[line.find(key)? + key.len()..].trim_start();
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e')))
        .unwrap_or(rest.len());
    rest[..end].parse().ok().filter(|x: &f64| x.is_finite())
}

fn secs_to_flicks(s: f64) -> Time {
    (s.max(0.0) * FLICKS_PER_SECOND as f64).round() as Time
}

fn parse_silence(log: &str, duration: Option<Time>) -> Vec<(Time, Time)> {
    let mut out = Vec::new();
    let mut open = None;
    for line in log.lines() {
        if let Some(s) = number_after(line, "silence_start:") {
            open = Some(secs_to_flicks(s));
        } else if let Some(e) = number_after(line, "silence_end:") {
            out.push((open.take().unwrap_or(0), secs_to_flicks(e)));
        }
    }
    // Silence running to the end of the file has no silence_end line.
    if let (Some(s), Some(d)) = (open, duration)
        && d > s
    {
        out.push((s, d));
    }
    out
}

fn parse_scenes(log: &str) -> Vec<Time> {
    log.lines()
        .filter(|l| l.contains("showinfo") && l.contains(" n:"))
        .filter_map(|l| number_after(l, "pts_time:"))
        .map(secs_to_flicks)
        .collect()
}

impl MediaBackend for Ffmpeg {
    fn probe(&self, path: &str) -> Result<MediaSpec, MediaError> {
        Ok(self.reprobe(path)?.spec.clone())
    }

    fn video_frame(&self, path: &str, time: Time) -> Result<Arc<Frame>, MediaError> {
        let info = self.info(path)?;
        let v = info
            .spec
            .video
            .ok_or_else(|| MediaError::Unsupported(format!("no video stream: {path}")))?;
        let n = if info.still {
            0
        } else {
            time_to_frame(time.max(0), v.fps)
        };
        self.frames.get(&self.ffmpeg, path, &info, n)
    }

    fn prefetch(&self, path: &str, time: Time) {
        // M-FRAME-5: unreadable files, audio-only files and stills need nothing ahead.
        // shortcut: only missing files skip the probe; an existing unreadable file is reprobed on
        // every call, cache the failure if offline-but-present media ever shows up in playback.
        if !std::path::Path::new(path).exists() {
            return;
        }
        let Ok(info) = self.info(path) else { return };
        if let Some(v) = info.spec.video {
            let n = time_to_frame(time.max(0), v.fps);
            self.frames.prefetch(&self.ffmpeg, path, &info, n);
        }
    }

    fn detect_silence(
        &self,
        path: &str,
        noise_db: f64,
        min_duration: Time,
    ) -> Result<Vec<(Time, Time)>, MediaError> {
        Ffmpeg::detect_silence(self, path, noise_db, min_duration)
    }

    fn detect_scenes(&self, path: &str, threshold: f64) -> Result<Vec<Time>, MediaError> {
        Ffmpeg::detect_scenes(self, path, threshold)
    }

    fn audio(
        &self,
        path: &str,
        start: Time,
        duration: Time,
        sample_rate: u32,
        filter: &str,
    ) -> Result<Vec<f32>, MediaError> {
        check_audio_filter(filter).map_err(MediaError::Ffmpeg)?;
        let info = self.info(path)?;
        let Some(a) = info.spec.audio else {
            return Ok(Vec::new());
        };
        if sample_rate == 0 {
            return Err(MediaError::Unsupported("sample_rate must be > 0".into()));
        }
        // Nothing exists before 0: a range starting earlier just yields less.
        let (start, duration) = if start < 0 {
            (0, duration.saturating_add(start))
        } else {
            (start, duration)
        };
        if duration <= 0 {
            return Ok(Vec::new());
        }
        let mut chain = Vec::new();
        // `-ac 2` would upmix mono at -3 dB; duplicate the channel at full level instead.
        if a.channels == 1 {
            chain.push("pan=stereo|c0=c0|c1=c0");
        }
        if !filter.is_empty() {
            chain.push(filter);
        }
        let mut c = command(&self.ffmpeg);
        c.args(["-v", "error", "-nostdin", "-ss", &seconds(start)])
            .args(["-t", &seconds(duration), "-i"])
            .arg(input(path))
            .args(["-map", "0:a:0"]);
        if !chain.is_empty() {
            c.args(["-af", &chain.join(",")]);
        }
        c.args(["-ac", "2", "-ar", &sample_rate.to_string()]).args([
            "-c:a",
            "pcm_f32le",
            "-f",
            "f32le",
            "-",
        ]);
        let (out, _) = run(c)?;
        let mut v: Vec<f32> = out
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect();
        v.truncate(v.len() / 2 * 2);
        Ok(v)
    }
}

/// Procedural source for tests and demos; ignores `path`. Formulas: SPEC M-PAT.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pattern {
    pub width: u32,
    pub height: u32,
    pub fps: Rational,
    pub duration: Option<Time>,
}

impl Pattern {
    fn check_fps(&self) -> Result<(), MediaError> {
        if self.fps.num == 0 || self.fps.den == 0 {
            return Err(MediaError::Unsupported(format!(
                "pattern fps {:?}",
                self.fps
            )));
        }
        Ok(())
    }
}

impl MediaBackend for Pattern {
    fn probe(&self, path: &str) -> Result<MediaSpec, MediaError> {
        self.check_fps()?;
        Ok(MediaSpec {
            name: path.to_string(),
            path: path.to_string(),
            duration: self.duration,
            video: Some(VideoInfo {
                width: self.width,
                height: self.height,
                fps: self.fps,
            }),
            audio: Some(AudioInfo {
                sample_rate: 48000,
                channels: 2,
            }),
        })
    }

    fn video_frame(&self, _path: &str, time: Time) -> Result<Arc<Frame>, MediaError> {
        self.check_fps()?;
        let n = (time_to_frame(time.max(0), self.fps) % 256) as u8;
        let (w, h) = (self.width as usize, self.height as usize);
        let mut rgba = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
                rgba.extend_from_slice(&[(x % 256) as u8, (y % 256) as u8, n, 255]);
            }
        }
        Ok(Arc::new(Frame {
            width: self.width,
            height: self.height,
            rgba,
        }))
    }

    fn audio(
        &self,
        _path: &str,
        _start: Time,
        duration: Time,
        sample_rate: u32,
        _filter: &str,
    ) -> Result<Vec<f32>, MediaError> {
        let s = i128::from(FLICKS_PER_SECOND);
        // round(duration * sr / s), half away from zero; negative durations give nothing.
        let frames = (2 * i128::from(duration.max(0)) * i128::from(sample_rate) + s) / (2 * s);
        let frames = usize::try_from(frames)
            .map_err(|_| MediaError::Unsupported(format!("audio range too long: {duration}")))?;
        Ok([0.5f32, 0.25].repeat(frames))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoCodec {
    H264,
    H265,
    Prores,
    Gif,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioCodec {
    Aac,
    Pcm,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportSettings {
    /// Output file. The container follows the extension.
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub fps: Rational,
    pub sample_rate: u32,
    pub video: VideoCodec,
    pub audio: AudioCodec,
    /// Constant rate factor for H264/H265 (lower = better). `None` = 18.
    #[serde(default)]
    pub crf: Option<u8>,
}

/// One ffmpeg encode: raw RGBA frames in, a finished file out.
pub struct Encoder {
    inner: encode::Inner,
}

impl Encoder {
    /// `audio`: the whole soundtrack, interleaved stereo f32 at `settings.sample_rate`.
    pub fn start(settings: &ExportSettings, audio: Option<&[f32]>) -> Result<Self, MediaError> {
        Ok(Encoder {
            inner: encode::start(settings, audio)?,
        })
    }

    /// `rgba.len()` must be `width * height * 4`.
    pub fn write_frame(&mut self, rgba: &[u8]) -> Result<(), MediaError> {
        self.inner.write_frame(rgba)
    }

    /// Waits for ffmpeg; `Err` if it failed. The file is complete when this returns `Ok`.
    pub fn finish(self) -> Result<(), MediaError> {
        self.inner.finish()
    }
}

/// A poisoned lock only means another caller panicked; the data is still usable.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// SPEC Q4 `time_to_frame`: the largest `n` with `floor(n * s * den / num) <= t`. Local copy so
/// this crate does not depend on adit-core's functions. `fps` must be non-zero.
fn time_to_frame(t: Time, fps: Rational) -> i64 {
    let s = i128::from(FLICKS_PER_SECOND);
    ((i128::from(t) + 1) * i128::from(fps.num) - 1).div_euclid(s * i128::from(fps.den)) as i64
}

/// Flicks (>= 0) as an exact-to-the-nanosecond seconds string for ffmpeg options.
fn seconds(t: Time) -> String {
    let s = FLICKS_PER_SECOND;
    let t = t.max(0);
    let nanos = i128::from(t % s) * 1_000_000_000 / i128::from(s);
    format!("{}.{nanos:09}", t / s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_to_frame_matches_q4() {
        let ntsc = Rational {
            num: 30000,
            den: 1001,
        };
        assert_eq!(time_to_frame(23_543_520, ntsc), 1);
        assert_eq!(time_to_frame(23_543_519, ntsc), 0);
        assert_eq!(time_to_frame(706_305_600, ntsc), 30);
        assert_eq!(time_to_frame(-1, ntsc), -1);
        let r24 = Rational { num: 24, den: 1 };
        assert_eq!(time_to_frame(29_400_000, r24), 1);
        assert_eq!(time_to_frame(29_399_999, r24), 0);
    }

    #[test]
    fn seconds_and_log_parsing() {
        assert_eq!(seconds(FLICKS_PER_SECOND * 3 / 2), "1.500000000");
        assert_eq!(seconds(1), "0.000000001");
        let log = "[silencedetect @ 0x1] silence_start: 1.00002\n\
                   [silencedetect @ 0x1] silence_end: 2 | silence_duration: 0.99998\n\
                   [silencedetect @ 0x1] silence_start: 2.5\n";
        let r = parse_silence(log, Some(3 * FLICKS_PER_SECOND));
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].1, 2 * FLICKS_PER_SECOND);
        assert_eq!(r[1], (5 * FLICKS_PER_SECOND / 2, 3 * FLICKS_PER_SECOND));
        let log = "[Parsed_showinfo_1 @ 0x2] config in time_base: 1/15360\n\
                   [Parsed_showinfo_1 @ 0x2] n:   0 pts:  15360 pts_time:1       duration:512\n";
        assert_eq!(parse_scenes(log), vec![FLICKS_PER_SECOND]);
    }
}
