//! In-process `Session` additions S-4 (`waveform`) and S-5 (`render_scaled`), docs/GUI.md.
//! Written from the contract only; expected values are hand-computed literals (SPEC M-PAT).
mod common;
use adit_core::{AudioInfo, MediaSpec, Rational, Time};
use adit_mcp::Session;
use adit_media::{Frame, MediaBackend, MediaError, Pattern};
use common::*;
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

fn pattern(duration: Time) -> Pattern {
    Pattern {
        width: 64,
        height: 64,
        fps: Rational { num: 30, den: 1 },
        duration: Some(duration),
    }
}

// ---------------------------------------------------------------- S-4

#[test]
fn s_4_pattern_gives_500_peaks_of_half() {
    let s = session();
    let w = s
        .waveform("clip.mov")
        .unwrap_or_else(|e| panic!("S-4: waveform of Pattern media failed: {e}"));
    // 10 s * 50 peaks per second = 500; left 0.5, right 0.25 -> larger |sample| = 0.5.
    assert_eq!(w.len(), 500, "S-4: Pattern (10 s) gives 500 peaks");
    for (i, p) in w.iter().enumerate() {
        assert!(
            (p - 0.5).abs() <= 1e-6,
            "S-4: peak {i} = {p}, expected 0.5 (the louder channel)"
        );
    }
}

#[test]
fn s_4_last_partial_bucket_counts() {
    // 10.01 s = 500.5 buckets of 1/50 s -> 501 peaks.
    let s = Session::new(Arc::new(pattern(10 * S + S / 100)));
    let w = s.waveform("clip.mov").expect("S-4: waveform");
    assert_eq!(w.len(), 501, "S-4: a last partial bucket counts");
    assert!(
        (w[500] - 0.5).abs() <= 1e-6,
        "S-4: the partial bucket holds its peak, got {}",
        w[500]
    );

    // 0.5 s -> 25 peaks.
    let s = Session::new(Arc::new(pattern(S / 2)));
    assert_eq!(
        s.waveform("clip.mov").expect("S-4: waveform").len(),
        25,
        "S-4: 50 peaks per second"
    );
}

/// 2 s of stereo: left is -0.75 in the first second and 0.1 after it, right is always 0.25.
struct Steps;

impl MediaBackend for Steps {
    fn probe(&self, path: &str) -> Result<MediaSpec, MediaError> {
        Ok(MediaSpec {
            name: path.to_string(),
            path: path.to_string(),
            duration: Some(2 * S),
            video: None,
            audio: Some(AudioInfo {
                sample_rate: 48000,
                channels: 2,
            }),
        })
    }
    fn video_frame(&self, _: &str, _: Time) -> Result<Arc<Frame>, MediaError> {
        Err(MediaError::Unsupported("no video".into()))
    }
    fn audio(
        &self,
        _: &str,
        start: Time,
        duration: Time,
        sample_rate: u32,
        _: &str,
    ) -> Result<Vec<f32>, MediaError> {
        let sr = sample_rate as f64;
        let first = (start as f64 * sr / S as f64).round() as i64;
        let frames = (duration as f64 * sr / S as f64).round() as i64;
        let mut out = Vec::with_capacity(frames as usize * 2);
        for n in first..first + frames {
            out.push(if n < sample_rate as i64 { -0.75 } else { 0.1 });
            out.push(0.25);
        }
        Ok(out)
    }
}

#[test]
fn s_4_peak_is_largest_abs_sample_of_either_channel_per_bucket() {
    let s = Session::new(Arc::new(Steps));
    let w = s.waveform("steps.wav").expect("S-4: waveform");
    assert_eq!(w.len(), 100, "S-4: 2 s -> 100 peaks");
    for (i, p) in w.iter().enumerate() {
        // Buckets 0..50 cover [0 s, 1 s): |-0.75| on the left. Buckets 50..100: right 0.25 > 0.1.
        let want = if i < 50 { 0.75 } else { 0.25 };
        assert!(
            (p - want).abs() <= 1e-6,
            "S-4: peak {i} = {p}, expected {want} (largest |sample| of either channel in \
             [{i}/50 s, {}/50 s))",
            i + 1
        );
    }
}

/// Pattern that counts `audio` calls and records their filters.
struct Counting {
    inner: Pattern,
    calls: AtomicUsize,
    filters: Mutex<Vec<String>>,
}

impl MediaBackend for Counting {
    fn probe(&self, path: &str) -> Result<MediaSpec, MediaError> {
        self.inner.probe(path)
    }
    fn video_frame(&self, path: &str, time: Time) -> Result<Arc<Frame>, MediaError> {
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
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.filters.lock().unwrap().push(filter.to_string());
        self.inner.audio(path, start, duration, sample_rate, filter)
    }
}

#[test]
fn s_4_computed_once_per_path_and_cached() {
    let media = Arc::new(Counting {
        inner: pattern(10 * S),
        calls: AtomicUsize::new(0),
        filters: Mutex::new(Vec::new()),
    });
    let s = Session::new(media.clone());
    let a = s.waveform("a.mov").expect("S-4: first call");
    let after_first = media.calls.load(Ordering::SeqCst);
    assert!(after_first >= 1, "S-4: computed from MediaBackend::audio");
    assert!(
        media.filters.lock().unwrap().iter().all(String::is_empty),
        "S-4: no filter, got {:?}",
        media.filters.lock().unwrap()
    );

    let b = s.waveform("a.mov").expect("S-4: second call");
    assert_eq!(
        media.calls.load(Ordering::SeqCst),
        after_first,
        "S-4: computed once per path, the second call reads the cache"
    );
    assert!(Arc::ptr_eq(&a, &b), "S-4: the cached Arc is returned again");
    assert_eq!(*a, *b, "S-4: same data");

    // Clones share the state, so they share the cache.
    let c = s.clone().waveform("a.mov").expect("S-4: clone");
    assert_eq!(
        media.calls.load(Ordering::SeqCst),
        after_first,
        "S-4: a clone of the session uses the same cache"
    );
    assert_eq!(*a, *c, "S-4: same data from a clone");

    // Another path is computed on its own.
    let d = s.waveform("b.mov").expect("S-4: other path");
    assert!(
        media.calls.load(Ordering::SeqCst) > after_first,
        "S-4: cached per path, another path is computed"
    );
    assert_eq!(d.len(), 500, "S-4: other path");
}

/// Media with a picture only.
struct Silent;

impl MediaBackend for Silent {
    fn probe(&self, path: &str) -> Result<MediaSpec, MediaError> {
        let mut spec = pattern(10 * S).probe(path)?;
        spec.audio = None;
        Ok(spec)
    }
    fn video_frame(&self, path: &str, time: Time) -> Result<Arc<Frame>, MediaError> {
        pattern(10 * S).video_frame(path, time)
    }
    fn audio(&self, _: &str, _: Time, _: Time, _: u32, _: &str) -> Result<Vec<f32>, MediaError> {
        Ok(Vec::new())
    }
}

/// Media whose audio cannot be read.
struct Failing {
    probe_ok: bool,
}

impl MediaBackend for Failing {
    fn probe(&self, path: &str) -> Result<MediaSpec, MediaError> {
        if self.probe_ok {
            pattern(10 * S).probe(path)
        } else {
            Err(MediaError::NotFound(path.to_string()))
        }
    }
    fn video_frame(&self, path: &str, time: Time) -> Result<Arc<Frame>, MediaError> {
        pattern(10 * S).video_frame(path, time)
    }
    fn audio(&self, _: &str, _: Time, _: Time, _: u32, _: &str) -> Result<Vec<f32>, MediaError> {
        Err(MediaError::Ffmpeg("decoder broke".into()))
    }
}

#[test]
fn s_4_media_without_audio_is_err() {
    let s = Session::new(Arc::new(Silent));
    let e = s
        .waveform("silent.mov")
        .expect_err("S-4: media without audio -> Err");
    assert!(!e.trim().is_empty(), "S-4: the error has a message");
}

#[test]
fn s_4_failing_backend_is_err() {
    for probe_ok in [true, false] {
        let s = Session::new(Arc::new(Failing { probe_ok }));
        let e = s
            .waveform("broken.mov")
            .expect_err("S-4: a failing backend -> Err");
        assert!(
            !e.trim().is_empty(),
            "S-4: the error has a message (probe_ok {probe_ok})"
        );
    }
}

// ---------------------------------------------------------------- S-5

/// A project of the given size with Pattern media on V2 over an opaque red solid on V1.
fn pattern_project(
    s: &Session,
    dir: &std::path::Path,
    w: u32,
    h: u32,
) -> (Proj, adit_core::Project) {
    let p = new_project(s, json!({ "width": w, "height": h }));
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
    let (proj, _) = s.snapshot(&p.id).expect("snapshot");
    (p, proj)
}

fn scaled(s: &Session, proj: &adit_core::Project, seq: u64, time: Time, divisor: u32) -> Frame {
    let f = s
        .render_scaled(proj, seq, time, divisor)
        .unwrap_or_else(|e| panic!("S-5: render_scaled divisor {divisor} failed: {e}"));
    assert_eq!(
        f.rgba.len(),
        (f.width * f.height * 4) as usize,
        "S-5: RGBA buffer matches the frame size (divisor {divisor})"
    );
    f
}

#[test]
fn s_5_divisor_1_equals_render() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let (p, proj) = pattern_project(&s, dir.path(), 64, 64);
    for time in [0, 15 * F30, 45 * F30] {
        let full = s.render(&proj, p.seq, time).expect("render");
        let one = scaled(&s, &proj, p.seq, time, 1);
        assert_eq!(
            (one.width, one.height),
            (64, 64),
            "S-5: divisor 1 keeps the size"
        );
        assert_eq!(
            (one.width, one.height),
            (full.width, full.height),
            "S-5: divisor 1 equals render (size)"
        );
        assert!(
            one.rgba == full.rgba,
            "S-5: divisor 1 equals render (bytes) at time {time}"
        );
    }
}

#[test]
fn s_5_size_is_ceil_of_size_over_divisor() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let (p, proj) = pattern_project(&s, dir.path(), 64, 64);
    let f2 = scaled(&s, &proj, p.seq, 0, 2);
    assert_eq!((f2.width, f2.height), (32, 32), "S-5: 64x64 / 2");
    let f4 = scaled(&s, &proj, p.seq, 0, 4);
    assert_eq!((f4.width, f4.height), (16, 16), "S-5: 64x64 / 4");

    // ceil(65 / 2) = 33, ceil(33 / 2) = 17; ceil(65 / 4) = 17, ceil(33 / 4) = 9.
    let (p, proj) = pattern_project(&s, dir.path(), 65, 33);
    let f1 = scaled(&s, &proj, p.seq, 0, 1);
    assert_eq!((f1.width, f1.height), (65, 33), "S-5: 65x33 / 1");
    let f2 = scaled(&s, &proj, p.seq, 0, 2);
    assert_eq!((f2.width, f2.height), (33, 17), "S-5: 65x33 / 2 rounds up");
    let f4 = scaled(&s, &proj, p.seq, 0, 4);
    assert_eq!((f4.width, f4.height), (17, 9), "S-5: 65x33 / 4 rounds up");
}

#[test]
fn s_5_other_divisors_are_err() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let (p, proj) = pattern_project(&s, dir.path(), 64, 64);
    for d in [0, 3, 5, 6, 8, 16, u32::MAX] {
        let e = match s.render_scaled(&proj, p.seq, 0, d) {
            Ok(f) => panic!(
                "S-5: divisor {d} must be Err, got a {}x{} frame",
                f.width, f.height
            ),
            Err(e) => e,
        };
        assert!(
            !e.trim().is_empty(),
            "S-5: divisor {d}: error has a message"
        );
    }
}

#[test]
fn s_5_unknown_sequence_is_err() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let (p, proj) = pattern_project(&s, dir.path(), 64, 64);
    // Guard: the same call with the real sequence works, so the Err below is about the id.
    scaled(&s, &proj, p.seq, 0, 2);
    for d in [1, 2, 4] {
        assert!(
            s.render_scaled(&proj, 999_999, 0, d).is_err(),
            "S-5: unknown sequence -> Err (divisor {d})"
        );
    }
}

#[test]
fn s_5_solid_colour_stays_that_colour() {
    let s = session();
    let p = new_project(&s, json!({}));
    // (0.2, 0.4, 0.8) * 255 = (51, 102, 204).
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [0.2, 0.4, 0.8, 1.0],
    );
    let (proj, _) = s.snapshot(&p.id).unwrap();
    for (d, w, h) in [(1u32, 64u32, 48u32), (2, 32, 24), (4, 16, 12)] {
        let f = scaled(&s, &proj, p.seq, 0, d);
        assert_eq!((f.width, f.height), (w, h), "S-5: 64x48 / {d}");
        for (i, px) in f.rgba.chunks(4).enumerate() {
            for (c, want) in [51i32, 102, 204, 255].into_iter().enumerate() {
                assert!(
                    (px[c] as i32 - want).abs() <= 2,
                    "S-5: divisor {d}, pixel {i} = {px:?}, expected (51, 102, 204, 255) ±2"
                );
            }
        }
    }
}

#[test]
fn s_5_scaled_frame_is_the_render_frame_shrunk() {
    let s = session();
    let dir = tempfile::tempdir().unwrap();
    let (p, proj) = pattern_project(&s, dir.path(), 64, 64);
    let time = 7 * F30;
    let full = s.render(&proj, p.seq, time).expect("render");
    assert_eq!((full.width, full.height), (64, 64));
    for d in [2u32, 4] {
        let small = scaled(&s, &proj, p.seq, time, d);
        let (sw, sh) = (64 / d, 64 / d);
        assert_eq!((small.width, small.height), (sw, sh), "S-5: size / {d}");
        for y in 0..sh {
            for x in 0..sw {
                let got = &small.rgba[((y * sw + x) * 4) as usize..][..4];
                for c in 0..4 {
                    // Any bilinear sample taken inside the d x d source block lies between the
                    // block's smallest and largest value (the picture is a smooth ramp).
                    let (mut lo, mut hi) = (255i32, 0i32);
                    for by in 0..d {
                        for bx in 0..d {
                            let v = full.rgba[(((y * d + by) * 64 + x * d + bx) * 4) as usize + c]
                                as i32;
                            lo = lo.min(v);
                            hi = hi.max(v);
                        }
                    }
                    let g = got[c] as i32;
                    assert!(
                        g >= lo - 2 && g <= hi + 2,
                        "S-5: divisor {d}, pixel ({x},{y}) channel {c} = {g}, outside its \
                         source block {lo}..{hi} ±2"
                    );
                }
            }
        }
    }
}

#[test]
fn s_5_works_on_the_snapshot_passed_in() {
    let s = session();
    let p = new_project(&s, json!({}));
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [0.0, 1.0, 0.0, 1.0],
    );
    let (old, _) = s.snapshot(&p.id).unwrap();
    // The session moves on; the snapshot still renders green.
    call(&s, "undo", json!({ "project_id": p.id }));
    let f = scaled(&s, &old, p.seq, 0, 4);
    assert_eq!((f.width, f.height), (16, 12), "S-5: 64x48 / 4");
    for px in f.rgba.chunks(4) {
        assert!(
            px[0] <= 2 && px[1] >= 253 && px[2] <= 2,
            "S-5: the frame of `render` on the snapshot passed in, got {px:?}"
        );
    }
}
