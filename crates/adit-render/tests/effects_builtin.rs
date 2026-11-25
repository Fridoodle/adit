//! SPEC E-BUILTIN: the shipped effects. Exact-formula filters are checked against hand-computed
//! pixels at non-default values plus identity at the neutral default; plausibility filters get the
//! checks the spec lists. 8-bit = round(v * 255), ±2 unless noted.

mod common;
use adit_core::*;
use adit_media::Frame;
use adit_render::{builtin_effects, validate_effect};
use common::*;

const FILTERS: &[&str] = &[
    "exposure",
    "brightness_contrast",
    "saturation",
    "temperature",
    "levels",
    "lift_gamma_gain",
    "invert",
    "tint",
    "hue_rotate",
    "flip",
    "rect",
    "ellipse",
    "linear_gradient",
    "gaussian_blur",
    "sharpen",
    "vignette",
    "pixelate",
    "chroma_key",
    "drop_shadow",
    "stroke",
    "noise",
];
const TRANSITIONS: &[&str] = &["cross_dissolve", "dip_to_black", "wipe", "push"];

#[test]
fn eb_all_listed_builtins_exist_with_right_kind_and_unique_names() {
    let defs = builtin_effects();
    let mut names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
    names.sort();
    let n = names.len();
    names.dedup();
    assert_eq!(names.len(), n, "E-BUILTIN names are unique");
    for name in FILTERS {
        let d = defs.iter().find(|d| d.name == *name).unwrap_or_else(|| panic!("missing built-in {name}"));
        assert_eq!(d.kind, EffectKind::Filter, "{name} is a filter");
    }
    for name in TRANSITIONS {
        let d = defs.iter().find(|d| d.name == *name).unwrap_or_else(|| panic!("missing built-in {name}"));
        assert_eq!(d.kind, EffectKind::Transition, "{name} is a transition");
    }
}

#[test]
fn eb_param_names_and_defaults_match_spec() {
    let table: &[(&str, &[(&str, Option<f64>)])] = &[
        ("exposure", &[("stops", Some(0.0))]),
        ("brightness_contrast", &[("brightness", Some(0.0)), ("contrast", Some(1.0))]),
        ("saturation", &[("amount", Some(1.0))]),
        ("temperature", &[("temp", Some(0.0)), ("tint", Some(0.0))]),
        (
            "levels",
            &[
                ("in_black", Some(0.0)),
                ("in_white", Some(1.0)),
                ("gamma", Some(1.0)),
                ("out_black", Some(0.0)),
                ("out_white", Some(1.0)),
            ],
        ),
        (
            "lift_gamma_gain",
            &[
                ("lift_r", Some(0.0)),
                ("lift_g", Some(0.0)),
                ("lift_b", Some(0.0)),
                ("gamma_r", Some(1.0)),
                ("gamma_g", Some(1.0)),
                ("gamma_b", Some(1.0)),
                ("gain_r", Some(1.0)),
                ("gain_g", Some(1.0)),
                ("gain_b", Some(1.0)),
            ],
        ),
        ("invert", &[("amount", Some(1.0))]),
        ("tint", &[("r", Some(1.0)), ("g", Some(1.0)), ("b", Some(1.0)), ("amount", Some(1.0))]),
        ("hue_rotate", &[("degrees", Some(0.0))]),
        ("flip", &[("horizontal", Some(0.0)), ("vertical", Some(0.0))]),
        (
            "rect",
            &[
                ("x", Some(0.25)),
                ("y", Some(0.25)),
                ("w", Some(0.5)),
                ("h", Some(0.5)),
                ("r", Some(1.0)),
                ("g", Some(1.0)),
                ("b", Some(1.0)),
                ("a", Some(1.0)),
            ],
        ),
        (
            "ellipse",
            &[
                ("cx", Some(0.5)),
                ("cy", Some(0.5)),
                ("rx", Some(0.25)),
                ("ry", Some(0.25)),
                ("r", None),
                ("g", None),
                ("b", None),
                ("a", None),
            ],
        ),
        (
            "linear_gradient",
            &[
                ("r0", None),
                ("g0", None),
                ("b0", None),
                ("a0", None),
                ("r1", None),
                ("g1", None),
                ("b1", None),
                ("a1", None),
                ("angle", Some(0.0)),
            ],
        ),
        ("gaussian_blur", &[("radius", Some(4.0))]),
        ("sharpen", &[("amount", Some(0.0))]),
        ("vignette", &[("amount", Some(0.5)), ("radius", Some(0.75))]),
        ("pixelate", &[("size", Some(8.0))]),
        (
            "chroma_key",
            &[
                ("r", Some(0.0)),
                ("g", Some(1.0)),
                ("b", Some(0.0)),
                ("threshold", Some(0.3)),
                ("softness", Some(0.1)),
            ],
        ),
        (
            "drop_shadow",
            &[("dx", Some(4.0)), ("dy", Some(4.0)), ("blur", Some(4.0)), ("opacity", Some(0.5))],
        ),
        (
            "stroke",
            &[("width", Some(2.0)), ("r", Some(0.0)), ("g", Some(0.0)), ("b", Some(0.0)), ("a", Some(1.0))],
        ),
        ("noise", &[("amount", Some(0.1)), ("seed", Some(0.0))]),
    ];
    let defs = builtin_effects();
    for (name, params) in table {
        let d = defs.iter().find(|d| d.name == *name).unwrap_or_else(|| panic!("missing built-in {name}"));
        for (pn, default) in params.iter() {
            let pd = d
                .params
                .iter()
                .find(|p| p.name == *pn)
                .unwrap_or_else(|| panic!("{name} lacks param {pn}"));
            if let Some(v) = default {
                assert!((pd.default - v).abs() < 1e-9, "{name}.{pn} default {} != {v}", pd.default);
            }
        }
    }
}

#[test]
fn eb_every_builtin_validates() {
    let defs = builtin_effects();
    assert!(defs.len() >= FILTERS.len() + TRANSITIONS.len());
    for d in &defs {
        assert!(!d.wgsl.trim().is_empty(), "{} has real source", d.name);
        if let Err(m) = validate_effect(d) {
            panic!("built-in {} fails validate_effect: {m}", d.name);
        }
    }
}

// ------------------------------------------------------------------ helpers

/// 16x16 solid `color` with one effect; pixel (8,8).
fn solid_fx(color: [f32; 4], effect: &str, params: &[(&str, f64)]) -> Frame {
    let mut b = B::new();
    let s = b.seq(16, 16);
    let v = b.vtrack(s);
    let c = b.solid(v, 0, S, color);
    b.effect(c, effect, params);
    render(&b.p, s, 0, &Fake::new())
}

fn check(color: [f32; 4], effect: &str, params: &[(&str, f64)], rgb: [u8; 3], what: &str) {
    let f = solid_fx(color, effect, params);
    assert_px(&f, 8, 8, rgb, &format!("{effect} {params:?}: {what}"));
}

/// Media image (canvas = image size) with one effect, optional solid background below.
fn image_fx(img: Frame, effect: &str, params: &[(&str, f64)], bg: Option<[f32; 4]>) -> Frame {
    let mut b = B::new();
    let s = b.seq(img.width, img.height);
    let v1 = b.vtrack(s);
    let v2 = b.vtrack(s);
    if let Some(c) = bg {
        b.solid(v1, 0, S, c);
    }
    let m = b.media("img", Some((img.width, img.height)), false);
    let c = b.media_clip(v2, 0, S, m);
    b.effect(c, effect, params);
    render(&b.p, s, 0, &Fake::images(vec![("img", img)]))
}

/// Solid on V2 over a solid background on V1, one effect on the top solid.
fn over(bg: [f32; 4], color: [f32; 4], effect: &str, params: &[(&str, f64)], size: u32) -> Frame {
    let mut b = B::new();
    let s = b.seq(size, size);
    let v1 = b.vtrack(s);
    let v2 = b.vtrack(s);
    b.solid(v1, 0, S, bg);
    let c = b.solid(v2, 0, S, color);
    b.effect(c, effect, params);
    render(&b.p, s, 0, &Fake::new())
}

const C1: [f32; 4] = [0.8, 0.4, 0.2, 1.0]; // → (204, 102, 51)

// ------------------------------------------------------------------ exact formulas

#[test]
fn eb_exposure() {
    check(C1, "exposure", &[], [204, 102, 51], "identity at stops 0");
    check(C1, "exposure", &[("stops", -1.0)], [102, 51, 26], "c*0.5 = .4,.2,.1");
    check([0.5, 0.25, 0.1, 1.0], "exposure", &[("stops", 2.0)], [255, 255, 102], "c*4 = 2,1,.4 clamped");
    // Alpha unchanged: (.25,.25,.25,a .5) * 2 = .5 at a .5 over black → .25 → 64.
    check([0.25, 0.25, 0.25, 0.5], "exposure", &[("stops", 1.0)], [64, 64, 64], "alpha unchanged");
}

#[test]
fn eb_brightness_contrast() {
    let c = [0.6, 0.5, 0.3, 1.0];
    check(c, "brightness_contrast", &[], [153, 128, 77], "identity");
    // (c-.5)*2+.5+.1: .8, .6, .2
    check(c, "brightness_contrast", &[("brightness", 0.1), ("contrast", 2.0)], [204, 153, 51], "formula");
}

#[test]
fn eb_saturation() {
    check(C1, "saturation", &[], [204, 102, 51], "identity at amount 1");
    // l = .2126 for red → 54.2
    check(RED, "saturation", &[("amount", 0.0)], [54, 54, 54], "amount 0 = luma");
    // mix(l, c, .5): r = .1063 + .5 = .6063 → 154.6; g = b = .1063 → 27.1
    check(RED, "saturation", &[("amount", 0.5)], [155, 27, 27], "amount .5");
}

#[test]
fn eb_temperature() {
    let g = [0.5, 0.5, 0.5, 1.0];
    check(g, "temperature", &[], [128, 128, 128], "identity");
    // r = .5+.1 = .6 → 153; g = .5-.1 = .4 → 102; b = .5-.1 = .4 → 102
    check(g, "temperature", &[("temp", 1.0), ("tint", -1.0)], [153, 102, 102], "formula");
}

#[test]
fn eb_levels() {
    let c = [0.3, 0.5, 0.8, 1.0];
    check(c, "levels", &[], [77, 128, 204], "identity");
    let p = [("in_black", 0.2), ("in_white", 0.6), ("gamma", 2.0), ("out_black", 0.1), ("out_white", 0.9)];
    // r: (.3-.2)/.4 = .25 → ^.5 = .5 → .5*.8+.1 = .5 → 128
    // g: .75 → .8660 → .7928 → 202.2; b: 1.5 → clamp 1 → .9 → 229.5
    check(c, "levels", &p, [128, 202, 230], "formula");
}

#[test]
fn eb_lift_gamma_gain() {
    let c = [0.2, 0.3, 0.25, 1.0];
    check(c, "lift_gamma_gain", &[], [51, 77, 64], "identity");
    // r: .2+.1 = .3 → 76.5; g: .3*2 = .6 → 153; b: .25^(1/2) = .5 → 128
    check(c, "lift_gamma_gain", &[("lift_r", 0.1), ("gain_g", 2.0), ("gamma_b", 2.0)], [77, 153, 128], "formula");
}

#[test]
fn eb_invert() {
    let c = [0.2, 0.6, 1.0, 1.0];
    check(c, "invert", &[("amount", 0.0)], [51, 153, 255], "identity at amount 0");
    check(c, "invert", &[], [204, 102, 0], "full invert at default 1");
    // c + .25*(1-2c): .35 → 89.25, .55 → 140.25, .75 → 191.25
    check(c, "invert", &[("amount", 0.25)], [89, 140, 191], "amount .25");
}

#[test]
fn eb_tint() {
    let g = [0.8, 0.8, 0.8, 1.0];
    check(g, "tint", &[], [204, 204, 204], "identity at white tint");
    // mix(c, c*(1,.5,0), .5) = (.8, .6, .4)
    check(g, "tint", &[("r", 1.0), ("g", 0.5), ("b", 0.0), ("amount", 0.5)], [204, 153, 102], "formula");
}

#[test]
fn eb_hue_rotate() {
    check(C1, "hue_rotate", &[], [204, 102, 51], "identity at 0");
    let f = solid_fx(RED, "hue_rotate", &[("degrees", 120.0)]);
    assert_px_tol(&f, 8, 8, [0, 255, 0], 6, "hue_rotate 120: red → green");
    let f = solid_fx(RED, "hue_rotate", &[("degrees", 240.0)]);
    assert_px_tol(&f, 8, 8, [0, 0, 255], 6, "hue_rotate 240: red → blue");
}

#[test]
fn eb_flip() {
    let lr = || frame_fn(64, 64, |x, _| if x < 32 { [255, 0, 0, 255] } else { [0, 255, 0, 255] });
    let tb = || frame_fn(64, 64, |_, y| if y < 32 { [255, 0, 0, 255] } else { [0, 255, 0, 255] });
    let f = image_fx(lr(), "flip", &[], None);
    assert_px(&f, 0, 32, [255, 0, 0], "flip default identity");
    let f = image_fx(lr(), "flip", &[("horizontal", 0.4)], None);
    assert_px(&f, 0, 32, [255, 0, 0], "flip .4 does not mirror");
    let f = image_fx(lr(), "flip", &[("horizontal", 1.0)], None);
    assert_px(&f, 0, 32, [0, 255, 0], "flip horizontal: left edge green");
    assert_px(&f, 63, 32, [255, 0, 0], "flip horizontal: right edge red");
    assert_px(&f, 30, 5, [0, 255, 0], "flip horizontal");
    let f = image_fx(tb(), "flip", &[("vertical", 1.0)], None);
    assert_px(&f, 32, 0, [0, 255, 0], "flip vertical: top green");
    assert_px(&f, 32, 63, [255, 0, 0], "flip vertical: bottom red");
}

#[test]
fn eb_rect_defaults_edges() {
    let black = [0.0, 0.0, 0.0, 1.0];
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v = b.vtrack(s);
    let c = b.solid(v, 0, S, black);
    b.effect(c, "rect", &[]);
    let f = render(&b.p, s, 0, &Fake::new());
    // .25 <= (x+.5)/64 < .75 → x in 16..=47; same for y.
    for (x, y) in [(16, 16), (47, 47), (16, 47), (47, 16), (32, 32)] {
        assert_px(&f, x, y, [255, 255, 255], "rect default inside");
    }
    for (x, y) in [(15, 32), (48, 32), (32, 15), (32, 48), (0, 0)] {
        assert_px(&f, x, y, [0, 0, 0], "rect default outside");
    }
}

#[test]
fn eb_rect_custom_alpha_over() {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v = b.vtrack(s);
    let c = b.solid(v, 0, S, BLUE);
    let p = [("x", 0.5), ("y", 0.0), ("w", 0.25), ("h", 0.5), ("r", 1.0), ("g", 0.0), ("b", 0.0), ("a", 0.5)];
    b.effect(c, "rect", &p);
    let f = render(&b.p, s, 0, &Fake::new());
    // Inside x 32..=47, y 0..=31: red a .5 over blue → (.5, 0, .5) → (128, 0, 128).
    for (x, y) in [(32, 0), (47, 31), (40, 16)] {
        assert_px(&f, x, y, [128, 0, 128], "rect custom inside");
    }
    for (x, y) in [(31, 10), (48, 10), (40, 32)] {
        assert_px(&f, x, y, [0, 0, 255], "rect custom outside");
    }
}

#[test]
fn eb_ellipse() {
    let black = [0.0, 0.0, 0.0, 1.0];
    let col = [("r", 0.0), ("g", 1.0), ("b", 0.0), ("a", 1.0)];
    let f = over(black, black, "ellipse", &col, 64);
    // Circle r .25 around (.5,.5): (32, 18) → dy = 18.5/64 - .5 = -.211 → inside.
    for (x, y) in [(32, 32), (32, 18), (45, 32)] {
        assert_px(&f, x, y, [0, 255, 0], "ellipse default inside");
    }
    for (x, y) in [(32, 4), (2, 2), (60, 32), (48, 48)] {
        assert_px(&f, x, y, [0, 0, 0], "ellipse default outside");
    }
    // rx .4, ry .1: (52,32): dx .32 → (.32/.4)² = .64 → inside; (32,42): dy .164 → 2.7 → outside.
    let mut p = col.to_vec();
    p.extend([("rx", 0.4), ("ry", 0.1)]);
    let f = over(black, black, "ellipse", &p, 64);
    assert_px(&f, 52, 32, [0, 255, 0], "ellipse wide inside");
    assert_px(&f, 32, 42, [0, 0, 0], "ellipse flat outside");
}

#[test]
fn eb_linear_gradient() {
    let p = [
        ("r0", 1.0), ("g0", 0.0), ("b0", 0.0), ("a0", 1.0),
        ("r1", 0.0), ("g1", 0.0), ("b1", 1.0), ("a1", 1.0),
    ];
    let f = over(WHITE, WHITE, "linear_gradient", &p, 64);
    // angle 0 = left to right: t ≈ uv.x.
    assert_px_tol(&f, 0, 32, [255, 0, 0], 6, "gradient left = colour 0");
    assert_px_tol(&f, 63, 32, [0, 0, 255], 6, "gradient right = colour 1");
    assert_px_tol(&f, 32, 32, [128, 0, 128], 6, "gradient middle");
    assert_px_tol(&f, 32, 0, [128, 0, 128], 6, "angle 0 constant along y");
    assert_px_tol(&f, 32, 63, [128, 0, 128], 6, "angle 0 constant along y");
    // Replaces the image: transparent colour 0 lets the black canvas through at the left,
    // even though the input solid was white.
    let mut q = p.to_vec();
    q[3] = ("a0", 0.0);
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v = b.vtrack(s);
    let c = b.solid(v, 0, S, WHITE);
    b.effect(c, "linear_gradient", &q);
    let f = render(&b.p, s, 0, &Fake::new());
    assert_px_tol(&f, 0, 32, [0, 0, 0], 6, "gradient replaces input (transparent left)");
    assert_px_tol(&f, 63, 32, [0, 0, 255], 6, "gradient right opaque blue");
}

// ------------------------------------------------------------------ plausibility

fn step(lo: u8, hi: u8) -> Frame {
    frame_fn(64, 64, |x, _| if x < 32 { [lo, lo, lo, 255] } else { [hi, hi, hi, 255] })
}

#[test]
fn eb_gaussian_blur() {
    let f = image_fx(step(255, 0), "gaussian_blur", &[("radius", 0.0)], None);
    assert_px(&f, 31, 32, [255, 255, 255], "blur 0 identity");
    assert_px(&f, 32, 32, [0, 0, 0], "blur 0 identity");
    let f = image_fx(step(255, 0), "gaussian_blur", &[], None);
    assert!(px(&f, 31, 32)[0] < 250, "blur 4 softens the edge (white side): {:?}", px(&f, 31, 32));
    assert!(px(&f, 32, 32)[0] > 5, "blur 4 softens the edge (black side): {:?}", px(&f, 32, 32));
    assert_px(&f, 16, 32, [255, 255, 255], "blur 4 flat area far from edges unchanged");
    assert_px(&f, 47, 32, [0, 0, 0], "blur 4 flat area far from edges unchanged");
}

#[test]
fn eb_sharpen() {
    let f = image_fx(step(64, 191), "sharpen", &[], None);
    assert_px(&f, 31, 32, [64, 64, 64], "sharpen 0 identity");
    assert_px(&f, 32, 32, [191, 191, 191], "sharpen 0 identity");
    let f = image_fx(step(64, 191), "sharpen", &[("amount", 1.0)], None);
    let changed = (28..36).any(|x| {
        let v = px(&f, x, 32)[0];
        let input = if x < 32 { 64 } else { 191 };
        v.abs_diff(input) > 4
    });
    assert!(changed, "sharpen 1 changes pixels at the edge");
    assert_px(&f, 12, 32, [64, 64, 64], "sharpen leaves flat area");
}

#[test]
fn eb_vignette() {
    let f = over(WHITE, WHITE, "vignette", &[("amount", 0.0)], 64);
    assert_px(&f, 0, 0, [255, 255, 255], "vignette 0 identity");
    let f = over(WHITE, WHITE, "vignette", &[], 64);
    assert!(px(&f, 32, 32)[0] >= 245, "vignette keeps centre bright: {:?}", px(&f, 32, 32));
    assert!(px(&f, 0, 0)[0] < 230, "vignette darkens corner: {:?}", px(&f, 0, 0));
}

#[test]
fn eb_pixelate() {
    let ramp = || frame_fn(64, 64, |x, _| [(x * 4) as u8, 0, 0, 255]);
    let f = image_fx(ramp(), "pixelate", &[("size", 1.0)], None);
    assert_px(&f, 7, 32, [28, 0, 0], "pixelate 1 identity");
    let f = image_fx(ramp(), "pixelate", &[], None);
    let row: Vec<u8> = (0..64).map(|x| px(&f, x, 32)[0]).collect();
    let changes = row.windows(2).filter(|w| w[0].abs_diff(w[1]) > 1).count();
    assert!(changes <= 9, "pixelate 8: ~8 blocks per row, value changes {changes}: {row:?}");
    assert!(changes >= 4, "pixelate 8 keeps block structure: {row:?}");
}

#[test]
fn eb_chroma_key() {
    let f = over(WHITE, GREEN, "chroma_key", &[], 16);
    assert_px(&f, 8, 8, [255, 255, 255], "chroma_key: pure key colour → alpha 0");
    let f = over(WHITE, RED, "chroma_key", &[], 16);
    assert_px(&f, 8, 8, [255, 0, 0], "chroma_key keeps other colours");
}

fn red_square() -> Frame {
    // 64x64, opaque red at [24,40)², transparent elsewhere.
    frame_fn(64, 64, |x, y| {
        if (24..40).contains(&x) && (24..40).contains(&y) {
            [255, 0, 0, 255]
        } else {
            [0, 0, 0, 0]
        }
    })
}

#[test]
fn eb_drop_shadow() {
    let f = image_fx(red_square(), "drop_shadow", &[("opacity", 0.0)], Some(WHITE));
    assert_px(&f, 41, 41, [255, 255, 255], "drop_shadow opacity 0 identity");
    let f = image_fx(red_square(), "drop_shadow", &[], Some(WHITE));
    assert_px(&f, 32, 32, [255, 0, 0], "drop_shadow keeps the object");
    assert!(px(&f, 41, 41)[0] < 235, "shadow at +dx,+dy darkens: {:?}", px(&f, 41, 41));
    assert_px(&f, 20, 20, [255, 255, 255], "no shadow on the opposite side");
}

#[test]
fn eb_stroke() {
    let f = image_fx(red_square(), "stroke", &[("width", 0.0)], Some(WHITE));
    for x in 36..44 {
        assert!(px(&f, x, 32)[0] >= 250, "stroke width 0 identity at x={x}");
    }
    let f = image_fx(red_square(), "stroke", &[], Some(WHITE));
    assert!(
        (36..44).any(|x| px(&f, x, 32)[0] < 100),
        "stroke 2 px black near the edge at x 36..44: {:?}",
        (36..44).map(|x| px(&f, x, 32)).collect::<Vec<_>>()
    );
    assert_px(&f, 32, 32, [255, 0, 0], "stroke keeps the interior");
    assert_px(&f, 52, 32, [255, 255, 255], "stroke does not reach far");
}

#[test]
fn eb_noise() {
    let grey = [0.5, 0.5, 0.5, 1.0];
    let f = over(grey, grey, "noise", &[("amount", 0.0)], 64);
    assert!(f.rgba.chunks(4).all(|p| p[0].abs_diff(128) <= 2), "noise 0 identity");
    let f = over(grey, grey, "noise", &[], 64);
    let vals: Vec<u8> = f.rgba.chunks(4).map(|p| p[0]).collect();
    let off = vals.iter().filter(|v| v.abs_diff(128) > 3).count();
    let mean = vals.iter().map(|v| *v as f64).sum::<f64>() / vals.len() as f64;
    assert!(off > vals.len() / 10, "noise .1 changes many pixels: {off}/{}", vals.len());
    assert!((mean - 128.0).abs() < 10.0, "noise roughly zero-mean: {mean}");
}
