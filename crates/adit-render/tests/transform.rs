//! SPEC V-5 (crop) and V-6 (transform). Canvas 64x64, centre (32,32), y down, rotation clockwise.
//! Pixels are probed at least one pixel away from bilinear edges.

mod common;
use adit_core::*;
use adit_media::Frame;
use common::*;

/// 64x64 sequence with one media clip of `img` (path "img") and the given clip params.
fn place(img: Frame, params: &[(&str, f64)]) -> Frame {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v = b.vtrack(s);
    let m = b.media("img", Some((img.width, img.height)), false);
    let c = b.media_clip(v, 0, S, m);
    for (k, val) in params {
        b.set(c, k, *val);
    }
    render(&b.p, s, 0, &Fake::images(vec![("img", img)]))
}

fn green(w: u32, h: u32) -> Frame {
    solid_frame(w, h, [0, 255, 0, 255])
}

fn white(w: u32, h: u32) -> Frame {
    solid_frame(w, h, [255, 255, 255, 255])
}

const G: [u8; 3] = [0, 255, 0];
const K: [u8; 3] = [0, 0, 0];
const W: [u8; 3] = [255, 255, 255];

fn check(f: &Frame, inside: &[(u32, u32)], outside: &[(u32, u32)], col: [u8; 3], what: &str) {
    for &(x, y) in inside {
        assert_px(f, x, y, col, &format!("{what}: inside"));
    }
    for &(x, y) in outside {
        assert_px(f, x, y, K, &format!("{what}: outside"));
    }
}

// ------------------------------------------------------------------ V-5

#[test]
fn v5_crop_each_side_of_solid() {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v = b.vtrack(s);
    let c = b.solid(v, 0, S, WHITE);
    // left .25 → x < 16 cut; right .125 → x >= 56 cut; top .5 → y < 32 cut; bottom .25 → y >= 48 cut.
    b.set(c, "crop_left", 0.25);
    b.set(c, "crop_right", 0.125);
    b.set(c, "crop_top", 0.5);
    b.set(c, "crop_bottom", 0.25);
    let f = render(&b.p, s, 0, &Fake::new());
    check(
        &f,
        &[(18, 40), (53, 40), (30, 34), (30, 45), (17, 33), (54, 46)],
        &[(13, 40), (58, 40), (30, 29), (30, 50), (2, 2), (62, 62)],
        W,
        "V-5 crop all sides",
    );
}

#[test]
fn v5_crop_is_fraction_of_image_not_canvas() {
    // 32x32 image at canvas [16,48); crop_left .25 of the image = 8 px → visible x in [24,48).
    // (Canvas-relative cropping would cut x < 16 instead and leave x=20 white.)
    let f = place(white(32, 32), &[("crop_left", 0.25)]);
    check(
        &f,
        &[(26, 32), (45, 32)],
        &[(20, 32), (50, 32)],
        W,
        "V-5 image-relative crop",
    );
}

#[test]
fn v5_crop_happens_in_image_space_before_transform() {
    // crop_left .5 of 32x32 → image columns [16,32) remain; moved +10 px → canvas [42,58).
    let f = place(white(32, 32), &[("crop_left", 0.5), ("position_x", 10.0)]);
    check(
        &f,
        &[(44, 32), (56, 32)],
        &[(40, 32), (60, 32), (30, 32)],
        W,
        "V-5 crop then move",
    );
}

// ------------------------------------------------------------------ V-6

#[test]
fn v6_position_moves_in_canvas_pixels() {
    // 16x16 centred at [24,40); +10 x, -5 y → x [34,50), y [19,35).
    let f = place(green(16, 16), &[("position_x", 10.0), ("position_y", -5.0)]);
    check(
        &f,
        &[(35, 20), (48, 33), (42, 27)],
        &[(32, 27), (52, 27), (40, 17), (40, 37)],
        G,
        "V-6 position",
    );
}

#[test]
fn v6_non_uniform_scale_about_centre() {
    // 16x16 scaled (2, 0.5) about its centre (32,32) → 32x8: x [16,48), y [28,36).
    let f = place(green(16, 16), &[("scale_x", 2.0), ("scale_y", 0.5)]);
    check(
        &f,
        &[(17, 29), (46, 34), (32, 32)],
        &[(14, 32), (50, 32), (32, 26), (32, 38)],
        G,
        "V-6 scale (2, 0.5)",
    );
}

#[test]
fn v6_rotation_90_is_clockwise() {
    // Quadrants (TL red, TR green, BL blue, BR white) 32x32 at [16,48).
    // 90° clockwise (y down): TL→TR, TR→BR, BR→BL, BL→TL.
    let f = place(quadrants(32, 32), &[]);
    assert_px(&f, 24, 24, [255, 0, 0], "V-6 unrotated TL red");
    let f = place(quadrants(32, 32), &[("rotation", 90.0)]);
    assert_px(&f, 24, 24, [0, 0, 255], "V-6 rot 90 cw: TL shows blue");
    assert_px(&f, 40, 24, [255, 0, 0], "V-6 rot 90 cw: TR shows red");
    assert_px(&f, 40, 40, [0, 255, 0], "V-6 rot 90 cw: BR shows green");
    assert_px(&f, 24, 40, [255, 255, 255], "V-6 rot 90 cw: BL shows white");
}

#[test]
fn v6_rotation_180() {
    let f = place(quadrants(32, 32), &[("rotation", 180.0)]);
    assert_px(&f, 24, 24, [255, 255, 255], "V-6 rot 180: TL shows white");
    assert_px(&f, 40, 24, [0, 0, 255], "V-6 rot 180: TR shows blue");
    assert_px(&f, 24, 40, [0, 255, 0], "V-6 rot 180: BL shows green");
    assert_px(&f, 40, 40, [255, 0, 0], "V-6 rot 180: BR shows red");
    assert_px(&f, 12, 32, [0, 0, 0], "V-6 rot 180 stays inside [16,48)");
}

#[test]
fn v6_rotation_90_of_non_square_image() {
    // 32x16 (x [16,48), y [24,40)) rotated 90° about centre → 16x32: x [24,40), y [16,48).
    let f = place(green(32, 16), &[("rotation", 90.0)]);
    check(
        &f,
        &[(32, 18), (32, 45), (26, 32), (37, 32)],
        &[(20, 32), (43, 32), (32, 13), (32, 50)],
        G,
        "V-6 rot 90 of 32x16",
    );
}

#[test]
fn v6_anchor_alone_does_not_move_image() {
    // Anchor only sets the pivot of scale/rotation; with neither the image stays at [24,40).
    let f = place(green(16, 16), &[("anchor_x", 8.0), ("anchor_y", -8.0)]);
    check(
        &f,
        &[(25, 25), (38, 38)],
        &[(22, 32), (41, 32), (32, 22), (32, 41)],
        G,
        "V-6 anchor only",
    );
}

#[test]
fn v6_anchor_is_pivot_for_scale() {
    // anchor_x 8 → pivot at image right-edge middle = canvas (40,32). scale_x 2 about x=40:
    // [24,40) → [8,40). (Scaling about the centre would give [16,48).)
    let f = place(green(16, 16), &[("anchor_x", 8.0), ("scale_x", 2.0)]);
    check(
        &f,
        &[(10, 32), (38, 32), (24, 25)],
        &[(6, 32), (42, 32), (24, 22)],
        G,
        "V-6 anchor + scale",
    );
}

#[test]
fn v6_anchor_is_pivot_for_rotation() {
    // anchor_x 8 → pivot (40,32); 180° → x [40,56), y [24,40).
    let f = place(green(16, 16), &[("anchor_x", 8.0), ("rotation", 180.0)]);
    check(
        &f,
        &[(42, 26), (54, 38)],
        &[(37, 30), (58, 30), (30, 30)],
        G,
        "V-6 anchor + rot 180",
    );
    // anchor_y 8 → pivot at bottom middle (32,40). 90° cw maps offset (dx,dy) → (-dy,dx):
    // dx∈[-8,8], dy∈[-16,0] → x [32,48), y [32,48).
    let f = place(green(16, 16), &[("anchor_y", 8.0), ("rotation", 90.0)]);
    check(
        &f,
        &[(34, 34), (46, 46)],
        &[(30, 40), (40, 30), (50, 40)],
        G,
        "V-6 anchor_y + rot 90",
    );
}

#[test]
fn v6_transform_params_evaluated_at_local_time() {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v = b.vtrack(s);
    let m = b.media("img", Some((16, 16)), false);
    let c = b.media_clip(v, S, 2 * S, m);
    b.keys(
        c,
        "position_x",
        &[(0, 0.0, Easing::Linear), (2 * S, 20.0, Easing::Linear)],
    );
    // t = 2s → local 1s → position_x 10 → x [34,50). (Sequence time 2s would give 20 → [44,60).)
    let f = render(&b.p, s, 2 * S, &Fake::images(vec![("img", green(16, 16))]));
    check(
        &f,
        &[(35, 32), (48, 32)],
        &[(32, 32), (52, 32)],
        G,
        "V-6 keyframed position",
    );
}

#[test]
fn v6_outside_image_is_transparent_over_lower_track() {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v1 = b.vtrack(s);
    let v2 = b.vtrack(s);
    b.solid(v1, 0, S, BLUE);
    let m = b.media("img", Some((16, 16)), false);
    let c = b.media_clip(v2, 0, S, m);
    b.set(c, "rotation", 45.0);
    let f = render(&b.p, s, 0, &Fake::images(vec![("img", green(16, 16))]));
    assert_px(&f, 32, 32, [0, 255, 0], "V-6 rotated image centre");
    assert_px(
        &f,
        2,
        2,
        [0, 0, 255],
        "V-6 outside transformed image is transparent",
    );
    // 45° square: pixel (24,24) (offset (-7.5,-7.5) from centre) has rotated coordinate
    // u = (dx+dy)/sqrt(2) = -10.6, beyond the half-side 8 → outside.
    assert_px(
        &f,
        24,
        24,
        [0, 0, 255],
        "V-6 45° rotation leaves old corner empty",
    );
}
