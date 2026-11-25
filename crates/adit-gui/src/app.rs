//! The egui window (GUI.md U-APP-*). Public items and signatures here are fixed.

mod bridge;
mod monitors;
mod panels;
mod playback;
mod timeline;
mod worker;

use crate::{Action, KeyChord, Op, Options, UiState};
use adit_core::{Command, FLICKS_PER_SECOND, Id, Project, Rational, Time, time_to_frame};
use adit_mcp::{ProjectInfo, Session};
use eframe::egui;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// One open project.
struct Tab {
    id: String,
    name: String,
    path: Option<String>,
    dirty: bool,
    project: Arc<Project>,
    revision: u64,
    ui: UiState,
}

enum Dialog {
    Speed { percent: f64, ripple: bool },
    Gain { db: f64 },
    Wand { text: String },
    ConfirmClose,
}

/// The egui application (GUI.md U-APP-*).
pub struct App {
    session: Session,
    options: Options,
    keymap: Vec<(KeyChord, Action)>,
    tabs: Vec<Tab>,
    active: usize,
    status: String,
    dialog: Option<Dialog>,
    shared: Arc<bridge::Shared>,
    monitors: monitors::Monitors,
    playback: playback::Playback,
    /// Timeline gesture in progress: what was under the pointer at press.
    press: Option<crate::Hit>,
    /// Program-monitor rectangle drawn with the wand, `[x, y, w, h]` in 0..1.
    wand_rect: Option<[f64; 4]>,
    wand_press: Option<egui::Pos2>,
    requests: Vec<Value>,
    polled: Option<Instant>,
    export_job: Option<String>,
    /// `effect_list` of the active project and the (project, revision) it was read at.
    effects: Vec<Value>,
    effects_at: Option<(String, u64)>,
    /// Effect Controls values being dragged, committed on release.
    fx_drag: HashMap<String, f64>,
    /// Audio filter text being typed.
    fx_text: Option<(Id, String)>,
    show_effect_controls: bool,
    show_effects: bool,
    /// Something other than this window changed a project or called a `ui_*` tool.
    client_seen: bool,
    allow_close: bool,
    /// `ui_screenshot` callers waiting for the next painted frame.
    shots: Vec<std::sync::mpsc::SyncSender<Arc<egui::ColorImage>>>,
    shots_since: Option<Instant>,
}

const S: f64 = FLICKS_PER_SECOND as f64;

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, session: Session, options: Options) -> Self {
        let ctx = cc.egui_ctx.clone();
        ctx.set_theme(egui::Theme::Dark);
        let shared = Arc::new(bridge::Shared::default());
        let mut app = App {
            monitors: monitors::Monitors::new(&ctx, &session),
            playback: playback::Playback::default(),
            keymap: crate::default_keymap(),
            session: session.clone(),
            options,
            tabs: Vec::new(),
            active: 0,
            status: String::new(),
            dialog: None,
            shared: shared.clone(),
            press: None,
            wand_rect: None,
            wand_press: None,
            requests: Vec::new(),
            polled: None,
            export_job: None,
            effects: Vec::new(),
            effects_at: None,
            fx_drag: HashMap::new(),
            fx_text: None,
            show_effect_controls: false,
            show_effects: false,
            client_seen: false,
            allow_close: false,
            shots: Vec::new(),
            shots_since: None,
        };
        let mut want = None;
        if let Some(path) = app.options.open.clone() {
            want = app.call("project_open", json!({ "path": path }));
        }
        if session.projects().is_empty() {
            want = app.call("project_new", json!({ "name": "Untitled" }));
        }
        app.sync();
        app.client_seen = false;
        if let Some(id) = want.as_ref().and_then(|v| v["project_id"].as_str()) {
            app.activate(id);
        }
        session.set_ui(Some(Arc::new(bridge::Bridge {
            shared,
            session: session.clone(),
            ctx,
        })));
        app
    }

    /// Session id (`p1`, ...) of the project in the active tab.
    pub fn project_id(&self) -> Option<&str> {
        self.tab().map(|t| t.id.as_str())
    }

    /// Editing state of the active tab.
    pub fn ui_state(&self) -> Option<&UiState> {
        self.tab().map(|t| &t.ui)
    }

    /// Runs an action exactly as its key would (applies the resulting `Op`s through the session).
    pub fn run_action(&mut self, action: Action) {
        let Some(t) = self.tabs.get_mut(self.active) else {
            return;
        };
        let project = t.project.clone();
        let ops = crate::perform(action, &project, &mut t.ui);
        self.apply(ops);
    }

    fn tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active)
    }

    fn activate(&mut self, id: &str) {
        if let Some(i) = self.tabs.iter().position(|t| t.id == id) {
            self.active = i;
        }
    }

    /// Follows the session: tabs for opened/closed projects, fresh snapshots (U-APP-3).
    fn sync(&mut self) {
        let infos = self.session.projects();
        let active = self.project_id().map(str::to_string);
        let n = self.tabs.len();
        self.tabs.retain(|t| infos.iter().any(|i| i.id == t.id));
        let mut changed = self.tabs.len() != n;
        for info in &infos {
            match self.tabs.iter_mut().find(|t| t.id == info.id) {
                Some(t) => {
                    t.name.clone_from(&info.name);
                    t.path.clone_from(&info.path);
                    t.dirty = info.dirty;
                    if t.revision != info.revision {
                        refresh(t, &self.session);
                        changed = true;
                    }
                }
                None => {
                    if let Some(t) = new_tab(&self.session, info) {
                        self.tabs.push(t);
                        changed = true;
                    }
                }
            }
        }
        self.client_seen |= changed;
        self.active = active
            .and_then(|id| self.tabs.iter().position(|t| t.id == id))
            .unwrap_or(self.active)
            .min(self.tabs.len().saturating_sub(1));
    }

    /// Re-reads the active project after this window changed it (not counted as a client).
    fn refresh_active(&mut self) {
        let infos = self.session.projects();
        if let Some(t) = self.tabs.get_mut(self.active)
            && let Some(info) = infos.iter().find(|i| i.id == t.id)
        {
            t.dirty = info.dirty;
            t.path.clone_from(&info.path);
            refresh(t, &self.session);
        }
    }

    /// A tool call; an error goes to the status bar (U-APP-9).
    fn call(&mut self, tool: &str, args: Value) -> Option<Value> {
        match self.session.call(tool, args) {
            Ok(o) => Some(o.json),
            Err(e) => {
                self.status = e;
                None
            }
        }
    }

    /// A tool call on the active project.
    fn pcall(&mut self, tool: &str, mut args: Value) -> Option<Value> {
        args["project_id"] = json!(self.project_id()?);
        let r = self.call(tool, args);
        self.refresh_active();
        r
    }

    /// One atomic batch through the `edit` tool.
    fn edit(&mut self, commands: Vec<Command>) -> Option<Value> {
        if commands.is_empty() {
            return None;
        }
        match serde_json::to_value(&commands) {
            Ok(c) => self.pcall("edit", json!({ "commands": c })),
            Err(e) => {
                self.status = e.to_string();
                None
            }
        }
    }

    fn apply(&mut self, ops: Vec<Op>) {
        for op in ops {
            match op {
                Op::Edit(c) => {
                    self.edit(c);
                }
                Op::Undo => {
                    self.pcall("undo", json!({}));
                }
                Op::Redo => {
                    self.pcall("redo", json!({}));
                }
                Op::Save => self.save(false),
                Op::Import => self.import(),
                Op::Export => self.export(),
                Op::SpeedDialog => {
                    let percent = self.selected_clip().map_or(100.0, |c| c.speed * 100.0);
                    self.dialog = Some(Dialog::Speed {
                        percent,
                        ripple: false,
                    });
                }
                Op::GainDialog => self.dialog = Some(Dialog::Gain { db: 0.0 }),
                Op::WandPrompt => {
                    self.dialog = Some(Dialog::Wand {
                        text: String::new(),
                    })
                }
            }
        }
    }

    /// First selected clip that still exists.
    fn selected_clip(&self) -> Option<&adit_core::Clip> {
        let t = self.tab()?;
        t.ui.selection
            .iter()
            .find_map(|&id| t.project.clip(id).map(|(_, _, c)| c))
    }

    fn save(&mut self, ask: bool) {
        let Some(t) = self.tab() else { return };
        let path = if ask || t.path.is_none() {
            match rfd::FileDialog::new()
                .add_filter("adit project", &["adit"])
                .set_file_name(format!("{}.adit", t.name))
                .save_file()
            {
                Some(p) => Some(p.to_string_lossy().into_owned()),
                None => return,
            }
        } else {
            None
        };
        let args = path.map_or(json!({}), |p| json!({ "path": p }));
        if self.pcall("project_save", args).is_some() {
            self.status = format!(
                "saved {}",
                self.tab().and_then(|t| t.path.clone()).unwrap_or_default()
            );
        }
    }

    fn open(&mut self) {
        let Some(p) = rfd::FileDialog::new()
            .add_filter("adit project", &["adit"])
            .pick_file()
        else {
            return;
        };
        if let Some(v) = self.call("project_open", json!({ "path": p.to_string_lossy() })) {
            self.sync();
            if let Some(id) = v["project_id"].as_str() {
                self.activate(id);
            }
        }
    }

    fn new_project(&mut self) {
        if let Some(v) = self.call("project_new", json!({ "name": "Untitled" })) {
            self.sync();
            if let Some(id) = v["project_id"].as_str() {
                self.activate(id);
            }
        }
    }

    fn import(&mut self) {
        let Some(files) = rfd::FileDialog::new().pick_files() else {
            return;
        };
        let paths: Vec<String> = files
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        if self
            .pcall("media_import", json!({ "paths": paths }))
            .is_some()
        {
            self.status = format!("imported {} file(s)", paths.len());
        }
    }

    fn export(&mut self) {
        let Some(t) = self.tab() else { return };
        let seq = t.ui.sequence_id;
        let Some(p) = rfd::FileDialog::new()
            .add_filter("H.264 video", &["mp4"])
            .add_filter("ProRes", &["mov"])
            .add_filter("GIF", &["gif"])
            .set_file_name(format!("{}.mp4", t.name))
            .save_file()
        else {
            return;
        };
        let path = p.to_string_lossy().into_owned();
        let codec = match p.extension().and_then(|e| e.to_str()) {
            Some("mov") => json!({ "codec": "prores", "audio": "pcm" }),
            Some("gif") => json!({ "codec": "gif", "audio": "none" }),
            _ => json!({}),
        };
        let mut args = json!({ "sequence_id": seq, "path": path });
        if let (Some(a), Some(c)) = (args.as_object_mut(), codec.as_object()) {
            a.extend(c.clone());
        }
        if let Some(v) = self.pcall("export", args) {
            self.export_job = v["job_id"].as_str().map(str::to_string);
            self.status = format!("exporting to {path}");
        }
    }

    /// Requests list and export job, a few times a second.
    fn poll(&mut self) {
        if self
            .polled
            .is_some_and(|t| t.elapsed() < Duration::from_millis(250))
        {
            return;
        }
        self.polled = Some(Instant::now());
        if let Ok(o) = self.session.call("request_list", json!({})) {
            self.requests = o.json["requests"].as_array().cloned().unwrap_or_default();
        }
        if let Some(job) = self.export_job.clone()
            && let Ok(o) = self.session.call("job_status", json!({ "job_id": job }))
        {
            let j = &o.json;
            match j["state"].as_str() {
                Some("running") => {
                    self.status = format!(
                        "exporting: frame {} of {}",
                        j["frames_done"], j["frames_total"]
                    )
                }
                Some(s) => {
                    self.status = format!(
                        "export {s}: {}{}",
                        j["path"].as_str().unwrap_or(""),
                        j["error"]
                            .as_str()
                            .map(|e| format!(" ({e})"))
                            .unwrap_or_default()
                    );
                    self.export_job = None;
                }
                None => self.export_job = None,
            }
        }
    }

    /// Keys through the key map unless a text field or dialog has focus (U-KEYS).
    fn keys(&mut self, ctx: &egui::Context) {
        if self.dialog.is_some() || ctx.text_edit_focused() {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        for e in events {
            if let egui::Event::Key {
                key,
                pressed: true,
                modifiers: m,
                ..
            } = e
            {
                let chord = KeyChord {
                    key,
                    ctrl: m.ctrl || m.command,
                    shift: m.shift,
                    alt: m.alt,
                };
                if let Some(&(_, a)) = self.keymap.iter().find(|(c, _)| *c == chord) {
                    self.run_action(a);
                }
            }
        }
    }

    /// `ui_set` changes queued by the bridge, applied on the UI thread.
    fn apply_bridge(&mut self) {
        for s in self.shared.take_pending() {
            if let Some(pid) = &s.project_id {
                self.activate(pid);
            }
            let Some(t) = self.tabs.get_mut(self.active) else {
                continue;
            };
            let fps = t.project.sequence(t.ui.sequence_id).map(|q| q.fps);
            let round = |x: Time| fps.map_or(x, |f| frame(x, f));
            if let Some(p) = s.playhead {
                t.ui.playhead = round(p);
            }
            if let Some(m) = s.mark_in {
                t.ui.mark_in = m.map(round);
            }
            if let Some(m) = s.mark_out {
                t.ui.mark_out = m.map(round);
            }
            if let Some(sel) = s.selection {
                t.ui.selection = sel;
            }
            if let Some(tool) = s.tool {
                t.ui.tool = tool;
            }
        }
    }

    fn mirror(&self) -> Option<Value> {
        let t = self.tab()?;
        let u = &t.ui;
        Some(json!({
            "project_id": t.id,
            "sequence_id": u.sequence_id,
            "playhead": u.playhead,
            "playhead_s": u.playhead as f64 / S,
            "selection": u.selection,
            "mark_in": u.mark_in,
            "mark_out": u.mark_out,
            "tool": bridge::tool_name(u.tool),
            "playing": u.playing,
        }))
    }

    fn any_dirty(&self) -> bool {
        self.tabs.iter().any(|t| t.dirty)
    }
}

impl Drop for App {
    fn drop(&mut self) {
        // The bridge holds a session clone; detaching breaks the cycle.
        self.session.set_ui(None);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.sync();
        self.apply_bridge();
        if self.shared.take_touched() {
            self.client_seen = true;
        }
        self.poll();
        self.keys(&ctx);
        if let Some(t) = self.tabs.get_mut(self.active) {
            self.playback.tick(
                &self.session,
                &t.id,
                t.revision,
                &t.project,
                &mut t.ui,
                &mut self.status,
            );
            if t.ui.playing {
                ctx.request_repaint();
            }
        } else {
            self.playback.stop();
        }
        if let Some(t) = self.tabs.get(self.active) {
            self.monitors
                .update(&t.id, t.revision, &t.project, &t.ui, &mut self.status);
        }
        self.monitors.upload(&ctx);

        if ctx.input(|i| i.viewport().close_requested()) && self.any_dirty() && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.dialog = Some(Dialog::ConfirmClose);
        }

        self.draw(ui);

        // Keep polling the session for edits made over MCP (U-APP-3).
        ctx.request_repaint_after(Duration::from_millis(100));
        self.shared.set_mirror(self.mirror());
        self.shots.append(&mut self.shared.take_shots());
        if self.shots.is_empty() {
            return;
        }
        // Wait (up to 3 s) for a Program render in flight, so the shot shows the current state.
        let since = *self.shots_since.get_or_insert_with(Instant::now);
        if self.monitors.busy() && since.elapsed() < Duration::from_secs(3) {
            ctx.request_repaint_after(Duration::from_millis(20));
        } else {
            let shots = std::mem::take(&mut self.shots);
            self.shots_since = None;
            ctx.request_screenshot(move |img| {
                for tx in shots {
                    let _ = tx.try_send(img.clone());
                }
            });
        }
    }
}

/// Opens the window and blocks until it closes.
pub fn run(session: Session, options: Options) -> Result<(), String> {
    let native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("adit")
            .with_inner_size([1600.0, 1000.0]),
        ..Default::default()
    };
    eframe::run_native(
        "adit",
        native,
        Box::new(move |cc| Ok(Box::new(App::new(cc, session, options)))),
    )
    .map_err(|e| e.to_string())
}

fn new_tab(session: &Session, info: &ProjectInfo) -> Option<Tab> {
    let (project, revision) = session.snapshot(&info.id)?;
    let seq = project.sequences.first().map_or(0, |s| s.id);
    Some(Tab {
        id: info.id.clone(),
        name: info.name.clone(),
        path: info.path.clone(),
        dirty: info.dirty,
        project: Arc::new(project),
        revision,
        ui: UiState::new(seq),
    })
}

/// New snapshot; drops what no longer exists from the tab's state.
fn refresh(t: &mut Tab, session: &Session) {
    let Some((p, rev)) = session.snapshot(&t.id) else {
        return;
    };
    if p.sequence(t.ui.sequence_id).is_none() {
        t.ui = UiState::new(p.sequences.first().map_or(0, |s| s.id));
    }
    t.ui.selection.retain(|&id| p.clip(id).is_some());
    t.ui.targets.retain(|&id| p.track(id).is_some());
    if t.ui
        .source
        .as_ref()
        .is_some_and(|s| !p.media.iter().any(|m| m.id == s.media_id))
    {
        t.ui.source = None;
    }
    t.project = Arc::new(p);
    t.revision = rev;
}

/// Start of the frame containing `t`.
fn frame(t: Time, fps: Rational) -> Time {
    adit_core::frame_to_time(time_to_frame(t, fps), fps)
}

/// `hh:mm:ss:ff`, non-drop at ceil(fps).
fn timecode(t: Time, fps: Rational) -> String {
    let per = (fps.num as i64 + fps.den as i64 - 1) / (fps.den.max(1) as i64);
    let per = per.max(1);
    let f = time_to_frame(t.max(0), fps);
    let (s, ff) = (f / per, f % per);
    format!("{:02}:{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60, ff)
}
