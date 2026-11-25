//! Queries: Q1-Q5, P1, Project::new and the lookup helpers.
mod common;
use common::*;
use std::collections::BTreeMap;

const NTSC: Rational = Rational {
    num: 30000,
    den: 1001,
};
const ELEVEN: Rational = Rational { num: 11, den: 1 };

#[test]
fn project_new_is_empty_with_next_id_1() {
    let p = Project::new("My film");
    assert_eq!(p.name, "My film");
    assert_eq!(p.next_id, 1);
    assert!(p.media.is_empty() && p.sequences.is_empty() && p.effects.is_empty());
    assert!(p.validate().is_ok());
}

#[test]
fn lookups_sequence_track_clip() {
    let Base {
        mut fx, m, seq, v2, ..
    } = base();
    let c = fx.clip(v2, m, 0, S, 0);
    let p = fx.p.clone();
    assert_eq!(p.sequence(seq).map(|s| s.id), Some(seq));
    assert!(p.sequence(v2).is_none(), "a track id is not a sequence");
    assert!(p.sequence(999).is_none());
    let (s, t) = p.track(v2).expect("track");
    assert_eq!((s.id, t.id), (seq, v2));
    assert!(p.track(seq).is_none());
    let (s, t, cl) = p.clip(c).expect("clip");
    assert_eq!((s.id, t.id, cl.id), (seq, v2, c));
    assert!(p.clip(v2).is_none());
    assert!(p.clip(999).is_none());
}

// --- Q1 ---

#[test]
fn q1_duration_of_empty_sequence_is_zero() {
    let Base { fx, seq, .. } = base();
    assert_eq!(sq(&fx.p, seq).duration(), 0);
}

#[test]
fn q1_duration_is_max_clip_end_over_all_tracks() {
    let Base {
        mut fx,
        m,
        seq,
        v1,
        v2,
        a1,
        ..
    } = base();
    fx.clip(v1, m, 0, 3 * S, 0);
    fx.clip(v2, m, S, 6 * S, 0); // ends at 7 s, not on the last track
    fx.clip(a1, m, 2 * S, 3 * S, 0);
    assert_eq!(sq(&fx.p, seq).duration(), 7 * S);
}

// --- Q2 ---

#[test]
fn q2_no_keyframes_returns_value() {
    let p = param(7.0, vec![]);
    assert_eq!(p.eval(0), 7.0);
    assert_eq!(p.eval(-5 * S), 7.0);
    assert_eq!(p.eval(123 * S), 7.0);
}

#[test]
fn q2_clamps_outside_keyframes_and_ignores_value() {
    let p = param(
        7.0,
        vec![kf(S, 10.0, Easing::Linear), kf(3 * S, 30.0, Easing::Linear)],
    );
    assert_eq!(p.eval(0), 10.0);
    assert_eq!(p.eval(S), 10.0);
    assert_eq!(p.eval(3 * S), 30.0);
    assert_eq!(p.eval(10 * S), 30.0);
}

#[test]
fn q2_linear() {
    let p = param(
        0.0,
        vec![kf(S, 10.0, Easing::Linear), kf(3 * S, 30.0, Easing::Linear)],
    );
    assert_approx(p.eval(2 * S), 20.0);
    assert_approx(p.eval(S + S / 2), 15.0);
}

#[test]
fn q2_hold() {
    let p = param(
        0.0,
        vec![kf(S, 10.0, Easing::Hold), kf(3 * S, 30.0, Easing::Linear)],
    );
    assert_eq!(p.eval(2 * S), 10.0);
    assert_eq!(p.eval(3 * S - 1), 10.0);
    assert_eq!(p.eval(3 * S), 30.0);
}

#[test]
fn q2_ease_in_out_curves() {
    let curve = |e: Easing| param(0.0, vec![kf(0, 0.0, e), kf(4 * S, 100.0, Easing::Linear)]);
    // EaseIn u^2
    assert_approx(curve(Easing::EaseIn).eval(S), 6.25);
    assert_approx(curve(Easing::EaseIn).eval(2 * S), 25.0);
    // EaseOut 1-(1-u)^2
    assert_approx(curve(Easing::EaseOut).eval(S), 43.75);
    assert_approx(curve(Easing::EaseOut).eval(2 * S), 75.0);
    // EaseInOut 3u^2-2u^3
    assert_approx(curve(Easing::EaseInOut).eval(S), 15.625);
    assert_approx(curve(Easing::EaseInOut).eval(2 * S), 50.0);
    assert_approx(curve(Easing::EaseInOut).eval(3 * S), 84.375);
}

#[test]
fn q2_easing_comes_from_k0_not_k1() {
    let p = param(
        0.0,
        vec![kf(0, 0.0, Easing::EaseIn), kf(4 * S, 100.0, Easing::Hold)],
    );
    assert_approx(p.eval(2 * S), 25.0);
}

#[test]
fn q2_three_keyframes_pick_the_right_segment() {
    let p = param(
        0.0,
        vec![
            kf(0, 0.0, Easing::Linear),
            kf(2 * S, 10.0, Easing::EaseIn),
            kf(4 * S, 30.0, Easing::Linear),
        ],
    );
    assert_approx(p.eval(S), 5.0);
    assert_eq!(p.eval(2 * S), 10.0);
    assert_approx(p.eval(3 * S), 15.0); // 10 + 20 * 0.5^2
}

// --- Q3 ---

fn q3_clip(reverse: bool) -> Clip {
    let mut c = mk_clip(1, media_src(1), 2 * S, 4 * S, S);
    c.speed = 2.0;
    c.reverse = reverse;
    c
}

#[test]
fn q3_source_time_forward() {
    let c = q3_clip(false);
    assert_eq!(c.source_time(2 * S), S);
    assert_eq!(c.source_time(3 * S), 3 * S);
    assert_eq!(c.source_time(6 * S), 9 * S);
    assert_eq!(c.source_time(0), -3 * S, "`at` outside the clip is allowed");
}

#[test]
fn q3_source_time_reversed() {
    let c = q3_clip(true);
    assert_eq!(c.source_time(2 * S), 9 * S);
    assert_eq!(c.source_time(3 * S), 7 * S);
    assert_eq!(c.source_time(6 * S), S);
}

#[test]
fn q3_source_time_rounds_half_away_from_zero() {
    let mut c = mk_clip(1, media_src(1), 0, 10, 100);
    c.speed = 1.5;
    assert_eq!(c.source_time(1), 102); // round(1.5) = 2
    assert_eq!(c.source_time(3), 105); // round(4.5) = 5
    assert_eq!(c.source_time(-1), 98); // round(-1.5) = -2
    c.reverse = true;
    assert_eq!(c.source_time(9), 102); // round((10-9)*1.5) = 2
}

// --- Q4 ---

#[test]
fn q4_frame_to_time_spec_examples() {
    assert_eq!(frame_to_time(1, NTSC), 23_543_520);
    assert_eq!(frame_to_time(30, NTSC), 706_305_600);
    assert_eq!(frame_to_time(1, FPS24), 29_400_000);
    assert_eq!(frame_to_time(0, FPS24), 0);
}

#[test]
fn q4_frame_to_time_floors() {
    assert_eq!(frame_to_time(1, ELEVEN), 64_145_454);
    assert_eq!(frame_to_time(2, ELEVEN), 128_290_909);
    assert_eq!(frame_to_time(1_000_000_000, ELEVEN), 64_145_454_545_454_545);
    assert_eq!(
        frame_to_time(-1, ELEVEN),
        -64_145_455,
        "floor, not truncation"
    );
}

#[test]
fn q4_frame_to_time_no_overflow() {
    // n * s * den = 7.06e22 overflows i64; the result fits.
    assert_eq!(
        frame_to_time(100_000_000_000, NTSC),
        2_354_352_000_000_000_000
    );
}

#[test]
fn q4_time_to_frame_is_largest_frame_not_after_t() {
    assert_eq!(time_to_frame(0, FPS24), 0);
    assert_eq!(time_to_frame(29_399_999, FPS24), 0);
    assert_eq!(time_to_frame(29_400_000, FPS24), 1);
    assert_eq!(time_to_frame(706_305_599, NTSC), 29);
    assert_eq!(time_to_frame(706_305_600, NTSC), 30);
    assert_eq!(time_to_frame(64_145_453, ELEVEN), 0);
    assert_eq!(time_to_frame(64_145_454, ELEVEN), 1);
    assert_eq!(time_to_frame(128_290_908, ELEVEN), 1);
    assert_eq!(time_to_frame(128_290_909, ELEVEN), 2);
    assert_eq!(
        time_to_frame(2_354_352_000_000_000_000, NTSC),
        100_000_000_000
    );
    assert_eq!(time_to_frame(-1, FPS24), -1);
}

// --- Q5 / P1 ---

#[test]
fn q5_param_stored_default_unknown() {
    let mut c = mk_clip(1, media_src(1), 0, 4 * S, 0);
    c.params.insert(
        "opacity".to_string(),
        param(
            0.3,
            vec![kf(0, 0.0, Easing::Linear), kf(2 * S, 1.0, Easing::Linear)],
        ),
    );
    c.params.insert("rotation".to_string(), param(45.0, vec![]));
    assert_approx(c.param("opacity", S).unwrap(), 0.5);
    assert_eq!(c.param("opacity", 5 * S), Some(1.0));
    assert_eq!(c.param("rotation", 3 * S), Some(45.0));
    assert_eq!(c.param("scale_x", 0), Some(1.0));
    assert_eq!(c.param("volume", 0), Some(0.0));
    assert_eq!(c.param("bogus", 0), None);
}

const P1_EXPECTED: &[(&str, f64)] = &[
    ("position_x", 0.0),
    ("position_y", 0.0),
    ("scale_x", 1.0),
    ("scale_y", 1.0),
    ("rotation", 0.0),
    ("anchor_x", 0.0),
    ("anchor_y", 0.0),
    ("opacity", 1.0),
    ("crop_left", 0.0),
    ("crop_top", 0.0),
    ("crop_right", 0.0),
    ("crop_bottom", 0.0),
    ("volume", 0.0),
    ("pan", 0.0),
];

/// Passes against the stubs (pure constant).
#[test]
fn p1_clip_params_exact_list() {
    assert_eq!(CLIP_PARAMS, P1_EXPECTED);
}

#[test]
fn p1_q5_defaults_for_every_clip_param() {
    let c = Clip {
        params: BTreeMap::new(),
        ..mk_clip(1, media_src(1), 0, S, 0)
    };
    for (name, default) in P1_EXPECTED {
        assert_eq!(c.param(name, S / 2), Some(*default), "{name}");
    }
}
