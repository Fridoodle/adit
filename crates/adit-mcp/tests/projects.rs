//! T-NEW, T-OPEN, T-SAVE, T-CLOSE, T-LIST, T-GET, T-3, T-6 (docs/MCP_TOOLS.md).
mod common;
use common::*;
use serde_json::{Value, json};

// ---------------------------------------------------------------- T-NEW

#[test]
fn t_new_defaults() {
    let s = session();
    let o = call(&s, "project_new", json!({ "name": "Defaults" })).json;
    let pid = o["project_id"].as_str().unwrap();
    let seq = o["sequence_id"].as_u64().unwrap();
    let tracks: Vec<u64> = o["track_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    // video_tracks=2, audio_tracks=2 -> 4 tracks.
    assert_eq!(tracks.len(), 4);
    let f = full(&s, pid);
    assert_eq!(f["undo_depth"], 0, "T-NEW: creation is not undoable");
    assert_eq!(f["redo_depth"], 0);
    let p = &f["project"];
    assert_eq!(p["name"], "Defaults");
    assert_eq!(p["sequences"].as_array().unwrap().len(), 1);
    let sq = &p["sequences"][0];
    assert_eq!(sq["id"].as_u64(), Some(seq));
    assert_eq!(sq["width"], 1920);
    assert_eq!(sq["height"], 1080);
    assert_eq!(sq["fps"], json!({ "num": 30, "den": 1 }));
    assert_eq!(sq["sample_rate"], 48000);
    let got: Vec<(u64, String, String)> = sq["tracks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            (
                t["id"].as_u64().unwrap(),
                t["kind"].as_str().unwrap().into(),
                t["name"].as_str().unwrap().into(),
            )
        })
        .collect();
    let want: Vec<(u64, String, String)> = vec![
        (tracks[0], "video".into(), "V1".into()),
        (tracks[1], "video".into(), "V2".into()),
        (tracks[2], "audio".into(), "A1".into()),
        (tracks[3], "audio".into(), "A2".into()),
    ];
    assert_eq!(
        got, want,
        "track_ids in order V1, V2, A1, A2 (SPEC C-SEQ-1)"
    );
    // Undo after creation does nothing.
    let u = call(&s, "undo", json!({ "project_id": pid })).json;
    assert_eq!(u["done"], 0);
}

#[test]
fn t_new_fps_strings_and_options() {
    let s = session();
    for (fps, num, den) in [
        ("30", 30, 1),
        ("29.97", 30000, 1001),
        ("23.976", 24000, 1001),
        ("59.94", 60000, 1001),
        ("25/1", 25, 1),
        ("24000/1001", 24000, 1001),
    ] {
        let p = new_project(
            &s,
            json!({ "fps": fps, "sample_rate": 44100, "video_tracks": 1, "audio_tracks": 3 }),
        );
        assert_eq!(p.tracks.len(), 4, "1 video + 3 audio");
        let sq = &project(&s, &p.id)["sequences"][0];
        assert_eq!(sq["fps"], json!({ "num": num, "den": den }), "fps {fps}");
        assert_eq!(sq["sample_rate"], 44100);
        assert_eq!(sq["width"], 64);
        assert_eq!(sq["height"], 48);
        let kinds: Vec<&str> = sq["tracks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["kind"].as_str().unwrap())
            .collect();
        assert_eq!(kinds, ["video", "audio", "audio", "audio"]);
    }
    for bad in ["abc", "0", "30/0", "", "-30"] {
        call_err(&s, "project_new", json!({ "name": "x", "fps": bad }));
    }
    // Missing name.
    call_err(&s, "project_new", json!({}));
    // A failed project_new opens nothing: only the 6 valid projects from the loop are open.
    assert_eq!(list(&s).len(), 6);
}

// ---------------------------------------------------------------- T-3 ids

fn list(s: &adit_mcp::Session) -> Vec<Value> {
    call(s, "project_list", json!({}))
        .json
        .as_array()
        .expect("project_list returns an array")
        .clone()
}

#[test]
fn t3_project_ids_unique_and_never_reused() {
    let s = session();
    let a = new_project(&s, json!({}));
    let b = new_project(&s, json!({}));
    assert_ne!(a.id, b.id);
    assert!(
        a.id.len() <= 8 && b.id.len() <= 8,
        "T-3: short ids, got {} {}",
        a.id,
        b.id
    );
    call(&s, "project_close", json!({ "project_id": b.id }));
    let c = new_project(&s, json!({}));
    assert_ne!(c.id, b.id, "T-3: ids are never reused within a run");
    assert_ne!(c.id, a.id);
}

#[test]
fn t3_unknown_project_id_is_error_everywhere() {
    let s = session();
    let _p = new_project(&s, json!({}));
    for (tool, args) in [
        ("project_get", json!({ "project_id": "nope" })),
        (
            "project_save",
            json!({ "project_id": "nope", "path": "x.adit" }),
        ),
        ("project_close", json!({ "project_id": "nope" })),
        ("edit", json!({ "project_id": "nope", "commands": [] })),
        ("undo", json!({ "project_id": "nope" })),
        ("redo", json!({ "project_id": "nope" })),
        ("media_import", json!({ "project_id": "nope", "paths": [] })),
        ("render_frame", json!({ "project_id": "nope" })),
        ("render_contact_sheet", json!({ "project_id": "nope" })),
        ("audio_levels", json!({ "project_id": "nope" })),
        ("export", json!({ "project_id": "nope", "path": "x.mp4" })),
        (
            "request_create",
            json!({ "project_id": "nope", "text": "x" }),
        ),
    ] {
        call_err(&s, tool, args);
    }
}

#[test]
fn t3_two_projects_are_independent() {
    let s = session();
    let a = new_project(&s, json!({}));
    let b = new_project(&s, json!({}));
    let b_before = full(&s, &b.id);
    add_solid(
        &s,
        &a.id,
        a.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    add_solid(
        &s,
        &a.id,
        a.v1(),
        json!("1s"),
        json!("1s"),
        [0.0, 0.0, 1.0, 1.0],
    );
    assert_eq!(full(&s, &b.id), b_before, "edit in a must not touch b");
    call(&s, "undo", json!({ "project_id": a.id }));
    assert_eq!(full(&s, &b.id), b_before, "undo in a must not touch b");
    assert_eq!(depths(&s, &a.id), (1, 1));
    // Undo in b has nothing to undo even though a has history.
    let u = call(&s, "undo", json!({ "project_id": b.id })).json;
    assert_eq!(u["done"], 0);
    assert_eq!(
        depths(&s, &a.id),
        (1, 1),
        "undo in b must not touch a's history"
    );
    // Closing a leaves b intact.
    call(&s, "project_close", json!({ "project_id": a.id }));
    assert_eq!(full(&s, &b.id), b_before);
}

// ---------------------------------------------------------------- T-CLOSE / T-LIST

#[test]
fn t_close_and_t_list() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let a = new_project(&s, json!({ "name": "Alpha" }));
    let b = new_project(&s, json!({ "name": "Beta" }));
    let l = list(&s);
    assert_eq!(l.len(), 2);
    let ea = l
        .iter()
        .find(|e| e["project_id"] == a.id.as_str())
        .expect("a listed");
    assert_eq!(ea["name"], "Alpha");
    assert!(ea["path"].is_null(), "never saved -> path null");
    assert!(ea.get("dirty").is_some_and(|d| d.is_boolean()));

    // Edit -> dirty; save -> clean with path; edit -> dirty again.
    add_solid(
        &s,
        &a.id,
        a.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    let e = |s: &adit_mcp::Session| {
        list(s)
            .into_iter()
            .find(|e| e["project_id"] == a.id.as_str())
            .unwrap()
    };
    assert_eq!(e(&s)["dirty"], true, "T-LIST: dirty after an edit");
    let path = dir.path().join("alpha.adit");
    call(
        &s,
        "project_save",
        json!({ "project_id": a.id, "path": path.to_str().unwrap() }),
    );
    assert_eq!(e(&s)["dirty"], false, "T-LIST: clean after save");
    assert!(same_path(e(&s)["path"].as_str().unwrap(), &path));
    call(&s, "undo", json!({ "project_id": a.id }));
    assert_eq!(e(&s)["dirty"], true, "undo changes the project -> dirty");

    call(&s, "project_close", json!({ "project_id": a.id }));
    let l = list(&s);
    assert_eq!(l.len(), 1);
    assert_eq!(l[0]["project_id"], b.id.as_str());
    call_err(&s, "project_get", json!({ "project_id": a.id }));
    call_err(&s, "project_close", json!({ "project_id": a.id }));
}

// ---------------------------------------------------------------- T-SAVE / T-OPEN

#[test]
fn t_save_open_round_trip() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(&s, json!({ "name": "Round" }));
    let m = import_pattern(&s, &p.id, dir.path(), "src.mov");
    let r = edit(
        &s,
        &p.id,
        json!([
            { "op": "define_effect", "def": swap_def("swap") },
            { "op": "add_clip", "track_id": p.tracks[0], "linked_track_id": p.tracks[2],
              "source": media_src(m), "start": 0, "duration": "2s", "source_in": "1s" },
            { "op": "add_clip", "track_id": p.tracks[1], "source": text_src("hi"), "start": "1s", "duration": "1s" },
            { "op": "add_marker", "sequence_id": p.seq, "time": "0.5s", "name": "here" }
        ]),
    );
    let v = r["created"][1][0].as_u64().unwrap();
    edit(
        &s,
        &p.id,
        json!([{ "op": "add_effect", "clip_id": v, "effect": "swap", "params": { "gain": 0.5 } }]),
    );
    let before = project(&s, &p.id);

    let path = dir.path().join("round.adit");
    call(
        &s,
        "project_save",
        json!({ "project_id": p.id, "path": path.to_str().unwrap() }),
    );
    let text = std::fs::read_to_string(&path).unwrap();
    // Pretty-printed JSON: multi-line with indentation.
    assert!(
        text.contains('\n') && text.contains("\n  "),
        "T-SAVE: pretty-printed"
    );
    let on_disk: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        on_disk, before,
        "T-SAVE: the file is the adit_core::Project JSON"
    );
    // Atomic write leaves no temp file behind.
    let names: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(
        names.len(),
        2,
        "only src.mov and round.adit remain, got {names:?}"
    );

    let o = call(
        &s,
        "project_open",
        json!({ "path": path.to_str().unwrap() }),
    )
    .json;
    let q = o["project_id"].as_str().unwrap();
    assert_ne!(q, p.id, "open gives a new project id");
    assert_eq!(project(&s, q), before, "T-OPEN: Open(Save(p)) == p");
    let e = list(&s).into_iter().find(|e| e["project_id"] == q).unwrap();
    assert_eq!(e["dirty"], false, "freshly opened -> clean");
    assert!(same_path(e["path"].as_str().unwrap(), &path));
}

#[test]
fn t_save_remembers_path_and_requires_one() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(&s, json!({}));
    let e = call_err(&s, "project_save", json!({ "project_id": p.id }));
    assert!(!e.is_empty(), "never saved, no path -> error");
    assert!(std::fs::read_dir(dir.path()).unwrap().next().is_none());

    let path = dir.path().join("remember.adit");
    call(
        &s,
        "project_save",
        json!({ "project_id": p.id, "path": path.to_str().unwrap() }),
    );
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    call(&s, "project_save", json!({ "project_id": p.id }));
    let on_disk: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(
        on_disk,
        project(&s, &p.id),
        "save without path writes to the remembered path"
    );

    // Unwritable target -> error, the existing file is left intact.
    let bad = dir.path().join("no_such_dir").join("x.adit");
    call_err(
        &s,
        "project_save",
        json!({ "project_id": p.id, "path": bad.to_str().unwrap() }),
    );
    let still: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(still, on_disk);
}

#[test]
fn t_save_is_atomic_under_concurrent_reads() {
    // A reader polling the file must never see a partial write (temp file + rename).
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(&s, json!({}));
    // Many markers make the file large enough that a non-atomic write is observable.
    let markers: Vec<Value> = (0..2000)
        .map(|i| json!({ "op": "add_marker", "sequence_id": p.seq, "time": i * 1000, "name": format!("marker number {i} with a longish name") }))
        .collect();
    edit(&s, &p.id, Value::Array(markers));
    let path = dir.path().join("atomic.adit");
    call(
        &s,
        "project_save",
        json!({ "project_id": p.id, "path": path.to_str().unwrap() }),
    );

    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let reader = {
        let path = path.clone();
        let stop = stop.clone();
        std::thread::spawn(move || {
            let mut good = 0u32;
            while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                // Transient OS sharing errors during the rename are tolerated; torn content is not.
                if let Ok(b) = std::fs::read(&path) {
                    assert!(
                        serde_json::from_slice::<Value>(&b).is_ok(),
                        "torn read of {} bytes",
                        b.len()
                    );
                    good += 1;
                }
            }
            good
        })
    };
    for i in 0..30 {
        if i % 2 == 0 {
            call(&s, "undo", json!({ "project_id": p.id }));
        } else {
            call(&s, "redo", json!({ "project_id": p.id }));
        }
        call(&s, "project_save", json!({ "project_id": p.id }));
    }
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    let good = reader.join().expect("reader never saw a torn file");
    assert!(good > 0);
    let names: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names.len(), 1, "no temp files left: {names:?}");
}

#[test]
fn t_open_errors() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    call_err(
        &s,
        "project_open",
        json!({ "path": dir.path().join("missing.adit").to_str().unwrap() }),
    );

    let bad_json = dir.path().join("bad.adit");
    std::fs::write(&bad_json, "{ this is not json").unwrap();
    call_err(
        &s,
        "project_open",
        json!({ "path": bad_json.to_str().unwrap() }),
    );

    // Valid JSON, valid shape, but fails validate: two overlapping clips on one track (I1).
    let p = new_project(&s, json!({}));
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("2s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    let mut pj = project(&s, &p.id);
    let tr = pj["sequences"][0]["tracks"][0].clone();
    let mut c2 = tr["clips"][0].clone();
    c2["id"] = json!(pj["next_id"].as_u64().unwrap());
    c2["start"] = json!(S); // [1s, 3s) overlaps [0, 2s)
    pj["next_id"] = json!(pj["next_id"].as_u64().unwrap() + 1);
    pj["sequences"][0]["tracks"][0]["clips"]
        .as_array_mut()
        .unwrap()
        .push(c2);
    let invalid = dir.path().join("invalid.adit");
    std::fs::write(&invalid, serde_json::to_string_pretty(&pj).unwrap()).unwrap();
    call_err(
        &s,
        "project_open",
        json!({ "path": invalid.to_str().unwrap() }),
    );

    // next_id not above the ids in use (I2).
    let mut pj2 = project(&s, &p.id);
    pj2["next_id"] = json!(1);
    let invalid2 = dir.path().join("invalid2.adit");
    std::fs::write(&invalid2, serde_json::to_string(&pj2).unwrap()).unwrap();
    call_err(
        &s,
        "project_open",
        json!({ "path": invalid2.to_str().unwrap() }),
    );

    assert_eq!(list(&s).len(), 1, "failed opens add no project");
}

// ---------------------------------------------------------------- T-GET

#[test]
fn t_get_summary_exact_values() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(&s, json!({ "name": "Sum", "fps": "29.97" }));
    let (v1, v2, a1) = (p.tracks[0], p.tracks[1], p.tracks[2]);
    let m = import_pattern(&s, &p.id, dir.path(), "clip.mov"); // undo step 1

    // 29.97 fps: one frame = floor(705_600_000 * 1001 / 30000) = 23_543_520 flicks, exact.
    // c1 = [15f, 30f) = [353_152_800, 706_305_600) -> 0.5005 s .. 1.001 s.
    let c1 = add_solid(
        &s,
        &p.id,
        v1,
        json!("15f"),
        json!("15f"),
        [1.0, 0.0, 0.0, 1.0],
    ); // 2
    // c4 = [30f, 60f) -> ends at 1_412_611_200 = 2.002 s.
    let c4 = add_solid(
        &s,
        &p.id,
        v1,
        json!("30f"),
        json!("30f"),
        [0.0, 0.0, 1.0, 1.0],
    ); // 3
    let long = "0123456789abcdefghijABCDEFGHIJ0123456789XYZ"; // 43 chars
    let r = edit(
        &s,
        &p.id,
        json!([
            { "op": "add_clip", "track_id": v2, "source": text_src(long), "start": 0, "duration": "30f" },
            { "op": "add_clip", "track_id": v2, "source": { "type": "adjustment" }, "start": "30f", "duration": "15f" },
            { "op": "add_clip", "track_id": a1, "source": media_src(m), "start": 0, "duration": "30f", "source_in": "1s" },
            { "op": "add_marker", "sequence_id": p.seq, "time": "30f", "name": "m1" },
            { "op": "define_effect", "def": swap_def("swap") },
            { "op": "add_effect", "clip_id": c1, "effect": "swap" },
            // 10f = 235_435_200 flicks; half = 117_717_600 <= both clip durations (I5).
            { "op": "add_transition", "track_id": v1, "left": c1, "right": c4, "effect": "cross_dissolve", "duration": "10f" },
            { "op": "add_sequence", "name": "Nest", "width": 32, "height": 32, "fps": { "num": 30, "den": 1 },
              "sample_rate": 48000, "video_tracks": 1, "audio_tracks": 0 }
        ]),
    ); // 4
    let text_id = r["created"][0][0].as_u64().unwrap();
    let adj_id = r["created"][1][0].as_u64().unwrap();
    let aud_id = r["created"][2][0].as_u64().unwrap();
    let marker_id = r["created"][3][0].as_u64().unwrap();
    let tr_id = r["created"][6][0].as_u64().unwrap();
    let nest_seq = r["created"][7][0].as_u64().unwrap();
    let r = edit(
        &s,
        &p.id,
        json!([{ "op": "add_clip", "track_id": v2, "source": { "type": "sequence", "sequence_id": nest_seq },
                 "start": "45f", "duration": "15f" }]),
    ); // 5
    let nest_clip = r["created"][0][0].as_u64().unwrap();

    let sm = summary(&s, &p.id);
    assert_eq!(sm["project_id"], p.id.as_str());
    assert_eq!(sm["name"], "Sum");
    assert!(sm["path"].is_null());
    assert_eq!(sm["undo_depth"], 5, "import + 4 edits");
    assert_eq!(sm["redo_depth"], 0);
    assert_eq!(sm["effects"], json!(["swap"]));

    let media = sm["media"].as_array().unwrap();
    assert_eq!(media.len(), 1);
    assert_eq!(media[0]["id"].as_u64(), Some(m));
    assert!(media[0]["name"].is_string());
    assert!(same_path(
        media[0]["path"].as_str().unwrap(),
        &dir.path().join("clip.mov")
    ));
    approx(
        f(&media[0]["duration_s"]),
        10.0,
        1e-9,
        "media duration_s (Pattern 10 s)",
    );
    assert_eq!(media[0]["video"]["width"], 64);
    assert_eq!(media[0]["video"]["height"], 64);
    assert!(!media[0]["video"]["fps"].is_null());
    assert_eq!(
        media[0]["audio"],
        json!({ "sample_rate": 48000, "channels": 2 })
    );

    let seqs = sm["sequences"].as_array().unwrap();
    assert_eq!(seqs.len(), 2);
    let sq = &seqs[0];
    assert_eq!(sq["id"].as_u64(), Some(p.seq));
    assert!(sq["name"].is_string());
    assert_eq!(sq["width"], 64);
    assert_eq!(sq["height"], 48);
    assert_eq!(sq["fps"], "30000/1001");
    // duration = 60 frames = 1_412_611_200 flicks = 2.002 s.
    approx(f(&sq["duration_s"]), 2.002, 1e-9, "sequence duration_s");
    assert_eq!(sq["duration_frames"], 60);
    assert_eq!(seqs[1]["id"].as_u64(), Some(nest_seq));
    assert_eq!(seqs[1]["fps"], "30/1");
    assert_eq!(seqs[1]["duration_frames"], 0);
    approx(f(&seqs[1]["duration_s"]), 0.0, 1e-12, "empty sequence");

    let tracks = sq["tracks"].as_array().unwrap();
    assert_eq!(tracks.len(), 4);
    let t0 = &tracks[0];
    assert_eq!(t0["id"].as_u64(), Some(v1));
    assert_eq!(t0["name"], "V1");
    assert_eq!(t0["kind"], "video");
    for k in ["locked", "muted", "solo", "hidden"] {
        assert_eq!(t0[k], false, "{k}");
    }
    assert_eq!(tracks[2]["kind"], "audio");
    assert_eq!(tracks[2]["name"], "A1");

    let clips = t0["clips"].as_array().unwrap();
    assert_eq!(clips.len(), 2);
    let k1 = &clips[0];
    assert_eq!(k1["id"].as_u64(), Some(c1));
    assert_eq!(k1["name"], "Solid");
    assert_eq!(k1["source"], "solid");
    approx(
        f(&k1["start_s"]),
        0.5005,
        1e-9,
        "c1 start_s = 353_152_800 / s",
    );
    approx(f(&k1["end_s"]), 1.001, 1e-9, "c1 end_s = 706_305_600 / s");
    assert_eq!(k1["start_frame"], 15);
    assert_eq!(k1["end_frame"], 30);
    approx(f(&k1["source_in_s"]), 0.0, 1e-12, "c1 source_in_s");
    approx(f(&k1["speed"]), 1.0, 0.0, "speed");
    assert_eq!(k1["enabled"], true);
    assert!(k1["link"].is_null());
    assert_eq!(k1["effects"], json!(["swap"]));
    let k4 = &clips[1];
    assert_eq!(k4["id"].as_u64(), Some(c4));
    assert_eq!(k4["start_frame"], 30);
    assert_eq!(k4["end_frame"], 60);
    assert_eq!(k4["effects"], json!([]));

    let trs = t0["transitions"].as_array().unwrap();
    assert_eq!(trs.len(), 1);
    assert_eq!(trs[0]["id"].as_u64(), Some(tr_id));
    assert_eq!(trs[0]["left"].as_u64(), Some(c1));
    assert_eq!(trs[0]["right"].as_u64(), Some(c4));
    assert_eq!(trs[0]["effect"], "cross_dissolve");
    // 235_435_200 / 705_600_000 = 0.333666...
    approx(
        f(&trs[0]["duration_s"]),
        235_435_200.0 / 705_600_000.0,
        1e-9,
        "transition duration_s",
    );

    let v2c = tracks[1]["clips"].as_array().unwrap();
    assert_eq!(v2c.len(), 3);
    assert_eq!(v2c[0]["id"].as_u64(), Some(text_id));
    assert_eq!(v2c[0]["name"], "Text");
    assert_eq!(
        v2c[0]["source"], "text:0123456789abcdefghijABCDEFGHIJ0123456789",
        "first 40 chars"
    );
    assert_eq!(v2c[1]["id"].as_u64(), Some(adj_id));
    assert_eq!(v2c[1]["name"], "Adjustment");
    assert_eq!(v2c[1]["source"], "adjustment");
    assert_eq!(v2c[2]["id"].as_u64(), Some(nest_clip));
    assert_eq!(v2c[2]["name"], "Nest");
    assert_eq!(v2c[2]["source"], format!("sequence:{nest_seq}"));

    let a1c = tracks[2]["clips"].as_array().unwrap();
    assert_eq!(a1c.len(), 1);
    assert_eq!(a1c[0]["id"].as_u64(), Some(aud_id));
    assert_eq!(a1c[0]["source"], format!("media:{m}"));
    approx(f(&a1c[0]["source_in_s"]), 1.0, 1e-9, "source_in_s");
    assert_eq!(a1c[0]["end_frame"], 30);

    let markers = sq["markers"].as_array().unwrap();
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0]["id"].as_u64(), Some(marker_id));
    assert_eq!(markers[0]["name"], "m1");
    approx(f(&markers[0]["time_s"]), 1.001, 1e-9, "marker at frame 30");

    // Summary after undo reflects depths.
    call(&s, "undo", json!({ "project_id": p.id, "steps": 2 }));
    let sm = summary(&s, &p.id);
    assert_eq!(sm["undo_depth"], 3);
    assert_eq!(sm["redo_depth"], 2);
    assert_eq!(sm["effects"], json!([]), "define_effect undone");
}

#[test]
fn t_get_full_and_default_detail() {
    let s = session();
    let p = new_project(&s, json!({}));
    let f = full(&s, &p.id);
    assert!(f["project"]["next_id"].is_u64());
    assert!(f["project"]["sequences"].is_array());
    assert!(f["project"]["effects"].is_array());
    assert!(f["project"]["media"].is_array());
    // Default detail is the summary.
    let d = call(&s, "project_get", json!({ "project_id": p.id })).json;
    assert_eq!(d["project_id"], p.id.as_str());
    assert!(d.get("project").is_none(), "summary has no `project` field");
    call_err(
        &s,
        "project_get",
        json!({ "project_id": p.id, "detail": "bogus" }),
    );
}

// ---------------------------------------------------------------- T-6

#[test]
fn t6_sequence_default_is_first_sequence() {
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
    let r = edit(
        &s,
        &p.id,
        json!([{ "op": "add_sequence", "name": "Two", "width": 64, "height": 48, "fps": { "num": 30, "den": 1 },
                 "sample_rate": 48000, "video_tracks": 1, "audio_tracks": 0 }]),
    );
    let seq2 = r["created"][0][0].as_u64().unwrap();
    let v1b = r["created"][0][1].as_u64().unwrap();
    add_solid(&s, &p.id, v1b, json!(0), json!("2s"), [0.0, 0.0, 1.0, 1.0]);

    let (img, _) = frame_png(&s, &p.id, json!(0));
    assert_px(
        &img,
        32,
        24,
        [255, 0, 0],
        2,
        "no sequence_id -> first sequence (red)",
    );
    let o = call(
        &s,
        "render_frame",
        json!({ "project_id": p.id, "sequence_id": seq2, "format": "png" }),
    );
    assert_px(
        &decode(&o.images[0].bytes),
        32,
        24,
        [0, 0, 255],
        2,
        "sequence_id picks the second (blue)",
    );

    call_err(
        &s,
        "render_frame",
        json!({ "project_id": p.id, "sequence_id": 999_999 }),
    );

    // No sequence at all -> error.
    let q = new_project(&s, json!({}));
    edit(
        &s,
        &q.id,
        json!([{ "op": "remove_sequence", "sequence_id": q.seq }]),
    );
    call_err(&s, "render_frame", json!({ "project_id": q.id }));
    call_err(&s, "render_contact_sheet", json!({ "project_id": q.id }));
    call_err(&s, "audio_levels", json!({ "project_id": q.id }));
    call_err(
        &s,
        "export",
        json!({ "project_id": q.id, "path": "never.mp4" }),
    );
}
