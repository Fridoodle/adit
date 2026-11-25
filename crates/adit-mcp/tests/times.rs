//! T-4 time formats. Every value is checked through `project_get` detail "full" (raw flicks).
mod common;
use common::*;
use serde_json::json;

/// Adds a solid at `start` on V1 and returns its stored start in flicks.
fn start_of(s: &adit_mcp::Session, p: &Proj, start: serde_json::Value) -> i64 {
    let id = add_solid(s, &p.id, p.v1(), start, json!(1000), [1.0, 1.0, 1.0, 1.0]);
    let c = find_clip(&project(s, &p.id), id);
    edit(
        s,
        &p.id,
        json!([{ "op": "remove_clips", "clip_ids": [id] }]),
    );
    c["start"].as_i64().unwrap()
}

#[test]
fn t4_each_format_at_30fps() {
    let s = session();
    let p = new_project(&s, json!({}));
    // integer flicks pass through
    assert_eq!(start_of(&s, &p, json!(705_600_000)), 705_600_000);
    assert_eq!(start_of(&s, &p, json!(1)), 1);
    // seconds: round(1.5 * 705_600_000) = 1_058_400_000
    assert_eq!(start_of(&s, &p, json!("1.5s")), 1_058_400_000);
    assert_eq!(start_of(&s, &p, json!("2s")), 1_411_200_000);
    // round(0.001 * 705_600_000) = 705_600
    assert_eq!(start_of(&s, &p, json!("0.001s")), 705_600);
    // round(1e-9 * 705_600_000) = round(0.7056) = 1
    assert_eq!(start_of(&s, &p, json!("0.000000001s")), 1);
    // frames at 30/1: 45 * 23_520_000 = 1_058_400_000
    assert_eq!(start_of(&s, &p, json!("45f")), 1_058_400_000);
    assert_eq!(start_of(&s, &p, json!("0f")), 0);
    // timecode h:m:s:frames at 30 fps: 1 s + 15 frames = 705_600_000 + 352_800_000
    assert_eq!(start_of(&s, &p, json!("00:00:01:15")), 1_058_400_000);
    // 1 h 2 min 3 s 4 f = 3723 s + 4 f = 3723 * 705_600_000 + 4 * 23_520_000 = 2_626_948_800_000 + 94_080_000
    assert_eq!(start_of(&s, &p, json!("01:02:03:04")), 2_627_042_880_000);
}

#[test]
fn t4_frames_at_29_97_are_exact() {
    let s = session();
    let p = new_project(&s, json!({ "fps": "29.97" }));
    // SPEC Q4: frame 30 at 30000/1001 = floor(30 * 705_600_000 * 1001 / 30000) = 706_305_600
    assert_eq!(start_of(&s, &p, json!("30f")), 706_305_600);
    // frame 1 = 23_543_520
    assert_eq!(start_of(&s, &p, json!("1f")), 23_543_520);
    // seconds do not depend on fps
    assert_eq!(start_of(&s, &p, json!("1.5s")), 1_058_400_000);
    let q = new_project(&s, json!({ "fps": "23.976" }));
    // 24000/1001: frame 1 = floor(705_600_000 * 1001 / 24000) = 29_429_400
    assert_eq!(start_of(&s, &q, json!("1f")), 29_429_400);
}

#[test]
fn t4_every_listed_edit_field_is_converted() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(&s, json!({}));
    let m = import_pattern(&s, &p.id, dir.path(), "m.mov");

    // start, duration, source_in
    let r = edit(
        &s,
        &p.id,
        json!([{ "op": "add_clip", "track_id": p.v1(), "source": media_src(m),
                 "start": "1s", "duration": "60f", "source_in": "00:00:02:00" }]),
    );
    let c = r["created"][0][0].as_u64().unwrap();
    let k = find_clip(&project(&s, &p.id), c);
    assert_eq!(k["start"], 705_600_000, "start \"1s\"");
    assert_eq!(
        k["duration"], 1_411_200_000,
        "duration \"60f\" = 60 * 23_520_000"
    );
    assert_eq!(
        k["source_in"], 1_411_200_000,
        "source_in \"00:00:02:00\" = 2 s"
    );

    // delta (negative seconds and negative frames): source_in 2 s - 0.5 s = 1.5 s
    edit(
        &s,
        &p.id,
        json!([{ "op": "slip_clip", "clip_id": c, "delta": "-0.5s" }]),
    );
    assert_eq!(
        find_clip(&project(&s, &p.id), c)["source_in"],
        1_058_400_000
    );
    // -3f = -70_560_000 -> 987_840_000
    edit(
        &s,
        &p.id,
        json!([{ "op": "slip_clip", "clip_id": c, "delta": "-3f" }]),
    );
    assert_eq!(find_clip(&project(&s, &p.id), c)["source_in"], 987_840_000);
    // integer negative delta
    edit(
        &s,
        &p.id,
        json!([{ "op": "slip_clip", "clip_id": c, "delta": -840_000 }]),
    );
    assert_eq!(find_clip(&project(&s, &p.id), c)["source_in"], 987_000_000);

    // to: clip [1s, 3s) end trimmed to 2.5 s -> duration 1.5 s
    edit(
        &s,
        &p.id,
        json!([{ "op": "trim_clip", "clip_id": c, "edge": "end", "to": "2.5s" }]),
    );
    assert_eq!(find_clip(&project(&s, &p.id), c)["duration"], 1_058_400_000);

    // time: marker, and split_clip
    edit(
        &s,
        &p.id,
        json!([{ "op": "add_marker", "sequence_id": p.seq, "time": "00:00:00:10", "duration": "5f" }]),
    );
    let pj = project(&s, &p.id);
    assert_eq!(
        pj["sequences"][0]["markers"][0]["time"], 235_200_000,
        "10 * 23_520_000"
    );
    assert_eq!(
        pj["sequences"][0]["markers"][0]["duration"], 117_600_000,
        "5 * 23_520_000"
    );
    let r = edit(
        &s,
        &p.id,
        json!([{ "op": "split_clip", "clip_id": c, "time": "1.5s" }]),
    );
    let right = r["created"][0][0].as_u64().unwrap();
    let pj = project(&s, &p.id);
    assert_eq!(
        find_clip(&pj, c)["duration"],
        352_800_000,
        "left = [1s, 1.5s)"
    );
    assert_eq!(find_clip(&pj, right)["start"], 1_058_400_000);

    // end (and start) of a range: V2 solid [0, 4s); lift [1s, 60f) leaves [0,1s) and [2s,4s)
    let v2 = p.tracks[1];
    let x = add_solid(&s, &p.id, v2, json!(0), json!("4s"), [1.0, 0.0, 0.0, 1.0]);
    edit(
        &s,
        &p.id,
        json!([{ "op": "lift_range", "sequence_id": p.seq, "start": "1s", "end": "60f", "track_ids": [v2] }]),
    );
    let tr = find_track(&project(&s, &p.id), v2);
    let spans: Vec<(i64, i64)> = tr["clips"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["start"].as_i64().unwrap(),
                c["duration"].as_i64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        spans,
        vec![(0, 705_600_000), (1_411_200_000, 1_411_200_000)]
    );
    assert_eq!(tr["clips"][0]["id"].as_u64(), Some(x));

    // time in a keyframe, duration of a transition
    let a = add_solid(
        &s,
        &p.id,
        v2,
        json!("5s"),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    let b = add_solid(
        &s,
        &p.id,
        v2,
        json!("6s"),
        json!("1s"),
        [0.0, 1.0, 0.0, 1.0],
    );
    edit(
        &s,
        &p.id,
        json!([
            { "op": "set_keyframe", "target": { "clip_id": a, "name": "opacity" }, "time": "15f", "value": 0.5, "easing": "linear" },
            { "op": "add_transition", "track_id": v2, "left": a, "right": b, "effect": "cross_dissolve", "duration": "0.5s" }
        ]),
    );
    let pj = project(&s, &p.id);
    assert_eq!(
        find_clip(&pj, a)["params"]["opacity"]["keyframes"][0]["time"],
        352_800_000
    );
    assert_eq!(
        find_track(&pj, v2)["transitions"][0]["duration"],
        352_800_000
    );
}

#[test]
fn t4_nested_batch_commands_are_converted() {
    // Reading: the conversion applies to Command JSON at any depth, including `batch`.
    let s = session();
    let p = new_project(&s, json!({}));
    edit(
        &s,
        &p.id,
        json!([{ "op": "batch", "commands": [{ "op": "add_marker", "sequence_id": p.seq, "time": "2s" }] }]),
    );
    assert_eq!(
        project(&s, &p.id)["sequences"][0]["markers"][0]["time"],
        1_411_200_000
    );
}

#[test]
fn t4_frame_strings_use_named_sequence_fps() {
    let s = session();
    let p = new_project(&s, json!({})); // first sequence 30 fps
    let r = edit(
        &s,
        &p.id,
        json!([{ "op": "add_sequence", "name": "film", "width": 64, "height": 48, "fps": { "num": 24, "den": 1 },
                 "sample_rate": 48000, "video_tracks": 1, "audio_tracks": 0 }]),
    );
    let seq24 = r["created"][0][0].as_u64().unwrap();
    // With the tool's sequence_id = the 24 fps sequence: 24f = 1 s.
    call(
        &s,
        "edit",
        json!({ "project_id": p.id, "sequence_id": seq24,
                "commands": [{ "op": "add_marker", "sequence_id": seq24, "time": "24f", "name": "a" }] }),
    );
    // Without sequence_id: first sequence (30 fps): 24f = 24 * 23_520_000 = 564_480_000.
    edit(
        &s,
        &p.id,
        json!([{ "op": "add_marker", "sequence_id": seq24, "time": "24f", "name": "b" }]),
    );
    let pj = project(&s, &p.id);
    let sq = pj["sequences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|q| q["id"].as_u64() == Some(seq24))
        .unwrap()
        .clone();
    let times: Vec<(String, i64)> = sq["markers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| {
            (
                m["name"].as_str().unwrap().to_string(),
                m["time"].as_i64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        times,
        vec![
            ("b".to_string(), 564_480_000),
            ("a".to_string(), 705_600_000)
        ]
    );
}

#[test]
fn t4_frame_string_without_any_sequence_is_error() {
    let s = session();
    let p = new_project(&s, json!({}));
    edit(
        &s,
        &p.id,
        json!([{ "op": "remove_sequence", "sequence_id": p.seq }]),
    );
    let before = full(&s, &p.id);
    // A frame string with no sequence to take the fps from -> error.
    edit_err(
        &s,
        &p.id,
        json!([{ "op": "add_sequence", "name": "n", "width": 8, "height": 8, "fps": { "num": 30, "den": 1 },
                 "sample_rate": 48000, "video_tracks": 1, "audio_tracks": 0 },
               { "op": "add_marker", "sequence_id": 1, "time": "3f" }]),
    );
    assert_eq!(full(&s, &p.id), before);
}

#[test]
fn t4_malformed_strings_name_the_field() {
    let s = session();
    let p = new_project(&s, json!({}));
    for bad in [
        "abc",
        "",
        "1.5x",
        "s",
        "f",
        "1..5s",
        "--1s",
        "00:00:01",
        "00:00:01:15:00",
        "1s2",
        "0x10s",
    ] {
        let e = edit_err(
            &s,
            &p.id,
            json!([{ "op": "add_clip", "track_id": p.v1(), "source": solid([1.0, 0.0, 0.0, 1.0]),
                     "start": bad, "duration": "1s" }]),
        );
        assert!(
            e.contains("start"),
            "malformed start {bad:?}: error must name the field, got: {e}"
        );
        let e = edit_err(
            &s,
            &p.id,
            json!([{ "op": "add_clip", "track_id": p.v1(), "source": solid([1.0, 0.0, 0.0, 1.0]),
                     "start": 0, "duration": bad }]),
        );
        assert!(
            e.contains("duration"),
            "malformed duration {bad:?}: got: {e}"
        );
    }
    let c = add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    let before2 = full(&s, &p.id);
    let e = edit_err(
        &s,
        &p.id,
        json!([{ "op": "trim_clip", "clip_id": c, "edge": "end", "to": "zz" }]),
    );
    assert!(e.contains("to"), "{e}");
    let e = edit_err(
        &s,
        &p.id,
        json!([{ "op": "split_clip", "clip_id": c, "time": "half" }]),
    );
    assert!(e.contains("time"), "{e}");
    assert_eq!(full(&s, &p.id), before2);
    // Tool-level time arguments too.
    let e = call_err(
        &s,
        "render_frame",
        json!({ "project_id": p.id, "time": "soon" }),
    );
    assert!(e.contains("time"), "{e}");
    let e = call_err(
        &s,
        "audio_levels",
        json!({ "project_id": p.id, "start": "1q" }),
    );
    assert!(e.contains("start"), "{e}");
}

#[test]
fn t4_negative_where_not_allowed_is_error() {
    let s = session();
    let p = new_project(&s, json!({}));
    let before = full(&s, &p.id);
    edit_err(
        &s,
        &p.id,
        json!([{ "op": "add_clip", "track_id": p.v1(), "source": solid([1.0, 0.0, 0.0, 1.0]), "start": "-0.5s", "duration": "1s" }]),
    );
    edit_err(
        &s,
        &p.id,
        json!([{ "op": "add_clip", "track_id": p.v1(), "source": solid([1.0, 0.0, 0.0, 1.0]), "start": 0, "duration": "-3f" }]),
    );
    assert_eq!(full(&s, &p.id), before);
}

#[test]
fn t4_tool_time_arguments() {
    let s = session();
    let p = new_project(&s, json!({}));
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("3s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    // render_frame time "1.5s" -> frame 45 at 30 fps.
    let (_, j) = frame_png(&s, &p.id, json!("1.5s"));
    approx(f(&j["time_s"]), 1.5, 1e-9, "time_s");
    assert_eq!(j["frame"], 45);
    let (_, j) = frame_png(&s, &p.id, json!("00:00:02:03"));
    assert_eq!(j["frame"], 63, "2 s * 30 + 3");
    approx(f(&j["time_s"]), 2.1, 1e-9, "63 / 30");
    let (_, j) = frame_png(&s, &p.id, json!(705_600_000));
    assert_eq!(j["frame"], 30);
}
