//! SPEC V-1, V-2, V-3, V-7, V-8, V-10: canvas, layering, source images, blending, adjustment
//! clips, determinism. Expected pixels are hand-computed (8-bit = round(v * 255), ±2).

mod common;
use adit_core::*;
use adit_media::Frame;
use adit_render::RenderError;
use common::*;

/// 16x16 sequence with `n` video tracks.
fn small(n: usize) -> (B, Id, Vec<Id>) {
    let mut b = B::new();
    let s = b.seq(16, 16);
    let t = (0..n).map(|_| b.vtrack(s)).collect();
    (b, s, t)
}

// ------------------------------------------------------------------ V-1

#[test]
fn v1_frame_is_sequence_sized_and_canvas_opaque_black() {
    let mut b = B::new();
    let s = b.seq(40, 24);
    b.vtrack(s);
    for t in [0, S, 100 * S] {
        let f = render(&b.p, s, t, &Fake::new());
        assert_size(&f, 40, 24);
        for (i, p) in f.rgba.chunks(4).enumerate() {
            assert_eq!(p, [0, 0, 0, 255], "V-1 empty canvas pixel {i} at t={t}");
        }
    }
}

#[test]
fn v1_nested_sequence_renders_on_transparent_canvas() {
    let mut b = B::new();
    let inner = b.seq(16, 16);
    let it = b.vtrack(inner);
    let c = b.solid(it, 0, S, RED);
    b.set(c, "opacity", 0.5);
    let empty = b.seq(16, 16);
    b.vtrack(empty);
    let outer = b.seq(16, 16);
    let v1 = b.vtrack(outer);
    let v2 = b.vtrack(outer);
    b.solid(v1, 0, 2 * S, WHITE);
    b.seq_clip(v2, 0, S, inner);
    b.seq_clip(v2, S, S, empty);
    // Inner: red at sa=0.5 over (0,0,0,0) → s'=s, out.a=0.5, rgb=red → (1,0,0,0.5).
    // Outer: over white → r = 0.5*1+0.5*1 = 1 → 255; g,b = 0.5*0+0.5*1 = 0.5 → 128.
    // (An opaque-black nested canvas would give (128,0,0).)
    let f = render(&b.p, outer, ms(500), &Fake::new());
    assert_px(
        &f,
        8,
        8,
        [255, 128, 128],
        "V-1 nested half-transparent red over white",
    );
    // Empty nested sequence: fully transparent → white shows.
    let f = render(&b.p, outer, ms(1500), &Fake::new());
    assert_px(
        &f,
        8,
        8,
        [255, 255, 255],
        "V-1 empty nested sequence is transparent",
    );
}

// ------------------------------------------------------------------ V-2

#[test]
fn v2_tracks_draw_in_index_order() {
    let (mut b, s, t) = small(3);
    b.solid(t[0], 0, S, RED);
    b.solid(t[1], 0, S, GREEN);
    let top = b.solid(t[2], 0, S, BLUE);
    b.set(top, "crop_right", 0.5); // top layer covers only x < 8
    let f = render(&b.p, s, 0, &Fake::new());
    assert_px(&f, 2, 8, [0, 0, 255], "V-2 index 2 on top");
    assert_px(&f, 13, 8, [0, 255, 0], "V-2 index 1 above index 0");
}

#[test]
fn v2_hidden_track_and_disabled_clip_skipped() {
    let (mut b, s, t) = small(3);
    b.solid(t[0], 0, S, RED);
    b.solid(t[1], 0, S, GREEN);
    let c = b.solid(t[2], 0, S, BLUE);
    b.track_mut(t[1]).hidden = true;
    b.clip_mut(c).enabled = false;
    let f = render(&b.p, s, 0, &Fake::new());
    assert_px(
        &f,
        8,
        8,
        [255, 0, 0],
        "V-2 hidden track + disabled clip skipped",
    );
}

#[test]
fn v2_audio_tracks_draw_nothing() {
    let mut b = B::new();
    let s = b.seq(16, 16);
    let v = b.vtrack(s);
    let a = b.atrack(s);
    let m = b.media("av", Some((16, 16)), true);
    b.solid(v, 0, S, RED);
    b.media_clip(a, 0, S, m);
    let fake = Fake::images(vec![("av", solid_frame(16, 16, [255, 255, 255, 255]))]);
    let f = render(&b.p, s, 0, &fake);
    assert_px(&f, 8, 8, [255, 0, 0], "V-2 audio track above draws nothing");
}

#[test]
fn v2_clip_containing_time_is_drawn_half_open() {
    let (mut b, s, t) = small(1);
    b.solid(t[0], 0, S, RED);
    b.solid(t[0], S, S, GREEN);
    b.solid(t[0], 3 * S, S, BLUE);
    let cases: [(Time, [u8; 3]); 7] = [
        (0, [255, 0, 0]),
        (S - 1, [255, 0, 0]),
        (S, [0, 255, 0]),
        (2 * S - 1, [0, 255, 0]),
        (2 * S, [0, 0, 0]), // end is exclusive; gap → black canvas
        (ms(2500), [0, 0, 0]),
        (3 * S, [0, 0, 255]),
    ];
    for (time, rgb) in cases {
        let f = render(&b.p, s, time, &Fake::new());
        assert_px(&f, 8, 8, rgb, &format!("V-2 t={time}"));
    }
    let f = render(&b.p, s, 4 * S, &Fake::new());
    assert_px(&f, 8, 8, [0, 0, 0], "V-2 after last clip");
}

// ------------------------------------------------------------------ V-3

fn green(w: u32, h: u32) -> Frame {
    solid_frame(w, h, [0, 255, 0, 255])
}

#[test]
fn v3_media_smaller_than_canvas_is_centred_at_own_size() {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v = b.vtrack(s);
    let m = b.media("img", Some((16, 16)), false);
    b.media_clip(v, 0, S, m);
    let fake = Fake::images(vec![("img", green(16, 16))]);
    let f = render(&b.p, s, 0, &fake);
    assert_size(&f, 64, 64);
    // Centred 16x16 → canvas [24,40) in x and y.
    for (x, y) in [(24, 24), (39, 39), (32, 32), (24, 39), (39, 24)] {
        assert_px(&f, x, y, [0, 255, 0], "V-3 inside 16x16 at [24,40)");
    }
    for (x, y) in [(22, 32), (41, 32), (32, 22), (32, 41), (0, 0), (63, 63)] {
        assert_px(&f, x, y, [0, 0, 0], "V-3 outside 16x16 image");
    }
}

#[test]
fn v3_non_square_media_is_centred() {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v = b.vtrack(s);
    let m = b.media("img", Some((20, 10)), false);
    b.media_clip(v, 0, S, m);
    let fake = Fake::images(vec![("img", green(20, 10))]);
    let f = render(&b.p, s, 0, &fake);
    // 20x10 centred: x [22,42), y [27,37).
    for (x, y) in [(23, 28), (40, 35), (32, 32)] {
        assert_px(&f, x, y, [0, 255, 0], "V-3 inside 20x10");
    }
    for (x, y) in [(20, 32), (43, 32), (32, 25), (32, 38)] {
        assert_px(&f, x, y, [0, 0, 0], "V-3 outside 20x10");
    }
}

#[test]
fn v3_media_larger_than_canvas_is_not_fitted() {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v = b.vtrack(s);
    let m = b.media("big", Some((128, 128)), false);
    b.media_clip(v, 0, S, m);
    // 128x128: central [32,96)² green, border red. 1:1 centred → canvas shows exactly the green
    // centre; a fit-to-canvas would show red near the edges.
    let img = frame_fn(128, 128, |x, y| {
        if (32..96).contains(&x) && (32..96).contains(&y) {
            [0, 255, 0, 255]
        } else {
            [255, 0, 0, 255]
        }
    });
    let fake = Fake::images(vec![("big", img)]);
    let f = render(&b.p, s, 0, &fake);
    for (x, y) in [(0, 0), (63, 0), (0, 63), (63, 63), (32, 32), (1, 40)] {
        assert_px(&f, x, y, [0, 255, 0], "V-3 128x128 shown 1:1, centre crop");
    }
}

fn time_media(ch: usize) -> Fake {
    Fake::new().with_video(move |p, t| {
        if p == "t" {
            Ok(time_frame(t, ch, 16, 16))
        } else {
            Err(adit_media::MediaError::NotFound(p.into()))
        }
    })
}

#[test]
fn v3_media_frame_requested_at_source_time() {
    let mut b = B::new();
    let s = b.seq(16, 16);
    let v = b.vtrack(s);
    let m = b.media("t", Some((16, 16)), false);
    let c = b.media_clip(v, S, 2 * S, m);
    b.clip_mut(c).source_in = 2 * S;
    b.clip_mut(c).speed = 2.0;
    let fake = time_media(0);
    // Q3: local = 1.5s - 1s = 0.5s; source = 2s + round(0.5s * 2) = 3s = 2_116_800_000 → frame 90.
    let f = render(&b.p, s, ms(1500), &fake);
    assert_px(&f, 8, 8, [90, 0, 0], "V-3 media at source_time (frame 90)");
    assert!(
        fake.video_calls()
            .contains(&("t".to_string(), 2_116_800_000)),
        "V-3 video_frame(path, 3s) expected, calls: {:?}",
        fake.video_calls()
    );
}

#[test]
fn v3_reversed_media_clip_source_time() {
    let mut b = B::new();
    let s = b.seq(16, 16);
    let v = b.vtrack(s);
    let m = b.media("t", Some((16, 16)), false);
    let c = b.media_clip(v, 0, 30 * F30, m);
    b.clip_mut(c).reverse = true;
    // Q3 reversed: local 10 frames → source = round((30F - 10F) * 1) = 20F → frame 20.
    let f = render(&b.p, s, 10 * F30, &time_media(1));
    assert_px(&f, 8, 8, [0, 20, 0], "V-3 reversed clip shows frame 20");
}

#[test]
fn v3_unreadable_media_renders_transparent() {
    let (mut b, s, t) = small(2);
    b.solid(t[0], 0, S, RED);
    let m = b.media("missing.mp4", Some((16, 16)), false);
    b.media_clip(t[1], 0, S, m);
    let f = render(&b.p, s, 0, &Fake::new()); // every Fake call errors
    assert_px(
        &f,
        8,
        8,
        [255, 0, 0],
        "V-3 unreadable media is transparent, frame still Ok",
    );
}

#[test]
fn v3_missing_sequence_is_error() {
    let (b, _s, _t) = small(1);
    let r = try_render(&b.p, 9999, 0, &Fake::new());
    assert_eq!(r, Err(RenderError::SequenceNotFound(9999)));
}

#[test]
fn v3_solid_is_sequence_sized() {
    let mut b = B::new();
    let s = b.seq(48, 32);
    let v = b.vtrack(s);
    b.solid(v, 0, S, BLUE);
    let f = render(&b.p, s, 0, &Fake::new());
    for (x, y) in [(0, 0), (47, 0), (0, 31), (47, 31), (24, 16)] {
        assert_px(&f, x, y, [0, 0, 255], "V-3 solid covers the whole sequence");
    }
}

#[test]
fn v3_nested_sequence_at_source_time_and_own_size() {
    let mut b = B::new();
    let inner = b.seq(16, 16);
    let it = b.vtrack(inner);
    b.solid(it, 0, S, RED);
    b.solid(it, S, S, BLUE);
    let outer = b.seq(64, 64);
    let v = b.vtrack(outer);
    let c = b.seq_clip(v, ms(500), ms(500), inner);
    b.clip_mut(c).source_in = S;
    // t=0.75s: local 0.25s → inner time 1s + 0.25s = 1.25s → blue (outer time 0.75s would be red).
    let f = render(&b.p, outer, ms(750), &Fake::new());
    assert_px(&f, 32, 32, [0, 0, 255], "V-3 nested at source_time");
    // 16x16 nested on 64x64 at own size: [24,40).
    assert_px(&f, 25, 25, [0, 0, 255], "V-3 nested inside");
    assert_px(&f, 4, 4, [0, 0, 0], "V-3 nested not stretched");
    assert_px(&f, 60, 32, [0, 0, 0], "V-3 nested not stretched");
}

// ---- V-3 text (glyph shapes are font-dependent: checked for placement and colour only)

fn text_seq(
    content: &str,
    size: f64,
    color: [f32; 4],
    align: TextAlign,
    bg: Option<[f32; 4]>,
) -> (Project, Id) {
    let mut b = B::new();
    let s = b.seq(128, 64);
    let v1 = b.vtrack(s);
    let v2 = b.vtrack(s);
    if let Some(c) = bg {
        b.solid(v1, 0, S, c);
    }
    b.clip(
        v2,
        0,
        S,
        ClipSource::Text {
            text: TextSpec {
                content: content.into(),
                font: String::new(),
                size,
                color,
                align,
                bold: false,
                italic: false,
            },
        },
    );
    (b.p, s)
}

fn lit(f: &Frame) -> Vec<(u32, u32)> {
    let mut v = vec![];
    for y in 0..f.height {
        for x in 0..f.width {
            if px(f, x, y)[0] > 128 {
                v.push((x, y));
            }
        }
    }
    v
}

fn centroid(pts: &[(u32, u32)]) -> (f64, f64) {
    let n = pts.len() as f64;
    (
        pts.iter().map(|p| p.0 as f64).sum::<f64>() / n,
        pts.iter().map(|p| p.1 as f64).sum::<f64>() / n,
    )
}

#[test]
fn v3_text_is_drawn_centred_in_its_colour() {
    let (p, s) = text_seq("H", 32.0, RED, TextAlign::Center, None);
    let f = render(&p, s, 0, &Fake::new());
    assert_size(&f, 128, 64);
    let l = lit(&f);
    assert!(
        l.len() > 30,
        "V-3 text: 32px 'H' lights pixels, got {}",
        l.len()
    );
    let (cx, cy) = centroid(&l);
    assert!(
        (cx - 64.0).abs() <= 4.0,
        "V-3 text centred horizontally, centroid x {cx}"
    );
    assert!(
        (cy - 32.0).abs() <= 8.0,
        "V-3 text centred vertically, centroid y {cy}"
    );
    // Red text over black: green and blue stay 0 everywhere; some pixel is solid red.
    assert!(
        f.rgba.chunks(4).all(|p| p[1] <= 2 && p[2] <= 2),
        "V-3 text colour is red only"
    );
    assert!(
        f.rgba.chunks(4).any(|p| p[0] >= 240),
        "V-3 text has fully covered pixels"
    );
    assert_px(&f, 0, 0, [0, 0, 0], "V-3 text corner empty");
}

#[test]
fn v3_text_background_is_transparent() {
    let (p, s) = text_seq("H", 32.0, WHITE, TextAlign::Center, Some(BLUE));
    let f = render(&p, s, 0, &Fake::new());
    for (x, y) in [(2, 2), (125, 2), (2, 61), (125, 61)] {
        assert_px(
            &f,
            x,
            y,
            [0, 0, 255],
            "V-3 text image transparent outside glyphs",
        );
    }
}

#[test]
fn v3_text_size_scales_glyphs() {
    let (p, s) = text_seq("H", 12.0, WHITE, TextAlign::Center, None);
    let small = lit(&render(&p, s, 0, &Fake::new())).len();
    let (p, s) = text_seq("H", 36.0, WHITE, TextAlign::Center, None);
    let big = lit(&render(&p, s, 0, &Fake::new())).len();
    assert!(small > 0, "V-3 12px text visible");
    assert!(
        big > 3 * small,
        "V-3 36px text has ~9x the ink of 12px: {big} vs {small}"
    );
}

#[test]
fn v3_text_newline_breaks_lines() {
    let span = |c: &str| {
        let (p, s) = text_seq(c, 16.0, WHITE, TextAlign::Center, None);
        let l = lit(&render(&p, s, 0, &Fake::new()));
        let ys: Vec<u32> = l.iter().map(|p| p.1).collect();
        (*ys.iter().max().unwrap() - *ys.iter().min().unwrap()) as f64
    };
    let one = span("H");
    let two = span("H\nH");
    assert!(two >= 1.6 * one, "V-3 two lines are taller: {two} vs {one}");
    // Two lines stay centred as a block: check via a symmetric two-line text.
    let (p, s) = text_seq("H\nH", 16.0, WHITE, TextAlign::Center, None);
    let (cx, cy) = centroid(&lit(&render(&p, s, 0, &Fake::new())));
    assert!(
        (cx - 64.0).abs() <= 4.0 && (cy - 32.0).abs() <= 8.0,
        "V-3 block centred: ({cx},{cy})"
    );
}

#[test]
fn v3_text_align_positions_lines_within_block() {
    let lines = |align| {
        let (p, s) = text_seq("HHHH\nH", 16.0, WHITE, align, None);
        let l = lit(&render(&p, s, 0, &Fake::new()));
        let ys: Vec<u32> = l.iter().map(|p| p.1).collect();
        let mid = (*ys.iter().max().unwrap() + *ys.iter().min().unwrap()) / 2;
        let top: Vec<_> = l.iter().copied().filter(|p| p.1 < mid).collect();
        let bot: Vec<_> = l.iter().copied().filter(|p| p.1 > mid).collect();
        let minx = |v: &[(u32, u32)]| v.iter().map(|p| p.0).min().unwrap() as f64;
        let maxx = |v: &[(u32, u32)]| v.iter().map(|p| p.0).max().unwrap() as f64;
        (
            centroid(&top).0,
            centroid(&bot).0,
            minx(&top),
            minx(&bot),
            maxx(&top),
            maxx(&bot),
        )
    };
    let (t, bo, tmin, bmin, _, _) = lines(TextAlign::Left);
    assert!(
        (t - 64.0).abs() <= 4.0,
        "V-3 left-aligned block still centred: {t}"
    );
    assert!(
        bo < t - 8.0,
        "V-3 left: short line left of long line ({bo} vs {t})"
    );
    assert!(
        (bmin - tmin).abs() <= 3.0,
        "V-3 left: left edges match ({bmin} vs {tmin})"
    );
    let (t, bo, _, _, tmax, bmax) = lines(TextAlign::Right);
    assert!(
        bo > t + 8.0,
        "V-3 right: short line right of long line ({bo} vs {t})"
    );
    assert!(
        (bmax - tmax).abs() <= 3.0,
        "V-3 right: right edges match ({bmax} vs {tmax})"
    );
    let (t, bo, _, _, _, _) = lines(TextAlign::Center);
    assert!(
        (bo - t).abs() <= 3.0,
        "V-3 center: lines share a centre ({bo} vs {t})"
    );
}

// ------------------------------------------------------------------ V-7

#[test]
fn v7_spec_examples() {
    // White solid at opacity 0.5 on black → 0.5*255 = 127.5 → 128.
    let (mut b, s, t) = small(1);
    let c = b.solid(t[0], 0, S, WHITE);
    b.set(c, "opacity", 0.5);
    let f = render(&b.p, s, 0, &Fake::new());
    assert_px(&f, 8, 8, [128, 128, 128], "V-7 white at opacity 0.5");

    // Red under blue with Add → min(1, d + s) = (1, 0, 1).
    let (mut b, s, t) = small(2);
    b.solid(t[0], 0, S, RED);
    let c = b.solid(t[1], 0, S, BLUE);
    b.clip_mut(c).blend = BlendMode::Add;
    let f = render(&b.p, s, 0, &Fake::new());
    assert_px(&f, 8, 8, [255, 0, 255], "V-7 red + blue Add");

    // Grey 0.5 under grey 0.5 Multiply → 0.25 → 63.75 → 64.
    let (mut b, s, t) = small(2);
    b.solid(t[0], 0, S, [0.5, 0.5, 0.5, 1.0]);
    let c = b.solid(t[1], 0, S, [0.5, 0.5, 0.5, 1.0]);
    b.clip_mut(c).blend = BlendMode::Multiply;
    let f = render(&b.p, s, 0, &Fake::new());
    assert_px(&f, 8, 8, [64, 64, 64], "V-7 grey Multiply grey");
}

#[test]
fn v7_every_blend_mode_opaque() {
    // d = (0.25, 0.5, 0.75) below, s = (0.75, 0.25, 0.5) on top, both opaque → out = B(d, s).
    let table: [(BlendMode, [u8; 3]); 8] = [
        (BlendMode::Normal, [191, 64, 128]), // s: .75→191.25, .25→63.75, .5→127.5
        (BlendMode::Add, [255, 191, 255]),   // min(1, 1.0 / .75 / 1.25)
        (BlendMode::Multiply, [48, 32, 96]), // .1875→47.8, .125→31.9, .375→95.6
        (BlendMode::Screen, [207, 159, 223]), // 1-.75*.25=.8125→207.2, 1-.5*.75=.625→159.4, 1-.25*.5=.875→223.1
        (BlendMode::Overlay, [96, 64, 191]), // d<.5: 2*.25*.75=.375→95.6; d>=.5: 1-2*.5*.75=.25; 1-2*.25*.5=.75
        (BlendMode::Darken, [64, 64, 128]),  // min
        (BlendMode::Lighten, [191, 128, 191]), // max
        (BlendMode::Difference, [128, 64, 64]), // |d-s| = .5, .25, .25
    ];
    for (mode, rgb) in table {
        let (mut b, s, t) = small(2);
        b.solid(t[0], 0, S, [0.25, 0.5, 0.75, 1.0]);
        let c = b.solid(t[1], 0, S, [0.75, 0.25, 0.5, 1.0]);
        b.clip_mut(c).blend = mode;
        let f = render(&b.p, s, 0, &Fake::new());
        assert_px(&f, 8, 8, rgb, &format!("V-7 blend {mode:?}"));
    }
}

#[test]
fn v7_blend_uses_layer_alpha() {
    // d = 0.5 opaque, s = 0.5 Multiply at opacity 0.5: s' = B = 0.25 (da = 1);
    // out = 0.5*0.25 + 0.5*1*0.5 = 0.375 → 95.6 → 96.
    let (mut b, s, t) = small(2);
    b.solid(t[0], 0, S, [0.5, 0.5, 0.5, 1.0]);
    let c = b.solid(t[1], 0, S, [0.5, 0.5, 0.5, 1.0]);
    b.clip_mut(c).blend = BlendMode::Multiply;
    b.set(c, "opacity", 0.5);
    let f = render(&b.p, s, 0, &Fake::new());
    assert_px(&f, 8, 8, [96, 96, 96], "V-7 Multiply at opacity 0.5");
}

#[test]
fn v7_solid_alpha_multiplies_with_opacity() {
    // Solid (1,1,1,0.5) at opacity 0.5 → sa = 0.25 → 0.25*255 = 63.75 → 64.
    let (mut b, s, t) = small(1);
    let c = b.solid(t[0], 0, S, [1.0, 1.0, 1.0, 0.5]);
    b.set(c, "opacity", 0.5);
    let f = render(&b.p, s, 0, &Fake::new());
    assert_px(&f, 8, 8, [64, 64, 64], "V-7 solid alpha x opacity");
}

#[test]
fn v7_general_formula_on_transparent_nested_canvas() {
    let mut b = B::new();
    let inner = b.seq(16, 16);
    let i1 = b.vtrack(inner);
    let i2 = b.vtrack(inner);
    let c = b.solid(i1, 0, S, RED);
    b.set(c, "opacity", 0.5);
    let c = b.solid(i2, 0, S, BLUE);
    b.set(c, "opacity", 0.5);
    b.clip_mut(c).blend = BlendMode::Multiply;
    let outer = b.seq(16, 16);
    let v = b.vtrack(outer);
    b.seq_clip(v, 0, S, inner);
    // Inner after red: d = (1,0,0), da = 0.5.
    // Blue s = (0,0,1), sa = 0.5, Multiply: B = d*s = 0.
    // s' = (1-da)*s + da*B = (0,0,0.5); out.a = 0.5 + 0.5*0.5 = 0.75;
    // rgb = (0.5*(0,0,0.5) + 0.5*0.5*(1,0,0)) / 0.75 = (0.25,0,0.25)/0.75 = (1/3, 0, 1/3).
    // Outer over opaque black: 0.75 * (1/3, 0, 1/3) = (0.25, 0, 0.25) → (64, 0, 64).
    // (Ignoring da would give s' = 0 → (64, 0, 0).)
    let f = render(&b.p, outer, 0, &Fake::new());
    assert_px(&f, 8, 8, [64, 0, 64], "V-7 general formula with da = 0.5");
}

#[test]
fn v7_opacity_keyframes_evaluated_at_local_time() {
    let (mut b, s, t) = small(1);
    let c = b.solid(t[0], S, 2 * S, WHITE);
    b.keys(
        c,
        "opacity",
        &[(0, 0.0, Easing::Linear), (S, 1.0, Easing::Linear)],
    );
    // t = 1.5s → local 0.5s → opacity 0.5 → 128 (sequence-time evaluation would give 255).
    let f = render(&b.p, s, ms(1500), &Fake::new());
    assert_px(
        &f,
        8,
        8,
        [128, 128, 128],
        "V-7 opacity keyframe at local 0.5s",
    );
    // After the last keyframe: 1.0.
    let f = render(&b.p, s, ms(2500), &Fake::new());
    assert_px(&f, 8, 8, [255, 255, 255], "V-7 opacity after last keyframe");

    let (mut b, s, t) = small(1);
    let c = b.solid(t[0], S, 2 * S, WHITE);
    b.keys(
        c,
        "opacity",
        &[(0, 0.0, Easing::EaseIn), (S, 1.0, Easing::Linear)],
    );
    // EaseIn u = 0.5 → u² = 0.25 → 63.75 → 64.
    let f = render(&b.p, s, ms(1500), &Fake::new());
    assert_px(&f, 8, 8, [64, 64, 64], "V-7 opacity EaseIn at u=0.5");
}

// ------------------------------------------------------------------ V-8

#[test]
fn v8_adjustment_runs_effects_on_canvas_below_only() {
    let (mut b, s, t) = small(3);
    b.solid(t[0], 0, S, [0.25, 0.25, 0.25, 1.0]);
    let a = b.adjustment(t[1], 0, S);
    b.effect(a, "exposure", &[("stops", 1.0)]);
    let top = b.solid(t[2], 0, S, RED);
    b.set(top, "crop_right", 0.5); // red only on x < 8
    let f = render(&b.p, s, 0, &Fake::new());
    // 0.25 * 2^1 = 0.5 → 128.
    assert_px(
        &f,
        12,
        8,
        [128, 128, 128],
        "V-8 adjustment brightens canvas below",
    );
    assert_px(
        &f,
        2,
        8,
        [255, 0, 0],
        "V-8 layer above adjustment untouched",
    );
}

#[test]
fn v8_adjustment_includes_black_base_canvas() {
    let (mut b, s, t) = small(1);
    let a = b.adjustment(t[0], 0, S);
    b.effect(a, "invert", &[]);
    let f = render(&b.p, s, 0, &Fake::new());
    assert_px(
        &f,
        8,
        8,
        [255, 255, 255],
        "V-8 inverted opaque black canvas",
    );
}

#[test]
fn v8_adjustment_opacity_mixes_result() {
    let (mut b, s, t) = small(2);
    b.solid(t[0], 0, S, [0.25, 0.25, 0.25, 1.0]);
    let a = b.adjustment(t[1], 0, S);
    b.effect(a, "exposure", &[("stops", 1.0)]);
    b.set(a, "opacity", 0.5);
    // mix(0.25, 0.5, 0.5) = 0.375 → 95.6 → 96.
    let f = render(&b.p, s, 0, &Fake::new());
    assert_px(&f, 8, 8, [96, 96, 96], "V-8 adjustment at opacity 0.5");
}

#[test]
fn v8_adjustment_ignores_transform_crop_blend() {
    let (mut b, s, t) = small(2);
    b.solid(t[0], 0, S, [0.25, 0.25, 0.25, 1.0]);
    let a = b.adjustment(t[1], 0, S);
    b.effect(a, "exposure", &[("stops", 1.0)]);
    b.set(a, "position_x", 8.0);
    b.set(a, "scale_x", 0.5);
    b.set(a, "rotation", 45.0);
    b.set(a, "crop_left", 0.5);
    b.clip_mut(a).blend = BlendMode::Multiply; // would give 0.25*0.5 = 0.125 → 32
    let f = render(&b.p, s, 0, &Fake::new());
    for (x, y) in [(0, 0), (15, 15), (8, 8), (0, 15), (15, 0)] {
        assert_px(
            &f,
            x,
            y,
            [128, 128, 128],
            "V-8 transform/crop/blend ignored",
        );
    }
}

// ------------------------------------------------------------------ V-10

fn busy_project() -> (Project, Id) {
    let mut b = B::new();
    let inner = b.seq(16, 16);
    let it = b.vtrack(inner);
    b.solid(it, 0, 4 * S, [0.2, 0.7, 0.4, 0.8]);
    let s = b.seq(64, 48);
    let v1 = b.vtrack(s);
    let v2 = b.vtrack(s);
    let v3 = b.vtrack(s);
    let m = b.media("t", Some((16, 16)), false);
    let c = b.media_clip(v1, 0, 4 * S, m);
    b.set(c, "rotation", 30.0);
    b.set(c, "scale_x", 2.5);
    b.effect(c, "noise", &[("amount", 0.3), ("seed", 7.0)]);
    let l = b.solid(v2, 0, 2 * S, RED);
    let r = b.solid(v2, 2 * S, 2 * S, BLUE);
    b.effect(r, "gaussian_blur", &[("radius", 3.0)]);
    b.transition(v2, Some(l), Some(r), "cross_dissolve", S, &[]);
    b.set(l, "opacity", 0.7);
    let n = b.seq_clip(v3, 0, 4 * S, inner);
    b.clip_mut(n).blend = BlendMode::Screen;
    b.keys(
        n,
        "position_x",
        &[(0, -10.0, Easing::EaseInOut), (4 * S, 10.0, Easing::Linear)],
    );
    (b.p, s)
}

#[test]
fn v10_rendering_is_deterministic_and_pure() {
    let (p, s) = busy_project();
    let before = p.clone();
    let fake = time_media(0);
    let a = render(&p, s, ms(1750), &fake);
    let _other = render(&p, s, ms(300), &fake);
    let b = render(&p, s, ms(1750), &fake);
    assert!(a == b, "V-10 same project + time → identical frames");
    assert_eq!(p, before, "V-10 rendering does not modify the project");
}
