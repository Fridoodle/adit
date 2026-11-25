//! SPEC A-1..A-7: `mix_audio`. 48 kHz sequences; one sample frame = SF = 14_700 flicks.
//! Constant fake sources: "a" = (0.5, 0.25), "b" = (0.4, 0.8) unless stated.

mod common;
use adit_core::*;
use adit_media::MediaError;
use adit_render::{RenderError, mix_audio};
use common::*;

const TOL: f32 = 1e-4;

fn ab() -> Fake {
    const_audio(&[("a", 0.5, 0.25), ("b", 0.4, 0.8), ("av", 0.5, 0.25)], 1.0)
}

/// Sequence with `n` audio tracks and media "a" and "b".
fn seq(n: usize) -> (B, Id, Vec<Id>, Id, Id) {
    let mut b = B::new();
    let s = b.seq(16, 16);
    let t = (0..n).map(|_| b.atrack(s)).collect();
    let ma = b.media("a", None, true);
    let mb = b.media("b", None, true);
    (b, s, t, ma, mb)
}

// ------------------------------------------------------------------ A-1

#[test]
fn a1_exact_sample_counts() {
    let (b, s, ..) = seq(1);
    let cases: [(Time, usize); 6] = [
        (S, 96_000),       // 1 s * 48000 = 48000 frames
        (ms(520), 49_920), // .52 * 48000 = 24960 frames
        (7_350, 2),        // 7350*48000/S = 0.5 → round half away → 1 frame
        (7_349, 0),        // 0.49993 → 0
        (10_000, 2),       // 0.68 → 1
        (0, 0),
    ];
    for (dur, n) in cases {
        let v = mix(&b.p, s, ms(250), dur, &ab());
        assert_eq!(v.len(), n, "A-1 duration {dur} flicks → {n} samples");
        assert!(v.iter().all(|x| *x == 0.0), "A-1 empty sequence is silence");
    }
}

#[test]
fn a1_count_uses_sequence_sample_rate() {
    let (mut b, s, ..) = seq(1);
    b.seq_mut(s).sample_rate = 44_100;
    let v = mix(&b.p, s, 0, S, &ab());
    assert_eq!(v.len(), 88_200, "A-1 1 s at 44.1 kHz");
}

#[test]
fn a1_empty_space_is_silence() {
    let (mut b, s, t, ma, _) = seq(1);
    b.media_clip(t[0], ms(250), ms(250), ma); // frames [12000, 24000)
    let v = mix(&b.p, s, 0, S, &ab());
    assert_eq!(v.len(), 96_000);
    assert_lr(&v, 0, 0.0, 0.0, TOL, "A-1 before clip");
    assert_lr(&v, 11_999, 0.0, 0.0, TOL, "A-1 just before clip");
    assert_lr(&v, 12_000, 0.5, 0.25, TOL, "A-1 clip first frame");
    assert_lr(&v, 23_999, 0.5, 0.25, TOL, "A-1 clip last frame");
    assert_lr(&v, 24_000, 0.0, 0.0, TOL, "A-1 clip end exclusive");
    assert_lr(&v, 47_999, 0.0, 0.0, TOL, "A-1 after clip");
}

#[test]
fn a1_mix_window_offset() {
    let (mut b, s, t, ma, _) = seq(1);
    b.media_clip(t[0], 0, S, ma);
    let v = mix(&b.p, s, ms(500), ms(750), &ab());
    assert_eq!(v.len(), 72_000, ".75 s → 36000 frames");
    assert_lr(&v, 0, 0.5, 0.25, TOL, "A-1 window inside clip");
    assert_lr(
        &v,
        23_999,
        0.5,
        0.25,
        TOL,
        "A-1 last frame of clip (t = 1s - 1 frame)",
    );
    assert_lr(&v, 24_000, 0.0, 0.0, TOL, "A-1 past clip end");
}

#[test]
fn a1_only_audio_tracks_contribute() {
    let mut b = B::new();
    let s = b.seq(16, 16);
    let v = b.vtrack(s);
    b.atrack(s);
    let m = b.media("av", Some((16, 16)), true);
    b.media_clip(v, 0, S, m);
    let out = mix(&b.p, s, 0, S, &ab());
    assert_eq!(out.len(), 96_000);
    assert!(
        out.iter().all(|x| *x == 0.0),
        "A-1 video track audio is not mixed"
    );
}

#[test]
fn a1_mute_solo_disabled() {
    let build = |f: &dyn Fn(&mut B, &[Id], Id)| {
        let (mut b, s, t, ma, mb) = seq(2);
        b.media_clip(t[0], 0, S, ma);
        let c = b.media_clip(t[1], 0, S, mb);
        f(&mut b, &t, c);
        let v = mix(&b.p, s, 0, S, &ab());
        lr(&v, 1000)
    };
    let close = |got: (f32, f32), want: (f32, f32), what: &str| {
        assert!(
            (got.0 - want.0).abs() <= TOL && (got.1 - want.1).abs() <= TOL,
            "{what}: got {got:?}, want {want:?}"
        );
    };
    close(
        build(&|_, _, _| {}),
        (0.9, 1.05),
        "A-1/A-4 both tracks sum (.5+.4, .25+.8)",
    );
    close(
        build(&|b, t, _| b.track_mut(t[1]).muted = true),
        (0.5, 0.25),
        "A-1 muted track skipped",
    );
    close(
        build(&|b, t, _| b.track_mut(t[1]).solo = true),
        (0.4, 0.8),
        "A-1 solo: only solo tracks",
    );
    close(
        build(&|b, t, _| {
            b.track_mut(t[0]).solo = true;
            b.track_mut(t[1]).solo = true;
        }),
        (0.9, 1.05),
        "A-1 two solo tracks both play",
    );
    close(
        build(&|b, _, c| b.clip_mut(c).enabled = false),
        (0.5, 0.25),
        "A-1 disabled clip skipped",
    );
}

// ------------------------------------------------------------------ A-2

fn filters_for(speed: f64, reverse: bool, user: &str) -> Vec<String> {
    let (mut b, s, t, ma, _) = seq(1);
    let c = b.media_clip(t[0], 0, ms(500), ma);
    b.clip_mut(c).speed = speed;
    b.clip_mut(c).reverse = reverse;
    b.clip_mut(c).audio_filter = user.into();
    let fake = ab();
    let v = mix(&b.p, s, 0, S, &fake);
    assert_eq!(v.len(), 96_000);
    let calls = fake.audio_calls();
    assert!(!calls.is_empty(), "A-2 backend audio() called");
    for call in &calls {
        assert_eq!(call.path, "a");
        assert_eq!(
            call.sample_rate, SR,
            "A-2 fetched at the sequence sample rate"
        );
    }
    calls.into_iter().map(|c| c.filter).collect()
}

fn steps(filter: &str) -> Vec<String> {
    if filter.is_empty() {
        return vec![];
    }
    filter.split(',').map(|s| s.trim().to_string()).collect()
}

fn atempo_factors(steps: &[String]) -> Vec<f64> {
    steps
        .iter()
        .filter_map(|s| s.strip_prefix("atempo="))
        .map(|v| v.parse::<f64>().expect("atempo value"))
        .collect()
}

#[test]
fn a2_filter_chain_plain_reverse_user() {
    for f in filters_for(1.0, false, "") {
        assert_eq!(f, "", "A-2 no filter for a plain clip");
    }
    for f in filters_for(1.0, true, "") {
        assert_eq!(f, "areverse", "A-2 reverse → areverse");
    }
    for f in filters_for(1.0, false, "volume=0.5") {
        assert_eq!(f, "volume=0.5", "A-2 user filter alone");
    }
}

#[test]
fn a2_filter_chain_order_and_atempo() {
    for f in filters_for(2.0, true, "volume=0.5,highpass=f=200") {
        let st = steps(&f);
        assert_eq!(
            st.first().map(String::as_str),
            Some("areverse"),
            "A-2 areverse first: {f}"
        );
        assert_eq!(
            &st[st.len() - 2..],
            ["volume=0.5", "highpass=f=200"],
            "A-2 user filter last: {f}"
        );
        let mid = &st[1..st.len() - 2];
        assert!(
            !mid.is_empty() && mid.iter().all(|s| s.starts_with("atempo=")),
            "A-2 atempo between: {f}"
        );
        let prod: f64 = atempo_factors(mid).iter().product();
        assert!(
            (prod - 2.0).abs() < 1e-6,
            "A-2 atempo product = speed 2: {f}"
        );
    }
}

#[test]
fn a2_atempo_chained_within_ffmpeg_range() {
    for speed in [0.25, 0.1, 3.0, 4.0] {
        for f in filters_for(speed, false, "") {
            let st = steps(&f);
            assert!(
                st.iter().all(|s| s.starts_with("atempo=")),
                "A-2 only atempo steps: {f}"
            );
            let fac = atempo_factors(&st);
            assert!(
                fac.iter().all(|x| (0.5..=100.0).contains(x)),
                "A-2 each step in [0.5, 100]: {f}"
            );
            let prod: f64 = fac.iter().product();
            assert!(
                (prod - speed).abs() < 1e-6 * speed.max(1.0),
                "A-2 product {prod} = speed {speed}: {f}"
            );
        }
    }
}

#[test]
fn a2_source_range_requested() {
    let (mut b, s, t, ma, _) = seq(1);
    let c = b.media_clip(t[0], ms(500), ms(500), ma);
    b.clip_mut(c).source_in = S;
    b.clip_mut(c).speed = 2.0;
    let fake = ab();
    let v = mix(&b.p, s, 0, S, &fake);
    // Source range [source_in, source_in + round(0.5 s * 2)) = [1 s, 2 s).
    let calls = fake.audio_calls();
    let lo = calls.iter().map(|c| c.start).min().expect("audio() called");
    let hi = calls.iter().map(|c| c.start + c.duration).max().unwrap();
    assert!((lo - S).abs() <= SF, "A-2 range starts at source_in: {lo}");
    assert!(
        (hi - 2 * S).abs() <= SF,
        "A-2 range ends at source_in + round(dur*speed): {hi}"
    );
    // Fake emulates atempo → exactly the clip length of constant samples.
    assert_lr(&v, 23_999, 0.0, 0.0, TOL, "A-2 silence before clip");
    assert_lr(&v, 24_000, 0.5, 0.25, TOL, "A-2 clip start");
    assert_lr(&v, 47_999, 0.5, 0.25, TOL, "A-2 clip end");
}

#[test]
fn a2_source_in_offsets_samples() {
    let (mut b, s, t, ma, _) = seq(1);
    let c = b.media_clip(t[0], ms(500), ms(500), ma);
    b.clip_mut(c).source_in = 2 * S;
    // Ramp: L = source frame / 1e5. Mix frame 24000 + k ↔ source frame 96000 + k.
    let v = mix(&b.p, s, 0, S, &ramp_audio("a"));
    assert_lr(
        &v,
        24_000,
        0.96,
        0.0,
        TOL,
        "A-2 first clip frame = source_in",
    );
    assert_lr(&v, 24_100, 0.961, 0.0, TOL, "A-2 frame 100 into clip");
    assert_lr(&v, 47_999, 1.19999, 0.0, TOL, "A-2 last clip frame");
    assert_lr(&v, 23_999, 0.0, 0.0, TOL, "A-2 before clip");
}

#[test]
fn a2_result_padded_or_cut_to_clip_length() {
    // Backend returns half the requested length → second half of the clip is silence.
    let (mut b, s, t, ma, _) = seq(1);
    b.media_clip(t[0], 0, ms(500), ma);
    let short = const_audio(&[("a", 0.5, 0.25)], 0.5);
    let v = mix(&b.p, s, 0, S, &short);
    assert_eq!(v.len(), 96_000);
    assert_lr(&v, 11_999, 0.5, 0.25, TOL, "A-2 delivered part");
    assert_lr(&v, 12_001, 0.0, 0.0, TOL, "A-2 zero-padded");
    assert_lr(&v, 23_999, 0.0, 0.0, TOL, "A-2 zero-padded");
    // Backend returns twice the length → cut at the clip end.
    let long = const_audio(&[("a", 0.5, 0.25)], 2.0);
    let v = mix(&b.p, s, 0, S, &long);
    assert_eq!(v.len(), 96_000);
    assert_lr(&v, 23_999, 0.5, 0.25, TOL, "A-2 clip end");
    assert_lr(&v, 24_000, 0.0, 0.0, TOL, "A-2 cut at clip end");
    assert_lr(&v, 36_000, 0.0, 0.0, TOL, "A-2 cut at clip end");
}

// ------------------------------------------------------------------ A-3

fn one_clip(params: &[(&str, f64)]) -> Vec<f32> {
    let (mut b, s, t, ma, _) = seq(1);
    let c = b.media_clip(t[0], 0, S, ma);
    for (k, v) in params {
        b.set(c, k, *v);
    }
    mix(&b.p, s, 0, S, &ab())
}

#[test]
fn a3_volume_and_pan_spec_examples() {
    // 10^(-6.0206/20) = 0.5 → (.25, .125).
    let v = one_clip(&[("volume", -6.0206)]);
    assert_lr(&v, 100, 0.25, 0.125, 1e-4, "A-3 volume -6.0206 dB");
    // pan 1: L gain min(1, 0) = 0, R gain min(1, 2) = 1.
    let v = one_clip(&[("pan", 1.0)]);
    assert_lr(&v, 100, 0.0, 0.25, TOL, "A-3 pan 1");
    // pan -.5: L min(1, 1.5) = 1, R min(1, .5) = .5 → (.5, .125).
    let v = one_clip(&[("pan", -0.5)]);
    assert_lr(&v, 100, 0.5, 0.125, TOL, "A-3 pan -.5");
    // volume +6.0206 dB → gain 2 → (1.0, .5).
    let v = one_clip(&[("volume", 6.0206)]);
    assert_lr(&v, 100, 1.0, 0.5, 1e-4, "A-3 volume +6 dB");
}

#[test]
fn a3_volume_keyframes_per_sample_at_local_time() {
    let (mut b, s, t, ma, _) = seq(1);
    let c = b.media_clip(t[0], ms(500), S, ma);
    b.keys(
        c,
        "volume",
        &[(0, -6.0206, Easing::Linear), (S, 0.0, Easing::Linear)],
    );
    let v = mix(&b.p, s, 0, ms(1500), &ab());
    // Mix frame 24000 = local 0: gain .5 → L .25.
    assert_lr(&v, 24_000, 0.25, 0.125, 1e-3, "A-3 fade start");
    // Local .5 s (frame 48000): volume -3.0103 dB → gain .70711 → (.35355, .17678).
    // (Interpolating gain linearly would give .75 * .5 = .375.)
    assert_lr(
        &v,
        48_000,
        0.353_55,
        0.176_78,
        1e-3,
        "A-3 fade midpoint in dB",
    );
    // Local .25 s (frame 36000): -4.51545 dB → gain .59460 → (.29730, .14865).
    assert_lr(&v, 36_000, 0.297_30, 0.148_65, 1e-3, "A-3 fade quarter");
    // Last frame (local 1 s - 1 frame): ≈ 0 dB → (.5, .25).
    assert_lr(&v, 71_999, 0.5, 0.25, 1e-3, "A-3 fade end");
}

// ------------------------------------------------------------------ A-4

#[test]
fn a4_tracks_sum_without_limiter() {
    let (mut b, s, t, ma, _) = seq(3);
    b.media_clip(t[0], 0, S, ma);
    b.media_clip(t[1], 0, S, ma);
    let v = mix(&b.p, s, 0, S, &ab());
    assert_lr(&v, 500, 1.0, 0.5, TOL, "A-4 two clips: L 1.0, R 0.5");
    b.media_clip(t[2], 0, S, ma);
    let v = mix(&b.p, s, 0, S, &ab());
    assert_lr(&v, 500, 1.5, 0.75, TOL, "A-4 three clips: no limiter");
}

// ------------------------------------------------------------------ A-5

#[test]
fn a5_two_sided_equal_power_crossfade() {
    let (mut b, s, t, ma, mb) = seq(1);
    let l = b.media_clip(t[0], 0, S, ma);
    let r = b.media_clip(t[0], S, S, mb);
    b.transition(
        t[0],
        Some(l),
        Some(r),
        "definitely_not_an_effect",
        ms(500),
        &[],
    );
    // Window [.75 s, 1.25 s).
    let v = mix(&b.p, s, 0, 2 * S, &ab());
    assert_eq!(v.len(), 192_000);
    assert_lr(&v, 24_000, 0.5, 0.25, TOL, "A-5 before window: left only");
    assert_lr(&v, 36_000, 0.5, 0.25, 1e-3, "A-5 p = 0");
    // p = .25 (frame 42000 = .875 s): cos(π/8) = .92388, sin(π/8) = .38268
    // L = .5*.92388 + .4*.38268 = .61501; R = .25*.92388 + .8*.38268 = .53712.
    assert_lr(&v, 42_000, 0.615_01, 0.537_12, 1e-3, "A-5 p = .25");
    // p = .5 (frame 48000 = 1 s): both .70711 → L .63640, R .74246.
    assert_lr(&v, 48_000, 0.636_40, 0.742_46, 1e-3, "A-5 p = .5");
    assert_lr(&v, 60_000, 0.4, 0.8, TOL, "A-5 after window: right only");
}

#[test]
fn a5_left_clip_sampled_past_its_end() {
    let (mut b, s, t, ma, mb) = seq(1);
    let l = b.media_clip(t[0], 0, S, ma);
    let r = b.media_clip(t[0], S, S, mb);
    b.transition(t[0], Some(l), Some(r), "xfade", ms(500), &[]);
    // ramp on "a" (L = source frame / 1e5), "b" silent.
    // 1.1 s = frame 52800; p = (1.1 - .75)/.5 = .7 → cos(.35π) = .45399 → L = .45399 * .528 = .23971.
    let v = mix(&b.p, s, 0, 2 * S, &ramp_audio("a"));
    assert_lr(
        &v,
        52_800,
        0.239_71,
        0.0,
        1e-3,
        "A-5 left clip continues past its end",
    );
    // p = .25 at .875 s: .92388 * .42 = .38803.
    assert_lr(
        &v,
        42_000,
        0.388_03,
        0.0,
        1e-3,
        "A-5 left clip inside window",
    );
}

#[test]
fn a5_one_sided_fades() {
    // Fade out: left only, window [.5 s, 1 s).
    let (mut b, s, t, ma, mb) = seq(1);
    let l = b.media_clip(t[0], 0, S, ma);
    b.transition(t[0], Some(l), None, "x", ms(500), &[]);
    let v = mix(&b.p, s, 0, 2 * S, &ab());
    assert_lr(&v, 12_000, 0.5, 0.25, TOL, "A-5 fade-out before window");
    // p = .5 at .75 s: .70711 * (.5, .25) = (.35355, .17678).
    assert_lr(
        &v,
        36_000,
        0.353_55,
        0.176_78,
        1e-3,
        "A-5 fade-out midpoint",
    );
    // Fade in: right only on clip [1 s, 2 s), window [1 s, 1.5 s).
    let r = b.media_clip(t[0], S, S, mb);
    b.track_mut(t[0]).transitions.clear();
    b.transition(t[0], None, Some(r), "x", ms(500), &[]);
    let v = mix(&b.p, s, 0, 2 * S, &ab());
    // p = .5 at 1.25 s: .70711 * (.4, .8) = (.28284, .56569).
    assert_lr(&v, 60_000, 0.282_84, 0.565_69, 1e-3, "A-5 fade-in midpoint");
    assert_lr(&v, 48_000, 0.0, 0.0, 1e-3, "A-5 fade-in p = 0 is silent");
    assert_lr(&v, 76_000, 0.4, 0.8, TOL, "A-5 after fade-in window");
}

// ------------------------------------------------------------------ A-6

#[test]
fn a6_nested_sequence_mix_from_source_in_ignoring_speed() {
    let mut b = B::new();
    let inner = b.seq(16, 16);
    let it = b.atrack(inner);
    let ma = b.media("a", None, true);
    b.media_clip(it, 0, 2 * S, ma);
    let outer = b.seq(16, 16);
    let ot = b.atrack(outer);
    let c = b.seq_clip(ot, ms(500), ms(500), inner);
    b.clip_mut(c).source_in = S;
    b.clip_mut(c).speed = 2.0;
    // Outer .625 s (frame 30000): local .125 s → inner 1.125 s (speed ignored) → source frame 54000.
    // (Applying speed 2 would give inner 1.25 s → .6.)
    let v = mix(&b.p, outer, 0, S, &ramp_audio("a"));
    assert_eq!(v.len(), 96_000);
    assert_lr(
        &v,
        30_000,
        0.54,
        0.0,
        TOL,
        "A-6 nested mix at source_in + local",
    );
    assert_lr(
        &v,
        24_000,
        0.48,
        0.0,
        TOL,
        "A-6 nested clip start = inner 1 s",
    );
    assert_lr(&v, 23_999, 0.0, 0.0, TOL, "A-6 before nested clip");
}

// ------------------------------------------------------------------ A-7

#[test]
fn a7_unfetchable_audio_is_silence() {
    let (mut b, s, t, ma, _) = seq(2);
    let bad = b.media("broken.wav", None, true);
    b.media_clip(t[0], 0, S, bad);
    b.media_clip(t[1], 0, S, ma);
    let fake = Fake::new().with_audio(|p, _, d, sr, _| match p {
        "a" => Ok([0.5f32, 0.25].repeat(n_frames(d, sr))),
        _ => Err(MediaError::Ffmpeg("boom".into())),
    });
    let v = mix(&b.p, s, 0, S, &fake);
    assert_eq!(v.len(), 96_000);
    assert_lr(
        &v,
        100,
        0.5,
        0.25,
        TOL,
        "A-7 failing clip contributes silence",
    );
}

#[test]
fn a7_missing_sequence_is_error() {
    let (b, ..) = seq(1);
    assert_eq!(
        mix_audio(&b.p, 4242, 0, S, &ab()),
        Err(RenderError::SequenceNotFound(4242))
    );
}

#[test]
fn a_mix_does_not_modify_project() {
    let (mut b, s, t, ma, mb) = seq(2);
    let l = b.media_clip(t[0], 0, S, ma);
    let r = b.media_clip(t[0], S, S, mb);
    b.transition(t[0], Some(l), Some(r), "x", ms(500), &[]);
    b.keys(
        l,
        "pan",
        &[(0, -1.0, Easing::EaseOut), (S, 1.0, Easing::Linear)],
    );
    let before = b.p.clone();
    let x = mix(&b.p, s, 0, 2 * S, &ab());
    let y = mix(&b.p, s, 0, 2 * S, &ab());
    assert_eq!(x, y, "mix is deterministic");
    assert_eq!(b.p, before, "mix does not modify the project");
}
