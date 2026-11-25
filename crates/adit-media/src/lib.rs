//! adit-media: everything that touches media files, through `ffmpeg` / `ffprobe` subprocesses.
//! Contract: docs/SPEC.md Part 2. Signatures here are fixed; bodies and private items are free.

use adit_core::{MediaSpec, Rational, Time};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
}

/// Real files through ffmpeg subprocesses.
pub struct Ffmpeg {
    _private: (),
}

impl Ffmpeg {
    /// Finds `ffmpeg` and `ffprobe` (env `ADIT_FFMPEG` / `ADIT_FFPROBE`, else PATH) and checks they run.
    pub fn new() -> Result<Self, MediaError> {
        todo!()
    }

    /// Silent stretches `[start, end)`, via ffmpeg `silencedetect`.
    pub fn detect_silence(
        &self,
        _path: &str,
        _noise_db: f64,
        _min_duration: Time,
    ) -> Result<Vec<(Time, Time)>, MediaError> {
        todo!()
    }

    /// Times of scene cuts, via ffmpeg scene score > `threshold` (0..1).
    pub fn detect_scenes(&self, _path: &str, _threshold: f64) -> Result<Vec<Time>, MediaError> {
        todo!()
    }
}

impl MediaBackend for Ffmpeg {
    fn probe(&self, _path: &str) -> Result<MediaSpec, MediaError> {
        todo!()
    }
    fn video_frame(&self, _path: &str, _time: Time) -> Result<Arc<Frame>, MediaError> {
        todo!()
    }
    fn audio(
        &self,
        _path: &str,
        _start: Time,
        _duration: Time,
        _sample_rate: u32,
        _filter: &str,
    ) -> Result<Vec<f32>, MediaError> {
        todo!()
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

impl MediaBackend for Pattern {
    fn probe(&self, _path: &str) -> Result<MediaSpec, MediaError> {
        todo!()
    }
    fn video_frame(&self, _path: &str, _time: Time) -> Result<Arc<Frame>, MediaError> {
        todo!()
    }
    fn audio(
        &self,
        _path: &str,
        _start: Time,
        _duration: Time,
        _sample_rate: u32,
        _filter: &str,
    ) -> Result<Vec<f32>, MediaError> {
        todo!()
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
    _private: (),
}

impl Encoder {
    /// `audio`: the whole soundtrack, interleaved stereo f32 at `settings.sample_rate`.
    pub fn start(_settings: &ExportSettings, _audio: Option<&[f32]>) -> Result<Self, MediaError> {
        todo!()
    }

    /// `rgba.len()` must be `width * height * 4`.
    pub fn write_frame(&mut self, _rgba: &[u8]) -> Result<(), MediaError> {
        todo!()
    }

    /// Waits for ffmpeg; `Err` if it failed. The file is complete when this returns `Ok`.
    pub fn finish(self) -> Result<(), MediaError> {
        todo!()
    }
}
