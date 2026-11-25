//! adit-render: turns a project + a time into pixels and samples. Same code for GUI preview,
//! headless screenshots and export. Contract: docs/SPEC.md Part 3. Signatures here are fixed;
//! bodies and private items are free.

use adit_core::{EffectDef, Id, Project, Sequence, TextSpec, Time};
use adit_media::{ExportSettings, Frame, MediaBackend, MediaError};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicBool;

mod audio;
mod builtins;
mod compose;
mod effects;
mod export;
mod gpu;
mod text;

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
    gpu: gpu::Gpu,
    builtins: Vec<EffectDef>,
    fonts: text::Fonts,
    /// Effect sources that passed `validate_effect`.
    checked: HashSet<String>,
    /// Sequences nested above the one being rendered (0 = top level); bounds recursion.
    nest: usize,
    /// Nested sequences already composited in the current frame, by (sequence, time, depth).
    nested: HashMap<(Id, Time, usize), wgpu::Texture>,
    /// Recently rasterised text layers by (spec, width, height), newest last.
    texts: Vec<(TextSpec, u32, u32, wgpu::Texture)>,
}

impl Renderer {
    /// Works without a window or a real GPU (falls back to a software adapter).
    pub fn new() -> Result<Self, RenderError> {
        Ok(Renderer {
            gpu: gpu::Gpu::new()?,
            builtins: builtins::builtin_effects(),
            fonts: Default::default(),
            checked: HashSet::new(),
            nest: 0,
            nested: HashMap::new(),
            texts: Vec::new(),
        })
    }

    /// One composited frame of the sequence at `time`, at sequence resolution (SPEC V-*).
    pub fn render_frame(
        &mut self,
        project: &Project,
        sequence_id: Id,
        time: Time,
        media: &dyn MediaBackend,
    ) -> Result<Frame, RenderError> {
        let seq = project
            .sequence(sequence_id)
            .ok_or(RenderError::SequenceNotFound(sequence_id))?;
        self.render_frame_scaled(project, sequence_id, time, media, (seq.width, seq.height))
    }

    /// `render_frame` bilinearly scaled to `out` (width, height), the scale export uses (X-1).
    pub fn render_frame_scaled(
        &mut self,
        project: &Project,
        sequence_id: Id,
        time: Time,
        media: &dyn MediaBackend,
        out: (u32, u32),
    ) -> Result<Frame, RenderError> {
        let seq = project
            .sequence(sequence_id)
            .ok_or(RenderError::SequenceNotFound(sequence_id))?;
        let rgba = self.render_sized(project, seq, time, media, out)?;
        Ok(Frame {
            width: out.0,
            height: out.1,
            rgba,
        })
    }

    /// RGBA8 rows of the frame, bilinearly scaled to `out` when that differs from the sequence.
    fn render_sized(
        &mut self,
        project: &Project,
        seq: &Sequence,
        time: Time,
        media: &dyn MediaBackend,
        out: (u32, u32),
    ) -> Result<Vec<u8>, RenderError> {
        let scope = self.gpu.scope();
        // A panic caught by a caller may have left it raised.
        self.nest = 0;
        self.nested.clear();
        self.gpu.recycle();
        let r = (|| {
            let mut enc = self.gpu.encoder();
            let mut tex =
                self.render_seq(&mut enc, project, seq, time, media, [0.0, 0.0, 0.0, 1.0])?;
            if out != (seq.width, seq.height) {
                let empty = self.gpu.empty.clone();
                tex = self.internal(&mut enc, &builtins::SCALE, [&tex, &empty], &[], out)?;
            }
            self.gpu.finish(enc, &tex)
        })();
        self.nested.clear();
        scope.end(r)
    }
}

/// The effects shipped with adit (SPEC E-BUILTIN). `wgsl` holds their real source.
pub fn builtin_effects() -> Vec<EffectDef> {
    builtins::builtin_effects()
}

/// Compiles the definition the way the renderer would. `Err` = message for the author (SPEC E-4).
pub fn validate_effect(def: &EffectDef) -> Result<(), String> {
    effects::validate(def)
}

/// Sequence audio for `[start, start + duration)`: interleaved stereo f32 at the sequence sample
/// rate (SPEC A-*).
pub fn mix_audio(
    project: &Project,
    sequence_id: Id,
    start: Time,
    duration: Time,
    media: &dyn MediaBackend,
) -> Result<Vec<f32>, RenderError> {
    audio::mix_audio(project, sequence_id, start, duration, media)
}

/// Renders the whole sequence to a file (SPEC X-*). `progress` gets (frames_done, frames_total).
/// Setting `cancel` stops it with `RenderError::Cancelled`.
pub fn export(
    renderer: &mut Renderer,
    project: &Project,
    sequence_id: Id,
    settings: &ExportSettings,
    media: &dyn MediaBackend,
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<(), RenderError> {
    export::export(
        renderer,
        project,
        sequence_id,
        settings,
        media,
        progress,
        cancel,
    )
}
