//! SPEC V-9 (video transitions) plus the transition side of E-1/E-3.
//! Standard layout: clip L [0,2s), clip R [2s,4s), two-sided 1 s transition → window [1.5s, 2.5s).

mod common;
use adit_core::*;
use adit_media::{Frame, MediaError};
use adit_render::RenderError;
use common::*;

/// Two solids on one track of a `w`x`w` sequence with a two-sided transition.
fn solids(w: u32, effect: &str, params: &[(&str, f64)]) -> (B, Id, Id, Id, Id) {
    let mut b = B::new();
    let s = b.seq(w, w);
    let v = b.vtrack(s);
    let l = b.solid(v, 0, 2 * S, RED);
    let r = b.solid(v, 2 * S, 2 * S, BLUE);
    b.transition(v, Some(l), Some(r), effect, S, params);
    (b, s, v, l, r)
}

#[test]
fn v9_cross_dissolve_spec_example() {
    let (b, s, ..) = solids(16, "cross_dissolve", &[]);
    let cases: [(Time, [u8; 3]); 6] = [
        (2 * S, [128, 0, 128]),   // p = 0.5 → 127.5 each
        (ms(1750), [191, 0, 64]), // p = 0.25 → .75*255 = 191.25, .25*255 = 63.75
        (ms(1400), [255, 0, 0]),  // before window
        (ms(1500), [255, 0, 0]),  // p = 0
        (ms(2250), [64, 0, 191]), // p = 0.75
        (ms(2500), [0, 0, 255]),  // window end is exclusive
    ];
    for (t, rgb) in cases {
        let f = render(&b.p, s, t, &Fake::new());
        assert_px(&f, 8, 8, rgb, &format!("V-9 cross_dissolve t={t}"));
    }
}

#[test]
fn v9_dip_to_black() {
    let (b, s, ..) = solids(16, "dip_to_black", &[]);
    let cases: [(Time, [u8; 3]); 4] = [
        (ms(1600), [204, 0, 0]), // p=.1: mix(A, black, .2) = .8 → 204
        (ms(1750), [128, 0, 0]), // p=.25: mix(A, black, .5)
        (2 * S, [0, 0, 0]),      // p=.5: mix(black, B, 0)
        (ms(2250), [0, 0, 128]), // p=.75: mix(black, B, .5)
    ];
    for (t, rgb) in cases {
        let f = render(&b.p, s, t, &Fake::new());
        assert_px(&f, 8, 8, rgb, &format!("V-9 dip_to_black t={t}"));
    }
}

#[test]
fn v9_wipe_boundary() {
    let (b, s, ..) = solids(64, "wipe", &[]);
    // p = .25: B where uv.x < .25 → pixel centres (x+.5)/64 < .25 → x <= 15.
    let f = render(&b.p, s, ms(1750), &Fake::new());
    for x in [0, 2, 13] {
        assert_px(&f, x, 32, [0, 0, 255], "V-9 wipe p=.25 left is B");
    }
    for x in [18, 40, 63] {
        assert_px(&f, x, 32, [255, 0, 0], "V-9 wipe p=.25 right is A");
    }
    // p = .75: B for x <= 47.
    let f = render(&b.p, s, ms(2250), &Fake::new());
    assert_px(&f, 45, 5, [0, 0, 255], "V-9 wipe p=.75 x=45 is B");
    assert_px(&f, 50, 60, [255, 0, 0], "V-9 wipe p=.75 x=50 is A");
}

fn halves(left: [u8; 4], right: [u8; 4]) -> Frame {
    frame_fn(64, 64, |x, _| if x < 32 { left } else { right })
}

#[test]
fn v9_push_moves_both_layers() {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v = b.vtrack(s);
    let ma = b.media("a", Some((64, 64)), false);
    let mb = b.media("b", Some((64, 64)), false);
    let l = b.media_clip(v, 0, 2 * S, ma);
    let r = b.media_clip(v, 2 * S, 2 * S, mb);
    b.transition(v, Some(l), Some(r), "push", S, &[]);
    const RD: [u8; 3] = [255, 0, 0];
    const GR: [u8; 3] = [0, 255, 0];
    const YE: [u8; 3] = [255, 255, 0];
    const WH: [u8; 3] = [255, 255, 255];
    let fake = Fake::images(vec![
        ("a", halves([255, 0, 0, 255], [0, 255, 0, 255])), // A: red | green
        ("b", halves([255, 255, 0, 255], [255, 255, 255, 255])), // B: yellow | white
    ]);
    // p = .25: uv.x < .75 → A at uv.x + .25; else B at uv.x - .75.
    //   x=8  uv .133 → A .383 red;  x=20 uv .320 → A .570 green;  x=40 uv .633 → A .883 green;
    //   x=50 uv .789 → B .039 yellow;  x=62 uv .977 → B .227 yellow.
    let f = render(&b.p, s, ms(1750), &fake);
    for (x, c) in [(8, RD), (20, GR), (40, GR), (50, YE), (62, YE)] {
        assert_px(&f, x, 32, c, &format!("V-9 push p=.25 x={x}"));
    }
    // p = .75: uv.x < .25 → A at uv.x + .75; else B at uv.x - .25.
    //   x=8 uv .133 → A .883 green;  x=20 uv .320 → B .070 yellow;  x=40 uv .633 → B .383 yellow;
    //   x=50 uv .789 → B .539 white.
    let f = render(&b.p, s, ms(2250), &fake);
    for (x, c) in [(8, GR), (20, YE), (40, YE), (50, WH)] {
        assert_px(&f, x, 32, c, &format!("V-9 push p=.75 x={x}"));
    }
}

#[test]
fn v9_clips_sampled_outside_their_range() {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v = b.vtrack(s);
    let ma = b.media("a", Some((64, 64)), false);
    let mb = b.media("b", Some((64, 64)), false);
    let l = b.media_clip(v, 0, 2 * S, ma);
    let r = b.media_clip(v, 2 * S, 2 * S, mb);
    b.clip_mut(r).source_in = 2 * S; // source time == sequence time for both clips
    b.transition(v, Some(l), Some(r), "wipe", S, &[]);
    // "a" encodes its 30 fps frame index in red, "b" in green.
    let fake = Fake::new().with_video(|p, t| match p {
        "a" => Ok(time_frame(t, 0, 64, 64)),
        "b" => Ok(time_frame(t, 1, 64, 64)),
        _ => Err(MediaError::NotFound(p.into())),
    });
    // t=2.25s (p=.75): A on the right, sampled past its end: source 2.25s → frame 67 (67.5 floored).
    let f = render(&b.p, s, ms(2250), &fake);
    assert_px(
        &f,
        60,
        32,
        [67, 0, 0],
        "V-9 left clip continues past its end",
    );
    // t=1.75s (p=.25): B on the left, before its start: 2s + round(-0.25s) = 1.75s → frame 52.
    let f = render(&b.p, s, ms(1750), &fake);
    assert_px(
        &f,
        2,
        32,
        [0, 52, 0],
        "V-9 right clip sampled before its start",
    );
}

#[test]
fn v9_one_sided_left_fades_to_transparent() {
    let mut b = B::new();
    let s = b.seq(16, 16);
    let v1 = b.vtrack(s);
    let v2 = b.vtrack(s);
    b.solid(v1, 0, 4 * S, WHITE);
    let l = b.solid(v2, 0, 2 * S, RED);
    b.transition(v2, Some(l), None, "cross_dissolve", S, &[]); // window [1s, 2s)
    let cases: [(Time, [u8; 3]); 4] = [
        (ms(900), [255, 0, 0]),
        (S, [255, 0, 0]),            // p = 0
        (ms(1750), [255, 191, 191]), // p=.75: red at alpha .25 over white: g = .75 → 191
        (2 * S, [255, 255, 255]),
    ];
    for (t, rgb) in cases {
        let f = render(&b.p, s, t, &Fake::new());
        assert_px(&f, 8, 8, rgb, &format!("V-9 left-only t={t}"));
    }
}

#[test]
fn v9_one_sided_right_fades_in() {
    let mut b = B::new();
    let s = b.seq(16, 16);
    let v1 = b.vtrack(s);
    let v2 = b.vtrack(s);
    b.solid(v1, 0, 4 * S, WHITE);
    let r = b.solid(v2, 2 * S, 2 * S, BLUE);
    b.transition(v2, None, Some(r), "cross_dissolve", S, &[]); // window [2s, 3s)
    let cases: [(Time, [u8; 3]); 3] = [
        (ms(1900), [255, 255, 255]),
        (ms(2250), [191, 191, 255]), // p=.25: blue at alpha .25 over white: r,g = .75
        (3 * S, [0, 0, 255]),
    ];
    for (t, rgb) in cases {
        let f = render(&b.p, s, t, &Fake::new());
        assert_px(&f, 8, 8, rgb, &format!("V-9 right-only t={t}"));
    }
}

#[test]
fn v9_layers_include_opacity_and_mix_premultiplied() {
    let (mut b, s, _v, l, _r) = solids(16, "cross_dissolve", &[]);
    b.set(l, "opacity", 0.5);
    // A premult (0.5,0,0 | a .5), B (0,0,1 | 1); p=.5 → (0.25, 0, 0.5 | a .75).
    // Over black: (0.25, 0, 0.5) → (64, 0, 128). (Straight-alpha mixing would give (96,0,96).)
    let f = render(&b.p, s, 2 * S, &Fake::new());
    assert_px(&f, 8, 8, [64, 0, 128], "V-9 premultiplied mix with opacity");
}

#[test]
fn v9_transition_layer_composited_normal() {
    let mut b = B::new();
    let s = b.seq(16, 16);
    let v1 = b.vtrack(s);
    let v2 = b.vtrack(s);
    b.solid(v1, 0, 4 * S, [0.5, 0.5, 0.5, 1.0]);
    let l = b.solid(v2, 0, 2 * S, RED);
    let r = b.solid(v2, 2 * S, 2 * S, BLUE);
    b.clip_mut(l).blend = BlendMode::Multiply;
    b.clip_mut(r).blend = BlendMode::Multiply;
    b.transition(v2, Some(l), Some(r), "cross_dissolve", S, &[]);
    // Outside the window the clip's Multiply applies: red * 0.5 grey = (0.5,0,0) → 128.
    let f = render(&b.p, s, ms(1400), &Fake::new());
    assert_px(&f, 8, 8, [128, 0, 0], "V-9 outside window uses clip blend");
    // In the window (p = 0) the transition layer = A, composited with Normal → pure red.
    let f = render(&b.p, s, ms(1500), &Fake::new());
    assert_px(&f, 8, 8, [255, 0, 0], "V-9 window layer composited Normal");
}

// ---- E-1 / E-2 / E-3 for transitions

const T_CTX: &str = "fn transition(uv: vec2f) -> vec4f {
    return vec4f(ctx.progress, p.level, ctx.duration / 4.0, 1.0);
}";

#[test]
fn e1_transition_ctx_progress_duration_and_params() {
    let (mut b, s, v, ..) = solids(16, "my_t", &[("level", 0.6)]);
    b.def("my_t", EffectKind::Transition, T_CTX, &[("level", 0.2)]);
    // p=.25 → 64; level .6 → 153; duration 1 s / 4 = .25 → 64.
    let f = render(&b.p, s, ms(1750), &Fake::new());
    assert_px(&f, 8, 8, [64, 153, 64], "E-1 transition ctx + params");
    // Missing param on the transition → ParamDef default .2 → 51.
    b.track_mut(v).transitions[0].params.clear();
    let f = render(&b.p, s, ms(1750), &Fake::new());
    assert_px(&f, 8, 8, [64, 51, 64], "E-2 transition param default");
}

#[test]
fn e1_transition_src_a_and_src_b() {
    let (mut b, s, ..) = solids(16, "split", &[]);
    b.def(
        "split",
        EffectKind::Transition,
        "fn transition(uv: vec2f) -> vec4f { return select(src_b(uv), src_a(uv), uv.x < 0.5); }",
        &[],
    );
    let f = render(&b.p, s, 2 * S, &Fake::new());
    assert_px(&f, 3, 8, [255, 0, 0], "E-1 src_a is the left clip");
    assert_px(&f, 12, 8, [0, 0, 255], "E-1 src_b is the right clip");
}

#[test]
fn e3_project_transition_shadows_builtin() {
    let (mut b, s, ..) = solids(16, "wipe", &[]);
    b.def(
        "wipe",
        EffectKind::Transition,
        "fn transition(uv: vec2f) -> vec4f { return vec4f(0.0, 1.0, 0.0, 1.0); }",
        &[],
    );
    let f = render(&b.p, s, 2 * S, &Fake::new());
    assert_px(
        &f,
        2,
        8,
        [0, 255, 0],
        "E-3 project 'wipe' wins over built-in",
    );
    assert_px(
        &f,
        13,
        8,
        [0, 255, 0],
        "E-3 project 'wipe' wins over built-in",
    );
}

#[test]
fn e3_transition_kind_mismatch_and_unknown() {
    let (b, s, ..) = solids(16, "invert", &[]); // built-in filter used as transition
    assert_eq!(
        try_render(&b.p, s, 2 * S, &Fake::new()),
        Err(RenderError::UnknownEffect("invert".into())),
        "E-3 filter used on a cut"
    );
    let (mut b, s, ..) = solids(16, "my_filter", &[]);
    b.def(
        "my_filter",
        EffectKind::Filter,
        "fn effect(uv: vec2f) -> vec4f { return src(uv); }",
        &[],
    );
    assert_eq!(
        try_render(&b.p, s, 2 * S, &Fake::new()),
        Err(RenderError::UnknownEffect("my_filter".into())),
        "E-3 project filter used on a cut"
    );
    let (b, s, ..) = solids(16, "no_such_transition", &[]);
    assert_eq!(
        try_render(&b.p, s, 2 * S, &Fake::new()),
        Err(RenderError::UnknownEffect("no_such_transition".into())),
        "V-4/E-3 unknown transition"
    );
}
