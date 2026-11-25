//! U-GEO-1, U-GEO-2, U-GEO-3, U-SNAP.
mod common;
use common::*;

fn layout(
    px_per_sec: f64,
    scroll: Time,
    header: f32,
    ruler: f32,
    track: f32,
    edge: f32,
) -> TimelineLayout {
    TimelineLayout {
        px_per_sec,
        scroll,
        header_width: header,
        ruler_height: ruler,
        track_height: track,
        edge_px: edge,
    }
}

// ------------------------------------------------------------------------------------ U-GEO-1

#[test]
fn u_geo_1_time_to_x_exact() {
    // 120 + (t - 2 s) * 150 / s
    let l = layout(150.0, 2 * S, 120.0, 30.0, 50.0, 6.0);
    assert_eq!(l.time_to_x(4 * S), 420.0);
    assert_eq!(l.time_to_x(2 * S), 120.0);
    assert_eq!(l.time_to_x(0), -180.0);
    assert_eq!(l.time_to_x(2 * S + F), 125.0);
    assert_eq!(l.time_to_x(2 * S + S / 2), 195.0);
    // 120 + (t - s/3) * 90 / s
    let l = layout(90.0, S / 3, 120.0, 30.0, 50.0, 6.0);
    assert_eq!(l.time_to_x(2 * S), 270.0);
    assert_eq!(l.time_to_x(S / 3), 120.0);
    assert_eq!(l.time_to_x(0), 90.0);
}

#[test]
fn u_geo_1_x_to_time_exact() {
    // 2 s + round((x - 120) * s / 150); s / 150 = 4_704_000.
    let l = layout(150.0, 2 * S, 120.0, 30.0, 50.0, 6.0);
    assert_eq!(l.x_to_time(420.0), 4 * S);
    assert_eq!(l.x_to_time(120.0), 2 * S);
    assert_eq!(l.x_to_time(121.0), 2 * S + 4_704_000);
    assert_eq!(l.x_to_time(120.5), 2 * S + 2_352_000);
    assert_eq!(l.x_to_time(20.0), 940_800_000);
    assert_eq!(l.x_to_time(0.0), 846_720_000);
    let l = layout(90.0, S / 3, 120.0, 30.0, 50.0, 6.0);
    assert_eq!(l.x_to_time(270.0), 2 * S);
    // s / 11 = 64_145_454.54..: rounds to nearest.
    let l = layout(11.0, 0, 120.0, 30.0, 50.0, 6.0);
    assert_eq!(l.x_to_time(121.0), 64_145_455);
    assert_eq!(l.x_to_time(122.0), 128_290_909);
    assert_eq!(l.x_to_time(119.0), -64_145_455);
}

#[test]
fn u_geo_1_inverse_within_one_pixel() {
    for px in [1.0, 7.5, 11.0, 100.0, 333.0, 2000.0] {
        for scroll in [0, 3 * S + 17, 100 * S] {
            let l = layout(px, scroll, 120.0, 30.0, 50.0, 6.0);
            let pixel = (S as f64 / px).ceil() as Time;
            for i in 0..200 {
                let t = scroll + i * (S / 7 + 12_345);
                let back = l.x_to_time(l.time_to_x(t));
                assert!(
                    (back - t).abs() <= pixel,
                    "px {px} scroll {scroll}: {t} -> {back}"
                );
            }
            let mut x = 120.0f32;
            while x < 1300.0 {
                let back = l.time_to_x(l.x_to_time(x));
                assert!((back - x).abs() <= 1.0, "px {px}: x {x} -> {back}");
                x += 7.3;
            }
        }
    }
}

// ------------------------------------------------------------------------------------ U-GEO-2

#[test]
fn u_geo_2_rows_order() {
    // `tracks` order: V1, A1, V2, A2, V3 (kinds interleaved).
    let p = B::new(FPS30)
        .track(V1, TrackKind::Video)
        .track(A1, TrackKind::Audio)
        .track(V2, TrackKind::Video)
        .track(A2, TrackKind::Audio)
        .track(V3, TrackKind::Video)
        .build();
    let l = layout(100.0, 0, 100.0, 20.0, 40.0, 5.0);
    assert_eq!(l.rows(&p, SEQ), vec![V3, V2, V1, A1, A2]);
    assert_eq!(
        l.rows(&p, STALE),
        Vec::<Id>::new(),
        "missing sequence: no rows"
    );
}

// ------------------------------------------------------------------------------------ U-GEO-3

/// Rows: V2 y [20,60), V1 [60,100), A1 [100,140). x = 100 + 100 * seconds.
/// V1: c100 [1s,3s) (200 px), c101 [5s,5.14s) (14 px), c102 [6s,6.15s) (15 px = 3 * edge).
/// A1: c103 [0,2s).
fn hit_project() -> Project {
    B::new(FPS30)
        .track(V1, TrackKind::Video)
        .track(V2, TrackKind::Video)
        .track(A1, TrackKind::Audio)
        .clip(V1, 100, S, 2 * S)
        .clip(V1, 101, 5 * S, 98_784_000)
        .clip(V1, 102, 6 * S, 105_840_000)
        .clip(A1, 103, 0, 2 * S)
        .build()
}

fn hl() -> TimelineLayout {
    layout(100.0, 0, 100.0, 20.0, 40.0, 5.0)
}

/// x_to_time for `hl()`: round((x - 100) * 7_056_000).
const PX: Time = 7_056_000;

fn at_clip(clip_id: Id, track_id: Id, part: ClipPart, time: Time) -> Hit {
    Hit::Clip {
        clip_id,
        track_id,
        part,
        time,
    }
}

#[test]
fn u_geo_3_ruler() {
    let (p, l) = (hit_project(), hl());
    assert_eq!(l.hit(&p, SEQ, 150.0, 10.0), Hit::Ruler { time: S / 2 });
    assert_eq!(l.hit(&p, SEQ, 100.0, 0.0), Hit::Ruler { time: 0 });
    assert_eq!(l.hit(&p, SEQ, 400.0, 19.5), Hit::Ruler { time: 3 * S });
    assert_eq!(
        l.hit(&p, SEQ, 99.0, 10.0),
        Hit::Nothing,
        "left of the header edge"
    );
    assert_eq!(l.hit(&p, SEQ, 0.0, 10.0), Hit::Nothing);
    assert_eq!(l.hit(&p, SEQ, 150.0, -1.0), Hit::Nothing, "y < 0");
}

#[test]
fn u_geo_3_header_and_outside_rows() {
    let (p, l) = (hit_project(), hl());
    assert_eq!(l.hit(&p, SEQ, 50.0, 70.0), Hit::Header { track_id: V1 });
    assert_eq!(l.hit(&p, SEQ, 99.5, 139.9), Hit::Header { track_id: A1 });
    assert_eq!(l.hit(&p, SEQ, 0.0, 20.0), Hit::Header { track_id: V2 });
    assert_eq!(
        l.hit(&p, SEQ, 50.0, 140.0),
        Hit::Nothing,
        "below the last row"
    );
    assert_eq!(l.hit(&p, SEQ, 300.0, 140.0), Hit::Nothing);
    assert_eq!(l.hit(&p, SEQ, 300.0, 1000.0), Hit::Nothing);
}

#[test]
fn u_geo_3_rows_boundaries_and_empty() {
    let (p, l) = (hit_project(), hl());
    assert_eq!(
        l.hit(&p, SEQ, 150.0, 20.0),
        Hit::Empty {
            track_id: V2,
            time: S / 2
        }
    );
    assert_eq!(
        l.hit(&p, SEQ, 300.0, 59.9),
        Hit::Empty {
            track_id: V2,
            time: 2 * S
        }
    );
    assert_eq!(
        l.hit(&p, SEQ, 300.0, 60.0),
        at_clip(100, V1, ClipPart::Body, 2 * S)
    );
    assert_eq!(
        l.hit(&p, SEQ, 450.0, 99.9),
        Hit::Empty {
            track_id: V1,
            time: 3 * S + S / 2
        }
    );
    // End is exclusive: x of c100's end hits empty space at 3 s.
    assert_eq!(
        l.hit(&p, SEQ, 400.0, 70.0),
        Hit::Empty {
            track_id: V1,
            time: 3 * S
        }
    );
    assert_eq!(
        l.hit(&p, SEQ, 150.0, 110.0),
        at_clip(103, A1, ClipPart::Body, S / 2)
    );
}

#[test]
fn u_geo_3_edges() {
    let (p, l) = (hit_project(), hl());
    let h = |x: f32| l.hit(&p, SEQ, x, 70.0);
    // c100: x 200..400. Start zone: x - 200 < 5. End zone: 400 - x <= 5.
    assert_eq!(h(200.0), at_clip(100, V1, ClipPart::StartEdge, S));
    assert_eq!(
        h(204.5),
        at_clip(100, V1, ClipPart::StartEdge, 104 * PX + PX / 2)
    );
    assert_eq!(h(205.0), at_clip(100, V1, ClipPart::Body, 105 * PX));
    assert_eq!(h(394.0), at_clip(100, V1, ClipPart::Body, 294 * PX));
    assert_eq!(h(395.0), at_clip(100, V1, ClipPart::EndEdge, 295 * PX));
    assert_eq!(h(399.0), at_clip(100, V1, ClipPart::EndEdge, 299 * PX));
    // A1 clip at the header edge: x 100 = time 0 = its start.
    assert_eq!(
        l.hit(&p, SEQ, 100.0, 110.0),
        at_clip(103, A1, ClipPart::StartEdge, 0)
    );
}

#[test]
fn u_geo_3_edges_need_three_edge_widths() {
    let (p, l) = (hit_project(), hl());
    let h = |x: f32| l.hit(&p, SEQ, x, 70.0);
    // c101 is 14 px wide (< 15): body everywhere.
    assert_eq!(h(600.0), at_clip(101, V1, ClipPart::Body, 5 * S));
    assert_eq!(h(613.0), at_clip(101, V1, ClipPart::Body, 513 * PX));
    // c102 is exactly 15 px wide: edges active.
    assert_eq!(h(700.0), at_clip(102, V1, ClipPart::StartEdge, 6 * S));
    assert_eq!(
        h(707.5),
        at_clip(102, V1, ClipPart::Body, 607 * PX + PX / 2)
    );
    assert_eq!(h(712.0), at_clip(102, V1, ClipPart::EndEdge, 612 * PX));
}

#[test]
fn u_geo_3_scrolled() {
    let p = hit_project();
    let l = layout(100.0, 2 * S, 100.0, 20.0, 40.0, 5.0);
    // c100 [1s,3s) now spans x 0..200; x 100 = 2 s.
    assert_eq!(
        l.hit(&p, SEQ, 100.0, 70.0),
        at_clip(100, V1, ClipPart::Body, 2 * S)
    );
    assert_eq!(
        l.hit(&p, SEQ, 196.0, 70.0),
        at_clip(100, V1, ClipPart::EndEdge, 2 * S + 96 * PX)
    );
    assert_eq!(l.hit(&p, SEQ, 50.0, 70.0), Hit::Header { track_id: V1 });
    assert_eq!(
        l.hit(&p, SEQ, 150.0, 5.0),
        Hit::Ruler {
            time: 2 * S + S / 2
        }
    );
}

// -------------------------------------------------------------------------------------- U-SNAP

/// 0.6 frame: the nearest frame of `x + G` is `x + F` when `x` is on a frame.
const G: Time = 14_112_000;

/// c100 [1s,2s); c101 [5s+1000, 6s+1000) (not frame aligned); marker 9s+333; other sequence's
/// clip [3s+5000, ...).
fn snap_project() -> Project {
    B::std()
        .clip(V1, 100, S, S)
        .clip(A1, 101, 5 * S + 1000, S)
        .marker(500, 9 * S + 333)
        .other_seq_clip(200, 3 * S + 5000, S)
        .build()
}

fn snap_ui() -> UiState {
    let mut u = ui();
    u.playhead = 10 * S;
    u.mark_in = Some(7 * S + 333);
    u.mark_out = Some(8 * S + 333);
    u
}

#[test]
fn u_snap_each_candidate_kind() {
    let (p, u) = (snap_project(), snap_ui());
    let sn = |t: Time, tol: Time| snap(t, &p, &u, &[], tol);
    assert_eq!(sn(G, G), 0, "0");
    assert_eq!(sn(10 * S + G, G), 10 * S, "playhead");
    assert_eq!(sn(7 * S + 833, 500), 7 * S + 333, "mark_in");
    assert_eq!(sn(8 * S - 67, 400), 8 * S + 333, "mark_out");
    assert_eq!(sn(9 * S + 343, 10), 9 * S + 333, "marker");
    assert_eq!(sn(S - G, G), S, "clip start");
    assert_eq!(sn(2 * S + G, G), 2 * S, "clip end");
    assert_eq!(sn(5 * S + 1500, 500), 5 * S + 1000, "unaligned start as is");
    assert_eq!(sn(6 * S + 900, 100), 6 * S + 1000, "unaligned end as is");
}

#[test]
fn u_snap_tolerance_inclusive_then_nearest_frame() {
    let (p, u) = (snap_project(), snap_ui());
    let sn = |t: Time, tol: Time| snap(t, &p, &u, &[], tol);
    assert_eq!(sn(G, G - 1), F);
    assert_eq!(sn(10 * S + G, G - 1), 10 * S + F);
    assert_eq!(sn(5 * S + 1500, 499), 5 * S);
    assert_eq!(sn(7 * S + 833, 499), 7 * S);
    assert_eq!(sn(S - G, 0), S - F);
    // The other sequence's clip is not a candidate.
    assert_eq!(sn(3 * S + 5100, 200), 3 * S);
}

#[test]
fn u_snap_nearest_and_ties() {
    let (p, u) = (snap_project(), snap_ui());
    let sn = |t: Time, tol: Time| snap(t, &p, &u, &[], tol);
    assert_eq!(sn(S + 2 * S / 5, S), S, "1.4 s: 1 s is nearer than 2 s");
    assert_eq!(sn(S + 3 * S / 5, S), 2 * S, "1.6 s: 2 s is nearer than 1 s");
    // Tie between c100's end (2 s) and mark_in (2 s + 2000): the smaller.
    let mut u2 = snap_ui();
    u2.mark_in = Some(2 * S + 2000);
    assert_eq!(snap(2 * S + 1000, &p, &u2, &[], 1000), 2 * S);
    u2.mark_in = Some(2 * S - 2000);
    assert_eq!(snap(2 * S - 1000, &p, &u2, &[], 1000), 2 * S - 2000);
}

#[test]
fn u_snap_exclude() {
    let (p, u) = (snap_project(), snap_ui());
    assert_eq!(snap(5 * S + 1500, &p, &u, &[101], 500), 5 * S);
    assert_eq!(snap(6 * S + 900, &p, &u, &[101], 100), 6 * S);
    assert_eq!(snap(S + G, &p, &u, &[100], G), S + F);
    assert_eq!(snap(5 * S + 1500, &p, &u, &[100], 500), 5 * S + 1000);
}

#[test]
fn u_snap_off_rounds_to_nearest_frame() {
    let p = snap_project();
    let mut u = snap_ui();
    u.snap = false;
    assert_eq!(snap(5 * S + 1500, &p, &u, &[], 500), 5 * S);
    assert_eq!(snap(G, &p, &u, &[], G), F);
    assert_eq!(snap(7 * S + 833, &p, &u, &[], S), 7 * S);
    assert_eq!(snap(S - G, &p, &u, &[], S), S - F);
    // 30000/1001: 10 s is frame 299.7 -> nearest frame 300 = 7_063_056_000 (floor would be 299).
    let ntsc = B::new(NTSC).track(V1, TrackKind::Video).build();
    let mut u = ui();
    u.snap = false;
    assert_eq!(snap(10 * S, &ntsc, &u, &[], 0), 7_063_056_000);
    u.snap = true;
    assert_eq!(snap(10 * S, &ntsc, &u, &[], 0), 7_063_056_000);
}
