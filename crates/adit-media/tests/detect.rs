//! M-DET-1: silence and scene detection.
mod common;

use adit_media::Ffmpeg;
use common::*;

fn ff() -> Ffmpeg {
    Ffmpeg::new().expect("Ffmpeg::new with ffmpeg/ffprobe on PATH")
}

const TENTH: i64 = S / 10;

#[test]
fn m_det_1_silence_between_tones() {
    timed(60, || {
        let d = TempDir::new("det-sil");
        let p = tone_with_gap_wav(d.path(), "gap.wav", 1.0, 2.0);
        let r = ff().detect_silence(&p, -30.0, S / 2).unwrap();
        assert_eq!(r.len(), 1, "{r:?}");
        let (a, b) = r[0];
        assert!((a - S).abs() <= TENTH, "start {a} not within 0.1 s of 1 s");
        assert!(
            (b - 2 * S).abs() <= TENTH,
            "end {b} not within 0.1 s of 2 s"
        );
    })
}

#[test]
fn m_det_1_no_silence_is_empty() {
    timed(60, || {
        let d = TempDir::new("det-nosil");
        let p = tone_wav(d.path());
        assert_eq!(ff().detect_silence(&p, -30.0, S / 2).unwrap(), vec![]);
    })
}

#[test]
fn m_det_1_silence_shorter_than_min_duration_is_ignored() {
    timed(60, || {
        let d = TempDir::new("det-short");
        let p = tone_with_gap_wav(d.path(), "gap.wav", 1.0, 1.3); // 0.3 s gap < 0.5 s
        assert_eq!(ff().detect_silence(&p, -30.0, S / 2).unwrap(), vec![]);
    })
}

#[test]
fn m_det_1_scene_cut_red_to_blue() {
    timed(60, || {
        let d = TempDir::new("det-cut");
        let p = cut_mp4(d.path());
        let r = ff().detect_scenes(&p, 0.3).unwrap();
        assert_eq!(r.len(), 1, "{r:?}");
        // Within one frame (1/30 s) of 1 s.
        assert!(
            (r[0] - S).abs() <= t30(1),
            "cut at {} not within one frame of 1 s",
            r[0]
        );
    })
}

#[test]
fn m_det_1_no_cut_is_empty() {
    timed(60, || {
        let d = TempDir::new("det-nocut");
        let p = nocut_mp4(d.path());
        assert_eq!(ff().detect_scenes(&p, 0.3).unwrap(), Vec::<i64>::new());
    })
}

#[test]
fn m_det_1_missing_file_is_err() {
    timed(30, || {
        let d = TempDir::new("det-missing");
        let ff = ff();
        assert!(
            ff.detect_silence(&d.file("nope.wav"), -30.0, S / 2)
                .is_err()
        );
        assert!(ff.detect_scenes(&d.file("nope.mp4"), 0.3).is_err());
    })
}
