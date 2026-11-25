//! SPEC E-BLUR-1 (separable blur must match the single-pass built-in) and E-3 (a project effect
//! named `gaussian_blur` is never replaced by the built-in or its fast path).

mod common;
use adit_core::*;
use adit_media::Frame;
use adit_render::builtin_effects;
use common::*;

const W: u32 = 160;
const H: u32 = 90;

/// Opaque, detailed, non-square input: 8 px checkerboard (sharp edges) with a hash-noise texture,
/// plus a horizontal stripe and a vertical stripe so x/y mixups show.
fn detailed() -> Frame {
    frame_fn(W, H, |x, y| {
        let check = ((x / 8 + y / 8) % 2) * 160;
        let n =
            (x.wrapping_mul(73856093) ^ y.wrapping_mul(19349663)).wrapping_mul(2654435761) >> 24;
        let r = (check + n / 4).min(255) as u8;
        let g = if y == 40 || y == 41 {
            255
        } else {
            (n / 2) as u8
        };
        let b = if x == 20 {
            255
        } else {
            (x * 255 / (W - 1)) as u8
        };
        [r, g, b, 255]
    })
}

fn render_with(def: Option<EffectDef>, effect: &str, params: &[(&str, f64)], img: &Frame) -> Frame {
    let mut b = B::new();
    let s = b.seq(W, H);
    let v = b.vtrack(s);
    let m = b.media("img", Some((W, H)), false);
    let c = b.media_clip(v, 0, S, m);
    if let Some(d) = def {
        b.p.effects.push(d);
    }
    b.effect(c, effect, params);
    render(&b.p, s, 0, &Fake::images(vec![("img", img.clone())]))
}

#[test]
fn e_blur_1_matches_project_copy_of_builtin_at_all_radii() {
    let img = detailed();
    let mut reference = builtin_effects()
        .into_iter()
        .find(|d| d.name == "gaussian_blur")
        .expect("built-in gaussian_blur");
    reference.name = "ref_blur".into();
    for radius in [1.0, 4.0, 13.0, 30.0] {
        let params = [("radius", radius)];
        let fast = render_with(None, "gaussian_blur", &params, &img);
        let slow = render_with(Some(reference.clone()), "ref_blur", &params, &img);
        assert_size(&fast, W, H);
        assert_size(&slow, W, H);
        assert_ne!(
            slow.rgba, img.rgba,
            "radius {radius}: reference blur changed the picture"
        );
        assert_ne!(
            fast.rgba, img.rgba,
            "radius {radius}: built-in blur changed the picture"
        );
        for y in 0..H {
            for x in 0..W {
                let (a, r) = (px(&fast, x, y), px(&slow, x, y));
                for ch in 0..4 {
                    assert!(
                        a[ch].abs_diff(r[ch]) <= 2,
                        "E-BLUR-1 radius {radius} at ({x},{y}) ch {ch}: built-in {a:?} vs reference {r:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn e_3_project_gaussian_blur_overrides_builtin() {
    let img = detailed();
    let red = def(
        "gaussian_blur",
        EffectKind::Filter,
        "fn effect(uv: vec2f) -> vec4f { return vec4f(1.0, 0.0, 0.0, src(uv).a); }",
        &[("radius", 4.0)],
    );
    for params in [&[][..], &[("radius", 13.0)][..]] {
        let f = render_with(Some(red.clone()), "gaussian_blur", params, &img);
        assert_size(&f, W, H);
        for y in 0..H {
            for x in 0..W {
                assert_eq!(px(&f, x, y), [255, 0, 0, 255], "E-3 at ({x},{y})");
            }
        }
    }
}
