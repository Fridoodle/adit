//! M-AUDIO-1..3, M-PROC-1 (audio side).
//! Fixtures are ramps (value = t/4 etc.), so every sample says which source time it came from.
mod common;

use adit_media::{Ffmpeg, MediaBackend, MediaError};
use common::*;

fn ff() -> Ffmpeg {
    Ffmpeg::new().expect("Ffmpeg::new with ffmpeg/ffprobe on PATH")
}

/// SPEC M-AUDIO-1: 2 * round(duration * sr / s) samples within +-2 frames.
fn assert_len(v: &[f32], frames: usize) {
    assert_eq!(v.len() % 2, 0, "interleaved stereo must have even length");
    let got = v.len() / 2;
    assert!(
        got.abs_diff(frames) <= 2,
        "got {got} frames, expected {frames} +- 2"
    );
}

/// Checks sample frames (skipping edges for resampler ringing) against `left(t)`/`right(t)`,
/// with `t` = source seconds of output frame i = start_s + i / sr.
fn assert_ramp(
    v: &[f32],
    sr: u32,
    start_s: f64,
    left: impl Fn(f64) -> f64,
    right: impl Fn(f64) -> f64,
) {
    let n = v.len() / 2;
    assert!(n > 200, "too few samples to check: {n}");
    for i in (100..n - 100).step_by(97) {
        let t = start_s + i as f64 / sr as f64;
        let (l, r) = (v[2 * i] as f64, v[2 * i + 1] as f64);
        assert!(
            (l - left(t)).abs() < 1e-3,
            "left[{i}] = {l}, expected {}",
            left(t)
        );
        assert!(
            (r - right(t)).abs() < 1e-3,
            "right[{i}] = {r}, expected {}",
            right(t)
        );
    }
}

#[test]
fn m_audio_1_stereo_range_length_and_content() {
    timed(60, || {
        let d = TempDir::new("au-st");
        let p = ramp_stereo_wav(d.path());
        // [1 s, 1.5 s) at 48000 -> 24000 frames.
        let v = ff().audio(&p, S, S / 2, 48000, "").unwrap();
        assert_len(&v, 24000);
        assert_ramp(&v, 48000, 1.0, |t| t / 4.0, |t| -t / 8.0);
        // First frame is source t = 1 s (not 0 s): 0.25 / -0.125.
        assert!((v[0] - 0.25).abs() < 2e-3, "first left = {}", v[0]);
        assert!((v[1] + 0.125).abs() < 2e-3, "first right = {}", v[1]);
    })
}

#[test]
fn m_audio_1_resamples_to_requested_rate() {
    timed(60, || {
        let d = TempDir::new("au-rs");
        let p = ramp_stereo_wav(d.path());
        // [0.5 s, 1.5 s) at 22050 -> 22050 frames.
        let v = ff().audio(&p, S / 2, S, 22050, "").unwrap();
        assert_len(&v, 22050);
        assert_ramp(&v, 22050, 0.5, |t| t / 4.0, |t| -t / 8.0);
    })
}

#[test]
fn m_audio_1_mono_duplicated_to_both_channels() {
    timed(60, || {
        let d = TempDir::new("au-mono");
        let p = ramp_mono_wav(d.path()); // 44100 Hz mono
        // [2 s, 2.25 s) at 48000 -> 12000 frames.
        let v = ff().audio(&p, 2 * S, S / 4, 48000, "").unwrap();
        assert_len(&v, 12000);
        assert_ramp(&v, 48000, 2.0, |t| t / 4.0, |t| t / 4.0);
        for f in v.as_chunks::<2>().0 {
            assert!((f[0] - f[1]).abs() < 1e-6, "mono not duplicated: {f:?}");
        }
    })
}

#[test]
fn m_audio_1_length_is_rounded_duration() {
    timed(60, || {
        let d = TempDir::new("au-len");
        let p = ramp_stereo_wav(d.path());
        // 1/3 s at 44100 = 14700 frames exactly.
        let v = ff().audio(&p, 0, 235_200_000, 44100, "").unwrap();
        assert_len(&v, 14700);
    })
}

#[test]
fn m_audio_1_from_aac_in_mp4() {
    timed(60, || {
        let d = TempDir::new("au-aac");
        let p = av_mp4(d.path(), "av.mp4"); // 2 s 44100 Hz AAC sine, amplitude 1/8
        let v = ff().audio(&p, S / 2, S, 48000, "").unwrap();
        assert_len(&v, 48000);
        let rms = (v.iter().map(|x| (*x as f64).powi(2)).sum::<f64>() / v.len() as f64).sqrt();
        // sine amplitude 0.125, upmixed to stereo by `-ac 2` at -3 dB -> rms 0.0625.
        assert!((rms - 0.0625).abs() < 0.015, "rms {rms}");
    })
}

#[test]
fn m_audio_1_range_past_end_yields_fewer_samples() {
    timed(60, || {
        let d = TempDir::new("au-end");
        let p = ramp_stereo_wav(d.path()); // 3 s
        let ff = ff();
        // [2.5 s, 3.5 s): only 0.5 s exists -> 24000 frames, starting at source 2.5 s.
        let v = ff.audio(&p, 5 * S / 2, S, 48000, "").unwrap();
        assert_len(&v, 24000);
        assert!((v[0] - 0.625).abs() < 2e-3, "first left = {}", v[0]);
        // Entirely past the end: Ok, nothing (allow the +-2 frame slack).
        let v = ff.audio(&p, 5 * S, S, 48000, "").unwrap();
        assert!(v.len() <= 4, "past end returned {} samples", v.len());
    })
}

#[test]
fn m_audio_2_volume_filter_halves_samples() {
    timed(60, || {
        let d = TempDir::new("au-vol");
        let p = ramp_stereo_wav(d.path());
        let v = ff().audio(&p, S, S / 2, 48000, "volume=0.5").unwrap();
        assert_len(&v, 24000);
        assert_ramp(&v, 48000, 1.0, |t| t / 8.0, |t| -t / 16.0);
    })
}

#[test]
fn m_audio_2_atempo_halves_length_after_cut() {
    timed(60, || {
        let d = TempDir::new("au-tempo");
        let p = ramp_stereo_wav(d.path());
        // [1 s, 2 s) at 2x tempo -> ~0.5 s = 24000 frames.
        let v = ff().audio(&p, S, S, 48000, "atempo=2.0").unwrap();
        let n = v.len() / 2;
        // shortcut: atempo's own edge handling yields ~1.6% extra; allow 3%.
        assert!(
            n.abs_diff(24000) <= 720,
            "atempo=2.0 gave {n} frames, expected ~24000"
        );
        // Filter applied after the cut: output middle is source 1.5 s (0.375), not 3 s.
        let mid = v[2 * 12000] as f64;
        assert!(
            (mid - 0.375).abs() < 0.01,
            "middle sample {mid}, expected ~0.375"
        );
        assert!((v[0] as f64 - 0.25).abs() < 0.01, "first sample {}", v[0]);
    })
}

#[test]
fn m_audio_2_invalid_filter_is_ffmpeg_error() {
    timed(60, || {
        let d = TempDir::new("au-badf");
        let p = ramp_stereo_wav(d.path());
        let r = ff().audio(&p, 0, S, 48000, "this_filter_does_not_exist=3");
        assert!(matches!(r, Err(MediaError::Ffmpeg(_))), "{r:?}");
    })
}

#[test]
fn m_audio_3_file_without_audio_is_empty() {
    timed(60, || {
        let d = TempDir::new("au-none");
        let ff = ff();
        let v = id_video(d.path(), "id30.mov", "30", 30);
        assert_eq!(ff.audio(&v, 0, S, 48000, "").unwrap(), Vec::<f32>::new());
        let png = still_png(d.path());
        assert_eq!(ff.audio(&png, 0, S, 48000, "").unwrap(), Vec::<f32>::new());
    })
}

#[test]
fn m_proc_1_audio_path_with_space_and_non_ascii() {
    timed(60, || {
        let d = TempDir::new("au-uni");
        let sub = d.unicode_subdir();
        let p = s(&sub.join("ton ö.wav"));
        ffmpeg(&[
            "-f",
            "lavfi",
            "-i",
            "aevalsrc='t/4|-t/8':s=48000:d=1",
            "-c:a",
            "pcm_f32le",
            &p,
        ]);
        let v = ff().audio(&p, 0, S / 2, 48000, "").unwrap();
        assert_len(&v, 24000);
        assert_ramp(&v, 48000, 0.0, |t| t / 4.0, |t| -t / 8.0);
    })
}

#[test]
fn audio_missing_file_is_not_found() {
    // Spec gap: M-PROBE-3 names NotFound for probe; the same error is the sensible reading here.
    timed(30, || {
        let d = TempDir::new("au-missing");
        let r = ff().audio(&d.file("nope.wav"), 0, S, 48000, "");
        assert!(matches!(r, Err(MediaError::NotFound(_))), "{r:?}");
    })
}
