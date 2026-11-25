//! Pure editing logic behind the window (GUI.md U-STATE .. U-DRAG-*). No egui widgets, no IO.

use crate::{
    Action, ClipPart, Drag, Hit, KeyChord, Op, SourceState, TimelineLayout, Tool, UiState,
};
use adit_core::{
    Clip, ClipSource, Command, Document, Easing, Edge, FLICKS_PER_SECOND, Id, MAX_TIME,
    ParamTarget, PasteItem, PlaceMode, Project, Sequence, Time, Track, TrackKind, TrimMode,
    frame_to_time, time_to_frame,
};
use eframe::egui::Key;

const S: Time = FLICKS_PER_SECOND;

pub(crate) fn new_state(sequence_id: Id) -> UiState {
    UiState {
        sequence_id,
        playhead: 0,
        selection: Vec::new(),
        mark_in: None,
        mark_out: None,
        tool: Tool::Select,
        snap: true,
        linked_selection: true,
        playing: false,
        shuttle: 0,
        targets: Vec::new(),
        source: None,
        px_per_sec: 100.0,
        scroll: 0,
        view_width: 1000.0,
    }
}

// ------------------------------------------------------------------------------------ helpers

fn frame(s: &Sequence, t: Time) -> i64 {
    time_to_frame(t, s.fps)
}

fn at(s: &Sequence, n: i64) -> Time {
    frame_to_time(n, s.fps)
}

/// Nearest frame start; ties go to the earlier frame.
fn round_frame(s: &Sequence, t: Time) -> Time {
    let (a, b) = (at(s, frame(s, t)), at(s, frame(s, t) + 1));
    if t - a <= b - t { a } else { b }
}

/// Unlocked tracks of the sequence, restricted to `ui.targets` when that is non-empty.
fn targets<'a>(s: &'a Sequence, ui: &UiState) -> Vec<&'a Track> {
    s.tracks
        .iter()
        .filter(|t| !t.locked && (ui.targets.is_empty() || ui.targets.contains(&t.id)))
        .collect()
}

fn all_clips(s: &Sequence) -> impl Iterator<Item = &Clip> {
    s.tracks.iter().flat_map(|t| &t.clips)
}

fn edit_points(s: &Sequence) -> Vec<Time> {
    let mut v = vec![0];
    v.extend(all_clips(s).flat_map(|c| [c.start, c.end()]));
    v
}

/// Clips of `ids` plus (when asked) their linked sets (SPEC R6), in track then time order.
/// Ids that do not exist are dropped; so are clips on locked tracks (U-FIX-1).
fn with_links(s: &Sequence, ids: &[Id], linked: bool) -> Vec<Id> {
    linked_set(s, ids, linked)
        .into_iter()
        .filter(|&(_, locked)| !locked)
        .map(|(id, _)| id)
        .collect()
}

/// `with_links` before dropping locked clips: `(id, on a locked track)`.
fn linked_set(s: &Sequence, ids: &[Id], linked: bool) -> Vec<(Id, bool)> {
    let links: Vec<Id> = if linked {
        all_clips(s)
            .filter(|c| ids.contains(&c.id))
            .filter_map(|c| c.link)
            .collect()
    } else {
        Vec::new()
    };
    s.tracks
        .iter()
        .flat_map(|t| t.clips.iter().map(move |c| (c, t.locked)))
        .filter(|(c, _)| ids.contains(&c.id) || c.link.is_some_and(|l| links.contains(&l)))
        .map(|(c, locked)| (c.id, locked))
        .collect()
}

fn effective(s: &Sequence, ui: &UiState) -> Vec<Id> {
    with_links(s, &ui.selection, ui.linked_selection)
}

/// Selected ids that still exist in the sequence, in selection order.
fn existing(s: &Sequence, ui: &UiState) -> Vec<Id> {
    ui.selection
        .iter()
        .copied()
        .filter(|id| all_clips(s).any(|c| c.id == *id))
        .collect()
}

fn under(c: &Clip, t: Time) -> bool {
    c.start <= t && t < c.end()
}

fn strictly_inside(c: &Clip, t: Time) -> bool {
    c.start < t && t < c.end()
}

fn stop(ui: &mut UiState) {
    ui.playing = false;
    ui.shuttle = 0;
}

fn clips_by_id<'a>(s: &'a Sequence, ids: &[Id]) -> Vec<&'a Clip> {
    all_clips(s).filter(|c| ids.contains(&c.id)).collect()
}

fn edit(cmds: Vec<Command>) -> Vec<Op> {
    if cmds.is_empty() {
        vec![]
    } else {
        vec![Op::Edit(cmds)]
    }
}

fn zoom(ui: &mut UiState, out: bool) {
    ui.px_per_sec = if out {
        (ui.px_per_sec / 1.5).max(1.0)
    } else {
        (ui.px_per_sec * 1.5).min(2000.0)
    };
}

// ------------------------------------------------------------------------------------ perform

pub(crate) fn perform(action: Action, project: &Project, ui: &mut UiState) -> Vec<Op> {
    let Some(s) = project.sequence(ui.sequence_id) else {
        return vec![];
    };
    let ph = ui.playhead;
    let dur = s.duration();
    match action {
        Action::Copy => {
            if !effective(s, ui).is_empty() {
                return vec![Op::Copy];
            }
        }
        Action::Cut => {
            if !effective(s, ui).is_empty() {
                let mut ops = vec![Op::Copy];
                ops.extend(perform(Action::Delete, project, ui));
                return ops;
            }
        }
        Action::Paste => return vec![Op::Paste],
        Action::Unlink => {
            let clip_ids: Vec<Id> = clips_by_id(s, &effective(s, ui))
                .into_iter()
                .filter(|c| c.link.is_some())
                .map(|c| c.id)
                .collect();
            if !clip_ids.is_empty() {
                return edit(vec![Command::UnlinkClips { clip_ids }]);
            }
        }
        Action::SetLabel(n) => {
            if n <= 8 {
                return edit(
                    effective(s, ui)
                        .into_iter()
                        .map(|clip_id| Command::SetClip {
                            clip_id,
                            name: None,
                            enabled: None,
                            reverse: None,
                            blend: None,
                            audio_filter: None,
                            label: Some(n),
                        })
                        .collect(),
                );
            }
        }
        Action::CloseGap { track_id, time } => {
            let ok = s.tracks.iter().any(|t| {
                t.id == track_id
                    && !t.locked
                    && !t.clips.iter().any(|c| under(c, time))
                    && t.clips.iter().any(|c| c.start > time)
            });
            if ok {
                return edit(vec![Command::CloseGap { track_id, time }]);
            }
        }
        Action::SetTool(t) => ui.tool = t,
        Action::PlayPause => {
            if ui.playing {
                stop(ui);
            } else {
                ui.playing = true;
                ui.shuttle = 1;
                if ph >= dur {
                    ui.playhead = 0;
                }
            }
        }
        Action::ShuttleRight => {
            ui.shuttle = if ui.shuttle <= 0 {
                1
            } else {
                (ui.shuttle * 2).min(8)
            };
            ui.playing = true;
        }
        Action::ShuttleLeft => {
            ui.shuttle = if ui.shuttle >= 0 {
                -1
            } else {
                (ui.shuttle * 2).max(-8)
            };
            ui.playing = true;
        }
        Action::ShuttleStop => stop(ui),
        Action::Step(n) => {
            stop(ui);
            let end = frame(s, dur);
            ui.playhead = at(s, frame(s, ph).saturating_add(n).clamp(0, end));
        }
        Action::GoToStart => {
            stop(ui);
            ui.playhead = 0;
        }
        Action::GoToEnd => {
            stop(ui);
            ui.playhead = at(s, frame(s, dur));
        }
        Action::NextEdit | Action::PrevEdit => {
            stop(ui);
            let pts = edit_points(s);
            let p = if action == Action::NextEdit {
                pts.into_iter().filter(|&t| t > ph).min()
            } else {
                pts.into_iter().filter(|&t| t < ph).max()
            };
            ui.playhead = p.unwrap_or(ph);
        }
        Action::MarkIn => {
            ui.mark_in = Some(ph);
            if ui.mark_out.is_some_and(|o| o <= ph) {
                ui.mark_out = None;
            }
        }
        Action::MarkOut => {
            ui.mark_out = Some(ph);
            if ui.mark_in.is_some_and(|i| i >= ph) {
                ui.mark_in = None;
            }
        }
        Action::MarkClip => {
            let sel = effective(s, ui);
            let clips: Vec<&Clip> = if sel.is_empty() {
                targets(s, ui)
                    .into_iter()
                    .flat_map(|t| &t.clips)
                    .filter(|c| under(c, ph))
                    .collect()
            } else {
                clips_by_id(s, &sel)
            };
            if let (Some(a), Some(b)) = (
                clips.iter().map(|c| c.start).min(),
                clips.iter().map(|c| c.end()).max(),
            ) {
                ui.mark_in = Some(a);
                ui.mark_out = Some(b);
            }
        }
        Action::ClearIn => ui.mark_in = None,
        Action::ClearOut => ui.mark_out = None,
        Action::ClearInOut => {
            ui.mark_in = None;
            ui.mark_out = None;
        }
        Action::GoToIn => ui.playhead = ui.mark_in.unwrap_or(ph),
        Action::GoToOut => ui.playhead = ui.mark_out.unwrap_or(ph),
        Action::SelectAtPlayhead => {
            ui.selection = targets(s, ui)
                .into_iter()
                .flat_map(|t| &t.clips)
                .filter(|c| under(c, ph))
                .map(|c| c.id)
                .collect();
        }
        Action::SelectAll => ui.selection = all_clips(s).map(|c| c.id).collect(),
        Action::DeselectAll => ui.selection.clear(),
        Action::ToggleSnap => ui.snap = !ui.snap,
        Action::ToggleLinkedSelection => ui.linked_selection = !ui.linked_selection,
        Action::MatchFrame => {
            let hit = targets(s, ui)
                .into_iter()
                .rev()
                .filter(|t| t.kind == TrackKind::Video)
                .find_map(|t| {
                    t.clips.iter().find_map(|c| match c.source {
                        ClipSource::Media { media_id } if under(c, ph) => {
                            Some((media_id, c.source_time(ph)))
                        }
                        _ => None,
                    })
                });
            if let Some((media_id, playhead)) = hit {
                ui.source = Some(SourceState {
                    media_id,
                    playhead,
                    mark_in: None,
                    mark_out: None,
                });
            }
        }
        Action::ZoomIn => zoom(ui, false),
        Action::ZoomOut => zoom(ui, true),
        Action::ZoomFit => {
            let secs = (dur as f64 / S as f64).max(1.0);
            ui.px_per_sec = (ui.view_width as f64 / secs).clamp(1.0, 2000.0);
            ui.scroll = 0;
        }
        Action::AddEdit => {
            let sel = effective(s, ui);
            if !existing(s, ui).is_empty() {
                return edit(
                    clips_by_id(s, &sel)
                        .into_iter()
                        .filter(|c| strictly_inside(c, ph))
                        .map(|c| Command::SplitClip {
                            clip_id: c.id,
                            time: ph,
                            include_linked: false,
                        })
                        .collect(),
                );
            }
            let ts = targets(s, ui);
            if ts
                .iter()
                .any(|t| t.clips.iter().any(|c| strictly_inside(c, ph)))
            {
                return edit(vec![Command::SplitAt {
                    sequence_id: s.id,
                    time: ph,
                    track_ids: Some(ts.iter().map(|t| t.id).collect()),
                }]);
            }
        }
        Action::AddEditAll => {
            let any = s
                .tracks
                .iter()
                .filter(|t| !t.locked)
                .any(|t| t.clips.iter().any(|c| strictly_inside(c, ph)));
            if any {
                return edit(vec![Command::SplitAt {
                    sequence_id: s.id,
                    time: ph,
                    track_ids: None,
                }]);
            }
        }
        Action::RippleTrimPrev => {
            if let Some(a) = edit_points(s).into_iter().filter(|&t| t < ph).max() {
                ui.playhead = a;
                return edit(vec![Command::ExtractRange {
                    sequence_id: s.id,
                    start: a,
                    end: ph,
                    track_ids: None,
                }]);
            }
        }
        Action::RippleTrimNext => {
            if let Some(b) = edit_points(s).into_iter().filter(|&t| t > ph).min() {
                return edit(vec![Command::ExtractRange {
                    sequence_id: s.id,
                    start: ph,
                    end: b,
                    track_ids: None,
                }]);
            }
        }
        Action::Delete | Action::RippleDelete => {
            let all = linked_set(s, &existing(s, ui), ui.linked_selection);
            let mut clip_ids: Vec<Id> = existing(s, ui)
                .into_iter()
                .filter(|id| all.contains(&(*id, false)))
                .collect();
            if clip_ids.is_empty() {
                return vec![];
            }
            ui.selection.clear();
            let mut include_linked = ui.linked_selection;
            // A linked partner on a locked track would make the core reject the batch: name the
            // unlocked clips explicitly instead (U-FIX-14).
            if include_linked
                && linked_set(s, &clip_ids, true)
                    .iter()
                    .any(|&(_, locked)| locked)
            {
                clip_ids = with_links(s, &clip_ids, true);
                include_linked = false;
            }
            return edit(vec![if action == Action::Delete {
                Command::RemoveClips {
                    clip_ids,
                    include_linked,
                }
            } else {
                Command::RippleDeleteClips {
                    clip_ids,
                    include_linked,
                }
            }]);
        }
        Action::Lift | Action::Extract => {
            let (Some(start), Some(end)) = (ui.mark_in, ui.mark_out) else {
                return vec![];
            };
            let track_ids = (!ui.targets.is_empty()).then(|| ui.targets.clone());
            ui.playhead = start;
            ui.mark_in = None;
            ui.mark_out = None;
            let sequence_id = s.id;
            return edit(vec![if action == Action::Lift {
                Command::LiftRange {
                    sequence_id,
                    start,
                    end,
                    track_ids,
                }
            } else {
                Command::ExtractRange {
                    sequence_id,
                    start,
                    end,
                    track_ids,
                }
            }]);
        }
        Action::Insert | Action::Overwrite => return insert(action, project, s, ui),
        Action::Nudge(n) => {
            let ids = effective(s, ui);
            return edit(moves(s, &ids, n.saturating_mul(at(s, 1)), None));
        }
        Action::ToggleEnable => {
            let clips = clips_by_id(s, &effective(s, ui));
            let enabled = !clips.iter().all(|c| c.enabled);
            return edit(
                clips
                    .iter()
                    .map(|c| Command::SetClip {
                        clip_id: c.id,
                        name: None,
                        enabled: Some(enabled),
                        reverse: None,
                        blend: None,
                        audio_filter: None,
                        label: None,
                    })
                    .collect(),
            );
        }
        Action::ToggleLink => {
            let clip_ids = existing(s, ui);
            let first = clips_by_id(s, &clip_ids).first().and_then(|c| c.link);
            if first.is_some() && clips_by_id(s, &clip_ids).iter().all(|c| c.link == first) {
                return edit(vec![Command::UnlinkClips { clip_ids }]);
            } else if clip_ids.len() >= 2 {
                return edit(vec![Command::LinkClips { clip_ids }]);
            }
        }
        Action::Group => {
            let clip_ids = existing(s, ui);
            if clip_ids.len() >= 2 {
                return edit(vec![Command::GroupClips { clip_ids }]);
            }
        }
        Action::Ungroup => {
            let clip_ids = existing(s, ui);
            if !clip_ids.is_empty() {
                return edit(vec![Command::UngroupClips { clip_ids }]);
            }
        }
        Action::ApplyVideoTransition => {
            return edit(transitions(s, ui, TrackKind::Video, "cross_dissolve"));
        }
        Action::ApplyAudioTransition => {
            return edit(transitions(s, ui, TrackKind::Audio, "crossfade"));
        }
        Action::ApplyDefaultTransitions => {
            let mut v = transitions(s, ui, TrackKind::Video, "cross_dissolve");
            v.extend(transitions(s, ui, TrackKind::Audio, "crossfade"));
            return edit(v);
        }
        Action::AddMarker => {
            return edit(vec![Command::AddMarker {
                sequence_id: s.id,
                time: ph,
                duration: 0,
                name: String::new(),
                note: String::new(),
            }]);
        }
        Action::Undo => return vec![Op::Undo],
        Action::Redo => return vec![Op::Redo],
        Action::Save => return vec![Op::Save],
        Action::Import => return vec![Op::Import],
        Action::Export => return vec![Op::Export],
        Action::WandPrompt => return vec![Op::WandPrompt],
        Action::SpeedDialog if !ui.selection.is_empty() => return vec![Op::SpeedDialog],
        Action::GainDialog if !ui.selection.is_empty() => return vec![Op::GainDialog],
        Action::SpeedDialog | Action::GainDialog => {}
    }
    vec![]
}

fn insert(action: Action, project: &Project, s: &Sequence, ui: &mut UiState) -> Vec<Op> {
    let Some(src) = &ui.source else {
        return vec![];
    };
    let Some(media) = project.media.iter().find(|m| m.id == src.media_id) else {
        return vec![];
    };
    let src_in = src.mark_in.unwrap_or(0);
    let src_out = src
        .mark_out
        .or(media.spec.duration)
        .unwrap_or(src_in + 5 * S);
    let duration = src_out - src_in;
    if duration <= 0 {
        return vec![];
    }
    let start = ui.mark_in.unwrap_or(ui.playhead);
    let first = |kind| {
        targets(s, ui)
            .into_iter()
            .find(|t| t.kind == kind)
            .map(|t| t.id)
    };
    let video = first(TrackKind::Video).filter(|_| media.spec.video.is_some());
    let audio = first(TrackKind::Audio).filter(|_| media.spec.audio.is_some());
    let (track_id, linked_track_id) = match (video, audio) {
        (Some(v), a) => (v, a),
        (None, Some(a)) => (a, None),
        (None, None) => return vec![],
    };
    let cmd = Command::AddClip {
        track_id,
        linked_track_id,
        source: ClipSource::Media {
            media_id: src.media_id,
        },
        start,
        duration,
        source_in: src_in,
        mode: if action == Action::Insert {
            PlaceMode::Insert
        } else {
            PlaceMode::Overwrite
        },
        name: None,
    };
    ui.playhead = at(s, frame(s, start + duration));
    ui.selection.clear();
    ui.mark_in = None;
    ui.mark_out = None;
    vec![Op::Edit(vec![cmd])]
}

/// One `MoveClip` per clip, ordered so a moving clip never overwrites another moving clip.
/// Nothing when there is no change or any clip would start before 0.
fn moves(s: &Sequence, ids: &[Id], dt: Time, track: Option<Id>) -> Vec<Command> {
    let mut clips = clips_by_id(s, ids);
    if (dt == 0 && track.is_none()) || clips.iter().any(|c| c.start + dt < 0) {
        return vec![];
    }
    clips.sort_by_key(|c| if dt > 0 { -c.start } else { c.start });
    clips
        .into_iter()
        .map(|c| Command::MoveClip {
            clip_id: c.id,
            start: c.start + dt,
            track_id: track,
            mode: PlaceMode::Overwrite,
            include_linked: false,
        })
        .collect()
}

fn transitions(s: &Sequence, ui: &UiState, kind: TrackKind, effect: &str) -> Vec<Command> {
    let ph = ui.playhead;
    targets(s, ui)
        .into_iter()
        .filter(|t| t.kind == kind)
        .filter_map(|t| {
            let left = t.clips.iter().find(|c| c.end() == ph);
            let right = t.clips.iter().find(|c| c.start == ph);
            let duration = match (left, right) {
                (Some(l), Some(r)) => S.min(2 * l.duration).min(2 * r.duration),
                (Some(c), None) | (None, Some(c)) => S.min(c.duration),
                (None, None) => return None,
            };
            Some(Command::AddTransition {
                track_id: t.id,
                left: left.map(|c| c.id),
                right: right.map(|c| c.id),
                effect: effect.into(),
                duration,
                params: Default::default(),
            })
        })
        .collect()
}

// ------------------------------------------------------------------------------------- keymap

pub(crate) fn default_keymap() -> Vec<(KeyChord, Action)> {
    use Action::*;
    use Tool::*;
    let k = |key, ctrl, shift, alt, a| {
        (
            KeyChord {
                key,
                ctrl,
                shift,
                alt,
            },
            a,
        )
    };
    let p = |key, a| k(key, false, false, false, a);
    let sh = |key, a| k(key, false, true, false, a);
    let c = |key, a| k(key, true, false, false, a);
    let cs = |key, a| k(key, true, true, false, a);
    let al = |key, a| k(key, false, false, true, a);
    let als = |key, a| k(key, false, true, true, a);
    vec![
        p(Key::V, SetTool(Select)),
        p(Key::A, SetTool(TrackSelect)),
        p(Key::B, SetTool(Ripple)),
        p(Key::N, SetTool(Roll)),
        p(Key::R, SetTool(RateStretch)),
        p(Key::C, SetTool(Razor)),
        p(Key::Y, SetTool(Slip)),
        p(Key::U, SetTool(Slide)),
        p(Key::P, SetTool(Pen)),
        p(Key::H, SetTool(Hand)),
        p(Key::Z, SetTool(Zoom)),
        p(Key::T, SetTool(Type)),
        sh(Key::W, SetTool(Wand)),
        cs(Key::W, WandPrompt),
        p(Key::Space, PlayPause),
        p(Key::K, ShuttleStop),
        p(Key::J, ShuttleLeft),
        p(Key::L, ShuttleRight),
        p(Key::ArrowLeft, Step(-1)),
        p(Key::ArrowRight, Step(1)),
        sh(Key::ArrowLeft, Step(-5)),
        sh(Key::ArrowRight, Step(5)),
        p(Key::Home, GoToStart),
        p(Key::End, GoToEnd),
        p(Key::ArrowUp, PrevEdit),
        p(Key::ArrowDown, NextEdit),
        p(Key::I, MarkIn),
        p(Key::O, MarkOut),
        p(Key::X, MarkClip),
        cs(Key::X, ClearInOut),
        cs(Key::I, ClearIn),
        cs(Key::O, ClearOut),
        sh(Key::I, GoToIn),
        sh(Key::O, GoToOut),
        c(Key::K, AddEdit),
        cs(Key::K, AddEditAll),
        p(Key::Q, RippleTrimPrev),
        p(Key::W, RippleTrimNext),
        p(Key::Delete, Delete),
        c(Key::C, Copy),
        c(Key::X, Cut),
        c(Key::V, Paste),
        p(Key::Backspace, Delete),
        sh(Key::Delete, RippleDelete),
        sh(Key::Backspace, RippleDelete),
        p(Key::Semicolon, Lift),
        p(Key::Quote, Extract),
        p(Key::Comma, Insert),
        p(Key::Period, Overwrite),
        p(Key::D, SelectAtPlayhead),
        c(Key::A, SelectAll),
        cs(Key::A, DeselectAll),
        sh(Key::E, ToggleEnable),
        al(Key::ArrowLeft, Nudge(-1)),
        al(Key::ArrowRight, Nudge(1)),
        als(Key::ArrowLeft, Nudge(-5)),
        als(Key::ArrowRight, Nudge(5)),
        c(Key::L, ToggleLink),
        c(Key::G, Group),
        cs(Key::G, Ungroup),
        c(Key::D, ApplyVideoTransition),
        cs(Key::D, ApplyAudioTransition),
        sh(Key::D, ApplyDefaultTransitions),
        p(Key::M, AddMarker),
        p(Key::S, ToggleSnap),
        p(Key::F, MatchFrame),
        p(Key::Equals, ZoomIn),
        p(Key::Minus, ZoomOut),
        p(Key::Backslash, ZoomFit),
        c(Key::Z, Undo),
        cs(Key::Z, Redo),
        c(Key::S, Save),
        c(Key::I, Import),
        c(Key::M, Export),
        c(Key::R, SpeedDialog),
        p(Key::G, GainDialog),
    ]
}

// ---------------------------------------------------------------------------------- geometry

pub(crate) fn time_to_x(l: &TimelineLayout, time: Time) -> f32 {
    (l.header_width as f64 + (time - l.scroll) as f64 * l.px_per_sec / S as f64) as f32
}

pub(crate) fn x_to_time(l: &TimelineLayout, x: f32) -> Time {
    l.scroll + ((x as f64 - l.header_width as f64) * S as f64 / l.px_per_sec).round() as Time
}

pub(crate) fn rows(project: &Project, sequence_id: Id) -> Vec<Id> {
    let Some(s) = project.sequence(sequence_id) else {
        return vec![];
    };
    let of = |k| s.tracks.iter().filter(move |t| t.kind == k);
    of(TrackKind::Video)
        .rev()
        .chain(of(TrackKind::Audio))
        .map(|t| t.id)
        .collect()
}

pub(crate) fn hit(l: &TimelineLayout, project: &Project, sequence_id: Id, x: f32, y: f32) -> Hit {
    if y < 0.0 {
        return Hit::Nothing;
    }
    if y < l.ruler_height {
        return if x >= l.header_width {
            Hit::Ruler {
                time: l.x_to_time(x),
            }
        } else {
            Hit::Nothing
        };
    }
    let rows = rows(project, sequence_id);
    let i = ((y - l.ruler_height) / l.track_height) as usize;
    let (Some(&track_id), Some(s)) = (rows.get(i), project.sequence(sequence_id)) else {
        return Hit::Nothing;
    };
    if x < l.header_width {
        return Hit::Header { track_id };
    }
    let t = l.x_to_time(x);
    let clip = s
        .tracks
        .iter()
        .filter(|t| t.id == track_id)
        .flat_map(|t| &t.clips)
        .find(|c| under(c, t));
    let Some(c) = clip else {
        return Hit::Empty { track_id, time: t };
    };
    let (x0, x1) = (l.time_to_x(c.start), l.time_to_x(c.end()));
    let part = if x1 - x0 < 3.0 * l.edge_px {
        ClipPart::Body
    } else if x - x0 < l.edge_px {
        ClipPart::StartEdge
    } else if x1 - x <= l.edge_px {
        ClipPart::EndEdge
    } else {
        ClipPart::Body
    };
    Hit::Clip {
        clip_id: c.id,
        track_id,
        part,
        time: t,
    }
}

/// Snap targets of U-SNAP, without the clips in `exclude`.
fn candidates(s: &Sequence, ui: &UiState, exclude: &[Id]) -> Vec<Time> {
    let mut cands = vec![0, ui.playhead];
    cands.extend(ui.mark_in);
    cands.extend(ui.mark_out);
    cands.extend(s.markers.iter().map(|m| m.time));
    cands.extend(
        all_clips(s)
            .filter(|c| !exclude.contains(&c.id))
            .flat_map(|c| [c.start, c.end()]),
    );
    cands
}

pub(crate) fn snap(
    time: Time,
    project: &Project,
    ui: &UiState,
    exclude: &[Id],
    tolerance: Time,
) -> Time {
    let Some(s) = project.sequence(ui.sequence_id) else {
        return time;
    };
    let nearest = round_frame(s, time);
    if !ui.snap {
        return nearest;
    }
    candidates(s, ui, exclude)
        .into_iter()
        .filter(|c| (c - time).abs() <= tolerance)
        .min_by_key(|c| ((c - time).abs(), *c))
        .unwrap_or(nearest)
}

// -------------------------------------------------------------------------------------- drags

pub(crate) fn drag(d: Drag, project: &Project, ui: &mut UiState) -> Vec<Op> {
    let Some(s) = project.sequence(ui.sequence_id) else {
        return vec![];
    };
    let linked = ui.linked_selection;
    let (clip_id, track_id, part, from_time) = match d.from {
        Hit::Ruler { time } => {
            ui.playhead = at(s, frame(s, d.to_time));
            if time == d.to_time {
                stop(ui);
                ui.selection.clear();
            }
            return vec![];
        }
        Hit::Header { .. } | Hit::Nothing => return vec![],
        Hit::Clip {
            clip_id,
            track_id,
            part,
            time,
        } => (Some(clip_id), track_id, part, time),
        Hit::Empty { track_id, time } => (None, track_id, ClipPart::Body, time),
    };
    let clip = clip_id.and_then(|id| all_clips(s).find(|c| c.id == id));
    let dt = d.to_time - from_time;
    let click = dt == 0 && !d.to_track.is_some_and(|t| t != track_id);
    let tool = ui.tool;
    match (tool, clip) {
        (Tool::Select, None) => {
            if click {
                ui.selection.clear();
            }
        }
        (Tool::Select, Some(c)) if click => {
            if !d.shift {
                ui.selection = vec![c.id];
            } else if let Some(i) = ui.selection.iter().position(|&i| i == c.id) {
                ui.selection.remove(i);
            } else {
                ui.selection.push(c.id);
            }
        }
        (Tool::Select | Tool::Ripple | Tool::Roll, Some(c)) if part != ClipPart::Body && !click => {
            let mode = match tool {
                Tool::Ripple => TrimMode::Ripple,
                Tool::Roll => TrimMode::Roll,
                _ => TrimMode::Normal,
            };
            let edge = if part == ClipPart::StartEdge {
                Edge::Start
            } else {
                Edge::End
            };
            return edit(vec![Command::TrimClip {
                clip_id: c.id,
                edge,
                to: d.to_time,
                mode,
                include_linked: linked,
            }]);
        }
        (Tool::Select, Some(c)) => {
            let mut sel = ui.selection.clone();
            if !sel.contains(&c.id) {
                sel = vec![c.id];
            }
            let ids = with_links(s, &sel, linked);
            // A track change is only for a lone clip, onto a track of the same kind.
            let kind_of = |id| s.tracks.iter().find(|t| t.id == id).map(|t| t.kind);
            let track = d.to_track.filter(|&t| {
                ids == [c.id]
                    && t != track_id
                    && kind_of(t).is_some()
                    && kind_of(t) == kind_of(track_id)
            });
            let cmds = moves(s, &ids, dt, track);
            if cmds.is_empty() {
                return vec![];
            }
            ui.selection = sel;
            return edit(cmds);
        }
        (Tool::RateStretch, Some(c)) if part == ClipPart::EndEdge && !click => {
            let new = d.to_time - c.start;
            if new <= 0 {
                return vec![];
            }
            let speed = c.speed * c.duration as f64 / new as f64;
            if !(0.01..=100.0).contains(&speed) {
                return vec![];
            }
            return edit(vec![Command::SetClipSpeed {
                clip_id: c.id,
                speed,
                ripple: false,
                include_linked: linked,
            }]);
        }
        (Tool::Razor, Some(c)) if click && strictly_inside(c, from_time) => {
            return edit(vec![if d.shift {
                Command::SplitAt {
                    sequence_id: s.id,
                    time: from_time,
                    track_ids: None,
                }
            } else {
                Command::SplitClip {
                    clip_id: c.id,
                    time: from_time,
                    include_linked: linked,
                }
            }]);
        }
        (Tool::Slip, Some(c)) if part == ClipPart::Body && !click => {
            return edit(vec![Command::SlipClip {
                clip_id: c.id,
                delta: -dt,
                include_linked: linked,
            }]);
        }
        (Tool::Slide, Some(c)) if part == ClipPart::Body && !click => {
            return edit(vec![Command::SlideClip {
                clip_id: c.id,
                delta: dt,
            }]);
        }
        (Tool::TrackSelect, _) if click => {
            let t0 = clip.map_or(from_time, |c| c.start);
            ui.selection = s
                .tracks
                .iter()
                .filter(|t| d.shift || t.id == track_id)
                .flat_map(|t| &t.clips)
                .filter(|c| c.start >= t0)
                .map(|c| c.id)
                .collect();
        }
        (Tool::Hand, _) => ui.scroll = (ui.scroll - dt).max(0),
        (Tool::Zoom, _) if click => zoom(ui, d.alt),
        (Tool::Pen, Some(c)) if click => {
            let local = from_time - c.start;
            return edit(vec![Command::SetKeyframe {
                target: ParamTarget {
                    clip_id: c.id,
                    effect_id: None,
                    name: "opacity".into(),
                },
                time: local,
                value: c.param("opacity", local).unwrap_or(1.0),
                easing: Easing::Linear,
            }]);
        }
        (Tool::Wand, _) => {
            let (a, b) = match (click, clip) {
                (true, Some(c)) => (c.start, c.end()),
                (true, None) => return vec![],
                _ => (from_time.min(d.to_time), from_time.max(d.to_time)),
            };
            ui.mark_in = Some(a);
            ui.mark_out = Some(b);
            ui.selection = s
                .tracks
                .iter()
                .filter(|t| t.id == track_id)
                .flat_map(|t| &t.clips)
                .filter(|c| c.start < b && c.end() > a)
                .map(|c| c.id)
                .collect();
            return vec![Op::WandPrompt];
        }
        _ => {}
    }
    vec![]
}

// ----------------------------------------------------------------------------------- U-ED-*

pub(crate) fn copy(project: &Project, ui: &UiState) -> Vec<crate::ClipboardItem> {
    let Some(s) = project.sequence(ui.sequence_id) else {
        return vec![];
    };
    let ids = effective(s, ui);
    s.tracks
        .iter()
        .flat_map(|t| {
            let mut v: Vec<&Clip> = t.clips.iter().filter(|c| ids.contains(&c.id)).collect();
            v.sort_by_key(|c| c.start);
            v.into_iter().map(move |c| crate::ClipboardItem {
                track_id: t.id,
                clip: c.clone(),
            })
        })
        .collect()
}

pub(crate) fn paste(
    items: &[crate::ClipboardItem],
    project: &Project,
    ui: &mut UiState,
) -> Vec<Op> {
    let Some(s) = project.sequence(ui.sequence_id) else {
        return vec![];
    };
    let (Some(base), Some(end)) = (
        items.iter().map(|i| i.clip.start).min(),
        items.iter().map(|i| i.clip.end()).max(),
    ) else {
        return vec![];
    };
    let ph = ui.playhead;
    let kept: Vec<PasteItem> = items
        .iter()
        .filter(|i| s.tracks.iter().any(|t| t.id == i.track_id && !t.locked))
        .map(|i| {
            let mut clip = i.clip.clone();
            clip.start = ph + (clip.start - base);
            PasteItem {
                track_id: i.track_id,
                clip,
            }
        })
        .collect();
    if kept.is_empty() {
        return vec![];
    }
    ui.playhead = at(s, frame(s, ph + (end - base)));
    ui.selection.clear();
    edit(vec![Command::PasteClips {
        items: kept,
        mode: PlaceMode::Overwrite,
    }])
}

pub(crate) fn transition_at(
    project: &Project,
    sequence_id: Id,
    track_id: Id,
    time: Time,
    effect: &str,
) -> Vec<Op> {
    let Some(t) = project
        .sequence(sequence_id)
        .and_then(|s| s.tracks.iter().find(|t| t.id == track_id))
        .filter(|t| !t.locked)
    else {
        return vec![];
    };
    let add = |left: Option<&Clip>, right: Option<&Clip>, duration: Time| Command::AddTransition {
        track_id,
        left: left.map(|c| c.id),
        right: right.map(|c| c.id),
        effect: effect.into(),
        duration,
        params: Default::default(),
    };
    if let Some(c) = t.clips.iter().find(|c| under(c, time)) {
        let cmd = if time - c.start < c.end() - time {
            match t.clips.iter().find(|p| p.end() == c.start) {
                Some(p) => add(Some(p), Some(c), S.min(2 * p.duration).min(2 * c.duration)),
                None => add(None, Some(c), S.min(c.duration)),
            }
        } else {
            match t.clips.iter().find(|n| n.start == c.end()) {
                Some(n) => add(Some(c), Some(n), S.min(2 * c.duration).min(2 * n.duration)),
                None => add(Some(c), None, S.min(c.duration)),
            }
        };
        return edit(vec![cmd]);
    }
    let l = t
        .clips
        .iter()
        .filter(|c| c.end() <= time)
        .max_by_key(|c| c.end());
    let r = t
        .clips
        .iter()
        .filter(|c| c.start > time)
        .min_by_key(|c| c.start);
    let (Some(l), Some(r)) = (l, r) else {
        return vec![];
    };
    edit(vec![
        Command::CloseGap { track_id, time },
        add(Some(l), Some(r), S.min(2 * l.duration).min(2 * r.duration)),
    ])
}

pub(crate) fn snap_delta(
    delta: Time,
    clips: &[Id],
    project: &Project,
    ui: &UiState,
    tolerance: Time,
) -> Time {
    // U-ED-23h: clamped first, so `abs` and the sums below cannot overflow.
    let delta = delta.clamp(-MAX_TIME, MAX_TIME);
    let tolerance = tolerance.clamp(-MAX_TIME, MAX_TIME);
    let Some(s) = project.sequence(ui.sequence_id) else {
        return delta;
    };
    let rounded = round_frame(s, delta.abs()) * delta.signum();
    if !ui.snap {
        return rounded;
    }
    let cands = candidates(s, ui, clips);
    // (distance, resulting delta) per edge/candidate pair; the smallest pair in range wins.
    clips_by_id(s, clips)
        .into_iter()
        .flat_map(|c| [c.start, c.end()])
        .flat_map(|e| cands.iter().map(move |&k| ((k - e - delta).abs(), k - e)))
        .filter(|&(d, _)| d <= tolerance)
        .min()
        .map_or(rounded, |(_, r)| r)
}

pub(crate) fn preview(d: Drag, project: &Project, ui: &UiState) -> Option<Project> {
    let mut u = ui.clone();
    let mut doc = Document::new(project.clone());
    let mut any = false;
    for op in drag(d, project, &mut u) {
        if let Op::Edit(commands) = op {
            any = true;
            doc.apply(Command::Batch { commands }).ok()?;
        }
    }
    any.then(|| doc.project().clone())
}

pub(crate) fn context_menu(
    hit: Hit,
    has_clipboard: bool,
    project: &Project,
    ui: &mut UiState,
) -> Vec<crate::MenuItem> {
    use crate::MenuItem::*;
    let Some(s) = project.sequence(ui.sequence_id) else {
        return vec![];
    };
    let mut v = Vec::new();
    match hit {
        Hit::Clip { clip_id, .. } => {
            if !s
                .tracks
                .iter()
                .any(|t| !t.locked && t.clips.iter().any(|c| c.id == clip_id))
            {
                return v;
            }
            if !effective(s, ui).contains(&clip_id) {
                ui.selection = vec![clip_id];
            }
            let eff = clips_by_id(s, &effective(s, ui));
            v.extend([Do(Action::Cut), Do(Action::Copy)]);
            if has_clipboard {
                v.push(Do(Action::Paste));
            }
            v.extend([Do(Action::Delete), Do(Action::RippleDelete)]);
            if eff.iter().any(|c| c.link.is_some()) {
                v.push(Do(Action::Unlink));
            }
            if ui.selection.len() >= 2 {
                v.push(Do(Action::Group));
            }
            if eff.iter().any(|c| c.group.is_some()) {
                v.push(Do(Action::Ungroup));
            }
            v.extend([
                Do(Action::ToggleEnable),
                Do(Action::SpeedDialog),
                Do(Action::GainDialog),
                Labels,
                Transitions,
                Do(Action::WandPrompt),
            ]);
        }
        Hit::Empty { track_id, time } => {
            if has_clipboard {
                v.push(Do(Action::Paste));
            }
            let gap = Action::CloseGap { track_id, time };
            if !perform(gap, project, ui).is_empty() {
                v.push(Do(gap));
            }
            if !transition_at(project, ui.sequence_id, track_id, time, "").is_empty() {
                v.push(Transitions);
            }
            v.push(Do(Action::WandPrompt));
        }
        _ => {}
    }
    v
}
