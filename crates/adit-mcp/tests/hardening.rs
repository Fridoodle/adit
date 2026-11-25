//! SPEC Part 5 hardening rules as seen through `Session::call`: H-1 (deep WGSL), H-4 (nesting
//! depth via `edit`), H-5 (`audio_levels` ranges), H-6 (forbidden `audio_filter`), H-9 (effect
//! references), H-11 (contact sheet size), H-12 (export over a media file), H-13 (export job
//! limit), H-14 (detection uses the session backend). Process-killing inputs are in
//! `hardening_crash.rs`.
mod common;
use adit_mcp::Session;
use adit_media::Ffmpeg;
use common::*;
use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
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

fn ffmpeg_session() -> Session {
    Session::new(Arc::new(Ffmpeg::new().expect("ffmpeg on PATH")))
}

/// `ffmpeg -hide_banner -loglevel error -y <args>`; panics on failure.
fn ffmpeg(args: &[&str]) {
    let out = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .args(args)
        .output()
        .expect("ffmpeg on PATH");
    assert!(
        out.status.success(),
        "fixture ffmpeg {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// 3 s 48 kHz mono wav: 0.5-amplitude 440 Hz tone, silent on [1 s, 2 s).
fn gap_wav(dir: &Path) -> String {
    let p = dir.join("gap.wav").to_str().unwrap().to_string();
    ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        "aevalsrc='if(between(t,1,2),0,0.5*sin(2*PI*440*t))':s=48000:d=3",
        "-c:a",
        "pcm_s16le",
        &p,
    ]);
    p
}

/// 2 s 64x64 30 fps h264: red for 1 s, then blue.
fn cut_mp4(dir: &Path) -> String {
    let p = dir.join("cut.mp4").to_str().unwrap().to_string();
    ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        "color=c=red:s=64x64:r=30:d=1",
        "-f",
        "lavfi",
        "-i",
        "color=c=blue:s=64x64:r=30:d=1",
        "-filter_complex",
        "[0:v][1:v]concat=n=2:v=1:a=0",
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        &p,
    ]);
    p
}

fn import(s: &Session, pid: &str, path: &str) -> u64 {
    call(
        s,
        "media_import",
        json!({ "project_id": pid, "paths": [path] }),
    )
    .json["media"][0]["id"]
        .as_u64()
        .expect("media id")
}

/// Module-scope constant nested `depth` parentheses deep.
fn parens(depth: usize) -> String {
    format!(
        "const K: f32 = {}1.0{};\nfn effect(uv: vec2f) -> vec4f {{ return src(uv) * K; }}\n",
        "(".repeat(depth),
        ")".repeat(depth)
    )
}

// ------------------------------------------------------------------------------------- H-1

#[test]
fn h_1_effect_check_too_deep_is_not_ok_and_session_answers() {
    let s = session();
    let r = call(
        &s,
        "effect_check",
        json!({ "kind": "filter", "wgsl": parens(65), "params": [] }),
    )
    .json;
    assert_eq!(r["ok"], false, "H-1: depth 65 must fail: {r}");
    assert!(r["error"].as_str().is_some_and(|e| !e.is_empty()));
    // Still answering, and depth 64 is fine.
    let r = call(
        &s,
        "effect_check",
        json!({ "kind": "filter", "wgsl": parens(64), "params": [] }),
    )
    .json;
    assert_eq!(r["ok"], true, "{r}");
}

#[test]
fn h_1_define_effect_too_deep_is_error_and_changes_nothing() {
    let s = session();
    let p = new_project(&s, json!({}));
    let before = full(&s, &p.id);
    let def = json!({ "name": "deep", "kind": "filter", "wgsl": parens(65), "params": [] });
    edit_err(&s, &p.id, json!([{ "op": "define_effect", "def": def }]));
    assert_eq!(full(&s, &p.id), before);
}

// ------------------------------------------------------------------------------------- H-4

fn add_seq(name: &str) -> Value {
    json!({ "op": "add_sequence", "name": name, "width": 64, "height": 48,
            "fps": { "num": 30, "den": 1 }, "sample_rate": 48000, "video_tracks": 1 })
}

fn nest(track: u64, seq: u64) -> Value {
    json!({ "op": "add_clip", "track_id": track,
            "source": { "type": "sequence", "sequence_id": seq },
            "start": 0, "duration": "1s" })
}

#[test]
fn h_4_edit_nesting_depth_16_ok_17_rejected() {
    let s = session();
    let p = new_project(&s, json!({}));
    // 15 more sequences, each with one video track: created = [[seq, track], ...].
    let made = edit(
        &s,
        &p.id,
        Value::Array((1..=15).map(|i| add_seq(&format!("S{i}"))).collect()),
    );
    let ids: Vec<(u64, u64)> = made["created"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c[0].as_u64().unwrap(), c[1].as_u64().unwrap()))
        .collect();
    assert_eq!(ids.len(), 15);
    // main nests S1, S1 nests S2, ..., S14 nests S15: 16 sequences deep.
    let mut cmds = vec![nest(p.v1(), ids[0].0)];
    for k in 0..14 {
        cmds.push(nest(ids[k].1, ids[k + 1].0));
    }
    edit(&s, &p.id, Value::Array(cmds));
    // One more level: S15 nests a new S16 -> depth 17.
    let more = edit(&s, &p.id, json!([add_seq("S16")]));
    let s16 = more["created"][0][0].as_u64().unwrap();
    let before = full(&s, &p.id);
    edit_err(&s, &p.id, json!([nest(ids[14].1, s16)]));
    assert_eq!(full(&s, &p.id), before, "G1: nothing changed");
}

// ------------------------------------------------------------------------------------- H-5

#[test]
fn h_5_audio_levels_start_not_before_end_is_error() {
    let s = session();
    let p = new_project(&s, json!({}));
    add_solid(&s, &p.id, p.v1(), json!(0), json!("2s"), [1.0; 4]);
    for (a, b) in [("1s", "1s"), ("2s", "1s")] {
        call_err(
            &s,
            "audio_levels",
            json!({ "project_id": p.id, "start": a, "end": b }),
        );
    }
}

// ------------------------------------------------------------------------------------- H-6

/// `ametadata` filter writing to `file`, escaped for an ffmpeg filter option.
fn ametadata_to(file: &Path) -> String {
    let p = file
        .to_str()
        .unwrap()
        .replace('\\', "/")
        .replace(':', "\\:");
    format!("ametadata=mode=print:file='{p}'")
}

#[test]
fn h_6_forbidden_audio_filter_creates_no_file_and_levels_work() {
    timed(120, || {
        let dir = tempfile::tempdir().unwrap();
        let wav = gap_wav(dir.path());
        let s = ffmpeg_session();
        let p = new_project(&s, json!({}));
        let m = import(&s, &p.id, &wav);
        let a1 = p.tracks[2];
        let clip = edit(
            &s,
            &p.id,
            json!([{ "op": "add_clip", "track_id": a1, "source": media_src(m),
                     "start": 0, "duration": "1s" }]),
        )["created"][0][0]
            .as_u64()
            .unwrap();
        let leak = dir.path().join("leak.txt");
        // The edit layer may refuse the filter or accept it; the media layer must refuse it.
        let _ = s.call(
            "edit",
            json!({ "project_id": p.id, "commands": [
                { "op": "set_clip", "clip_id": clip, "audio_filter": ametadata_to(&leak) }] }),
        );
        let r = call(
            &s,
            "audio_levels",
            json!({ "project_id": p.id, "buckets": 10 }),
        )
        .json;
        // A-7: a clip whose audio cannot be fetched is silence; levels still come back.
        assert_eq!(r["rms_db"].as_array().map(Vec::len), Some(10), "{r}");
        assert!(!leak.exists(), "H-6: {leak:?} must not be created");
    })
}

// ------------------------------------------------------------------------------------- H-9

/// Project with a solid on V1 carrying an instance of the project-defined filter `swap`.
fn with_swap(s: &Session) -> (Proj, u64) {
    let p = new_project(s, json!({}));
    let c = add_solid(
        s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.5, 0.0, 1.0],
    );
    edit(
        s,
        &p.id,
        json!([{ "op": "define_effect", "def": swap_def("swap") },
               { "op": "add_effect", "clip_id": c, "effect": "swap" }]),
    );
    (p, c)
}

#[test]
fn h_9_remove_effect_def_in_use_is_error() {
    let s = session();
    let (p, _) = with_swap(&s);
    let before = full(&s, &p.id);
    edit_err(
        &s,
        &p.id,
        json!([{ "op": "remove_effect_def", "name": "swap" }]),
    );
    assert_eq!(full(&s, &p.id), before, "project and undo depth unchanged");
}

#[test]
fn h_9_define_effect_changing_kind_in_use_is_error() {
    let s = session();
    let (p, _) = with_swap(&s);
    let before = full(&s, &p.id);
    let def = json!({ "name": "swap", "kind": "transition",
        "wgsl": "fn transition(uv: vec2f) -> vec4f { return mix(src_a(uv), src_b(uv), ctx.progress); }",
        "params": [] });
    edit_err(&s, &p.id, json!([{ "op": "define_effect", "def": def }]));
    assert_eq!(full(&s, &p.id), before, "project and undo depth unchanged");
}

#[test]
fn h_9_remove_unused_effect_def_ok() {
    let s = session();
    let (p, _) = with_swap(&s);
    edit(
        &s,
        &p.id,
        json!([{ "op": "define_effect", "def": swap_def("unused") }]),
    );
    edit(
        &s,
        &p.id,
        json!([{ "op": "remove_effect_def", "name": "unused" }]),
    );
    let names: Vec<Value> = project(&s, &p.id)["effects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["name"].clone())
        .collect();
    assert_eq!(names, vec![json!("swap")]);
}

#[test]
fn h_9_remove_def_shadowing_builtin_of_same_kind_ok() {
    let s = session();
    let p = new_project(&s, json!({}));
    let c = add_solid(&s, &p.id, p.v1(), json!(0), json!("1s"), [1.0; 4]);
    // A project filter named like the built-in filter `invert` (E-3: it shadows it).
    edit(
        &s,
        &p.id,
        json!([{ "op": "define_effect", "def": swap_def("invert") },
               { "op": "add_effect", "clip_id": c, "effect": "invert" }]),
    );
    // Removing it leaves the clip on the built-in `invert`, still a filter: allowed.
    edit(
        &s,
        &p.id,
        json!([{ "op": "remove_effect_def", "name": "invert" }]),
    );
    let pj = project(&s, &p.id);
    assert_eq!(pj["effects"], json!([]));
    assert_eq!(find_clip(&pj, c)["effects"][0]["effect"], "invert");
}

// ------------------------------------------------------------------------------------- H-11

#[test]
fn h_11_small_contact_sheet_still_works() {
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
    let o = call(
        &s,
        "render_contact_sheet",
        json!({ "project_id": p.id, "count": 4, "cell_size": 64 }),
    );
    // 64x48 cells, 3 columns, 2 rows: 192 x 96.
    let img = decode(&o.images[0].bytes);
    assert_eq!(img.dimensions(), (192, 96));
}

// ------------------------------------------------------------------------------------- H-12

#[test]
fn h_12_export_onto_a_media_file_is_error() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(&s, json!({}));
    let m = import_pattern(&s, &p.id, dir.path(), "source.mp4");
    edit(
        &s,
        &p.id,
        json!([{ "op": "add_clip", "track_id": p.v1(), "source": media_src(m),
                 "start": 0, "duration": "1s" }]),
    );
    let src = dir.path().join("source.mp4");
    let before = std::fs::read(&src).unwrap();
    let r = s.call(
        "export",
        json!({ "project_id": p.id, "path": src.to_str().unwrap() }),
    );
    if let Ok(o) = &r {
        // Let a wrongly started job end before looking at the file.
        let _ = s.call(
            "job_wait",
            json!({ "job_id": o.json["job_id"], "timeout_s": 60 }),
        );
    }
    assert!(r.is_err(), "H-12: export onto media file must be an error");
    assert_eq!(std::fs::read(&src).unwrap(), before, "media file untouched");
}

// ------------------------------------------------------------------------------------- H-13

/// Assumption: "at once" = jobs still `running` in this session (all projects together).
#[test]
fn h_13_fifth_concurrent_export_is_error() {
    timed(300, || {
        let s = session();
        let dir = tempfile::tempdir().unwrap();
        let p = new_project(&s, json!({}));
        // 1 hour at 30 fps: far longer than the test runs. No audio: nothing mixed in memory.
        add_solid(&s, &p.id, p.v1(), json!(0), json!("3600s"), [1.0; 4]);
        let start = |k: usize| {
            s.call(
                "export",
                json!({ "project_id": p.id, "audio": "none",
                        "path": dir.path().join(format!("o{k}.mp4")).to_str().unwrap() }),
            )
        };
        let mut jobs = Vec::new();
        for k in 0..4 {
            jobs.push(
                start(k).unwrap_or_else(|e| panic!("export {k}: {e}")).json["job_id"].clone(),
            );
        }
        let fifth = start(4);
        if let Ok(o) = &fifth {
            jobs.push(o.json["job_id"].clone());
        }
        // Clean up before asserting, so a failure does not leave encoders running.
        let stop = |jobs: &[Value]| {
            for j in jobs {
                call(&s, "job_cancel", json!({ "job_id": j }));
            }
            for j in jobs {
                let st = call(&s, "job_wait", json!({ "job_id": j, "timeout_s": 60 })).json;
                assert_eq!(st["state"], "cancelled", "{st}");
            }
        };
        stop(&jobs);
        assert!(
            fifth.is_err(),
            "H-13: a fifth running export must be refused"
        );
        // With the four ended, a new export starts again.
        let again = start(5).expect("export after the others ended");
        stop(&[again.json["job_id"].clone()]);
    })
}

// ------------------------------------------------------------------------------------- H-14

#[test]
fn h_14_detect_with_pattern_session_is_error() {
    timed(120, || {
        let dir = tempfile::tempdir().unwrap();
        // Real files, so an ffmpeg fallback would succeed: only the session backend decides.
        let wav = gap_wav(dir.path());
        let mp4 = cut_mp4(dir.path());
        let s = session();
        let p = new_project(&s, json!({}));
        let a = import(&s, &p.id, &wav);
        let v = import(&s, &p.id, &mp4);
        call_err(
            &s,
            "detect_silence",
            json!({ "project_id": p.id, "media_id": a, "noise_db": -30, "min_duration": "0.5s" }),
        );
        call_err(
            &s,
            "detect_scenes",
            json!({ "project_id": p.id, "media_id": v }),
        );
    })
}

#[test]
fn h_14_detect_with_ffmpeg_session_works() {
    timed(120, || {
        let dir = tempfile::tempdir().unwrap();
        let wav = gap_wav(dir.path());
        let mp4 = cut_mp4(dir.path());
        let s = ffmpeg_session();
        let p = new_project(&s, json!({}));
        let a = import(&s, &p.id, &wav);
        let v = import(&s, &p.id, &mp4);
        let r = call(
            &s,
            "detect_silence",
            json!({ "project_id": p.id, "media_id": a, "noise_db": -30, "min_duration": "0.5s" }),
        )
        .json;
        let ranges = r["ranges"].as_array().unwrap();
        assert_eq!(ranges.len(), 1, "{r}");
        approx(f(&ranges[0]["start_s"]), 1.0, 0.1, "silence start");
        approx(f(&ranges[0]["end_s"]), 2.0, 0.1, "silence end");
        let r = call(
            &s,
            "detect_scenes",
            json!({ "project_id": p.id, "media_id": v }),
        )
        .json;
        let cuts = r["cuts_s"].as_array().unwrap();
        assert_eq!(cuts.len(), 1, "{r}");
        approx(f(&cuts[0]), 1.0, 1.0 / 30.0, "cut");
    })
}
