//! Compositing (SPEC V-1..V-9): tracks, clip layers, effects, transitions.

use crate::builtins::{self, BLEND, MIX, TRANSFORM};
use crate::effects;
use crate::gpu::CtxVals;
use crate::{RenderError, Renderer};
use adit_core::*;
use adit_media::MediaBackend;
use wgpu::{CommandEncoder, Texture};

pub(crate) fn secs(t: Time) -> f64 {
    t as f64 / FLICKS_PER_SECOND as f64
}

/// V-9 window of `tr` on `track`: (start, left clip, right clip). `None` if a named clip is
/// missing or neither side is set.
pub(crate) fn window<'a>(
    track: &'a Track,
    tr: &Transition,
) -> Option<(Time, Option<&'a Clip>, Option<&'a Clip>)> {
    let find = |id: Option<Id>| match id {
        Some(id) => track.clips.iter().find(|c| c.id == id).map(Some),
        None => Some(None),
    };
    let (left, right) = (find(tr.left)?, find(tr.right)?);
    let start = match (left, right) {
        (Some(l), Some(_)) => l.end() - tr.duration / 2,
        (Some(l), None) => l.end() - tr.duration,
        (None, Some(r)) => r.start,
        (None, None) => return None,
    };
    Some((start, left, right))
}

fn blend_index(b: BlendMode) -> f32 {
    match b {
        BlendMode::Normal => 0.0,
        BlendMode::Add => 1.0,
        BlendMode::Multiply => 2.0,
        BlendMode::Screen => 3.0,
        BlendMode::Overlay => 4.0,
        BlendMode::Darken => 5.0,
        BlendMode::Lighten => 6.0,
        BlendMode::Difference => 7.0,
    }
}

impl Renderer {
    /// E-3: project definitions first, then built-ins; the kind must match.
    fn lookup(
        &self,
        project: &Project,
        name: &str,
        kind: EffectKind,
    ) -> Result<EffectDef, RenderError> {
        project
            .effects
            .iter()
            .chain(&self.builtins)
            .find(|d| d.name == name && d.kind == kind)
            .cloned()
            .ok_or_else(|| RenderError::UnknownEffect(name.to_string()))
    }

    /// The compiled source of `def`, checked with naga once per distinct source (E-4).
    fn effect_source(&mut self, def: &EffectDef) -> Result<String, RenderError> {
        let src = effects::assemble(def);
        if !self.checked.contains(&src) {
            effects::validate(def)
                .map_err(|m| RenderError::Shader(format!("{}: {m}", def.name)))?;
            self.checked.insert(src.clone());
        }
        Ok(src)
    }

    /// A renderer-internal pass (trusted source, not run through `validate`).
    pub(crate) fn internal(
        &mut self,
        enc: &mut CommandEncoder,
        spec: &builtins::Spec,
        inputs: [&Texture; 2],
        params: &[f32],
        out: (u32, u32),
    ) -> Result<Texture, RenderError> {
        let src = effects::assemble(&builtins::internal(spec));
        let ctx = [out.0 as f32, out.1 as f32, 0.0, 0.0, 0.0, 0.0];
        self.gpu.pass(enc, &src, inputs, ctx, params, out)
    }

    fn blend(
        &mut self,
        enc: &mut CommandEncoder,
        canvas: &Texture,
        layer: &Texture,
        mode: BlendMode,
    ) -> Result<Texture, RenderError> {
        let size = (canvas.width(), canvas.height());
        self.internal(enc, &BLEND, [canvas, layer], &[blend_index(mode)], size)
    }

    /// The sequence composited at `time` onto a canvas cleared to `base` (V-1, V-2).
    pub(crate) fn render_seq(
        &mut self,
        enc: &mut CommandEncoder,
        project: &Project,
        seq: &Sequence,
        time: Time,
        media: &dyn MediaBackend,
        base: [f64; 4],
    ) -> Result<Texture, RenderError> {
        let size = (seq.width, seq.height);
        let mut canvas = self.gpu.fill(enc, seq.width, seq.height, base)?;
        let frame = time_to_frame(time, seq.fps) as f32;
        let tracks = seq
            .tracks
            .iter()
            .filter(|t| t.kind == TrackKind::Video && !t.hidden);
        for track in tracks {
            let active = track.transitions.iter().find_map(|tr| {
                let w = window(track, tr)?;
                (w.0 <= time && time < w.0 + tr.duration).then_some((tr, w))
            });
            if let Some((tr, (start, left, right))) = active {
                let def = self.lookup(project, &tr.effect, EffectKind::Transition)?;
                let a = self.side(enc, project, seq, left, time, media)?;
                let b = self.side(enc, project, seq, right, time, media)?;
                let params: Vec<f32> = def
                    .params
                    .iter()
                    .map(|d| tr.params.get(&d.name).copied().unwrap_or(d.default) as f32)
                    .collect();
                let local = time - start;
                let ctx: CtxVals = [
                    size.0 as f32,
                    size.1 as f32,
                    secs(local) as f32,
                    secs(tr.duration) as f32,
                    (local as f64 / tr.duration as f64) as f32,
                    frame,
                ];
                let src = self.effect_source(&def)?;
                let empty = self.gpu.empty.clone();
                let inputs = [a.as_ref().unwrap_or(&empty), b.as_ref().unwrap_or(&empty)];
                let layer = self.gpu.pass(enc, &src, inputs, ctx, &params, size)?;
                // E-6 shortcut: the clips' own blend modes are ignored inside the window.
                canvas = self.blend(enc, &canvas, &layer, BlendMode::Normal)?;
                continue;
            }
            let Some(clip) = track
                .clips
                .iter()
                .find(|c| c.start <= time && time < c.end())
            else {
                continue;
            };
            if !clip.enabled {
                continue;
            }
            if matches!(clip.source, ClipSource::Adjustment) {
                let defs = self.clip_effects(project, clip)?;
                let result = self.run_effects(enc, seq, clip, time, &defs, canvas.clone())?;
                let amount = param(clip, "opacity", time - clip.start).clamp(0.0, 1.0) as f32;
                canvas = self.internal(enc, &MIX, [&canvas, &result], &[amount], size)?;
            } else if let Some(layer) = self.layer(enc, project, seq, clip, time, media)? {
                canvas = self.blend(enc, &canvas, &layer, clip.blend)?;
            }
        }
        Ok(canvas)
    }

    /// One side of a transition: the clip's layer, or `None` (transparent) if absent/disabled.
    fn side(
        &mut self,
        enc: &mut CommandEncoder,
        project: &Project,
        seq: &Sequence,
        clip: Option<&Clip>,
        time: Time,
        media: &dyn MediaBackend,
    ) -> Result<Option<Texture>, RenderError> {
        match clip {
            Some(c) if c.enabled => self.layer(enc, project, seq, c, time, media),
            _ => Ok(None),
        }
    }

    fn clip_effects<'c>(
        &self,
        project: &Project,
        clip: &'c Clip,
    ) -> Result<Vec<(EffectDef, &'c EffectInstance)>, RenderError> {
        clip.effects
            .iter()
            .filter(|e| e.enabled)
            .map(|e| Ok((self.lookup(project, &e.effect, EffectKind::Filter)?, e)))
            .collect()
    }

    /// V-4: the enabled effects in order, each image to a same-sized image.
    fn run_effects(
        &mut self,
        enc: &mut CommandEncoder,
        seq: &Sequence,
        clip: &Clip,
        time: Time,
        defs: &[(EffectDef, &EffectInstance)],
        mut img: Texture,
    ) -> Result<Texture, RenderError> {
        let local = time - clip.start;
        let frame = time_to_frame(time, seq.fps) as f32;
        let empty = self.gpu.empty.clone();
        for (def, inst) in defs {
            let params: Vec<f32> = def
                .params
                .iter()
                .map(|d| {
                    inst.params
                        .get(&d.name)
                        .map_or(d.default, |p| p.eval(local)) as f32
                })
                .collect();
            let size = (img.width(), img.height());
            let ctx: CtxVals = [
                size.0 as f32,
                size.1 as f32,
                secs(local) as f32,
                secs(clip.duration) as f32,
                (local as f64 / clip.duration as f64) as f32,
                frame,
            ];
            let src = self.effect_source(def)?;
            img = self
                .gpu
                .pass(enc, &src, [&img, &empty], ctx, &params, size)?;
        }
        Ok(img)
    }

    /// The clip's canvas-sized layer at `time`: source, effects, crop, transform, opacity.
    /// `None` = nothing to draw (unreadable media, degenerate scale).
    fn layer(
        &mut self,
        enc: &mut CommandEncoder,
        project: &Project,
        seq: &Sequence,
        clip: &Clip,
        time: Time,
        media: &dyn MediaBackend,
    ) -> Result<Option<Texture>, RenderError> {
        // Resolved first so an unknown effect is an error even when the source is unreadable.
        let defs = self.clip_effects(project, clip)?;
        let img = match &clip.source {
            ClipSource::Media { media_id } => {
                let Some(m) = project.media.iter().find(|m| m.id == *media_id) else {
                    return Ok(None);
                };
                match media.video_frame(&m.spec.path, clip.source_time(time)) {
                    Ok(f)
                        if f.width > 0
                            && f.height > 0
                            && f.rgba.len() == f.width as usize * f.height as usize * 4 =>
                    {
                        self.gpu.upload(&f.rgba, f.width, f.height)?
                    }
                    _ => return Ok(None),
                }
            }
            ClipSource::Solid { color } => {
                self.gpu
                    .fill(enc, seq.width, seq.height, color.map(f64::from))?
            }
            ClipSource::Text { text } => {
                let rgba = self.fonts.render(text, seq.width, seq.height);
                self.gpu.upload(&rgba, seq.width, seq.height)?
            }
            ClipSource::Sequence { sequence_id } => {
                let nested = project
                    .sequence(*sequence_id)
                    .ok_or(RenderError::SequenceNotFound(*sequence_id))?;
                let t = clip.source_time(time);
                self.render_seq(enc, project, nested, t, media, [0.0; 4])?
            }
            // shortcut: an adjustment clip as a transition side draws nothing.
            ClipSource::Adjustment => return Ok(None),
        };
        let img = self.run_effects(enc, seq, clip, time, &defs, img)?;

        let local = time - clip.start;
        let p = |n| param(clip, n, local);
        let (w, h) = (f64::from(img.width()), f64::from(img.height()));
        let (sx, sy) = (p("scale_x"), p("scale_y"));
        let (sin, cos) = p("rotation").to_radians().sin_cos();
        if !(sx * sy).is_finite() || sx * sy == 0.0 || !sin.is_finite() {
            return Ok(None);
        }
        let (ax, ay) = (p("anchor_x"), p("anchor_y"));
        // Canvas pixel P maps to image pixel a + (R S)^-1 (P - anchor on canvas).
        let params = [
            cos / sx,
            sin / sx,
            -sin / sy,
            cos / sy,
            w / 2.0 + ax,
            h / 2.0 + ay,
            f64::from(seq.width) / 2.0 + p("position_x") + ax,
            f64::from(seq.height) / 2.0 + p("position_y") + ay,
            p("crop_left") * w,
            p("crop_top") * h,
            (1.0 - p("crop_right")) * w,
            (1.0 - p("crop_bottom")) * h,
            p("opacity").clamp(0.0, 1.0),
        ]
        .map(|v| v as f32);
        let empty = self.gpu.empty.clone();
        let size = (seq.width, seq.height);
        self.internal(enc, &TRANSFORM, [&img, &empty], &params, size)
            .map(Some)
    }
}

/// A built-in clip property at `local` (Q5); every name used here is in `CLIP_PARAMS`.
pub(crate) fn param(clip: &Clip, name: &str, local: Time) -> f64 {
    clip.param(name, local).unwrap_or(0.0)
}
