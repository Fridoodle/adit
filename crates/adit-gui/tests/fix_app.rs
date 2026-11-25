//! Window rules added after the GUI review (docs/GUI.md U-FIX-4 .. U-FIX-13), headless through
//! egui_kittest. Expected values come from the rule text and hand arithmetic.
//!
//! Timeline geometry used for mouse input (src/app/timeline.rs today, GUI.md U-GEO): the track
//! header button `V1` sits 4 px right of and below the top-left corner of the V1 row, the header is
//! 150 px wide, and a fresh tab shows 100 px per second from time 0.
//!
//! Widget labels assumed where the contract names none (listed for the contract):
//! - the Effect Controls text-clip content field is the only `MultilineTextInput` on screen while
//!   the Source/Effect Controls panel shows Effect Controls and no dialog is open;
//! - the Speed dialog has exactly one numeric field (`SpinButton`) and a button `OK`;
//! - the sequence selector is a node whose label or value is the current sequence name; after
//!   clicking it a node labelled (or valued) with every other sequence name exists and clicking that
//!   one chooses it (fits a combo box and a row of buttons alike);
//! - the Close Project confirmation has a button `Cancel`;
//! - the quit prompt has the heading `Unsaved changes` (as today).
mod app_common;
use adit_core::{ClipSource, Id, MAX_TIME, MediaSpec, Time, TrackKind};
use adit_gui::{Action, App, Options, Tool};
use adit_media::{Frame, MediaBackend, MediaError, Pattern};
use app_common::*;
use eframe::egui;
use eframe::egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable, by};
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::mpsc::sync_channel;
use std::time::Duration;

fn ctrl() -> egui::Modifiers {
    egui::Modifiers {
        ctrl: true,
        command: true,
        ..Default::default()
    }
}

/// Like `harness`, but 60 steps per simulated second so two clicks make a double-click.
fn harness_fast(s: &adit_mcp::Session) -> Harness<'static, App> {
    let s2 = s.clone();
    let mut h = Harness::builder()
        .with_size(egui::vec2(W, H))
        .with_step_dt(1.0 / 60.0)
        .build_eframe(move |cc| App::new(cc, s2, Options::default()));
    h.run_steps(3);
    h
}

/// Press and release at one point, a frame apart (a click for egui, as a mouse does it).
fn click_at(h: &mut Harness<'static, App>, p: egui::Pos2) {
    h.hover_at(p);
    h.step();
    h.drag_at(p);
    h.step();
    h.drop_at(p);
    h.run_steps(3);
}

/// Screen point of sequence time `secs` in the middle of the V1 row.
fn v1_point(h: &Harness<App>, secs: f64) -> egui::Pos2 {
    let r = h.get_by_label("V1").rect();
    let (left, top) = (r.min.x - 4.0, r.min.y - 4.0);
    egui::pos2(left + 150.0 + (secs * 100.0) as f32, top + 22.0)
}

/// Every text a node shows: its label and, for labels and fields, its value.
fn node_text(n: &egui_kittest::Node<'_>) -> String {
    let a = n.accesskit_node();
    format!(
        "{} {}",
        a.label().unwrap_or_default(),
        a.value().unwrap_or_default()
    )
}

fn any_text(h: &Harness<App>, pred: impl Fn(&str) -> bool) -> bool {
    h.query_all(by()).any(|n| pred(&node_text(&n)))
}

fn ui_set(h: &mut Harness<'static, App>, s: &adit_mcp::Session, v: Value) {
    call_stepping(h, s, "ui_set", v.clone()).unwrap_or_else(|e| panic!("ui_set {v}: {e}"));
}

/// `ui_set` that must fail.
fn ui_set_err(h: &mut Harness<'static, App>, s: &adit_mcp::Session, v: Value, rule: &str) {
    if let Ok(o) = call_stepping(h, s, "ui_set", v.clone()) {
        panic!("{rule}: ui_set {v} must be an error, got Ok({})", o.json);
    }
}

fn track_ids(s: &adit_mcp::Session, pid: &str, kind: TrackKind) -> Vec<Id> {
    snapshot(s, pid).sequences[0]
        .tracks
        .iter()
        .filter(|t| t.kind == kind)
        .map(|t| t.id)
        .collect()
}

/// `(start, duration)` of the clips on a track, in time order.
fn spans(s: &adit_mcp::Session, pid: &str, track: Id) -> Vec<(Time, Time)> {
    let p = snapshot(s, pid);
    let mut v: Vec<_> = p
        .track(track)
        .expect("track")
        .1
        .clips
        .iter()
        .map(|c| (c.start, c.duration))
        .collect();
    v.sort();
    v
}

fn text_spec(content: &str) -> Value {
    json!({ "content": content, "font": "", "size": 96, "color": [1, 1, 1, 1],
            "align": "center", "bold": false, "italic": false })
}

fn add_text(
    s: &adit_mcp::Session,
    pid: &str,
    track: Id,
    start: &str,
    name: &str,
    content: &str,
) -> Id {
    let r = edit(
        s,
        pid,
        json!([{ "op": "add_clip", "track_id": track, "source": { "type": "text", "text": text_spec(content) },
                 "start": start, "duration": "2s", "name": name }]),
    );
    r["created"][0][0].as_u64().expect("created")
}

fn content(s: &adit_mcp::Session, pid: &str, id: Id) -> String {
    match &snapshot(s, pid).clip(id).expect("clip").2.source {
        ClipSource::Text { text } => text.content.clone(),
        other => panic!("clip {id} is not text: {other:?}"),
    }
}

fn field_value(h: &Harness<App>) -> Option<String> {
    h.get_by_role(Role::MultilineTextInput).value()
}

fn speed(s: &adit_mcp::Session, pid: &str, id: Id) -> f64 {
    snapshot(s, pid).clip(id).expect("clip").2.speed
}

/// Runs `f` on another thread while the harness is NOT stepped; panics if it takes over 10 s.
fn without_frames<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = sync_channel(1);
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(Duration::from_secs(10))
        .expect("U-FIX-9: ui_set / ui_get must answer without the window drawing a frame")
}

// ------------------------------------------------------------------------------- U-FIX-4

#[test]
fn u_fix_4_razor_click_snapping_off_splits_on_a_frame() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, v1) = active(&h, &s);
    add_named(&s, &pid, v1, json!(0), json!("3s"), "Cut1");
    h.run_steps(3);
    h.state_mut().run_action(Action::ToggleSnap);
    ui_set(&mut h, &s, json!({ "tool": "razor" }));
    assert!(!h.state().ui_state().unwrap().snap);
    // 151.3 px = 1.513 s = frame 45.39 → nearest frame 45 = 1.5 s.
    let p = v1_point(&h, 1.513);
    click_at(&mut h, p);
    let got = spans(&s, &pid, v1);
    assert_eq!(
        got,
        vec![(0, 45 * F30), (45 * F30, 3 * S - 45 * F30)],
        "U-FIX-4: Razor click splits exactly on frame 45"
    );
    assert!(got.iter().all(|(a, d)| a % F30 == 0 && d % F30 == 0));
}

#[test]
fn u_fix_4_razor_click_snapping_on_lands_on_candidate() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, seq, v1) = active(&h, &s);
    let v2 = track_ids(&s, &pid, TrackKind::Video)[1];
    add_named(&s, &pid, v1, json!(0), json!("3s"), "Cut2");
    add_named(&s, &pid, v2, json!(0), json!("1s"), "Edge2");
    edit(
        &s,
        &pid,
        json!([{ "op": "add_marker", "sequence_id": seq, "time": "2s" }]),
    );
    h.run_steps(3);
    assert!(h.state().ui_state().unwrap().snap, "snap on by default");
    ui_set(&mut h, &s, json!({ "tool": "razor" }));
    // Marker at 2 s, pointer 5.5 px right of it (≤ 8 px): split at 2 s (unsnapped: frame 61.65).
    let p = v1_point(&h, 2.055);
    click_at(&mut h, p);
    assert_eq!(
        spans(&s, &pid, v1),
        vec![(0, 2 * S), (2 * S, S)],
        "U-FIX-4: Razor click snaps to the marker"
    );
    // End of the V2 clip at 1 s, pointer 6 px right of it: split at 1 s (unsnapped: frame 31.8).
    let p = v1_point(&h, 1.06);
    click_at(&mut h, p);
    assert_eq!(
        spans(&s, &pid, v1),
        vec![(0, S), (S, S), (2 * S, S)],
        "U-FIX-4: Razor click snaps to a clip edge"
    );
}

// ------------------------------------------------------------------------------- U-FIX-5

/// Untitled with text clips T1 "Hello" `[0, 2s)` and T2 "World" `[3s, 5s)` on V1, T1 selected,
/// Effect Controls shown. Returns (pid, t1, t2).
fn text_setup(h: &mut Harness<'static, App>, s: &adit_mcp::Session) -> (String, Id, Id) {
    let (pid, _, v1) = active(h, s);
    let t1 = add_text(s, &pid, v1, "0s", "T1", "Hello");
    let t2 = add_text(s, &pid, v1, "3s", "T2", "World");
    h.run_steps(3);
    ui_set(h, s, json!({ "selection": [t1] }));
    h.get_by_label("Effect Controls").click();
    h.run_steps(3);
    assert_eq!(field_value(h).as_deref(), Some("Hello"));
    (pid, t1, t2)
}

/// Focuses the content field, selects all and types `text` one key per frame.
fn type_into_field(h: &mut Harness<'static, App>, text: &str) {
    h.get_by_role(Role::MultilineTextInput).click();
    h.run_steps(2);
    h.key_press_modifiers(ctrl(), egui::Key::A);
    h.run_steps(1);
    for ch in text.chars() {
        h.get_by_role(Role::MultilineTextInput)
            .type_text(&ch.to_string());
        h.run_steps(1);
    }
}

#[test]
fn u_fix_5_text_field_holds_typing_and_commits_once_on_blur() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, t1, _) = text_setup(&mut h, &s);
    let d0 = undo_depth(&s, &pid);
    type_into_field(&mut h, "XYZ");
    for i in 0..5 {
        assert_eq!(
            field_value(&h).as_deref(),
            Some("XYZ"),
            "U-FIX-5: typed text stays in the field (frame {i})"
        );
        h.run_steps(1);
    }
    assert_eq!(
        content(&s, &pid, t1),
        "Hello",
        "not committed while focused"
    );
    // Focus leaves the field.
    h.get_by_label("Project").click();
    h.run_steps(4);
    assert_eq!(content(&s, &pid, t1), "XYZ", "U-FIX-5: committed on blur");
    assert_eq!(undo_depth(&s, &pid), d0 + 1, "U-FIX-5: one command");
}

#[test]
fn u_fix_5_selecting_another_clip_commits_to_the_first() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, t1, t2) = text_setup(&mut h, &s);
    let d0 = undo_depth(&s, &pid);
    type_into_field(&mut h, "ABC");
    h.run_steps(2);
    ui_set(&mut h, &s, json!({ "selection": [t2] }));
    h.run_steps(4);
    assert_eq!(content(&s, &pid, t1), "ABC", "U-FIX-5: commit goes to T1");
    assert_eq!(content(&s, &pid, t2), "World", "T2 untouched");
    assert_eq!(undo_depth(&s, &pid), d0 + 1, "one command");
    step_until(&mut h, "field shows T2", |h| {
        field_value(h).as_deref() == Some("World")
    });
}

/// Passes today (the field is rebuilt from the clip every frame); guards the fix against a stale
/// buffer once the field keeps its own text.
#[test]
fn u_fix_5_mcp_edit_shows_in_unfocused_field() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, t1, _) = text_setup(&mut h, &s);
    type_into_field(&mut h, "Q");
    h.get_by_label("Project").click();
    h.run_steps(4);
    h.get_by_label("Effect Controls").click();
    h.run_steps(3);
    edit(
        &s,
        &pid,
        json!([{ "op": "set_text", "clip_id": t1, "text": text_spec("Changed") }]),
    );
    step_until(&mut h, "MCP text edit shows in the field", |h| {
        field_value(h).as_deref() == Some("Changed")
    });
}

// ------------------------------------------------------------------------------- U-FIX-6

/// A: Untitled with clip `ca` (2 s, V1), selected. B: a new project with clip `cb`.
/// Returns (pa, ca, pb, cb). The window stays on A.
fn two_projects(h: &mut Harness<'static, App>, s: &adit_mcp::Session) -> (String, Id, String, Id) {
    let (pa, _, va) = active(h, s);
    let ca = add_named(s, &pa, va, json!(0), json!("2s"), "ClipA");
    let o = call(s, "project_new", json!({ "name": "ProjB" })).json;
    let pb = o["project_id"].as_str().unwrap().to_string();
    let vb = o["track_ids"][0].as_u64().unwrap();
    let cb = add_named(s, &pb, vb, json!(0), json!("2s"), "ClipB");
    h.run_steps(3);
    ui_set(h, s, json!({ "project_id": pa, "selection": [ca] }));
    assert_eq!(h.state().project_id(), Some(pa.as_str()));
    (pa, ca, pb, cb)
}

#[test]
fn u_fix_6_speed_dialog_acts_on_captured_project_and_clips() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pa, _, pb, _) = two_projects(&mut h, &s);
    // Speed applies only to media and sequence clips (SPEC C-SPEED-1), so use media clips.
    let media_clip = |pid: &str| -> Id {
        let m = call(
            &s,
            "media_import",
            json!({ "project_id": pid, "paths": ["speed.mov"] }),
        )
        .json["media"][0]["id"]
            .as_u64()
            .unwrap();
        let track = snapshot(&s, pid).sequences[0].tracks[0].id;
        edit(
            &s,
            pid,
            json!([{ "op": "add_clip", "track_id": track, "source": { "type": "media", "media_id": m },
                     "start": "3s", "duration": "2s" }]),
        )["created"][0][0]
            .as_u64()
            .unwrap()
    };
    let (ca, cb) = (media_clip(&pa), media_clip(&pb));
    h.run_steps(3);
    ui_set(&mut h, &s, json!({ "project_id": pa, "selection": [ca] }));
    h.state_mut().run_action(Action::SpeedDialog);
    step_until(&mut h, "speed dialog", |h| has_label(h, "OK"));
    h.get_by_role(Role::SpinButton).set_value_accesskit(50.0);
    h.run_steps(2);
    // Switch tab and selection to B while the dialog is open.
    ui_set(&mut h, &s, json!({ "project_id": pb, "selection": [cb] }));
    assert_eq!(h.state().project_id(), Some(pb.as_str()));
    h.get_by_label("OK").click();
    h.run_steps(3);
    assert_eq!(speed(&s, &pa, ca), 0.5, "U-FIX-6: A's clip got 50 %");
    assert_eq!(speed(&s, &pb, cb), 1.0, "U-FIX-6: B's clip untouched");
}

#[test]
fn u_fix_6_wand_request_carries_captured_project_and_clips() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pa, ca, pb, cb) = two_projects(&mut h, &s);
    let seq_a = snapshot(&s, &pa).sequences[0].id;
    ui_set(&mut h, &s, json!({ "mark_in": "30f", "mark_out": "60f" }));
    h.state_mut().run_action(Action::WandPrompt);
    step_until(&mut h, "wand prompt", |h| has_label(h, "Request"));
    h.get_by_label("Request").click();
    h.run_steps(2);
    h.get_by_label("Request").type_text("make it blue");
    h.run_steps(2);
    ui_set(&mut h, &s, json!({ "project_id": pb, "selection": [cb] }));
    h.get_by_label("Send").click();
    h.run_steps(3);
    let rs = requests(&s);
    assert_eq!(rs.len(), 1, "one request: {rs:?}");
    let r = &rs[0];
    assert_eq!(
        r["project_id"],
        json!(pa),
        "U-FIX-6: project captured at open"
    );
    assert_eq!(r["sequence_id"].as_u64(), Some(seq_a));
    assert_eq!(
        r["clip_ids"],
        json!([ca]),
        "U-FIX-6: clips captured at open"
    );
    assert_eq!(req_time(r, "start"), 30 * F30);
    assert_eq!(req_time(r, "end"), 60 * F30);
}

#[test]
fn u_fix_6_dialog_closes_when_its_project_is_gone() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pa, _, _, _) = two_projects(&mut h, &s);
    h.state_mut().run_action(Action::SpeedDialog);
    step_until(&mut h, "speed dialog", |h| has_label(h, "Clip Speed"));
    call(&s, "project_close", json!({ "project_id": pa }));
    step_until(&mut h, "speed dialog closes after project_close", |h| {
        !has_label(h, "Clip Speed") && !has_label(h, "OK")
    });
}

// ------------------------------------------------------------------------------- U-FIX-7

/// Gap in px between the bottom of the `Program` title and the top of the `Play` button: the
/// Program monitor area (monitor image plus spacing) of a project with the given tracks.
fn program_gap(video: u32, audio: u32) -> f32 {
    let s = session();
    call(
        &s,
        "project_new",
        json!({ "name": "Tall", "video_tracks": video, "audio_tracks": audio }),
    );
    let mut h = harness(&s, Options::default());
    h.run_steps(5);
    let prog = h.get_by_label("Program").rect();
    let play = h.get_by_label("Play").rect();
    play.min.y - prog.max.y
}

/// The monitor image has no accessible node; it is letterboxed into the space between the
/// `Program` title and the transport row, and at 1600 px wide that space is wider than 200 px
/// times 16:9, so the gap bounds the image height.
#[test]
fn u_fix_7_track_count_does_not_shrink_program_monitor() {
    let g4 = program_gap(2, 2);
    let g18 = program_gap(12, 6);
    assert!(g4 >= 200.0, "4 tracks: Program monitor >= 200 px, got {g4}");
    assert!(
        g18 >= 200.0,
        "U-FIX-7: Program monitor >= 200 px high with 18 tracks, got {g18}"
    );
    assert!(
        (g18 - g4).abs() <= 2.0,
        "U-FIX-7: same height with 18 tracks ({g18}) as with 4 ({g4})"
    );
}

// ------------------------------------------------------------------------------- U-FIX-8

/// Project "Seqs" with sequences Alpha (first) and Beta, one clip in Alpha.
/// Returns (pid, alpha, beta, clip).
fn two_sequences(s: &adit_mcp::Session) -> (String, Id, Id, Id) {
    let o = call(s, "project_new", json!({ "name": "Seqs" })).json;
    let pid = o["project_id"].as_str().unwrap().to_string();
    let alpha = o["sequence_id"].as_u64().unwrap();
    let v1 = o["track_ids"][0].as_u64().unwrap();
    edit(
        s,
        &pid,
        json!([{ "op": "set_sequence", "sequence_id": alpha, "name": "Alpha" }]),
    );
    let r = edit(
        s,
        &pid,
        json!([{ "op": "add_sequence", "name": "Beta", "width": 1920, "height": 1080,
                 "fps": { "num": 30, "den": 1 }, "sample_rate": 48000,
                 "video_tracks": 1, "audio_tracks": 1 }]),
    );
    let beta = r["created"][0][0].as_u64().expect("beta id");
    let c = add_named(s, &pid, v1, json!(0), json!("2s"), "InAlpha");
    (pid, alpha, beta, c)
}

fn named(h: &Harness<App>, name: &str) -> bool {
    h.query_all(by().predicate(move |n| {
        n.label().as_deref() == Some(name) || n.value().as_deref() == Some(name)
    }))
    .next()
    .is_some()
}

fn click_named(h: &Harness<App>, name: &str) {
    h.query_all(by().predicate(move |n| {
        n.label().as_deref() == Some(name) || n.value().as_deref() == Some(name)
    }))
    .next()
    .unwrap_or_else(|| panic!("no node named {name:?}"))
    .click();
}

fn assert_fresh(h: &Harness<App>, seq: Id) {
    let u = h.state().ui_state().unwrap();
    assert_eq!(u.sequence_id, seq);
    assert_eq!(u.playhead, 0, "playhead 0");
    assert!(u.selection.is_empty(), "selection cleared");
    assert_eq!((u.mark_in, u.mark_out), (None, None), "marks cleared");
}

#[test]
fn u_fix_8_selector_lists_and_switches_sequences() {
    let s = session();
    let (_, alpha, beta, c) = two_sequences(&s);
    let mut h = harness(&s, Options::default());
    assert_eq!(h.state().ui_state().unwrap().sequence_id, alpha);
    ui_set(
        &mut h,
        &s,
        json!({ "playhead": "10f", "mark_in": "2f", "mark_out": "8f", "selection": [c] }),
    );
    assert!(named(&h, "Alpha"), "selector shows the current sequence");
    click_named(&h, "Alpha");
    h.run_steps(2);
    assert!(named(&h, "Beta"), "U-FIX-8: selector lists Beta");
    click_named(&h, "Beta");
    h.run_steps(3);
    assert_fresh(&h, beta);
    let v = call_stepping(&mut h, &s, "ui_get", json!({})).unwrap().json;
    assert_eq!(v["sequence_id"].as_u64(), Some(beta), "ui_get reports it");
}

#[test]
fn u_fix_8_ui_set_sequence_id_switches_and_rejects_unknown() {
    let s = session();
    let (_, alpha, beta, c) = two_sequences(&s);
    let mut h = harness(&s, Options::default());
    ui_set(
        &mut h,
        &s,
        json!({ "playhead": "10f", "mark_in": "2f", "mark_out": "8f", "selection": [c] }),
    );
    ui_set(&mut h, &s, json!({ "sequence_id": beta }));
    h.run_steps(2);
    assert_fresh(&h, beta);
    let v = call_stepping(&mut h, &s, "ui_get", json!({})).unwrap().json;
    assert_eq!(v["sequence_id"].as_u64(), Some(beta));
    let before = h.state().ui_state().unwrap().clone();
    ui_set_err(
        &mut h,
        &s,
        json!({ "sequence_id": 999_999 }),
        "U-FIX-8 unknown id",
    );
    h.run_steps(2);
    assert_eq!(h.state().ui_state().unwrap(), &before);
    ui_set(&mut h, &s, json!({ "sequence_id": alpha }));
    h.run_steps(2);
    assert_eq!(h.state().ui_state().unwrap().sequence_id, alpha);
}

/// Removing the sequence being shown (here the first one, so no switch is needed) falls back to
/// the (new) first one. Passes today (refresh falls back already).
#[test]
fn u_fix_8_removed_current_sequence_falls_back_to_first() {
    let s = session();
    let (pid, alpha, beta, _) = two_sequences(&s);
    let mut h = harness(&s, Options::default());
    assert_eq!(h.state().ui_state().unwrap().sequence_id, alpha);
    edit(
        &s,
        &pid,
        json!([{ "op": "remove_sequence", "sequence_id": alpha }]),
    );
    step_until(&mut h, "fall back to Beta", |h| {
        h.state().ui_state().unwrap().sequence_id == beta
    });
}

#[test]
fn u_fix_8_removed_switched_sequence_falls_back_to_first() {
    let s = session();
    let (pid, alpha, beta, _) = two_sequences(&s);
    let mut h = harness(&s, Options::default());
    ui_set(&mut h, &s, json!({ "sequence_id": beta }));
    h.run_steps(2);
    assert_eq!(h.state().ui_state().unwrap().sequence_id, beta);
    edit(
        &s,
        &pid,
        json!([{ "op": "remove_sequence", "sequence_id": beta }]),
    );
    step_until(&mut h, "fall back to Alpha", |h| {
        h.state().ui_state().unwrap().sequence_id == alpha
    });
}

// ------------------------------------------------------------------------------- U-FIX-9

#[test]
fn u_fix_9_ui_set_without_open_project_is_error() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, _) = active(&h, &s);
    call(&s, "project_close", json!({ "project_id": pid }));
    step_until(&mut h, "no tab left", |h| h.state().project_id().is_none());
    ui_set_err(&mut h, &s, json!({ "playhead": 0 }), "U-FIX-9 no project");
}

#[test]
fn u_fix_9_times_out_of_range_are_errors() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let before = h.state().ui_state().unwrap().clone();
    for v in [
        json!({ "playhead": MAX_TIME + F30 }),
        json!({ "playhead": -F30 }),
        json!({ "mark_in": MAX_TIME + F30 }),
        json!({ "mark_out": -F30 }),
    ] {
        ui_set_err(&mut h, &s, v, "U-FIX-9 time range");
    }
    h.run_steps(2);
    assert_eq!(h.state().ui_state().unwrap(), &before, "nothing applied");
    // The bound itself is allowed.
    ui_set(&mut h, &s, json!({ "playhead": MAX_TIME }));
    h.run_steps(2);
    assert_eq!(h.state().ui_state().unwrap().playhead, MAX_TIME);
}

#[test]
fn u_fix_9_mark_in_not_before_mark_out_is_error() {
    let s = session();
    let mut h = harness(&s, Options::default());
    for v in [
        json!({ "mark_in": "20f", "mark_out": "10f" }),
        json!({ "mark_in": "10f", "mark_out": "10f" }),
    ] {
        ui_set_err(&mut h, &s, v, "U-FIX-9 new marks");
    }
    ui_set(&mut h, &s, json!({ "mark_in": "5f", "mark_out": "10f" }));
    h.run_steps(2);
    let before = h.state().ui_state().unwrap().clone();
    assert_eq!(
        (before.mark_in, before.mark_out),
        (Some(5 * F30), Some(10 * F30))
    );
    for v in [
        json!({ "mark_in": "15f" }), // new in vs existing out 10f
        json!({ "mark_in": "10f" }), // equal to existing out
        json!({ "mark_out": "3f" }), // existing in 5f vs new out
        json!({ "mark_out": "5f" }), // equal to existing in
    ] {
        ui_set_err(&mut h, &s, v, "U-FIX-9 marks 5f/10f");
    }
    h.run_steps(2);
    assert_eq!(h.state().ui_state().unwrap(), &before, "state unchanged");
    let g = call_stepping(&mut h, &s, "ui_get", json!({})).unwrap().json;
    assert_eq!(g["mark_in"].as_i64(), Some(5 * F30));
    assert_eq!(g["mark_out"].as_i64(), Some(10 * F30));
}

/// Passes today (the bridge already rejects unknown tool names).
#[test]
fn u_fix_9_unknown_tool_is_error() {
    let s = session();
    let mut h = harness(&s, Options::default());
    ui_set_err(&mut h, &s, json!({ "tool": "chainsaw" }), "U-FIX-9 tool");
    assert_eq!(h.state().ui_state().unwrap().tool, Tool::Select);
}

/// `ui_set` then `ui_get` with no frame drawn in between (the harness is not stepped while the
/// calls run on another thread): `get` sees the last `set`, in call order.
#[test]
fn u_fix_9_set_visible_to_get_before_next_frame() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, _) = active(&h, &s);
    let s2 = s.clone();
    let (a, b, g) = without_frames(move || {
        let a = s2.call(
            "ui_set",
            json!({ "playhead": "12f", "mark_in": "3f", "tool": "razor" }),
        );
        let b = s2.call("ui_set", json!({ "playhead": "7f" }));
        let g = s2.call("ui_get", json!({})).map(|o| o.json);
        (a.map(|_| ()), b.map(|_| ()), g)
    });
    a.expect("first ui_set");
    b.expect("second ui_set");
    let g = g.expect("ui_get");
    assert_eq!(g["project_id"], json!(pid));
    assert_eq!(
        g["playhead"].as_i64(),
        Some(7 * F30),
        "U-FIX-9: last set wins"
    );
    assert_eq!(g["mark_in"].as_i64(), Some(3 * F30));
    assert_eq!(g["tool"], "razor");
    // And the drawn state on the next frame.
    h.run_steps(1);
    let u = h.state().ui_state().unwrap();
    assert_eq!(u.playhead, 7 * F30);
    assert_eq!(u.mark_in, Some(3 * F30));
    assert_eq!(u.tool, Tool::Razor);
}

#[test]
fn u_fix_9_ui_set_project_opened_a_moment_ago() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let s2 = s.clone();
    let (pid, set, g) = without_frames(move || {
        let o = s2
            .call("project_new", json!({ "name": "JustNow" }))
            .expect("project_new");
        let pid = o.json["project_id"].as_str().unwrap().to_string();
        let set = s2.call("ui_set", json!({ "project_id": pid })).map(|_| ());
        let g = s2.call("ui_get", json!({})).map(|o| o.json);
        (pid, set, g)
    });
    set.expect("U-FIX-9: ui_set on a project opened a moment ago");
    assert_eq!(
        g.expect("ui_get")["project_id"],
        json!(pid),
        "U-FIX-9: visible to get at once"
    );
    h.run_steps(2);
    assert_eq!(h.state().project_id(), Some(pid.as_str()), "tab active");
}

// ------------------------------------------------------------------------------- U-FIX-10

#[test]
fn u_fix_10_mcp_close_of_dirty_project_leaves_status() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let o = call(&s, "project_new", json!({ "name": "Walrus6" })).json;
    let pid = o["project_id"].as_str().unwrap().to_string();
    let v1 = o["track_ids"][0].as_u64().unwrap();
    add_named(&s, &pid, v1, json!(0), json!("1s"), "W");
    step_until(&mut h, "tab Walrus6", |h| has_text(h, "Walrus6"));
    call(&s, "project_close", json!({ "project_id": pid }));
    step_until(
        &mut h,
        "status naming Walrus6 and its unsaved changes",
        |h| {
            any_text(h, |t| {
                t.contains("Walrus6") && t.to_lowercase().contains("unsaved")
            })
        },
    );
}

fn request_close(h: &mut Harness<'static, App>) -> bool {
    h.input_mut()
        .viewports
        .entry(egui::ViewportId::ROOT)
        .or_default()
        .events
        .push(egui::ViewportEvent::Close);
    h.step();
    h.output()
        .viewport_output
        .get(&egui::ViewportId::ROOT)
        .is_some_and(|v| v.commands.contains(&egui::ViewportCommand::CancelClose))
}

#[test]
fn u_fix_10_fresh_untitled_does_not_prompt_on_quit() {
    let s = session();
    let mut h = harness(&s, Options::default());
    assert!(
        !request_close(&mut h),
        "U-FIX-10: a fresh untouched Untitled does not cancel the close"
    );
    h.run_steps(2);
    assert!(!has_label(&h, "Unsaved changes"), "no quit prompt");
}

#[test]
fn u_fix_10_quit_prompt_for_edited_project_goes_away_when_saved() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, _) = active(&h, &s);
    h.state_mut().run_action(Action::AddMarker);
    h.run_steps(2);
    assert!(
        request_close(&mut h),
        "edited dirty project cancels the close"
    );
    step_until(&mut h, "quit prompt", |h| has_label(h, "Unsaved changes"));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("q.adit");
    call(
        &s,
        "project_save",
        json!({ "project_id": pid, "path": path.to_str().unwrap() }),
    );
    step_until(&mut h, "quit prompt gone after save", |h| {
        !has_label(h, "Unsaved changes")
    });
}

#[test]
fn u_fix_10_file_close_project_on_dirty_project_asks_first() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, _) = active(&h, &s);
    h.state_mut().run_action(Action::AddMarker);
    h.run_steps(2);
    h.get_by_label("File").click();
    h.run_steps(2);
    h.get_by_label("Close Project").click();
    h.run_steps(3);
    assert!(
        s.projects().iter().any(|p| p.id == pid),
        "U-FIX-10: not closed before confirming"
    );
    assert!(has_label(&h, "Cancel"), "U-FIX-10: a confirmation is shown");
    h.get_by_label("Cancel").click();
    h.run_steps(3);
    assert!(s.projects().iter().any(|p| p.id == pid), "Cancel keeps it");
}

// ------------------------------------------------------------------------------- U-FIX-11

/// Pattern whose pictures of any path containing "kiwi" cannot be read.
struct Offline(Pattern);

impl MediaBackend for Offline {
    fn probe(&self, path: &str) -> Result<MediaSpec, MediaError> {
        self.0.probe(path)
    }
    fn video_frame(&self, path: &str, time: Time) -> Result<Arc<Frame>, MediaError> {
        if path.contains("kiwi") {
            return Err(MediaError::NotFound(path.into()));
        }
        self.0.video_frame(path, time)
    }
    fn audio(
        &self,
        path: &str,
        start: Time,
        duration: Time,
        sample_rate: u32,
        filter: &str,
    ) -> Result<Vec<f32>, MediaError> {
        self.0.audio(path, start, duration, sample_rate, filter)
    }
}

#[test]
fn u_fix_11_offline_media_reported_and_labelled() {
    let s = session_with(Arc::new(Offline(pattern())));
    let mut h = harness(&s, Options::default());
    let (pid, _, v1) = active(&h, &s);
    let o = call(
        &s,
        "media_import",
        json!({ "project_id": pid, "paths": ["kiwi.mp4", "emu.mp4"] }),
    )
    .json;
    let (kiwi, emu) = (&o["media"][0], &o["media"][1]);
    let kiwi_name = kiwi["name"].as_str().unwrap().to_string();
    let emu_name = emu["name"].as_str().unwrap().to_string();
    // Clip names equal the media names, so "<name>" reads the same either way. The offline clip
    // is away from the playhead so no Program render touches it.
    edit(
        &s,
        &pid,
        json!([
            { "op": "add_clip", "track_id": v1, "source": { "type": "media", "media_id": emu["id"] },
              "start": 0, "duration": "2s", "name": emu_name },
            { "op": "add_clip", "track_id": v1, "source": { "type": "media", "media_id": kiwi["id"] },
              "start": "5s", "duration": "2s", "name": kiwi_name }
        ]),
    );
    let status = format!("media offline: {kiwi_name}");
    step_until(&mut h, &status, |h| has_text(h, &status));
    let label = format!("[offline] {kiwi_name}");
    step_until(&mut h, &label, |h| has_label(h, &label));
    assert!(has_label(&h, &emu_name), "readable clip labelled normally");
    assert!(
        !has_text(&h, &format!("[offline] {emu_name}")),
        "readable clip not marked offline"
    );
}

// ------------------------------------------------------------------------------- U-FIX-12

#[test]
fn u_fix_12_type_tool_adds_track_and_text_in_one_undo_step() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, _) = active(&h, &s);
    let vids = track_ids(&s, &pid, TrackKind::Video);
    assert_eq!(vids.len(), 2);
    for &t in &vids {
        add_named(&s, &pid, t, json!(0), json!("10s"), "Busy");
    }
    h.run_steps(3);
    ui_set(&mut h, &s, json!({ "tool": "type" }));
    let next_id = snapshot(&s, &pid).next_id;
    let d0 = undo_depth(&s, &pid);
    let prog = h.get_by_label("Program").rect();
    click_at(&mut h, egui::pos2(prog.min.x + 200.0, prog.max.y + 100.0));
    let after = track_ids(&s, &pid, TrackKind::Video);
    assert_eq!(after.len(), 3, "U-FIX-12: one more video track");
    let new = *after.iter().find(|t| !vids.contains(t)).unwrap();
    assert_eq!(new, next_id, "U-FIX-12: new track id = next_id before");
    let p = snapshot(&s, &pid);
    let clips = &p.track(new).unwrap().1.clips;
    assert_eq!(clips.len(), 1);
    assert!(matches!(clips[0].source, ClipSource::Text { .. }));
    assert_eq!(undo_depth(&s, &pid), d0 + 1, "U-FIX-12: one undo step");
    call(&s, "undo", json!({ "project_id": pid }));
    assert_eq!(
        track_ids(&s, &pid, TrackKind::Video),
        vids,
        "one undo removes track and clip"
    );
}

// ------------------------------------------------------------------------------- U-FIX-13

#[test]
fn u_fix_13_effects_browser_filter_only_on_unlocked_video_clips() {
    let s = session();
    let mut h = harness_fast(&s);
    let (pid, _, v1) = active(&h, &s);
    let v2 = track_ids(&s, &pid, TrackKind::Video)[1];
    let a1 = track_ids(&s, &pid, TrackKind::Audio)[0];
    let c1 = add_named(&s, &pid, v1, json!(0), json!("2s"), "Free");
    let c2 = add_named(&s, &pid, v2, json!(0), json!("2s"), "Locked");
    let m = call(
        &s,
        "media_import",
        json!({ "project_id": pid, "paths": ["snd.mp4"] }),
    )
    .json["media"][0]["id"]
        .as_u64()
        .unwrap();
    let c3 = edit(
        &s,
        &pid,
        json!([{ "op": "add_clip", "track_id": a1, "source": { "type": "media", "media_id": m },
                 "start": 0, "duration": "2s" }]),
    )["created"][0][0]
        .as_u64()
        .unwrap();
    edit(
        &s,
        &pid,
        json!([{ "op": "set_track", "track_id": v2, "locked": true }]),
    );
    let filter = call(&s, "effect_list", json!({ "project_id": pid })).json["effects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "filter")
        .expect("a filter")["name"]
        .as_str()
        .unwrap()
        .to_string();
    h.run_steps(3);
    ui_set(&mut h, &s, json!({ "selection": [c1, c2, c3] }));
    let d0 = undo_depth(&s, &pid);
    h.get_by_label("Effects").click();
    // 0.5 s pause so the tab click does not make the next two clicks a triple-click.
    h.run_steps(30);
    {
        // Two clicks in one queue, 1/60 s apart: a double-click.
        let n = h.get_by_label(&filter);
        n.click();
        n.click();
    }
    h.run_steps(3);
    let p = snapshot(&s, &pid);
    let fx = |id: Id| p.clip(id).unwrap().2.effects.len();
    assert_eq!(fx(c1), 1, "U-FIX-13: unlocked video clip gets {filter}");
    assert_eq!(fx(c2), 0, "locked track untouched");
    assert_eq!(fx(c3), 0, "audio clip untouched");
    assert_eq!(undo_depth(&s, &pid), d0 + 1, "the batch succeeded");
}
