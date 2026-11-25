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

/// Modal dialogs. Each remembers the project and clips it acts on (U-FIX-6).
enum Dialog {
    Speed {
        pid: String,
        clips: Vec<Id>,
        linked: bool,
        percent: f64,
        ripple: bool,
    },
    Gain {
        pid: String,
        clips: Vec<Id>,
        db: f64,
    },
    Wand {
        pid: String,
        sequence_id: Id,
        clips: Vec<Id>,
        mark_in: Option<Time>,
        mark_out: Option<Time>,
        rect: Option<[f64; 4]>,
        text: String,
        /// A request to change adit itself (U-ED-18), from this panel if any.
        app: bool,
        panel: Option<String>,
    },
    /// U-ED-19: files for Claude to bring in as `text` says.
    AiImport {
        pid: String,
        sequence_id: Id,
        paths: Vec<String>,
        text: String,
    },
    ConfirmClose,
    CloseProject {
        pid: String,
        name: String,
    },
}

impl Dialog {
    fn project(&self) -> Option<&str> {
        match self {
            Dialog::Speed { pid, .. }
            | Dialog::Gain { pid, .. }
            | Dialog::Wand { pid, .. }
            | Dialog::AiImport { pid, .. }
            | Dialog::CloseProject { pid, .. } => Some(pid),
            Dialog::ConfirmClose => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum TextField {
    Content,
    Font,
    AudioFilter,
}

/// Text typed into an Effect Controls field, held until the field loses focus (U-FIX-5).
struct TextBuf {
    pid: String,
    clip: Id,
    field: TextField,
    widget: egui::Id,
    value: String,
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
    text_buf: Option<TextBuf>,
    /// Media path -> readable (`None` while the check runs) (U-FIX-11).
    offline: HashMap<String, Option<bool>>,
    offline_check: worker::Checker,
    show_effect_controls: bool,
    /// Vertical scroll of the timeline track rows, px (U-FIX-7).
    timeline_scroll_y: f32,
    show_effects: bool,
    /// Something other than this window changed a project or called a `ui_*` tool.
    client_seen: bool,
    allow_close: bool,
    /// `ui_screenshot` callers waiting for the next painted frame.
    shots: Vec<std::sync::mpsc::SyncSender<Arc<egui::ColorImage>>>,
    shots_since: Option<Instant>,
    ctx: egui::Context,
    /// Copied clips, shared by all tabs (U-ED-10).
    clipboard: Vec<crate::ClipboardItem>,
    /// Timeline right-click menu: the hit it was opened on and its items (U-ED-10).
    menu: Option<(crate::Hit, Vec<crate::MenuItem>)>,
    uniform_scale: bool,
    /// Program monitor divisor 1, 2 or 4 (U-ED-17).
    preview_scale: u32,
    /// Waveforms by media path, `None` while computing (U-ED-14).
    waves: worker::Worker<Vec<String>, Vec<(String, Wave)>>,
    wave_cache: HashMap<String, Option<Wave>>,
    /// Panels a wand click may name, with their rects this frame (U-ED-18).
    panel_rects: Vec<(&'static str, egui::Rect)>,
    wand_panel_press: Option<egui::Pos2>,
    /// Track and unsnapped time under the pointer in the timeline, for file drops (U-ED-16).
    pointer_track: Option<(Id, Time)>,
}

type Wave = Result<Arc<Vec<f32>>, String>;

const S: f64 = FLICKS_PER_SECOND as f64;

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, session: Session, options: Options) -> Self {
        let ctx = cc.egui_ctx.clone();
        ctx.set_theme(egui::Theme::Dark);
        let shared = Arc::new(bridge::Shared::default());
        let s3 = session.clone();
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
            text_buf: None,
            offline: HashMap::new(),
            offline_check: worker::Checker::new(&ctx, &session),
            show_effect_controls: false,
            timeline_scroll_y: 0.0,
            show_effects: false,
            client_seen: false,
            allow_close: false,
            shots: Vec::new(),
            shots_since: None,
            ctx: ctx.clone(),
            clipboard: Vec::new(),
            menu: None,
            uniform_scale: true,
            preview_scale: 2,
            waves: worker::Worker::new(&ctx, move |paths: Vec<String>| {
                paths
                    .into_iter()
                    .map(|p| {
                        let w = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            s3.waveform(&p)
                        }))
                        .unwrap_or_else(|_| Err("waveform panicked".into()));
                        (p, w)
                    })
                    .collect()
            }),
            wave_cache: HashMap::new(),
            panel_rects: Vec::new(),
            wand_panel_press: None,
            pointer_track: None,
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

    /// Opens the AI import dialog for these files (GUI.md U-ED-19).
    pub fn ai_import(&mut self, paths: Vec<String>) {
        let Some(t) = self.tab() else {
            self.status = "open a project first".into();
            return;
        };
        self.dialog = Some(Dialog::AiImport {
            pid: t.id.clone(),
            sequence_id: t.ui.sequence_id,
            paths,
            text: String::new(),
        });
    }

    /// File > AI Import... and the Project panel menu: asks for files first (U-ED-19).
    fn ai_import_pick(&mut self) {
        if let Some(files) = rfd::FileDialog::new().pick_files() {
            self.ai_import(
                files
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect(),
            );
        }
    }

    /// The wand prompt for a change to adit itself (U-ED-18).
    fn app_wand(&mut self, panel: Option<&str>, rect: Option<[f64; 4]>) {
        let Some(t) = self.tab() else {
            self.status = "open a project first".into();
            return;
        };
        self.dialog = Some(Dialog::Wand {
            pid: t.id.clone(),
            sequence_id: t.ui.sequence_id,
            clips: Vec::new(),
            mark_in: None,
            mark_out: None,
            rect,
            text: String::new(),
            app: true,
            panel: panel.map(str::to_string),
        });
    }

    /// Tool `Wand`: a click or rectangle on a panel listed in `panel_rects` (U-ED-18).
    fn wand_panels(&mut self, ctx: &egui::Context, had_dialog: bool) {
        if had_dialog
            || self.dialog.is_some()
            || self.ui_state().map(|u| u.tool) != Some(crate::Tool::Wand)
        {
            self.wand_panel_press = None;
            return;
        }
        let (pressed, released, pos) = ctx.input(|i| {
            (
                i.pointer.primary_pressed(),
                i.pointer.primary_released(),
                i.pointer.interact_pos(),
            )
        });
        let panel_at = |p: egui::Pos2| {
            self.panel_rects
                .iter()
                .find(|(_, r)| r.contains(p))
                .map(|(n, _)| *n)
        };
        if pressed {
            // Not through a menu or popup lying over the panel.
            self.wand_panel_press = pos.filter(|&p| {
                panel_at(p).is_some()
                    && ctx
                        .layer_id_at(p)
                        .is_none_or(|l| l.order == egui::Order::Background)
            });
        }
        if !released {
            return;
        }
        let (Some(a), Some(b)) = (self.wand_panel_press.take(), pos) else {
            return;
        };
        let w = ctx.content_rect();
        let rect = (a.distance(b) > 4.0).then(|| {
            let r = egui::Rect::from_two_pos(a, b).intersect(w);
            [
                ((r.left() - w.left()) / w.width()) as f64,
                ((r.top() - w.top()) / w.height()) as f64,
                (r.width() / w.width()) as f64,
                (r.height() / w.height()) as f64,
            ]
        });
        let panel = panel_at(a);
        self.app_wand(panel, rect);
    }

    /// Files dropped on the window: one `media_import`, then placed if over a track (U-ED-16).
    fn file_drop(&mut self, ctx: &egui::Context) {
        let paths: Vec<String> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_string_lossy().into_owned())
                .collect()
        });
        if paths.is_empty() {
            return;
        }
        let Some(v) = self.pcall("media_import", json!({ "paths": paths })) else {
            return;
        };
        self.status = format!("imported {} file(s)", paths.len());
        let ids: Vec<Id> = v["media"]
            .as_array()
            .map(|a| a.iter().filter_map(|m| m["id"].as_u64()).collect())
            .unwrap_or_default();
        if let Some((track, at)) = self.pointer_track {
            self.drop_media(&ids, Some(track), at);
        }
    }

    /// Runs an action exactly as its key would (applies the resulting `Op`s through the session).
    pub fn run_action(&mut self, action: Action) {
        let Some(t) = self.tabs.get_mut(self.active) else {
            return;
        };
        let project = t.project.clone();
        // Taken before `perform`: Cut clears the selection like Delete (U-ED-1).
        let copied =
            matches!(action, Action::Copy | Action::Cut).then(|| crate::copy(&project, &t.ui));
        let ops = crate::perform(action, &project, &mut t.ui);
        if ops.is_empty() {
            self.explain_locked(action);
        }
        for op in ops {
            match (&op, &copied) {
                (Op::Copy, Some(c)) => self.set_clipboard(c.clone()),
                _ => self.apply(vec![op]),
            }
        }
    }

    /// U-ED-10, U-ED-21: the shared clipboard, and text so the OS delivers a paste event for ^V.
    fn set_clipboard(&mut self, items: Vec<crate::ClipboardItem>) {
        self.clipboard = items;
        self.ctx.copy_text("adit clips".into());
    }

    /// An edit that did nothing because every clip it would touch is on a locked track (U-FIX-1)
    /// says so, as the core would (U-APP-9).
    fn explain_locked(&mut self, action: Action) {
        let edits = matches!(
            action,
            Action::AddEdit
                | Action::Delete
                | Action::RippleDelete
                | Action::Nudge(_)
                | Action::ToggleEnable
        );
        let Some(t) = self.tab().filter(|_| edits) else {
            return;
        };
        let locked = t.ui.selection.iter().find_map(|&id| {
            let (_, track, _) = t.project.clip(id)?;
            track.locked.then_some(track.id)
        });
        if let Some(track) = locked {
            self.status = format!("track {track} is locked");
        }
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
        for t in self
            .tabs
            .iter()
            .filter(|t| !infos.iter().any(|i| i.id == t.id))
        {
            // By name only when work was lost; otherwise by id, so a closed project's name leaves
            // the window with its tab (U-APP-3 vs U-FIX-10).
            self.status = if t.dirty && t.revision > 0 {
                format!("project {} was closed with unsaved changes", t.name)
            } else {
                format!("project {} was closed", t.id)
            };
        }
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
        self.check_media();
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

    /// One atomic batch through the `edit` tool, on the active project.
    fn edit(&mut self, commands: Vec<Command>) -> Option<Value> {
        let pid = self.project_id()?.to_string();
        self.edit_in(&pid, commands)
    }

    /// One atomic batch on a given project.
    fn edit_in(&mut self, pid: &str, commands: Vec<Command>) -> Option<Value> {
        if commands.is_empty() {
            return None;
        }
        let r = match serde_json::to_value(&commands) {
            Ok(c) => self.call("edit", json!({ "project_id": pid, "commands": c })),
            Err(e) => {
                self.status = e.to_string();
                None
            }
        };
        let infos = self.session.projects();
        if let Some(t) = self.tabs.iter_mut().find(|t| t.id == pid) {
            if let Some(info) = infos.iter().find(|i| i.id == pid) {
                t.dirty = info.dirty;
            }
            refresh(t, &self.session);
        }
        r
    }

    /// Starts a readability check for new media; reports offline media once (U-FIX-11).
    fn check_media(&mut self) {
        for t in &self.tabs {
            for m in t.project.media.iter().filter(|m| m.spec.video.is_some()) {
                if !self.offline.contains_key(&m.spec.path) {
                    self.offline.insert(m.spec.path.clone(), None);
                    self.offline_check.request(m.spec.path.clone());
                }
            }
        }
        while let Some((path, ok)) = self.offline_check.result() {
            if !ok {
                let name = self
                    .tabs
                    .iter()
                    .flat_map(|t| &t.project.media)
                    .find(|m| m.spec.path == path)
                    .map_or(path.clone(), |m| m.spec.name.clone());
                self.status = format!("media offline: {name}");
            }
            self.offline.insert(path, Some(ok));
        }
    }

    /// Commits held Effect Controls text once its field lost focus or the clip changed (U-FIX-5).
    fn commit_text(&mut self, ctx: &egui::Context) {
        let Some(b) = &self.text_buf else { return };
        let focused = ctx.memory(|m| m.has_focus(b.widget));
        let same_clip = self.project_id() == Some(b.pid.as_str())
            && self.selected_clip().map(|c| c.id) == Some(b.clip);
        if focused && same_clip {
            return;
        }
        let Some(b) = self.text_buf.take() else {
            return;
        };
        let Some((p, _)) = self.session.snapshot(&b.pid) else {
            return;
        };
        let Some((_, _, clip)) = p.clip(b.clip) else {
            return;
        };
        let cmd = match (b.field, &clip.source) {
            (TextField::AudioFilter, _) if clip.audio_filter != b.value => Command::SetClip {
                clip_id: b.clip,
                name: None,
                enabled: None,
                reverse: None,
                blend: None,
                audio_filter: Some(b.value),
                label: None,
            },
            (TextField::Content, adit_core::ClipSource::Text { text })
                if text.content != b.value =>
            {
                let mut text = text.clone();
                text.content = b.value;
                Command::SetText {
                    clip_id: b.clip,
                    text,
                }
            }
            (TextField::Font, adit_core::ClipSource::Text { text }) if text.font != b.value => {
                let mut text = text.clone();
                text.font = b.value;
                Command::SetText {
                    clip_id: b.clip,
                    text,
                }
            }
            _ => return,
        };
        self.edit_in(&b.pid, vec![cmd]);
    }

    fn apply(&mut self, ops: Vec<Op>) {
        for op in ops {
            match op {
                Op::Copy => {
                    let Some(t) = self.tab() else { continue };
                    let items = crate::copy(&t.project, &t.ui);
                    self.set_clipboard(items);
                }
                Op::Paste => {
                    let Some(t) = self.tabs.get_mut(self.active) else {
                        continue;
                    };
                    let project = t.project.clone();
                    let ops = crate::paste(&self.clipboard, &project, &mut t.ui);
                    self.apply(ops);
                }
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
                    let Some(t) = self.tab() else { continue };
                    self.dialog = Some(Dialog::Speed {
                        pid: t.id.clone(),
                        clips: existing_selection(t),
                        linked: t.ui.linked_selection,
                        percent,
                        ripple: false,
                    });
                }
                Op::GainDialog => {
                    let Some(t) = self.tab() else { continue };
                    self.dialog = Some(Dialog::Gain {
                        pid: t.id.clone(),
                        clips: existing_selection(t),
                        db: 0.0,
                    });
                }
                Op::WandPrompt => {
                    let Some(t) = self.tab() else { continue };
                    let d = Dialog::Wand {
                        pid: t.id.clone(),
                        sequence_id: t.ui.sequence_id,
                        clips: existing_selection(t),
                        mark_in: t.ui.mark_in,
                        mark_out: t.ui.mark_out,
                        rect: self.wand_rect,
                        text: String::new(),
                        app: false,
                        panel: None,
                    };
                    self.wand_rect = None;
                    self.dialog = Some(d);
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
            // The OS turns ^C / ^X / ^V into these instead of key events (U-ED-10).
            match e {
                egui::Event::Copy => self.run_action(Action::Copy),
                egui::Event::Cut => self.run_action(Action::Cut),
                egui::Event::Paste(_) => self.run_action(Action::Paste),
                _ => {}
            }
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

    /// `ui_set` changes queued by the bridge (validated and rounded there), in call order.
    fn apply_bridge(&mut self) {
        for s in self.shared.take_pending() {
            if let Some(pid) = &s.project_id {
                self.activate(pid);
            }
            if let Some(t) = self.tabs.get_mut(self.active) {
                s.apply(&mut t.ui);
            }
            if let Some(d) = s.preview_scale {
                self.preview_scale = d;
            }
        }
    }

    /// Unsaved work worth asking about: a dirty project that was ever changed (U-FIX-10).
    fn any_unsaved(&self) -> bool {
        self.tabs.iter().any(|t| t.dirty && t.revision > 0)
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
        self.commit_text(&ctx);
        if let Some(pid) = self.dialog.as_ref().and_then(Dialog::project)
            && !self.tabs.iter().any(|t| t.id == pid)
        {
            self.status = format!("project {pid} is closed; its dialog was dismissed");
            self.dialog = None;
        }
        if matches!(self.dialog, Some(Dialog::ConfirmClose)) && !self.any_unsaved() {
            self.dialog = None;
        }
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
            self.monitors.update(
                &t.id,
                t.revision,
                &t.project,
                &t.ui,
                self.preview_scale,
                &mut self.status,
            );
        }
        self.monitors.upload(&ctx);

        if ctx.input(|i| i.viewport().close_requested()) && self.any_unsaved() && !self.allow_close
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.dialog = Some(Dialog::ConfirmClose);
        }

        let had_dialog = self.dialog.is_some();
        self.draw(ui);
        self.wand_panels(&ctx, had_dialog);
        self.file_drop(&ctx);

        // Poll the session for edits made over MCP (U-APP-3), when idle at most every 500 ms
        // (U-FIX-13). Bridge calls, finished renders and playback wake the window themselves.
        ctx.request_repaint_after(Duration::from_millis(500));
        self.shared.set_mirror(
            self.project_id().map(str::to_string),
            self.preview_scale,
            self.tabs
                .iter()
                .map(|t| (t.id.clone(), t.ui.clone()))
                .collect(),
        );
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
    warm_renderer(&session);
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

/// Creates the session's GPU device before the window creates its own. Two devices starting at
/// the same time on the Vulkan backend froze the window at boot (knowledge/egui-037-gui-notes.md).
fn warm_renderer(session: &Session) {
    let mut doc = adit_core::Document::new(Project::new("warm"));
    let made = doc.apply(adit_core::Command::AddSequence {
        name: String::new(),
        width: 16,
        height: 16,
        fps: adit_core::Rational { num: 30, den: 1 },
        sample_rate: 48000,
        video_tracks: 0,
        audio_tracks: 0,
    });
    if let Ok(made) = made
        && let Some(&seq) = made.created.first()
    {
        // A failure here is reported by the first real render; nothing to do with it now.
        let _ = session.render(doc.project(), seq, 0);
    }
}

/// Selected clip ids that still exist.
fn existing_selection(t: &Tab) -> Vec<Id> {
    t.ui.selection
        .iter()
        .copied()
        .filter(|&id| t.project.clip(id).is_some())
        .collect()
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
