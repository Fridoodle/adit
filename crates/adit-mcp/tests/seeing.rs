//! T-FRAME, T-SHEET, T-LEVELS, T-SILENCE / T-SCENES (Pattern backend), T-5 image blocks.
mod common;
use common::*;
use serde_json::json;

fn red_then_blue(s: &adit_mcp::Session, extra: serde_json::Value) -> Proj {
    let p = new_project(s, extra);
    add_solid(
        s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    add_solid(
        s,
        &p.id,
        p.v1(),
        json!("1s"),
        json!("1s"),
        [0.0, 0.0, 1.0, 1.0],
    );
    p
}

// ---------------------------------------------------------------- T-FRAME

#[test]
fn t_frame_size_rule() {
    let s = session();
    let p = red_then_blue(&s, json!({})); // 64x48
    // (max_size, expected w, h): never upscaled; longer side <= max_size; aspect 4:3 kept.
    for (max, w, h) in [
        (640, 64, 48),
        (64, 64, 48),
        (48, 48, 36),
        (32, 32, 24),
        (16, 16, 12),
    ] {
        let o = call(
            &s,
            "render_frame",
            json!({ "project_id": p.id, "max_size": max, "format": "png" }),
        );
        let img = decode(&o.images[0].bytes);
        assert_eq!((img.width(), img.height()), (w, h), "max_size {max}");
        assert_eq!(o.json["width"], w);
        assert_eq!(o.json["height"], h);
    }
    // Default max_size = 640: 64x48 stays 64x48.
    let o = call(&s, "render_frame", json!({ "project_id": p.id }));
    let img = decode(&o.images[0].bytes);
    assert_eq!((img.width(), img.height()), (64, 48));
    // Portrait 48x64 with max 32 -> 24x32.
    let q = red_then_blue(&s, json!({ "width": 48, "height": 64 }));
    let o = call(
        &s,
        "render_frame",
        json!({ "project_id": q.id, "max_size": 32, "format": "png" }),
    );
    let img = decode(&o.images[0].bytes);
    assert_eq!((img.width(), img.height()), (24, 32));
    // 1280x720 with default 640 -> 640x360 (downscale).
    let r = red_then_blue(&s, json!({ "width": 1280, "height": 720 }));
    let o = call(&s, "render_frame", json!({ "project_id": r.id }));
    let img = decode(&o.images[0].bytes);
    assert_eq!((img.width(), img.height()), (640, 360));
    assert_px(&img, 320, 180, [255, 0, 0], 12, "scaled red");
}

#[test]
fn t_frame_formats_magic_and_colour() {
    let s = session();
    let p = red_then_blue(&s, json!({}));
    // Default format jpeg, default time 0 -> red.
    let o = call(&s, "render_frame", json!({ "project_id": p.id }));
    assert_eq!(o.images.len(), 1, "T-5: one image block");
    assert_eq!(o.images[0].mime, "image/jpeg");
    assert_eq!(&o.images[0].bytes[..3], &[0xFF, 0xD8, 0xFF], "JPEG magic");
    let img = decode(&o.images[0].bytes);
    assert_px(&img, 32, 24, [255, 0, 0], 12, "jpeg red at default time 0");
    assert_eq!(o.json["frame"], 0);
    approx(f(&o.json["time_s"]), 0.0, 1e-12, "default time");
    assert_eq!(o.json["width"], 64);
    assert_eq!(o.json["height"], 48);

    let o = call(
        &s,
        "render_frame",
        json!({ "project_id": p.id, "format": "png", "time": "1.5s" }),
    );
    assert_eq!(o.images[0].mime, "image/png");
    assert_eq!(
        &o.images[0].bytes[..8],
        &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A],
        "PNG magic"
    );
    let img = decode(&o.images[0].bytes);
    for (x, y) in [(0, 0), (63, 47), (32, 24)] {
        assert_px(&img, x, y, [0, 0, 255], 2, "png blue at 1.5 s");
    }
    assert_eq!(o.json["frame"], 45);
    // After the end: black canvas (SPEC V-1).
    let (img, _) = frame_png(&s, &p.id, json!("3s"));
    assert_px(&img, 32, 24, [0, 0, 0], 2, "empty canvas is black");
    call_err(
        &s,
        "render_frame",
        json!({ "project_id": p.id, "format": "gif" }),
    );
}

#[test]
fn t_frame_media_pattern_pixels() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(&s, json!({ "width": 64, "height": 64 }));
    let m = import_pattern(&s, &p.id, dir.path(), "pat.mov");
    edit(
        &s,
        &p.id,
        json!([{ "op": "add_clip", "track_id": p.v1(), "source": media_src(m), "start": 0, "duration": "2s", "source_in": "10f" }]),
    );
    // Time 5f -> source time 15f -> Pattern frame 15: pixel (x, y) = (x, y, 15).
    let (img, j) = frame_png(&s, &p.id, json!("5f"));
    assert_eq!(j["frame"], 5);
    assert_px(&img, 10, 20, [10, 20, 15], 2, "pattern pixel");
    assert_px(&img, 50, 3, [50, 3, 15], 2, "pattern pixel");
}

#[test]
fn t_frame_rendering_changes_nothing() {
    let s = session();
    let p = red_then_blue(&s, json!({}));
    let before = full(&s, &p.id);
    frame_png(&s, &p.id, json!("0.5s"));
    call(
        &s,
        "render_contact_sheet",
        json!({ "project_id": p.id, "count": 2, "cell_size": 16 }),
    );
    call(
        &s,
        "audio_levels",
        json!({ "project_id": p.id, "buckets": 2 }),
    );
    assert_eq!(full(&s, &p.id), before);
}

// ---------------------------------------------------------------- T-SHEET

#[test]
fn t_sheet_layout_times_and_colours() {
    let s = session();
    let p = red_then_blue(&s, json!({})); // 64x48, end = 2 s
    // count 4, columns 3 -> 2 rows; cell longer side 32 -> 32x24; image 96x48.
    let o = call(
        &s,
        "render_contact_sheet",
        json!({ "project_id": p.id, "count": 4, "columns": 3, "cell_size": 32 }),
    );
    assert_eq!(o.images.len(), 1);
    let img = decode(&o.images[0].bytes);
    assert_eq!((img.width(), img.height()), (96, 48));
    assert_eq!(o.json["columns"], 3);
    assert_eq!(o.json["cell_width"], 32);
    assert_eq!(o.json["cell_height"], 24);
    // times = 0 + (i + 0.5) * 2 / 4 = 0.25, 0.75, 1.25, 1.75
    let t: Vec<f64> = o.json["times_s"]
        .as_array()
        .unwrap()
        .iter()
        .map(f)
        .collect();
    assert_eq!(t.len(), 4);
    for (g, w) in t.iter().zip([0.25, 0.75, 1.25, 1.75]) {
        approx(*g, w, 1e-6, "times_s");
    }
    // Row by row: (0,0) red, (1,0) red, (2,0) blue, (0,1) blue. Cell centres.
    let tol = 16; // the sheet format is not fixed (may be JPEG)
    assert_px(&img, 16, 12, [255, 0, 0], tol, "cell 0");
    assert_px(&img, 48, 12, [255, 0, 0], tol, "cell 1");
    assert_px(&img, 80, 12, [0, 0, 255], tol, "cell 2");
    assert_px(&img, 16, 36, [0, 0, 255], tol, "cell 3 (second row)");
}

#[test]
fn t_sheet_defaults_and_range() {
    let s = session();
    let p = red_then_blue(&s, json!({}));
    // Defaults: count 9, columns 3, cell_size 320 -> cells 320x240 (scaled to longer side 320), 960x720.
    let o = call(&s, "render_contact_sheet", json!({ "project_id": p.id }));
    let img = decode(&o.images[0].bytes);
    assert_eq!((img.width(), img.height()), (960, 720));
    assert_eq!(o.json["cell_width"], 320);
    assert_eq!(o.json["cell_height"], 240);
    let t: Vec<f64> = o.json["times_s"]
        .as_array()
        .unwrap()
        .iter()
        .map(f)
        .collect();
    assert_eq!(t.len(), 9);
    for (i, g) in t.iter().enumerate() {
        approx(*g, (i as f64 + 0.5) * 2.0 / 9.0, 1e-6, "default times");
    }
    // cell 0 at 0.111 s red; cell 8 (row 2, col 2) at 1.889 s blue.
    assert_px(&img, 160, 120, [255, 0, 0], 16, "cell 0");
    assert_px(&img, 800, 600, [0, 0, 255], 16, "cell 8");

    // start/end as T-4 strings: [1s, 2s), count 2 -> 1.25, 1.75.
    let o = call(
        &s,
        "render_contact_sheet",
        json!({ "project_id": p.id, "start": "1s", "end": "60f", "count": 2, "columns": 2, "cell_size": 16 }),
    );
    let t: Vec<f64> = o.json["times_s"]
        .as_array()
        .unwrap()
        .iter()
        .map(f)
        .collect();
    assert_eq!(t.len(), 2);
    approx(t[0], 1.25, 1e-6, "t0");
    approx(t[1], 1.75, 1e-6, "t1");
    let img = decode(&o.images[0].bytes);
    assert_eq!((img.width(), img.height()), (32, 12));
}

// ---------------------------------------------------------------- T-LEVELS

/// Assumption (contract silent): rms = sqrt(mean of squares over all samples of both channels),
/// peak = max |sample| over both channels, both in dBFS = 20 log10(x).
#[test]
fn t_levels_pattern_audio() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(&s, json!({}));
    let m = import_pattern(&s, &p.id, dir.path(), "tone.mov");
    let a1 = p.tracks[2];
    let r = edit(
        &s,
        &p.id,
        json!([
            // video solid makes the sequence 4 s long; audio only in [0, 2s)
            { "op": "add_clip", "track_id": p.v1(), "source": solid([0.0, 0.0, 0.0, 1.0]), "start": 0, "duration": "4s" },
            { "op": "add_clip", "track_id": a1, "source": media_src(m), "start": 0, "duration": "1s" },
            { "op": "add_clip", "track_id": a1, "source": media_src(m), "start": "1s", "duration": "1s", "source_in": "1s" }
        ]),
    );
    let quiet = r["created"][2][0].as_u64().unwrap();
    edit(
        &s,
        &p.id,
        json!([{ "op": "set_param", "target": { "clip_id": quiet, "name": "volume" }, "value": -6.0206 }]),
    );

    let o = call(
        &s,
        "audio_levels",
        json!({ "project_id": p.id, "buckets": 4 }),
    )
    .json;
    approx(f(&o["bucket_s"]), 1.0, 1e-9, "bucket_s = 4 s / 4");
    let rms: Vec<f64> = o["rms_db"].as_array().unwrap().iter().map(f).collect();
    let peak: Vec<f64> = o["peak_db"].as_array().unwrap().iter().map(f).collect();
    assert_eq!(rms.len(), 4);
    assert_eq!(peak.len(), 4);
    // Bucket 0: L 0.5, R 0.25. rms = sqrt((0.25 + 0.0625) / 2) = sqrt(0.15625) -> 10 log10(0.15625) = -8.0618 dB.
    // peak = 0.5 -> -6.0206 dB.
    approx(rms[0], -8.0618, 0.05, "rms bucket 0");
    approx(peak[0], -6.0206, 0.05, "peak bucket 0");
    // Bucket 1: gain 10^(-6.0206/20) = 0.5 -> L 0.25, R 0.125 (SPEC A-3).
    // rms = sqrt((0.0625 + 0.015625) / 2) = sqrt(0.0390625) -> -14.0824 dB; peak 0.25 -> -12.0412 dB.
    approx(rms[1], -14.0824, 0.05, "rms bucket 1");
    approx(peak[1], -12.0412, 0.05, "peak bucket 1");
    // Independent of how channels combine: halving the signal lowers both by 6.0206 dB.
    approx(rms[0] - rms[1], 6.0206, 0.05, "rms difference");
    approx(peak[0] - peak[1], 6.0206, 0.05, "peak difference");
    // Silence -> -120.
    for i in 2..4 {
        approx(rms[i], -120.0, 0.01, "silent rms");
        approx(peak[i], -120.0, 0.01, "silent peak");
    }

    // start/end strings: [1s, 2s) in 2 buckets of 0.5 s, both the quiet clip.
    let o = call(
        &s,
        "audio_levels",
        json!({ "project_id": p.id, "start": "1s", "end": "60f", "buckets": 2 }),
    )
    .json;
    approx(f(&o["bucket_s"]), 0.5, 1e-9, "bucket_s");
    for v in o["rms_db"].as_array().unwrap() {
        approx(f(v), -14.0824, 0.05, "rms in [1s,2s)");
    }

    // Defaults: start 0, end = sequence end (4 s), 50 buckets of 0.08 s.
    let o = call(&s, "audio_levels", json!({ "project_id": p.id })).json;
    approx(f(&o["bucket_s"]), 0.08, 1e-9, "4 s / 50");
    assert_eq!(o["rms_db"].as_array().unwrap().len(), 50);
    assert_eq!(o["peak_db"].as_array().unwrap().len(), 50);
    approx(f(&o["peak_db"][0]), -6.0206, 0.05, "first default bucket");
    approx(f(&o["peak_db"][49]), -120.0, 0.01, "last default bucket");

    // Muting the track silences everything (SPEC A-1).
    edit(
        &s,
        &p.id,
        json!([{ "op": "set_track", "track_id": a1, "muted": true }]),
    );
    let o = call(
        &s,
        "audio_levels",
        json!({ "project_id": p.id, "buckets": 4 }),
    )
    .json;
    approx(f(&o["peak_db"][0]), -120.0, 0.01, "muted");
}

// ---------------------------------------------------------------- T-SILENCE / T-SCENES

#[test]
fn t_silence_and_scenes_unavailable_with_pattern() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(&s, json!({}));
    let m = import_pattern(&s, &p.id, dir.path(), "x.mov");
    call_err(
        &s,
        "detect_silence",
        json!({ "project_id": p.id, "media_id": m }),
    );
    call_err(
        &s,
        "detect_scenes",
        json!({ "project_id": p.id, "media_id": m }),
    );
}
