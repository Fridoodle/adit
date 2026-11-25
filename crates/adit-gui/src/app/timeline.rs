//! The Timeline panel: drawing, mouse gestures and bin drops (U-APP-1, U-APP-4).

use super::{App, S, timecode};
use crate::{ClipPart, Drag, Hit, TimelineLayout, Tool};
use adit_core::{ClipSource, Command, Id, PlaceMode, Project, Time, TrackKind};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, pos2, vec2};

const HEADER: f32 = 150.0;
const RULER: f32 = 24.0;
const TRACK: f32 = 44.0;
const EDGE: f32 = 6.0;

fn layout(ui: &crate::UiState) -> TimelineLayout {
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

impl App {
    pub(super) fn timeline_panel(&mut self, ui: &mut egui::Ui) {
        let mut switch = None;
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
                let snap = if t.ui.snap { "Snap on" } else { "Snap off" };
                ui.weak(snap);
            }
        });
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
        let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        t.ui.view_width = (rect.width() - HEADER).max(1.0);
        let lay = layout(&t.ui);
        let rows = lay.rows(&project, seq.id);
        let rows_rect = Rect::from_min_max(pos2(rect.left(), rect.top() + RULER), rect.max);
        let max_scroll = (rows.len() as f32 * TRACK - rows_rect.height()).max(0.0);
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
        for (i, &tid) in rows.iter().enumerate() {
            let Some(track) = seq.tracks.iter().find(|x| x.id == tid) else {
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
                let offline = match c.source {
                    ClipSource::Media { media_id } => project
                        .media
                        .iter()
                        .find(|m| m.id == media_id)
                        .is_some_and(|m| self.offline.get(&m.spec.path) == Some(&Some(false))),
                    _ => false,
                };
                let r = Rect::from_x_y_ranges(x0..=x1.max(x0 + 1.0), y + 3.0..=y + TRACK - 3.0);
                let base = match (&c.source, track.kind) {
                    _ if offline => Color32::from_rgb(150, 50, 50),
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
                let r = Rect::from_x_y_ranges(x_of(a)..=x_of(b), y + 3.0..=y + 15.0);
                area_painter.rect_filled(
                    r,
                    2.0,
                    Color32::from_rgba_unmultiplied(255, 255, 255, 90),
                );
                area_painter.line_segment(
                    [r.left_bottom(), r.right_top()],
                    Stroke::new(1.0, Color32::BLACK),
                );
            }
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

        // Mouse gestures (U-APP-4): hit on press, snap, `drag` on release.
        let (mods, press_pos, pos) = ui.input(|i| {
            (
                i.modifiers,
                i.pointer.press_origin(),
                i.pointer.interact_pos(),
            )
        });
        let row_at = |y: f32| -> Option<Id> {
            let i = ((y - RULER) / TRACK).floor();
            (y >= RULER && i >= 0.0)
                .then(|| rows.get(i as usize).copied())
                .flatten()
        };
        // U-FIX-4: pointer times are snapped for clicks and drags alike; U-FIX-3: not for Hand.
        let pointer_time = |x: f32, from: &Hit, ui_state: &crate::UiState| {
            let raw = lay.x_to_time(x.max(HEADER)).max(0);
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
            let tol = (8.0 * S / ui_state.px_per_sec).round() as Time;
            crate::snap(raw, &project, ui_state, &exclude, tol)
        };
        let pressed = |p: Pos2, ui_state: &crate::UiState| {
            let (x, y) = local(p);
            let h = lay.hit(&project, seq.id, x, y);
            with_time(h, pointer_time(x, &h, ui_state))
        };
        let mut ops = Vec::new();
        let finish = |from: Hit, p: Pos2, ui_state: &mut crate::UiState, click: bool| {
            let (x, y) = local(p);
            let (to_time, to_track) = if click {
                (hit_time(&from), hit_track(&from))
            } else {
                (pointer_time(x, &from, ui_state), row_at(y))
            };
            crate::drag(
                Drag {
                    from,
                    to_time,
                    to_track,
                    shift: mods.shift,
                    alt: mods.alt,
                },
                &project,
                ui_state,
            )
        };
        if resp.drag_started()
            && let Some(p) = press_pos
        {
            self.press = Some(pressed(p, &t.ui));
        }
        if let (Some(from), Some(p)) = (self.press, pos) {
            if resp.dragged() {
                if matches!(from, Hit::Ruler { .. }) {
                    // Live scrubbing: a ruler drag only moves the playhead.
                    ops.extend(finish(from, p, &mut t.ui, false));
                } else {
                    let (x, y) = local(p);
                    let to = pointer_time(x, &from, &t.ui);
                    preview(
                        &area_painter,
                        &project,
                        &t.ui,
                        from,
                        to,
                        row_at(y),
                        &rows,
                        &|tt| x_of(tt),
                        &row_top,
                    );
                }
            }
            if resp.drag_stopped() {
                self.press = None;
                ops.extend(finish(from, p, &mut t.ui, false));
            }
        }
        if resp.clicked()
            && let Some(p) = press_pos.or(pos)
        {
            let from = pressed(p, &t.ui);
            ops.extend(finish(from, p, &mut t.ui, true));
        }
        if resp.hovered() {
            let d = ui.input(|i| i.smooth_scroll_delta());
            if mods.command && d.y != 0.0 {
                let f = if d.y > 0.0 { 1.1 } else { 1.0 / 1.1 };
                t.ui.px_per_sec = (t.ui.px_per_sec * f).clamp(1.0, 2000.0);
            } else if d.y != 0.0 && max_scroll > 0.0 && !mods.shift {
                self.timeline_scroll_y = (sy - d.y).clamp(0.0, max_scroll);
            } else if d.x + d.y != 0.0 {
                let dt = (-(d.x + d.y) as f64 * S / t.ui.px_per_sec) as Time;
                t.ui.scroll = (t.ui.scroll + dt).max(0);
            }
        }
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
        self.apply(ops);
        self.edit(toggles);
        if let Some((media, p)) = drop {
            let (x, y) = local(p);
            self.drop_media(*media, row_at(y), lay.x_to_time(x.max(HEADER)).max(0));
        }
    }

    /// Bin to timeline drop: `AddClip` overwrite, linked audio when the media has both (U-APP-4).
    fn drop_media(&mut self, media_id: Id, track: Option<Id>, at: Time) {
        let Some(t) = self.tab() else { return };
        let p = &t.project;
        let (Some(track_id), Some(m)) = (track, p.media.iter().find(|m| m.id == media_id)) else {
            return;
        };
        let Some((seq, track)) = p.track(track_id) else {
            return;
        };
        let tol = (8.0 * S / t.ui.px_per_sec).round() as Time;
        let start = crate::snap(at, p, &t.ui, &[], tol);
        let duration = m.spec.duration.unwrap_or(5 * adit_core::FLICKS_PER_SECOND);
        let linked = (track.kind == TrackKind::Video && m.spec.audio.is_some())
            .then(|| {
                seq.tracks
                    .iter()
                    .find(|x| x.kind == TrackKind::Audio && !x.locked)
                    .map(|x| x.id)
            })
            .flatten();
        let cmd = Command::AddClip {
            track_id,
            linked_track_id: linked,
            source: ClipSource::Media { media_id },
            start,
            duration,
            source_in: 0,
            mode: PlaceMode::Overwrite,
            name: None,
        };
        self.edit(vec![cmd]);
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
    ui: &crate::UiState,
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
