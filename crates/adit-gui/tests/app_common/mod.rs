//! Helpers for the headless window tests (tests/app.rs). Contract: docs/GUI.md U-APP-*.
//! Every expected value in the tests is worked out by hand from the contract.
#![allow(dead_code)]

use adit_core::{Project, Rational};
use adit_gui::{App, Options};
use adit_mcp::{Session, ToolOutput};
use adit_media::{MediaBackend, Pattern};
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::mpsc::sync_channel;
use std::time::{Duration, Instant};

/// Flicks per second.
pub const S: i64 = 705_600_000;
/// One frame at 30 fps.
pub const F30: i64 = 23_520_000;
/// Window size used by every test.
pub const W: f32 = 1600.0;
pub const H: f32 = 1000.0;

pub fn pattern() -> Pattern {
    Pattern {
        width: 64,
        height: 64,
        fps: Rational { num: 30, den: 1 },
        duration: Some(10 * S),
    }
}

pub fn session() -> Session {
    Session::new(Arc::new(pattern()))
}

pub fn session_with(media: Arc<dyn MediaBackend>) -> Session {
    Session::new(media)
}

/// Builds the window headlessly (no GPU needed unless a test renders) and runs 3 frames.
pub fn harness(s: &Session, options: Options) -> Harness<'static, App> {
    let s2 = s.clone();
    let mut h = Harness::builder()
        .with_size(egui::vec2(W, H))
        .build_eframe(move |cc| App::new(cc, s2, options));
    h.run_steps(3);
    h
}

/// Runs frames (with a short sleep, so background renders can land) until `cond` holds.
pub fn step_until(h: &mut Harness<'static, App>, what: &str, cond: impl Fn(&Harness<App>) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        h.step();
        if cond(h) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "timed out after 20 s waiting for: {what}"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

pub fn has_label(h: &Harness<App>, text: &str) -> bool {
    h.query_all_by_label(text).next().is_some()
}

pub fn has_text(h: &Harness<App>, part: &str) -> bool {
    h.query_all_by_label_contains(part).next().is_some()
}

/// Calls a tool from another thread while the window keeps running frames, so a bridge that
/// answers on the UI thread (e.g. `ui_screenshot` waiting for the next painted frame) can.
pub fn call_stepping(
    h: &mut Harness<'static, App>,
    s: &Session,
    tool: &str,
    args: Value,
) -> Result<ToolOutput, String> {
    let (tx, rx) = sync_channel(1);
    let (s2, t2) = (s.clone(), tool.to_string());
    std::thread::spawn(move || {
        let _ = tx.send(s2.call(&t2, args));
    });
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Ok(r) = rx.try_recv() {
            h.step();
            return r;
        }
        assert!(
            Instant::now() < deadline,
            "{tool} did not return within 20 s"
        );
        h.step();
        std::thread::sleep(Duration::from_millis(5));
    }
}

pub fn call(s: &Session, tool: &str, args: Value) -> ToolOutput {
    match s.call(tool, args.clone()) {
        Ok(o) => o,
        Err(e) => panic!("{tool} {args} failed: {e}"),
    }
}

pub fn edit(s: &Session, pid: &str, commands: Value) -> Value {
    call(
        s,
        "edit",
        json!({ "project_id": pid, "commands": commands }),
    )
    .json
}

pub fn solid(rgba: [f64; 4]) -> Value {
    json!({ "type": "solid", "color": rgba })
}

/// Adds a named solid clip; returns its id.
pub fn add_named(s: &Session, pid: &str, track: u64, start: Value, dur: Value, name: &str) -> u64 {
    let r = edit(
        s,
        pid,
        json!([{ "op": "add_clip", "track_id": track, "source": solid([0.2, 0.4, 0.6, 1.0]),
                 "start": start, "duration": dur, "name": name }]),
    );
    r["created"][0][0].as_u64().expect("created[0][0]")
}

pub fn snapshot(s: &Session, pid: &str) -> Project {
    s.snapshot(pid)
        .unwrap_or_else(|| panic!("no project {pid}"))
        .0
}

pub fn clip_ids(p: &Project) -> Vec<u64> {
    p.sequences
        .iter()
        .flat_map(|q| q.tracks.iter())
        .flat_map(|t| t.clips.iter().map(|c| c.id))
        .collect()
}

/// (project id, sequence id, V1 track id) of the active tab.
pub fn active(h: &Harness<App>, s: &Session) -> (String, u64, u64) {
    let pid = h.state().project_id().expect("active project").to_string();
    let p = snapshot(s, &pid);
    let seq = &p.sequences[0];
    let v1 = seq
        .tracks
        .iter()
        .find(|t| t.kind == adit_core::TrackKind::Video)
        .expect("a video track")
        .id;
    (pid, seq.id, v1)
}

pub fn undo_depth(s: &Session, pid: &str) -> u64 {
    call(
        s,
        "project_get",
        json!({ "project_id": pid, "detail": "full" }),
    )
    .json["undo_depth"]
        .as_u64()
        .unwrap()
}

/// `request_list` as an array (accepts the bare-array form too).
pub fn requests(s: &Session) -> Vec<Value> {
    let j = call(s, "request_list", json!({})).json;
    if let Some(a) = j.as_array() {
        return a.clone();
    }
    j["requests"].as_array().expect("requests").clone()
}

/// A time field of a request in flicks: raw `name`, else `name_s` seconds.
pub fn req_time(r: &Value, name: &str) -> i64 {
    if let Some(t) = r[name].as_i64() {
        return t;
    }
    let sec = r[format!("{name}_s")]
        .as_f64()
        .unwrap_or_else(|| panic!("request has no {name} / {name}_s: {r}"));
    (sec * S as f64).round() as i64
}
