//! `UiBridge` for the `ui_*` MCP tools (U-APP-8). Called on MCP threads; it only touches shared
//! state and never waits for the UI thread except in `screenshot`.

use crate::Tool;
use adit_core::{Id, Time};
use adit_mcp::{Image, Session, UiBridge};
use eframe::egui;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{SyncSender, sync_channel};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

/// A validated `ui_set`, applied by the window at the start of its next frame.
#[derive(Default)]
pub struct UiSet {
    pub project_id: Option<String>,
    pub playhead: Option<Time>,
    pub selection: Option<Vec<Id>>,
    pub mark_in: Option<Option<Time>>,
    pub mark_out: Option<Option<Time>>,
    pub tool: Option<Tool>,
}

#[derive(Default)]
pub struct Shared {
    /// `ui_get` JSON of the active tab, rewritten every frame.
    mirror: Mutex<Option<Value>>,
    pending: Mutex<Vec<UiSet>>,
    shots: Mutex<Vec<SyncSender<Arc<egui::ColorImage>>>>,
    touched: AtomicBool,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Shared {
    pub fn set_mirror(&self, v: Option<Value>) {
        *lock(&self.mirror) = v;
    }
    pub fn take_pending(&self) -> Vec<UiSet> {
        std::mem::take(&mut *lock(&self.pending))
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

/// A time field: absent → `None`, `null` → `Some(None)` when `nullable`.
fn time(v: &Value, key: &str, nullable: bool) -> Result<Option<Option<Time>>, String> {
    match v.get(key) {
        None => Ok(None),
        Some(Value::Null) if nullable => Ok(Some(None)),
        Some(x) => match x.as_i64() {
            Some(t) if t >= 0 => Ok(Some(Some(t))),
            _ => Err(format!("{key} must be a time >= 0, got {x}")),
        },
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
        lock(&self.shared.mirror)
            .clone()
            .ok_or_else(|| "no project is open in the window".to_string())
    }

    fn set(&self, v: &Value) -> Result<(), String> {
        self.shared.touched.store(true, Ordering::Relaxed);
        if !v.is_object() {
            return Err("ui_set takes an object".into());
        }
        let mut s = UiSet::default();
        match v.get("project_id") {
            None | Some(Value::Null) => {}
            Some(Value::String(p)) => {
                if self.session.revision(p).is_none() {
                    return Err(format!("unknown project_id {p:?}"));
                }
                s.project_id = Some(p.clone());
            }
            Some(x) => return Err(format!("project_id must be a string, got {x}")),
        }
        s.playhead = time(v, "playhead", false)?.flatten();
        s.mark_in = time(v, "mark_in", true)?;
        s.mark_out = time(v, "mark_out", true)?;
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
            let pid = match &s.project_id {
                Some(p) => p.clone(),
                None => lock(&self.shared.mirror)
                    .as_ref()
                    .and_then(|m| m["project_id"].as_str().map(str::to_string))
                    .ok_or("no project is open in the window")?,
            };
            let (p, _) = self
                .session
                .snapshot(&pid)
                .ok_or_else(|| format!("unknown project_id {pid:?}"))?;
            if let Some(bad) = ids.iter().find(|&&id| p.clip(id).is_none()) {
                return Err(format!("clip {bad} not found in project {pid}"));
            }
            s.selection = Some(ids);
        }
        lock(&self.shared.pending).push(s);
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
