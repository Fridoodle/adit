//! `UiBridge` for the `ui_*` MCP tools (U-APP-8, U-FIX-9). Called on MCP threads; it only touches
//! shared state and never waits for the UI thread except in `screenshot`.

use crate::{Tool, UiState};
use adit_core::{Id, MAX_TIME, Time, frame_to_time, time_to_frame};
use adit_mcp::{Image, Session, UiBridge};
use eframe::egui;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{SyncSender, sync_channel};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

/// A validated `ui_set`, times already rounded to frames; applied by the window next frame.
#[derive(Default, Clone)]
pub struct UiSet {
    pub project_id: Option<String>,
    /// Switches sequence: playhead 0, selection and marks cleared (U-FIX-8).
    pub sequence_id: Option<Id>,
    pub playhead: Option<Time>,
    pub selection: Option<Vec<Id>>,
    pub mark_in: Option<Option<Time>>,
    pub mark_out: Option<Option<Time>>,
    pub tool: Option<Tool>,
}

impl UiSet {
    /// Everything except the project switch.
    pub fn apply(&self, ui: &mut UiState) {
        if let Some(seq) = self.sequence_id {
            switch_sequence(ui, seq);
        }
        if let Some(p) = self.playhead {
            ui.playhead = p;
        }
        if let Some(m) = self.mark_in {
            ui.mark_in = m;
        }
        if let Some(m) = self.mark_out {
            ui.mark_out = m;
        }
        if let Some(s) = &self.selection {
            ui.selection.clone_from(s);
        }
        if let Some(t) = self.tool {
            ui.tool = t;
        }
    }
}

/// U-FIX-8: a fresh view of another sequence.
pub fn switch_sequence(ui: &mut UiState, seq: Id) {
    if ui.sequence_id != seq {
        ui.sequence_id = seq;
        ui.playhead = 0;
        ui.selection.clear();
        ui.mark_in = None;
        ui.mark_out = None;
        ui.playing = false;
        ui.shuttle = 0;
    }
}

/// What `ui_get` answers: the window's tabs as last drawn, plus `ui_set`s accepted since.
#[derive(Default)]
struct Mirror {
    active: Option<String>,
    tabs: Vec<(String, UiState)>,
    pending: Vec<UiSet>,
}

#[derive(Default)]
pub struct Shared {
    mirror: Mutex<Mirror>,
    shots: Mutex<Vec<SyncSender<Arc<egui::ColorImage>>>>,
    touched: AtomicBool,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Shared {
    /// The window's state after a frame. Skipped while `ui_set`s wait to be applied: the mirror
    /// already shows them and the next frame writes it again.
    pub fn set_mirror(&self, active: Option<String>, tabs: Vec<(String, UiState)>) {
        let mut m = lock(&self.mirror);
        if m.pending.is_empty() {
            m.active = active;
            m.tabs = tabs;
        }
    }
    pub fn take_pending(&self) -> Vec<UiSet> {
        std::mem::take(&mut lock(&self.mirror).pending)
    }
    pub fn take_shots(&self) -> Vec<SyncSender<Arc<egui::ColorImage>>> {
        std::mem::take(&mut *lock(&self.shots))
    }
    pub fn take_touched(&self) -> bool {
        self.touched.swap(false, Ordering::Relaxed)
    }
}

pub struct Bridge {
    pub shared: Arc<Shared>,
    pub session: Session,
    pub ctx: egui::Context,
}

const TOOLS: &[(Tool, &str)] = &[
    (Tool::Select, "select"),
    (Tool::TrackSelect, "track_select"),
    (Tool::Ripple, "ripple"),
    (Tool::Roll, "roll"),
    (Tool::RateStretch, "rate_stretch"),
    (Tool::Razor, "razor"),
    (Tool::Slip, "slip"),
    (Tool::Slide, "slide"),
    (Tool::Pen, "pen"),
    (Tool::Hand, "hand"),
    (Tool::Zoom, "zoom"),
    (Tool::Type, "type"),
    (Tool::Wand, "wand"),
];

pub fn tool_name(t: Tool) -> &'static str {
    TOOLS
        .iter()
        .find(|(x, _)| *x == t)
        .map_or("select", |(_, n)| n)
}

pub fn state_json(pid: &str, u: &UiState) -> Value {
    json!({
        "project_id": pid,
        "sequence_id": u.sequence_id,
        "playhead": u.playhead,
        "playhead_s": u.playhead as f64 / adit_core::FLICKS_PER_SECOND as f64,
        "selection": u.selection,
        "mark_in": u.mark_in,
        "mark_out": u.mark_out,
        "tool": tool_name(u.tool),
        "playing": u.playing,
    })
}

/// A time field: absent → `None`, `null` → `Some(None)` when `nullable`.
fn time(v: &Value, key: &str, nullable: bool) -> Result<Option<Option<Time>>, String> {
    match v.get(key) {
        None => Ok(None),
        Some(Value::Null) if nullable => Ok(Some(None)),
        Some(x) => match x.as_i64() {
            Some(t) if (0..=MAX_TIME).contains(&t) => Ok(Some(Some(t))),
            _ => Err(format!("{key} must be a time in 0..={MAX_TIME}, got {x}")),
        },
    }
}

impl Bridge {
    /// Validates a `ui_set` against the mirror and returns it with times rounded to frames.
    fn check(&self, m: &Mirror, v: &Value) -> Result<(String, UiSet, UiState), String> {
        if !v.is_object() {
            return Err("ui_set takes an object".into());
        }
        let pid = match v.get("project_id") {
            None | Some(Value::Null) => {
                m.active.clone().ok_or("no project is open in the window")?
            }
            Some(Value::String(p)) => p.clone(),
            Some(x) => return Err(format!("project_id must be a string, got {x}")),
        };
        let (p, _) = self
            .session
            .snapshot(&pid)
            .ok_or_else(|| format!("unknown project_id {pid:?}"))?;
        let mut s = UiSet {
            project_id: Some(pid.clone()),
            ..Default::default()
        };
        // A project opened a moment ago has no tab yet: it starts from the defaults.
        let mut ui = m
            .tabs
            .iter()
            .find(|(id, _)| *id == pid)
            .map(|(_, u)| u.clone())
            .unwrap_or_else(|| UiState::new(p.sequences.first().map_or(0, |q| q.id)));
        if let Some(x) = v.get("sequence_id") {
            let id = x
                .as_u64()
                .filter(|&id| p.sequence(id).is_some())
                .ok_or_else(|| format!("sequence {x} not found in project {pid}"))?;
            s.sequence_id = Some(id);
        }
        let fps = p
            .sequence(s.sequence_id.unwrap_or(ui.sequence_id))
            .map(|q| q.fps);
        let round = |t: Time| fps.map_or(t, |f| frame_to_time(time_to_frame(t, f), f));
        let opt = |o: Option<Option<Time>>| o.map(|x| x.map(round));
        s.playhead = time(v, "playhead", false)?.flatten().map(round);
        s.mark_in = opt(time(v, "mark_in", true)?);
        s.mark_out = opt(time(v, "mark_out", true)?);
        if let Some(t) = v.get("tool") {
            let name = t.as_str().unwrap_or_default();
            let tool = TOOLS
                .iter()
                .find(|(_, n)| n.eq_ignore_ascii_case(name))
                .ok_or_else(|| format!("unknown tool {t}"))?;
            s.tool = Some(tool.0);
        }
        if let Some(sel) = v.get("selection") {
            let ids: Vec<Id> = sel
                .as_array()
                .and_then(|a| a.iter().map(Value::as_u64).collect())
                .ok_or_else(|| format!("selection must be an array of clip ids, got {sel}"))?;
            if let Some(bad) = ids.iter().find(|&&id| p.clip(id).is_none()) {
                return Err(format!("clip {bad} not found in project {pid}"));
            }
            s.selection = Some(ids);
        }
        s.apply(&mut ui);
        if let (Some(a), Some(b)) = (ui.mark_in, ui.mark_out)
            && a >= b
        {
            return Err(format!("mark_in ({a}) must be before mark_out ({b})"));
        }
        Ok((pid, s, ui))
    }
}

impl UiBridge for Bridge {
    fn screenshot(&self) -> Result<Image, String> {
        self.shared.touched.store(true, Ordering::Relaxed);
        let (tx, rx) = sync_channel(1);
        lock(&self.shared.shots).push(tx);
        self.ctx.request_repaint();
        let img = rx
            .recv_timeout(Duration::from_secs(10))
            .map_err(|_| "the window did not paint a frame within 10 s".to_string())?;
        png(&img)
    }

    fn get(&self) -> Result<Value, String> {
        self.shared.touched.store(true, Ordering::Relaxed);
        let m = lock(&self.shared.mirror);
        let pid = m
            .active
            .as_ref()
            .ok_or("no project is open in the window")?;
        let (_, u) = m
            .tabs
            .iter()
            .find(|(id, _)| id == pid)
            .ok_or("no project is open in the window")?;
        Ok(state_json(pid, u))
    }

    fn set(&self, v: &Value) -> Result<(), String> {
        self.shared.touched.store(true, Ordering::Relaxed);
        let mut m = lock(&self.shared.mirror);
        let (pid, s, ui) = self.check(&m, v)?;
        match m.tabs.iter_mut().find(|(id, _)| *id == pid) {
            Some(t) => t.1 = ui,
            None => m.tabs.push((pid.clone(), ui)),
        }
        m.active = Some(pid);
        m.pending.push(s);
        drop(m);
        self.ctx.request_repaint();
        Ok(())
    }
}

fn png(img: &egui::ColorImage) -> Result<Image, String> {
    let [w, h] = img.size;
    let bytes: Vec<u8> = img
        .pixels
        .iter()
        .flat_map(|c| c.to_srgba_unmultiplied())
        .collect();
    let rgba = image::RgbaImage::from_raw(w as u32, h as u32, bytes)
        .ok_or("screenshot has the wrong number of pixels")?;
    let mut out = std::io::Cursor::new(Vec::new());
    rgba.write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| format!("png encoding failed: {e}"))?;
    Ok(Image {
        mime: "image/png".into(),
        bytes: out.into_inner(),
    })
}
