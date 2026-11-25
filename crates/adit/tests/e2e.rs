//! End-to-end story with real ffmpeg over `adit mcp`, plus the ffmpeg-only tools (T-IMPORT
//! failures, T-SILENCE, T-SCENES).
mod common;
use common::*;
use serde_json::{Value, json};
use std::time::Duration;

fn clip(summary: &Value, track: u64, i: usize) -> &Value {
    let t = summary["sequences"][0]["tracks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"].as_u64() == Some(track))
        .expect("track in summary");
    &t["clips"][i]
}

fn png_at(m: &mut Mcp, pid: &str, time: &str) -> image::RgbaImage {
    let r = m.ok(
        "render_frame",
        json!({ "project_id": pid, "time": time, "format": "png", "max_size": 64 }),
    );
    assert_eq!(r.images[0].0, "image/png");
    image::load_from_memory(&r.images[0].1).unwrap().to_rgba8()
}

#[test]
fn e2e_story_real_ffmpeg() {
    let dir = tempfile::tempdir().unwrap();
    let red = color_clip(dir.path(), "red.mp4", "red", 2);
    let blue = color_clip(dir.path(), "blue.mp4", "blue", 2);
    let (mut m, _) = Mcp::start(Media::Ffmpeg, Some(dir.path()));
    let (pid, _, tr) = new_project(
        &mut m,
        json!({ "name": "story", "width": 64, "height": 64, "fps": "30" }),
    );
    let (v1, v2, a1) = (tr[0], tr[1], tr[2]);

    // Import: one relative (resolved against the server's cwd), one absolute.
    let r = m
        .ok(
            "media_import",
            json!({ "project_id": pid, "paths": ["red.mp4", blue.to_str().unwrap()] }),
        )
        .json;
    let ms = r["media"].as_array().unwrap();
    assert_eq!(ms.len(), 2);
    for x in ms {
        approx(f(&x["duration_s"]), 2.0, 0.05, "fixture duration");
        assert_eq!(x["video"]["width"], 64);
        assert_eq!(x["audio"]["sample_rate"], 48000);
    }
    let (rm, bm) = (ms[0]["id"].as_u64().unwrap(), ms[1]["id"].as_u64().unwrap());
    let sm = m.ok("project_get", json!({ "project_id": pid })).json;
    let stored = sm["media"][0]["path"].as_str().unwrap().to_string();
    assert!(
        std::path::Path::new(&stored).is_absolute(),
        "T-IMPORT: stored absolute: {stored}"
    );
    assert!(same_path(&stored, &red));

    // Back to back with linked audio: R [0, 1.5s), B [1.5s, 3s).
    let r = m.ok("edit", json!({ "project_id": pid, "commands": [
        { "op": "add_clip", "track_id": v1, "linked_track_id": a1, "source": { "type": "media", "media_id": rm },
          "start": 0, "duration": "1.5s" },
        { "op": "add_clip", "track_id": v1, "linked_track_id": a1, "source": { "type": "media", "media_id": bm },
          "start": "1.5s", "duration": "45f" }] })).json;
    let rv = r["created"][0][0].as_u64().unwrap();
    let bv = r["created"][1][0].as_u64().unwrap();
    let ba = r["created"][1][1].as_u64().unwrap();

    // Split R at 0.5 s (linked audio too), then ripple-delete the left half [0, 0.5s).
    let r = m.ok("edit", json!({ "project_id": pid, "commands": [{ "op": "split_clip", "clip_id": rv, "time": "0.5s" }] })).json;
    let created = r["created"][0].as_array().unwrap().clone();
    assert_eq!(
        created.len(),
        2,
        "split of a linked pair creates two right parts"
    );
    let rv2 = created[0].as_u64().unwrap();
    let ra2 = created[1].as_u64().unwrap();
    m.ok("edit", json!({ "project_id": pid, "commands": [{ "op": "ripple_delete_clips", "clip_ids": [rv] }] }));

    // Now: R' [0, 1s) source_in 0.5 s, B [1s, 2.5s); same on A1.
    let sm = m.ok("project_get", json!({ "project_id": pid })).json;
    for (track, ids) in [(v1, [rv2, bv]), (a1, [ra2, ba])] {
        let c0 = clip(&sm, track, 0);
        let c1 = clip(&sm, track, 1);
        assert_eq!(c0["id"].as_u64(), Some(ids[0]));
        assert_eq!(c1["id"].as_u64(), Some(ids[1]));
        approx(f(&c0["start_s"]), 0.0, 1e-9, "R' start");
        approx(f(&c0["end_s"]), 1.0, 1e-9, "R' end");
        approx(f(&c0["source_in_s"]), 0.5, 1e-9, "R' source_in");
        approx(f(&c1["start_s"]), 1.0, 1e-9, "B start");
        approx(f(&c1["end_s"]), 2.5, 1e-9, "B end");
        assert!(clip(&sm, track, 2).is_null(), "two clips only");
    }
    assert!(!clip(&sm, v1, 0)["link"].is_null());
    assert_eq!(
        clip(&sm, v1, 0)["link"],
        clip(&sm, a1, 0)["link"],
        "right halves linked to each other"
    );
    assert_eq!(clip(&sm, v1, 1)["link"], clip(&sm, a1, 1)["link"]);
    assert_ne!(clip(&sm, v1, 0)["link"], clip(&sm, v1, 1)["link"]);

    // Cross dissolve 0.5 s on the cut at 1 s -> window [0.75 s, 1.25 s).
    // Text on V2 [2s, 2.5s). New filter (SPEC E-5) on B with gain 0.5: blue -> (0.5, 0, 0).
    m.ok("edit", json!({ "project_id": pid, "commands": [
        { "op": "add_transition", "track_id": v1, "left": rv2, "right": bv, "effect": "cross_dissolve", "duration": "0.5s" },
        { "op": "add_clip", "track_id": v2, "source": { "type": "text", "text": {
            "content": "HI", "font": "", "size": 32.0, "color": [1.0, 1.0, 1.0, 1.0], "align": "center",
            "bold": true, "italic": false } }, "start": "2s", "duration": "0.5s" },
        { "op": "define_effect", "def": { "name": "swap", "kind": "filter", "wgsl": SWAP_WGSL,
            "params": [{ "name": "gain", "default": 1.0, "min": 0.0, "max": 2.0 }] } },
        { "op": "add_effect", "clip_id": bv, "effect": "swap", "params": { "gain": 0.5 } }] }));

    // render_frame checks (PNG, lossless; sources are h264 so allow ±12).
    let img = png_at(&mut m, &pid, "0.25s");
    assert_rgb(px(&img, 32, 32), [255, 0, 0], 12, "0.25 s: red");
    // 1.0 s: progress 0.5 -> 0.5 * red + 0.5 * (0.5, 0, 0) = (0.75, 0, 0) -> 191.
    let img = png_at(&mut m, &pid, "1s");
    assert_rgb(px(&img, 32, 32), [191, 0, 0], 12, "1.0 s: mid dissolve");
    let img = png_at(&mut m, &pid, "1.75s");
    assert_rgb(
        px(&img, 32, 32),
        [128, 0, 0],
        12,
        "1.75 s: swapped blue at gain 0.5",
    );
    let img = png_at(&mut m, &pid, "2.25s");
    assert_rgb(
        px(&img, 1, 1),
        [128, 0, 0],
        12,
        "2.25 s: corner outside the text",
    );
    let white = img
        .pixels()
        .filter(|p| p[0] > 200 && p[1] > 200 && p[2] > 200)
        .count();
    assert!(white >= 10, "text draws white pixels, found {white}");

    // Audio: the sine plays everywhere in [0, 2.5s); ffmpeg's sine is amplitude 1/8 (-18 dBFS peak).
    let lv = m
        .ok("audio_levels", json!({ "project_id": pid, "buckets": 5 }))
        .json;
    for v in lv["peak_db"].as_array().unwrap() {
        approx(
            f(v),
            -18.06,
            4.0,
            "sine peak (aac overshoot measured about +2 dB)",
        );
    }

    // Export to mp4 and verify with ffprobe / ffmpeg.
    let out = dir.path().join("story.mp4");
    let job = m
        .ok(
            "export",
            json!({ "project_id": pid, "path": out.to_str().unwrap(), "crf": 10 }),
        )
        .json["job_id"]
        .clone();
    let st = m
        .ok("job_wait", json!({ "job_id": job, "timeout_s": 300 }))
        .json;
    assert_eq!(st["state"], "done", "{st}");
    // X-1: frames n with n / 30 < 2.5 s -> n = 0..74 -> 75 frames.
    assert_eq!(st["frames_done"], 75);
    assert_eq!(st["frames_total"], 75);
    let p = ffprobe(&out);
    assert_eq!(
        (p.frames, p.width, p.height, p.fps.as_str()),
        (75, 64, 64, "30/1")
    );
    assert!(p.has_audio, "export has an audio stream");
    assert_rgb(
        rgb_at(&ffmpeg_frame(&out, 7, 64, 64), 64, 32, 32),
        [255, 0, 0],
        16,
        "frame 7 red",
    );
    assert_rgb(
        rgb_at(&ffmpeg_frame(&out, 30, 64, 64), 64, 32, 32),
        [191, 0, 0],
        20,
        "frame 30 dissolve",
    );
    assert_rgb(
        rgb_at(&ffmpeg_frame(&out, 52, 64, 64), 64, 32, 32),
        [128, 0, 0],
        16,
        "frame 52 swapped blue",
    );
    assert_rgb(
        rgb_at(&ffmpeg_frame(&out, 67, 64, 64), 64, 2, 2),
        [128, 0, 0],
        16,
        "frame 67 corner",
    );

    // The saved project renders the same through `adit render`.
    let proj = dir.path().join("story.adit");
    m.ok(
        "project_save",
        json!({ "project_id": pid, "path": proj.to_str().unwrap() }),
    );
    let cli = dir.path().join("cli.mp4");
    let (code, _, err) = run_adit(
        &[
            "render",
            proj.to_str().unwrap(),
            "-o",
            cli.to_str().unwrap(),
        ],
        Duration::from_secs(300),
    );
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(ffprobe(&cli).frames, 75);
    assert_rgb(
        rgb_at(&ffmpeg_frame(&cli, 52, 64, 64), 64, 32, 32),
        [128, 0, 0],
        16,
        "cli frame 52",
    );
}

#[test]
fn e2e_import_failure_adds_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let red = color_clip(dir.path(), "red.mp4", "red", 1);
    let txt = dir.path().join("notes.txt");
    std::fs::write(&txt, "not a video").unwrap();
    let (mut m, _) = Mcp::start(Media::Ffmpeg, None);
    let (pid, _, _) = new_project(&mut m, json!({ "name": "imp" }));
    let before = m
        .ok(
            "project_get",
            json!({ "project_id": pid, "detail": "full" }),
        )
        .json;
    let missing = dir.path().join("missing.mp4");
    m.err(
        "media_import",
        json!({ "project_id": pid, "paths": [red.to_str().unwrap(), missing.to_str().unwrap()] }),
    );
    m.err(
        "media_import",
        json!({ "project_id": pid, "paths": [red.to_str().unwrap(), txt.to_str().unwrap()] }),
    );
    assert_eq!(
        m.ok(
            "project_get",
            json!({ "project_id": pid, "detail": "full" })
        )
        .json,
        before,
        "T-IMPORT: nothing added"
    );
    // Without ADIT_MEDIA the real backend is used: a missing file cannot be imported.
    m.err(
        "media_import",
        json!({ "project_id": pid, "paths": ["Z:/definitely/not/here.mp4"] }),
    );
}

#[test]
fn e2e_detect_silence_and_scenes() {
    let dir = tempfile::tempdir().unwrap();
    let cut = cut_and_silence_clip(dir.path());
    let (mut m, _) = Mcp::start(Media::Ffmpeg, None);
    let (pid, _, _) = new_project(&mut m, json!({ "name": "det" }));
    let r = m
        .ok(
            "media_import",
            json!({ "project_id": pid, "paths": [cut.to_str().unwrap()] }),
        )
        .json;
    let mid = r["media"][0]["id"].as_u64().unwrap();

    // Defaults noise -35 dB, min 0.4 s: silence [1 s, 2 s).
    let s = m
        .ok(
            "detect_silence",
            json!({ "project_id": pid, "media_id": mid }),
        )
        .json;
    let ranges = s["ranges"].as_array().unwrap();
    assert_eq!(ranges.len(), 1, "{s}");
    approx(f(&ranges[0]["start_s"]), 1.0, 0.1, "silence start");
    approx(f(&ranges[0]["end_s"]), 2.0, 0.1, "silence end");
    // min_duration as a T-4 string longer than the gap -> nothing.
    let s = m
        .ok(
            "detect_silence",
            json!({ "project_id": pid, "media_id": mid, "noise_db": -30, "min_duration": "1.5s" }),
        )
        .json;
    assert_eq!(s["ranges"], json!([]));

    // Red -> blue at 1 s.
    let c = m
        .ok(
            "detect_scenes",
            json!({ "project_id": pid, "media_id": mid }),
        )
        .json;
    let cuts = c["cuts_s"].as_array().unwrap();
    assert_eq!(cuts.len(), 1, "{c}");
    approx(f(&cuts[0]), 1.0, 0.034, "cut within one frame of 1 s");
    let c = m
        .ok(
            "detect_scenes",
            json!({ "project_id": pid, "media_id": mid, "threshold": 0.99 }),
        )
        .json;
    assert!(c["cuts_s"].as_array().unwrap().len() <= 1);
    m.err(
        "detect_silence",
        json!({ "project_id": pid, "media_id": 999_999 }),
    );
}
