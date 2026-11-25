//! T-EXPORT and T-JOB. Export encodes with the real ffmpeg (on PATH) even with the Pattern backend.
mod common;
use common::*;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

fn wait(s: &adit_mcp::Session, job: &Value, timeout_s: u64) -> Value {
    call(
        s,
        "job_wait",
        json!({ "job_id": job, "timeout_s": timeout_s }),
    )
    .json
}

#[test]
fn t_export_done_and_file_matches() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(&s, json!({}));
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("0.5s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!("0.5s"),
        json!("0.5s"),
        [0.0, 0.0, 1.0, 1.0],
    );
    let out = dir.path().join("out.mp4");
    let job = call(
        &s,
        "export",
        json!({ "project_id": p.id, "path": out.to_str().unwrap(), "crf": 10 }),
    )
    .json["job_id"]
        .clone();
    assert!(!job.is_null());
    let st = wait(&s, &job, 120);
    assert_eq!(st["state"], "done", "{st}");
    // X-1: frames n >= 0 with n / 30 s < 1 s -> 30 frames.
    assert_eq!(st["frames_done"], 30);
    assert_eq!(st["frames_total"], 30);
    assert!(same_path(st["path"].as_str().unwrap(), &out));
    // job_status after the end gives the same state.
    let st2 = call(&s, "job_status", json!({ "job_id": job })).json;
    assert_eq!(st2["state"], "done");
    let (n, w, h, fps) = probe_video(&out);
    assert_eq!((n, w, h, fps.as_str()), (30, 64, 48, "30/1"));
    // SPEC X-4: frame 5 red, frame 20 blue (±12).
    assert_rgb(
        rgb_at(&ffmpeg_frame(&out, 5, 64, 48), 64, 32, 24),
        [255, 0, 0],
        12,
        "frame 5",
    );
    assert_rgb(
        rgb_at(&ffmpeg_frame(&out, 20, 64, 48), 64, 32, 24),
        [0, 0, 255],
        12,
        "frame 20",
    );
}

#[test]
fn t_export_options_width_height_fps() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(&s, json!({}));
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    let out = dir.path().join("small.mp4");
    let job = call(
        &s,
        "export",
        json!({ "project_id": p.id, "path": out.to_str().unwrap(), "width": 32, "height": 24, "fps": "15", "audio": "none" }),
    )
    .json["job_id"]
        .clone();
    let st = wait(&s, &job, 120);
    assert_eq!(st["state"], "done", "{st}");
    // 1 s at 15 fps -> 15 frames.
    assert_eq!(st["frames_total"], 15);
    let (n, w, h, fps) = probe_video(&out);
    assert_eq!((n, w, h, fps.as_str()), (15, 32, 24, "15/1"));
}

#[test]
fn t_export_snapshot_ignores_later_edits() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(&s, json!({}));
    let red = add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    let out = dir.path().join("snap.mp4");
    let job = call(
        &s,
        "export",
        json!({ "project_id": p.id, "path": out.to_str().unwrap() }),
    )
    .json["job_id"]
        .clone();
    // Immediately replace the red 1 s clip by a blue 2 s one.
    edit(
        &s,
        &p.id,
        json!([{ "op": "remove_clips", "clip_ids": [red] },
               { "op": "add_clip", "track_id": p.v1(), "source": solid([0.0, 0.0, 1.0, 1.0]), "start": 0, "duration": "2s" }]),
    );
    let st = wait(&s, &job, 120);
    assert_eq!(st["state"], "done", "{st}");
    assert_eq!(st["frames_total"], 30, "snapshot: 1 s, not 2 s");
    let (n, _, _, _) = probe_video(&out);
    assert_eq!(n, 30);
    assert_rgb(
        rgb_at(&ffmpeg_frame(&out, 15, 64, 48), 64, 32, 24),
        [255, 0, 0],
        12,
        "snapshot is red",
    );
}

/// 10 min at 30 fps = 18_000 frames, no audio tracks: long enough to still be running after 1 s.
fn long_project(s: &adit_mcp::Session) -> Proj {
    let p = new_project(s, json!({ "audio_tracks": 0 }));
    add_solid(
        s,
        &p.id,
        p.v1(),
        json!(0),
        json!("600s"),
        [0.0, 1.0, 0.0, 1.0],
    );
    p
}

#[test]
fn t_job_wait_timeout_then_cancel() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let p = long_project(&s);
    let out = dir.path().join("long.mp4");
    let job = call(
        &s,
        "export",
        json!({ "project_id": p.id, "path": out.to_str().unwrap() }),
    )
    .json["job_id"]
        .clone();
    let t0 = Instant::now();
    let st = wait(&s, &job, 1);
    let el = t0.elapsed();
    assert!(
        el >= Duration::from_millis(900) && el < Duration::from_secs(5),
        "job_wait timeout 1 s took {el:?}"
    );
    assert_eq!(st["state"], "running", "{st}");
    let done = st["frames_done"].as_u64().unwrap();
    let total = st["frames_total"].as_u64().unwrap();
    assert!(done < 18_000 && done <= total);
    if done > 0 {
        assert_eq!(total, 18_000, "600 s * 30 fps");
    }
    let st = call(&s, "job_status", json!({ "job_id": job })).json;
    assert_eq!(st["state"], "running");

    call(&s, "job_cancel", json!({ "job_id": job }));
    let st = wait(&s, &job, 30);
    assert_eq!(st["state"], "cancelled", "{st}");
    assert!(st["frames_done"].as_u64().unwrap() < 18_000);
    // SPEC X-2: the partial file is removed.
    std::thread::sleep(Duration::from_millis(200));
    assert!(!out.exists(), "cancelled export leaves no file");
}

#[test]
fn t_job_failed_unwritable_path() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(&s, json!({}));
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    let out = dir.path().join("no_such_dir").join("deeper").join("x.mp4");
    // Reading: a path that cannot be written makes the job end `failed` with an error.
    // Rejecting it up front with a tool error is accepted too.
    match s.call(
        "export",
        json!({ "project_id": p.id, "path": out.to_str().unwrap() }),
    ) {
        Err(e) => assert!(!e.is_empty()),
        Ok(o) => {
            let st = wait(&s, &o.json["job_id"], 60);
            assert_eq!(st["state"], "failed", "{st}");
            assert!(!st["error"].as_str().expect("error text").is_empty());
        }
    }
    assert!(!out.exists());
}

#[test]
fn t_export_errors() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    call_err(&s, "job_status", json!({ "job_id": "no_such_job" }));
    call_err(
        &s,
        "job_wait",
        json!({ "job_id": "no_such_job", "timeout_s": 1 }),
    );
    let p = new_project(&s, json!({}));
    // Empty sequence (SPEC X-3): error now or a failed job, never a file.
    let out = dir.path().join("empty.mp4");
    match s.call(
        "export",
        json!({ "project_id": p.id, "path": out.to_str().unwrap() }),
    ) {
        Err(_) => {}
        Ok(o) => assert_eq!(wait(&s, &o.json["job_id"], 60)["state"], "failed"),
    }
    assert!(!out.exists());
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    call_err(
        &s,
        "export",
        json!({ "project_id": p.id, "path": dir.path().join("a.mp4").to_str().unwrap(), "codec": "nope" }),
    );
    call_err(&s, "export", json!({ "project_id": p.id }));
}

#[test]
fn t_export_two_jobs_independent() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(&s, json!({}));
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("0.5s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    let a = dir.path().join("a.mp4");
    let b = dir.path().join("b.gif");
    let ja = call(
        &s,
        "export",
        json!({ "project_id": p.id, "path": a.to_str().unwrap() }),
    )
    .json["job_id"]
        .clone();
    let jb = call(
        &s,
        "export",
        json!({ "project_id": p.id, "path": b.to_str().unwrap(), "codec": "gif", "audio": "none" }),
    )
    .json["job_id"]
        .clone();
    assert_ne!(ja, jb);
    assert_eq!(wait(&s, &ja, 120)["state"], "done");
    assert_eq!(wait(&s, &jb, 120)["state"], "done");
    assert_eq!(probe_video(&a).0, 15, "0.5 s at 30 fps");
    assert_eq!(&std::fs::read(&b).unwrap()[..4], b"GIF8");
}
