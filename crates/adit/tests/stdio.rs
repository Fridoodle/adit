//! `adit mcp` over stdio: T-1, T-2, T-3, T-5, Binary (stdout only protocol, ADIT_MEDIA=pattern).
mod common;
use common::*;
use serde_json::json;
use std::time::Duration;

#[test]
fn t1_initialize_reports_adit_tools_and_instructions() {
    let (mut m, init) = Mcp::start(Media::Pattern, None);
    assert_eq!(init["serverInfo"]["name"], "adit", "T-1: server name");
    assert!(
        init["capabilities"]["tools"].is_object(),
        "T-1: tool capability: {init}"
    );
    assert!(init["protocolVersion"].is_string());
    let ins = init["instructions"]
        .as_str()
        .expect("T-1: instructions string");
    // Time formats (T-4).
    assert!(ins.contains("flick"), "instructions mention flicks");
    // A number with an `s` suffix, one with an `f` suffix, and the h:m:s:frames form.
    assert!(
        num_with_suffix(ins, 's'),
        "instructions show a seconds string like \"1.5s\""
    );
    assert!(
        num_with_suffix(ins, 'f'),
        "instructions show a frames string like \"45f\""
    );
    assert!(has_timecode(ins), "instructions show the h:m:s:frames form");
    // Shader contract (SPEC E-1).
    for needle in [
        "fn effect",
        "fn transition",
        "src(",
        "src_a(",
        "src_b(",
        "ctx.size",
        "ctx.time",
        "ctx.progress",
        "p.",
    ] {
        assert!(
            ins.contains(needle),
            "instructions contain the E-1 item {needle}"
        );
    }

    let r = m.request("tools/list", json!({}));
    let tools = r["result"]["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("tools/list: {r}"));
    for name in TOOL_NAMES {
        let t = tools
            .iter()
            .find(|t| t["name"] == *name)
            .unwrap_or_else(|| panic!("tools/list lacks {name}"));
        assert_eq!(t["inputSchema"]["type"], "object", "{name}: inputSchema");
        assert!(
            t["description"].as_str().is_some_and(|d| !d.is_empty()),
            "{name}: description"
        );
    }
}

/// `"<digits>[.<digits>]<suffix>"` followed by a non-alphanumeric character.
fn num_with_suffix(s: &str, suffix: char) -> bool {
    let b: Vec<char> = s.chars().collect();
    (1..b.len()).any(|i| {
        b[i] == suffix
            && b[i - 1].is_ascii_digit()
            && b.get(i + 1).is_none_or(|c| !c.is_alphanumeric())
    })
}

/// `d:d:d:d` (digits separated by three colons), or the words h:m:s:frames / hh:mm:ss:ff.
fn has_timecode(s: &str) -> bool {
    if s.contains("h:m:s") || s.contains("hh:mm:ss") {
        return true;
    }
    s.split(|c: char| c.is_whitespace() || c == '"' || c == '`' || c == '(' || c == ')' || c == ',')
        .any(|w| {
            let parts: Vec<&str> = w.split(':').collect();
            parts.len() == 4
                && parts
                    .iter()
                    .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
        })
}

#[test]
fn t2_tool_failure_is_iserror_result_and_server_survives() {
    let (mut m, _) = Mcp::start(Media::Pattern, None);
    let e = m.err("project_get", json!({ "project_id": "nope" }));
    assert!(e.contains("nope"), "T-2: message names the cause: {e}");
    let (pid, _, tracks) = new_project(&mut m, json!({ "name": "x", "width": 64, "height": 48 }));
    m.ok("edit", json!({ "project_id": pid, "commands": [
        { "op": "add_clip", "track_id": tracks[0], "source": solid([1.0, 0.0, 0.0, 1.0]), "start": 0, "duration": "1s" }] }));
    let before = m
        .ok(
            "project_get",
            json!({ "project_id": pid, "detail": "full" }),
        )
        .json;
    // Overlapping clip -> core error text passes through, nothing changes.
    let e = m.err("edit", json!({ "project_id": pid, "commands": [
        { "op": "add_clip", "track_id": tracks[0], "source": solid([0.0, 1.0, 0.0, 1.0]), "start": "0.5s", "duration": "1s" }] }));
    assert!(
        e.contains(&format!("clips would overlap on track {}", tracks[0])),
        "{e}"
    );
    assert_eq!(
        m.ok(
            "project_get",
            json!({ "project_id": pid, "detail": "full" })
        )
        .json,
        before,
        "failed call changes nothing"
    );
}

#[test]
fn t2_unknown_tool_and_malformed_input_are_jsonrpc_errors() {
    let (mut m, _) = Mcp::start(Media::Pattern, None);
    let r = m.request(
        "tools/call",
        json!({ "name": "no_such_tool", "arguments": {} }),
    );
    assert!(
        r["error"]["code"].is_i64(),
        "unknown tool -> JSON-RPC error: {r}"
    );
    assert!(r.get("result").is_none());
    let r = m.request(
        "tools/call",
        json!({ "name": "project_new", "arguments": "not an object" }),
    );
    assert!(
        r["error"]["code"].is_i64(),
        "malformed params -> JSON-RPC error: {r}"
    );
    let r = m.request("tools/call", json!({ "arguments": {} }));
    assert!(
        r["error"]["code"].is_i64(),
        "missing tool name -> JSON-RPC error: {r}"
    );
    let r = m.request("no/such/method", json!({}));
    assert_eq!(r["error"]["code"], -32601, "unknown method: {r}");
    // Garbage on stdin: the server keeps running and answers the next request.
    m.send_line("{ this is not json");
    m.send_line("[1, 2");
    let r = m.request("tools/list", json!({}));
    assert!(
        r["result"]["tools"].is_array(),
        "server alive after malformed input: {r}"
    );
    let (pid, _, _) = new_project(&mut m, json!({ "name": "still here" }));
    assert!(!pid.is_empty());
}

#[test]
fn t5_result_blocks_and_compact_json() {
    let (mut m, _) = Mcp::start(Media::Pattern, None);
    let (pid, _, tracks) = new_project(&mut m, json!({ "name": "x", "width": 64, "height": 48 }));
    let r = m.ok("project_get", json!({ "project_id": pid }));
    assert!(!r.text.contains('\n'), "T-5: compact JSON");
    assert!(r.images.is_empty());
    assert_eq!(
        r.raw["content"].as_array().unwrap().len(),
        1,
        "T-5: one text block"
    );
    m.ok("edit", json!({ "project_id": pid, "commands": [
        { "op": "add_clip", "track_id": tracks[0], "source": solid([0.0, 1.0, 0.0, 1.0]), "start": 0, "duration": "1s" }] }));
    let r = m.ok("render_frame", json!({ "project_id": pid, "time": "0.5s" }));
    assert!(!r.text.contains('\n'));
    assert_eq!(r.json["frame"], 15);
    approx(f(&r.json["time_s"]), 0.5, 1e-9, "time_s");
    assert_eq!(r.images.len(), 1, "render_frame: one image block");
    assert_eq!(r.images[0].0, "image/jpeg");
    assert_eq!(&r.images[0].1[..3], &[0xFF, 0xD8, 0xFF]);
    let img = image::load_from_memory(&r.images[0].1).unwrap().to_rgba8();
    assert_rgb(px(&img, 32, 24), [0, 255, 0], 12, "green over stdio");
    let r = m.ok(
        "render_frame",
        json!({ "project_id": pid, "format": "png", "max_size": 32 }),
    );
    assert_eq!(r.images[0].0, "image/png");
    let img = image::load_from_memory(&r.images[0].1).unwrap().to_rgba8();
    assert_eq!((img.width(), img.height()), (32, 24));
}

#[test]
fn binary_stdout_only_protocol_and_exit_on_stdin_close() {
    // The client panics on any stdout line that is not a JSON-RPC 2.0 message.
    let (mut m, _) = Mcp::start(Media::Pattern, None);
    let dir = tempfile::tempdir().unwrap();
    let (pid, seq, tracks) = new_project(
        &mut m,
        json!({ "name": "noisy", "width": 64, "height": 48, "audio_tracks": 0 }),
    );
    m.ok("edit", json!({ "project_id": pid, "commands": [
        { "op": "add_clip", "track_id": tracks[0], "source": solid([1.0, 0.0, 0.0, 1.0]), "start": 0, "duration": "0.5s" },
        { "op": "add_marker", "sequence_id": seq, "time": 0 }] }));
    let _ = m.err(
        "edit",
        json!({ "project_id": pid, "commands": [{ "op": "remove_clips", "clip_ids": [999] }] }),
    );
    m.ok(
        "render_contact_sheet",
        json!({ "project_id": pid, "count": 2, "cell_size": 16 }),
    );
    let out = dir.path().join("o.mp4");
    let job = m
        .ok(
            "export",
            json!({ "project_id": pid, "path": out.to_str().unwrap() }),
        )
        .json["job_id"]
        .clone();
    let st = m
        .ok("job_wait", json!({ "job_id": job, "timeout_s": 120 }))
        .json;
    assert_eq!(st["state"], "done", "{st}");
    m.ok(
        "project_save",
        json!({ "project_id": pid, "path": dir.path().join("p.adit").to_str().unwrap() }),
    );
    let _ = m.err(
        "project_open",
        json!({ "path": dir.path().join("missing.adit").to_str().unwrap() }),
    );
    m.ok("undo", json!({ "project_id": pid }));
    // stdin closed -> the server exits.
    m.close_stdin();
    let st = m
        .wait_exit(Duration::from_secs(15))
        .expect("adit mcp exits when stdin closes");
    assert!(st.success(), "clean exit, got {st:?}");
}

#[test]
fn t3_two_projects_in_one_server() {
    let (mut m, _) = Mcp::start(Media::Pattern, None);
    let (a, _, ta) = new_project(&mut m, json!({ "name": "A", "width": 64, "height": 48 }));
    let (b, _, tb) = new_project(&mut m, json!({ "name": "B", "width": 64, "height": 48 }));
    assert_ne!(a, b);
    m.ok("edit", json!({ "project_id": b, "commands": [
        { "op": "add_clip", "track_id": tb[0], "source": solid([0.0, 0.0, 1.0, 1.0]), "start": 0, "duration": "1s" }] }));
    let b_before = m
        .ok("project_get", json!({ "project_id": b, "detail": "full" }))
        .json;
    m.ok("edit", json!({ "project_id": a, "commands": [
        { "op": "add_clip", "track_id": ta[0], "source": solid([1.0, 0.0, 0.0, 1.0]), "start": 0, "duration": "2s" }] }));
    m.ok("edit", json!({ "project_id": a, "commands": [
        { "op": "add_clip", "track_id": ta[1], "source": solid([0.0, 1.0, 0.0, 1.0]), "start": 0, "duration": "1s" }] }));
    assert_eq!(
        m.ok("project_get", json!({ "project_id": b, "detail": "full" }))
            .json,
        b_before
    );
    let u = m.ok("undo", json!({ "project_id": a })).json;
    assert_eq!(u, json!({ "done": 1, "undo_depth": 1, "redo_depth": 1 }));
    assert_eq!(
        m.ok("project_get", json!({ "project_id": b, "detail": "full" }))
            .json,
        b_before
    );
    // Each renders its own content.
    let ra = m.ok("render_frame", json!({ "project_id": a, "format": "png" }));
    let rb = m.ok("render_frame", json!({ "project_id": b, "format": "png" }));
    let ia = image::load_from_memory(&ra.images[0].1).unwrap().to_rgba8();
    let ib = image::load_from_memory(&rb.images[0].1).unwrap().to_rgba8();
    assert_rgb(px(&ia, 32, 24), [255, 0, 0], 2, "a is red");
    assert_rgb(px(&ib, 32, 24), [0, 0, 255], 2, "b is blue");
    let l = m.ok("project_list", json!({})).json;
    assert_eq!(l.as_array().unwrap().len(), 2);
}

#[test]
fn binary_adit_media_pattern_mode() {
    let (mut m, _) = Mcp::start(Media::Pattern, None);
    let (pid, _, tracks) = new_project(
        &mut m,
        json!({ "name": "pat", "width": 64, "height": 64, "audio_tracks": 1 }),
    );
    // Pattern ignores the path: a file that does not exist imports as 64x64, 30 fps, 10 s, 48 kHz stereo.
    let r = m
        .ok(
            "media_import",
            json!({ "project_id": pid, "paths": ["Z:/definitely/not/here.mp4"] }),
        )
        .json;
    let media = &r["media"][0];
    approx(f(&media["duration_s"]), 10.0, 1e-9, "Pattern duration");
    assert_eq!(media["video"]["width"], 64);
    assert_eq!(media["video"]["height"], 64);
    assert_eq!(media["audio"]["sample_rate"], 48000);
    assert_eq!(media["audio"]["channels"], 2);
    let mid = media["id"].as_u64().unwrap();
    m.ok("edit", json!({ "project_id": pid, "commands": [
        { "op": "add_clip", "track_id": tracks[0], "linked_track_id": tracks[1],
          "source": { "type": "media", "media_id": mid }, "start": 0, "duration": "1s", "source_in": "10f" }] }));
    // 5f into the clip -> Pattern frame 15: pixel (x, y) = (x, y, 15).
    let r = m.ok(
        "render_frame",
        json!({ "project_id": pid, "time": "5f", "format": "png" }),
    );
    let img = image::load_from_memory(&r.images[0].1).unwrap().to_rgba8();
    assert_rgb(px(&img, 10, 20), [10, 20, 15], 2, "pattern pixel");
    // Pattern has no silence/scene detection.
    m.err(
        "detect_silence",
        json!({ "project_id": pid, "media_id": mid }),
    );
    m.err(
        "detect_scenes",
        json!({ "project_id": pid, "media_id": mid }),
    );
    // Export still encodes with ffmpeg.
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("pattern.mp4");
    let job = m
        .ok(
            "export",
            json!({ "project_id": pid, "path": out.to_str().unwrap() }),
        )
        .json["job_id"]
        .clone();
    let st = m
        .ok("job_wait", json!({ "job_id": job, "timeout_s": 120 }))
        .json;
    assert_eq!(st["state"], "done", "{st}");
    let p = ffprobe(&out);
    assert_eq!(
        (p.frames, p.width, p.height, p.fps.as_str(), p.has_audio),
        (30, 64, 64, "30/1", true)
    );
}
