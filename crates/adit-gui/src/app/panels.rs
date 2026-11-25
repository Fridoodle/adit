//! Layout and every panel except the timeline (U-APP-1, 4, 5, 6).

use super::{App, Dialog, S, monitors, timecode};
use crate::{Action, SourceState, Tool};
use adit_core::{
    BlendMode, ClipSource, Command, Easing, FLICKS_PER_SECOND, Id, ParamTarget, PlaceMode, Time,
    TrackKind,
};
use eframe::egui::{self, Color32, RichText};
use serde_json::json;

const TOOLS: &[(Tool, &str, &str)] = &[
    (Tool::Select, "Sel", "Selection (V)"),
    (Tool::TrackSelect, "Trk", "Track Select (A)"),
    (Tool::Ripple, "Rip", "Ripple Edit (B)"),
    (Tool::Roll, "Rol", "Rolling Edit (N)"),
    (Tool::RateStretch, "Rate", "Rate Stretch (R)"),
    (Tool::Razor, "Raz", "Razor (C)"),
    (Tool::Slip, "Slip", "Slip (Y)"),
    (Tool::Slide, "Sld", "Slide (U)"),
    (Tool::Pen, "Pen", "Pen (P)"),
    (Tool::Hand, "Hand", "Hand (H)"),
    (Tool::Zoom, "Zoom", "Zoom (Z)"),
    (Tool::Type, "Type", "Type (T)"),
    (Tool::Wand, "Wand", "Magic Wand (Shift+W)"),
];

type ParamRow = (Option<Id>, String, f64, Option<(f64, f64)>);

const BLENDS: &[(BlendMode, &str)] = &[
    (BlendMode::Normal, "Normal"),
    (BlendMode::Add, "Add"),
    (BlendMode::Multiply, "Multiply"),
    (BlendMode::Screen, "Screen"),
    (BlendMode::Overlay, "Overlay"),
    (BlendMode::Darken, "Darken"),
    (BlendMode::Lighten, "Lighten"),
    (BlendMode::Difference, "Difference"),
];

fn title(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).strong().color(Color32::from_gray(220)));
}

fn frame(fill: u8) -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_gray(fill))
        .inner_margin(6)
}

impl App {
    pub(super) fn draw(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("menu").show(ui, |ui| self.menu(ui));
        egui::Panel::top("tabs").frame(frame(30)).show(ui, |ui| {
            ui.horizontal(|ui| {
                let mut pick = None;
                for (i, t) in self.tabs.iter().enumerate() {
                    let name = if t.dirty {
                        format!("{} *", t.name)
                    } else {
                        t.name.clone()
                    };
                    if ui.selectable_label(i == self.active, name).clicked() {
                        pick = Some(i);
                    }
                }
                if let Some(i) = pick {
                    self.active = i;
                }
            });
        });
        egui::Panel::bottom("status")
            .frame(frame(26))
            .show(ui, |ui| self.status_bar(ui));
        let h = ui.available_height();
        egui::Panel::bottom("lower")
            .resizable(true)
            .default_size(h * 0.48)
            .frame(frame(20))
            .show(ui, |ui| {
                egui::Panel::left("bin")
                    .resizable(true)
                    .default_size(300.0)
                    .frame(frame(26))
                    .show(ui, |ui| self.bin_panel(ui));
                egui::Panel::left("tools")
                    .exact_size(64.0)
                    .frame(frame(30))
                    .show(ui, |ui| self.tool_bar(ui));
                egui::Panel::right("meters")
                    .exact_size(56.0)
                    .frame(frame(26))
                    .show(ui, |ui| self.meters(ui));
                egui::CentralPanel::default()
                    .frame(frame(22))
                    .show(ui, |ui| self.timeline_panel(ui));
            });
        let w = ui.available_width();
        egui::Panel::left("source")
            .resizable(true)
            .default_size(w * 0.36)
            .frame(frame(26))
            .show(ui, |ui| self.source_panel(ui));
        egui::Panel::right("requests")
            .resizable(true)
            .default_size(260.0)
            .frame(frame(26))
            .show(ui, |ui| self.requests_panel(ui));
        egui::CentralPanel::default()
            .frame(frame(26))
            .show(ui, |ui| self.program_panel(ui));
        self.dialogs(ui.ctx());
    }

    fn menu(&mut self, ui: &mut egui::Ui) {
        let mut act: Option<Action> = None;
        egui::MenuBar::new().ui(ui, |ui| {
            let mut item = |ui: &mut egui::Ui, text: &str, a: Action| {
                if ui.button(text).clicked() {
                    act = Some(a);
                }
            };
            ui.menu_button("File", |ui| {
                if ui.button("New Project").clicked() {
                    self.new_project();
                }
                if ui.button("Open Project...").clicked() {
                    self.open();
                }
                item(ui, "Save", Action::Save);
                if ui.button("Save As...").clicked() {
                    self.save(true);
                }
                if ui.button("Close Project").clicked() {
                    self.pcall("project_close", json!({}));
                    self.sync();
                }
                ui.separator();
                item(ui, "Import...", Action::Import);
                item(ui, "Export Media...", Action::Export);
                ui.separator();
                if ui.button("Exit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Edit", |ui| {
                item(ui, "Undo", Action::Undo);
                item(ui, "Redo", Action::Redo);
                ui.separator();
                item(ui, "Clear", Action::Delete);
                item(ui, "Ripple Delete", Action::RippleDelete);
                ui.separator();
                item(ui, "Select All", Action::SelectAll);
                item(ui, "Deselect All", Action::DeselectAll);
            });
            ui.menu_button("Clip", |ui| {
                item(ui, "Speed/Duration...", Action::SpeedDialog);
                item(ui, "Audio Gain...", Action::GainDialog);
                item(ui, "Enable", Action::ToggleEnable);
                item(ui, "Link / Unlink", Action::ToggleLink);
                item(ui, "Group", Action::Group);
                item(ui, "Ungroup", Action::Ungroup);
                ui.separator();
                item(ui, "Insert", Action::Insert);
                item(ui, "Overwrite", Action::Overwrite);
                item(ui, "Nudge Left", Action::Nudge(-1));
                item(ui, "Nudge Right", Action::Nudge(1));
            });
            ui.menu_button("Sequence", |ui| {
                item(ui, "Add Edit", Action::AddEdit);
                item(ui, "Add Edit to All Tracks", Action::AddEditAll);
                item(ui, "Ripple Trim Previous", Action::RippleTrimPrev);
                item(ui, "Ripple Trim Next", Action::RippleTrimNext);
                item(ui, "Lift", Action::Lift);
                item(ui, "Extract", Action::Extract);
                ui.separator();
                item(ui, "Apply Video Transition", Action::ApplyVideoTransition);
                item(ui, "Apply Audio Transition", Action::ApplyAudioTransition);
                item(
                    ui,
                    "Apply Default Transitions",
                    Action::ApplyDefaultTransitions,
                );
                ui.separator();
                item(ui, "Match Frame", Action::MatchFrame);
                item(ui, "Snap", Action::ToggleSnap);
                item(ui, "Linked Selection", Action::ToggleLinkedSelection);
                item(ui, "Zoom In", Action::ZoomIn);
                item(ui, "Zoom Out", Action::ZoomOut);
                item(ui, "Zoom to Sequence", Action::ZoomFit);
            });
            ui.menu_button("Markers", |ui| {
                item(ui, "Mark In", Action::MarkIn);
                item(ui, "Mark Out", Action::MarkOut);
                item(ui, "Mark Clip", Action::MarkClip);
                item(ui, "Go to In", Action::GoToIn);
                item(ui, "Go to Out", Action::GoToOut);
                item(ui, "Clear In", Action::ClearIn);
                item(ui, "Clear Out", Action::ClearOut);
                item(ui, "Clear In and Out", Action::ClearInOut);
                ui.separator();
                item(ui, "Add Marker", Action::AddMarker);
            });
            ui.menu_button("Window", |ui| {
                let mut pick = None;
                for (i, t) in self.tabs.iter().enumerate() {
                    if ui.selectable_label(i == self.active, &t.name).clicked() {
                        pick = Some(i);
                    }
                }
                if let Some(i) = pick {
                    self.active = i;
                }
                ui.separator();
                ui.checkbox(&mut self.show_effect_controls, "Effect Controls panel");
                ui.checkbox(&mut self.show_effects, "Effects panel");
            });
            ui.menu_button("Help", |ui| {
                ui.label("adit: edit by hand or let Claude edit over MCP.");
                ui.label("Keys follow Premiere Pro defaults (Space play, J/K/L, I/O, ...).");
                if ui.button("Copy MCP command").clicked() {
                    self.copy_mcp(ui.ctx());
                }
            });
        });
        if let Some(a) = act {
            self.run_action(a);
        }
    }

    fn mcp_command(&self) -> Option<String> {
        let url = self.options.mcp_url.as_ref()?;
        let header = self
            .options
            .mcp_token
            .as_ref()
            .map(|t| format!(" --header \"Authorization: Bearer {t}\""))
            .unwrap_or_default();
        Some(format!(
            "claude mcp add --transport http adit {url}{header}"
        ))
    }

    fn copy_mcp(&mut self, ctx: &egui::Context) {
        if let Some(c) = self.mcp_command() {
            ctx.copy_text(c);
            self.status = "copied the claude mcp add command".into();
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            match self.options.mcp_url.clone() {
                Some(url) => {
                    ui.label(format!("MCP {url}"));
                    if ui
                        .button("Copy claude mcp add")
                        .on_hover_text("Copies a ready-to-paste command including the token")
                        .clicked()
                    {
                        self.copy_mcp(ui.ctx());
                    }
                    if !self.client_seen {
                        ui.weak("no client has called a tool yet");
                    }
                }
                None => {
                    ui.weak("MCP is off");
                }
            }
            ui.separator();
            // A real widget so the message is accessible (U-APP-10).
            ui.add(egui::Label::new(self.status.as_str()).wrap_mode(egui::TextWrapMode::Extend));
        });
    }

    fn tool_bar(&mut self, ui: &mut egui::Ui) {
        let Some(t) = self.tabs.get_mut(self.active) else {
            return;
        };
        ui.vertical_centered_justified(|ui| {
            for &(tool, short, tip) in TOOLS {
                if ui
                    .add(
                        egui::Button::selectable(t.ui.tool == tool, short)
                            .small()
                            .wrap_mode(egui::TextWrapMode::Extend),
                    )
                    .on_hover_text(tip)
                    .clicked()
                {
                    t.ui.tool = tool;
                }
            }
        });
    }

    fn meters(&mut self, ui: &mut egui::Ui) {
        ui.small("dB");
        let (rect, _) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
        let p = ui.painter_at(rect);
        let w = (rect.width() - 6.0) / 2.0;
        for (i, peak) in self.playback.peaks.iter().enumerate() {
            let db = 20.0 * peak.max(1e-6).log10();
            let f = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
            let x = rect.left() + i as f32 * (w + 6.0);
            let bar = egui::Rect::from_min_max(
                egui::pos2(x, rect.top()),
                egui::pos2(x + w, rect.bottom()),
            );
            p.rect_filled(bar, 0.0, Color32::from_gray(12));
            let lit =
                egui::Rect::from_min_max(egui::pos2(x, rect.bottom() - f * rect.height()), bar.max);
            let c = if db > -6.0 {
                Color32::from_rgb(230, 70, 60)
            } else {
                Color32::from_rgb(70, 200, 90)
            };
            p.rect_filled(lit, 0.0, c);
        }
    }

    fn source_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .selectable_label(!self.show_effect_controls, "Source")
                .clicked()
            {
                self.show_effect_controls = false;
            }
            if ui
                .selectable_label(self.show_effect_controls, "Effect Controls")
                .clicked()
            {
                self.show_effect_controls = true;
            }
        });
        ui.separator();
        if self.show_effect_controls {
            egui::ScrollArea::vertical().show(ui, |ui| self.effect_controls(ui));
            return;
        }
        let Some(t) = self.tabs.get_mut(self.active) else {
            return;
        };
        let Some(src) = t.ui.source.clone() else {
            ui.weak("Double-click a clip in the Project bin to load it here.");
            return;
        };
        let Some(m) = t.project.media.iter().find(|m| m.id == src.media_id) else {
            return;
        };
        let aspect = m
            .spec
            .video
            .as_ref()
            .map_or(16.0 / 9.0, |v| v.width as f32 / v.height.max(1) as f32);
        let fps = m.spec.video.as_ref().map(|v| v.fps);
        monitors::show(
            ui,
            self.monitors.source_tex.as_ref(),
            aspect,
            self.monitors.source_err.as_deref(),
        );
        let dur = m.spec.duration.unwrap_or(5 * FLICKS_PER_SECOND);
        let mut sec = src.playhead as f64 / S;
        let mut cmd = None;
        ui.horizontal(|ui| {
            ui.label(&m.spec.name);
            if let Some(f) = fps {
                ui.monospace(timecode(src.playhead, f));
            }
            let slider = egui::Slider::new(&mut sec, 0.0..=dur as f64 / S).show_value(false);
            if ui.add(slider).changed() {
                let mut tt = (sec * S) as Time;
                if let Some(f) = fps {
                    tt = super::frame(tt, f);
                }
                cmd = Some(SourceState {
                    playhead: tt,
                    ..src.clone()
                });
            }
            if ui.button("Mark In").clicked() {
                cmd = Some(SourceState {
                    mark_in: Some(src.playhead),
                    ..src.clone()
                });
            }
            if ui.button("Mark Out").clicked() {
                cmd = Some(SourceState {
                    mark_out: Some(src.playhead),
                    ..src.clone()
                });
            }
        });
        if let Some(s) = cmd {
            t.ui.source = Some(s);
        }
        ui.horizontal(|ui| {
            if ui.button("Insert").clicked() {
                self.run_action(Action::Insert);
            }
            if ui.button("Overwrite").clicked() {
                self.run_action(Action::Overwrite);
            }
        });
    }

    fn program_panel(&mut self, ui: &mut egui::Ui) {
        title(ui, "Program");
        let Some(t) = self.tab() else {
            return;
        };
        let (aspect, fps, playhead, tool, seq_id) = match t.project.sequence(t.ui.sequence_id) {
            Some(s) => (
                s.width as f32 / s.height.max(1) as f32,
                Some(s.fps),
                t.ui.playhead,
                t.ui.tool,
                s.id,
            ),
            None => (16.0 / 9.0, None, 0, t.ui.tool, 0),
        };
        let (rect, resp) = monitors::show(
            ui,
            self.monitors.program_tex.as_ref(),
            aspect,
            self.monitors.program_err.as_deref(),
        );
        let norm = |p: egui::Pos2| {
            [
                ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64,
                ((p.y - rect.top()) / rect.height()).clamp(0.0, 1.0) as f64,
            ]
        };
        if tool == Tool::Wand {
            if resp.drag_started() {
                self.wand_press = ui.input(|i| i.pointer.press_origin());
            }
            if let (Some(a), Some(b)) = (self.wand_press, ui.input(|i| i.pointer.interact_pos())) {
                let r = egui::Rect::from_two_pos(a, b).intersect(rect);
                ui.painter().rect_stroke(
                    r,
                    0.0,
                    egui::Stroke::new(1.5, Color32::from_rgb(255, 220, 80)),
                    egui::StrokeKind::Inside,
                );
                if resp.drag_stopped() {
                    let (p, q) = (norm(a), norm(b));
                    self.wand_rect = Some([
                        p[0].min(q[0]),
                        p[1].min(q[1]),
                        (p[0] - q[0]).abs(),
                        (p[1] - q[1]).abs(),
                    ]);
                    self.wand_press = None;
                    self.run_action(Action::WandPrompt);
                }
            }
        }
        if tool == Tool::Type && resp.clicked() {
            self.add_text_clip(seq_id, playhead);
        }
        ui.horizontal(|ui| {
            if let Some(f) = fps {
                ui.monospace(
                    RichText::new(timecode(playhead, f)).color(Color32::from_rgb(90, 160, 255)),
                );
            }
            let playing = self.ui_state().is_some_and(|u| u.playing);
            let buttons = [
                ("|<", Action::GoToStart, "Go to start (Home)"),
                ("<|", Action::Step(-1), "Step back (Left)"),
                (
                    if playing { "Pause" } else { "Play" },
                    Action::PlayPause,
                    "Play/Stop (Space)",
                ),
                ("|>", Action::Step(1), "Step forward (Right)"),
                (">|", Action::GoToEnd, "Go to end (End)"),
                ("In", Action::MarkIn, "Mark In (I)"),
                ("Out", Action::MarkOut, "Mark Out (O)"),
                ("Lift", Action::Lift, "Lift (;)"),
                ("Extract", Action::Extract, "Extract (')"),
            ];
            for (text, a, tip) in buttons {
                if ui.button(text).on_hover_text(tip).clicked() {
                    self.run_action(a);
                }
            }
        });
    }

    /// Tool `Type` in the Program monitor: a 5 s text clip on the highest free video track (U-APP-4).
    fn add_text_clip(&mut self, seq_id: Id, at: Time) {
        let Some(t) = self.tab() else { return };
        let Some(seq) = t.project.sequence(seq_id) else {
            return;
        };
        let len = 5 * FLICKS_PER_SECOND;
        let free = seq
            .tracks
            .iter()
            .rev()
            .filter(|x| x.kind == TrackKind::Video && !x.locked)
            .find(|x| x.clips.iter().all(|c| c.end() <= at || c.start >= at + len))
            .map(|x| x.id);
        let track_id = match free {
            Some(id) => id,
            None => {
                let add = Command::AddTrack {
                    sequence_id: seq_id,
                    kind: TrackKind::Video,
                    name: None,
                    index: None,
                };
                match self.edit(vec![add]) {
                    Some(v) => match v["created"][0][0].as_u64() {
                        Some(id) => id,
                        None => return,
                    },
                    None => return,
                }
            }
        };
        let text = adit_core::TextSpec {
            content: "Title".into(),
            font: String::new(),
            size: 96.0,
            color: [1.0, 1.0, 1.0, 1.0],
            align: adit_core::TextAlign::Center,
            bold: false,
            italic: false,
        };
        let add = Command::AddClip {
            track_id,
            linked_track_id: None,
            source: ClipSource::Text { text },
            start: at,
            duration: len,
            source_in: 0,
            mode: PlaceMode::Place,
            name: Some("Title".into()),
        };
        if let Some(id) = self
            .edit(vec![add])
            .and_then(|v| v["created"][0][0].as_u64())
            && let Some(t) = self.tabs.get_mut(self.active)
        {
            t.ui.selection = vec![id];
        }
    }

    fn bin_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.selectable_label(!self.show_effects, "Project").clicked() {
                self.show_effects = false;
            }
            if ui.selectable_label(self.show_effects, "Effects").clicked() {
                self.show_effects = true;
            }
        });
        ui.separator();
        if self.show_effects {
            self.effects_browser(ui);
            return;
        }
        if ui.button("Import...").clicked() {
            self.run_action(Action::Import);
        }
        let Some(t) = self.tabs.get_mut(self.active) else {
            return;
        };
        if t.project.media.is_empty() {
            ui.weak("No media yet. File > Import (Ctrl+I).");
        }
        let project = t.project.clone();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for m in &project.media {
                let loaded = t.ui.source.as_ref().is_some_and(|s| s.media_id == m.id);
                let dur = m
                    .spec
                    .duration
                    .map_or("still".into(), |d| format!("{:.2}s", d as f64 / S));
                let r = ui.add(
                    egui::Button::selectable(loaded, format!("{}  {dur}", m.spec.name))
                        .sense(egui::Sense::click_and_drag()),
                );
                r.dnd_set_drag_payload(m.id);
                if r.double_clicked() {
                    t.ui.source = Some(SourceState {
                        media_id: m.id,
                        playhead: 0,
                        mark_in: None,
                        mark_out: None,
                    });
                    self.show_effect_controls = false;
                }
            }
        });
    }

    fn refresh_effects(&mut self) {
        let Some(t) = self.tab() else { return };
        let key = (t.id.clone(), t.revision);
        if self.effects_at.as_ref() == Some(&key) {
            return;
        }
        if let Ok(o) = self
            .session
            .call("effect_list", json!({ "project_id": t.id }))
        {
            self.effects = o.json["effects"].as_array().cloned().unwrap_or_default();
        }
        self.effects_at = Some(key);
    }

    /// Double-click: a filter on the selection, a transition at the playhead cut (U-APP-4).
    fn effects_browser(&mut self, ui: &mut egui::Ui) {
        self.refresh_effects();
        let mut apply: Option<(String, bool)> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for kind in ["transition", "filter"] {
                ui.weak(if kind == "filter" {
                    "Video Effects"
                } else {
                    "Transitions"
                });
                for e in self.effects.iter().filter(|e| e["kind"] == kind) {
                    let name = e["name"].as_str().unwrap_or_default();
                    if ui.selectable_label(false, name).double_clicked() {
                        apply = Some((name.to_string(), kind == "transition"));
                    }
                }
            }
        });
        let Some((name, transition)) = apply else {
            return;
        };
        let Some(t) = self.tabs.get_mut(self.active) else {
            return;
        };
        let project = t.project.clone();
        let cmds: Vec<Command> = if transition {
            let action = if name == "crossfade" {
                Action::ApplyAudioTransition
            } else {
                Action::ApplyVideoTransition
            };
            // The default-transition rule places it; only the effect differs.
            crate::perform(action, &project, &mut t.ui)
                .into_iter()
                .flat_map(|op| match op {
                    crate::Op::Edit(c) => c,
                    _ => vec![],
                })
                .map(|c| match c {
                    Command::AddTransition {
                        track_id,
                        left,
                        right,
                        duration,
                        params,
                        ..
                    } => Command::AddTransition {
                        track_id,
                        left,
                        right,
                        effect: name.clone(),
                        duration,
                        params,
                    },
                    c => c,
                })
                .collect()
        } else {
            t.ui.selection
                .iter()
                .filter(|&&id| project.clip(id).is_some())
                .map(|&clip_id| Command::AddEffect {
                    clip_id,
                    effect: name.clone(),
                    index: None,
                    params: Default::default(),
                })
                .collect()
        };
        if cmds.is_empty() {
            self.status = if transition {
                "no cut at the playhead".into()
            } else {
                "select clips to apply the effect to".into()
            };
        }
        self.edit(cmds);
    }

    /// Selected clip's properties, each with a keyframe toggle at the playhead (U-APP-4).
    fn effect_controls(&mut self, ui: &mut egui::Ui) {
        self.refresh_effects();
        let Some(t) = self.tab() else { return };
        let Some((_, track, clip)) = t.ui.selection.iter().find_map(|&id| t.project.clip(id))
        else {
            ui.weak("Select a clip in the timeline.");
            return;
        };
        let clip = clip.clone();
        let kind = track.kind;
        let local = (t.ui.playhead - clip.start).clamp(0, clip.duration);
        let mut cmds: Vec<Command> = Vec::new();
        ui.label(RichText::new(&clip.name).strong());
        // (effect instance or built-in, parameter, value at the playhead, range)
        let mut rows: Vec<ParamRow> = Vec::new();
        let video = [
            "position_x",
            "position_y",
            "scale_x",
            "scale_y",
            "rotation",
            "anchor_x",
            "anchor_y",
            "opacity",
            "crop_left",
            "crop_top",
            "crop_right",
            "crop_bottom",
        ];
        let names: &[&str] = match kind {
            TrackKind::Video => &video,
            TrackKind::Audio => &["volume", "pan"],
        };
        for &n in names {
            let range = match n {
                "opacity" => Some((0.0, 1.0)),
                "pan" => Some((-1.0, 1.0)),
                _ => None,
            };
            rows.push((
                None,
                n.to_string(),
                clip.param(n, local).unwrap_or(0.0),
                range,
            ));
        }
        for e in &clip.effects {
            let def = self.effects.iter().find(|d| d["name"] == e.effect.as_str());
            let defs = def
                .and_then(|d| d["params"].as_array())
                .cloned()
                .unwrap_or_default();
            for p in defs {
                let name = p["name"].as_str().unwrap_or_default().to_string();
                let v = e
                    .params
                    .get(&name)
                    .map_or(p["default"].as_f64().unwrap_or(0.0), |x| x.eval(local));
                let range = p["min"].as_f64().zip(p["max"].as_f64());
                rows.push((Some(e.id), name, v, range));
            }
        }
        egui::Grid::new("fx")
            .num_columns(3)
            .striped(true)
            .show(ui, |ui| {
                let mut last_effect = None;
                for (eff, name, cur, range) in rows {
                    if eff != last_effect {
                        if let Some(e) = clip.effects.iter().find(|e| Some(e.id) == eff) {
                            let mut on = e.enabled;
                            if ui
                                .checkbox(&mut on, RichText::new(&e.effect).strong())
                                .changed()
                            {
                                cmds.push(Command::SetEffectEnabled {
                                    clip_id: clip.id,
                                    effect_id: e.id,
                                    enabled: on,
                                });
                            }
                            if ui.small_button("Remove").clicked() {
                                cmds.push(Command::RemoveEffect {
                                    clip_id: clip.id,
                                    effect_id: e.id,
                                });
                            }
                            ui.end_row();
                        }
                        last_effect = eff;
                    }
                    let target = ParamTarget {
                        clip_id: clip.id,
                        effect_id: eff,
                        name: name.clone(),
                    };
                    let param = match eff {
                        None => clip.params.get(&name),
                        Some(id) => clip
                            .effects
                            .iter()
                            .find(|e| e.id == id)
                            .and_then(|e| e.params.get(&name)),
                    };
                    let keyed = param.is_some_and(|p| !p.keyframes.is_empty());
                    let at_key = param.is_some_and(|p| p.keyframes.iter().any(|k| k.time == local));
                    ui.label(&name);
                    let key = format!("{}/{eff:?}/{name}", clip.id);
                    let mut v = self.fx_drag.get(&key).copied().unwrap_or(cur);
                    let mut dv = egui::DragValue::new(&mut v).speed(0.01).max_decimals(3);
                    if let Some((a, b)) = range {
                        dv = dv.range(a..=b);
                    }
                    let r = ui.add(dv);
                    if r.changed() {
                        self.fx_drag.insert(key.clone(), v);
                    }
                    if ((r.changed() && !r.dragged()) || r.drag_stopped())
                        && let Some(v) = self.fx_drag.remove(&key)
                    {
                        cmds.push(if keyed {
                            Command::SetKeyframe {
                                target: target.clone(),
                                time: local,
                                value: v,
                                easing: Easing::Linear,
                            }
                        } else {
                            Command::SetParam {
                                target: target.clone(),
                                value: v,
                            }
                        });
                    }
                    if ui
                        .add(egui::Button::selectable(at_key, "Key").small())
                        .on_hover_text("Keyframe at the playhead")
                        .clicked()
                    {
                        cmds.push(if at_key {
                            Command::RemoveKeyframe {
                                target,
                                time: local,
                            }
                        } else {
                            Command::SetKeyframe {
                                target,
                                time: local,
                                value: cur,
                                easing: Easing::Linear,
                            }
                        });
                    }
                    ui.end_row();
                }
            });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Blend");
            let current = BLENDS
                .iter()
                .find(|b| b.0 == clip.blend)
                .map_or("Normal", |b| b.1);
            egui::ComboBox::from_id_salt("blend")
                .selected_text(current)
                .show_ui(ui, |ui| {
                    for &(b, n) in BLENDS {
                        if ui.selectable_label(b == clip.blend, n).clicked() && b != clip.blend {
                            cmds.push(set_clip(clip.id, |c| {
                                if let Command::SetClip { blend, .. } = c {
                                    *blend = Some(b);
                                }
                            }));
                        }
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.label("Speed %");
            let key = format!("{}/speed", clip.id);
            let mut v = self
                .fx_drag
                .get(&key)
                .copied()
                .unwrap_or(clip.speed * 100.0);
            let r = ui.add(egui::DragValue::new(&mut v).range(1.0..=10000.0).speed(1.0));
            if r.changed() {
                self.fx_drag.insert(key.clone(), v);
            }
            if ((r.changed() && !r.dragged()) || r.drag_stopped())
                && let Some(v) = self.fx_drag.remove(&key)
            {
                cmds.push(Command::SetClipSpeed {
                    clip_id: clip.id,
                    speed: v / 100.0,
                    ripple: false,
                    include_linked: true,
                });
            }
        });
        ui.horizontal(|ui| {
            ui.label("Audio filter");
            let text = match &mut self.fx_text {
                Some((id, s)) if *id == clip.id => s,
                slot => {
                    *slot = Some((clip.id, clip.audio_filter.clone()));
                    &mut slot.as_mut().expect("just set").1
                }
            };
            let r = ui.text_edit_singleline(text);
            if r.lost_focus() && *text != clip.audio_filter {
                let f = text.clone();
                cmds.push(set_clip(clip.id, |c| {
                    if let Command::SetClip { audio_filter, .. } = c {
                        *audio_filter = Some(f);
                    }
                }));
            }
        });
        if let ClipSource::Text { text } = &clip.source {
            ui.label("Text");
            let mut spec = text.clone();
            let r = ui.text_edit_multiline(&mut spec.content);
            let s = ui.add(
                egui::DragValue::new(&mut spec.size)
                    .range(1.0..=2000.0)
                    .prefix("size "),
            );
            if (r.lost_focus() || s.drag_stopped() || (s.changed() && !s.dragged()))
                && spec != *text
            {
                cmds.push(Command::SetText {
                    clip_id: clip.id,
                    text: spec,
                });
            }
        }
        self.edit(cmds);
    }

    fn requests_panel(&mut self, ui: &mut egui::Ui) {
        title(ui, "Requests");
        ui.separator();
        if self.requests.is_empty() {
            ui.weak("Mark a range and use the magic wand (Ctrl+Shift+W) to ask Claude.");
        }
        egui::ScrollArea::vertical().show(ui, |ui| {
            for r in self.requests.iter().rev() {
                let status = r["status"].as_str().unwrap_or("?");
                let color = match status {
                    "done" => Color32::from_rgb(90, 200, 110),
                    "claimed" => Color32::from_rgb(240, 190, 60),
                    _ => Color32::from_gray(170),
                };
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(status).color(color).small());
                    ui.label(r["text"].as_str().unwrap_or_default());
                });
                if let Some(m) = r["message"].as_str().filter(|m| !m.is_empty()) {
                    ui.label(RichText::new(m).italics());
                }
                ui.separator();
            }
        });
    }

    fn dialogs(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.dialog.take() else {
            return;
        };
        let mut close = false;
        let mut cmds = Vec::new();
        let mut send: Option<String> = None;
        let mut quit = false;
        let sel: Vec<Id> = self.tab().map_or(vec![], |t| {
            t.ui.selection
                .iter()
                .copied()
                .filter(|&id| t.project.clip(id).is_some())
                .collect()
        });
        let linked = self.ui_state().is_some_and(|u| u.linked_selection);
        let describe = self.wand_description();
        let r = egui::Modal::new(egui::Id::unique("dialog")).show(ctx, |ui| match &mut d {
            Dialog::Speed { percent, ripple } => {
                ui.heading("Clip Speed");
                ui.add(
                    egui::DragValue::new(percent)
                        .range(1.0..=10000.0)
                        .suffix(" %"),
                );
                ui.checkbox(ripple, "Ripple edit, shifting trailing clips");
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        cmds = sel
                            .iter()
                            .map(|&clip_id| Command::SetClipSpeed {
                                clip_id,
                                speed: *percent / 100.0,
                                ripple: *ripple,
                                include_linked: linked,
                            })
                            .collect();
                        close = true;
                    }
                    close |= ui.button("Cancel").clicked();
                });
            }
            Dialog::Gain { db } => {
                ui.heading("Audio Gain");
                ui.add(egui::DragValue::new(db).range(-96.0..=24.0).suffix(" dB"));
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        cmds = sel
                            .iter()
                            .map(|&clip_id| Command::SetParam {
                                target: ParamTarget {
                                    clip_id,
                                    effect_id: None,
                                    name: "volume".into(),
                                },
                                value: *db,
                            })
                            .collect();
                        close = true;
                    }
                    close |= ui.button("Cancel").clicked();
                });
            }
            Dialog::Wand { text } => {
                ui.heading("Magic Wand");
                ui.label(&describe);
                let label = ui.label("Request");
                let edit = ui.add(
                    egui::TextEdit::multiline(text)
                        .desired_rows(3)
                        .hint_text("What should Claude do here?"),
                );
                edit.labelled_by(label.id);
                ui.horizontal(|ui| {
                    if ui.button("Send").clicked() && !text.trim().is_empty() {
                        send = Some(text.clone());
                        close = true;
                    }
                    close |= ui.button("Cancel").clicked();
                });
            }
            Dialog::ConfirmClose => {
                ui.heading("Unsaved changes");
                ui.label("Some projects have unsaved changes. Quit anyway?");
                ui.horizontal(|ui| {
                    if ui.button("Quit without saving").clicked() {
                        quit = true;
                        close = true;
                    }
                    close |= ui.button("Cancel").clicked();
                });
            }
        });
        if r.should_close() {
            close = true;
        }
        if !close {
            self.dialog = Some(d);
        }
        self.edit(cmds);
        if let Some(text) = send {
            self.send_request(text);
        }
        if quit {
            self.allow_close = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if close {
            self.wand_rect = None;
        }
    }

    fn wand_description(&self) -> String {
        let Some(t) = self.tab() else {
            return String::new();
        };
        let fps = t
            .project
            .sequence(t.ui.sequence_id)
            .map_or(adit_core::Rational { num: 30, den: 1 }, |s| s.fps);
        let range = match (t.ui.mark_in, t.ui.mark_out) {
            (None, None) => "no range marked".to_string(),
            (a, b) => format!(
                "range {} to {}",
                a.map_or("start".into(), |x| timecode(x, fps)),
                b.map_or("end".into(), |x| timecode(x, fps))
            ),
        };
        let n = t.ui.selection.len();
        let rect = if self.wand_rect.is_some() {
            ", a rectangle of the frame"
        } else {
            ""
        };
        format!("{range}, {n} clip(s) selected{rect}")
    }

    /// U-APP-5: files the request for Claude.
    fn send_request(&mut self, text: String) {
        let Some(t) = self.tab() else { return };
        let mut args = json!({
            "sequence_id": t.ui.sequence_id,
            "text": text,
            "clip_ids": t.ui.selection,
        });
        if let Some(a) = t.ui.mark_in {
            args["start"] = json!(a);
        }
        if let Some(b) = t.ui.mark_out {
            args["end"] = json!(b);
        }
        if let Some(r) = self.wand_rect {
            args["rect"] = json!(r);
        }
        if let Some(v) = self.pcall("request_create", args) {
            self.status = format!("request {} sent to Claude", v["request_id"]);
            self.polled = None;
        }
    }
}

fn set_clip(clip_id: Id, f: impl FnOnce(&mut Command)) -> Command {
    let mut c = Command::SetClip {
        clip_id,
        name: None,
        enabled: None,
        reverse: None,
        blend: None,
        audio_filter: None,
    };
    f(&mut c);
    c
}
