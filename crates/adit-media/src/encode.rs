//! Encoder internals: ffmpeg reading raw RGBA from stdin.

use std::io::Write;
use std::path::PathBuf;
use std::process::{ChildStdin, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::proc::{Proc, bin, command, input};
use crate::{AudioCodec, ExportSettings, MediaError, VideoCodec};

/// Removes the file on drop.
pub(crate) struct TempFile(PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

// Field order is drop order: close stdin, kill ffmpeg, then delete the temp file it had open.
pub(crate) struct Inner {
    stdin: Option<ChildStdin>,
    proc: Proc,
    _audio: Option<TempFile>,
    frame_len: usize,
    failed: Option<MediaError>,
}

fn unsupported(msg: &str) -> MediaError {
    MediaError::Unsupported(msg.to_string())
}

pub(crate) fn start(st: &ExportSettings, audio: Option<&[f32]>) -> Result<Inner, MediaError> {
    let (w, h) = (st.width, st.height);
    if w == 0 || h == 0 {
        return Err(unsupported("width and height must be > 0"));
    }
    if st.fps.num == 0 || st.fps.den == 0 {
        return Err(unsupported("fps must be > 0"));
    }
    let even = w % 2 == 0 && h % 2 == 0;
    match st.video {
        VideoCodec::H264 | VideoCodec::H265 if !even => {
            return Err(unsupported(
                "H264/H265 (yuv420p) need even width and height",
            ));
        }
        VideoCodec::Prores if w % 2 != 0 => {
            return Err(unsupported("ProRes (4:2:2) needs an even width"));
        }
        _ => {}
    }
    let frame_len = (w as usize)
        .checked_mul(h as usize)
        .and_then(|x| x.checked_mul(4))
        .ok_or_else(|| unsupported("frame size overflows"))?;
    let audio = audio.filter(|_| st.audio != AudioCodec::None);
    if audio.is_some() && st.sample_rate == 0 {
        return Err(unsupported("sample_rate must be > 0"));
    }
    if audio.is_some_and(|a| a.len() % 2 != 0) {
        return Err(unsupported(
            "audio must be interleaved stereo (even length)",
        ));
    }

    // shortcut: the soundtrack goes through a temp file (ffmpeg has one stdin, used for video);
    // switch to a named pipe if multi-GB soundtracks matter.
    let audio_file = match audio {
        None => None,
        Some(a) => Some(write_temp(a)?),
    };

    let crf = st.crf.unwrap_or(18).to_string();
    let rate = format!("{}/{}", st.fps.num, st.fps.den);
    let ffmpeg = bin("ADIT_FFMPEG", "ffmpeg");
    let mut c = command(&ffmpeg);
    c.args(["-v", "error", "-nostdin", "-y"])
        .args([
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            "-s",
            &format!("{w}x{h}"),
        ])
        .args(["-framerate", &rate, "-i", "-"]);
    if let Some(f) = &audio_file {
        c.args(["-f", "f32le", "-ar", &st.sample_rate.to_string()])
            .args(["-ch_layout", "stereo", "-i"])
            .arg(input(&f.0.to_string_lossy()))
            .args(["-map", "0:v", "-map", "1:a"]);
    }
    // RGB -> YUV with the BT.709 matrix, tagged as such, so players don't shift colours.
    let yuv = |c: &mut std::process::Command, pix: &str| {
        c.args([
            "-vf",
            &format!("scale=out_color_matrix=bt709:out_range=tv,format={pix}"),
        ])
        .args([
            "-colorspace",
            "bt709",
            "-color_primaries",
            "bt709",
            "-color_trc",
            "bt709",
        ]);
    };
    match st.video {
        VideoCodec::H264 => {
            c.args(["-c:v", "libx264", "-preset", "medium", "-crf", &crf]);
            yuv(&mut c, "yuv420p");
        }
        VideoCodec::H265 => {
            c.args([
                "-c:v",
                "libx265",
                "-crf",
                &crf,
                "-x265-params",
                "log-level=error",
            ]);
            yuv(&mut c, "yuv420p");
        }
        VideoCodec::Prores => {
            c.args(["-c:v", "prores_ks", "-profile:v", "3"]);
            yuv(&mut c, "yuv422p10le");
        }
        VideoCodec::Gif => {
            c.args(["-vf", "split[a][b];[a]palettegen[p];[b][p]paletteuse"]);
        }
    }
    if audio_file.is_some() {
        match st.audio {
            AudioCodec::Aac => c.args(["-c:a", "aac", "-b:a", "192k"]),
            AudioCodec::Pcm => c.args(["-c:a", "pcm_s16le"]),
            AudioCodec::None => unreachable!("filtered above"),
        };
    }
    c.arg(input(&st.path)).stdin(Stdio::piped());

    let mut proc = Proc::spawn(c)?;
    let stdin = proc.child.stdin.take();
    Ok(Inner {
        stdin,
        proc,
        _audio: audio_file,
        frame_len,
        failed: None,
    })
}

fn write_temp(a: &[f32]) -> Result<TempFile, MediaError> {
    static N: AtomicU64 = AtomicU64::new(0);
    let p = std::env::temp_dir().join(format!(
        "adit-audio-{}-{}.f32",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let f = TempFile(p);
    let bytes: Vec<u8> = a.iter().flat_map(|x| x.to_le_bytes()).collect();
    std::fs::write(&f.0, bytes).map_err(|e| MediaError::Io(format!("writing temp audio: {e}")))?;
    Ok(f)
}

impl Inner {
    pub fn write_frame(&mut self, rgba: &[u8]) -> Result<(), MediaError> {
        if rgba.len() != self.frame_len {
            return Err(MediaError::Io(format!(
                "frame is {} bytes, expected {}",
                rgba.len(),
                self.frame_len
            )));
        }
        if let Some(e) = &self.failed {
            return Err(e.clone());
        }
        let stdin = self
            .stdin
            .as_mut()
            .expect("stdin open until failure or finish");
        if let Err(e) = stdin.write_all(rgba) {
            // A broken pipe means ffmpeg quit; its exit status and stderr say why.
            self.stdin = None;
            let err = match self.proc.wait() {
                Err(x) => x,
                Ok(()) => MediaError::Io(format!("writing frame: {e}")),
            };
            self.failed = Some(err.clone());
            return Err(err);
        }
        Ok(())
    }

    pub fn finish(mut self) -> Result<(), MediaError> {
        if let Some(e) = self.failed.take() {
            return Err(e);
        }
        drop(self.stdin.take());
        self.proc.wait()
    }
}
