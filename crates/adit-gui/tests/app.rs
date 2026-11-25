//! The window (`App`), headless through egui_kittest: U-APP-1..6, 8, 9 (docs/GUI.md "Window").
//!
//! Widget labels assumed where the contract names none (see U-APP-5 / U-APP-9 tests):
//! - the magic-wand prompt has a text field labelled `Request` (e.g. `labelled_by` a `Request`
//!   label) and a button `Send`;
//! - a clip in the timeline, a project tab and a status-bar message are accessibility nodes whose
//!   label contains the clip name / project name / message text.
//!
//! Only `ui_screenshot` needs a renderer: egui_kittest's wgpu feature, which prefers a software
//! adapter (WARP on Windows).
mod app_common;
use adit_core::{Id, MediaSpec, Time};
use adit_gui::{Action, Options};
use adit_media::{Frame, MediaBackend, MediaError, Pattern};
use app_common::*;
use eframe::egui;
use egui_kittest::kittest::Queryable;
use serde_json::json;
use std::sync::Arc;

fn ctrl() -> egui::Modifiers {
    // What egui-winit reports for Ctrl on Windows: ctrl and command both set.
    egui::Modifiers {
        ctrl: true,
        command: true,
        ..Default::default()
    }
}

fn markers(s: &adit_mcp::Session, pid: &str) -> Vec<Time> {
    snapshot(s, pid).sequences[0]
        .markers
        .iter()
        .map(|m| m.time)
        .collect()
}

// ---------------------------------------------------------------- U-APP-1

#[test]
fn u_app_1_panel_titles_and_menus() {
    let s = session();
    let h = harness(&s, Options::default());
    for label in [
        "Project",
        "Effects",
        "Source",
        "Effect Controls",
        "Program",
        "Timeline",
        "Requests",
    ] {
        assert!(
            has_label(&h, label),
            "U-APP-1: panel title {label:?} exists as text"
        );
    }
    for menu in [
        "File", "Edit", "Clip", "Sequence", "Markers", "Window", "Help",
    ] {
        assert!(has_label(&h, menu), "U-APP-1: menu {menu:?}");
    }
}

#[test]
fn u_app_1_clip_name_shown_in_timeline() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, v1) = active(&h, &s);
    assert!(!has_text(&h, "Zebra42"));
    add_named(&s, &pid, v1, json!(0), json!("2s"), "Zebra42");
    step_until(&mut h, "clip name Zebra42 in the timeline", |h| {
        has_text(h, "Zebra42")
    });
}

#[test]
fn u_app_1_one_tab_per_open_project() {
    let s = session();
    call(&s, "project_new", json!({ "name": "Okapi7" }));
    call(&s, "project_new", json!({ "name": "Quokka8" }));
    let mut h = harness(&s, Options::default());
    step_until(&mut h, "tabs for both projects", |h| {
        has_text(h, "Okapi7") && has_text(h, "Quokka8")
    });
    assert_eq!(
        s.projects().len(),
        2,
        "U-APP-2: projects open -> no Untitled created"
    );
    assert!(
        !has_text(&h, "Untitled"),
        "no tab for a project that is not open"
    );
}

// ---------------------------------------------------------------- U-APP-2

#[test]
fn u_app_2_creates_untitled() {
    let s = session();
    let h = harness(&s, Options::default());
    let ps = s.projects();
    assert_eq!(ps.len(), 1, "U-APP-2: exactly one project created");
    assert_eq!(ps[0].name, "Untitled");
    assert_eq!(h.state().project_id(), Some(ps[0].id.as_str()));
    let p = snapshot(&s, &ps[0].id);
    let ui = h.state().ui_state().expect("ui_state of the active tab");
    assert_eq!(ui.sequence_id, p.sequences[0].id);
    assert!(has_text(&h, "Untitled"), "tab for Untitled");
}

#[test]
fn u_app_2_opens_options_open() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("saved.adit");
    let path_s = path.to_str().unwrap().to_string();
    // Build and save with one session...
    let s0 = session();
    let o = call(
        &s0,
        "project_new",
        json!({ "name": "Saved9", "width": 64, "height": 48 }),
    )
    .json;
    let pid0 = o["project_id"].as_str().unwrap().to_string();
    let v1 = o["track_ids"][0].as_u64().unwrap();
    let seq = o["sequence_id"].as_u64().unwrap();
    add_named(&s0, &pid0, v1, json!(0), json!("2s"), "Giraffe3");
    call(
        &s0,
        "project_save",
        json!({ "project_id": pid0, "path": path_s }),
    );

    // ...open it in a window on a fresh session.
    let s = session();
    let mut h = harness(
        &s,
        Options {
            open: Some(path_s),
            ..Default::default()
        },
    );
    let ps = s.projects();
    assert_eq!(ps.len(), 1, "U-APP-2: the file, no Untitled");
    assert_eq!(ps[0].name, "Saved9");
    assert_eq!(h.state().project_id(), Some(ps[0].id.as_str()));
    assert_eq!(h.state().ui_state().unwrap().sequence_id, seq);
    step_until(&mut h, "clip of the opened file in the timeline", |h| {
        has_text(h, "Giraffe3")
    });
}

#[test]
fn u_app_2_missing_file_falls_back_to_untitled() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nope.adit");
    let path_s = path.to_str().unwrap().to_string();
    // The message the status bar must show: what project_open says for that path.
    let msg = session()
        .call("project_open", json!({ "path": path_s }))
        .err()
        .expect("opening a missing file fails");
    let s = session();
    let mut h = harness(
        &s,
        Options {
            open: Some(path_s),
            ..Default::default()
        },
    );
    h.run_steps(10);
    let ps = s.projects();
    assert_eq!(ps.len(), 1);
    assert_eq!(ps[0].name, "Untitled", "U-APP-2: failure -> Untitled");
    assert_eq!(h.state().project_id(), Some(ps[0].id.as_str()));
    assert!(
        has_text(&h, &msg),
        "U-APP-2: failure {msg:?} shown in the status bar"
    );
}

// ---------------------------------------------------------------- U-APP-3

#[test]
fn u_app_3_edit_by_claude_appears() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, v1) = active(&h, &s);
    h.run_steps(5);
    // No input to the window: only frames.
    add_named(&s, &pid, v1, json!("1s"), json!("1s"), "Ibis11");
    step_until(&mut h, "Claude's clip shows up", |h| has_text(h, "Ibis11"));
    call(&s, "undo", json!({ "project_id": pid }));
    step_until(&mut h, "undone clip disappears", |h| !has_text(h, "Ibis11"));
}

#[test]
fn u_app_3_projects_opened_and_closed_over_mcp() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let o = call(&s, "project_new", json!({ "name": "Narwhal5" })).json;
    let pid = o["project_id"].as_str().unwrap().to_string();
    step_until(&mut h, "tab for a project opened over MCP", |h| {
        has_text(h, "Narwhal5")
    });
    assert!(has_text(&h, "Untitled"), "existing tab stays");
    call(&s, "project_close", json!({ "project_id": pid }));
    step_until(&mut h, "tab removed after project_close", |h| {
        !has_text(h, "Narwhal5")
    });
}

#[test]
fn u_app_3_vanished_selection_ids_dropped() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, v1) = active(&h, &s);
    let a = add_named(&s, &pid, v1, json!(0), json!("1s"), "A");
    let b = add_named(&s, &pid, v1, json!("1s"), json!("1s"), "B");
    h.run_steps(3);
    h.state_mut().run_action(Action::SelectAll);
    h.run_steps(2);
    assert_eq!(h.state().ui_state().unwrap().selection, vec![a, b]);
    edit(&s, &pid, json!([{ "op": "remove_clips", "clip_ids": [a] }]));
    step_until(&mut h, "removed clip dropped from selection", |h| {
        h.state().ui_state().unwrap().selection == vec![b]
    });
}

// ---------------------------------------------------------------- U-APP-4

#[test]
fn u_app_4_run_action_select_all_delete_undo() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, v1) = active(&h, &s);
    let a = add_named(&s, &pid, v1, json!(0), json!("1s"), "A");
    let b = add_named(&s, &pid, v1, json!("2s"), json!("1s"), "B");
    h.run_steps(3);
    let d0 = undo_depth(&s, &pid);

    h.state_mut().run_action(Action::SelectAll);
    h.state_mut().run_action(Action::Delete);
    h.run_steps(2);
    assert!(
        clip_ids(&snapshot(&s, &pid)).is_empty(),
        "U-DEL through the session"
    );
    assert_eq!(undo_depth(&s, &pid), d0 + 1, "one batch = one undo step");
    assert!(h.state().ui_state().unwrap().selection.is_empty());

    h.state_mut().run_action(Action::Undo);
    h.run_steps(2);
    let mut ids = clip_ids(&snapshot(&s, &pid));
    ids.sort();
    assert_eq!(ids, vec![a, b], "U-OPS Undo -> undo tool");
    assert_eq!(undo_depth(&s, &pid), d0);

    h.state_mut().run_action(Action::Redo);
    h.run_steps(2);
    assert!(
        clip_ids(&snapshot(&s, &pid)).is_empty(),
        "U-OPS Redo -> redo tool"
    );
}

#[test]
fn u_app_4_key_m_adds_marker_and_ctrl_z_undoes() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, _) = active(&h, &s);
    let d0 = undo_depth(&s, &pid);
    h.key_press(egui::Key::M);
    h.run_steps(3);
    assert_eq!(
        markers(&s, &pid),
        vec![0],
        "U-MARKER at playhead 0 via key M"
    );
    assert_eq!(undo_depth(&s, &pid), d0 + 1);
    h.key_press_modifiers(ctrl(), egui::Key::Z);
    h.run_steps(3);
    assert!(markers(&s, &pid).is_empty(), "^Z -> Undo");
}

#[test]
fn u_app_4_arrow_right_steps_one_frame() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, v1) = active(&h, &s);
    add_named(&s, &pid, v1, json!(0), json!("2s"), "A");
    h.run_steps(3);
    h.key_press(egui::Key::ArrowRight);
    h.run_steps(2);
    // Untitled is 30 fps (project_new default): one frame = 23520000 flicks.
    assert_eq!(
        h.state().ui_state().unwrap().playhead,
        F30,
        "U-STEP via ArrowRight"
    );
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::ArrowRight);
    h.run_steps(2);
    assert_eq!(
        h.state().ui_state().unwrap().playhead,
        6 * F30,
        "⇧ArrowRight = Step(5)"
    );
}

#[test]
fn u_app_4_space_toggles_playing() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, v1) = active(&h, &s);
    add_named(&s, &pid, v1, json!(0), json!("5s"), "A");
    h.run_steps(3);
    assert!(!h.state().ui_state().unwrap().playing);
    h.key_press(egui::Key::Space);
    h.run_steps(1);
    let ui = h.state().ui_state().unwrap();
    assert!(ui.playing, "U-PLAY via Space");
    assert_eq!(ui.shuttle, 1);
    h.key_press(egui::Key::Space);
    h.run_steps(1);
    let ui = h.state().ui_state().unwrap();
    assert!(!ui.playing, "second Space stops");
    assert_eq!(ui.shuttle, 0);
}

#[test]
fn u_app_4_keys_ignored_while_text_field_has_focus() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, _) = active(&h, &s);
    h.state_mut().run_action(Action::WandPrompt);
    step_until(&mut h, "wand prompt text field `Request`", |h| {
        has_label(h, "Request")
    });
    h.get_by_label("Request").click();
    h.run_steps(2);
    h.key_press(egui::Key::M);
    h.key_press(egui::Key::Space);
    h.run_steps(3);
    assert!(
        markers(&s, &pid).is_empty(),
        "M typed into a text field adds no marker"
    );
    assert!(
        !h.state().ui_state().unwrap().playing,
        "Space in a text field does not play"
    );
}

// ---------------------------------------------------------------- U-APP-5

#[test]
fn u_app_5_wand_prompt_creates_request() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, seq, v1) = active(&h, &s);
    let c = add_named(&s, &pid, v1, json!(0), json!("4s"), "Heron4");
    h.run_steps(3);
    // Marks 30f..60f = [1 s, 2 s) at 30 fps, the clip selected.
    for a in [
        Action::GoToStart,
        Action::Step(30),
        Action::MarkIn,
        Action::Step(30),
        Action::MarkOut,
        Action::SelectAll,
    ] {
        h.state_mut().run_action(a);
    }
    h.run_steps(2);
    let ui = h.state().ui_state().unwrap();
    assert_eq!((ui.mark_in, ui.mark_out), (Some(S), Some(2 * S)));
    assert_eq!(ui.selection, vec![c]);

    h.state_mut().run_action(Action::WandPrompt);
    step_until(&mut h, "wand prompt text field `Request`", |h| {
        has_label(h, "Request")
    });
    h.get_by_label("Request").click();
    h.run_steps(2);
    h.get_by_label("Request").type_text("make it blue");
    h.run_steps(2);
    h.get_by_label("Send").click();
    h.run_steps(3);

    let rs = requests(&s);
    assert_eq!(rs.len(), 1, "U-APP-5: one request filed: {rs:?}");
    let r = &rs[0];
    assert_eq!(r["project_id"], json!(pid));
    assert_eq!(r["sequence_id"].as_u64(), Some(seq));
    assert_eq!(r["text"], "make it blue");
    assert_eq!(req_time(r, "start"), S, "start = mark_in");
    assert_eq!(req_time(r, "end"), 2 * S, "end = mark_out");
    assert_eq!(r["clip_ids"], json!([c]), "clip_ids = selection");
    assert!(
        r["rect"].is_null(),
        "no Program-monitor rectangle drawn -> no rect"
    );
    assert_eq!(r["status"], "pending");

    // The Requests panel lists it, and later the completion message.
    step_until(&mut h, "request text in the Requests panel", |h| {
        has_text(h, "make it blue")
    });
    let rid = r["request_id"].clone();
    call(
        &s,
        "request_complete",
        json!({ "request_id": rid, "message": "Recoloured Heron4" }),
    );
    step_until(&mut h, "completion message in the Requests panel", |h| {
        has_text(h, "Recoloured Heron4")
    });
}

// ---------------------------------------------------------------- U-APP-6

#[test]
fn u_app_6_status_bar_shows_mcp_url() {
    let s = session();
    let url = "http://127.0.0.1:7999/mcp";
    let h = harness(
        &s,
        Options {
            mcp_url: Some(url.into()),
            mcp_token: Some("0123456789abcdef0123456789abcdef".into()),
            ..Default::default()
        },
    );
    assert!(has_text(&h, url), "U-APP-6: mcp_url in the status bar");
}

// ---------------------------------------------------------------- U-APP-8

#[test]
fn u_app_8_ui_get_describes_active_tab() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, seq, _) = active(&h, &s);
    let v = call_stepping(&mut h, &s, "ui_get", json!({}))
        .expect("U-APP-8: window registered a bridge")
        .json;
    assert_eq!(v["project_id"], json!(pid));
    assert_eq!(v["sequence_id"].as_u64(), Some(seq));
    assert_eq!(v["playhead"].as_i64(), Some(0));
    assert_eq!(v["playhead_s"].as_f64(), Some(0.0));
    assert_eq!(v["selection"], json!([]));
    assert!(v["mark_in"].is_null() && v["mark_out"].is_null(), "{v}");
    assert!(
        v["tool"]
            .as_str()
            .is_some_and(|t| t.eq_ignore_ascii_case("select")),
        "tool {v}"
    );
    assert_eq!(v["playing"], json!(false));
}

#[test]
fn u_app_8_ui_set_moves_playhead_and_marks() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, v1) = active(&h, &s);
    let c = add_named(&s, &pid, v1, json!(0), json!("2s"), "A");
    h.run_steps(3);
    call_stepping(&mut h, &s, "ui_set", json!({ "playhead": "15f" })).expect("ui_set");
    h.run_steps(2);
    assert_eq!(
        h.state().ui_state().unwrap().playhead,
        15 * F30,
        "U-APP-8: 15f at 30 fps"
    );

    call_stepping(
        &mut h,
        &s,
        "ui_set",
        // 1 frame + 5 flicks rounds to 1 frame; marks 10f, 20f.
        json!({ "playhead": F30 + 5, "mark_in": "10f", "mark_out": "20f", "selection": [c] }),
    )
    .expect("ui_set");
    h.run_steps(2);
    let ui = h.state().ui_state().unwrap();
    assert_eq!(ui.playhead, F30, "times rounded to frames");
    assert_eq!((ui.mark_in, ui.mark_out), (Some(10 * F30), Some(20 * F30)));
    assert_eq!(ui.selection, vec![c]);

    let v = call_stepping(&mut h, &s, "ui_get", json!({})).unwrap().json;
    assert_eq!(v["playhead"].as_i64(), Some(F30));
    assert_eq!(v["selection"], json!([c]));
}

#[test]
fn u_app_8_ui_set_bad_ids_are_errors() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let before = h.state().ui_state().unwrap().clone();
    let e = call_stepping(&mut h, &s, "ui_set", json!({ "project_id": "p999" }))
        .err()
        .expect("U-APP-8: unknown project_id -> error");
    assert!(!e.trim().is_empty());
    let e = call_stepping(&mut h, &s, "ui_set", json!({ "selection": [999_999] }))
        .err()
        .expect("U-APP-8: non-existent clip id -> error");
    assert!(!e.trim().is_empty());
    h.run_steps(2);
    assert_eq!(
        h.state().ui_state().unwrap(),
        &before,
        "failed ui_set changes nothing"
    );
}

/// Reading: `project_id` in `ui_set` switches the active tab.
#[test]
fn u_app_8_ui_set_project_id_switches_tab() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let o = call(&s, "project_new", json!({ "name": "Second" })).json;
    let p2 = o["project_id"].as_str().unwrap().to_string();
    h.run_steps(3);
    call_stepping(&mut h, &s, "ui_set", json!({ "project_id": p2 })).expect("ui_set");
    h.run_steps(2);
    assert_eq!(h.state().project_id(), Some(p2.as_str()));
    assert_eq!(
        h.state().ui_state().unwrap().sequence_id,
        o["sequence_id"].as_u64().unwrap()
    );
}

/// Needs egui_kittest's `wgpu` renderer (software adapter preferred) to paint a frame.
#[test]
fn u_app_8_ui_screenshot_is_png_of_window() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let o = call_stepping(&mut h, &s, "ui_screenshot", json!({})).expect("ui_screenshot");
    assert_eq!(o.images.len(), 1, "one image block");
    assert_eq!(o.images[0].mime, "image/png");
    let img = image::load_from_memory_with_format(&o.images[0].bytes, image::ImageFormat::Png)
        .expect("PNG decodes");
    assert_eq!(
        (img.width(), img.height()),
        (W as u32, H as u32),
        "whole window at 1 px per point"
    );
}

// ---------------------------------------------------------------- U-APP-9

#[test]
fn u_app_9_rejected_batch_goes_to_status_bar() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, v1) = active(&h, &s);
    let c = add_named(&s, &pid, v1, json!(0), json!("1s"), "A");
    edit(
        &s,
        &pid,
        json!([{ "op": "set_track", "track_id": v1, "locked": true }]),
    );
    h.run_steps(3);
    let d0 = undo_depth(&s, &pid);
    h.state_mut().run_action(Action::SelectAll);
    h.state_mut().run_action(Action::Delete);
    h.run_steps(5);
    assert_eq!(
        clip_ids(&snapshot(&s, &pid)),
        vec![c],
        "rejected batch changes nothing"
    );
    assert_eq!(undo_depth(&s, &pid), d0);
    // SPEC G5: TrackLocked, whose text is "track <id> is locked".
    let msg = format!("track {v1} is locked");
    assert!(has_text(&h, &msg), "U-APP-9: {msg:?} in the status bar");
    // Still alive and responsive.
    h.run_steps(10);
    h.state_mut().run_action(Action::AddMarker);
    h.run_steps(2);
    assert_eq!(markers(&s, &pid).len(), 1);
}

#[test]
fn u_app_9_failing_mcp_call_does_not_disturb_window() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _, _) = active(&h, &s);
    for _ in 0..3 {
        let r = s.call(
            "edit",
            json!({ "project_id": pid, "commands": [{ "op": "add_clip", "track_id": 999_999,
                "source": solid([1.0, 1.0, 1.0, 1.0]), "start": 0, "duration": "1s" }] }),
        );
        assert!(r.is_err());
        assert!(s.call("no_such_tool", json!({})).is_err());
        h.run_steps(5);
    }
    assert_eq!(h.state().project_id(), Some(pid.as_str()));
    h.state_mut().run_action(Action::AddMarker);
    h.run_steps(2);
    assert_eq!(markers(&s, &pid).len(), 1);
}

/// Pattern that probes fine but cannot read pictures or sound of any path containing "missing".
struct MissingMedia(Pattern);

impl MediaBackend for MissingMedia {
    fn probe(&self, path: &str) -> Result<MediaSpec, MediaError> {
        self.0.probe(path)
    }
    fn video_frame(&self, path: &str, time: Time) -> Result<Arc<Frame>, MediaError> {
        if path.contains("missing") {
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
        if path.contains("missing") {
            return Err(MediaError::NotFound(path.into()));
        }
        self.0.audio(path, start, duration, sample_rate, filter)
    }
}

#[test]
fn u_app_9_missing_media_and_failed_renders_do_not_panic() {
    let s = session_with(Arc::new(MissingMedia(pattern())));
    let dir = tempfile::tempdir().unwrap();
    let mut h = harness(&s, Options::default());
    let (pid, _, v1) = active(&h, &s);
    let file = dir.path().join("missing.mov");
    let o = call(
        &s,
        "media_import",
        json!({ "project_id": pid, "paths": [file.to_str().unwrap()] }),
    )
    .json;
    let m: Id = o["media"][0]["id"].as_u64().unwrap();
    let a1 = snapshot(&s, &pid).sequences[0]
        .tracks
        .iter()
        .find(|t| t.kind == adit_core::TrackKind::Audio)
        .unwrap()
        .id;
    edit(
        &s,
        &pid,
        json!([{ "op": "add_clip", "track_id": v1, "linked_track_id": a1,
                 "source": { "type": "media", "media_id": m }, "start": 0, "duration": "3s" }]),
    );
    // Program monitor renders the broken clip, the Source monitor loads it, playback mixes it.
    h.run_steps(20);
    h.state_mut().run_action(Action::MatchFrame);
    h.run_steps(20);
    h.state_mut().run_action(Action::PlayPause);
    for _ in 0..30 {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    h.state_mut().run_action(Action::PlayPause);
    h.run_steps(5);
    // Still responsive.
    let n = markers(&s, &pid).len();
    h.state_mut().run_action(Action::AddMarker);
    h.run_steps(2);
    assert_eq!(markers(&s, &pid).len(), n + 1);
}
