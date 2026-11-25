//! The Timeline panel: drawing, mouse gestures, right-click menu and drops (U-APP-1, U-APP-4,
//! U-ED-10, U-ED-12 .. U-ED-14).

use super::{App, S, timecode};
use crate::{Action, ClipPart, Drag, Hit, MenuItem, TimelineLayout, Tool, UiState};
use adit_core::{ClipSource, Command, Id, PlaceMode, Project, Sequence, Time, TrackKind};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, pos2, vec2};
use std::sync::Arc;

const HEADER: f32 = 150.0;
const RULER: f32 = 24.0;
const TRACK: f32 = 44.0;
const EDGE: f32 = 8.0;
/// Height of the zoom bar under the track rows (U-NAV-3).
const BAR: f32 = 14.0;

/// Clip colour labels 0..=8 (U-ED-10, U-ED-14); 0 keeps the clip's kind colour.
const LABELS: [(&str, Color32); 9] = [
    ("None", Color32::TRANSPARENT),
    ("Violet", Color32::from_rgb(166, 140, 218)),
    ("Iris", Color32::from_rgb(93, 120, 236)),
    ("Caribbean", Color32::from_rgb(35, 191, 168)),
    ("Lavender", Color32::from_rgb(227, 146, 227)),
    ("Cerulean", Color32::from_rgb(29, 165, 226)),
    ("Forest", Color32::from_rgb(99, 153, 43)),
    ("Rose", Color32::from_rgb(241, 93, 141)),
    ("Mango", Color32::from_rgb(241, 155, 47)),
];

fn layout(ui: &UiState) -> TimelineLayout {
    TimelineLayout {
        px_per_sec: ui.px_per_sec,
        scroll: ui.scroll,
        header_width: HEADER,
        ruler_height: RULER,
        track_height: TRACK,
        edge_px: EDGE,
    }
}

/// Seconds between ruler ticks so labels are at least 80 px apart.
fn tick_step(px_per_sec: f64) -> f64 {
    [
        0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0, 1800.0,
    ]
    .into_iter()
    .find(|s| s * px_per_sec >= 80.0)
    .unwrap_or(3600.0)
}

/// Snap tolerance: 8 px in time (U-APP-4, U-ED-21).
fn tolerance(ui: &UiState) -> Time {
    (8.0 * S / ui.px_per_sec).round() as Time
}

/// Button text of a context-menu action (U-ED-10).
fn menu_text(a: Action) -> &'static str {
    match a {
        Action::Cut => "Cut",
        Action::Copy => "Copy",
        Action::Paste => "Paste",
        Action::Delete => "Delete",
        Action::RippleDelete | Action::CloseGap { .. } => "Ripple Delete",
        Action::Unlink => "Ungroup Audio/Video",
        Action::Group => "Group",
        Action::Ungroup => "Ungroup Clips",
        Action::ToggleEnable => "Enable/Disable",
        Action::SpeedDialog => "Speed...",
        Action::GainDialog => "Audio Gain...",
        Action::WandPrompt => "Ask AI...",
        _ => "?",
    }
}

/// Clips a `Select` body drag of `clip_id` moves: the selection if it holds the clip, else the
/// clip, with linked partners when linked selection is on (as `drag` moves them).
fn moving(seq: &Sequence, clip_id: Id, ui: &UiState) -> Vec<Id> {
    let mut ids = if ui.selection.contains(&clip_id) {
        ui.selection.clone()
    } else {
        vec![clip_id]
    };
    if ui.linked_selection {
        let all = || seq.tracks.iter().flat_map(|t| &t.clips);
        let links: Vec<Id> = all()
            .filter(|c| ids.contains(&c.id))
            .filter_map(|c| c.link)
            .collect();
        ids.extend(
            all()
                .filter(|c| c.link.is_some_and(|l| links.contains(&l)) && !ids.contains(&c.id))
                .map(|c| c.id)
                .collect::<Vec<_>>(),
        );
    }
    ids
}

impl App {
    pub(super) fn timeline_panel(&mut self, ui: &mut egui::Ui) {
        self.pointer_track = None;
        if self.menu.is_some() || ui.input(|i| i.pointer.secondary_clicked()) {
            self.refresh_effects();
        }
        if let Some(done) = self.waves.latest() {
            for (path, w) in done {
                self.wave_cache.insert(path, Some(w));
            }
        }
        // U-ED-23f: a failed waveform is tried again once the project changes.
        if let Some(t) = self.tab()
            && self.wave_rev != Some((t.id.clone(), t.revision))
        {
            self.wave_rev = Some((t.id.clone(), t.revision));
            self.wave_cache.retain(|_, w| !matches!(w, Some(Err(_))));
        }
        let mut switch = None;
        let mut toggle_snap = false;
        ui.horizontal(|ui| {
            ui.strong("Timeline");
            if let Some(t) = self.tab()
                && let Some(seq) = t.project.sequence(t.ui.sequence_id)
            {
                // Sequence selector (U-FIX-8): the current name, a menu of the others.
                ui.menu_button(seq.name.as_str(), |ui| {
                    for q in t.project.sequences.iter().filter(|q| q.id != seq.id) {
                        if ui.button(q.name.as_str()).clicked() {
                            switch = Some(q.id);
                        }
                    }
                })
                .response
                .on_hover_text("Sequence");
                ui.monospace(timecode(t.ui.playhead, seq.fps));
                toggle_snap = ui
                    .add(egui::Button::selectable(t.ui.snap, "Snap"))
                    .on_hover_text("Snapping (S)")
                    .clicked();
            }
        });
        if toggle_snap {
            self.run_action(Action::ToggleSnap);
        }
        if let (Some(id), Some(t)) = (switch, self.tabs.get_mut(self.active)) {
            super::bridge::switch_sequence(&mut t.ui, id);
            self.timeline_scroll_y = 0.0;
        }
        let Some(t) = self.tabs.get_mut(self.active) else {
            ui.weak("No project open.");
            return;
        };
        let project = t.project.clone();
        let Some(seq) = project.sequence(t.ui.sequence_id) else {
            ui.weak("No sequence.");
            return;
        };
        // Exactly the space left: the track count never changes the panel size (U-FIX-7).
        let (full, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
        let bottom = (full.bottom() - BAR).max(full.top());
        let rect = Rect::from_min_max(full.min, pos2(full.right(), bottom));
        let resp = ui.interact(
            rect,
            ui.scope_id().with("timeline"),
            Sense::click_and_drag(),
        );
        t.ui.view_width = (rect.width() - HEADER).max(1.0);
        let (mods, press_pos, pos, latest) = ui.input(|i| {
            (
                i.modifiers,
                i.pointer.press_origin(),
                i.pointer.interact_pos(),
                i.pointer.latest_pos(),
            )
        });

        // Zoom bar (U-NAV-3), applied before this frame's layout.
        let bar_rect = Rect::from_min_max(pos2(full.left() + HEADER, rect.bottom()), full.max);
        let bar_resp = ui.interact(bar_rect, ui.scope_id().with("zoom bar"), Sense::drag());
        bar_resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ScrollBar, true, "Zoom bar"));
        let dur = seq.duration();
        let key = (t.id.clone(), seq.id);
        let thumb = |u: &UiState| {
            let (a, b) = crate::zoom_bar(u, dur);
            let w = bar_rect.width();
            let x0 = bar_rect.left() + a as f32 * w;
            let x1 = (bar_rect.left() + b as f32 * w).max(x0 + 16.0);
            let over = (x1 - bar_rect.right()).max(0.0);
            (x0 - over, x1 - over)
        };
        if bar_resp.drag_started_by(egui::PointerButton::Primary)
            && let Some(p) = press_pos
        {
            let (x0, x1) = thumb(&t.ui);
            let part = if p.x < x0 || p.x > x1 {
                None
            } else if p.x - x0 < 6.0 {
                Some(crate::BarPart::Start)
            } else if x1 - p.x < 6.0 {
                Some(crate::BarPart::End)
            } else {
                Some(crate::BarPart::Thumb)
            };
            self.bar = part.map(|part| (key.clone(), p.x, crate::bar_grab(&t.ui, dur, part)));
        }
        // A gesture of another tab or sequence (switched mid-drag) is dropped.
        if self.bar.as_ref().is_some_and(|b| b.0 != key) {
            self.bar = None;
        }
        if self.pan.as_ref().is_some_and(|g| g.0 != key) {
            self.pan = None;
        }
        if let Some((_, x, grab)) = self.bar.clone() {
            if let Some(p) = pos {
                let d = (p.x - x) as f64 / bar_rect.width().max(1.0) as f64;
                crate::bar_drag(&mut t.ui, &grab, d);
            }
            if !bar_resp.dragged() {
                self.bar = None;
            }
        }

        let rows = layout(&t.ui).rows(&project, seq.id);
        let rows_rect = Rect::from_min_max(pos2(rect.left(), rect.top() + RULER), rect.max);
        let max_scroll = (rows.len() as f32 * TRACK - rows_rect.height()).max(0.0);
        // Pan (U-NAV-5), started on an earlier frame, applied before this frame's layout.
        if let Some((_, p0, scroll0, sy0)) = self.pan.clone() {
            if let Some(p) = pos {
                let dt = ((p.x - p0.x) as f64 * S / t.ui.px_per_sec).round() as Time;
                t.ui.scroll = scroll0.saturating_sub(dt).max(0);
                self.timeline_scroll_y = (sy0 - (p.y - p0.y)).clamp(0.0, max_scroll);
            }
            if !resp.dragged() {
                self.pan = None;
            }
        }
        let lay = layout(&t.ui);
        let sy = self.timeline_scroll_y.clamp(0.0, max_scroll);
        let x_of = |time: Time| rect.left() + lay.time_to_x(time);
        let row_top = |i: usize| rect.top() + RULER + i as f32 * TRACK - sy;
        // Panel-local coordinates as `TimelineLayout` sees them: rows scrolled, ruler fixed.
        let local = |p: Pos2| {
            let y = p.y - rect.top();
            (p.x - rect.left(), if y < RULER { y } else { y + sy })
        };
        let area = Rect::from_min_max(pos2(rect.left() + HEADER, rect.top()), rect.max);
        let rows_area = area.intersect(rows_rect);

        // Mouse gestures (U-APP-4): hit on press, snap, `drag` on release.
        let row_at = |y: f32| -> Option<Id> {
            let i = ((y - RULER) / TRACK).floor();
            (y >= RULER && i >= 0.0)
                .then(|| rows.get(i as usize).copied())
                .flatten()
        };
        let raw_time = |x: f32| lay.x_to_time(x.max(HEADER)).max(0);
        // U-ED-12: a `Select` body drag snaps the moved clips' edges, not the pointer.
        let body_move = |h: &Hit, u: &UiState| {
            u.tool == Tool::Select
                && matches!(
                    h,
                    Hit::Clip {
                        part: ClipPart::Body,
                        ..
                    }
                )
        };
        // U-FIX-4: pointer times are snapped for clicks and drags alike; U-FIX-3: not for Hand.
        let pointer_time = |x: f32, from: &Hit, ui_state: &UiState| {
            let raw = raw_time(x);
            if ui_state.tool == Tool::Hand {
                return raw;
            }
            let exclude = match *from {
                Hit::Clip { clip_id, .. } if ui_state.selection.contains(&clip_id) => {
                    ui_state.selection.clone()
                }
                Hit::Clip { clip_id, .. } => vec![clip_id],
                _ => vec![],
            };
            crate::snap(raw, &project, ui_state, &exclude, tolerance(ui_state))
        };
        let pressed = |p: Pos2, ui_state: &UiState| {
            let (x, y) = local(p);
            let h = lay.hit(&project, seq.id, x, y);
            let time = if body_move(&h, ui_state) {
                raw_time(x)
            } else {
                pointer_time(x, &h, ui_state)
            };
            with_time(h, time)
        };
        let make_drag = |from: Hit, p: Pos2, ui_state: &UiState, click: bool| {
            let (x, y) = local(p);
            let (to_time, to_track) = if click {
                (hit_time(&from), hit_track(&from))
            } else if let Hit::Clip { clip_id, time, .. } = from
                && body_move(&from, ui_state)
            {
                let ids = moving(seq, clip_id, ui_state);
                let d = crate::snap_delta(
                    raw_time(x) - time,
                    &ids,
                    &project,
                    ui_state,
                    tolerance(ui_state),
                );
                (time + d, row_at(y))
            } else {
                (pointer_time(x, &from, ui_state), row_at(y))
            };
            Drag {
                from,
                to_time,
                to_track,
                shift: mods.shift,
                alt: mods.alt,
            }
        };
        // U-ED-13: while a gesture is dragged, the clips as it would leave them.
        let dragging = match (self.press, pos) {
            (Some(from), Some(p)) if resp.dragged() && !matches!(from, Hit::Ruler { .. }) => {
                Some(make_drag(from, p, &t.ui, false))
            }
            _ => None,
        };
        // U-ED-23b: computed once per change of the gesture or the project revision.
        let after = dragging.and_then(|d| match &self.drag_preview {
            Some((pid, rev, k, p)) if *pid == t.id && *rev == t.revision && *k == d => p.clone(),
            _ => {
                let p = crate::preview(d, &project, &t.ui).map(Arc::new);
                self.drag_preview = Some((t.id.clone(), t.revision, d, p.clone()));
                p
            }
        });
        let shown = after
            .as_ref()
            .and_then(|p| p.sequence(seq.id))
            .unwrap_or(seq);

        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::from_gray(22));
        painter.rect_filled(
            Rect::from_min_max(area.min, pos2(area.right(), area.top() + RULER)),
            0.0,
            Color32::from_gray(34),
        );
        let ruler_painter = ui.painter_at(area);
        let row_painter = ui.painter_at(rows_rect);
        let area_painter = ui.painter_at(rows_area);
        // Widgets of the rows live in a child clipped to the rows, not allocated in the panel.
        let mut rows_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));
        rows_ui.set_clip_rect(rows_rect);

        // Ruler ticks.
        let step = tick_step(lay.px_per_sec);
        let first = (lay.scroll as f64 / S / step).floor() as i64;
        for k in first.. {
            let tt = (k as f64 * step * S) as Time;
            let x = x_of(tt);
            if x > rect.right() {
                break;
            }
            ruler_painter.line_segment(
                [
                    pos2(x, rect.top() + RULER - 8.0),
                    pos2(x, rect.top() + RULER),
                ],
                Stroke::new(1.0, Color32::from_gray(120)),
            );
            ruler_painter.text(
                pos2(x + 3.0, rect.top() + 2.0),
                egui::Align2::LEFT_TOP,
                timecode(tt, seq.fps),
                egui::FontId::monospace(10.0),
                Color32::from_gray(160),
            );
        }

        // In/out range.
        if t.ui.mark_in.is_some() || t.ui.mark_out.is_some() {
            let a = x_of(t.ui.mark_in.unwrap_or(0));
            let b = x_of(t.ui.mark_out.unwrap_or(seq.duration()));
            let bottom = row_top(rows.len()).min(rect.bottom());
            let r = Rect::from_x_y_ranges(a..=b, rect.top()..=bottom);
            area_painter.rect_filled(r, 0.0, Color32::from_rgba_unmultiplied(90, 140, 255, 24));
            ruler_painter.rect_filled(
                Rect::from_x_y_ranges(a..=b, rect.top() + RULER - 5.0..=rect.top() + RULER),
                0.0,
                Color32::from_rgb(90, 140, 255),
            );
        }

        // Tracks and clips; rows scrolled out of view get no widgets.
        let mut toggles: Vec<Command> = Vec::new();
        let mut target_click = None;
        let mut want_waves: Vec<String> = Vec::new();
        for (i, &tid) in rows.iter().enumerate() {
            let Some(track) = shown.tracks.iter().find(|x| x.id == tid) else {
                continue;
            };
            let y = row_top(i);
            if y + TRACK < rows_rect.top() || y > rows_rect.bottom() {
                continue;
            }
            let row = Rect::from_min_size(pos2(rect.left(), y), vec2(rect.width(), TRACK));
            let shade = if i % 2 == 0 { 28 } else { 25 };
            row_painter.rect_filled(row, 0.0, Color32::from_gray(shade));
            let head = Rect::from_min_size(row.min, vec2(HEADER, TRACK));
            row_painter.rect_filled(head, 0.0, Color32::from_gray(40));
            row_painter.line_segment(
                [pos2(rect.left(), y + TRACK), pos2(rect.right(), y + TRACK)],
                Stroke::new(1.0, Color32::from_gray(15)),
            );
            let targeted = t.ui.targets.is_empty() || t.ui.targets.contains(&tid);
            let name = rows_ui.put(
                Rect::from_min_size(head.min + vec2(4.0, 4.0), vec2(56.0, 18.0)),
                egui::Button::selectable(targeted && !track.locked, track.name.as_str()).small(),
            );
            if name.on_hover_text("Target track").clicked() {
                target_click = Some(tid);
            }
            let flags = [
                ("L", track.locked, "Lock"),
                ("M", track.muted, "Mute"),
                ("S", track.solo, "Solo"),
                ("H", track.hidden, "Hide"),
            ];
            for (k, (label, on, tip)) in flags.into_iter().enumerate() {
                let r = Rect::from_min_size(
                    head.min + vec2(64.0 + k as f32 * 21.0, 4.0),
                    vec2(19.0, 18.0),
                );
                if rows_ui
                    .put(r, egui::Button::selectable(on, label).small())
                    .on_hover_text(tip)
                    .clicked()
                {
                    let v = Some(!on);
                    toggles.push(Command::SetTrack {
                        track_id: tid,
                        name: None,
                        locked: if k == 0 { v } else { None },
                        muted: if k == 1 { v } else { None },
                        solo: if k == 2 { v } else { None },
                        hidden: if k == 3 { v } else { None },
                    });
                }
            }

            for c in &track.clips {
                let (x0, x1) = (x_of(c.start), x_of(c.end()));
                if x1 < area.left() || x0 > area.right() {
                    continue;
                }
                let media = match c.source {
                    ClipSource::Media { media_id } => {
                        project.media.iter().find(|m| m.id == media_id)
                    }
                    _ => None,
                };
                let offline =
                    media.is_some_and(|m| self.offline.get(&m.spec.path) == Some(&Some(false)));
                let r = Rect::from_x_y_ranges(x0..=x1.max(x0 + 1.0), y + 3.0..=y + TRACK - 3.0);
                let base = match (&c.source, track.kind) {
                    _ if offline => Color32::from_rgb(150, 50, 50),
                    _ if (1..=8).contains(&c.label) => LABELS[c.label as usize].1,
                    (_, TrackKind::Audio) => Color32::from_rgb(60, 130, 90),
                    (ClipSource::Text { .. }, _) => Color32::from_rgb(140, 90, 170),
                    (ClipSource::Solid { .. } | ClipSource::Adjustment, _) => {
                        Color32::from_rgb(150, 110, 60)
                    }
                    _ => Color32::from_rgb(70, 110, 170),
                };
                let selected = t.ui.selection.contains(&c.id);
                let fill = if !c.enabled {
                    Color32::from_gray(70)
                } else if selected {
                    base.lerp_to_gamma(Color32::WHITE, 0.25)
                } else {
                    base
                };
                area_painter.rect_filled(r, 3.0, fill);
                // Waveform of the clip's source range, computed off the UI thread (U-ED-14).
                if let (TrackKind::Audio, Some(m)) = (track.kind, media) {
                    match self.wave_cache.get(&m.spec.path) {
                        Some(Some(Ok(peaks))) => {
                            let (mid, half) = (r.center().y, r.height() / 2.0 - 2.0);
                            let src = |x: f32| {
                                let at = lay.x_to_time(x - rect.left());
                                (c.source_time(at) as f64 / S * 50.0).max(0.0) as usize
                            };
                            let mut x = r.left().max(area.left());
                            while x < r.right().min(area.right()) {
                                let (a, b) = (src(x), src(x + 1.0));
                                let (a, b) = (a.min(b), a.max(b).max(a.min(b) + 1));
                                let pk = peaks
                                    .get(a..b.min(peaks.len()))
                                    .and_then(|s| s.iter().copied().reduce(f32::max))
                                    .unwrap_or(0.0);
                                if pk > 0.0 {
                                    area_painter.line_segment(
                                        [pos2(x, mid - pk * half), pos2(x, mid + pk * half)],
                                        Stroke::new(1.0, Color32::from_rgb(30, 70, 45)),
                                    );
                                }
                                x += 1.0;
                            }
                        }
                        Some(_) => {}
                        None => want_waves.push(m.spec.path.clone()),
                    }
                }
                // A 1 px outline keeps adjacent clips apart (U-FIX-7).
                let stroke = if selected {
                    Stroke::new(1.5, Color32::WHITE)
                } else {
                    Stroke::new(1.0, Color32::from_gray(10))
                };
                area_painter.rect_stroke(r, 3.0, stroke, egui::StrokeKind::Inside);
                // The clip name is a real widget so it is accessible (U-APP-10).
                let text_rect = r.shrink2(vec2(4.0, 2.0));
                let mut child = rows_ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(text_rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                child.set_clip_rect(text_rect.intersect(rows_area));
                let name = if c.name.is_empty() { "(clip)" } else { &c.name };
                let label = if offline {
                    format!("[offline] {name}")
                } else {
                    name.to_string()
                };
                child.add(
                    egui::Label::new(egui::RichText::new(label).color(Color32::WHITE).size(12.0))
                        .selectable(false)
                        .wrap_mode(egui::TextWrapMode::Extend),
                );
            }
            // U-ED-13: ranges that held a clip and lose it, or get another one, in red.
            if after.is_some()
                && let Some(old) = seq.tracks.iter().find(|x| x.id == tid)
            {
                for oc in &old.clips {
                    let same: Vec<_> = track.clips.iter().filter(|c| c.id == oc.id).collect();
                    let mut cuts = vec![oc.start, oc.end()];
                    for c in &same {
                        cuts.extend([c.start, c.end()]);
                    }
                    cuts.retain(|&x| x >= oc.start && x <= oc.end());
                    cuts.sort_unstable();
                    cuts.dedup();
                    for w in cuts.windows(2) {
                        let m = (w[0] + w[1]) / 2;
                        if !same.iter().any(|c| c.start <= m && m < c.end()) {
                            area_painter.rect_filled(
                                Rect::from_x_y_ranges(
                                    x_of(w[0])..=x_of(w[1]),
                                    y + 3.0..=y + TRACK - 3.0,
                                ),
                                3.0,
                                Color32::from_rgba_unmultiplied(230, 40, 40, 90),
                            );
                        }
                    }
                }
            }
            // U-ED-14: a transition is a named box over its range at the top of the row.
            for tr in &track.transitions {
                let left = tr
                    .left
                    .and_then(|id| track.clips.iter().find(|c| c.id == id));
                let right = tr
                    .right
                    .and_then(|id| track.clips.iter().find(|c| c.id == id));
                let (a, b) = match (left, right) {
                    (Some(l), Some(_)) => (l.end() - tr.duration / 2, l.end() + tr.duration / 2),
                    (Some(l), None) => (l.end() - tr.duration, l.end()),
                    (None, Some(r)) => (r.start, r.start + tr.duration),
                    (None, None) => continue,
                };
                let r = Rect::from_x_y_ranges(x_of(a)..=x_of(b), y + 3.0..=y + 17.0);
                area_painter.rect_filled(
                    r,
                    2.0,
                    Color32::from_rgba_unmultiplied(230, 230, 230, 150),
                );
                area_painter.rect_stroke(
                    r,
                    2.0,
                    Stroke::new(1.0, Color32::BLACK),
                    egui::StrokeKind::Inside,
                );
                area_painter.line_segment(
                    [r.left_bottom(), r.right_top()],
                    Stroke::new(1.0, Color32::from_gray(60)),
                );
                if r.width() >= 60.0 {
                    area_painter.with_clip_rect(r.intersect(rows_area)).text(
                        r.center(),
                        egui::Align2::CENTER_CENTER,
                        &tr.effect,
                        egui::FontId::proportional(10.0),
                        Color32::BLACK,
                    );
                }
            }
        }
        if !want_waves.is_empty() {
            for p in want_waves {
                self.wave_cache.insert(p, None);
            }
            // The newest request wins in `Worker`, so it names every waveform still missing.
            let missing: Vec<String> = self
                .wave_cache
                .iter()
                .filter(|(_, w)| w.is_none())
                .map(|(p, _)| p.clone())
                .collect();
            self.waves.request(missing);
        }
        if max_scroll > 0.0 {
            let h = rows_rect.height();
            let len = (h * h / (h + max_scroll)).max(20.0);
            let top = rows_rect.top() + (h - len) * sy / max_scroll;
            painter.rect_filled(
                Rect::from_x_y_ranges(rect.right() - 5.0..=rect.right() - 1.0, top..=top + len),
                2.0,
                Color32::from_gray(90),
            );
        }

        // Zoom bar: the thumb is the visible range, its ends resize it (U-NAV-3).
        let bar_painter = ui.painter_at(bar_rect);
        bar_painter.rect_filled(bar_rect, 0.0, Color32::from_gray(28));
        let (x0, x1) = thumb(&t.ui);
        let th = Rect::from_x_y_ranges(x0..=x1, bar_rect.top() + 2.0..=bar_rect.bottom() - 2.0);
        bar_painter.rect_filled(th, 5.0, Color32::from_gray(80));
        for cx in [x0 + 3.0, x1 - 3.0] {
            bar_painter.circle_filled(pos2(cx, th.center().y), 3.0, Color32::from_gray(150));
        }
        if let Some(p) = latest.filter(|p| self.bar.is_some() || bar_rect.contains(*p))
            && self
                .bar
                .as_ref()
                .is_none_or(|(_, _, g)| g.part != crate::BarPart::Thumb)
            && (self.bar.is_some() || p.x >= x0 && (p.x - x0 < 6.0 || x1 - p.x < 6.0) && p.x <= x1)
        {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        // Cursor over the rows: grabbing while panning, resize over an edge that would trim.
        if self.pan.is_some() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        } else if let Some(p) = latest.filter(|p| rows_area.contains(*p))
            && self.press.is_none()
        {
            let (x, y) = local(p);
            if let Hit::Clip { part, .. } = lay.hit(&project, seq.id, x, y) {
                let tool = t.ui.tool;
                let trims = match part {
                    ClipPart::StartEdge => matches!(tool, Tool::Select | Tool::Ripple | Tool::Roll),
                    ClipPart::EndEdge => matches!(
                        tool,
                        Tool::Select | Tool::Ripple | Tool::Roll | Tool::RateStretch
                    ),
                    ClipPart::Body => false,
                };
                if trims {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                }
            }
        } else if matches!(
            self.press,
            Some(Hit::Clip {
                part: ClipPart::StartEdge | ClipPart::EndEdge,
                ..
            })
        ) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }

        // Markers and playhead.
        for m in &seq.markers {
            let x = x_of(m.time);
            let top = rect.top() + RULER - 10.0;
            ruler_painter.add(egui::Shape::convex_polygon(
                vec![pos2(x - 5.0, top), pos2(x + 5.0, top), pos2(x, top + 8.0)],
                Color32::from_rgb(240, 180, 40),
                Stroke::NONE,
            ));
        }
        let px = x_of(t.ui.playhead);
        ruler_painter.line_segment(
            [pos2(px, rect.top()), pos2(px, rect.bottom())],
            Stroke::new(1.5, Color32::from_rgb(230, 60, 60)),
        );
        ruler_painter.add(egui::Shape::convex_polygon(
            vec![
                pos2(px - 6.0, rect.top()),
                pos2(px + 6.0, rect.top()),
                pos2(px, rect.top() + 10.0),
            ],
            Color32::from_rgb(230, 60, 60),
            Stroke::NONE,
        ));

        let mut ops = Vec::new();
        if let Some(d) = dragging {
            // With no result to show, the outline of U-APP-4.
            if after.is_none() {
                preview(
                    &area_painter,
                    &project,
                    &t.ui,
                    d.from,
                    d.to_time,
                    d.to_track,
                    &rows,
                    &|tt| x_of(tt),
                    &row_top,
                );
            }
            // U-ED-12: a line at the candidate a moved edge snapped to.
            if let (Hit::Clip { clip_id, time, .. }, Some(p)) = (d.from, pos)
                && body_move(&d.from, &t.ui)
            {
                let ids = moving(seq, clip_id, &t.ui);
                let raw = raw_time(local(p).0) - time;
                let mut off = t.ui.clone();
                off.snap = false;
                let tol = tolerance(&t.ui);
                let got = d.to_time - time;
                if got != crate::snap_delta(raw, &ids, &project, &off, tol) {
                    let edge = ids
                        .iter()
                        .filter_map(|&id| project.clip(id))
                        .flat_map(|(_, _, c)| [c.start, c.end()])
                        .find(|&e| crate::snap(e + raw, &project, &t.ui, &ids, tol) == e + got);
                    if let Some(e) = edge {
                        let x = x_of(e + got);
                        area_painter.line_segment(
                            [pos2(x, rows_area.top()), pos2(x, rows_area.bottom())],
                            Stroke::new(1.0, Color32::from_rgb(120, 220, 255)),
                        );
                    }
                }
            }
        }
        if resp.drag_started()
            && let Some(p) = press_pos
        {
            // A press left over from a release outside the window never carries over.
            self.press = None;
            let middle = resp.drag_started_by(egui::PointerButton::Middle);
            let primary = resp.drag_started_by(egui::PointerButton::Primary);
            let (x, y) = local(p);
            let on_empty = matches!(lay.hit(&project, seq.id, x, y), Hit::Empty { .. });
            let pan = rows_rect.contains(p)
                && (middle
                    || primary && t.ui.tool == Tool::Hand
                    || primary && t.ui.tool == Tool::Select && on_empty);
            if pan {
                self.pan = Some((key.clone(), p, t.ui.scroll, sy));
            } else if primary {
                self.press = Some(pressed(p, &t.ui));
            }
        }
        if let (Some(from), Some(p)) = (self.press, pos) {
            if resp.dragged() && matches!(from, Hit::Ruler { .. }) {
                // Live scrubbing: a ruler drag only moves the playhead.
                ops.extend(crate::drag(
                    make_drag(from, p, &t.ui, false),
                    &project,
                    &mut t.ui,
                ));
            }
            if resp.drag_stopped() {
                self.press = None;
                let d = make_drag(from, p, &t.ui, false);
                ops.extend(crate::drag(d, &project, &mut t.ui));
            }
        }
        if resp.clicked()
            && let Some(p) = press_pos.or(pos)
        {
            let from = pressed(p, &t.ui);
            let d = make_drag(from, p, &t.ui, true);
            ops.extend(crate::drag(d, &project, &mut t.ui));
        }
        // Right click: the menu of U-ED-9 at the hit, its time on a frame, not snapped.
        if resp.secondary_clicked()
            && let Some(p) = pos
        {
            let (x, y) = local(p);
            let h = lay.hit(&project, seq.id, x, y);
            let h = with_time(h, super::frame(hit_time(&h), seq.fps));
            let items = crate::context_menu(h, !self.clipboard.is_empty(), &project, &mut t.ui);
            self.menu = (!items.is_empty()).then_some((h, items));
        }
        let mut chosen = None;
        let mut transition = None;
        let mut menu_open = false;
        if let Some((hit, items)) = &self.menu {
            let kind = hit_track(hit)
                .and_then(|id| project.track(id))
                .map(|(_, tr)| tr.kind);
            let effects = &self.effects;
            menu_open = resp
                .context_menu(|ui| {
                    for it in items {
                        match it {
                            MenuItem::Do(a) => {
                                if ui.button(menu_text(*a)).clicked() {
                                    chosen = Some(*a);
                                }
                            }
                            MenuItem::Labels => {
                                ui.menu_button("Label", |ui| {
                                    for (n, (name, _)) in LABELS.iter().enumerate() {
                                        if ui.button(*name).clicked() {
                                            chosen = Some(Action::SetLabel(n as u8));
                                        }
                                    }
                                });
                            }
                            MenuItem::Transitions => {
                                ui.menu_button("Add Transition", |ui| {
                                    let names = effects
                                        .iter()
                                        .filter(|e| e["kind"] == "transition")
                                        .filter_map(|e| e["name"].as_str())
                                        .filter(|&n| {
                                            kind != Some(TrackKind::Audio) || n == "crossfade"
                                        });
                                    for n in names {
                                        if ui.button(n).clicked() {
                                            transition = Some(n.to_string());
                                        }
                                    }
                                });
                            }
                        }
                    }
                })
                .is_some();
        }
        if resp.hovered() {
            // egui turns Ctrl + wheel into `zoom_delta` and zeroes the scroll delta (U-NAV-4).
            let (d, z) = ui.input(|i| (i.smooth_scroll_delta(), i.zoom_delta()));
            if mods.command || mods.alt {
                let f = if mods.alt && !mods.command {
                    (d.y as f64 / 200.0).exp()
                } else {
                    z as f64
                };
                if let Some(p) = latest.filter(|p| p.x >= area.left())
                    && f != 1.0
                {
                    crate::zoom_at(&mut t.ui, f, raw_time(local(p).0));
                }
            } else if d.y != 0.0 && max_scroll > 0.0 && !mods.shift {
                self.timeline_scroll_y = (sy - d.y).clamp(0.0, max_scroll);
            } else if d.x + d.y != 0.0 {
                let dt = (-(d.x + d.y) as f64 * S / t.ui.px_per_sec) as Time;
                t.ui.scroll = (t.ui.scroll + dt).max(0);
            }
        }
        // For file drops (U-ED-16): the track and time under the pointer.
        self.pointer_track = latest.filter(|&p| rows_area.contains(p)).and_then(|p| {
            let (x, y) = local(p);
            row_at(y).map(|tr| (tr, raw_time(x)))
        });
        let drop = resp.dnd_release_payload::<Id>().zip(pos);
        if let Some(tid) = target_click {
            let ui_state = &mut t.ui;
            if ui_state.targets.is_empty() {
                ui_state.targets = rows.clone();
            }
            if let Some(k) = ui_state.targets.iter().position(|&x| x == tid) {
                ui_state.targets.remove(k);
            } else {
                ui_state.targets.push(tid);
            }
            if ui_state.targets.len() == rows.len() {
                ui_state.targets.clear();
            }
        }
        let menu_hit = self.menu.as_ref().map(|m| m.0);
        if !menu_open {
            self.menu = None;
        }
        // U-ED-23c: files dropped with no pointer position go where the pointer first shows up.
        if latest.is_some()
            && let Some((pid, ids)) = self.drop_pending.take()
            && pid == t.id
            && let Some((track, at)) = self.pointer_track
        {
            self.drop_media(&ids, Some(track), at);
        }
        self.apply(ops);
        self.edit(toggles);
        if let Some(a) = chosen {
            self.run_action(a);
        }
        if let (
            Some(effect),
            Some(Hit::Clip { track_id, time, .. } | Hit::Empty { track_id, time }),
        ) = (transition, menu_hit)
        {
            let ops = crate::transition_at(&project, seq.id, track_id, time, &effect);
            if ops.is_empty() {
                self.status = "no place for a transition there".into();
            }
            self.apply(ops);
        }
        if let Some((media, p)) = drop {
            let (x, y) = local(p);
            self.drop_media(&[*media], row_at(y), raw_time(x));
        }
    }

    /// Bin or file drop (U-APP-4, U-ED-16): one batch of `AddClip` overwrite on `track`, the
    /// first at the snapped `at`, each next where the previous ends; linked audio when the media
    /// has both.
    pub(super) fn drop_media(&mut self, media: &[Id], track: Option<Id>, at: Time) {
        let Some(t) = self.tab() else { return };
        let p = &t.project;
        let Some((seq, track)) = track.and_then(|id| p.track(id)) else {
            return;
        };
        let audio = seq
            .tracks
            .iter()
            .find(|x| x.kind == TrackKind::Audio && !x.locked)
            .map(|x| x.id);
        let mut start = crate::snap(at, p, &t.ui, &[], tolerance(&t.ui));
        let mut cmds = Vec::new();
        for &media_id in media {
            let Some(m) = p.media.iter().find(|m| m.id == media_id) else {
                continue;
            };
            let duration = m.spec.duration.unwrap_or(5 * adit_core::FLICKS_PER_SECOND);
            let linked = (track.kind == TrackKind::Video && m.spec.audio.is_some())
                .then_some(audio)
                .flatten();
            cmds.push(Command::AddClip {
                track_id: track.id,
                linked_track_id: linked,
                source: ClipSource::Media { media_id },
                start,
                duration,
                source_in: 0,
                mode: PlaceMode::Overwrite,
                name: None,
            });
            start += duration;
        }
        self.edit(cmds);
    }
}

/// The hit with its time replaced (by the snapped pointer time).
fn with_time(h: Hit, t: Time) -> Hit {
    match h {
        Hit::Ruler { .. } => Hit::Ruler { time: t },
        Hit::Clip {
            clip_id,
            track_id,
            part,
            ..
        } => Hit::Clip {
            clip_id,
            track_id,
            part,
            time: t,
        },
        Hit::Empty { track_id, .. } => Hit::Empty { track_id, time: t },
        h => h,
    }
}

fn hit_time(h: &Hit) -> Time {
    match *h {
        Hit::Ruler { time } | Hit::Clip { time, .. } | Hit::Empty { time, .. } => time,
        _ => 0,
    }
}

fn hit_track(h: &Hit) -> Option<Id> {
    match *h {
        Hit::Clip { track_id, .. } | Hit::Empty { track_id, .. } | Hit::Header { track_id } => {
            Some(track_id)
        }
        _ => None,
    }
}

/// Outline of what the gesture would do; nothing is applied before release (U-APP-4).
#[allow(clippy::too_many_arguments)]
fn preview(
    p: &egui::Painter,
    project: &Project,
    ui: &UiState,
    from: Hit,
    to: Time,
    to_track: Option<Id>,
    rows: &[Id],
    x_of: &dyn Fn(Time) -> f32,
    row_top: &dyn Fn(usize) -> f32,
) {
    let row = |id: Id| rows.iter().position(|&r| r == id);
    let stroke = Stroke::new(1.5, Color32::from_rgb(255, 220, 80));
    let outline = |track: Id, a: Time, b: Time| {
        if let Some(i) = row(track) {
            let y = row_top(i);
            let r =
                Rect::from_x_y_ranges(x_of(a.min(b))..=x_of(a.max(b)), y + 2.0..=y + TRACK - 2.0);
            p.rect_stroke(r, 3.0, stroke, egui::StrokeKind::Inside);
        }
    };
    match from {
        Hit::Clip {
            clip_id,
            track_id,
            part,
            time,
        } => {
            let Some((_, _, c)) = project.clip(clip_id) else {
                return;
            };
            let dt = to - time;
            match (ui.tool, part) {
                (Tool::Select | Tool::Ripple | Tool::Roll, ClipPart::StartEdge) => {
                    outline(track_id, to, c.end())
                }
                (
                    Tool::Select | Tool::Ripple | Tool::Roll | Tool::RateStretch,
                    ClipPart::EndEdge,
                ) => outline(track_id, c.start, to),
                (Tool::Select, ClipPart::Body) => {
                    outline(to_track.unwrap_or(track_id), c.start + dt, c.end() + dt)
                }
                (Tool::Slide, _) => outline(track_id, c.start + dt, c.end() + dt),
                (Tool::Wand, _) => outline(track_id, time, to),
                _ => outline(track_id, c.start, c.end()),
            }
        }
        Hit::Empty { track_id, time } if ui.tool == Tool::Wand => outline(track_id, time, to),
        _ => {}
    }
}
