//! M-PAT: procedural source. Expected values from the SPEC formulas and Q4 arithmetic.
mod common;

use adit_core::{AudioInfo, MediaSpec, Rational, VideoInfo};
use adit_media::{MediaBackend, Pattern};
use common::*;

const R30: Rational = Rational { num: 30, den: 1 };
const R2997: Rational = Rational {
    num: 30000,
    den: 1001,
};

fn pat(fps: Rational, duration: Option<i64>) -> Pattern {
    Pattern {
        width: 300,
        height: 260,
        fps,
        duration,
    }
}

/// pixel (x, y) = (x % 256, y % 256, n % 256, 255), checked for every pixel.
fn assert_pattern(p: &Pattern, t: i64, n: u32) {
    let f = p.video_frame("ignored", t).unwrap();
    assert_eq!((f.width, f.height), (300, 260));
    assert_eq!(f.rgba.len(), 300 * 260 * 4);
    for y in 0..260u32 {
        for x in 0..300u32 {
            let i = ((y * 300 + x) * 4) as usize;
            assert_eq!(
                &f.rgba[i..i + 4],
                &[(x % 256) as u8, (y % 256) as u8, (n % 256) as u8, 255],
                "t={t}: pixel ({x},{y}), expected frame {n}"
            );
        }
    }
}

#[test]
fn m_pat_probe() {
    let p = pat(R2997, Some(3 * S));
    assert_eq!(
        p.probe("some/where/thing.xyz").unwrap(),
        MediaSpec {
            name: "some/where/thing.xyz".into(),
            path: "some/where/thing.xyz".into(),
            duration: Some(3 * S),
            video: Some(VideoInfo {
                width: 300,
                height: 260,
                fps: R2997
            }),
            audio: Some(AudioInfo {
                sample_rate: 48000,
                channels: 2
            }),
        }
    );
    let p = pat(R30, None);
    let m = p.probe("x").unwrap();
    assert_eq!(m.duration, None);
    assert_eq!(m.name, "x");
    assert_eq!(m.video.unwrap().fps, R30);
}

#[test]
fn m_pat_video_frame_30fps() {
    let p = pat(R30, Some(20 * S));
    assert_pattern(&p, 0, 0);
    assert_pattern(&p, t30(7), 7);
    assert_pattern(&p, t30(8) - 1, 7);
    assert_pattern(&p, t30(8), 8);
    assert_pattern(&p, t30(300), 300 % 256); // n wraps: 44
    assert_pattern(&p, t30(511) + 5, 255);
    assert_pattern(&p, -5, 0);
    assert_pattern(&p, -S, 0);
}

#[test]
fn m_pat_video_frame_30000_1001() {
    let p = pat(R2997, None);
    // Q4 example: frame 1 -> 23,543,520; frame 30 -> 706,305,600.
    assert_pattern(&p, 23_543_520, 1);
    assert_pattern(&p, 23_543_519, 0);
    assert_pattern(&p, 706_305_600, 30);
    assert_pattern(&p, 706_305_599, 29);
    assert_pattern(&p, S, 29); // 1 s is inside frame 29 (starts 682,762,080)
}

#[test]
fn m_pat_audio_values_and_length() {
    let p = pat(R30, Some(10 * S));
    // 1 s at 48000 -> 48000 frames, L = 0.5, R = 0.25 exactly.
    let v = p.audio("x", 0, S, 48000, "").unwrap();
    assert_eq!(v.len(), 2 * 48000);
    for f in v.chunks_exact(2) {
        assert_eq!(f, &[0.5f32, 0.25]);
    }
    // 1/3 s at 44100 -> 14700 frames; start and filter ignored.
    let v = p
        .audio("x", 7 * S, 235_200_000, 44100, "volume=0.1")
        .unwrap();
    assert_eq!(v.len(), 2 * 14700);
    assert!(v.chunks_exact(2).all(|f| f == [0.5f32, 0.25]));
    // 2.5 frames rounds half away from zero -> 3 frames.
    let v = p
        .audio("x", 0, 36_750, 48000, "not a valid filter [[")
        .unwrap();
    assert_eq!(v, vec![0.5, 0.25, 0.5, 0.25, 0.5, 0.25]);
    // 2.4 frames -> 2.
    let v = p.audio("x", 0, 35_280, 48000, "").unwrap();
    assert_eq!(v, vec![0.5, 0.25, 0.5, 0.25]);
    // 1 flick -> 0 frames.
    assert_eq!(p.audio("x", 0, 1, 48000, "").unwrap(), Vec::<f32>::new());
}
