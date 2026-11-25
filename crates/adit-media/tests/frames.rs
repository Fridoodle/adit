//! M-FRAME-1..4, M-PROC-1 (decode side).
//! Each access order runs on its own freshly generated file (own temp dir, own path) and its own
//! backend, so a cache filled by one order cannot hide a wrong decode in another.
mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use adit_media::{Ffmpeg, MediaBackend, MediaError};
use common::*;

fn ff() -> Ffmpeg {
    Ffmpeg::new().expect("Ffmpeg::new with ffmpeg/ffprobe on PATH")
}

#[derive(Clone, Copy, Debug)]
enum Fixture {
    /// Lossless png-in-mov, 30 fps, pixel-exact identity.
    Id30,
    /// Lossless png-in-mov, 30000/1001 fps.
    Id2997,
    /// H.264 in mp4, 30 fps, flat grey 4N.
    Gray30Mp4,
    /// H.264 in mkv (1 ms timestamps, so pts are rounded), 30000/1001 fps, flat grey 4N.
    Gray2997Mkv,
}

const FRAMES: u32 = 60;

impl Fixture {
    fn make(self, d: &TempDir) -> String {
        match self {
            Fixture::Id30 => id_video(d.path(), "id30.mov", "30", FRAMES),
            Fixture::Id2997 => id_video(d.path(), "id2997.mov", "30000/1001", FRAMES),
            Fixture::Gray30Mp4 => gray_h264(d.path(), "gray30.mp4", "30", FRAMES),
            Fixture::Gray2997Mkv => gray_h264(d.path(), "gray2997.mkv", "30000/1001", FRAMES),
        }
    }
    fn start(self, n: u32) -> i64 {
        match self {
            Fixture::Id30 | Fixture::Gray30Mp4 => t30(n as i64),
            Fixture::Id2997 | Fixture::Gray2997Mkv => t2997(n as i64),
        }
    }
    /// Asserts the decoded picture is frame `n`.
    fn check(self, f: &adit_media::Frame, n: u32, ctx: &str) {
        match self {
            Fixture::Id30 | Fixture::Id2997 => {
                assert_eq!(
                    f.rgba[0] as u32, n,
                    "{ctx}: wanted frame {n}, decoded frame {}",
                    f.rgba[0]
                );
                assert_id_frame(f, n);
            }
            Fixture::Gray30Mp4 | Fixture::Gray2997Mkv => {
                let got = gray_frame_number(f);
                assert_eq!(got, n, "{ctx}: wanted frame {n}, decoded frame {got}");
            }
        }
    }
}

/// M-FRAME-1: ask for the start time of each frame in `order`, assert the exact frame.
fn run_order(fx: Fixture, order: Vec<u32>, tag: &'static str) {
    timed(120, move || {
        let d = TempDir::new(tag);
        let p = fx.make(&d);
        let ff = ff();
        for (i, &n) in order.iter().enumerate() {
            let f = ff.video_frame(&p, fx.start(n)).unwrap();
            fx.check(&f, n, &format!("{fx:?} request #{i}"));
        }
    })
}

fn sequential_then_jumps() -> Vec<u32> {
    let mut v: Vec<u32> = (0..FRAMES).collect();
    v.extend([37, 5, 58, 6, 0]);
    v
}
fn backwards() -> Vec<u32> {
    (0..FRAMES).rev().collect()
}
fn random() -> Vec<u32> {
    vec![37, 5, 58, 6, 0, 59, 1, 30, 29, 31, 12, 44, 2, 3, 50, 49, 17]
}

#[test]
fn m_frame_1_sequential_id_30fps() {
    run_order(Fixture::Id30, sequential_then_jumps(), "seq-id30");
}
#[test]
fn m_frame_1_sequential_id_2997fps() {
    run_order(Fixture::Id2997, sequential_then_jumps(), "seq-id2997");
}
#[test]
fn m_frame_1_sequential_h264_mp4_30fps() {
    run_order(Fixture::Gray30Mp4, sequential_then_jumps(), "seq-g30");
}
#[test]
fn m_frame_1_sequential_h264_mkv_2997fps() {
    run_order(Fixture::Gray2997Mkv, sequential_then_jumps(), "seq-g2997");
}
#[test]
fn m_frame_1_backwards_id_30fps() {
    run_order(Fixture::Id30, backwards(), "back-id30");
}
#[test]
fn m_frame_1_backwards_id_2997fps() {
    run_order(Fixture::Id2997, backwards(), "back-id2997");
}
#[test]
fn m_frame_1_backwards_h264_mp4_30fps() {
    run_order(Fixture::Gray30Mp4, backwards(), "back-g30");
}
#[test]
fn m_frame_1_backwards_h264_mkv_2997fps() {
    run_order(Fixture::Gray2997Mkv, backwards(), "back-g2997");
}
#[test]
fn m_frame_1_random_id_30fps() {
    run_order(Fixture::Id30, random(), "rand-id30");
}
#[test]
fn m_frame_1_random_id_2997fps() {
    run_order(Fixture::Id2997, random(), "rand-id2997");
}
#[test]
fn m_frame_1_random_h264_mp4_30fps() {
    run_order(Fixture::Gray30Mp4, random(), "rand-g30");
}
#[test]
fn m_frame_1_random_h264_mkv_2997fps() {
    run_order(Fixture::Gray2997Mkv, random(), "rand-g2997");
}

/// M-FRAME-1 with n = time_to_frame(t): any t inside [start(n), start(n+1)) gives frame n.
fn run_inside_frame(fx: Fixture, tag: &'static str) {
    timed(120, move || {
        let d = TempDir::new(tag);
        let p = fx.make(&d);
        let ff = ff();
        for n in [0u32, 1, 7, 29, 30, 31, 58] {
            let (a, b) = (fx.start(n), fx.start(n + 1));
            for t in [b - 1, a + (b - a) / 2, a + 1] {
                let f = ff.video_frame(&p, t).unwrap();
                fx.check(&f, n, &format!("{fx:?} t={t}"));
            }
        }
    })
}

#[test]
fn m_frame_1_time_inside_frame_id_2997fps() {
    run_inside_frame(Fixture::Id2997, "inside-id2997");
}
#[test]
fn m_frame_1_time_inside_frame_h264_mkv_2997fps() {
    run_inside_frame(Fixture::Gray2997Mkv, "inside-g2997");
}
#[test]
fn m_frame_1_time_inside_frame_h264_mp4_30fps() {
    run_inside_frame(Fixture::Gray30Mp4, "inside-g30");
}

#[test]
fn m_frame_1_decodes_at_stream_size() {
    timed(60, || {
        let d = TempDir::new("size");
        let p = av_mp4(d.path(), "av.mp4");
        let f = ff().video_frame(&p, t30(10)).unwrap();
        assert_eq!((f.width, f.height), (64, 48));
        assert_eq!(f.rgba.len(), 64 * 48 * 4);
    })
}

#[test]
fn m_frame_2_clamps_before_start_and_after_end() {
    timed(120, || {
        let d = TempDir::new("clamp");
        let ff = ff();
        let p = id_video(d.path(), "id30.mov", "30", FRAMES);
        for t in [-1, -S, i64::MIN / 4] {
            assert_id_frame(&ff.video_frame(&p, t).unwrap(), 0);
        }
        // Duration = 60 frames = 2 s; t >= duration -> last frame (59).
        for t in [t30(60), t30(60) + 1, 100 * S] {
            assert_id_frame(&ff.video_frame(&p, t).unwrap(), 59);
        }
        let g = gray_h264(d.path(), "gray2997.mkv", "30000/1001", FRAMES);
        assert_eq!(gray_frame_number(&ff.video_frame(&g, -5).unwrap()), 0);
        assert_eq!(
            gray_frame_number(&ff.video_frame(&g, t2997(60)).unwrap()),
            59
        );
        assert_eq!(gray_frame_number(&ff.video_frame(&g, 50 * S).unwrap()), 59);
    })
}

/// still.png: pixel (x, y) = (200, 100, 50, 16x + y).
fn assert_still(f: &adit_media::Frame) {
    assert_eq!((f.width, f.height), (16, 16));
    assert_eq!(f.rgba.len(), 16 * 16 * 4);
    for y in 0..16u32 {
        for x in 0..16u32 {
            let i = ((y * 16 + x) * 4) as usize;
            let a = (x * 16 + y) as u8;
            assert_eq!(f.rgba[i + 3], a, "alpha at ({x},{y})");
            if a > 0 {
                // Straight alpha: colour unchanged by alpha (premultiplied would scale it).
                assert_eq!(&f.rgba[i..i + 3], &[200, 100, 50], "rgb at ({x},{y}) a={a}");
            }
        }
    }
}

#[test]
fn m_frame_2_still_returns_image_for_every_time() {
    timed(60, || {
        let d = TempDir::new("still-t");
        let p = still_png(d.path());
        let ff = ff();
        for t in [-S, 0, 1, t30(1), 3 * S, 1000 * S] {
            assert_still(&ff.video_frame(&p, t).unwrap());
        }
    })
}

#[test]
fn m_frame_4_png_keeps_straight_alpha() {
    timed(60, || {
        let d = TempDir::new("alpha");
        let p = still_png(d.path());
        assert_still(&ff().video_frame(&p, 0).unwrap());
    })
}

#[test]
fn m_frame_4_opaque_source_has_alpha_255() {
    timed(60, || {
        let d = TempDir::new("opaque");
        let p = av_mp4(d.path(), "av.mp4");
        let f = ff().video_frame(&p, t30(3)).unwrap();
        assert!(f.rgba.as_chunks::<4>().0.iter().all(|px| px[3] == 255));
        // gray_frame_number also asserts a == 255 on an h264 source.
        let g = gray_h264(d.path(), "g.mp4", "30", 10);
        assert_eq!(gray_frame_number(&ff().video_frame(&g, t30(4)).unwrap()), 4);
    })
}

/// Median wall time of one ffmpeg process that decodes a single frame of `p`.
fn one_launch(p: &str) -> Duration {
    let mut v: Vec<Duration> = (0..5)
        .map(|_| {
            let t0 = Instant::now();
            let out = std::process::Command::new("ffmpeg")
                .args(["-v", "error", "-ss", "0.5", "-i", p, "-frames:v", "1"])
                .args(["-f", "rawvideo", "-pix_fmt", "rgba", "-"])
                .output()
                .unwrap();
            assert!(out.status.success());
            t0.elapsed()
        })
        .collect();
    v.sort();
    v[2]
}

#[test]
fn m_frame_3_sequential_reads_reuse_one_process() {
    timed(120, || {
        let d = TempDir::new("seq-time");
        let p = id_video(d.path(), "id30.mov", "30", FRAMES);
        let launch = one_launch(&p);
        let ff = ff();
        let t0 = Instant::now();
        for n in 0..FRAMES {
            assert_id_frame(&ff.video_frame(&p, t30(n as i64)).unwrap(), n);
        }
        let took = t0.elapsed();
        // SPEC bound: < 5 s. On a fast machine 60 launches also fit in 5 s, so additionally
        // require well under 60 launches: at most 20x one measured launch.
        let bound = Duration::from_secs(5).min(launch * 20);
        assert!(
            took < bound,
            "60 sequential frames took {took:?}; bound {bound:?} (one ffmpeg launch = {launch:?})"
        );
    })
}

#[test]
fn m_frame_3_same_frame_returns_same_arc() {
    timed(60, || {
        let d = TempDir::new("arc");
        let p = id_video(d.path(), "id30.mov", "30", 20);
        let ff = ff();
        let a = ff.video_frame(&p, t30(5)).unwrap();
        let b = ff.video_frame(&p, t30(5)).unwrap();
        assert!(
            Arc::ptr_eq(&a, &b),
            "same time twice must return the same Arc"
        );
        // Same frame through a different time inside it.
        let c = ff.video_frame(&p, t30(5) + 1000).unwrap();
        assert!(
            Arc::ptr_eq(&a, &c),
            "same frame n twice must return the same Arc"
        );
        // After visiting other frames, still cached.
        let _ = ff.video_frame(&p, t30(12)).unwrap();
        let e = ff.video_frame(&p, t30(5)).unwrap();
        assert!(Arc::ptr_eq(&a, &e));
        // Different frames are different pictures.
        let other = ff.video_frame(&p, t30(6)).unwrap();
        assert!(!Arc::ptr_eq(&a, &other));
        assert_id_frame(&a, 5);
        assert_id_frame(&other, 6);
    })
}

#[test]
fn m_proc_1_video_frame_path_with_space_and_non_ascii() {
    timed(60, || {
        let d = TempDir::new("frame-uni");
        let sub = d.unicode_subdir();
        let p = id_video(&sub, "clip ä.mov", "30", 6);
        let ff = ff();
        assert_id_frame(&ff.video_frame(&p, t30(2)).unwrap(), 2);
        assert_id_frame(&ff.video_frame(&p, t30(0)).unwrap(), 0);
    })
}

#[test]
fn video_frame_missing_file_is_not_found() {
    // Spec gap: M-PROBE-3 names NotFound for probe; the same error is the sensible reading here.
    timed(30, || {
        let d = TempDir::new("frame-missing");
        let r = ff().video_frame(&d.file("nope.mp4"), 0);
        assert!(matches!(r, Err(MediaError::NotFound(_))), "{r:?}");
    })
}
