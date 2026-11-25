//! Window rules added after the review (docs/GUI.md U-ED-23), headless through egui_kittest.
//! Written from the contract only; expected values are worked out by hand.
//!
//! Not observable in tests, so no test: (b) drag preview computed once per gesture change,
//! (c) dropped files without a pointer position placed at the first reported position,
//! (e) Effects browser re-reading `effect_list` every 2 s, (f) retry of a failed waveform.
//!
//! Readings taken where the contract names no seam:
//! - "Shift held" for (a) is the input modifiers of the frame carrying `egui::Event::Cut`
//!   (`Event::Cut` itself carries none);
//! - the `Wand` tool is set with `ui_set { tool: "wand" }`, as tests/ed_app.rs does.
mod app_common;
mod common;
use adit_core::{Id, Time};
use adit_gui::{App, Options};
use app_common::*;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use serde_json::{Value, json};

fn ui_set(h: &mut Harness<'static, App>, s: &adit_mcp::Session, v: Value) {
    call_stepping(h, s, "ui_set", v.clone()).unwrap_or_else(|e| panic!("ui_set {v}: {e}"));
    h.run_steps(2);
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

// ------------------------------------------------------------------------------ U-ED-23 (a)

/// V1 holds `A` `[0, 1s)` and `B` `[1s, 2s)`; `A` is selected. Returns (pid, v1, a, b).
fn two_adjacent(h: &mut Harness<'static, App>, s: &adit_mcp::Session) -> (String, Id, Id, Id) {
    let (pid, _, v1) = active(h, s);
    let a = add_named(s, &pid, v1, json!(0), json!("1s"), "Aaa");
    let b = add_named(s, &pid, v1, json!("1s"), json!("1s"), "Bbb");
    h.run_steps(3);
    ui_set(h, s, json!({ "selection": [a] }));
    (pid, v1, a, b)
}

fn event_with_modifiers(h: &mut Harness<'static, App>, ev: egui::Event, m: egui::Modifiers) {
    h.event_modifiers(ev, m);
    h.run_steps(5);
}

#[test]
fn u_ed_23a_shift_cut_event_ripple_deletes_and_leaves_clipboard_empty() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, v1, _a, _b) = two_adjacent(&mut h, &s);
    let d0 = undo_depth(&s, &pid);
    event_with_modifiers(&mut h, egui::Event::Cut, egui::Modifiers::SHIFT);
    // A is removed and B closes up: [1s, 2s) -> [0, 1s).
    assert_eq!(
        spans(&s, &pid, v1),
        vec![(0, S)],
        "U-ED-23a: Shift + Cut event runs RippleDelete: the later clip moves left"
    );
    assert_eq!(undo_depth(&s, &pid), d0 + 1, "U-ED-23a: one undo step");

    h.event(egui::Event::Paste("x".into()));
    h.run_steps(3);
    assert_eq!(
        spans(&s, &pid, v1),
        vec![(0, S)],
        "U-ED-23a: the clipboard is untouched, so a paste does nothing"
    );
    assert_eq!(
        undo_depth(&s, &pid),
        d0 + 1,
        "U-ED-23a: the paste adds no undo step"
    );
}

#[test]
fn u_ed_23a_cut_event_without_shift_still_cuts() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, v1, _a, _b) = two_adjacent(&mut h, &s);
    let d0 = undo_depth(&s, &pid);
    event_with_modifiers(&mut h, egui::Event::Cut, egui::Modifiers::NONE);
    assert_eq!(
        spans(&s, &pid, v1),
        vec![(S, S)],
        "U-ED-23a: a plain Cut removes A and leaves B where it was"
    );
    assert_eq!(undo_depth(&s, &pid), d0 + 1, "U-ED-23a: one undo step");
    h.event(egui::Event::Paste("x".into()));
    h.run_steps(3);
    assert_eq!(
        spans(&s, &pid, v1),
        vec![(0, S), (S, S)],
        "U-ED-23a: the plain Cut filled the clipboard, paste brings A back at the playhead 0"
    );
}

// ------------------------------------------------------------------------------ U-ED-23 (d)

#[test]
fn u_ed_23d_wand_double_click_on_effect_applies_nothing_and_opens_prompt() {
    let s = session();
    let s2 = s.clone();
    // 60 steps per simulated second so two clicks make a double-click (as in ed_app.rs U-ED-20).
    let mut h = Harness::builder()
        .with_size(egui::vec2(W, H))
        .with_step_dt(1.0 / 60.0)
        .build_eframe(move |cc| App::new(cc, s2, Options::default()));
    h.run_steps(3);
    let (pid, _, v1) = active(&h, &s);
    let c = add_named(&s, &pid, v1, json!(0), json!("2s"), "Wash");
    h.run_steps(3);
    ui_set(&mut h, &s, json!({ "selection": [c] }));
    h.get_by_label("Effects").click();
    h.run_steps(30);
    ui_set(&mut h, &s, json!({ "tool": "wand" }));
    h.run_steps(30);
    assert!(
        !has_label(&h, "Request"),
        "test setup: no prompt before the click"
    );
    let d0 = undo_depth(&s, &pid);
    {
        let n = h
            .query_all_by_label_contains("exposure")
            .next()
            .expect("the built-in effect `exposure` is listed in the Effects browser");
        n.click();
        n.click();
    }
    h.run_steps(5);
    assert!(
        snapshot(&s, &pid).clip(c).unwrap().2.effects.is_empty(),
        "U-ED-23d: with the Wand a double click on an effect applies nothing"
    );
    assert_eq!(
        undo_depth(&s, &pid),
        d0,
        "U-ED-23d: the project is unchanged"
    );
    step_until(
        &mut h,
        "U-ED-23d: the wand click opens the prompt (field `Request`)",
        |h| has_label(h, "Request"),
    );
}

// ------------------------------------------------------------------------------ U-ED-23 (g)

/// Two open projects (A is the tab shown first, B is another); both number V1 alike.
fn two_projects(h: &mut Harness<'static, App>, s: &adit_mcp::Session) -> (String, Id, String, Id) {
    let (pa, _, va) = active(h, s);
    let o = call(s, "project_new", json!({ "name": "ProjB" })).json;
    let pb = o["project_id"].as_str().unwrap().to_string();
    let vb = o["track_ids"][0].as_u64().unwrap();
    assert_eq!(va, vb, "test setup: two default projects number V1 alike");
    h.run_steps(3);
    (pa, va, pb, vb)
}

fn copy_in_a_paste_in_b(
    h: &mut Harness<'static, App>,
    s: &adit_mcp::Session,
    pa: &str,
    pb: &str,
    clip: Id,
) {
    ui_set(h, s, json!({ "project_id": pa, "selection": [clip] }));
    h.event(egui::Event::Copy);
    h.run_steps(3);
    ui_set(h, s, json!({ "project_id": pb }));
    assert_eq!(h.state().project_id(), Some(pb), "test setup: B is shown");
    h.event(egui::Event::Paste("x".into()));
    h.run_steps(3);
}

#[test]
fn u_ed_23g_pasting_a_media_clip_into_another_project_adds_nothing() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pa, va, pb, vb) = two_projects(&mut h, &s);
    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("take.mov");
    std::fs::write(&f, b"pattern backend ignores this file").unwrap();
    let m = call(
        &s,
        "media_import",
        json!({ "project_id": pa, "paths": [f.to_str().unwrap()] }),
    )
    .json["media"][0]["id"]
        .as_u64()
        .expect("media id");
    let c = edit(
        &s,
        &pa,
        json!([{ "op": "add_clip", "track_id": va,
                 "source": { "type": "media", "media_id": m },
                 "start": 0, "duration": "1s" }]),
    )["created"][0][0]
        .as_u64()
        .unwrap();
    let d0 = undo_depth(&s, &pb);
    copy_in_a_paste_in_b(&mut h, &s, &pa, &pb, c);
    assert_eq!(
        spans(&s, &pb, vb),
        vec![],
        "U-ED-23g: a media clip is left out of a paste into another project"
    );
    assert_eq!(
        undo_depth(&s, &pb),
        d0,
        "U-ED-23g: nothing is pasted, no undo step"
    );
    assert_eq!(spans(&s, &pa, va), vec![(0, S)], "project A untouched");
}

#[test]
fn u_ed_23g_text_and_solid_clips_still_paste_into_another_project() {
    let src_text = json!({ "type": "text", "text": {
        "content": "Hi", "font": "", "size": 24.0, "color": [1.0, 1.0, 1.0, 1.0],
        "align": "center", "bold": false, "italic": false } });
    for (what, source) in [("text", src_text), ("solid", solid([0.2, 0.4, 0.6, 1.0]))] {
        let s = session();
        let mut h = harness(&s, Options::default());
        let (pa, va, pb, vb) = two_projects(&mut h, &s);
        let c = edit(
            &s,
            &pa,
            json!([{ "op": "add_clip", "track_id": va, "source": source,
                     "start": "1s", "duration": "1s" }]),
        )["created"][0][0]
            .as_u64()
            .unwrap();
        copy_in_a_paste_in_b(&mut h, &s, &pa, &pb, c);
        // B's playhead is 0 and the base is the clip's own start: the copy starts at 0.
        assert_eq!(
            spans(&s, &pb, vb),
            vec![(0, S)],
            "U-ED-23g: a {what} clip is pasted into another project"
        );
    }
}

// ------------------------------------------------------------------------------ U-ED-23 (h)

#[test]
fn u_ed_23h_snap_delta_does_not_panic_for_extreme_values() {
    let p = common::B::std()
        .clip(common::V1, 101, 0, common::S)
        .clip(common::V1, 102, 2 * common::S, common::S)
        .build();
    let extremes: [Time; 6] = [i64::MIN, i64::MIN + 1, -1, 0, i64::MAX - 1, i64::MAX];
    let tolerances: [Time; 6] = [i64::MIN, -1, 0, 8, i64::MAX - 1, i64::MAX];
    for snap in [true, false] {
        let mut ui = common::ui();
        ui.snap = snap;
        for clips in [vec![101], vec![101, 102], vec![]] {
            for &d in &extremes {
                for &t in &tolerances {
                    // U-ED-23h: only "returns" is specified, so the value is not checked.
                    let _ = adit_gui::snap_delta(d, &clips, &p, &ui, t);
                }
            }
        }
    }
}
