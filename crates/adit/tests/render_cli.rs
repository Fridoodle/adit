//! `adit render` and usage errors (Binary section).
mod common;
use common::*;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::Duration;

fn solid_clip(id: u64, start: i64, dur: i64, color: [f64; 4]) -> Value {
    json!({ "id": id, "name": "Solid", "start": start, "duration": dur,
            "source": { "type": "solid", "color": color }, "source_in": 0, "speed": 1.0,
            "reverse": false, "enabled": true, "link": null, "group": null, "blend": "normal",
            "audio_filter": "", "params": {}, "effects": [] })
}

fn track(id: u64, name: &str, clips: Vec<Value>) -> Value {
    json!({ "id": id, "kind": "video", "name": name, "locked": false, "muted": false, "solo": false,
            "hidden": false, "clips": clips, "transitions": [] })
}

/// Hand-written adit_core::Project JSON (model.rs): sequence 1 (64x48, 30 fps) has a red solid
/// [0, 1s); sequence 4 (32x32, 30 fps) has a blue solid [0, 0.5s). No audio tracks.
fn write_project(dir: &Path) -> PathBuf {
    let p = json!({
        "name": "cli", "next_id": 7, "media": [], "effects": [],
        "sequences": [
            { "id": 1, "name": "Main", "width": 64, "height": 48, "fps": { "num": 30, "den": 1 }, "sample_rate": 48000,
              "tracks": [track(2, "V1", vec![solid_clip(3, 0, S, [1.0, 0.0, 0.0, 1.0])])], "markers": [] },
            { "id": 4, "name": "Other", "width": 32, "height": 32, "fps": { "num": 30, "den": 1 }, "sample_rate": 48000,
              "tracks": [track(5, "V1", vec![solid_clip(6, 0, S / 2, [0.0, 0.0, 1.0, 1.0])])], "markers": [] }
        ]
    });
    let path = dir.join("cli.adit");
    std::fs::write(&path, serde_json::to_string_pretty(&p).unwrap()).unwrap();
    path
}

const T: Duration = Duration::from_secs(120);

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

#[test]
fn binary_render_default_sequence_h264() {
    let dir = tempfile::tempdir().unwrap();
    let proj = write_project(dir.path());
    let out = dir.path().join("out.mp4");
    let (code, _, err) = run_adit(&["render", s(&proj), "-o", s(&out)], T);
    assert_eq!(code, Some(0), "render succeeds; stderr: {err}");
    let p = ffprobe(&out);
    // X-1: 1 s at 30 fps -> 30 frames.
    assert_eq!(
        (p.frames, p.width, p.height, p.fps.as_str()),
        (30, 64, 48, "30/1")
    );
    assert!(
        !p.has_audio,
        "no audio tracks -> no audio stream (SPEC X-1)"
    );
    assert_rgb(
        rgb_at(&ffmpeg_frame(&out, 10, 64, 48), 64, 32, 24),
        [255, 0, 0],
        12,
        "frame 10 red",
    );
}

#[test]
fn binary_render_sequence_and_codecs() {
    let dir = tempfile::tempdir().unwrap();
    let proj = write_project(dir.path());
    let out = dir.path().join("other.mp4");
    let (code, _, err) = run_adit(
        &[
            "render",
            s(&proj),
            "-o",
            s(&out),
            "--sequence",
            "4",
            "--codec",
            "h264",
        ],
        T,
    );
    assert_eq!(code, Some(0), "{err}");
    let p = ffprobe(&out);
    // 0.5 s at 30 fps -> 15 frames, 32x32.
    assert_eq!((p.frames, p.width, p.height), (15, 32, 32));
    assert_rgb(
        rgb_at(&ffmpeg_frame(&out, 7, 32, 32), 32, 16, 16),
        [0, 0, 255],
        12,
        "blue",
    );

    let gif = dir.path().join("o.gif");
    let (code, _, err) = run_adit(&["render", s(&proj), "-o", s(&gif), "--codec", "gif"], T);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(&std::fs::read(&gif).unwrap()[..4], b"GIF8");

    let mov = dir.path().join("o.mov");
    let (code, _, err) = run_adit(&["render", s(&proj), "-o", s(&mov), "--codec", "prores"], T);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(ffprobe(&mov).frames, 30);

    let h265 = dir.path().join("o265.mp4");
    let (code, _, err) = run_adit(&["render", s(&proj), "-o", s(&h265), "--codec", "h265"], T);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(ffprobe(&h265).frames, 30);
}

#[test]
fn binary_render_failures_exit_nonzero_with_message() {
    let dir = tempfile::tempdir().unwrap();
    let proj = write_project(dir.path());
    let out = dir.path().join("x.mp4");
    // Missing project file.
    let (code, _, err) = run_adit(
        &["render", s(&dir.path().join("missing.adit")), "-o", s(&out)],
        T,
    );
    assert!(code.is_some_and(|c| c != 0), "missing project -> non-zero");
    assert!(!err.trim().is_empty(), "message on stderr");
    // Not JSON.
    let bad = dir.path().join("bad.adit");
    std::fs::write(&bad, "nope").unwrap();
    let (code, _, err) = run_adit(&["render", s(&bad), "-o", s(&out)], T);
    assert!(code.is_some_and(|c| c != 0));
    assert!(!err.trim().is_empty());
    // Unknown sequence.
    let (code, _, err) = run_adit(&["render", s(&proj), "-o", s(&out), "--sequence", "99"], T);
    assert!(code.is_some_and(|c| c != 0));
    assert!(err.contains("99"), "message names the sequence: {err}");
    // Unwritable output.
    let deep = dir.path().join("no_dir").join("deeper").join("x.mp4");
    let (code, _, err) = run_adit(&["render", s(&proj), "-o", s(&deep)], T);
    assert!(code.is_some_and(|c| c != 0));
    assert!(!err.trim().is_empty());
    assert!(!out.exists() && !deep.exists(), "no output left behind");
}

#[test]
fn binary_bad_usage_exit_2() {
    let dir = tempfile::tempdir().unwrap();
    let proj = write_project(dir.path());
    let out = dir.path().join("u.mp4");
    let short = Duration::from_secs(20);
    for args in [
        vec![],
        vec!["frobnicate"],
        vec!["--frobnicate"],
        vec!["render"],
        vec!["render", s(&proj)],
        vec!["render", s(&proj), "-o"],
        vec!["render", s(&proj), "-o", s(&out), "--codec", "nope"],
        vec![
            "render",
            s(&proj),
            "-o",
            s(&out),
            "--sequence",
            "notanumber",
        ],
        vec!["serve", "--port", "notanumber"],
        vec!["serve", "--port", "70000"],
        vec!["mcp", "--bogus"],
    ] {
        let (code, stdout, err) = run_adit(&args, short);
        assert_eq!(
            code,
            Some(2),
            "adit {args:?}: bad usage -> exit 2; stderr: {err}"
        );
        assert!(
            err.to_lowercase().contains("usage"),
            "adit {args:?}: usage on stderr, got {err:?}"
        );
        assert!(stdout.is_empty(), "adit {args:?}: nothing on stdout");
    }
    assert!(!out.exists());
}

#[test]
fn binary_help_prints_usage_on_stderr() {
    let (code, _, err) = run_adit(&["--help"], Duration::from_secs(20));
    // Reading: --help prints usage on stderr; exit code 0 or 2 both accepted (contract unclear).
    assert!(
        matches!(code, Some(0) | Some(2)),
        "--help exit code {code:?}"
    );
    for word in [
        "mcp",
        "serve",
        "render",
        "--port",
        "-o",
        "--sequence",
        "--codec",
    ] {
        assert!(err.contains(word), "usage mentions {word}: {err}");
    }
}
