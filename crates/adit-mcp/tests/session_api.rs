//! In-process `Session` API for the GUI: S-1, S-2, S-3 (docs/GUI.md "Session additions").
//! Expected values are worked out by hand from docs/GUI.md, docs/MCP_TOOLS.md and SPEC M-PAT / A-3.
mod common;
use adit_core::{Id, MediaSpec, Rational, Time};
use adit_mcp::{Image, Session, UiBridge};
use adit_media::{Frame, MediaBackend, MediaError, Pattern};
use common::*;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{SyncSender, sync_channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn pattern() -> Pattern {
    Pattern {
        width: 64,
        height: 64,
        fps: Rational { num: 30, den: 1 },
        duration: Some(10 * S),
    }
}

fn rev(s: &Session, pid: &str) -> u64 {
    s.revision(pid)
        .unwrap_or_else(|| panic!("revision({pid}) is None"))
}

fn snap(s: &Session, pid: &str) -> (adit_core::Project, u64) {
    s.snapshot(pid)
        .unwrap_or_else(|| panic!("snapshot({pid}) is None"))
}

fn clip_count(p: &adit_core::Project) -> usize {
    p.sequences
        .iter()
        .flat_map(|q| q.tracks.iter())
        .map(|t| t.clips.len())
        .sum()
}

// ---------------------------------------------------------------- S-1

#[test]
fn s1_revision_grows_on_edit_undo_redo_import() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(&s, json!({}));
    let r0 = rev(&s, &p.id);

    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    let r1 = rev(&s, &p.id);
    assert!(r1 > r0, "S-1: edit grows revision ({r0} -> {r1})");

    call(&s, "undo", json!({ "project_id": p.id }));
    let r2 = rev(&s, &p.id);
    assert!(r2 > r1, "S-1: undo grows revision ({r1} -> {r2})");

    call(&s, "redo", json!({ "project_id": p.id }));
    let r3 = rev(&s, &p.id);
    assert!(r3 > r2, "S-1: redo grows revision ({r2} -> {r3})");

    import_pattern(&s, &p.id, dir.path(), "a.mov");
    let r4 = rev(&s, &p.id);
    assert!(r4 > r3, "S-1: import grows revision ({r3} -> {r4})");
}

#[test]
fn s1_revision_unchanged_on_reads_renders_and_failed_edits() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(&s, json!({}));
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("2s"),
        [0.0, 1.0, 0.0, 1.0],
    );
    let r = rev(&s, &p.id);

    full(&s, &p.id);
    summary(&s, &p.id);
    call(&s, "project_list", json!({}));
    call(&s, "effect_list", json!({ "project_id": p.id }));
    frame_png(&s, &p.id, json!("15f"));
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
    assert_eq!(rev(&s, &p.id), r, "S-1: reads and renders keep revision");

    // Failed edit: unknown track (SPEC G1: nothing changes).
    edit_err(
        &s,
        &p.id,
        json!([{ "op": "add_clip", "track_id": 999_999, "source": solid([1.0, 1.0, 1.0, 1.0]),
                 "start": 0, "duration": "1s" }]),
    );
    // Failed import: one bad file fails the whole call (T-IMPORT); Pattern probes anything, so
    // use an invalid argument instead.
    call_err(&s, "media_import", json!({ "project_id": p.id }));
    assert_eq!(rev(&s, &p.id), r, "S-1: failed edits keep revision");

    // The in-process accessors are reads too.
    let (proj, _) = snap(&s, &p.id);
    s.render(&proj, p.seq, 0).expect("render");
    s.mix(&proj, p.seq, 0, S).expect("mix");
    s.source_frame(dir.path().join("x.mov").to_str().unwrap(), 0)
        .expect("source_frame");
    assert_eq!(
        rev(&s, &p.id),
        r,
        "S-1: render/mix/source_frame keep revision"
    );
}

/// Reading of T-3: an edit to one project is not a change to another.
#[test]
fn s1_revision_is_per_project() {
    let s = session();
    let a = new_project(&s, json!({ "name": "a" }));
    let b = new_project(&s, json!({ "name": "b" }));
    let rb = rev(&s, &b.id);
    add_solid(
        &s,
        &a.id,
        a.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    assert_eq!(rev(&s, &b.id), rb, "S-1: edit to a leaves b's revision");
}

#[test]
fn s1_projects_order_and_fields() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    assert!(
        s.projects().is_empty(),
        "S-1: fresh session has no projects"
    );
    let a = new_project(&s, json!({ "name": "Alpha" }));
    let b = new_project(&s, json!({ "name": "Beta" }));
    let c = new_project(&s, json!({ "name": "Gamma" }));

    let ps = s.projects();
    let ids: Vec<&str> = ps.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(
        ids,
        [a.id.as_str(), b.id.as_str(), c.id.as_str()],
        "opening order"
    );
    let names: Vec<&str> = ps.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Alpha", "Beta", "Gamma"]);
    for p in &ps {
        assert_eq!(p.path, None, "{}: never saved -> no path", p.id);
        assert!(p.dirty, "{}: T-7 new never-saved project is dirty", p.id);
        assert_eq!(
            Some(p.revision),
            s.revision(&p.id),
            "{}: revision field",
            p.id
        );
    }

    // Save b: path remembered, not dirty.
    let path = dir.path().join("beta.adit");
    call(
        &s,
        "project_save",
        json!({ "project_id": b.id, "path": path.to_str().unwrap() }),
    );
    let ps = s.projects();
    let pb = ps.iter().find(|p| p.id == b.id).expect("b listed");
    assert!(
        same_path(pb.path.as_deref().expect("saved path"), &path),
        "path {:?}",
        pb.path
    );
    assert!(!pb.dirty, "saved -> not dirty");

    // Close b: the rest keep their order. Opening another appends it.
    call(&s, "project_close", json!({ "project_id": b.id }));
    let o = call(
        &s,
        "project_open",
        json!({ "path": path.to_str().unwrap() }),
    )
    .json;
    let reopened = o["project_id"].as_str().unwrap().to_string();
    let ids: Vec<String> = s.projects().into_iter().map(|p| p.id).collect();
    assert_eq!(ids, [a.id.clone(), c.id.clone(), reopened.clone()]);
    assert_ne!(reopened, b.id, "T-3: ids never reused");
    assert_eq!(s.revision(&b.id), None, "closed project has no revision");
}

#[test]
fn s1_unknown_id_is_none() {
    let s = session();
    new_project(&s, json!({}));
    assert_eq!(s.revision("p999"), None);
    assert!(s.snapshot("p999").is_none());
    assert_eq!(s.revision(""), None);
}

#[test]
fn s1_snapshot_matches_project_get_and_revision() {
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
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!("1s"),
        json!("1s"),
        [0.0, 0.0, 1.0, 1.0],
    );
    let (proj, r) = snap(&s, &p.id);
    assert_eq!(
        Some(r),
        s.revision(&p.id),
        "quiet session: snapshot revision = revision()"
    );
    assert_eq!(
        serde_json::to_value(&proj).unwrap(),
        project(&s, &p.id),
        "snapshot = project_get full"
    );
    assert_eq!(clip_count(&proj), 2);

    // A snapshot is a copy: later edits do not change it.
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!("2s"),
        json!("1s"),
        [0.0, 1.0, 0.0, 1.0],
    );
    assert_eq!(clip_count(&proj), 2);
    let (proj2, r2) = snap(&s, &p.id);
    assert_eq!(clip_count(&proj2), 3);
    assert!(r2 > r);
}

/// "taken together": while another thread edits, every snapshot's revision maps to exactly one
/// project state, and more revision never means fewer clips (each edit adds one clip).
#[test]
fn s1_snapshot_project_and_revision_are_consistent_under_concurrent_edits() {
    let s = session();
    let p = new_project(&s, json!({}));
    let (s2, pid, v1) = (s.clone(), p.id.clone(), p.v1());
    let writer = std::thread::spawn(move || {
        for i in 0..40i64 {
            add_solid(
                &s2,
                &pid,
                v1,
                json!(i * F30),
                json!(F30),
                [1.0, 1.0, 1.0, 1.0],
            );
        }
    });
    let mut seen: Vec<(u64, usize)> = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(60);
    while !writer.is_finished() {
        assert!(
            Instant::now() < deadline,
            "writer did not finish within 60 s"
        );
        let (proj, r) = snap(&s, &p.id);
        seen.push((r, clip_count(&proj)));
    }
    writer.join().unwrap();
    let (proj, r) = snap(&s, &p.id);
    seen.push((r, clip_count(&proj)));
    assert_eq!(clip_count(&proj), 40);
    seen.sort();
    for w in seen.windows(2) {
        let ((ra, ca), (rb, cb)) = (w[0], w[1]);
        if ra == rb {
            assert_eq!(ca, cb, "S-1: revision {ra} seen with {ca} and {cb} clips");
        } else {
            assert!(
                ca <= cb,
                "S-1: revision {ra} had {ca} clips, later {rb} had {cb}"
            );
        }
    }
}

// ---------------------------------------------------------------- S-2

/// Opaque red background on V1, Pattern media on V2 over it, so every pixel is opaque.
fn pattern_over_red(s: &Session, dir: &std::path::Path) -> Proj {
    let p = new_project(s, json!({}));
    let m = import_pattern(s, &p.id, dir, "pat.mov");
    edit(
        s,
        &p.id,
        json!([
            { "op": "add_clip", "track_id": p.tracks[0], "source": solid([1.0, 0.0, 0.0, 1.0]),
              "start": 0, "duration": "2s" },
            { "op": "add_clip", "track_id": p.tracks[1], "source": media_src(m),
              "start": 0, "duration": "1s" }
        ]),
    );
    p
}

fn assert_same_pixels(frame: &Frame, png: &image::RgbaImage, what: &str) {
    assert_eq!(
        (frame.width, frame.height),
        (png.width(), png.height()),
        "{what}: size"
    );
    assert_eq!(frame.rgba.len(), (frame.width * frame.height * 4) as usize);
    for (i, (a, b)) in frame.rgba.chunks(4).zip(png.pixels()).enumerate() {
        assert_eq!(
            a,
            &b.0[..],
            "{what}: pixel {} differs (render {a:?}, render_frame {:?})",
            i,
            b.0
        );
    }
}

#[test]
fn s2_render_equals_render_frame_png() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let p = pattern_over_red(&s, dir.path());
    let (proj, _) = snap(&s, &p.id);
    // 0, 15f (inside the Pattern clip) and 1.5 s (red only).
    for (t, arg) in [
        (0, json!(0)),
        (15 * F30, json!("15f")),
        (S + S / 2, json!("1.5s")),
    ] {
        let fr = s.render(&proj, p.seq, t).expect("render");
        assert_eq!((fr.width, fr.height), (64, 48), "S-2: sequence size");
        let (png, _) = frame_png(&s, &p.id, arg.clone());
        assert_same_pixels(&fr, &png, &format!("S-2 t={arg}"));
    }
    // Red only at 1.5 s: (255, 0, 0, 255) everywhere.
    let fr = s.render(&proj, p.seq, S + S / 2).unwrap();
    for px in fr.rgba.chunks(4) {
        assert_eq!(px, [255, 0, 0, 255], "S-2: opaque red solid");
    }
}

#[test]
fn s2_render_works_on_the_snapshot_passed_in() {
    let s = session();
    let p = new_project(&s, json!({}));
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [0.0, 0.0, 1.0, 1.0],
    );
    let (old, _) = snap(&s, &p.id);
    // Later edit turns the live project green; the old snapshot still renders blue.
    edit(
        &s,
        &p.id,
        json!([{ "op": "add_clip", "track_id": p.tracks[1], "source": solid([0.0, 1.0, 0.0, 1.0]),
                 "start": 0, "duration": "1s" }]),
    );
    let fr = s.render(&old, p.seq, 0).unwrap();
    assert_eq!(
        &fr.rgba[0..4],
        [0, 0, 255, 255],
        "S-2: renders the snapshot, not live state"
    );
    let (png, _) = frame_png(&s, &p.id, json!(0));
    assert_eq!(
        png.get_pixel(0, 0).0,
        [0, 255, 0, 255],
        "live state is green"
    );
}

#[test]
fn s2_render_and_mix_unknown_sequence_err() {
    let s = session();
    let p = new_project(&s, json!({}));
    let (proj, _) = snap(&s, &p.id);
    let bad: Id = 999_999;
    let e = s
        .render(&proj, bad, 0)
        .expect_err("S-2: render of a missing sequence");
    assert!(!e.trim().is_empty());
    let e = s
        .mix(&proj, bad, 0, S)
        .expect_err("SPEC A-7: mix fails for a missing sequence");
    assert!(!e.trim().is_empty());
}

#[test]
fn s2_mix_pattern_audio_a3() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(&s, json!({}));
    let m = import_pattern(&s, &p.id, dir.path(), "tone.mov");
    let a1 = p.tracks[2];
    let r = edit(
        &s,
        &p.id,
        json!([
            { "op": "add_clip", "track_id": a1, "source": media_src(m), "start": 0, "duration": "1s" },
            { "op": "add_clip", "track_id": a1, "source": media_src(m), "start": "1s", "duration": "1s",
              "source_in": "1s" },
            { "op": "add_clip", "track_id": a1, "source": media_src(m), "start": "2s", "duration": "1s",
              "source_in": "2s" }
        ]),
    );
    let quiet = r["created"][1][0].as_u64().unwrap();
    let right = r["created"][2][0].as_u64().unwrap();
    edit(
        &s,
        &p.id,
        json!([
            { "op": "set_param", "target": { "clip_id": quiet, "name": "volume" }, "value": -6.0206 },
            { "op": "set_param", "target": { "clip_id": right, "name": "pan" }, "value": 1.0 }
        ]),
    );
    let (proj, _) = snap(&s, &p.id);
    let out = s.mix(&proj, p.seq, 0, 3 * S).expect("mix");
    // A-1: 2 * round(3 s * 48000) samples.
    assert_eq!(out.len(), 2 * 144_000, "S-2 mix length");
    // Middle of each second, away from clip edges. Pattern audio L 0.5, R 0.25 (M-PAT).
    let at = |sec: f64| {
        let i = (sec * 48_000.0) as usize;
        (out[2 * i] as f64, out[2 * i + 1] as f64)
    };
    let (l, rr) = at(0.5);
    approx(l, 0.5, 1e-4, "unity L");
    approx(rr, 0.25, 1e-4, "unity R");
    // A-3: -6.0206 dB -> gain 0.5.
    let (l, rr) = at(1.5);
    approx(l, 0.25, 1e-4, "-6 dB L");
    approx(rr, 0.125, 1e-4, "-6 dB R");
    // A-3: pan 1 -> left gain min(1, 0) = 0, right gain min(1, 2) = 1.
    let (l, rr) = at(2.5);
    approx(l, 0.0, 1e-4, "pan 1 L");
    approx(rr, 0.25, 1e-4, "pan 1 R");

    // "returns what adit_render::mix_audio returns".
    let want = adit_render::mix_audio(&proj, p.seq, 0, 3 * S, &pattern()).expect("mix_audio");
    assert_eq!(out, want, "S-2: mix = mix_audio");
    // Offset start: 1.25 s for 0.5 s -> 2 * 24000 samples.
    let out = s.mix(&proj, p.seq, S + S / 4, S / 2).unwrap();
    let want = adit_render::mix_audio(&proj, p.seq, S + S / 4, S / 2, &pattern()).unwrap();
    assert_eq!(out.len(), 48_000);
    assert_eq!(out, want);
}

#[test]
fn s2_source_frame_is_pattern_m_pat() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("src.mov");
    std::fs::write(&path, b"x").unwrap();
    let path = path.to_str().unwrap();
    for (t, n) in [(0, 0u8), (15 * F30, 15), (-S, 0), (F30 * 3 / 2, 1)] {
        let fr = s.source_frame(path, t).expect("source_frame");
        assert_eq!((fr.width, fr.height), (64, 64), "M-PAT size");
        for (x, y) in [(0u32, 0u32), (5, 9), (63, 63), (31, 2)] {
            let i = ((y * 64 + x) * 4) as usize;
            assert_eq!(
                &fr.rgba[i..i + 4],
                [x as u8, y as u8, n, 255],
                "M-PAT t={t} ({x},{y})"
            );
        }
        let want = pattern().video_frame(path, t).unwrap();
        assert_eq!(*fr, *want, "S-2: source_frame = MediaBackend::video_frame");
    }
}

/// Pattern with a switchable delay in `video_frame` and `audio`, to hold a render open.
struct SlowMedia {
    inner: Pattern,
    slow: AtomicBool,
    entered: Mutex<Option<SyncSender<()>>>,
}

const HOLD: Duration = Duration::from_millis(2500);

impl SlowMedia {
    fn hold(&self) {
        if self.slow.load(Ordering::SeqCst) {
            if let Some(tx) = self.entered.lock().unwrap().as_ref() {
                let _ = tx.try_send(());
            }
            std::thread::sleep(HOLD);
        }
    }
}

impl MediaBackend for SlowMedia {
    fn probe(&self, path: &str) -> Result<MediaSpec, MediaError> {
        self.inner.probe(path)
    }
    fn video_frame(&self, path: &str, time: Time) -> Result<Arc<Frame>, MediaError> {
        self.hold();
        self.inner.video_frame(path, time)
    }
    fn audio(
        &self,
        path: &str,
        start: Time,
        duration: Time,
        sample_rate: u32,
        filter: &str,
    ) -> Result<Vec<f32>, MediaError> {
        self.hold();
        self.inner.audio(path, start, duration, sample_rate, filter)
    }
}

/// S-2: render / mix / source_frame do not lock the session state: an `edit` issued while one of
/// them is stuck inside the media backend finishes long before the backend returns.
#[test]
fn s2_render_mix_source_do_not_block_edits() {
    let media = Arc::new(SlowMedia {
        inner: pattern(),
        slow: AtomicBool::new(false),
        entered: Mutex::new(None),
    });
    let s = Session::new(media.clone());
    let dir = tempfile::tempdir().unwrap();
    let p = new_project(&s, json!({}));
    let m = import_pattern(&s, &p.id, dir.path(), "slow.mov");
    edit(
        &s,
        &p.id,
        json!([{ "op": "add_clip", "track_id": p.tracks[0], "linked_track_id": p.tracks[2],
                 "source": media_src(m), "start": 0, "duration": "1s" }]),
    );
    let (proj, _) = snap(&s, &p.id);
    let src_path = dir.path().join("slow.mov").to_str().unwrap().to_string();

    for which in ["render", "mix", "source_frame"] {
        let (tx, rx) = sync_channel(1);
        *media.entered.lock().unwrap() = Some(tx);
        media.slow.store(true, Ordering::SeqCst);
        let (s2, pr, seq, path) = (s.clone(), proj.clone(), p.seq, src_path.clone());
        let (done_tx, done_rx) = sync_channel::<Result<(), String>>(1);
        let worker = std::thread::spawn(move || {
            let r = match which {
                "render" => s2.render(&pr, seq, 0).map(|_| ()),
                "mix" => s2.mix(&pr, seq, 0, S / 2).map(|_| ()),
                _ => s2.source_frame(&path, 0).map(|_| ()),
            };
            let _ = done_tx.send(r);
        });
        rx.recv_timeout(Duration::from_secs(30))
            .unwrap_or_else(|_| panic!("{which}: never reached the media backend"));
        media.slow.store(false, Ordering::SeqCst);

        let t0 = Instant::now();
        let (s3, pid, v2) = (s.clone(), p.id.clone(), p.tracks[1]);
        let (etx, erx) = sync_channel(1);
        std::thread::spawn(move || {
            let r = s3.call(
                "edit",
                json!({ "project_id": pid, "commands": [{ "op": "add_clip", "track_id": v2,
                    "source": solid([1.0, 1.0, 1.0, 1.0]), "start": "5s", "duration": "1f" }] }),
            );
            let _ = etx.send(r.map(|_| ()));
        });
        let edit_result = erx
            .recv_timeout(Duration::from_secs(30))
            .unwrap_or_else(|_| panic!("{which}: edit did not finish within 30 s"));
        let took = t0.elapsed();
        edit_result.unwrap_or_else(|e| panic!("{which}: edit failed: {e}"));
        assert!(
            took < Duration::from_millis(1000),
            "S-2: edit took {took:?} while {which} was in the backend (held {HOLD:?})"
        );
        assert!(
            done_rx.try_recv().is_err(),
            "{which} finished before the edit; the test did not overlap them"
        );
        done_rx
            .recv_timeout(Duration::from_secs(60))
            .unwrap_or_else(|_| panic!("{which}: did not finish within 60 s"))
            .unwrap_or_else(|e| panic!("{which} failed: {e}"));
        worker.join().unwrap();
    }
}

// ---------------------------------------------------------------- S-3

#[test]
fn s3_ui_tools_are_listed() {
    let tools = Session::tools();
    for name in ["ui_screenshot", "ui_get", "ui_set"] {
        let t = tools
            .iter()
            .find(|t| t.name == name)
            .unwrap_or_else(|| panic!("S-3: {name} listed by tools()"));
        assert!(t.input_schema.is_object(), "{name}: input schema");
        assert!(!t.description.trim().is_empty(), "{name}: description");
    }
}

#[test]
fn s3_without_window_error_result() {
    let s = session();
    new_project(&s, json!({}));
    for (tool, args) in [
        ("ui_screenshot", json!({})),
        ("ui_get", json!({})),
        ("ui_set", json!({ "playhead": 0 })),
    ] {
        let e = call_err(&s, tool, args);
        assert!(
            e.to_lowercase().contains("window"),
            "S-3: {tool} without a window says so, got {e:?}"
        );
    }
}

fn png_bytes() -> Vec<u8> {
    let img = image::RgbaImage::from_pixel(3, 2, image::Rgba([10, 20, 30, 255]));
    let mut out = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .unwrap();
    out
}

/// Records what the session hands to `set`; `fail` makes every method fail with that message.
struct FakeBridge {
    state: Value,
    png: Vec<u8>,
    sets: Mutex<Vec<Value>>,
    fail: Option<String>,
}

impl UiBridge for FakeBridge {
    fn screenshot(&self) -> Result<Image, String> {
        if let Some(e) = &self.fail {
            return Err(e.clone());
        }
        Ok(Image {
            mime: "image/png".into(),
            bytes: self.png.clone(),
        })
    }
    fn get(&self) -> Result<Value, String> {
        match &self.fail {
            Some(e) => Err(e.clone()),
            None => Ok(self.state.clone()),
        }
    }
    fn set(&self, state: &Value) -> Result<(), String> {
        if let Some(e) = &self.fail {
            return Err(e.clone());
        }
        self.sets.lock().unwrap().push(state.clone());
        Ok(())
    }
}

/// A 25 fps project so frame strings in `ui_set` are distinguishable from the 30 fps default.
fn bridge_setup() -> (Session, Proj, Arc<FakeBridge>) {
    let s = session();
    // Another 30 fps project first, so "first project" and "window's project" differ.
    new_project(&s, json!({ "name": "other" }));
    let p = new_project(&s, json!({ "name": "win", "fps": "25" }));
    let bridge = Arc::new(FakeBridge {
        state: json!({ "project_id": p.id, "sequence_id": p.seq, "playhead": 0, "playhead_s": 0.0,
                       "selection": [], "mark_in": null, "mark_out": null, "tool": "select",
                       "playing": false }),
        png: png_bytes(),
        sets: Mutex::new(Vec::new()),
        fail: None,
    });
    s.set_ui(Some(bridge.clone()));
    (s, p, bridge)
}

#[test]
fn s3_ui_screenshot_returns_bridge_image() {
    let (s, _, b) = bridge_setup();
    let o = call(&s, "ui_screenshot", json!({}));
    assert_eq!(o.images.len(), 1, "S-3: one image block");
    assert_eq!(o.images[0].mime, "image/png");
    assert_eq!(o.images[0].bytes, b.png, "S-3: the bridge's PNG");
}

#[test]
fn s3_ui_get_returns_bridge_json() {
    let (s, _, b) = bridge_setup();
    let o = call(&s, "ui_get", json!({}));
    assert_eq!(o.json, b.state, "S-3: ui_get = UiBridge::get");
}

#[test]
fn s3_ui_set_passes_fields_and_converts_t4_times() {
    let (s, p, b) = bridge_setup();
    call(
        &s,
        "ui_set",
        json!({ "project_id": p.id, "playhead": "15f", "mark_in": "0.5s", "mark_out": "00:00:01:05",
                "selection": [7, 8], "tool": "razor" }),
    );
    let sets = b.sets.lock().unwrap().clone();
    assert_eq!(sets.len(), 1, "S-3: one UiBridge::set call");
    let v = &sets[0];
    // 15 frames at 25 fps = 15 * 705600000 / 25.
    assert_eq!(v["playhead"].as_i64(), Some(423_360_000), "playhead {v}");
    // 0.5 s = 352800000.
    assert_eq!(v["mark_in"].as_i64(), Some(352_800_000), "mark_in {v}");
    // 1 s + 5 frames at 25 fps = 30 frames = 846720000.
    assert_eq!(v["mark_out"].as_i64(), Some(846_720_000), "mark_out {v}");
    assert_eq!(v["selection"], json!([7, 8]));
    assert_eq!(v["tool"], "razor");
    assert_eq!(v["project_id"], json!(p.id));
}

#[test]
fn s3_ui_set_without_project_id_uses_window_sequence_fps() {
    let (s, _, b) = bridge_setup();
    call(&s, "ui_set", json!({ "playhead": "15f" }));
    call(&s, "ui_set", json!({ "playhead": 1234 }));
    let sets = b.sets.lock().unwrap().clone();
    assert_eq!(sets.len(), 2);
    assert_eq!(
        sets[0]["playhead"].as_i64(),
        Some(423_360_000),
        "S-3: 15f at the window's 25 fps, not the first project's 30 fps"
    );
    assert_eq!(
        sets[1]["playhead"].as_i64(),
        Some(1234),
        "flicks pass through"
    );
    for v in &sets {
        assert!(
            v.get("mark_in").is_none_or(Value::is_null),
            "no field invented: {v}"
        );
        assert!(
            v.get("selection").is_none_or(Value::is_null),
            "no field invented: {v}"
        );
    }
}

#[test]
fn s3_ui_set_malformed_time_is_error_and_not_forwarded() {
    let (s, _, b) = bridge_setup();
    let e = call_err(&s, "ui_set", json!({ "playhead": "abc" }));
    assert!(
        e.contains("playhead"),
        "T-4: error names the field, got {e:?}"
    );
    assert!(b.sets.lock().unwrap().is_empty(), "nothing forwarded");
}

#[test]
fn s3_bridge_errors_become_error_results() {
    let s = session();
    new_project(&s, json!({}));
    s.set_ui(Some(Arc::new(FakeBridge {
        state: json!({}),
        png: Vec::new(),
        sets: Mutex::new(Vec::new()),
        fail: Some("bridge says no".into()),
    })));
    for (tool, args) in [
        ("ui_screenshot", json!({})),
        ("ui_get", json!({})),
        ("ui_set", json!({ "playhead": 0 })),
    ] {
        let e = call_err(&s, tool, args);
        assert!(
            e.contains("bridge says no"),
            "{tool}: bridge error passed on, got {e:?}"
        );
    }
}

#[test]
fn s3_set_ui_none_detaches() {
    let (s, _, b) = bridge_setup();
    call(&s, "ui_get", json!({}));
    s.set_ui(None);
    for (tool, args) in [
        ("ui_screenshot", json!({})),
        ("ui_get", json!({})),
        ("ui_set", json!({ "playhead": 0 })),
    ] {
        let e = call_err(&s, tool, args);
        assert!(
            e.to_lowercase().contains("window"),
            "{tool} after detach: {e:?}"
        );
    }
    assert!(
        b.sets.lock().unwrap().is_empty(),
        "detached bridge gets no set"
    );
}

#[test]
fn s3_bridge_belongs_to_one_session() {
    let (_s, _, _) = bridge_setup();
    let other = session();
    new_project(&other, json!({}));
    let e = call_err(&other, "ui_get", json!({}));
    assert!(e.to_lowercase().contains("window"), "{e:?}");
}
