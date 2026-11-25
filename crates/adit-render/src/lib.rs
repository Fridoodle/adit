//! adit-render: turns a project + a time into pixels and samples. Same code for GUI preview,
//! headless screenshots and export. Contract: docs/SPEC.md Part 3. Signatures here are fixed;
//! bodies and private items are free.

use adit_core::{EffectDef, Id, Project, Time};
use adit_media::{ExportSettings, Frame, MediaBackend, MediaError};
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RenderError {
    #[error(transparent)]
    Media(#[from] MediaError),
    #[error("gpu: {0}")]
    Gpu(String),
    #[error("shader: {0}")]
    Shader(String),
    #[error("sequence {0} not found")]
    SequenceNotFound(Id),
    #[error("unknown effect: {0}")]
    UnknownEffect(String),
    #[error("cancelled")]
    Cancelled,
}

/// GPU compositor. Owns the device and compiled shaders; reuse one instance.
pub struct Renderer {
    _private: (),
}

impl Renderer {
    /// Works without a window or a real GPU (falls back to a software adapter).
    pub fn new() -> Result<Self, RenderError> {
        todo!()
    }

    /// One composited frame of the sequence at `time`, at sequence resolution (SPEC V-*).
    pub fn render_frame(
        &mut self,
        _project: &Project,
        _sequence_id: Id,
        _time: Time,
        _media: &dyn MediaBackend,
    ) -> Result<Frame, RenderError> {
        todo!()
    }
}

/// The effects shipped with adit (SPEC E-BUILTIN). `wgsl` holds their real source.
pub fn builtin_effects() -> Vec<EffectDef> {
    todo!()
}

/// Compiles the definition the way the renderer would. `Err` = message for the author (SPEC E-4).
pub fn validate_effect(_def: &EffectDef) -> Result<(), String> {
    todo!()
}

/// Sequence audio for `[start, start + duration)`: interleaved stereo f32 at the sequence sample
/// rate (SPEC A-*).
pub fn mix_audio(
    _project: &Project,
    _sequence_id: Id,
    _start: Time,
    _duration: Time,
    _media: &dyn MediaBackend,
) -> Result<Vec<f32>, RenderError> {
    todo!()
}

/// Renders the whole sequence to a file (SPEC X-*). `progress` gets (frames_done, frames_total).
/// Setting `cancel` stops it with `RenderError::Cancelled`.
pub fn export(
    _renderer: &mut Renderer,
    _project: &Project,
    _sequence_id: Id,
    _settings: &ExportSettings,
    _media: &dyn MediaBackend,
    _progress: &mut dyn FnMut(u64, u64),
    _cancel: &AtomicBool,
) -> Result<(), RenderError> {
    todo!()
}
