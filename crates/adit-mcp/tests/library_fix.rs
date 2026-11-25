//! Rules added after the review: T-LIB-7 (docs/MCP_TOOLS.md).
//! Written from the contract only; expected values are hand-computed literals.
mod common;
use adit_mcp::Session;
use adit_media::Pattern;
use common::*;
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Arc;

const LIMIT: usize = 1_048_576;

fn lib_session(dir: &Path) -> Session {
    Session::with_library(
        Arc::new(Pattern {
            width: 64,
            height: 64,
            fps: adit_core::Rational { num: 30, den: 1 },
            duration: Some(10 * S),
        }),
        dir.to_path_buf(),
    )
}

fn swap_def_max(name: &str, max: f64) -> Value {
    let mut d = swap_def(name);
    d["params"][0]["max"] = json!(max);
    d
}

fn put(dir: &Path, def: &Value) {
    std::fs::create_dir_all(dir).unwrap();
    let name = def["name"].as_str().unwrap();
    std::fs::write(dir.join(format!("{name}.json")), def.to_string()).unwrap();
}

fn files(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect(),
        Err(_) => Vec::new(),
    };
    v.sort();
    v
}

fn define(s: &Session, pid: &str, def: Value) -> Value {
    edit(s, pid, json!([{ "op": "define_effect", "def": def }]))
}

fn effects(s: &Session) -> Vec<Value> {
    call(s, "effect_list", json!({})).json["effects"]
        .as_array()
        .expect("effects array")
        .clone()
}

fn library_names(s: &Session) -> Vec<String> {
    effects(s)
        .iter()
        .filter(|e| e["origin"] == "library")
        .map(|e| e["name"].as_str().unwrap().to_string())
        .collect()
}

fn undo_depth(s: &Session, pid: &str) -> u64 {
    depths(s, pid).0
}

/// A valid filter whose JSON text is exactly `len` bytes (padded with a WGSL comment).
/// Writes a valid def padded with trailing JSON whitespace to exactly `len` bytes. (Padding the
/// WGSL would break SPEC H-1's 65,536-byte limit, which is not what T-LIB-7 is about.)
fn put_padded(dir: &Path, name: &str, len: usize) {
    std::fs::create_dir_all(dir).unwrap();
    let mut text = swap_def(name).to_string();
    assert!(len > text.len());
    text.push_str(&" ".repeat(len - text.len()));
    assert_eq!(text.len(), len, "test setup: file size");
    std::fs::write(dir.join(format!("{name}.json")), text).unwrap();
}

// ---------------------------------------------------------------- T-LIB-7 device names

#[test]
fn t_lib_7_device_names_are_not_saved_but_the_edit_succeeds() {
    for name in ["con", "NUL", "Com1", "lpt9", "PRN", "aux", "CON", "COM9"] {
        let tmp = tempfile::tempdir().unwrap();
        let s = lib_session(tmp.path());
        let p = new_project(&s, json!({}));
        let r = define(&s, &p.id, swap_def(name));
        assert_eq!(
            r["library"],
            json!([]),
            "T-LIB-7: device name {name:?} is left out of `library`"
        );
        assert_eq!(
            files(tmp.path()),
            [""; 0],
            "T-LIB-7: no file for device name {name:?}"
        );
        assert_eq!(
            summary(&s, &p.id)["effects"],
            json!([name]),
            "T-LIB-7: the edit still defines {name:?} in the project"
        );
        assert!(
            library_names(&s).is_empty(),
            "T-LIB-7: {name:?} is not listed from the library"
        );
    }
}

#[test]
fn t_lib_7_near_device_names_are_saved() {
    let tmp = tempfile::tempdir().unwrap();
    let s = lib_session(tmp.path());
    let p = new_project(&s, json!({}));
    let r = edit(
        &s,
        &p.id,
        json!([
            { "op": "define_effect", "def": swap_def("COM10") },
            { "op": "define_effect", "def": swap_def("console") },
            { "op": "define_effect", "def": swap_def("con") }
        ]),
    );
    assert_eq!(
        r["library"],
        json!(["COM10", "console"]),
        "T-LIB-7: only the exact device names COM1..COM9 etc. are refused"
    );
    assert_eq!(
        files(tmp.path()),
        ["COM10.json", "console.json"],
        "T-LIB-7: files of COM10 and console"
    );
}

// ---------------------------------------------------------------- T-LIB-7 case clash

#[test]
fn t_lib_7_case_clash_saves_nothing_and_exact_name_replaces() {
    let tmp = tempfile::tempdir().unwrap();
    put(tmp.path(), &swap_def_max("Glow", 2.0));
    let before = std::fs::read(tmp.path().join("Glow.json")).unwrap();
    let s = lib_session(tmp.path());
    let p = new_project(&s, json!({}));

    let r = define(&s, &p.id, swap_def_max("glow", 9.0));
    assert_eq!(
        r["library"],
        json!([]),
        "T-LIB-7: `glow` clashes with Glow.json ignoring case, not saved"
    );
    assert_eq!(
        files(tmp.path()),
        ["Glow.json"],
        "T-LIB-7: no second file next to Glow.json"
    );
    assert_eq!(
        std::fs::read(tmp.path().join("Glow.json")).unwrap(),
        before,
        "T-LIB-7: Glow.json still holds the old definition"
    );
    assert_eq!(
        summary(&s, &p.id)["effects"],
        json!(["glow"]),
        "T-LIB-7: the edit itself succeeds"
    );

    let p2 = new_project(&s, json!({}));
    let r = define(&s, &p2.id, swap_def_max("Glow", 5.0));
    assert_eq!(
        r["library"],
        json!(["Glow"]),
        "T-LIB-7: the exact name replaces the file"
    );
    assert_eq!(files(tmp.path()), ["Glow.json"], "T-LIB-7: still one file");
    let v: Value =
        serde_json::from_slice(&std::fs::read(tmp.path().join("Glow.json")).unwrap()).unwrap();
    assert_eq!(v["params"][0]["max"], json!(5.0), "T-LIB-7: replaced");
}

// ---------------------------------------------------------------- T-LIB-7 size limit

#[test]
fn t_lib_7_file_over_1_mib_is_not_listed_and_cannot_be_used() {
    let tmp = tempfile::tempdir().unwrap();
    put_padded(tmp.path(), "huge", LIMIT + 1);
    put_padded(tmp.path(), "fits", LIMIT - 1);
    put_padded(tmp.path(), "exact", LIMIT);
    let s = lib_session(tmp.path());
    assert_eq!(
        std::fs::metadata(tmp.path().join("huge.json"))
            .unwrap()
            .len(),
        1_048_577,
        "test setup: 1,048,577 bytes"
    );
    let mut names = library_names(&s);
    names.sort();
    assert_eq!(
        names,
        ["exact", "fits"],
        "T-LIB-7: files of more than 1,048,576 bytes are skipped by effect_list"
    );

    let p = new_project(&s, json!({}));
    let c = add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    let d0 = undo_depth(&s, &p.id);
    edit_err(
        &s,
        &p.id,
        json!([{ "op": "add_effect", "clip_id": c, "effect": "huge" }]),
    );
    assert_eq!(
        undo_depth(&s, &p.id),
        d0,
        "T-LIB-7: a failed add_effect changes nothing"
    );
    edit(
        &s,
        &p.id,
        json!([{ "op": "add_effect", "clip_id": c, "effect": "fits" }]),
    );
    assert_eq!(
        summary(&s, &p.id)["effects"],
        json!(["fits"]),
        "T-LIB-7: a file just under the limit still works"
    );
}

// ---------------------------------------------------------------- T-LIB-7 paste_clips filter

/// Project with one solid at [0, 1s) and a copy of its full JSON moved to 2 s with `filter`.
fn paste_setup(s: &Session, filter: &str) -> (Proj, Value) {
    let p = new_project(s, json!({}));
    let c = add_solid(
        s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    let mut clip = find_clip(&project(s, &p.id), c);
    clip["start"] = json!(2 * S);
    clip["audio_filter"] = json!(filter);
    let item = json!({ "track_id": p.v1(), "clip": clip });
    (p, item)
}

fn clip_count(s: &Session, p: &Proj) -> usize {
    find_track(&project(s, &p.id), p.v1())["clips"]
        .as_array()
        .unwrap()
        .len()
}

#[test]
fn t_lib_7_paste_clips_with_rejected_audio_filter_fails_and_changes_nothing() {
    let s = session();
    for bad in [
        "ametadata=mode=print:file=leak.txt",
        "volume=0.5;anullsink",
        "amovie=x.wav",
    ] {
        let (p, item) = paste_setup(&s, bad);
        let (d0, n0) = (undo_depth(&s, &p.id), clip_count(&s, &p));
        let before = project(&s, &p.id);
        edit_err(&s, &p.id, json!([{ "op": "paste_clips", "items": [item] }]));
        assert_eq!(
            undo_depth(&s, &p.id),
            d0,
            "T-LIB-7: rejected filter {bad:?}: undo depth unchanged"
        );
        assert_eq!(
            clip_count(&s, &p),
            n0,
            "T-LIB-7: rejected filter {bad:?}: clip count unchanged"
        );
        assert_eq!(
            project(&s, &p.id),
            before,
            "T-LIB-7: rejected filter {bad:?}: nothing changed"
        );
    }
}

#[test]
fn t_lib_7_paste_clips_with_accepted_audio_filter_pastes() {
    let s = session();
    let (p, item) = paste_setup(&s, "volume=0.5");
    let d0 = undo_depth(&s, &p.id);
    let r = edit(&s, &p.id, json!([{ "op": "paste_clips", "items": [item] }]));
    assert_eq!(
        r["created"].as_array().map(Vec::len),
        Some(1),
        "T-LIB-7: one command, one `created` entry: {r}"
    );
    assert_eq!(undo_depth(&s, &p.id), d0 + 1, "T-LIB-7: one undo step");
    assert_eq!(clip_count(&s, &p), 2, "T-LIB-7: the copy exists");
    let id = r["created"][0][0].as_u64().expect("new clip id");
    assert_eq!(
        find_clip(&project(&s, &p.id), id)["audio_filter"],
        json!("volume=0.5"),
        "T-LIB-7: `volume=0.5` is accepted as set_clip accepts it"
    );
}
