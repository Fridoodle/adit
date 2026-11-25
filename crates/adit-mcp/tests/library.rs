//! Effect library T-LIB-1..5 and request scopes T-REQ-4 (docs/MCP_TOOLS.md).
//! Written from the contract only; expected values are hand-computed literals.
mod common;
use adit_core::{EffectDef, EffectKind, Rational};
use adit_mcp::Session;
use adit_media::Pattern;
use common::*;
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

const FADE_WGSL: &str = "fn transition(uv: vec2f) -> vec4f { \
    return mix(src_a(uv), src_b(uv), ctx.progress); }";

fn pattern() -> Arc<Pattern> {
    Arc::new(Pattern {
        width: 64,
        height: 64,
        fps: Rational { num: 30, den: 1 },
        duration: Some(10 * S),
    })
}

fn lib_session(dir: &Path) -> Session {
    Session::with_library(pattern(), dir.to_path_buf())
}

fn fade_def(name: &str) -> Value {
    json!({ "name": name, "kind": "transition", "wgsl": FADE_WGSL, "params": [] })
}

/// `swap_def` with another `gain` maximum, to tell two definitions of one name apart.
fn swap_def_max(name: &str, max: f64) -> Value {
    let mut d = swap_def(name);
    d["params"][0]["max"] = json!(max);
    d
}

/// Puts a definition into the library by hand, as `<dir>/<name>.json`.
fn put(dir: &Path, def: &Value) {
    std::fs::create_dir_all(dir).unwrap();
    let name = def["name"].as_str().unwrap();
    std::fs::write(dir.join(format!("{name}.json")), def.to_string()).unwrap();
}

/// File names in `dir`, sorted; empty when the directory does not exist.
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

fn read_def(path: &Path) -> EffectDef {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("T-LIB-2: {} must exist: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("T-LIB-2: {} must be EffectDef JSON: {e}", path.display()))
}

fn as_def(v: &Value) -> EffectDef {
    serde_json::from_value(v.clone()).expect("test definition is an EffectDef")
}

fn define(s: &Session, pid: &str, def: Value) -> Value {
    edit(s, pid, json!([{ "op": "define_effect", "def": def }]))
}

fn effects(s: &Session, args: Value) -> Vec<Value> {
    call(s, "effect_list", args).json["effects"]
        .as_array()
        .expect("effects array")
        .clone()
}

fn named<'a>(l: &'a [Value], name: &str) -> Vec<&'a Value> {
    l.iter().filter(|e| e["name"] == name).collect()
}

fn project_effect_names(s: &Session, pid: &str) -> Value {
    summary(s, pid)["effects"].clone()
}

fn undo_depth(s: &Session, pid: &str) -> u64 {
    depths(s, pid).0
}

/// Two adjacent 2 s solids on V1; returns their ids.
fn two_clips(s: &Session, p: &Proj) -> (u64, u64) {
    let a = add_solid(
        s,
        &p.id,
        p.v1(),
        json!(0),
        json!("2s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    let b = add_solid(
        s,
        &p.id,
        p.v1(),
        json!("2s"),
        json!("2s"),
        [0.0, 0.0, 1.0, 1.0],
    );
    (a, b)
}

fn transitions(s: &Session, p: &Proj) -> Vec<Value> {
    find_track(&project(s, &p.id), p.v1())["transitions"]
        .as_array()
        .expect("transitions array")
        .clone()
}

// ---------------------------------------------------------------- T-LIB-1

#[test]
fn t_lib_1_session_new_saves_and_offers_nothing() {
    let s = session();
    let p = new_project(&s, json!({}));
    let r = define(&s, &p.id, swap_def("swap"));
    assert_eq!(
        r["library"],
        json!([]),
        "T-LIB-2: `library` is [] without a library"
    );
    for args in [json!({}), json!({ "project_id": p.id })] {
        let l = effects(&s, args);
        assert!(
            l.iter().all(|e| e["origin"] != "library"),
            "T-LIB-1: Session::new offers no library effect"
        );
    }
    // Another project cannot get the effect from anywhere.
    let p2 = new_project(&s, json!({}));
    let c = add_solid(
        &s,
        &p2.id,
        p2.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    edit_err(
        &s,
        &p2.id,
        json!([{ "op": "add_effect", "clip_id": c, "effect": "swap" }]),
    );
    assert_eq!(
        project_effect_names(&s, &p2.id),
        json!([]),
        "T-LIB-1: nothing is offered without a library"
    );
}

#[test]
fn t_lib_1_directory_is_created_on_first_save() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("effects");
    let s = lib_session(&dir);
    let p = new_project(&s, json!({}));
    let l = effects(&s, json!({ "project_id": p.id }));
    assert!(
        l.iter().all(|e| e["origin"] != "library"),
        "T-LIB-3: a missing directory is an empty library"
    );
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    assert!(
        !dir.exists(),
        "T-LIB-1: the directory is created on the first save, not before"
    );
    let r = define(&s, &p.id, swap_def("swap"));
    assert_eq!(r["library"], json!(["swap"]), "T-LIB-2: saved name");
    assert!(dir.is_dir(), "T-LIB-1: first save creates the directory");
    assert_eq!(files(&dir), ["swap.json"], "T-LIB-2: <dir>/<name>.json");
}

// ---------------------------------------------------------------- T-LIB-2

#[test]
fn t_lib_2_define_effect_is_saved_as_json() {
    let tmp = tempfile::tempdir().unwrap();
    let s = lib_session(tmp.path());
    let p = new_project(&s, json!({}));
    let r = define(&s, &p.id, swap_def("swap"));
    assert_eq!(r["library"], json!(["swap"]), "T-LIB-2: library names");
    assert_eq!(r["created"], json!([[]]), "T-EDIT: created unchanged");
    assert_eq!(files(tmp.path()), ["swap.json"], "T-LIB-2: file name");

    let def = read_def(&tmp.path().join("swap.json"));
    assert_eq!(
        def,
        as_def(&swap_def("swap")),
        "T-LIB-2: file = the EffectDef"
    );
    assert_eq!(def.name, "swap", "T-LIB-2: name");
    assert_eq!(def.kind, EffectKind::Filter, "T-LIB-2: kind");
    assert_eq!(def.wgsl, SWAP_WGSL, "T-LIB-2: wgsl");
    assert_eq!(def.params.len(), 1, "T-LIB-2: params");
    assert_eq!(def.params[0].name, "gain", "T-LIB-2: param name");
    assert_eq!(
        (def.params[0].default, def.params[0].min, def.params[0].max),
        (1.0, 0.0, 2.0),
        "T-LIB-2: param values"
    );
}

#[test]
fn t_lib_2_library_lists_saved_names_in_command_order() {
    let tmp = tempfile::tempdir().unwrap();
    let s = lib_session(tmp.path());
    let p = new_project(&s, json!({}));
    let r = edit(
        &s,
        &p.id,
        json!([
            { "op": "define_effect", "def": swap_def("zeta") },
            { "op": "add_clip", "track_id": p.v1(), "source": solid([1.0, 0.0, 0.0, 1.0]),
              "start": 0, "duration": "1s" },
            { "op": "define_effect", "def": fade_def("alpha") },
            { "op": "define_effect", "def": swap_def("mid") }
        ]),
    );
    assert_eq!(
        r["library"],
        json!(["zeta", "alpha", "mid"]),
        "T-LIB-2: names saved, in command order"
    );
    assert_eq!(
        r["created"].as_array().map(Vec::len),
        Some(4),
        "T-EDIT: one `created` entry per command"
    );
    assert_eq!(
        files(tmp.path()),
        ["alpha.json", "mid.json", "zeta.json"],
        "T-LIB-2: one file per definition"
    );
    assert_eq!(
        read_def(&tmp.path().join("alpha.json")),
        as_def(&fade_def("alpha")),
        "T-LIB-2: a transition is saved too"
    );
}

#[test]
fn t_lib_2_edit_without_define_effect_has_empty_library() {
    let tmp = tempfile::tempdir().unwrap();
    let s = lib_session(tmp.path());
    let p = new_project(&s, json!({}));
    let r = edit(
        &s,
        &p.id,
        json!([{ "op": "add_clip", "track_id": p.v1(), "source": solid([1.0, 0.0, 0.0, 1.0]),
                 "start": 0, "duration": "1s" }]),
    );
    assert_eq!(r["library"], json!([]), "T-LIB-2: [] when none");
    assert_eq!(files(tmp.path()), [""; 0], "T-LIB-2: nothing written");
}

#[test]
fn t_lib_2_define_in_nested_batch_is_saved() {
    let tmp = tempfile::tempdir().unwrap();
    let s = lib_session(tmp.path());
    let p = new_project(&s, json!({}));
    let r = edit(
        &s,
        &p.id,
        json!([
            { "op": "define_effect", "def": swap_def("top") },
            { "op": "batch", "commands": [
                { "op": "define_effect", "def": swap_def("depth1") },
                { "op": "batch", "commands": [
                    { "op": "define_effect", "def": fade_def("depth2") }
                ] }
            ] }
        ]),
    );
    assert_eq!(
        r["library"],
        json!(["top", "depth1", "depth2"]),
        "T-LIB-2: define_effect at any batch depth, in command order"
    );
    assert_eq!(
        r["created"].as_array().map(Vec::len),
        Some(2),
        "T-EDIT: one `created` entry per top-level command"
    );
    assert_eq!(
        files(tmp.path()),
        ["depth1.json", "depth2.json", "top.json"],
        "T-LIB-2: nested definitions are written"
    );
    assert_eq!(
        read_def(&tmp.path().join("depth2.json")),
        as_def(&fade_def("depth2")),
        "T-LIB-2: nested definition content"
    );
}

#[test]
fn t_lib_2_names_matching_the_pattern_are_saved() {
    let tmp = tempfile::tempdir().unwrap();
    let s = lib_session(tmp.path());
    let p = new_project(&s, json!({}));
    let long = "a".repeat(64);
    let r = edit(
        &s,
        &p.id,
        json!([
            { "op": "define_effect", "def": swap_def("A-b_9") },
            { "op": "define_effect", "def": swap_def(&long) },
            { "op": "define_effect", "def": swap_def("x") }
        ]),
    );
    assert_eq!(
        r["library"],
        json!(["A-b_9", long, "x"]),
        "T-LIB-2: [A-Za-z0-9_-]{{1,64}} names are saved"
    );
    assert_eq!(
        files(tmp.path()),
        [
            "A-b_9.json".to_string(),
            format!("{long}.json"),
            "x.json".to_string()
        ],
        "T-LIB-2: files of valid names"
    );
}

#[test]
fn t_lib_2_names_outside_the_pattern_are_not_saved() {
    let long = "a".repeat(65);
    for name in ["my look", "my.look", long.as_str(), "../evil", "a/b", "é"] {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("lib");
        let s = lib_session(&dir);
        let p = new_project(&s, json!({}));
        // The edit itself may accept or reject such a name; either way nothing reaches the disk.
        let r = s.call(
            "edit",
            json!({ "project_id": p.id,
                    "commands": [{ "op": "define_effect", "def": swap_def(name) }] }),
        );
        if let Ok(o) = r {
            assert_eq!(
                o.json["library"],
                json!([]),
                "T-LIB-2: name {name:?} does not match [A-Za-z0-9_-]{{1,64}}"
            );
        }
        assert_eq!(
            files(&dir),
            [""; 0],
            "T-LIB-2: no file for the name {name:?}"
        );
        assert_eq!(
            files(tmp.path())
                .into_iter()
                .filter(|f| f != "lib")
                .collect::<Vec<_>>(),
            [""; 0],
            "T-LIB-2: the name {name:?} writes nothing outside the library"
        );
    }
}

#[test]
fn t_lib_2_valid_name_next_to_invalid_one_is_still_saved() {
    let tmp = tempfile::tempdir().unwrap();
    let s = lib_session(tmp.path());
    let p = new_project(&s, json!({}));
    let r = s.call(
        "edit",
        json!({ "project_id": p.id, "commands": [
            { "op": "define_effect", "def": swap_def("my look") },
            { "op": "define_effect", "def": swap_def("good") }
        ] }),
    );
    // shortcut: if the edit rejects a name with a space the rule is not reachable here.
    if let Ok(o) = r {
        assert_eq!(
            o.json["library"],
            json!(["good"]),
            "T-LIB-2: only matching names are saved"
        );
        assert_eq!(files(tmp.path()), ["good.json"], "T-LIB-2: only that file");
    }
}

#[test]
fn t_lib_2_builtin_name_is_not_saved() {
    let tmp = tempfile::tempdir().unwrap();
    let s = lib_session(tmp.path());
    let p = new_project(&s, json!({}));
    // SPEC E-3: a project definition may shadow a built-in.
    let r = edit(
        &s,
        &p.id,
        json!([
            { "op": "define_effect", "def": swap_def("exposure") },
            { "op": "define_effect", "def": fade_def("cross_dissolve") },
            { "op": "define_effect", "def": swap_def("mine") }
        ]),
    );
    assert_eq!(
        r["library"],
        json!(["mine"]),
        "T-LIB-2: a built-in's name is not saved"
    );
    assert_eq!(
        files(tmp.path()),
        ["mine.json"],
        "T-LIB-2: no file for built-in names"
    );
}

#[test]
fn t_lib_2_save_replaces_an_older_file() {
    let tmp = tempfile::tempdir().unwrap();
    let s = lib_session(tmp.path());
    let p1 = new_project(&s, json!({}));
    define(&s, &p1.id, swap_def("swap"));
    assert_eq!(
        read_def(&tmp.path().join("swap.json")).params[0].max,
        2.0,
        "T-LIB-2: first save"
    );

    let p2 = new_project(&s, json!({}));
    let r = define(&s, &p2.id, swap_def_max("swap", 5.0));
    assert_eq!(r["library"], json!(["swap"]), "T-LIB-2: saved again");
    let def = read_def(&tmp.path().join("swap.json"));
    assert_eq!(def, as_def(&swap_def_max("swap", 5.0)), "T-LIB-2: replaced");
    assert_eq!(def.params[0].max, 5.0, "T-LIB-2: the newer definition");
    assert_eq!(files(tmp.path()), ["swap.json"], "T-LIB-2: one file");

    // A file that is not an EffectDef is replaced as well.
    std::fs::write(tmp.path().join("junk.json"), "not json").unwrap();
    define(&s, &p2.id, swap_def("junk"));
    assert_eq!(
        read_def(&tmp.path().join("junk.json")),
        as_def(&swap_def("junk")),
        "T-LIB-2: an older file is replaced whatever it held"
    );
}

#[test]
fn t_lib_2_failed_edit_saves_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let s = lib_session(tmp.path());
    let p = new_project(&s, json!({}));
    edit_err(
        &s,
        &p.id,
        json!([{ "op": "define_effect", "def": swap_def("ghost") },
               { "op": "remove_clips", "clip_ids": [424_242] }]),
    );
    assert_eq!(
        files(tmp.path()),
        [""; 0],
        "T-LIB-2: a failed edit saves nothing"
    );
    // A definition that fails validate_effect (T-EDIT) fails the edit too.
    let mut bad = swap_def("broken");
    bad["wgsl"] = json!("fn effect(uv: vec2f) -> vec4f { return nonsense; }");
    edit_err(
        &s,
        &p.id,
        json!([{ "op": "define_effect", "def": swap_def("ok_one") },
               { "op": "define_effect", "def": bad }]),
    );
    assert_eq!(
        files(tmp.path()),
        [""; 0],
        "T-LIB-2: a failed edit saves none of its definitions"
    );
    assert!(
        effects(&s, json!({ "project_id": p.id }))
            .iter()
            .all(|e| e["origin"] != "library"),
        "T-LIB-2: nothing in the library after failed edits"
    );
}

#[test]
fn t_lib_2_write_failure_does_not_fail_the_edit() {
    let tmp = tempfile::tempdir().unwrap();
    // The library "directory" is a regular file, so no file can be written into it.
    let dir = tmp.path().join("blocked");
    std::fs::write(&dir, b"a file, not a directory").unwrap();
    let s = lib_session(&dir);
    let p = new_project(&s, json!({}));
    let r = define(&s, &p.id, swap_def("swap"));
    assert_eq!(
        r["library"],
        json!([]),
        "T-LIB-2: a name that could not be written is left out"
    );
    assert_eq!(
        project_effect_names(&s, &p.id),
        json!(["swap"]),
        "T-LIB-2: a write failure does not fail the edit"
    );
    assert_eq!(
        std::fs::read(&dir).unwrap(),
        b"a file, not a directory",
        "T-LIB-2: the blocking file is untouched"
    );
}

#[test]
fn t_lib_2_undo_never_changes_the_library() {
    let tmp = tempfile::tempdir().unwrap();
    let s = lib_session(tmp.path());
    let p = new_project(&s, json!({}));
    define(&s, &p.id, swap_def("swap"));
    let before = std::fs::read(tmp.path().join("swap.json")).unwrap();

    let u = call(&s, "undo", json!({ "project_id": p.id })).json;
    assert_eq!(u["done"], 1, "T-UNDO: one step undone");
    assert_eq!(
        project_effect_names(&s, &p.id),
        json!([]),
        "T-UNDO: the project definition is gone"
    );
    assert_eq!(
        files(tmp.path()),
        ["swap.json"],
        "T-LIB-2: undo leaves the file"
    );
    assert_eq!(
        std::fs::read(tmp.path().join("swap.json")).unwrap(),
        before,
        "T-LIB-2: undo does not change the file"
    );
    let l = effects(&s, json!({ "project_id": p.id }));
    let sw = named(&l, "swap");
    assert_eq!(sw.len(), 1, "T-LIB-3: still listed once");
    assert_eq!(
        sw[0]["origin"], "library",
        "T-LIB-3: now only the library has it"
    );

    // Redo does not change it either.
    std::fs::remove_file(tmp.path().join("swap.json")).unwrap();
    call(&s, "redo", json!({ "project_id": p.id }));
    assert_eq!(
        files(tmp.path()),
        [""; 0],
        "T-LIB-2: only `edit` writes to the library"
    );
}

// ---------------------------------------------------------------- T-LIB-3

#[test]
fn t_lib_3_origin_builtin_project_library() {
    let tmp = tempfile::tempdir().unwrap();
    put(tmp.path(), &swap_def("lib_look"));
    put(tmp.path(), &fade_def("lib_fade"));
    let s = lib_session(tmp.path());
    let p = new_project(&s, json!({}));
    define(&s, &p.id, swap_def("proj_look"));

    let l = effects(&s, json!({ "project_id": p.id, "include_source": true }));
    for e in &l {
        let o = e["origin"].as_str().unwrap_or_else(|| {
            panic!(
                "T-LIB-3: every effect has an origin, missing on {}",
                e["name"]
            )
        });
        assert!(
            ["builtin", "project", "library"].contains(&o),
            "T-LIB-3: origin {o:?} of {}",
            e["name"]
        );
        assert_eq!(
            e["builtin"],
            o == "builtin",
            "T-LIB-3: `builtin` agrees with origin for {}",
            e["name"]
        );
    }
    for name in ["exposure", "invert", "cross_dissolve", "wipe"] {
        let e = named(&l, name);
        assert_eq!(e.len(), 1, "T-LIB-3: {name} listed once");
        assert_eq!(e[0]["origin"], "builtin", "T-LIB-3: {name}");
        assert_eq!(e[0]["builtin"], true, "T-LIB-3: {name}");
    }
    let e = named(&l, "proj_look");
    assert_eq!(e.len(), 1, "T-LIB-3: project effect listed once");
    assert_eq!(e[0]["origin"], "project", "T-LIB-3: defined by the project");
    assert_eq!(e[0]["builtin"], false, "T-LIB-3: project effect");

    let e = named(&l, "lib_look");
    assert_eq!(e.len(), 1, "T-LIB-3: library effect listed once");
    assert_eq!(e[0]["origin"], "library", "T-LIB-3: from the directory");
    assert_eq!(e[0]["builtin"], false, "T-LIB-3: library => builtin false");
    assert_eq!(e[0]["kind"], "filter", "T-LIB-3: kind of a library effect");
    assert_eq!(
        e[0]["params"],
        json!([{ "name": "gain", "default": 1.0, "min": 0.0, "max": 2.0 }]),
        "T-LIB-3: params of a library effect"
    );
    assert_eq!(e[0]["wgsl"], SWAP_WGSL, "T-FXLIST: include_source");
    let e = named(&l, "lib_fade");
    assert_eq!(e.len(), 1, "T-LIB-3: library transition listed once");
    assert_eq!(e[0]["origin"], "library", "T-LIB-3: library transition");
    assert_eq!(e[0]["kind"], "transition", "T-LIB-3: kind transition");
    assert_eq!(e[0]["params"], json!([]), "T-LIB-3: no params");

    // Without project_id: built-ins and the library. `proj_look` was saved by the edit above.
    let l = effects(&s, json!({}));
    assert!(
        l.iter().all(|e| e["origin"] != "project"),
        "T-FXLIST: no project effects without project_id"
    );
    for name in ["lib_look", "lib_fade", "proj_look"] {
        let e = named(&l, name);
        assert_eq!(e.len(), 1, "T-LIB-3: {name} listed once without project_id");
        assert_eq!(
            e[0]["origin"], "library",
            "T-LIB-3: {name} without project_id"
        );
        assert!(
            e[0].get("wgsl").is_none_or(Value::is_null),
            "T-FXLIST: no source unless include_source ({name})"
        );
    }
    assert_eq!(
        named(&l, "exposure")[0]["origin"],
        "builtin",
        "T-LIB-3: built-in without project_id"
    );
}

#[test]
fn t_lib_3_library_effects_are_listed_after_the_others() {
    let tmp = tempfile::tempdir().unwrap();
    // Names that sort before every built-in and project effect.
    put(tmp.path(), &swap_def("000_first"));
    put(tmp.path(), &fade_def("AAA_first"));
    let s = lib_session(tmp.path());
    let p = new_project(&s, json!({}));
    define(&s, &p.id, swap_def("zzz_project"));
    std::fs::remove_file(tmp.path().join("zzz_project.json")).unwrap();

    for args in [json!({}), json!({ "project_id": p.id })] {
        let l = effects(&s, args.clone());
        let first_lib = l
            .iter()
            .position(|e| e["origin"] == "library")
            .expect("T-LIB-3: library effects are listed");
        assert_eq!(
            l.len() - first_lib,
            2,
            "T-LIB-3: both library effects, nothing after them ({args})"
        );
        assert!(
            l[first_lib..].iter().all(|e| e["origin"] == "library"),
            "T-LIB-3: library effects come after the others ({args})"
        );
        assert!(
            l[..first_lib].iter().any(|e| e["origin"] == "builtin"),
            "T-LIB-3: built-ins come first ({args})"
        );
    }
}

#[test]
fn t_lib_3_invalid_files_are_skipped() {
    let tmp = tempfile::tempdir().unwrap();
    put(tmp.path(), &swap_def("good"));
    std::fs::write(tmp.path().join("bad.json"), "this is not json").unwrap();
    std::fs::write(tmp.path().join("shape.json"), r#"{"name":"shape"}"#).unwrap();
    std::fs::write(tmp.path().join("array.json"), "[1, 2, 3]").unwrap();
    std::fs::write(tmp.path().join("empty.json"), "").unwrap();
    std::fs::write(
        tmp.path().join("kind.json"),
        json!({ "name": "kind", "kind": "sparkle", "wgsl": SWAP_WGSL, "params": [] }).to_string(),
    )
    .unwrap();
    std::fs::create_dir(tmp.path().join("folder.json")).unwrap();
    let s = lib_session(tmp.path());

    let l = effects(&s, json!({}));
    let lib: Vec<&str> = l
        .iter()
        .filter(|e| e["origin"] == "library")
        .map(|e| e["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        lib,
        ["good"],
        "T-LIB-3: files that are not a valid EffectDef are skipped"
    );
}

#[test]
fn t_lib_3_directory_is_read_on_every_call() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("effects");
    let s = lib_session(&dir);
    assert!(
        named(&effects(&s, json!({})), "late").is_empty(),
        "T-LIB-3: not there yet"
    );

    // Added by hand after the session started.
    put(&dir, &swap_def("late"));
    let l = effects(&s, json!({}));
    let e = named(&l, "late");
    assert_eq!(e.len(), 1, "T-LIB-3: read on every call (file added)");
    assert_eq!(e[0]["origin"], "library", "T-LIB-3: origin of a new file");
    assert_eq!(f(&e[0]["params"][0]["max"]), 2.0, "T-LIB-3: first content");

    // Changed by hand.
    put(&dir, &swap_def_max("late", 7.0));
    let l = effects(&s, json!({}));
    assert_eq!(
        f(&named(&l, "late")[0]["params"][0]["max"]),
        7.0,
        "T-LIB-3: read on every call (file changed)"
    );

    // Removed by hand.
    std::fs::remove_file(dir.join("late.json")).unwrap();
    assert!(
        named(&effects(&s, json!({})), "late").is_empty(),
        "T-LIB-3: read on every call (file removed)"
    );
}

#[test]
fn t_lib_3_project_wins_over_library_duplicate() {
    let tmp = tempfile::tempdir().unwrap();
    let s = lib_session(tmp.path());
    let p = new_project(&s, json!({}));
    define(&s, &p.id, swap_def_max("dup", 3.0));
    // The edit saved its own copy; put a different one there by hand.
    put(tmp.path(), &swap_def_max("dup", 9.0));

    let l = effects(&s, json!({ "project_id": p.id }));
    let e = named(&l, "dup");
    assert_eq!(e.len(), 1, "T-LIB-3: a duplicate name is listed once");
    assert_eq!(e[0]["origin"], "project", "T-LIB-3: listed as \"project\"");
    assert_eq!(e[0]["builtin"], false, "T-LIB-3: not a built-in");
    assert_eq!(
        f(&e[0]["params"][0]["max"]),
        3.0,
        "T-LIB-3: the project's definition is the one listed"
    );

    // Without project_id only the library copy exists.
    let l = effects(&s, json!({}));
    let e = named(&l, "dup");
    assert_eq!(e.len(), 1, "T-LIB-3: listed once without project_id");
    assert_eq!(e[0]["origin"], "library", "T-LIB-3: library copy");
    assert_eq!(f(&e[0]["params"][0]["max"]), 9.0, "T-LIB-3: file content");

    // Another project that does not define it sees the library copy.
    let p2 = new_project(&s, json!({}));
    let l = effects(&s, json!({ "project_id": p2.id }));
    let e = named(&l, "dup");
    assert_eq!(e.len(), 1, "T-LIB-3: listed once for another project");
    assert_eq!(e[0]["origin"], "library", "T-LIB-3: other project");
}

// ---------------------------------------------------------------- T-LIB-4

#[test]
fn t_lib_4_add_effect_defines_the_library_effect_in_the_same_undo_step() {
    let tmp = tempfile::tempdir().unwrap();
    put(tmp.path(), &swap_def("swap"));
    let s = lib_session(tmp.path());
    let p = new_project(&s, json!({}));
    let c = add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.5, 0.0, 1.0],
    );
    assert_eq!(undo_depth(&s, &p.id), 1, "one edit so far");
    assert_eq!(project_effect_names(&s, &p.id), json!([]));

    let r = edit(
        &s,
        &p.id,
        json!([{ "op": "add_effect", "clip_id": c, "effect": "swap" }]),
    );
    assert_eq!(
        r["created"].as_array().map(Vec::len),
        Some(1),
        "T-LIB-4: exactly one `created` entry per command sent: {r}"
    );
    assert_eq!(
        r["created"][0].as_array().map(Vec::len),
        Some(1),
        "T-LIB-4: entry 0 belongs to the add_effect (its instance id)"
    );
    assert_eq!(r["undo_depth"], 2, "T-LIB-4: same undo step");
    assert_eq!(
        project_effect_names(&s, &p.id),
        json!(["swap"]),
        "T-LIB-4: the definition is put into the project"
    );
    let (proj, _) = s.snapshot(&p.id).unwrap();
    assert_eq!(
        proj.effects,
        vec![as_def(&swap_def("swap"))],
        "T-LIB-4: the project definition equals the library file"
    );
    let clip = find_clip(&project(&s, &p.id), c);
    assert_eq!(
        clip["effects"].as_array().map(Vec::len),
        Some(1),
        "T-LIB-4: the effect is on the clip"
    );
    assert_eq!(clip["effects"][0]["effect"], "swap", "T-LIB-4: by name");
    // SPEC E-5: (1, 0.5, 0) -> (b, g, r) = (0, 0.5, 1) -> (0, 128, 255).
    let (img, _) = frame_png(&s, &p.id, json!(0));
    assert_px(&img, 32, 24, [0, 128, 255], 2, "T-LIB-4: it renders");

    let u = call(&s, "undo", json!({ "project_id": p.id })).json;
    assert_eq!(u["done"], 1, "T-UNDO");
    assert_eq!(u["undo_depth"], 1, "T-LIB-4: one undo step for both");
    assert_eq!(
        find_clip(&project(&s, &p.id), c)["effects"],
        json!([]),
        "T-LIB-4: one undo removes the clip effect"
    );
    assert_eq!(
        project_effect_names(&s, &p.id),
        json!([]),
        "T-LIB-4: the same undo removes the definition"
    );
    assert_eq!(
        files(tmp.path()),
        ["swap.json"],
        "T-LIB-2: undo leaves the library"
    );
}

#[test]
fn t_lib_4_created_has_one_entry_per_command_sent() {
    let tmp = tempfile::tempdir().unwrap();
    put(tmp.path(), &swap_def("swap"));
    put(tmp.path(), &fade_def("lib_fade"));
    let s = lib_session(tmp.path());
    let p = new_project(&s, json!({}));
    let n = next_id(&s, &p.id);
    let r = edit(
        &s,
        &p.id,
        json!([
            { "op": "add_clip", "track_id": p.v1(), "source": solid([1.0, 0.0, 0.0, 1.0]),
              "start": 0, "duration": "2s" },
            { "op": "add_clip", "track_id": p.v1(), "source": solid([0.0, 0.0, 1.0, 1.0]),
              "start": "2s", "duration": "2s" },
            { "op": "add_effect", "clip_id": n, "effect": "swap" },
            { "op": "add_transition", "track_id": p.v1(), "left": n, "right": n + 1,
              "effect": "lib_fade", "duration": "0.5s" }
        ]),
    );
    let created = r["created"].as_array().expect("created array");
    assert_eq!(
        created.len(),
        4,
        "T-LIB-4: 4 commands sent -> 4 entries, the inserted define_effect has none: {r}"
    );
    assert_eq!(created[0], json!([n]), "T-LIB-4: entry 0 = first add_clip");
    assert_eq!(
        created[1],
        json!([n + 1]),
        "T-LIB-4: entry 1 = second add_clip"
    );
    assert_eq!(
        created[2].as_array().map(Vec::len),
        Some(1),
        "T-LIB-4: entry 2 = add_effect"
    );
    assert_eq!(
        created[3].as_array().map(Vec::len),
        Some(1),
        "T-LIB-4: entry 3 = add_transition"
    );
    assert_eq!(r["undo_depth"], 1, "T-LIB-4: one undo step for the batch");
    let mut names: Vec<String> = serde_json::from_value(project_effect_names(&s, &p.id)).unwrap();
    names.sort();
    assert_eq!(names, ["lib_fade", "swap"], "T-LIB-4: both definitions");

    call(&s, "undo", json!({ "project_id": p.id }));
    assert_eq!(depths(&s, &p.id), (0, 1), "T-LIB-4: all in one step");
    assert_eq!(
        project_effect_names(&s, &p.id),
        json!([]),
        "T-LIB-4: undo removes the definitions"
    );
    assert_eq!(
        find_track(&project(&s, &p.id), p.v1())["clips"],
        json!([]),
        "T-LIB-4: undo removes the clips"
    );
}

#[test]
fn t_lib_4_add_transition_uses_a_library_transition() {
    let tmp = tempfile::tempdir().unwrap();
    put(tmp.path(), &fade_def("lib_fade"));
    let s = lib_session(tmp.path());
    let p = new_project(&s, json!({}));
    let (a, b) = two_clips(&s, &p);
    assert_eq!(undo_depth(&s, &p.id), 2);

    let r = edit(
        &s,
        &p.id,
        json!([{ "op": "add_transition", "track_id": p.v1(), "left": a, "right": b,
                 "effect": "lib_fade", "duration": "0.5s" }]),
    );
    assert_eq!(
        r["created"].as_array().map(Vec::len),
        Some(1),
        "T-LIB-4: one `created` entry for one command: {r}"
    );
    assert_eq!(r["undo_depth"], 3, "T-LIB-4: same undo step");
    let t = transitions(&s, &p);
    assert_eq!(t.len(), 1, "T-LIB-4: the transition exists");
    assert_eq!(
        t[0]["effect"], "lib_fade",
        "T-LIB-4: with the library effect"
    );
    assert_eq!(
        project_effect_names(&s, &p.id),
        json!(["lib_fade"]),
        "T-LIB-4: defined in the project"
    );
    frame_png(&s, &p.id, json!("2s"));

    call(&s, "undo", json!({ "project_id": p.id }));
    assert_eq!(
        transitions(&s, &p).len(),
        0,
        "T-LIB-4: one undo removes the transition"
    );
    assert_eq!(
        project_effect_names(&s, &p.id),
        json!([]),
        "T-LIB-4: and the definition"
    );
}

#[test]
fn t_lib_4_set_transition_with_effect_uses_a_library_transition() {
    let tmp = tempfile::tempdir().unwrap();
    put(tmp.path(), &fade_def("lib_fade"));
    let s = lib_session(tmp.path());
    let p = new_project(&s, json!({}));
    let (a, b) = two_clips(&s, &p);
    let r = edit(
        &s,
        &p.id,
        json!([{ "op": "add_transition", "track_id": p.v1(), "left": a, "right": b,
                 "effect": "cross_dissolve", "duration": "0.5s" }]),
    );
    let tid = r["created"][0][0].as_u64().expect("transition id");
    assert_eq!(project_effect_names(&s, &p.id), json!([]));

    // Without `effect` nothing is looked up or defined.
    edit(
        &s,
        &p.id,
        json!([{ "op": "set_transition", "transition_id": tid, "duration": "10f" }]),
    );
    assert_eq!(
        project_effect_names(&s, &p.id),
        json!([]),
        "T-LIB-4: set_transition without effect defines nothing"
    );

    let r = edit(
        &s,
        &p.id,
        json!([{ "op": "set_transition", "transition_id": tid, "effect": "lib_fade" }]),
    );
    assert_eq!(
        r["created"].as_array().map(Vec::len),
        Some(1),
        "T-LIB-4: one `created` entry for one command: {r}"
    );
    assert_eq!(r["undo_depth"], 5, "T-LIB-4: same undo step");
    assert_eq!(
        transitions(&s, &p)[0]["effect"],
        "lib_fade",
        "T-LIB-4: set_transition with a library effect works"
    );
    assert_eq!(
        project_effect_names(&s, &p.id),
        json!(["lib_fade"]),
        "T-LIB-4: defined in the project"
    );

    call(&s, "undo", json!({ "project_id": p.id }));
    assert_eq!(
        transitions(&s, &p)[0]["effect"],
        "cross_dissolve",
        "T-LIB-4: one undo restores the transition"
    );
    assert_eq!(
        project_effect_names(&s, &p.id),
        json!([]),
        "T-LIB-4: and removes the definition"
    );
}

#[test]
fn t_lib_4_wrong_kind_is_error_and_changes_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    put(tmp.path(), &swap_def("lib_filter"));
    put(tmp.path(), &fade_def("lib_fade"));
    let s = lib_session(tmp.path());
    let p = new_project(&s, json!({}));
    let (a, b) = two_clips(&s, &p);
    let r = edit(
        &s,
        &p.id,
        json!([{ "op": "add_transition", "track_id": p.v1(), "left": a, "right": b,
                 "effect": "cross_dissolve", "duration": "0.5s" }]),
    );
    let tid = r["created"][0][0].as_u64().unwrap();
    let before = project(&s, &p.id);

    edit_err(
        &s,
        &p.id,
        json!([{ "op": "add_effect", "clip_id": a, "effect": "lib_fade" }]),
    );
    edit_err(
        &s,
        &p.id,
        json!([{ "op": "remove_transition", "transition_id": tid },
               { "op": "add_transition", "track_id": p.v1(), "left": a, "right": b,
                 "effect": "lib_filter", "duration": "0.5s" }]),
    );
    edit_err(
        &s,
        &p.id,
        json!([{ "op": "set_transition", "transition_id": tid, "effect": "lib_filter" }]),
    );
    // Not in the library at all.
    edit_err(
        &s,
        &p.id,
        json!([{ "op": "add_effect", "clip_id": a, "effect": "nowhere" }]),
    );
    assert_eq!(
        project(&s, &p.id),
        before,
        "T-LIB-4: a library effect of the wrong kind is an error, nothing changes"
    );
    assert_eq!(depths(&s, &p.id), (3, 0), "T-LIB-4: no undo step added");
}

#[test]
fn t_lib_4_project_and_batch_definitions_come_before_the_library() {
    let tmp = tempfile::tempdir().unwrap();
    let s = lib_session(tmp.path());
    let p = new_project(&s, json!({}));
    let c = add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    define(&s, &p.id, swap_def_max("dup", 3.0));
    put(tmp.path(), &swap_def_max("dup", 9.0));

    // Defined in the project: the library copy is not put in front.
    edit(
        &s,
        &p.id,
        json!([{ "op": "add_effect", "clip_id": c, "effect": "dup" }]),
    );
    let (proj, _) = s.snapshot(&p.id).unwrap();
    assert_eq!(
        proj.effects,
        vec![as_def(&swap_def_max("dup", 3.0))],
        "T-LIB-4: an effect the project defines is not taken from the library"
    );

    // Defined earlier in the batch: that definition is the one used.
    put(tmp.path(), &swap_def_max("early", 9.0));
    edit(
        &s,
        &p.id,
        json!([{ "op": "define_effect", "def": swap_def_max("early", 4.0) },
               { "op": "add_effect", "clip_id": c, "effect": "early" }]),
    );
    let (proj, _) = s.snapshot(&p.id).unwrap();
    let early = proj
        .effects
        .iter()
        .find(|d| d.name == "early")
        .expect("early is defined");
    assert_eq!(
        early.params[0].max, 4.0,
        "T-LIB-4: a definition earlier in the batch wins over the library"
    );
}

#[test]
fn t_lib_4_second_session_sharing_the_directory_sees_saved_effects() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("effects");
    let first = lib_session(&dir);
    let p1 = new_project(&first, json!({}));
    edit(
        &first,
        &p1.id,
        json!([{ "op": "define_effect", "def": swap_def("swap") },
               { "op": "define_effect", "def": fade_def("my_fade") }]),
    );

    let second = lib_session(&dir);
    let l = effects(&second, json!({}));
    for (name, kind) in [("swap", "filter"), ("my_fade", "transition")] {
        let e = named(&l, name);
        assert_eq!(e.len(), 1, "T-LIB-3: {name} offered to a later session");
        assert_eq!(e[0]["origin"], "library", "T-LIB-3: {name}");
        assert_eq!(e[0]["kind"], kind, "T-LIB-3: {name}");
    }
    let p2 = new_project(&second, json!({}));
    let (a, b) = two_clips(&second, &p2);
    let r = edit(
        &second,
        &p2.id,
        json!([{ "op": "add_effect", "clip_id": a, "effect": "swap" },
               { "op": "add_transition", "track_id": p2.v1(), "left": a, "right": b,
                 "effect": "my_fade", "duration": "0.5s" }]),
    );
    assert_eq!(
        r["created"].as_array().map(Vec::len),
        Some(2),
        "T-LIB-4: two commands sent -> two entries: {r}"
    );
    let (proj, _) = second.snapshot(&p2.id).unwrap();
    let mut defs = proj.effects.clone();
    defs.sort_by(|x, y| x.name.cmp(&y.name));
    assert_eq!(
        defs,
        vec![as_def(&fade_def("my_fade")), as_def(&swap_def("swap"))],
        "T-LIB-4: a later session reuses what the first one saved"
    );
}

// ---------------------------------------------------------------- T-LIB-5

/// `initialize` over the in-process HTTP server; returns the `instructions` text.
/// shortcut: one raw HTTP/1.1 request, body taken from the first `{` to the last `}`; move this
/// to the binary tests (crates/adit/tests/stdio.rs) if the transport framing ever changes.
fn instructions() -> String {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let s = session();
    std::thread::spawn(move || {
        let _ = adit_mcp::serve_http(s, port, "test-token".to_string());
    });
    let t0 = Instant::now();
    let mut stream = loop {
        match std::net::TcpStream::connect(("127.0.0.1", port)) {
            Ok(st) => break st,
            Err(e) if t0.elapsed() > Duration::from_secs(10) => panic!("server not up: {e}"),
            Err(_) => std::thread::sleep(Duration::from_millis(50)),
        }
    };
    let body = json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
        "protocolVersion": "2025-06-18", "capabilities": {},
        "clientInfo": { "name": "adit-tests", "version": "0" } } })
    .to_string();
    write!(
        stream,
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer test-token\r\n\
         Content-Type: application/json\r\nAccept: application/json, text/event-stream\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut raw = Vec::new();
    let mut buf = [0u8; 8192];
    // Until the server closes the connection or goes quiet (an event stream may stay open).
    while let Ok(n) = stream.read(&mut buf) {
        if n == 0 {
            break;
        }
        raw.extend_from_slice(&buf[..n]);
    }
    let text = String::from_utf8_lossy(&raw).into_owned();
    assert!(
        text.starts_with("HTTP/1.1 200") || text.starts_with("HTTP/1.0 200"),
        "initialize over HTTP: {text}"
    );
    let payload = &text[text.find("\r\n\r\n").expect("end of headers")..];
    let (a, b) = (
        payload.find('{').expect("JSON in the reply"),
        payload.rfind('}').expect("JSON in the reply"),
    );
    let v: Value = serde_json::from_str(&payload[a..=b])
        .unwrap_or_else(|e| panic!("reply is one JSON-RPC message ({e}): {payload}"));
    v["result"]["instructions"]
        .as_str()
        .unwrap_or_else(|| panic!("T-1: instructions string in {v}"))
        .to_string()
}

#[test]
fn t_lib_5_instructions_explain_library_and_request_scopes() {
    let ins = instructions();
    assert!(
        ins.contains("library"),
        "T-LIB-5: the text contains the word `library`"
    );
    assert!(
        ins.contains("source_dir"),
        "T-LIB-5: the text contains the word `source_dir`"
    );
    let low = ins.to_lowercase();
    for scope in ["edit", "app", "import"] {
        assert!(
            low.contains(&format!("\"{scope}\"")) || low.contains(&format!("`{scope}`")),
            "T-LIB-5: the three request scopes are explained, {scope} is not named as a value"
        );
    }
    assert!(
        low.contains("scope"),
        "T-LIB-5: the request scopes are explained"
    );
    assert!(
        low.contains("rebuild"),
        "T-LIB-5: scope app: change the source in source_dir, rebuild"
    );
    assert!(
        low.contains("restart"),
        "T-LIB-5: scope app: tell the user to restart"
    );
    assert!(
        low.contains("parameter"),
        "T-LIB-5: a new look is a well-named effect with parameters"
    );
    // T-1 still holds.
    assert!(ins.contains("flick"), "T-1: time formats stay in the text");
    assert!(ins.contains("fn effect"), "T-1: shader contract stays");
}

// ---------------------------------------------------------------- T-REQ-4

fn wait_request(s: &Session) -> Value {
    let r = call(s, "request_wait", json!({ "timeout_s": 5 })).json["request"].clone();
    assert!(r.is_object(), "T-REQ-2: a pending request is handed out");
    r
}

#[test]
fn t_req_4_defaults_are_edit_null_panel_and_no_paths() {
    let s = session();
    let p = new_project(&s, json!({}));
    call(
        &s,
        "request_create",
        json!({ "project_id": p.id, "text": "make it pop" }),
    );
    let l = request_list(&s, json!({}));
    assert_eq!(l.len(), 1);
    let w = wait_request(&s);
    for (who, r) in [("request_list", &l[0]), ("request_wait", &w)] {
        assert_eq!(
            r["scope"], "edit",
            "T-REQ-4: scope defaults to edit ({who})"
        );
        assert!(
            r.get("panel").is_some_and(Value::is_null),
            "T-REQ-4: panel is null when absent ({who}): {r}"
        );
        assert_eq!(r["paths"], json!([]), "T-REQ-4: paths default [] ({who})");
        assert!(
            r.get("source_dir").is_none(),
            "T-REQ-4: source_dir only for scope app ({who}): {r}"
        );
        assert_eq!(r["text"], "make it pop", "T-REQ-1 ({who})");
    }
}

#[test]
fn t_req_4_explicit_scope_edit_with_panel() {
    let s = session();
    let p = new_project(&s, json!({}));
    call(
        &s,
        "request_create",
        json!({ "project_id": p.id, "text": "trim this", "scope": "edit", "panel": "Timeline" }),
    );
    let l = request_list(&s, json!({}));
    let w = wait_request(&s);
    for (who, r) in [("request_list", &l[0]), ("request_wait", &w)] {
        assert_eq!(r["scope"], "edit", "T-REQ-4: scope ({who})");
        assert_eq!(r["panel"], "Timeline", "T-REQ-4: panel ({who})");
        assert_eq!(r["paths"], json!([]), "T-REQ-4: paths ({who})");
        assert!(
            r.get("source_dir").is_none(),
            "T-REQ-4: no source_dir for scope edit ({who})"
        );
    }
}

#[test]
fn t_req_4_scope_import_carries_paths() {
    let s = session();
    let p = new_project(&s, json!({}));
    call(
        &s,
        "request_create",
        json!({ "project_id": p.id, "text": "put these on V1 in order", "scope": "import",
                "paths": ["C:/clips/a.mov", "b.wav"] }),
    );
    let l = request_list(&s, json!({}));
    let w = wait_request(&s);
    for (who, r) in [("request_list", &l[0]), ("request_wait", &w)] {
        assert_eq!(r["scope"], "import", "T-REQ-4: scope ({who})");
        assert_eq!(
            r["paths"],
            json!(["C:/clips/a.mov", "b.wav"]),
            "T-REQ-4: paths as given, in order ({who})"
        );
        assert!(
            r.get("panel").is_some_and(Value::is_null),
            "T-REQ-4: panel null ({who})"
        );
        assert!(
            r.get("source_dir").is_none(),
            "T-REQ-4: no source_dir for scope import ({who})"
        );
        assert_eq!(r["text"], "put these on V1 in order", "T-REQ-1 ({who})");
    }
    assert!(
        summary(&s, &p.id)["media"].as_array().unwrap().is_empty(),
        "T-REQ-4: request_create imports nothing itself"
    );
}

#[test]
fn t_req_4_scope_app_has_source_dir_and_window_rect() {
    let s = session();
    let p = new_project(&s, json!({}));
    call(
        &s,
        "request_create",
        json!({ "project_id": p.id, "text": "make this button bigger", "scope": "app",
                "panel": "Effects", "rect": [0.5, 0.25, 0.125, 0.0625] }),
    );
    let l = request_list(&s, json!({}));
    let w = wait_request(&s);
    for (who, r) in [("request_list", &l[0]), ("request_wait", &w)] {
        assert_eq!(r["scope"], "app", "T-REQ-4: scope ({who})");
        assert_eq!(r["panel"], "Effects", "T-REQ-4: panel ({who})");
        assert_eq!(r["paths"], json!([]), "T-REQ-4: paths ({who})");
        assert_eq!(
            r["rect"],
            json!([0.5, 0.25, 0.125, 0.0625]),
            "T-REQ-4: rect passed through ({who})"
        );
        let sd = r
            .get("source_dir")
            .unwrap_or_else(|| panic!("T-REQ-4: source_dir key for scope app ({who}): {r}"));
        assert!(
            sd.is_null() || sd.is_string(),
            "T-REQ-4: source_dir is a string or null ({who}): {sd}"
        );
        if let Some(dir) = sd.as_str() {
            assert!(
                Path::new(dir).is_dir(),
                "T-REQ-4: source_dir is given only if it still exists ({who}): {dir}"
            );
        }
    }
    assert_eq!(
        l[0].get("source_dir"),
        w.get("source_dir"),
        "T-REQ-4: request_list and request_wait agree on source_dir"
    );
}

#[test]
fn t_req_4_unknown_scope_is_error_and_files_nothing() {
    let s = session();
    let p = new_project(&s, json!({}));
    for scope in [
        json!("delete"),
        json!("App"),
        json!("EDIT"),
        json!(""),
        json!("edit "),
        json!(42),
    ] {
        call_err(
            &s,
            "request_create",
            json!({ "project_id": p.id, "text": "x", "scope": scope }),
        );
    }
    assert_eq!(
        request_list(&s, json!({})).len(),
        0,
        "T-REQ-4: any other scope value -> error, no request is filed"
    );
}

#[test]
fn t_req_4_every_request_in_the_list_has_the_fields() {
    let s = session();
    let p = new_project(&s, json!({}));
    for args in [
        json!({ "text": "one" }),
        json!({ "text": "two", "scope": "app" }),
        json!({ "text": "three", "scope": "import", "paths": ["x.mp4"], "panel": "Project" }),
    ] {
        let mut a = args;
        a["project_id"] = json!(p.id);
        call(&s, "request_create", a);
    }
    // Claim the first and complete it: the fields stay on every status.
    let w = wait_request(&s);
    call(
        &s,
        "request_complete",
        json!({ "request_id": w["request_id"], "message": "ok" }),
    );
    let l = request_list(&s, json!({}));
    assert_eq!(l.len(), 3, "T-REQ-3: all requests");
    let by_text = |t: &str| {
        l.iter()
            .find(|r| r["text"] == t)
            .unwrap_or_else(|| panic!("request {t} listed"))
    };
    let (one, two, three) = (by_text("one"), by_text("two"), by_text("three"));
    assert_eq!(one["status"], "done", "T-REQ-3");
    assert_eq!(
        [&one["scope"], &two["scope"], &three["scope"]],
        [&json!("edit"), &json!("app"), &json!("import")],
        "T-REQ-4: scope for every request"
    );
    assert_eq!(
        [&one["paths"], &two["paths"], &three["paths"]],
        [&json!([]), &json!([]), &json!(["x.mp4"])],
        "T-REQ-4: paths for every request"
    );
    assert!(
        one.get("panel").is_some_and(Value::is_null)
            && two.get("panel").is_some_and(Value::is_null),
        "T-REQ-4: panel null when absent"
    );
    assert_eq!(three["panel"], "Project", "T-REQ-4: panel");
    assert!(
        one.get("source_dir").is_none() && three.get("source_dir").is_none(),
        "T-REQ-4: source_dir only for scope app"
    );
    assert!(
        two.get("source_dir").is_some(),
        "T-REQ-4: source_dir for scope app"
    );
}

#[test]
fn t_req_4_request_create_schema_lists_scope_panel_paths() {
    let tools = Session::tools();
    let t = tools
        .iter()
        .find(|t| t.name == "request_create")
        .expect("request_create is listed");
    for a in ["scope", "panel", "paths"] {
        assert!(
            t.input_schema["properties"].get(a).is_some(),
            "T-REQ-4 / T-1: schema property {a} missing in {}",
            t.input_schema
        );
    }
}
