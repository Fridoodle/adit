//! Window rules for editing comfort (docs/GUI.md U-ED-10 .. U-ED-20), headless through
//! egui_kittest. Written from the contract only; expected values are worked out by hand.
//!
//! Timeline geometry for mouse input, as in tests/fix_app.rs: the track header button `V1` sits
//! 4 px right of and below the top-left corner of the V1 row, the header is 150 px wide, a row
//! middle is 22 px down, and a fresh tab shows 100 px per second from time 0 (U-STATE).
//!
//! Readings taken where the contract names no seam (listed for the contract):
//! - context-menu entries, submenu entries, `Snap`, `Uniform Scale`, tool names, `AI Import...` and
//!   `Ask AI to Change adit...` are accessible labels, exactly as written in the rules;
//! - "selected" (`Snap`) and "checked" (`Uniform Scale`) are the accesskit toggled state;
//! - the preview-resolution selector is a node whose label or value is the current choice and,
//!   once clicked, shows the other choices the same way (as the sequence selector, U-FIX-14);
//! - a `Select` body drag snaps with the 8 px tolerance of U-APP-4;
//! - `panel` of an app request is the panel's title of U-APP-1 (`Requests`);
//! - a library effect's row in the Effects browser contains the effect name.
//!
//! No test: U-ED-13 and the looks of U-ED-14 (drawing only), tool hover texts (U-ED-11), the
//! Scale X / Scale Y fields of U-ED-15 and the Project panel's right-click menu of U-ED-19 (no
//! labelled seam), the native file dialog behind `AI Import...`.
mod app_common;
use adit_core::{Id, Time};
use adit_gui::{Action, App, Options, Tool};
use app_common::*;
use eframe::egui;
use eframe::egui::accesskit::Toggled;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable, by};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;

// ------------------------------------------------------------------------------- helpers

/// Screen point of sequence time `secs` in the middle of the V1 row.
fn v1_point(h: &Harness<App>, secs: f64) -> egui::Pos2 {
    let r = h.get_by_label("V1").rect();
    let (left, top) = (r.min.x - 4.0, r.min.y - 4.0);
    egui::pos2(left + 150.0 + (secs * 100.0) as f32, top + 22.0)
}

fn button(h: &Harness<'static, App>, p: egui::Pos2, button: egui::PointerButton, pressed: bool) {
    h.event(egui::Event::PointerButton {
        pos: p,
        button,
        pressed,
        modifiers: egui::Modifiers::NONE,
    });
}

/// Primary press and release at one point, a frame apart.
fn click_at(h: &mut Harness<'static, App>, p: egui::Pos2) {
    h.hover_at(p);
    h.step();
    h.drag_at(p);
    h.step();
    h.drop_at(p);
    h.run_steps(3);
}

/// Secondary press and release at one point, a frame apart; the pointer stays there.
fn right_click_at(h: &mut Harness<'static, App>, p: egui::Pos2) {
    h.hover_at(p);
    h.step();
    button(h, p, egui::PointerButton::Secondary, true);
    h.step();
    button(h, p, egui::PointerButton::Secondary, false);
    h.run_steps(3);
}

/// Primary drag from `a` to `b` through the midpoint, one event per frame.
fn drag(h: &mut Harness<'static, App>, a: egui::Pos2, b: egui::Pos2) {
    h.hover_at(a);
    h.step();
    h.drag_at(a);
    h.step();
    h.hover_at(a + (b - a) * 0.5);
    h.step();
    h.hover_at(b);
    h.run_steps(2);
    h.drop_at(b);
    h.run_steps(3);
}

/// Clicks the first node with exactly this label.
fn click_label(h: &mut Harness<'static, App>, label: &str, rule: &str) {
    h.query_all_by_label(label)
        .next()
        .unwrap_or_else(|| panic!("{rule}: nothing labelled {label:?} to click"))
        .click();
    h.run_steps(3);
}

/// Focuses the field labelled `label` and types `text` into it.
fn type_into(h: &mut Harness<'static, App>, label: &str, text: &str) {
    h.get_by_label(label).click();
    h.run_steps(2);
    h.get_by_label(label).type_text(text);
    h.run_steps(2);
}

fn named(h: &Harness<App>, name: &str) -> bool {
    h.query_all(by().predicate(move |n| {
        n.label().as_deref() == Some(name) || n.value().as_deref() == Some(name)
    }))
    .next()
    .is_some()
}

fn click_named(h: &mut Harness<'static, App>, name: &str, rule: &str) {
    h.query_all(by().predicate(move |n| {
        n.label().as_deref() == Some(name) || n.value().as_deref() == Some(name)
    }))
    .next()
    .unwrap_or_else(|| panic!("{rule}: no node named {name:?}"))
    .click();
    h.run_steps(3);
}

/// Accesskit toggled state of the first node with this label.
fn toggled(h: &Harness<App>, label: &str, rule: &str) -> Option<Toggled> {
    h.query_all_by_label(label)
        .next()
        .unwrap_or_else(|| panic!("{rule}: nothing labelled {label:?}"))
        .accesskit_node()
        .toggled()
}

fn ui_get(h: &mut Harness<'static, App>, s: &adit_mcp::Session) -> Value {
    call_stepping(h, s, "ui_get", json!({}))
        .unwrap_or_else(|e| panic!("ui_get: {e}"))
        .json
}

fn ui_set(h: &mut Harness<'static, App>, s: &adit_mcp::Session, v: Value) {
    call_stepping(h, s, "ui_set", v.clone()).unwrap_or_else(|e| panic!("ui_set {v}: {e}"));
    h.run_steps(2);
}

/// `ui_set` that must fail.
fn ui_set_err(h: &mut Harness<'static, App>, s: &adit_mcp::Session, v: Value, rule: &str) {
    if let Ok(o) = call_stepping(h, s, "ui_set", v.clone()) {
        panic!("{rule}: ui_set {v} must be an error, got Ok({})", o.json);
    }
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

fn label_of(s: &adit_mcp::Session, pid: &str, clip: Id) -> u8 {
    snapshot(s, pid).clip(clip).expect("clip").2.label
}

/// Untitled with `Left` `[0, 1s)` and `Right` `[3s, 4s)` on V1. Returns (pid, v1, left, right).
fn gap_setup(h: &mut Harness<'static, App>, s: &adit_mcp::Session) -> (String, Id, Id, Id) {
    let (pid, _, v1) = active(h, s);
    let l = add_named(s, &pid, v1, json!(0), json!("1s"), "Left");
    let r = add_named(s, &pid, v1, json!("3s"), json!("1s"), "Right");
    h.run_steps(3);
    (pid, v1, l, r)
}

/// Untitled with one clip `Ocelot5` `[1s, 3s)` on V1, drawn. Returns (pid, v1, clip).
fn one_clip(h: &mut Harness<'static, App>, s: &adit_mcp::Session) -> (String, Id, Id) {
    let (pid, _, v1) = active(h, s);
    let c = add_named(s, &pid, v1, json!("1s"), json!("2s"), "Ocelot5");
    step_until(h, "clip Ocelot5 in the timeline", |h| {
        has_label(h, "Ocelot5")
    });
    (pid, v1, c)
}

#[derive(Debug)]
struct Dropped(PathBuf);

impl egui::DroppedFile for Dropped {
    fn path(&self) -> &Path {
        &self.0
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        Err("the window imports by path".into())
    }
}

/// Hands the window dropped files with its next frame.
fn drop_files(h: &mut Harness<'static, App>, paths: &[PathBuf]) {
    for p in paths {
        h.input_mut()
            .dropped_files
            .push(Arc::new(Dropped(p.clone())));
    }
    h.run_steps(5);
}

// ------------------------------------------------------------------------------- U-ED-10

#[test]
fn u_ed_10_right_click_on_clip_shows_menu_and_selects_it() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (_, _, c) = one_clip(&mut h, &s);
    assert!(h.state().ui_state().unwrap().selection.is_empty());
    h.get_by_label("Ocelot5").click_secondary();
    h.run_steps(3);
    assert_eq!(
        h.state().ui_state().unwrap().selection,
        vec![c],
        "U-ED-10 / U-ED-9: a right click on an unselected clip selects it"
    );
    // One unlinked, ungrouped clip, empty clipboard (U-ED-9 order, U-ED-10 texts).
    for label in [
        "Cut",
        "Copy",
        "Delete",
        "Ripple Delete",
        "Enable/Disable",
        "Speed...",
        "Audio Gain...",
        "Label",
        "Add Transition",
        "Ask AI...",
    ] {
        assert!(
            has_label(&h, label),
            "U-ED-10: context menu of a clip has {label:?}"
        );
    }
    for label in ["Paste", "Ungroup Audio/Video", "Group", "Ungroup Clips"] {
        assert!(
            !has_label(&h, label),
            "U-ED-10: no {label:?} for one unlinked, ungrouped clip and an empty clipboard"
        );
    }
}

#[test]
fn u_ed_10_no_menu_entries_before_right_click() {
    let s = session();
    let mut h = harness(&s, Options::default());
    one_clip(&mut h, &s);
    for label in ["Ripple Delete", "Add Transition", "Ask AI..."] {
        assert!(
            !has_label(&h, label),
            "U-ED-10: {label:?} appears only in the right-click menu"
        );
    }
}

#[test]
fn u_ed_10_menu_delete_removes_the_clip() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, v1, _) = one_clip(&mut h, &s);
    let d0 = undo_depth(&s, &pid);
    // 2 s = middle of the clip `[1s, 3s)`.
    let p = v1_point(&h, 2.0);
    right_click_at(&mut h, p);
    click_label(&mut h, "Delete", "U-ED-10");
    assert_eq!(
        spans(&s, &pid, v1),
        vec![],
        "U-ED-10: `Delete` in the context menu removes the clicked clip"
    );
    assert_eq!(undo_depth(&s, &pid), d0 + 1, "U-ED-10: one undo step");
}

#[test]
fn u_ed_10_ripple_delete_on_gap_closes_it() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, v1, _, _) = gap_setup(&mut h, &s);
    let d0 = undo_depth(&s, &pid);
    // 2 s lies in the gap `[1s, 3s)`.
    let p = v1_point(&h, 2.0);
    right_click_at(&mut h, p);
    assert!(
        has_label(&h, "Ripple Delete"),
        "U-ED-10: a gap offers `Ripple Delete` (CloseGap)"
    );
    for label in ["Cut", "Copy", "Delete", "Label"] {
        assert!(
            !has_label(&h, label),
            "U-ED-10 / U-ED-9: an empty hit has no {label:?}"
        );
    }
    click_label(&mut h, "Ripple Delete", "U-ED-10");
    // SPEC C-GAP-1: the clips after the gap move left by its length, 2 s.
    assert_eq!(
        spans(&s, &pid, v1),
        vec![(0, S), (S, S)],
        "U-ED-10: `Ripple Delete` on the gap closes it"
    );
    assert_eq!(undo_depth(&s, &pid), d0 + 1, "U-ED-10: one undo step");
}

#[test]
fn u_ed_10_label_rose_sets_label_7() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, c) = one_clip(&mut h, &s);
    assert_eq!(label_of(&s, &pid, c), 0);
    let d0 = undo_depth(&s, &pid);
    let p = v1_point(&h, 2.0);
    right_click_at(&mut h, p);
    click_label(&mut h, "Label", "U-ED-10");
    for name in [
        "None",
        "Violet",
        "Iris",
        "Caribbean",
        "Lavender",
        "Cerulean",
        "Forest",
        "Rose",
        "Mango",
    ] {
        assert!(
            has_label(&h, name),
            "U-ED-10: submenu `Label` lists {name:?}"
        );
    }
    // None 0, Violet 1, Iris 2, Caribbean 3, Lavender 4, Cerulean 5, Forest 6, Rose 7, Mango 8.
    click_label(&mut h, "Rose", "U-ED-10");
    assert_eq!(
        label_of(&s, &pid, c),
        7,
        "U-ED-10: `Label > Rose` = SetLabel(7)"
    );
    assert_eq!(undo_depth(&s, &pid), d0 + 1, "U-ED-4: one batch");
}

#[test]
fn u_ed_10_add_transition_lists_effects_and_adds_at_the_cut() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, v1) = active(&h, &s);
    let x = add_named(&s, &pid, v1, json!(0), json!("2s"), "X");
    let y = add_named(&s, &pid, v1, json!("2s"), json!("2s"), "Y");
    h.run_steps(3);
    // 1.8 s is in X `[0, 2s)`, 0.2 s from its end and 1.8 s from its start: the end edge.
    let p = v1_point(&h, 1.8);
    right_click_at(&mut h, p);
    click_label(&mut h, "Add Transition", "U-ED-10");
    let listed = call(&s, "effect_list", json!({ "project_id": pid })).json;
    let transitions: Vec<String> = listed["effects"]
        .as_array()
        .expect("effects")
        .iter()
        .filter(|e| e["kind"] == "transition")
        .map(|e| e["name"].as_str().unwrap().to_string())
        .collect();
    assert!(
        transitions.iter().any(|n| n == "cross_dissolve"),
        "cross_dissolve is a transition effect: {transitions:?}"
    );
    for name in &transitions {
        assert!(
            has_label(&h, name),
            "U-ED-10: `Add Transition` on a video track lists {name:?}"
        );
    }
    click_label(&mut h, "cross_dissolve", "U-ED-10");
    let p = snapshot(&s, &pid);
    let ts = &p.track(v1).unwrap().1.transitions;
    assert_eq!(ts.len(), 1, "U-ED-10: one transition added: {ts:?}");
    assert_eq!(
        (ts[0].left, ts[0].right),
        (Some(x), Some(y)),
        "U-ED-6: end edge of X with Y starting there"
    );
    assert_eq!(ts[0].effect, "cross_dissolve");
    // min(1 s, 2 * 2 s, 2 * 2 s) = 1 s.
    assert_eq!(ts[0].duration, S, "U-ED-6: duration 1 s");
}

#[test]
fn u_ed_10_ask_ai_opens_the_wand_prompt() {
    let s = session();
    let mut h = harness(&s, Options::default());
    one_clip(&mut h, &s);
    let p = v1_point(&h, 2.0);
    right_click_at(&mut h, p);
    click_label(&mut h, "Ask AI...", "U-ED-10");
    step_until(&mut h, "U-ED-10: `Ask AI...` opens the wand prompt", |h| {
        has_label(h, "Request")
    });
}

#[test]
fn u_ed_10_copy_then_paste_duplicates_at_playhead() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, v1) = active(&h, &s);
    let c = add_named(&s, &pid, v1, json!(0), json!("1s"), "Dup");
    h.run_steps(3);
    ui_set(&mut h, &s, json!({ "selection": [c], "playhead": "60f" }));
    let d0 = undo_depth(&s, &pid);
    h.state_mut().run_action(Action::Copy);
    h.run_steps(2);
    assert_eq!(
        undo_depth(&s, &pid),
        d0,
        "U-ED-10: Copy does not edit the project"
    );
    h.state_mut().run_action(Action::Paste);
    h.run_steps(2);
    // base 0, span 1 s, playhead 60f = 2 s: the copy starts at 2 s.
    assert_eq!(
        spans(&s, &pid, v1),
        vec![(0, S), (2 * S, S)],
        "U-ED-10: Paste puts the copied clip at the playhead"
    );
    assert_eq!(undo_depth(&s, &pid), d0 + 1, "U-ED-2: one batch");
    let p = snapshot(&s, &pid);
    assert!(
        p.track(v1).unwrap().1.clips.iter().all(|c| c.name == "Dup"),
        "the copy keeps the clip's name"
    );
    let ui = h.state().ui_state().unwrap();
    assert_eq!(ui.playhead, 3 * S, "U-ED-2: playhead + span = 3 s");
    assert!(ui.selection.is_empty(), "U-ED-2: selection emptied");
}

#[test]
fn u_ed_10_paste_with_empty_clipboard_does_nothing() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, v1, _) = one_clip(&mut h, &s);
    let d0 = undo_depth(&s, &pid);
    h.state_mut().run_action(Action::Paste);
    h.run_steps(2);
    assert_eq!(
        spans(&s, &pid, v1),
        vec![(S, 2 * S)],
        "U-ED-10: nothing to paste"
    );
    assert_eq!(undo_depth(&s, &pid), d0);
}

#[test]
fn u_ed_10_clipboard_survives_switching_project_tabs() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pa, _, va) = active(&h, &s);
    let c = add_named(&s, &pa, va, json!("1s"), json!("1s"), "Dup");
    let o = call(&s, "project_new", json!({ "name": "ProjB" })).json;
    let pb = o["project_id"].as_str().unwrap().to_string();
    let vb = o["track_ids"][0].as_u64().unwrap();
    assert_eq!(
        va, vb,
        "test setup: two default projects number their V1 track alike"
    );
    h.run_steps(3);
    ui_set(&mut h, &s, json!({ "project_id": pa, "selection": [c] }));
    h.state_mut().run_action(Action::Copy);
    h.run_steps(2);
    ui_set(&mut h, &s, json!({ "project_id": pb }));
    assert_eq!(h.state().project_id(), Some(pb.as_str()));
    h.state_mut().run_action(Action::Paste);
    h.run_steps(2);
    // B's playhead is 0 and base is the clip's own start: the copy starts at 0.
    assert_eq!(
        spans(&s, &pb, vb),
        vec![(0, S)],
        "U-ED-10: the clipboard is shared by all tabs"
    );
    assert_eq!(
        snapshot(&s, &pb).track(vb).unwrap().1.clips[0].name,
        "Dup",
        "U-ED-10: the pasted clip is the copied one"
    );
    assert_eq!(spans(&s, &pa, va), vec![(S, S)], "project A untouched");
}

#[test]
fn u_ed_10_copy_and_paste_events_act_as_keys() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, v1) = active(&h, &s);
    let c = add_named(&s, &pid, v1, json!(0), json!("1s"), "Dup");
    h.run_steps(3);
    ui_set(&mut h, &s, json!({ "selection": [c], "playhead": "60f" }));
    let d0 = undo_depth(&s, &pid);
    h.event(egui::Event::Copy);
    h.run_steps(3);
    h.event(egui::Event::Paste("text from the system clipboard".into()));
    h.run_steps(3);
    assert_eq!(
        spans(&s, &pid, v1),
        vec![(0, S), (2 * S, S)],
        "U-ED-10: egui Copy then Paste events duplicate the clip at the playhead"
    );
    assert_eq!(undo_depth(&s, &pid), d0 + 1, "U-ED-10: one undo step");
}

#[test]
fn u_ed_10_cut_event_removes_selection_and_paste_brings_it_back() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, v1, c) = one_clip(&mut h, &s);
    ui_set(&mut h, &s, json!({ "selection": [c] }));
    let d0 = undo_depth(&s, &pid);
    h.event(egui::Event::Cut);
    h.run_steps(3);
    assert_eq!(
        spans(&s, &pid, v1),
        vec![],
        "U-ED-10: the egui Cut event acts as ^X"
    );
    assert_eq!(undo_depth(&s, &pid), d0 + 1);
    assert!(h.state().ui_state().unwrap().selection.is_empty());
    h.event(egui::Event::Paste("x".into()));
    h.run_steps(3);
    // Playhead 0, base = the clip's start: the 2 s clip comes back at 0.
    assert_eq!(
        spans(&s, &pid, v1),
        vec![(0, 2 * S)],
        "U-ED-10: Paste after Cut puts the clip at the playhead"
    );
}

#[test]
fn u_ed_10_clipboard_events_go_to_a_focused_text_field() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, v1, c) = one_clip(&mut h, &s);
    ui_set(&mut h, &s, json!({ "selection": [c] }));
    h.state_mut().run_action(Action::Copy);
    h.run_steps(2);
    h.state_mut().run_action(Action::WandPrompt);
    step_until(&mut h, "wand prompt text field `Request`", |h| {
        has_label(h, "Request")
    });
    h.get_by_label("Request").click();
    h.run_steps(2);
    let d0 = undo_depth(&s, &pid);
    h.event(egui::Event::Paste("tapir".into()));
    h.run_steps(3);
    assert_eq!(
        spans(&s, &pid, v1),
        vec![(S, 2 * S)],
        "U-ED-10: Paste while a text field has focus pastes no clip"
    );
    assert_eq!(
        h.get_by_label("Request").value().as_deref(),
        Some("tapir"),
        "U-ED-10: the pasted text goes into the focused field"
    );
    h.event(egui::Event::Cut);
    h.event(egui::Event::Copy);
    h.run_steps(3);
    assert_eq!(
        spans(&s, &pid, v1),
        vec![(S, 2 * S)],
        "U-ED-10: Cut while a text field has focus removes no clip"
    );
    assert_eq!(undo_depth(&s, &pid), d0, "U-ED-10: no edit at all");
}

// ------------------------------------------------------------------------------- U-ED-11

const TOOLS: [(&str, Tool); 13] = [
    ("Selection", Tool::Select),
    ("Track Select", Tool::TrackSelect),
    ("Ripple Edit", Tool::Ripple),
    ("Rolling Edit", Tool::Roll),
    ("Rate Stretch", Tool::RateStretch),
    ("Razor", Tool::Razor),
    ("Slip", Tool::Slip),
    ("Slide", Tool::Slide),
    ("Pen", Tool::Pen),
    ("Hand", Tool::Hand),
    ("Zoom", Tool::Zoom),
    ("Type", Tool::Type),
    ("Magic Wand", Tool::Wand),
];

#[test]
fn u_ed_11_every_tool_name_is_an_accessible_label() {
    let s = session();
    let h = harness(&s, Options::default());
    for (name, _) in TOOLS {
        assert!(
            has_label(&h, name),
            "U-ED-11: tool button with accessible label {name:?}"
        );
    }
    assert!(
        !has_label(&h, "Raz"),
        "U-ED-11: tool buttons show an icon, the short text `Raz` is gone"
    );
}

#[test]
fn u_ed_11_clicking_razor_sets_tool_razor() {
    let s = session();
    let mut h = harness(&s, Options::default());
    assert_eq!(ui_get(&mut h, &s)["tool"], "select");
    click_label(&mut h, "Razor", "U-ED-11");
    assert_eq!(h.state().ui_state().unwrap().tool, Tool::Razor);
    assert_eq!(
        ui_get(&mut h, &s)["tool"],
        "razor",
        "U-ED-11: clicking `Razor` selects the razor tool"
    );
}

#[test]
fn u_ed_11_each_tool_button_selects_its_tool() {
    let s = session();
    let mut h = harness(&s, Options::default());
    // `Selection` is the default, so it is clicked after another tool.
    for (name, tool) in TOOLS.iter().rev() {
        click_label(&mut h, name, "U-ED-11");
        assert_eq!(
            h.state().ui_state().unwrap().tool,
            *tool,
            "U-ED-11: button {name:?} selects {tool:?}"
        );
    }
}

// ------------------------------------------------------------------------------- U-ED-12

#[test]
fn u_ed_12_snap_button_flips_snap() {
    let s = session();
    let mut h = harness(&s, Options::default());
    assert!(has_label(&h, "Snap"), "U-ED-12: toggle button `Snap`");
    assert_eq!(
        ui_get(&mut h, &s)["snap"],
        json!(true),
        "U-ED-12: ui_get reports `snap`, on by default (U-STATE)"
    );
    click_label(&mut h, "Snap", "U-ED-12");
    assert!(
        !h.state().ui_state().unwrap().snap,
        "U-ED-12: `Snap` runs ToggleSnap"
    );
    assert_eq!(ui_get(&mut h, &s)["snap"], json!(false));
    click_label(&mut h, "Snap", "U-ED-12");
    assert!(h.state().ui_state().unwrap().snap, "second click: on again");
    assert_eq!(ui_get(&mut h, &s)["snap"], json!(true));
}

#[test]
fn u_ed_12_snap_button_is_selected_when_on() {
    let s = session();
    let mut h = harness(&s, Options::default());
    assert_eq!(
        toggled(&h, "Snap", "U-ED-12"),
        Some(Toggled::True),
        "U-ED-12: `Snap` is selected while snapping is on"
    );
    h.state_mut().run_action(Action::ToggleSnap);
    h.run_steps(2);
    assert_eq!(
        toggled(&h, "Snap", "U-ED-12"),
        Some(Toggled::False),
        "U-ED-12: `Snap` is not selected while snapping is off"
    );
}

#[test]
fn u_ed_12_ui_set_snap() {
    let s = session();
    let mut h = harness(&s, Options::default());
    ui_set(&mut h, &s, json!({ "snap": false }));
    assert!(
        !h.state().ui_state().unwrap().snap,
        "U-ED-12: ui_set {{snap: false}}"
    );
    assert_eq!(ui_get(&mut h, &s)["snap"], json!(false));
    for v in [json!({ "snap": "yes" }), json!({ "snap": 1 })] {
        ui_set_err(&mut h, &s, v, "U-ED-12 snap must be a bool");
    }
    h.run_steps(2);
    assert!(
        !h.state().ui_state().unwrap().snap,
        "a failed ui_set changes nothing"
    );
    ui_set(&mut h, &s, json!({ "snap": true }));
    assert!(h.state().ui_state().unwrap().snap);
    assert_eq!(ui_get(&mut h, &s)["snap"], json!(true));
}

/// Untitled with `Mover` `[1s, 3s)` and `Target` `[4s, 6s)` on V1. Returns (pid, v1).
fn move_setup(h: &mut Harness<'static, App>, s: &adit_mcp::Session) -> (String, Id) {
    let (pid, _, v1) = active(h, s);
    add_named(s, &pid, v1, json!("1s"), json!("2s"), "Mover");
    add_named(s, &pid, v1, json!("4s"), json!("2s"), "Target");
    h.run_steps(3);
    (pid, v1)
}

#[test]
fn u_ed_12_body_drag_lands_the_clip_end_on_the_next_clip() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, v1) = move_setup(&mut h, &s);
    let d0 = undo_depth(&s, &pid);
    assert!(h.state().ui_state().unwrap().snap);
    // Press at 2.0 s (the body), release 94 px to the right: delta 0.94 s. Mover's end would be
    // at 3.94 s, 6 px before Target's start at 4 s (tolerance 8 px): delta becomes 1 s.
    // (Snapping the pointer instead, U-FIX-4, would give 2.94 s -> frame 88 and a start of 58f.)
    let (a, b) = (v1_point(&h, 2.0), v1_point(&h, 2.94));
    drag(&mut h, a, b);
    assert_eq!(
        spans(&s, &pid, v1),
        vec![(2 * S, 2 * S), (4 * S, 2 * S)],
        "U-ED-12: Mover's end lands exactly on Target's start (4 s)"
    );
    assert_eq!(undo_depth(&s, &pid), d0 + 1, "one move = one undo step");
}

#[test]
fn u_ed_12_body_drag_with_snap_off_moves_by_whole_frames() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, v1) = move_setup(&mut h, &s);
    ui_set(&mut h, &s, json!({ "snap": false }));
    // delta 0.94 s = 28.2 frames -> 28 frames; start = 30f + 28f = 58f.
    let (a, b) = (v1_point(&h, 2.0), v1_point(&h, 2.94));
    drag(&mut h, a, b);
    assert_eq!(
        spans(&s, &pid, v1),
        vec![(58 * F30, 2 * S), (4 * S, 2 * S)],
        "U-ED-12 / U-ED-7: with snapping off the move is rounded to frames"
    );
}

// ------------------------------------------------------------------------------- U-ED-15

#[test]
fn u_ed_15_uniform_scale_checkbox_checked_by_default() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (_, _, c) = one_clip(&mut h, &s);
    ui_set(&mut h, &s, json!({ "selection": [c] }));
    click_label(&mut h, "Effect Controls", "U-ED-15");
    assert!(
        has_label(&h, "Uniform Scale"),
        "U-ED-15: Effect Controls has a checkbox `Uniform Scale`"
    );
    assert_eq!(
        toggled(&h, "Uniform Scale", "U-ED-15"),
        Some(Toggled::True),
        "U-ED-15: `Uniform Scale` is checked by default"
    );
    click_label(&mut h, "Uniform Scale", "U-ED-15");
    assert_eq!(
        toggled(&h, "Uniform Scale", "U-ED-15"),
        Some(Toggled::False),
        "U-ED-15: clicking the checkbox unchecks it"
    );
}

// ------------------------------------------------------------------------------- U-ED-16

#[test]
fn u_ed_16_dropped_files_are_imported_in_one_undo_step() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, _) = active(&h, &s);
    let dir = tempfile::tempdir().unwrap();
    let paths = [dir.path().join("lemur.mp4"), dir.path().join("gecko.mp4")];
    assert!(snapshot(&s, &pid).media.is_empty());
    let d0 = undo_depth(&s, &pid);
    drop_files(&mut h, &paths);
    step_until(&mut h, "U-ED-16: dropped files imported", |_| {
        !snapshot(&s, &pid).media.is_empty()
    });
    let p = snapshot(&s, &pid);
    assert_eq!(p.media.len(), 2, "U-ED-16: both dropped files imported");
    for (m, path) in p.media.iter().zip(&paths) {
        assert_eq!(
            Path::new(&m.spec.path).file_name(),
            path.file_name(),
            "U-ED-16: media in drop order"
        );
    }
    assert_eq!(
        undo_depth(&s, &pid),
        d0 + 1,
        "U-ED-16: one media_import = one undo step"
    );
    assert!(
        clip_ids(&p).is_empty(),
        "U-ED-16: pointer not over a track -> no clip placed"
    );
    h.run_steps(5);
    assert_eq!(
        snapshot(&s, &pid).media.len(),
        2,
        "U-ED-16: the drop is handled once"
    );
}

#[test]
fn u_ed_16_drop_over_a_track_places_the_media_end_to_end() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, v1) = active(&h, &s);
    let dir = tempfile::tempdir().unwrap();
    let paths = [dir.path().join("lemur.mp4"), dir.path().join("gecko.mp4")];
    // Pointer over V1 at 1 s: a frame boundary, 100 px from any snap candidate.
    let p = v1_point(&h, 1.0);
    h.hover_at(p);
    h.run_steps(2);
    drop_files(&mut h, &paths);
    step_until(&mut h, "U-ED-16: dropped files placed on V1", |_| {
        spans(&s, &pid, v1).len() == 2
    });
    assert_eq!(snapshot(&s, &pid).media.len(), 2, "U-ED-16: both imported");
    let got = spans(&s, &pid, v1);
    assert_eq!(got[0].0, S, "U-ED-16: the first clip at the pointer time");
    assert_eq!(
        got[1].0,
        got[0].0 + got[0].1,
        "U-ED-16: the next clip where the previous ends"
    );
}

// ------------------------------------------------------------------------------- U-ED-17

#[test]
fn u_ed_17_preview_scale_defaults_to_half() {
    let s = session();
    let mut h = harness(&s, Options::default());
    assert!(
        named(&h, "1/2"),
        "U-ED-17: the Program monitor selector shows `1/2` by default"
    );
    assert_eq!(
        ui_get(&mut h, &s)["preview_scale"].as_u64(),
        Some(2),
        "U-ED-17: ui_get reports preview_scale 2 by default"
    );
}

#[test]
fn u_ed_17_ui_set_preview_scale() {
    let s = session();
    let mut h = harness(&s, Options::default());
    ui_set(&mut h, &s, json!({ "preview_scale": 4 }));
    assert_eq!(
        ui_get(&mut h, &s)["preview_scale"].as_u64(),
        Some(4),
        "U-ED-17: ui_set {{preview_scale: 4}}"
    );
    step_until(&mut h, "U-ED-17: selector shows `1/4`", |h| named(h, "1/4"));
    ui_set(&mut h, &s, json!({ "preview_scale": 1 }));
    assert_eq!(ui_get(&mut h, &s)["preview_scale"].as_u64(), Some(1));
    step_until(&mut h, "U-ED-17: selector shows `Full`", |h| {
        named(h, "Full")
    });
    for v in [
        json!({ "preview_scale": 3 }),
        json!({ "preview_scale": 0 }),
        json!({ "preview_scale": 8 }),
    ] {
        ui_set_err(&mut h, &s, v, "U-ED-17 preview_scale must be 1, 2 or 4");
    }
    assert_eq!(
        ui_get(&mut h, &s)["preview_scale"].as_u64(),
        Some(1),
        "U-ED-17: a failed ui_set changes nothing"
    );
}

#[test]
fn u_ed_17_selector_chooses_full() {
    let s = session();
    let mut h = harness(&s, Options::default());
    click_named(&mut h, "1/2", "U-ED-17");
    assert!(named(&h, "Full"), "U-ED-17: selector offers `Full`");
    assert!(named(&h, "1/4"), "U-ED-17: selector offers `1/4`");
    click_named(&mut h, "Full", "U-ED-17");
    assert_eq!(
        ui_get(&mut h, &s)["preview_scale"].as_u64(),
        Some(1),
        "U-ED-17: choosing `Full` sets divisor 1"
    );
}

// ------------------------------------------------------------------------------- U-ED-18

#[test]
fn u_ed_18_edit_menu_ask_ai_to_change_adit_files_an_app_request() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, seq, _) = active(&h, &s);
    click_label(&mut h, "Edit", "U-ED-18");
    click_label(&mut h, "Ask AI to Change adit...", "U-ED-18");
    step_until(&mut h, "U-ED-18: prompt with field `Request`", |h| {
        has_label(h, "Request")
    });
    type_into(&mut h, "Request", "bigger timecode");
    click_label(&mut h, "Send", "U-ED-18");
    let rs = requests(&s);
    assert_eq!(rs.len(), 1, "U-ED-18: one request filed: {rs:?}");
    let r = &rs[0];
    assert_eq!(r["scope"], "app", "U-ED-18: scope app");
    assert!(
        r["panel"].is_null(),
        "U-ED-18: opened from the menu -> no panel: {r}"
    );
    assert!(r["rect"].is_null(), "U-ED-18: no rect: {r}");
    assert_eq!(r["text"], "bigger timecode");
    assert_eq!(r["project_id"], json!(pid));
    assert_eq!(r["sequence_id"].as_u64(), Some(seq));
    assert_eq!(r["status"], "pending");
}

#[test]
fn u_ed_18_wand_click_on_a_panel_names_it_in_the_request() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, _) = active(&h, &s);
    ui_set(&mut h, &s, json!({ "tool": "wand" }));
    let p = h.get_by_label("Requests").rect().center();
    click_at(&mut h, p);
    step_until(
        &mut h,
        "U-ED-18: wand click on a panel opens the prompt",
        |h| has_label(h, "Request"),
    );
    type_into(&mut h, "Request", "sort newest first");
    click_label(&mut h, "Send", "U-ED-18");
    let rs = requests(&s);
    assert_eq!(rs.len(), 1, "U-ED-18: one request filed: {rs:?}");
    let r = &rs[0];
    assert_eq!(r["scope"], "app", "U-ED-18: scope app");
    assert_eq!(
        r["panel"], "Requests",
        "U-ED-18: the request names the panel that was clicked"
    );
    assert!(
        r["rect"].is_null(),
        "U-ED-18: rect omitted for a click: {r}"
    );
    assert_eq!(r["text"], "sort newest first");
    assert_eq!(r["project_id"], json!(pid));
}

// ------------------------------------------------------------------------------- U-ED-19

#[test]
fn u_ed_19_ai_import_sends_import_request() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, seq, _) = active(&h, &s);
    let dir = tempfile::tempdir().unwrap();
    let paths: Vec<String> = ["alpha_take.mov", "beta_take.mov"]
        .iter()
        .map(|n| dir.path().join(n).to_str().unwrap().to_string())
        .collect();
    let d0 = undo_depth(&s, &pid);
    h.state_mut().ai_import(paths.clone());
    step_until(&mut h, "U-ED-19: dialog with field `Instructions`", |h| {
        has_label(h, "Instructions")
    });
    for name in ["alpha_take.mov", "beta_take.mov"] {
        assert!(has_text(&h, name), "U-ED-19: the dialog lists {name:?}");
    }
    assert!(has_label(&h, "Send"), "U-ED-19: button `Send`");
    assert!(requests(&s).is_empty(), "nothing sent before `Send`");
    type_into(&mut h, "Instructions", "rough cut of the best takes");
    click_label(&mut h, "Send", "U-ED-19");
    let rs = requests(&s);
    assert_eq!(rs.len(), 1, "U-ED-19: one request filed: {rs:?}");
    let r = &rs[0];
    assert_eq!(r["scope"], "import", "U-ED-19: scope import");
    assert_eq!(r["paths"], json!(paths), "U-ED-19: the paths, in order");
    assert_eq!(r["text"], "rough cut of the best takes");
    assert_eq!(r["project_id"], json!(pid));
    assert_eq!(r["sequence_id"].as_u64(), Some(seq));
    assert_eq!(r["status"], "pending");
    h.run_steps(5);
    assert!(
        snapshot(&s, &pid).media.is_empty(),
        "U-ED-19: nothing is imported by the window"
    );
    assert_eq!(
        undo_depth(&s, &pid),
        d0,
        "U-ED-19: the project is unchanged"
    );
    assert!(
        !has_label(&h, "Instructions"),
        "U-ED-19: the dialog closes after `Send`"
    );
}

#[test]
fn u_ed_19_file_menu_has_ai_import() {
    let s = session();
    let mut h = harness(&s, Options::default());
    assert!(!has_label(&h, "AI Import..."));
    click_label(&mut h, "File", "U-ED-19");
    assert!(
        has_label(&h, "AI Import..."),
        "U-ED-19: `File > AI Import...`"
    );
}

// ------------------------------------------------------------------------------- U-ED-20

/// A library directory holding one valid filter `teal_wash_7` (MCP_TOOLS.md T-LIB-2 file form).
fn library() -> (tempfile::TempDir, adit_mcp::Session) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("teal_wash_7.json"),
        r#"{"name":"teal_wash_7","kind":"filter","wgsl":"fn effect(uv: vec2f) -> vec4f { return src(uv); }","params":[]}"#,
    )
    .unwrap();
    let s = adit_mcp::Session::with_library(Arc::new(pattern()), dir.path().to_path_buf());
    (dir, s)
}

#[test]
fn u_ed_20_effects_browser_lists_library_effect() {
    let (_dir, s) = library();
    let mut h = harness(&s, Options::default());
    click_label(&mut h, "Effects", "U-ED-20");
    step_until(&mut h, "U-ED-20: library effect teal_wash_7 listed", |h| {
        has_text(h, "teal_wash_7")
    });
}

#[test]
fn u_ed_20_no_library_no_library_effect() {
    let s = session();
    let mut h = harness(&s, Options::default());
    click_label(&mut h, "Effects", "U-ED-20");
    h.run_steps(5);
    assert!(
        !has_text(&h, "teal_wash_7"),
        "U-ED-20 / T-LIB-1: a session without a library offers no library effect"
    );
}

#[test]
fn u_ed_20_library_effect_applies_like_any_other() {
    let (_dir, s) = library();
    let s2 = s.clone();
    // 60 steps per simulated second so two clicks make a double-click (as in fix_app.rs).
    let mut h = Harness::builder()
        .with_size(egui::vec2(W, H))
        .with_step_dt(1.0 / 60.0)
        .build_eframe(move |cc| App::new(cc, s2, Options::default()));
    h.run_steps(3);
    let (pid, _, v1) = active(&h, &s);
    let c = add_named(&s, &pid, v1, json!(0), json!("2s"), "Wash");
    h.run_steps(3);
    ui_set(&mut h, &s, json!({ "selection": [c] }));
    let d0 = undo_depth(&s, &pid);
    h.get_by_label("Effects").click();
    // 0.5 s pause so the tab click does not make the next two clicks a triple-click.
    h.run_steps(30);
    {
        let n = h
            .query_all_by_label_contains("teal_wash_7")
            .next()
            .expect("U-ED-20: library effect teal_wash_7 listed");
        n.click();
        n.click();
    }
    h.run_steps(3);
    let p = snapshot(&s, &pid);
    let fx = &p.clip(c).unwrap().2.effects;
    assert_eq!(
        fx.len(),
        1,
        "U-ED-20: the library effect is applied: {fx:?}"
    );
    assert_eq!(fx[0].effect, "teal_wash_7");
    assert_eq!(
        undo_depth(&s, &pid),
        d0 + 1,
        "U-ED-20 / T-LIB-4: one undo step"
    );
}
