//! T-EDIT, T-UNDO, T-IMPORT, T-FXLIST, T-FXCHECK and the in-process side of T-2.
mod common;
use common::*;
use serde_json::{Value, json};

// ---------------------------------------------------------------- T-EDIT

#[test]
fn t_edit_created_shape_and_undo_depth() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(&s, json!({}));
    let m = import_pattern(&s, &p.id, dir.path(), "m.mov"); // undo depth 1
    let n = next_id(&s, &p.id);
    let r = edit(
        &s,
        &p.id,
        json!([
            { "op": "add_clip", "track_id": p.tracks[0], "source": solid([1.0, 0.0, 0.0, 1.0]), "start": 0, "duration": "1s" },
            { "op": "add_clip", "track_id": p.tracks[0], "linked_track_id": p.tracks[2], "source": media_src(m),
              "start": "1s", "duration": "1s" },
            { "op": "add_marker", "sequence_id": p.seq, "time": 0 },
            { "op": "set_track", "track_id": p.tracks[1], "name": "Top" },
            { "op": "add_sequence", "name": "S2", "width": 8, "height": 8, "fps": { "num": 25, "den": 1 },
              "sample_rate": 48000, "video_tracks": 1, "audio_tracks": 1 }
        ]),
    );
    // SPEC G4/C-ADD-2/C-ADD-3: ids from next_id n in order; linked add takes video n+1, audio n+2,
    // link n+3; marker n+4; sequence n+5 with tracks n+6, n+7.
    assert_eq!(
        r["created"],
        json!([[n], [n + 1, n + 2], [n + 4], [], [n + 5, n + 6, n + 7]]),
        "T-EDIT created: one list per command"
    );
    assert_eq!(r["undo_depth"], 2, "import + one batch = 2 undo steps");
    let pj = project(&s, &p.id);
    assert_eq!(pj["next_id"].as_u64(), Some(n + 8));
    assert_eq!(find_clip(&pj, n + 1)["link"].as_u64(), Some(n + 3));
    assert_eq!(find_clip(&pj, n + 2)["link"].as_u64(), Some(n + 3));
    // One batch = one undo step.
    let u = call(&s, "undo", json!({ "project_id": p.id })).json;
    assert_eq!(u["done"], 1);
    let pj = project(&s, &p.id);
    assert_eq!(pj["next_id"].as_u64(), Some(n));
    assert_eq!(pj["sequences"].as_array().unwrap().len(), 1);
    // Empty batch: Ok, nothing changes, no undo step (SPEC C-BATCH-1, G2).
    let before = full(&s, &p.id);
    let r = edit(&s, &p.id, json!([]));
    assert_eq!(r["created"], json!([]));
    assert_eq!(full(&s, &p.id), before);
}

#[test]
fn t_edit_batch_is_atomic_when_third_command_fails() {
    let s = session();
    let p = new_project(&s, json!({}));
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    let before = full(&s, &p.id);
    let e = edit_err(
        &s,
        &p.id,
        json!([
            { "op": "add_clip", "track_id": p.v1(), "source": solid([0.0, 1.0, 0.0, 1.0]), "start": "2s", "duration": "1s" },
            { "op": "add_marker", "sequence_id": p.seq, "time": "1s", "name": "m" },
            { "op": "remove_clips", "clip_ids": [987654] }
        ]),
    );
    assert!(
        e.contains("987654"),
        "T-2: the error names the cause (NotFound 987654), got: {e}"
    );
    assert_eq!(
        full(&s, &p.id),
        before,
        "T-EDIT/G1: project and undo/redo depths unchanged"
    );

    // Third command fails with Overlap instead.
    let e = edit_err(
        &s,
        &p.id,
        json!([
            { "op": "add_marker", "sequence_id": p.seq, "time": 0 },
            { "op": "set_track", "track_id": p.v1(), "name": "renamed" },
            { "op": "add_clip", "track_id": p.v1(), "source": solid([0.0, 1.0, 0.0, 1.0]), "start": "0.5s", "duration": "1s" }
        ]),
    );
    assert!(e.to_lowercase().contains("overlap"), "{e}");
    assert_eq!(full(&s, &p.id), before);
}

#[test]
fn t_edit_unknown_or_wrong_kind_effect_rejected_before_apply() {
    let s = session();
    let p = new_project(&s, json!({}));
    let a = add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    let b = add_solid(
        &s,
        &p.id,
        p.v1(),
        json!("1s"),
        json!("1s"),
        [0.0, 0.0, 1.0, 1.0],
    );
    let before = full(&s, &p.id);
    let cases = [
        json!([{ "op": "add_marker", "sequence_id": p.seq, "time": 0 },
               { "op": "add_effect", "clip_id": a, "effect": "no_such_effect" }]),
        // a transition used as a filter
        json!([{ "op": "add_marker", "sequence_id": p.seq, "time": 0 },
               { "op": "add_effect", "clip_id": a, "effect": "cross_dissolve" }]),
        // a filter used as a transition
        json!([{ "op": "add_marker", "sequence_id": p.seq, "time": 0 },
               { "op": "add_transition", "track_id": p.v1(), "left": a, "right": b, "effect": "exposure", "duration": "0.5s" }]),
        json!([{ "op": "add_transition", "track_id": p.v1(), "left": a, "right": b, "effect": "no_such_transition", "duration": "0.5s" }]),
    ];
    for c in cases {
        edit_err(&s, &p.id, c.clone());
        assert_eq!(full(&s, &p.id), before, "nothing applied for {c}");
    }
    // The error names the unknown effect.
    let e = edit_err(
        &s,
        &p.id,
        json!([{ "op": "add_effect", "clip_id": a, "effect": "no_such_effect" }]),
    );
    assert!(e.contains("no_such_effect"), "{e}");
    // The right kinds work.
    edit(
        &s,
        &p.id,
        json!([{ "op": "add_effect", "clip_id": a, "effect": "exposure", "params": { "stops": 1.0 } },
               { "op": "add_transition", "track_id": p.v1(), "left": a, "right": b, "effect": "cross_dissolve", "duration": "0.5s" }]),
    );
}

#[test]
fn t_edit_define_effect_validated() {
    let s = session();
    let p = new_project(&s, json!({}));
    let before = full(&s, &p.id);
    let bad_defs = [
        json!({ "name": "broken", "kind": "filter", "wgsl": "fn effect(uv: vec2f) -> vec4f { return nonsense(; }", "params": [] }),
        json!({ "name": "nofn", "kind": "filter", "wgsl": "fn helper(x: f32) -> f32 { return x; }", "params": [] }),
        // filter code declared as a transition
        json!({ "name": "wrongkind", "kind": "transition", "wgsl": SWAP_WGSL,
                "params": [{ "name": "gain", "default": 1.0, "min": 0.0, "max": 2.0 }] }),
        // parameter used but not declared
        json!({ "name": "undeclared", "kind": "filter", "wgsl": SWAP_WGSL, "params": [] }),
        // parameter name that is not a WGSL identifier (E-2)
        json!({ "name": "badparam", "kind": "filter", "wgsl": "fn effect(uv: vec2f) -> vec4f { return src(uv); }",
                "params": [{ "name": "not an ident", "default": 0.0, "min": 0.0, "max": 1.0 }] }),
    ];
    for d in bad_defs {
        edit_err(
            &s,
            &p.id,
            json!([{ "op": "define_effect", "def": d.clone() }]),
        );
        assert_eq!(
            full(&s, &p.id),
            before,
            "invalid def {d} must change nothing"
        );
    }
    assert_eq!(summary(&s, &p.id)["effects"], json!([]));
}

#[test]
fn t_edit_define_then_use_in_same_batch_renders_e5() {
    let s = session();
    let p = new_project(&s, json!({}));
    let n = next_id(&s, &p.id);
    let r = edit(
        &s,
        &p.id,
        json!([
            { "op": "define_effect", "def": swap_def("swap") },
            { "op": "add_clip", "track_id": p.v1(), "source": solid([1.0, 0.5, 0.0, 1.0]), "start": 0, "duration": "1s" },
            { "op": "add_effect", "clip_id": n, "effect": "swap" }
        ]),
    );
    assert_eq!(r["created"][0], json!([]));
    assert_eq!(r["created"][1], json!([n]));
    let fx = r["created"][2][0].as_u64().unwrap();
    assert_eq!(fx, n + 1);
    // SPEC E-5: (1, 0.5, 0) -> (b, g, r) * gain 1 = (0, 0.5, 1) -> (0, 128, 255).
    let (img, _) = frame_png(&s, &p.id, json!(0));
    assert_px(&img, 32, 24, [0, 128, 255], 2, "E-5 gain 1");
    edit(
        &s,
        &p.id,
        json!([{ "op": "set_param", "target": { "clip_id": n, "effect_id": fx, "name": "gain" }, "value": 0.5 }]),
    );
    // gain 0.5 -> (0, 0.25, 0.5) -> (0, 64, 128).
    let (img, _) = frame_png(&s, &p.id, json!(0));
    assert_px(&img, 32, 24, [0, 64, 128], 2, "E-5 gain 0.5");

    // A project-defined effect is usable in a later edit too.
    let c = add_solid(
        &s,
        &p.id,
        p.v1(),
        json!("1s"),
        json!("1s"),
        [1.0, 0.5, 0.0, 1.0],
    );
    edit(
        &s,
        &p.id,
        json!([{ "op": "add_effect", "clip_id": c, "effect": "swap" }]),
    );
}

#[test]
fn t_edit_define_in_failed_batch_leaves_no_effect() {
    let s = session();
    let p = new_project(&s, json!({}));
    let before = full(&s, &p.id);
    edit_err(
        &s,
        &p.id,
        json!([{ "op": "define_effect", "def": swap_def("ghost") }, { "op": "remove_clips", "clip_ids": [424242] }]),
    );
    assert_eq!(full(&s, &p.id), before);
    let l = call(&s, "effect_list", json!({ "project_id": p.id })).json;
    assert!(
        !l["effects"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["name"] == "ghost")
    );
    // And a name defined in a *failed* earlier call is not usable later.
    let c = add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    edit_err(
        &s,
        &p.id,
        json!([{ "op": "add_effect", "clip_id": c, "effect": "ghost" }]),
    );
}

#[test]
fn t_edit_description_lists_every_op() {
    let tools = adit_mcp::Session::tools();
    let e = tools.iter().find(|t| t.name == "edit").expect("edit tool");
    for op in [
        "add_media",
        "remove_media",
        "set_media",
        "add_sequence",
        "remove_sequence",
        "set_sequence",
        "add_track",
        "remove_track",
        "move_track",
        "set_track",
        "add_marker",
        "remove_marker",
        "define_effect",
        "remove_effect_def",
        "add_clip",
        "remove_clips",
        "ripple_delete_clips",
        "lift_range",
        "extract_range",
        "close_gap",
        "split_at",
        "split_clip",
        "move_clip",
        "trim_clip",
        "slip_clip",
        "slide_clip",
        "set_clip_speed",
        "set_clip",
        "set_text",
        "link_clips",
        "unlink_clips",
        "group_clips",
        "ungroup_clips",
        "set_param",
        "set_keyframe",
        "remove_keyframe",
        "clear_keyframes",
        "add_effect",
        "remove_effect",
        "move_effect",
        "set_effect_enabled",
        "add_transition",
        "remove_transition",
        "set_transition",
        "batch",
    ] {
        assert!(
            e.description.contains(op),
            "T-EDIT: edit description must list op {op}"
        );
    }
}

// ---------------------------------------------------------------- T-UNDO

#[test]
fn t_undo_redo_steps() {
    let s = session();
    let p = new_project(&s, json!({}));
    let s0 = project(&s, &p.id);
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    let s1 = project(&s, &p.id);
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!("1s"),
        json!("1s"),
        [0.0, 1.0, 0.0, 1.0],
    );
    let s2 = project(&s, &p.id);
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!("2s"),
        json!("1s"),
        [0.0, 0.0, 1.0, 1.0],
    );
    let s3 = project(&s, &p.id);

    let u = call(&s, "undo", json!({ "project_id": p.id, "steps": 2 })).json;
    assert_eq!(u, json!({ "done": 2, "undo_depth": 1, "redo_depth": 2 }));
    assert_eq!(project(&s, &p.id), s1);
    // Default steps = 1.
    let u = call(&s, "redo", json!({ "project_id": p.id })).json;
    assert_eq!(u, json!({ "done": 1, "undo_depth": 2, "redo_depth": 1 }));
    assert_eq!(project(&s, &p.id), s2);
    // More steps than available: done = what was performed.
    let u = call(&s, "redo", json!({ "project_id": p.id, "steps": 5 })).json;
    assert_eq!(u, json!({ "done": 1, "undo_depth": 3, "redo_depth": 0 }));
    assert_eq!(project(&s, &p.id), s3);
    let u = call(&s, "undo", json!({ "project_id": p.id, "steps": 10 })).json;
    assert_eq!(u, json!({ "done": 3, "undo_depth": 0, "redo_depth": 3 }));
    assert_eq!(project(&s, &p.id), s0);
    let u = call(&s, "undo", json!({ "project_id": p.id })).json;
    assert_eq!(u, json!({ "done": 0, "undo_depth": 0, "redo_depth": 3 }));
    let u = call(&s, "redo", json!({ "project_id": p.id })).json;
    assert_eq!(u["done"], 1);
    // A new edit clears redo (SPEC G2).
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!("5s"),
        json!("1s"),
        [1.0, 1.0, 1.0, 1.0],
    );
    assert_eq!(depths(&s, &p.id), (2, 0));
    let u = call(&s, "redo", json!({ "project_id": p.id })).json;
    assert_eq!(u, json!({ "done": 0, "undo_depth": 2, "redo_depth": 0 }));
}

// ---------------------------------------------------------------- T-IMPORT

#[test]
fn t_import_one_undo_step_and_fields() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(&s, json!({}));
    let a = dir.path().join("a.mov");
    let b = dir.path().join("b.mov");
    std::fs::write(&a, b"x").unwrap();
    std::fs::write(&b, b"x").unwrap();
    let r = call(
        &s,
        "media_import",
        json!({ "project_id": p.id, "paths": [a.to_str().unwrap(), b.to_str().unwrap()] }),
    )
    .json;
    let media = r["media"].as_array().unwrap();
    assert_eq!(media.len(), 2);
    assert_ne!(media[0]["id"], media[1]["id"]);
    for m in media {
        assert!(m["id"].is_u64());
        assert!(m["name"].is_string());
        approx(f(&m["duration_s"]), 10.0, 1e-9, "Pattern duration");
        assert_eq!(m["video"]["width"], 64);
        assert_eq!(m["video"]["height"], 64);
        assert_eq!(m["audio"]["sample_rate"], 48000);
        assert_eq!(m["audio"]["channels"], 2);
    }
    assert_eq!(
        depths(&s, &p.id),
        (1, 0),
        "T-IMPORT: one undo step for the call"
    );
    let pj = project(&s, &p.id);
    assert_eq!(pj["media"].as_array().unwrap().len(), 2);
    assert!(same_path(pj["media"][0]["path"].as_str().unwrap(), &a));
    assert!(same_path(pj["media"][1]["path"].as_str().unwrap(), &b));
    assert_eq!(
        pj["media"][0]["duration"], 7_056_000_000i64,
        "10 s in flicks"
    );
    call(&s, "undo", json!({ "project_id": p.id }));
    assert_eq!(project(&s, &p.id)["media"], json!([]));
}

#[test]
fn t_import_relative_path_stored_absolute() {
    let s = session();
    let p = new_project(&s, json!({}));
    // Integration tests run with the crate directory as working directory.
    let rel = format!("t_import_rel_{}.mov", std::process::id());
    let abs = std::env::current_dir().unwrap().join(&rel);
    std::fs::write(&abs, b"x").unwrap();
    struct Rm(std::path::PathBuf);
    impl Drop for Rm {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _g = Rm(abs.clone());
    call(
        &s,
        "media_import",
        json!({ "project_id": p.id, "paths": [rel] }),
    );
    let stored = project(&s, &p.id)["media"][0]["path"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        std::path::Path::new(&stored).is_absolute(),
        "T-IMPORT: stored absolute, got {stored}"
    );
    assert!(same_path(&stored, &abs));
}

// ---------------------------------------------------------------- T-2 (in-process)

#[test]
fn t2_unknown_tool_and_bad_params_are_errors() {
    let s = session();
    let p = new_project(&s, json!({}));
    assert!(s.call("no_such_tool", json!({})).is_err());
    let before = full(&s, &p.id);
    call_err(
        &s,
        "edit",
        json!({ "project_id": p.id, "commands": "not a list" }),
    );
    call_err(
        &s,
        "edit",
        json!({ "project_id": p.id, "commands": [{ "op": "no_such_op" }] }),
    );
    call_err(
        &s,
        "edit",
        json!({ "project_id": p.id, "commands": [{ "op": "add_clip" }] }),
    );
    call_err(&s, "edit", json!({ "project_id": p.id }));
    call_err(&s, "project_new", json!({ "name": "x", "width": "wide" }));
    call_err(&s, "undo", json!({ "project_id": p.id, "steps": "many" }));
    assert_eq!(full(&s, &p.id), before, "failed calls change nothing");
    // The session keeps working.
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
}

#[test]
fn t2_core_error_text_is_passed_through() {
    let s = session();
    let p = new_project(&s, json!({}));
    let a = add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    edit(
        &s,
        &p.id,
        json!([{ "op": "set_track", "track_id": p.v1(), "locked": true }]),
    );
    // adit_core::Error::TrackLocked displays "track {id} is locked".
    let e = edit_err(
        &s,
        &p.id,
        json!([{ "op": "remove_clips", "clip_ids": [a] }]),
    );
    assert!(e.contains(&format!("track {} is locked", p.v1())), "{e}");
}

// ---------------------------------------------------------------- T-FXLIST / T-FXCHECK

const BUILTINS: &[(&str, &str)] = &[
    ("exposure", "filter"),
    ("brightness_contrast", "filter"),
    ("saturation", "filter"),
    ("temperature", "filter"),
    ("levels", "filter"),
    ("lift_gamma_gain", "filter"),
    ("invert", "filter"),
    ("tint", "filter"),
    ("hue_rotate", "filter"),
    ("flip", "filter"),
    ("rect", "filter"),
    ("ellipse", "filter"),
    ("linear_gradient", "filter"),
    ("gaussian_blur", "filter"),
    ("sharpen", "filter"),
    ("vignette", "filter"),
    ("pixelate", "filter"),
    ("chroma_key", "filter"),
    ("drop_shadow", "filter"),
    ("stroke", "filter"),
    ("noise", "filter"),
    ("cross_dissolve", "transition"),
    ("dip_to_black", "transition"),
    ("wipe", "transition"),
    ("push", "transition"),
];

fn effects(s: &adit_mcp::Session, args: Value) -> Vec<Value> {
    call(s, "effect_list", args).json["effects"]
        .as_array()
        .expect("effects array")
        .clone()
}

#[test]
fn t_fxlist_builtins_and_project_effects() {
    let s = session();
    let l = effects(&s, json!({}));
    for (name, kind) in BUILTINS {
        let e = l
            .iter()
            .find(|e| e["name"] == *name)
            .unwrap_or_else(|| panic!("builtin {name} listed"));
        assert_eq!(e["kind"], *kind, "{name}");
        assert_eq!(e["builtin"], true, "{name}");
        assert!(
            e.get("wgsl").is_none_or(|w| w.is_null()),
            "{name}: no source unless include_source"
        );
    }
    let mut names: Vec<&str> = l.iter().map(|e| e["name"].as_str().unwrap()).collect();
    let n = names.len();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), n, "names unique");
    let exposure = l.iter().find(|e| e["name"] == "exposure").unwrap();
    let stops = exposure["params"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "stops")
        .expect("stops param");
    assert_eq!(f(&stops["default"]), 0.0);
    assert!(stops["min"].is_number() && stops["max"].is_number());
    let bc = l
        .iter()
        .find(|e| e["name"] == "brightness_contrast")
        .unwrap();
    let contrast = bc["params"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["name"] == "contrast")
        .unwrap();
    assert_eq!(f(&contrast["default"]), 1.0);

    let src = effects(&s, json!({ "include_source": true }));
    let exp = src.iter().find(|e| e["name"] == "exposure").unwrap();
    assert!(exp["wgsl"].as_str().unwrap().contains("fn effect"));
    let cd = src.iter().find(|e| e["name"] == "cross_dissolve").unwrap();
    assert!(cd["wgsl"].as_str().unwrap().contains("fn transition"));

    let p = new_project(&s, json!({}));
    edit(
        &s,
        &p.id,
        json!([{ "op": "define_effect", "def": swap_def("swap") }]),
    );
    assert!(
        !effects(&s, json!({})).iter().any(|e| e["name"] == "swap"),
        "no project_id -> builtins only"
    );
    let l = effects(&s, json!({ "project_id": p.id, "include_source": true }));
    let sw = l
        .iter()
        .find(|e| e["name"] == "swap")
        .expect("project effect listed");
    assert_eq!(sw["builtin"], false);
    assert_eq!(sw["kind"], "filter");
    assert_eq!(sw["wgsl"], SWAP_WGSL);
    assert_eq!(
        sw["params"],
        json!([{ "name": "gain", "default": 1.0, "min": 0.0, "max": 2.0 }])
    );
    call_err(&s, "effect_list", json!({ "project_id": "nope" }));
}

#[test]
fn t_fxcheck() {
    let s = session();
    let gain = json!([{ "name": "gain", "default": 1.0, "min": 0.0, "max": 2.0 }]);
    let ok = call(
        &s,
        "effect_check",
        json!({ "kind": "filter", "wgsl": SWAP_WGSL, "params": gain }),
    )
    .json;
    assert_eq!(ok["ok"], true, "{ok}");
    let tr = call(
        &s,
        "effect_check",
        json!({ "kind": "transition", "params": [],
                "wgsl": "fn transition(uv: vec2f) -> vec4f { return mix(src_a(uv), src_b(uv), ctx.progress); }" }),
    )
    .json;
    assert_eq!(tr["ok"], true, "{tr}");
    for (kind, wgsl, params) in [
        (
            "filter",
            "fn effect(uv: vec2f) -> vec4f { return ; }",
            json!([]),
        ),
        ("filter", "this is not wgsl", json!([])),
        ("filter", "fn helper() -> f32 { return 1.0; }", json!([])),
        ("transition", SWAP_WGSL, gain.clone()),
        ("filter", SWAP_WGSL, json!([])),
        (
            "filter",
            "fn effect(uv: vec2f) -> f32 { return 1.0; }",
            json!([]),
        ),
    ] {
        let r = call(
            &s,
            "effect_check",
            json!({ "kind": kind, "wgsl": wgsl, "params": params }),
        )
        .json;
        assert_eq!(r["ok"], false, "{wgsl} should fail");
        assert!(!r["error"].as_str().expect("error string").is_empty());
    }
}
