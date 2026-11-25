//! SPEC Part 5 hardening cases that kill (or exhaust) the process today, through
//! `Session::call`. Each case is its own test; the ordinary cases are in `hardening.rs`.
mod common;
use adit_mcp::Session;
use common::*;
use serde_json::json;
use std::sync::mpsc;
use std::time::Duration;

/// Runs `f` on a thread; a hang longer than `secs` fails the test.
fn timed<T: Send + 'static>(secs: u64, f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        let _ = tx.send(r);
    });
    match rx.recv_timeout(Duration::from_secs(secs)) {
        Ok(Ok(v)) => v,
        Ok(Err(p)) => std::panic::resume_unwind(p),
        Err(_) => panic!("test hung: no result after {secs} s"),
    }
}

/// Effect body plus nested blocks, `depth` braces deep. Depth 64/65 overflows the stack today;
/// far deeper input is already refused by naga.
fn braces(depth: usize) -> String {
    format!(
        "fn effect(uv: vec2f) -> vec4f {{ {}{} return src(uv); }}\n",
        "{".repeat(depth - 1),
        "}".repeat(depth - 1)
    )
}

fn alive(s: &Session) {
    let r = call(s, "effect_list", json!({})).json;
    assert!(r["effects"].as_array().is_some_and(|e| !e.is_empty()));
}

// ------------------------------------------------------------------------------------- H-1

#[test]
fn h_1_effect_check_deep_braces_is_not_ok() {
    let s = session();
    let r = call(
        &s,
        "effect_check",
        json!({ "kind": "filter", "wgsl": braces(65), "params": [] }),
    )
    .json;
    assert_eq!(r["ok"], false, "{r}");
    alive(&s);
}

#[test]
fn h_1_define_effect_deep_braces_is_error() {
    let s = session();
    let p = new_project(&s, json!({}));
    let before = full(&s, &p.id);
    let def = json!({ "name": "deep", "kind": "filter", "wgsl": braces(65), "params": [] });
    edit_err(&s, &p.id, json!([{ "op": "define_effect", "def": def }]));
    assert_eq!(full(&s, &p.id), before);
    alive(&s);
}

// ------------------------------------------------------------------------------------- H-5

/// Pattern media (L 0.5, R 0.25) on A1 for [0, 1 s): the sequence ends at 1 s.
fn one_second_of_pattern_audio(s: &Session) -> Proj {
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(s, json!({}));
    let m = import_pattern(s, &p.id, dir.path(), "a.wav");
    edit(
        s,
        &p.id,
        json!([{ "op": "add_clip", "track_id": p.tracks[2], "source": media_src(m),
                 "start": 0, "duration": "1s" }]),
    );
    p
}

#[test]
fn h_5_audio_levels_huge_end_is_clamped() {
    timed(60, || {
        let s = session();
        let p = one_second_of_pattern_audio(&s);
        let r = call(
            &s,
            "audio_levels",
            json!({ "project_id": p.id, "end": 9_000_000_000_000_000_000u64, "buckets": 10 }),
        )
        .json;
        // end clamps to 1 s: 10 buckets of 0.1 s.
        approx(f(&r["bucket_s"]), 0.1, 1e-9, "bucket_s");
        // rms of both channels: sqrt((0.25 + 0.0625) / 2) = 0.3953 -> -8.062 dB; peak 0.5 -> -6.021 dB.
        let rms = r["rms_db"].as_array().unwrap();
        let peak = r["peak_db"].as_array().unwrap();
        assert_eq!((rms.len(), peak.len()), (10, 10));
        for k in 0..10 {
            approx(f(&rms[k]), -8.062, 0.05, "rms_db");
            approx(f(&peak[k]), -6.021, 0.05, "peak_db");
        }
    })
}

#[test]
fn h_5_audio_levels_start_past_clamped_end_is_error() {
    timed(60, || {
        let s = session();
        let p = one_second_of_pattern_audio(&s);
        // end clamps to 1 s, so start 5 s >= end.
        call_err(
            &s,
            "audio_levels",
            json!({ "project_id": p.id, "start": "5s", "end": 9_000_000_000_000_000_000u64 }),
        );
    })
}

// ------------------------------------------------------------------------------------- H-11

#[test]
fn h_11_contact_sheet_over_64_megapixels_is_error() {
    timed(120, || {
        let s = session();
        let p = new_project(&s, json!({}));
        add_solid(&s, &p.id, p.v1(), json!(0), json!("1s"), [1.0; 4]);
        // 2048x1536 cells, 3 columns x 34 rows: 6144 x 52224 = 320,864,256 pixels > 64,000,000.
        call_err(
            &s,
            "render_contact_sheet",
            json!({ "project_id": p.id, "count": 100, "cell_size": 2048 }),
        );
    })
}
