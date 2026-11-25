//! Program and Source monitor pictures, rendered on worker threads (U-APP-3).

use super::worker::Worker;
use crate::UiState;
use adit_core::{Id, Project, Time};
use adit_mcp::Session;
use eframe::egui;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

type Picture = Result<egui::ColorImage, String>;

pub struct Monitors {
    program: Worker<(Arc<Project>, Id, Time), Picture>,
    source: Worker<(String, Time), Picture>,
    program_key: Option<(String, u64, Id, Time)>,
    source_key: Option<(String, Time)>,
    program_new: Option<Picture>,
    source_new: Option<Picture>,
    pub program_tex: Option<egui::TextureHandle>,
    pub source_tex: Option<egui::TextureHandle>,
    pub program_err: Option<String>,
    pub source_err: Option<String>,
}

fn picture(f: &adit_media::Frame) -> Picture {
    let (w, h) = (f.width as usize, f.height as usize);
    if f.rgba.len() != w * h * 4 {
        return Err(format!("bad frame size {w}x{h}"));
    }
    Ok(egui::ColorImage::from_rgba_unmultiplied([w, h], &f.rgba))
}

/// A panic in a lower crate becomes a message instead of killing the worker.
fn guarded(f: impl FnOnce() -> Picture) -> Picture {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|_| Err("render panicked".into()))
}

impl Monitors {
    pub fn new(ctx: &egui::Context, session: &Session) -> Self {
        let (s1, s2) = (session.clone(), session.clone());
        Monitors {
            // shortcut: full-resolution CPU round trip per preview frame, add a preview scale when playback is too slow.
            program: Worker::new(ctx, move |(p, seq, t): (Arc<Project>, Id, Time)| {
                guarded(|| s1.render(&p, seq, t).and_then(|f| picture(&f)))
            }),
            source: Worker::new(ctx, move |(path, t): (String, Time)| {
                guarded(|| s2.source_frame(&path, t).and_then(|f| picture(&f)))
            }),
            program_key: None,
            source_key: None,
            program_new: None,
            source_new: None,
            program_tex: None,
            source_tex: None,
            program_err: None,
            source_err: None,
        }
    }

    /// Asks for new pictures when what they show changed; collects finished ones.
    pub fn update(
        &mut self,
        pid: &str,
        rev: u64,
        project: &Arc<Project>,
        ui: &UiState,
        status: &mut String,
    ) {
        let key = (pid.to_string(), rev, ui.sequence_id, ui.playhead);
        if self.program_key.as_ref() != Some(&key) {
            self.program
                .request((project.clone(), ui.sequence_id, ui.playhead));
            self.program_key = Some(key);
        }
        let src = ui.source.as_ref().and_then(|s| {
            let m = project.media.iter().find(|m| m.id == s.media_id)?;
            m.spec.video.as_ref()?;
            Some((m.spec.path.clone(), s.playhead))
        });
        if src.is_some() && self.source_key != src {
            if let Some(q) = src.clone() {
                self.source.request(q);
            }
        } else if src.is_none() {
            self.source_tex = None;
            self.source_err = None;
        }
        self.source_key = src;
        if let Some(p) = self.program.latest() {
            note(&mut self.program_err, &p, status);
            self.program_new = Some(p);
        }
        if let Some(p) = self.source.latest() {
            note(&mut self.source_err, &p, status);
            self.source_new = Some(p);
        }
    }

    /// A Program render is in flight.
    pub fn busy(&self) -> bool {
        self.program.busy()
    }

    pub fn upload(&mut self, ctx: &egui::Context) {
        for (new, tex, name) in [
            (&mut self.program_new, &mut self.program_tex, "program"),
            (&mut self.source_new, &mut self.source_tex, "source"),
        ] {
            if let Some(Ok(img)) = new.take() {
                match tex {
                    Some(t) => t.set(img, egui::TextureOptions::LINEAR),
                    None => *tex = Some(ctx.load_texture(name, img, egui::TextureOptions::LINEAR)),
                }
            }
        }
    }
}

/// Remembers a render error and shows a new one in the status bar once (U-APP-9).
fn note(err: &mut Option<String>, p: &Picture, status: &mut String) {
    match p {
        Ok(_) => *err = None,
        Err(e) => {
            if err.as_ref() != Some(e) {
                *status = format!("render failed: {e}");
            }
            *err = Some(e.clone());
        }
    }
}

/// Draws a picture letterboxed into the available space; returns its rect and a response.
pub fn show(
    ui: &mut egui::Ui,
    tex: Option<&egui::TextureHandle>,
    aspect: f32,
    err: Option<&str>,
) -> (egui::Rect, egui::Response) {
    let avail = ui.available_size() - egui::vec2(0.0, 30.0);
    let avail = avail.max(egui::vec2(16.0, 16.0));
    let (outer, resp) = ui.allocate_exact_size(avail, egui::Sense::click_and_drag());
    let size = if avail.x / avail.y > aspect {
        egui::vec2(avail.y * aspect, avail.y)
    } else {
        egui::vec2(avail.x, avail.x / aspect)
    };
    let rect = egui::Rect::from_center_size(outer.center(), size);
    let p = ui.painter_at(outer);
    p.rect_filled(outer, 0.0, egui::Color32::from_gray(12));
    p.rect_filled(rect, 0.0, egui::Color32::BLACK);
    if let Some(t) = tex {
        p.image(
            t.id(),
            rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
    if let Some(e) = err {
        p.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            e,
            egui::FontId::proportional(13.0),
            egui::Color32::from_rgb(230, 120, 120),
        );
    }
    (rect, resp)
}
