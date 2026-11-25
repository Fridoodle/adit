//! SPEC V-4 and E-1..E-5: runtime (project) effects, the shader contract, parameters, lookup,
//! validation.

mod common;
use adit_core::*;
use adit_render::{RenderError, validate_effect};
use common::*;

/// 16x16 (or `w`x`h`) sequence, one solid clip [start, start+dur) with the given effect defs and
/// effect instances; rendered at `t`.
struct Fx {
    b: B,
    s: Id,
    c: Id,
}

fn fx(w: u32, h: u32, color: [f32; 4], start: Time, dur: Time) -> Fx {
    let mut b = B::new();
    let s = b.seq(w, h);
    let v = b.vtrack(s);
    let c = b.solid(v, start, dur, color);
    Fx { b, s, c }
}

fn filter(b: &mut B, name: &str, body: &str, params: &[(&str, f64)]) {
    b.def(name, EffectKind::Filter, body, params);
}

// ------------------------------------------------------------------ V-4

#[test]
fn v4_effects_run_in_order() {
    let add = "fn effect(uv: vec2f) -> vec4f { return src(uv) + vec4f(0.25, 0.0, 0.0, 0.0); }";
    let dbl = "fn effect(uv: vec2f) -> vec4f { let c = src(uv); return vec4f(c.rgb * 2.0, c.a); }";
    for (order, expect) in [(["add_r", "double"], 255u8), (["double", "add_r"], 191u8)] {
        let mut x = fx(16, 16, [0.25, 0.0, 0.0, 1.0], 0, S);
        filter(&mut x.b, "add_r", add, &[]);
        filter(&mut x.b, "double", dbl, &[]);
        for e in order {
            x.b.effect(x.c, e, &[]);
        }
        // add then double: (.25+.25)*2 = 1.0 → 255; double then add: .5+.25 = .75 → 191.
        let f = render(&x.b.p, x.s, 0, &Fake::new());
        assert_px(&f, 8, 8, [expect, 0, 0], &format!("V-4 order {order:?}"));
    }
}

#[test]
fn v4_disabled_effect_is_skipped() {
    let mut x = fx(16, 16, [0.25, 0.0, 0.0, 1.0], 0, S);
    filter(
        &mut x.b,
        "green",
        "fn effect(uv: vec2f) -> vec4f { return vec4f(0.0, 1.0, 0.0, 1.0); }",
        &[],
    );
    let e = x.b.effect(x.c, "green", &[]);
    x.b.effect_mut(x.c, e).enabled = false;
    let f = render(&x.b.p, x.s, 0, &Fake::new());
    assert_px(
        &f,
        8,
        8,
        [64, 0, 0],
        "V-4 disabled effect skipped (.25 → 64)",
    );
}

#[test]
fn v4_unknown_effect_is_error() {
    let mut x = fx(16, 16, RED, 0, S);
    x.b.effect(x.c, "no_such_effect", &[]);
    assert_eq!(
        try_render(&x.b.p, x.s, 0, &Fake::new()),
        Err(RenderError::UnknownEffect("no_such_effect".into()))
    );
}

// ------------------------------------------------------------------ E-1

fn one(body: &str, params: &[(&str, f64)], w: u32, h: u32, color: [f32; 4]) -> adit_media::Frame {
    let mut x = fx(w, h, color, 0, S);
    filter(&mut x.b, "e", body, params);
    x.b.effect(x.c, "e", &[]);
    render(&x.b.p, x.s, 0, &Fake::new())
}

#[test]
fn e1_uv_origin_top_left() {
    let f = one(
        "fn effect(uv: vec2f) -> vec4f { return vec4f(uv.x, uv.y, 0.0, 1.0); }",
        &[],
        64,
        64,
        WHITE,
    );
    // Pixel centres: (0.5/64)*255 ≈ 2, (63.5/64)*255 ≈ 253, (32.5/64)*255 ≈ 129.5.
    assert_px(&f, 0, 0, [2, 2, 0], "E-1 uv at top-left");
    assert_px(&f, 63, 0, [253, 2, 0], "E-1 uv at top-right");
    assert_px(&f, 0, 63, [2, 253, 0], "E-1 uv at bottom-left");
    assert_px(&f, 32, 32, [130, 130, 0], "E-1 uv at centre");
}

#[test]
fn e1_src_is_transparent_outside_0_1() {
    let f = one(
        "fn effect(uv: vec2f) -> vec4f { return src(uv + vec2f(0.5, 0.0)); }",
        &[],
        64,
        64,
        WHITE,
    );
    assert_px(&f, 10, 32, [255, 255, 255], "E-1 src inside image");
    assert_px(&f, 28, 32, [255, 255, 255], "E-1 src inside image");
    assert_px(&f, 36, 32, [0, 0, 0], "E-1 src outside 0..1 is transparent");
    assert_px(&f, 60, 32, [0, 0, 0], "E-1 src outside 0..1 is transparent");
}

#[test]
fn e1_output_is_straight_alpha() {
    // (1,0,0,0.5) straight over black → 0.5 → 128 (premultiplied reading would give 255).
    let f = one(
        "fn effect(uv: vec2f) -> vec4f { return vec4f(1.0, 0.0, 0.0, 0.5); }",
        &[],
        16,
        16,
        WHITE,
    );
    assert_px(&f, 8, 8, [128, 0, 0], "E-1 straight alpha output");
}

#[test]
fn e1_ctx_time_duration_progress_frame() {
    // Clip [1s, 5s), rendered at 2s: time 1 s, duration 4 s, progress .25, sequence frame 60.
    let mut x = fx(16, 16, WHITE, S, 4 * S);
    filter(
        &mut x.b,
        "ctx",
        "fn effect(uv: vec2f) -> vec4f {
            return vec4f(ctx.time / 4.0, ctx.duration / 4.0, ctx.progress, 1.0);
        }",
        &[],
    );
    filter(
        &mut x.b,
        "frame",
        "fn effect(uv: vec2f) -> vec4f { return vec4f(ctx.frame / 255.0, 0.0, 0.0, 1.0); }",
        &[],
    );
    let e = x.b.effect(x.c, "ctx", &[]);
    let f = render(&x.b.p, x.s, 2 * S, &Fake::new());
    // .25 → 64, 1.0 → 255, .25 → 64.
    assert_px(&f, 8, 8, [64, 255, 64], "E-1 ctx.time/duration/progress");
    x.b.effect_mut(x.c, e).effect = "frame".into();
    let f = render(&x.b.p, x.s, 2 * S, &Fake::new());
    assert_px(
        &f,
        8,
        8,
        [60, 0, 0],
        "E-1 ctx.frame = sequence frame 60 (not local 30)",
    );
}

#[test]
fn e1_ctx_size_is_image_size() {
    let body = "fn effect(uv: vec2f) -> vec4f { return vec4f(ctx.size.x / 255.0, ctx.size.y / 255.0, 0.0, 1.0); }";
    // Solid → sequence-sized image 64x48.
    let f = one(body, &[], 64, 48, WHITE);
    assert_px(&f, 32, 24, [64, 48, 0], "E-1 ctx.size of a solid");
    // Media 40x20 on a 64x64 canvas → ctx.size (40, 20).
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v = b.vtrack(s);
    let m = b.media("img", Some((40, 20)), false);
    let c = b.media_clip(v, 0, S, m);
    filter(&mut b, "e", body, &[]);
    b.effect(c, "e", &[]);
    let fake = Fake::images(vec![("img", solid_frame(40, 20, [255, 255, 255, 255]))]);
    let f = render(&b.p, s, 0, &fake);
    assert_px(
        &f,
        32,
        32,
        [40, 20, 0],
        "E-1 ctx.size of a 40x20 media image",
    );
}

#[test]
fn e1_params_declared_in_order_and_overridden_by_instance() {
    let body = "fn effect(uv: vec2f) -> vec4f { return vec4f(p.a, p.b, 0.0, 1.0); }";
    let mut x = fx(16, 16, WHITE, 0, S);
    filter(&mut x.b, "ab", body, &[("a", 0.2), ("b", 0.4)]);
    let e = x.b.effect(x.c, "ab", &[]);
    let f = render(&x.b.p, x.s, 0, &Fake::new());
    assert_px(
        &f,
        8,
        8,
        [51, 102, 0],
        "E-1 p.a/.b defaults (.2→51, .4→102)",
    );
    x.b.effect_mut(x.c, e).params.insert("a".into(), param(0.6));
    let f = render(&x.b.p, x.s, 0, &Fake::new());
    assert_px(&f, 8, 8, [153, 102, 0], "E-1 instance value a=.6 → 153");
}

#[test]
fn e1_helpers_and_constants_allowed() {
    let body = "const K: f32 = 0.5;
        fn half(x: f32) -> f32 { return x * K; }
        struct Pair { a: f32, b: f32 }
        fn effect(uv: vec2f) -> vec4f {
            let q = Pair(half(src(uv).r), 0.0);
            var acc = 0.0;
            for (var i = 0; i < 4; i++) { acc += 0.0625; }
            return vec4f(q.a, q.b, acc, 1.0);
        }";
    let f = one(body, &[], 16, 16, WHITE);
    // half(1) = .5 → 128; acc = 4 * .0625 = .25 → 64.
    assert_px(&f, 8, 8, [128, 0, 64], "E-1 helper fn, const, struct, loop");
}

// ------------------------------------------------------------------ E-2

#[test]
fn e2_keyframed_param_evaluated_at_local_time() {
    let mut x = fx(16, 16, WHITE, S, 4 * S);
    filter(
        &mut x.b,
        "lvl",
        "fn effect(uv: vec2f) -> vec4f { return vec4f(p.v, 0.0, 0.0, 1.0); }",
        &[("v", 0.0)],
    );
    let e = x.b.effect(x.c, "lvl", &[]);
    x.b.effect_mut(x.c, e).params.insert(
        "v".into(),
        keyed(&[(0, 0.0, Easing::Linear), (2 * S, 1.0, Easing::Linear)]),
    );
    // t = 2s → local 1s → .5 → 128 (sequence time 2s would give 1.0 → 255).
    let f = render(&x.b.p, x.s, 2 * S, &Fake::new());
    assert_px(&f, 8, 8, [128, 0, 0], "E-2 keyframes at local time");
    // EaseInOut at u = .25: 3u² - 2u³ = .1875 - .03125 = .15625 → 39.8 → 40.
    x.b.effect_mut(x.c, e).params.insert(
        "v".into(),
        keyed(&[(0, 0.0, Easing::EaseInOut), (4 * S, 1.0, Easing::Linear)]),
    );
    let f = render(&x.b.p, x.s, 2 * S, &Fake::new());
    assert_px(&f, 8, 8, [40, 0, 0], "E-2 EaseInOut keyframes");
}

#[test]
fn e2_missing_param_takes_paramdef_default() {
    let f = one(
        "fn effect(uv: vec2f) -> vec4f { return vec4f(p.v, 0.0, 0.0, 1.0); }",
        &[("v", 0.6)],
        16,
        16,
        WHITE,
    );
    assert_px(&f, 8, 8, [153, 0, 0], "E-2 default .6 → 153");
}

// ------------------------------------------------------------------ E-3

#[test]
fn e3_project_effect_shadows_builtin() {
    let mut x = fx(16, 16, RED, 0, S);
    filter(
        &mut x.b,
        "invert",
        "fn effect(uv: vec2f) -> vec4f { return vec4f(0.0, 1.0, 0.0, 1.0); }",
        &[],
    );
    x.b.effect(x.c, "invert", &[]);
    // Project def → green; the built-in invert would give cyan (0,255,255).
    let f = render(&x.b.p, x.s, 0, &Fake::new());
    assert_px(&f, 8, 8, [0, 255, 0], "E-3 project def shadows built-in");
}

#[test]
fn e3_transition_kind_on_clip_is_unknown_effect() {
    let mut x = fx(16, 16, RED, 0, S);
    x.b.effect(x.c, "wipe", &[]);
    assert_eq!(
        try_render(&x.b.p, x.s, 0, &Fake::new()),
        Err(RenderError::UnknownEffect("wipe".into())),
        "E-3 built-in transition on a clip"
    );
    let mut x = fx(16, 16, RED, 0, S);
    x.b.def(
        "my_tr",
        EffectKind::Transition,
        "fn transition(uv: vec2f) -> vec4f { return src_a(uv); }",
        &[],
    );
    x.b.effect(x.c, "my_tr", &[]);
    assert_eq!(
        try_render(&x.b.p, x.s, 0, &Fake::new()),
        Err(RenderError::UnknownEffect("my_tr".into())),
        "E-3 project transition on a clip"
    );
}

// ------------------------------------------------------------------ E-4

const OK_FILTER: &str = "fn effect(uv: vec2f) -> vec4f { return src(uv) * p.gain; }";
const OK_TRANSITION: &str =
    "fn transition(uv: vec2f) -> vec4f { return mix(src_a(uv), src_b(uv), ctx.progress); }";

fn err(d: &EffectDef) -> String {
    match validate_effect(d) {
        Ok(()) => panic!("E-4 expected Err for {:?} / params {:?}", d.wgsl, d.params),
        Err(m) => {
            assert!(!m.trim().is_empty(), "E-4 error message must not be empty");
            m
        }
    }
}

#[test]
fn e4_valid_definitions_pass() {
    assert_eq!(
        validate_effect(&def("f", EffectKind::Filter, OK_FILTER, &[("gain", 1.0)])),
        Ok(())
    );
    assert_eq!(
        validate_effect(&def("t", EffectKind::Transition, OK_TRANSITION, &[])),
        Ok(())
    );
    let all_ctx = "fn effect(uv: vec2f) -> vec4f {
        return vec4f(ctx.size.x + ctx.time + ctx.duration + ctx.progress + ctx.frame, 0.0, 0.0, 1.0);
    }";
    assert_eq!(
        validate_effect(&def("c", EffectKind::Filter, all_ctx, &[])),
        Ok(())
    );
    let params: Vec<(String, f64)> = (0..64).map(|i| (format!("p{i}"), 0.0)).collect();
    let refs: Vec<(&str, f64)> = params.iter().map(|(n, v)| (n.as_str(), *v)).collect();
    assert_eq!(
        validate_effect(&def(
            "many",
            EffectKind::Filter,
            OK_FILTER.replace("p.gain", "p.p63").as_str(),
            &refs
        )),
        Ok(()),
        "E-2 exactly 64 params is allowed"
    );
}

#[test]
fn e4_rejects_bad_wgsl() {
    let cases = [
        ("syntax error", "fn effect(uv: vec2f) -> vec4f { return ; }"),
        (
            "undefined name",
            "fn effect(uv: vec2f) -> vec4f { return nope(uv); }",
        ),
        (
            "type error",
            "fn effect(uv: vec2f) -> vec4f { return 1.0 + true; }",
        ),
        (
            "recursion",
            "fn effect(uv: vec2f) -> vec4f { return effect(uv); }",
        ),
    ];
    for (what, wgsl) in cases {
        err(&def("x", EffectKind::Filter, wgsl, &[]));
        eprintln!("E-4 rejected: {what}");
    }
}

#[test]
fn e4_rejects_missing_or_wrong_entry_function() {
    let m = err(&def(
        "x",
        EffectKind::Filter,
        "fn helper(uv: vec2f) -> vec4f { return vec4f(1.0); }",
        &[],
    ));
    assert!(
        m.contains("effect"),
        "E-4 message names the missing `effect` fn: {m}"
    );
    let m = err(&def(
        "x",
        EffectKind::Transition,
        "fn effect(uv: vec2f) -> vec4f { return vec4f(1.0); }",
        &[],
    ));
    assert!(
        m.contains("transition"),
        "E-4 message names the missing `transition` fn: {m}"
    );
    err(&def(
        "x",
        EffectKind::Filter,
        "fn transition(uv: vec2f) -> vec4f { return src_a(uv); }",
        &[],
    ));
    for wrong in [
        "fn effect(uv: vec2f) -> vec3f { return vec3f(1.0); }",
        "fn effect(uv: f32) -> vec4f { return vec4f(1.0); }",
        "fn effect(uv: vec2f, k: f32) -> vec4f { return vec4f(k); }",
        "fn effect() -> vec4f { return vec4f(1.0); }",
        "fn effect(uv: vec2f) { }",
    ] {
        err(&def("x", EffectKind::Filter, wrong, &[]));
    }
}

#[test]
fn e4_rejects_own_bindings_and_entry_points() {
    err(&def(
        "x",
        EffectKind::Filter,
        "@group(3) @binding(7) var<uniform> extra: vec4f;
         fn effect(uv: vec2f) -> vec4f { return extra; }",
        &[],
    ));
    err(&def(
        "x",
        EffectKind::Filter,
        "@fragment fn my_main() -> @location(0) vec4f { return vec4f(1.0); }
         fn effect(uv: vec2f) -> vec4f { return vec4f(1.0); }",
        &[],
    ));
    err(&def(
        "x",
        EffectKind::Filter,
        "@compute @workgroup_size(1) fn cs() { }
         fn effect(uv: vec2f) -> vec4f { return vec4f(1.0); }",
        &[],
    ));
}

#[test]
fn e4_rejects_bad_param_names() {
    let m = err(&def(
        "x",
        EffectKind::Filter,
        OK_FILTER,
        &[("gain", 1.0), ("gain", 2.0)],
    ));
    assert!(
        m.contains("gain"),
        "E-4 duplicate param message names it: {m}"
    );
    for bad in ["1abc", "a-b", "has space", "fn", "p.q"] {
        let m = err(&def(
            "x",
            EffectKind::Filter,
            OK_FILTER,
            &[("gain", 1.0), (bad, 0.0)],
        ));
        assert!(
            m.contains(bad),
            "E-4 invalid param name message names {bad:?}: {m}"
        );
    }
    err(&def(
        "x",
        EffectKind::Filter,
        OK_FILTER,
        &[("gain", 1.0), ("", 0.0)],
    ));
}

#[test]
fn e4_rejects_more_than_64_params() {
    let params: Vec<(String, f64)> = (0..65).map(|i| (format!("p{i}"), 0.0)).collect();
    let refs: Vec<(&str, f64)> = params.iter().map(|(n, v)| (n.as_str(), *v)).collect();
    let m = err(&def(
        "x",
        EffectKind::Filter,
        "fn effect(uv: vec2f) -> vec4f { return src(uv); }",
        &refs,
    ));
    assert!(
        m.contains("64") || m.contains("65"),
        "E-4 message mentions the limit: {m}"
    );
}

#[test]
fn e4_never_panics_on_garbage() {
    let garbage = [
        String::new(),
        "   \n\t ".to_string(),
        "\0\0\0".to_string(),
        String::from_utf8_lossy(&[0xff, 0xfe, 0x00, 0x80, 0x9f, 0x41, 0x7b, 0xc3]).into_owned(),
        "fn effect(".to_string(),
        "}}}}{{{{".to_string(),
        "fn effect(uv: vec2f) -> vec4f { return vec4f(1.0); ".to_string(),
        "fn effect(uv: vec2f) -> vec4f { return vec4f(1.0); } }".to_string(),
        "x".repeat(100_000),
    ];
    for g in &garbage {
        err(&def("x", EffectKind::Filter, g, &[]));
        err(&def("x", EffectKind::Transition, g, &[]));
    }
}

#[test]
fn e4_validated_defs_render_without_shader_error() {
    // Each def validates and must then render red over the white input.
    let defs = [
        "fn effect(uv: vec2f) -> vec4f { return vec4f(1.0, 0.0, 0.0, 1.0); }",
        "fn effect(uv: vec2f) -> vec4f { let c = src(uv); return vec4f(c.r, 0.0, 0.0, c.a) * vec4f(p.gain, 1.0, 1.0, 1.0); }",
        "const R: vec3f = vec3f(1.0, 0.0, 0.0);
         fn tint(c: vec4f) -> vec4f { return vec4f(R * c.rgb, c.a); }
         fn effect(uv: vec2f) -> vec4f { return tint(src(clamp(uv, vec2f(0.0), vec2f(1.0)))); }",
        "fn effect(uv: vec2f) -> vec4f {
            var c = src(uv);
            if (ctx.size.x > 0.0 && ctx.frame >= 0.0) { c.g = 0.0; c.b = 0.0; }
            return c;
         }",
    ];
    for wgsl in defs {
        let d = def("chk", EffectKind::Filter, wgsl, &[("gain", 1.0)]);
        assert_eq!(validate_effect(&d), Ok(()), "E-4 should validate: {wgsl}");
        let mut x = fx(16, 16, WHITE, 0, S);
        x.b.p.effects.push(d);
        x.b.effect(x.c, "chk", &[]);
        match try_render(&x.b.p, x.s, 0, &Fake::new()) {
            Ok(f) => assert_px(&f, 8, 8, [255, 0, 0], "E-4 validated def renders"),
            Err(e) => panic!("E-4 validated def failed to render: {e:?}\n{wgsl}"),
        }
    }
}

#[test]
fn e4_invalid_project_def_fails_render() {
    let mut x = fx(16, 16, WHITE, 0, S);
    filter(
        &mut x.b,
        "broken",
        "fn effect(uv: vec2f) -> vec4f { return ; }",
        &[],
    );
    x.b.effect(x.c, "broken", &[]);
    let r = try_render(&x.b.p, x.s, 0, &Fake::new());
    assert!(
        matches!(
            r,
            Err(RenderError::Shader(_)) | Err(RenderError::UnknownEffect(_))
        ),
        "V-3/E-4 invalid effect → Err(Shader | UnknownEffect), got {r:?}"
    );
}

// ------------------------------------------------------------------ E-5

#[test]
fn e5_spec_example() {
    let body = "fn effect(uv: vec2f) -> vec4f { let c = src(uv);
  return vec4f(c.b, c.g, c.r, c.a) * vec4f(p.gain, p.gain, p.gain, 1.0); }";
    let mut x = fx(16, 16, [1.0, 0.5, 0.0, 1.0], 0, S);
    filter(&mut x.b, "swap", body, &[("gain", 1.0)]);
    let e = x.b.effect(x.c, "swap", &[]);
    // (b, g, r) = (0, .5, 1) → (0, 128, 255).
    let f = render(&x.b.p, x.s, 0, &Fake::new());
    assert_px(&f, 8, 8, [0, 128, 255], "E-5 gain 1");
    // gain .5 → (0, .25, .5) → (0, 64, 128).
    x.b.effect_mut(x.c, e)
        .params
        .insert("gain".into(), param(0.5));
    let f = render(&x.b.p, x.s, 0, &Fake::new());
    assert_px(&f, 8, 8, [0, 64, 128], "E-5 gain 0.5");
}
