//! M-FRAME-5 (prefetch), M-FRAME-6 (several positions).
mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use adit_media::{Ffmpeg, MediaBackend, MediaError};
use common::*;

fn ff() -> Ffmpeg {
    Ffmpeg::new().expect("Ffmpeg::new with ffmpeg/ffprobe on PATH")
}

const FRAMES: u32 = 200;

#[test]
fn m_frame_5_prefetch_does_not_change_pixels() {
    timed(120, || {
        let d = TempDir::new("pf-same");
        let p = id_video(d.path(), "id30.mov", "30", FRAMES);
        let with = ff();
        // 0 and 5: near; 150: far from any other read; 90: another jump.
        for n in [0u32, 5, 150, 90, 91, 3] {
            with.prefetch(&p, t30(n as i64));
            // Give a background implementation time to finish, and also read at once on some.
            if n % 2 == 0 {
                std::thread::sleep(Duration::from_millis(300));
            }
            let a = with.video_frame(&p, t30(n as i64)).unwrap();
            assert_id_frame(&a, n);
            let fresh = ff().video_frame(&p, t30(n as i64)).unwrap();
            assert_eq!(a.rgba, fresh.rgba, "frame {n}");
        }
    })
}

#[test]
fn m_frame_5_prefetch_negative_time_is_frame_0() {
    timed(60, || {
        let d = TempDir::new("pf-neg");
        let p = id_video(d.path(), "id30.mov", "30", 30);
        let f = ff();
        f.prefetch(&p, -S);
        assert_id_frame(&f.video_frame(&p, 0).unwrap(), 0);
        assert_id_frame(&f.video_frame(&p, -S).unwrap(), 0);
    })
}

#[test]
fn m_frame_5_prefetch_missing_file_is_harmless() {
    timed(60, || {
        let d = TempDir::new("pf-missing");
        let p = d.file("nope.mp4");
        let f = ff();
        f.prefetch(&p, 0);
        f.prefetch(&p, S);
        assert!(matches!(f.video_frame(&p, 0), Err(MediaError::NotFound(_))));
        assert!(matches!(f.probe(&p), Err(MediaError::NotFound(_))));
    })
}

#[test]
fn m_frame_5_prefetch_audio_only_file_is_harmless() {
    timed(60, || {
        let d = TempDir::new("pf-audio");
        let p = ramp_stereo_wav(d.path());
        let f = ff();
        f.prefetch(&p, 0);
        f.prefetch(&p, S);
        assert!(f.video_frame(&p, 0).is_err());
        assert!(f.probe(&p).unwrap().video.is_none());
    })
}

#[test]
fn m_frame_5_prefetch_still_is_harmless() {
    timed(60, || {
        let d = TempDir::new("pf-still");
        let p = still_png(d.path());
        let f = ff();
        f.prefetch(&p, 0);
        f.prefetch(&p, 10 * S);
        let a = f.video_frame(&p, 0).unwrap();
        let b = f.video_frame(&p, 10 * S).unwrap();
        assert_eq!((a.width, a.height), (16, 16));
        assert_eq!(a.rgba, b.rgba);
        // M-PAT/M-FRAME-4: pixel (1,2) = (200,100,50,16*1+2)
        let i = (2 * 16 + 1) * 4;
        assert_eq!(&a.rgba[i..i + 4], &[200, 100, 50, 18]);
    })
}

#[test]
fn m_frame_5_prefetch_repeated_and_unreachable_is_harmless() {
    timed(60, || {
        let d = TempDir::new("pf-rep");
        let p = id_video(d.path(), "id30.mov", "30", 60);
        let f = ff();
        for _ in 0..5 {
            f.prefetch(&p, t30(40));
        }
        f.prefetch(&p, 1000 * S); // past the end: last frame
        assert_id_frame(&f.video_frame(&p, t30(40)).unwrap(), 40);
        assert_id_frame(&f.video_frame(&p, 1000 * S).unwrap(), 59);
    })
}

#[test]
fn m_frame_6_alternating_positions_exact_and_fast() {
    timed(120, || {
        let d = TempDir::new("alt");
        let p = id_video(d.path(), "id30.mov", "30", FRAMES);
        let f = ff();
        // First access also probes; warm the probe so the clock measures reads.
        f.probe(&p).unwrap();
        let (a, b) = (0u32, 150u32); // 5 s apart
        let t0 = Instant::now();
        for i in 0..20u32 {
            let fa = f.video_frame(&p, t30((a + i) as i64)).unwrap();
            assert_id_frame(&fa, a + i);
            let fb = f.video_frame(&p, t30((b + i) as i64)).unwrap();
            assert_id_frame(&fb, b + i);
        }
        let el = t0.elapsed();
        assert!(
            el < Duration::from_secs(5),
            "40 alternating reads took {el:?}"
        );
    })
}

#[test]
fn m_frame_6_alternating_with_prefetch_exact_and_fast() {
    timed(120, || {
        let d = TempDir::new("alt-pf");
        let p = id_video(d.path(), "id30.mov", "30", FRAMES);
        let f = ff();
        f.probe(&p).unwrap();
        let (a, b) = (10u32, 160u32);
        let t0 = Instant::now();
        for i in 0..20u32 {
            f.prefetch(&p, t30((b + i + 1) as i64));
            assert_id_frame(&f.video_frame(&p, t30((a + i) as i64)).unwrap(), a + i);
            f.prefetch(&p, t30((a + i + 1) as i64));
            assert_id_frame(&f.video_frame(&p, t30((b + i) as i64)).unwrap(), b + i);
        }
        let el = t0.elapsed();
        assert!(el < Duration::from_secs(5), "alternating reads took {el:?}");
    })
}

#[test]
fn m_frame_6_five_positions_still_exact() {
    timed(120, || {
        let d = TempDir::new("five");
        let p = id_video(d.path(), "id30.mov", "30", FRAMES);
        let f = ff();
        let starts = [0u32, 40, 80, 120, 160]; // 5 readers wanted, at most 4 kept
        for i in 0..6u32 {
            for s in starts {
                assert_id_frame(&f.video_frame(&p, t30((s + i) as i64)).unwrap(), s + i);
            }
        }
    })
}

#[test]
fn m_frame_5_prefetch_while_other_thread_reads_sequentially() {
    timed(120, || {
        let d = TempDir::new("pf-thread");
        let p = id_video(d.path(), "id30.mov", "30", FRAMES);
        let f = Arc::new(ff());
        let reader = {
            let (f, p) = (f.clone(), p.clone());
            std::thread::spawn(move || {
                for n in 0..60u32 {
                    assert_id_frame(&f.video_frame(&p, t30(n as i64)).unwrap(), n);
                }
            })
        };
        for k in 0..30u32 {
            f.prefetch(&p, t30((100 + k * 3) as i64));
            std::thread::sleep(Duration::from_millis(5));
        }
        reader.join().expect("sequential reader thread");
        // Prefetched far frame is still exact.
        assert_id_frame(&f.video_frame(&p, t30(130)).unwrap(), 130);
    })
}
