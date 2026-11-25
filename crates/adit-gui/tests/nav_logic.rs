//! Timeline navigation logic (docs/GUI.md U-NAV-1, U-NAV-2). Written from the contract only; the
//! implementation was not read. Expected values are hand-computed literals (S = 705_600_000
//! flicks per second).
//!
//! Readings taken where the contract names no seam:
//! - a non-whole `px_per_sec` is compared within 1e-9, times derived from it within 1 flick;
//! - `zoom_bar` fractions are compared within 1e-12;
//! - `bar_drag` is applied to a `UiState` that may already differ from the one at `bar_grab`
//!   (the contract says the result depends only on the grab, `d` and `view_width`).
mod common;
use common::*;

fn ui(pps: f64, scroll: Time, vw: f32) -> UiState {
    let mut u = UiState::new(SEQ);
    u.px_per_sec = pps;
    u.scroll = scroll;
    u.view_width = vw;
    u
}

fn near(a: f64, b: f64, what: &str) {
    assert!((a - b).abs() < 1e-9, "{what}: got {a}, want {b}");
}

fn near_t(a: Time, b: Time, what: &str) {
    assert!((a - b).abs() <= 1, "{what}: got {a}, want {b} (+-1 flick)");
}

// ------------------------------------------------------------------------------- visible_span

#[test]
fn u_nav_visible_span_values() {
    assert_eq!(
        visible_span(&ui(100.0, 0, 1000.0)),
        10 * S,
        "1000 px at 100 px/s"
    );
    assert_eq!(visible_span(&ui(100.0, 0, 1500.0)), 10_584_000_000);
    assert_eq!(visible_span(&ui(300.0, 0, 1000.0)), 2_352_000_000);
    assert_eq!(visible_span(&ui(7.0, 0, 1000.0)), 100_800_000_000);
    assert_eq!(
        visible_span(&ui(11.0, 0, 1000.0)),
        64_145_454_545,
        "rounded to the nearest flick"
    );
}

// ------------------------------------------------------------------------------- U-NAV-1

#[test]
fn u_nav_1_example_zoom_in_keeps_anchor() {
    let mut u = ui(100.0, 0, 1000.0);
    zoom_at(&mut u, 2.0, 4 * S);
    near(u.px_per_sec, 200.0, "U-NAV-1 pps");
    assert_eq!(u.scroll, 2 * S, "U-NAV-1: anchor 4 s stays at screen x");
}

#[test]
fn u_nav_1_example_zoom_out_scroll_clamps_at_zero() {
    let mut u = ui(100.0, 0, 1000.0);
    zoom_at(&mut u, 0.5, S);
    near(u.px_per_sec, 50.0, "U-NAV-1 pps");
    assert_eq!(u.scroll, 0, "U-NAV-1: 1 s - 2 s < 0 -> 0");
}

#[test]
fn u_nav_1_example_max_zoom_clamps() {
    let mut u = ui(1500.0, 10 * S, 1000.0);
    zoom_at(&mut u, 2.0, 10 * S);
    near(u.px_per_sec, 2000.0, "U-NAV-1 pps clamped at 2000");
    assert_eq!(u.scroll, 10 * S);
}

#[test]
fn u_nav_1_min_zoom_clamps_at_one() {
    let mut u = ui(1.5, 0, 1000.0);
    zoom_at(&mut u, 0.1, 0);
    near(u.px_per_sec, 1.0, "U-NAV-1 pps clamped at 1");
    assert_eq!(u.scroll, 0);
}

#[test]
fn u_nav_1_zoom_out_with_scroll_keeps_anchor() {
    // pps 4 -> 2; scroll = 30 s - round(10 s * 4 / 2) = 10 s.
    let mut u = ui(4.0, 20 * S, 1000.0);
    zoom_at(&mut u, 0.5, 30 * S);
    near(u.px_per_sec, 2.0, "U-NAV-1 pps");
    assert_eq!(u.scroll, 10 * S);
}

#[test]
fn u_nav_1_factor_one_keeps_view() {
    let mut u = ui(100.0, 3 * S, 1000.0);
    zoom_at(&mut u, 1.0, 5 * S);
    near(u.px_per_sec, 100.0, "U-NAV-1 pps");
    assert_eq!(u.scroll, 3 * S);
}

#[test]
fn u_nav_1_rounds_scroll_to_flick() {
    // new 300; scroll = 1 s - round(1 s * 100 / 300) = 705_600_000 - 235_200_000.
    let mut u = ui(100.0, 0, 1000.0);
    zoom_at(&mut u, 3.0, S);
    near(u.px_per_sec, 300.0, "U-NAV-1 pps");
    near_t(u.scroll, 470_400_000, "U-NAV-1 scroll");
}

#[test]
fn u_nav_1_anchor_left_of_scroll() {
    // anchor before the left edge: max(0, 0 - round((0 - 5 s) * 100 / 200)) = 2.5 s = 1_764_000_000.
    let mut u = ui(100.0, 5 * S, 1000.0);
    zoom_at(&mut u, 2.0, 0);
    near(u.px_per_sec, 200.0, "U-NAV-1 pps");
    assert_eq!(
        u.scroll, 1_764_000_000,
        "U-NAV-1 formula holds for any anchor"
    );
}

#[test]
fn u_nav_1_invalid_factor_changes_nothing() {
    for f in [f64::NAN, 0.0, -2.0, -0.0, f64::INFINITY, f64::NEG_INFINITY] {
        let mut u = ui(123.0, 7 * S, 1000.0);
        let before = u.clone();
        zoom_at(&mut u, f, 9 * S);
        assert_eq!(u, before, "U-NAV-1: factor {f} leaves ui unchanged");
    }
}

// ------------------------------------------------------------------------------- U-NAV-2

#[test]
fn u_nav_2_total_values() {
    let u = ui(100.0, 5 * S, 1000.0);
    assert_eq!(zoom_bar_total(&u, 40 * S), 40 * S, "duration wins");
    assert_eq!(zoom_bar_total(&u, 4 * S), 15 * S, "scroll + visible wins");
    let v = ui(100.0, 0, 1000.0);
    assert_eq!(zoom_bar_total(&v, 4 * S), 10 * S, "visible wins");
    let w = ui(2000.0, 0, 1000.0);
    assert_eq!(zoom_bar_total(&w, 141_120_000), S, "the 1 s floor wins");
    assert_eq!(zoom_bar_total(&w, 0), S, "empty sequence: floor");
}

#[test]
fn u_nav_2_zoom_bar_examples() {
    let (a, b) = zoom_bar(&ui(100.0, 5 * S, 1000.0), 40 * S);
    near(a, 0.125, "U-NAV-2 a");
    near(b, 0.375, "U-NAV-2 b");
    let (a, b) = zoom_bar(&ui(100.0, 0, 1000.0), 4 * S);
    near(a, 0.0, "U-NAV-2 a (short sequence)");
    near(b, 1.0, "U-NAV-2 b (short sequence)");
}

#[test]
fn u_nav_2_zoom_bar_floor_and_overscroll() {
    // visible 0.5 s, floor total 1 s.
    let (a, b) = zoom_bar(&ui(2000.0, 0, 1000.0), 141_120_000);
    near(a, 0.0, "U-NAV-2 a");
    near(b, 0.5, "U-NAV-2 b");
    // scrolled past the end: total = 50 + 10 = 60 s.
    let (a, b) = zoom_bar(&ui(100.0, 50 * S, 1000.0), 40 * S);
    near(a, 50.0 / 60.0, "U-NAV-2 a (past the end)");
    near(b, 1.0, "U-NAV-2 b (past the end)");
}

#[test]
fn u_nav_2_bar_grab_records_state() {
    let u = ui(100.0, 5 * S, 1000.0);
    for part in [BarPart::Thumb, BarPart::Start, BarPart::End] {
        let g = bar_grab(&u, 40 * S, part);
        assert_eq!(
            g,
            BarGrab {
                part,
                scroll: 5 * S,
                visible: 10 * S,
                total: 40 * S
            },
            "U-NAV-2: grab records part, scroll, visible, total"
        );
    }
}

fn grab() -> (UiState, BarGrab, BarGrab, BarGrab) {
    let u = ui(100.0, 5 * S, 1000.0);
    let t = bar_grab(&u, 40 * S, BarPart::Thumb);
    let s = bar_grab(&u, 40 * S, BarPart::Start);
    let e = bar_grab(&u, 40 * S, BarPart::End);
    (u, t, s, e)
}

#[test]
fn u_nav_2_thumb_examples() {
    let (u0, t, _, _) = grab();
    for (d, want, what) in [
        (0.25, 15 * S, "d 0.25"),
        (1.0, 30 * S, "d 1 clamps to total - visible"),
        (-0.5, 0, "d -0.5 clamps to 0"),
    ] {
        let mut u = u0.clone();
        bar_drag(&mut u, &t, d);
        assert_eq!(u.scroll, want, "U-NAV-2 Thumb {what}");
        near(u.px_per_sec, 100.0, "U-NAV-2 Thumb keeps pps");
    }
}

#[test]
fn u_nav_2_start_drag_keeps_right_end() {
    let (u0, _, s, _) = grab();
    // d -0.125: start 0, pps 1000/15, scroll max(0, 15 s - 15 s) = 0.
    let mut u = u0.clone();
    bar_drag(&mut u, &s, -0.125);
    near(u.px_per_sec, 1000.0 / 15.0, "U-NAV-2 Start d -0.125 pps");
    near_t(u.scroll, 0, "U-NAV-2 Start d -0.125 scroll");
    // d 0.125: start 10 s, pps 200, scroll 15 s - 5 s.
    let mut u = u0.clone();
    bar_drag(&mut u, &s, 0.125);
    near(u.px_per_sec, 200.0, "U-NAV-2 Start d 0.125 pps");
    assert_eq!(u.scroll, 10 * S, "U-NAV-2 Start d 0.125 scroll");
    // d 1: start capped at end0 - 1 flick, pps 2000, scroll 14.5 s.
    let mut u = u0.clone();
    bar_drag(&mut u, &s, 1.0);
    near(u.px_per_sec, 2000.0, "U-NAV-2 Start d 1 pps");
    assert_eq!(u.scroll, 10_231_200_000, "U-NAV-2 Start d 1 scroll");
}

#[test]
fn u_nav_2_end_drag_keeps_left_end() {
    let (u0, _, _, e) = grab();
    // d 0.25: end 25 s, pps 1000 / 20 = 50.
    let mut u = u0.clone();
    bar_drag(&mut u, &e, 0.25);
    near(u.px_per_sec, 50.0, "U-NAV-2 End d 0.25 pps");
    assert_eq!(u.scroll, 5 * S, "U-NAV-2 End d 0.25 scroll");
    // d -0.25: end = max(5 s, 5 s + 1 flick) -> pps clamps at 2000.
    let mut u = u0.clone();
    bar_drag(&mut u, &e, -0.25);
    near(u.px_per_sec, 2000.0, "U-NAV-2 End d -0.25 pps");
    assert_eq!(u.scroll, 5 * S);
    // d 100: pps clamps at 1.
    let mut u = u0.clone();
    bar_drag(&mut u, &e, 100.0);
    near(u.px_per_sec, 1.0, "U-NAV-2 End d 100 pps");
    assert_eq!(u.scroll, 5 * S);
}

#[test]
fn u_nav_2_end_drag_other_view_width() {
    // view 500 px at 100 px/s: visible 5 s, total 40 s; d 0.125 -> end 10 s, pps 500 / 5... end -
    // scroll = 10 s, pps = 500 / 10 = 50.
    let u0 = ui(100.0, 0, 500.0);
    let e = bar_grab(&u0, 40 * S, BarPart::End);
    assert_eq!(e.visible, 5 * S, "U-NAV-2 grab uses view_width");
    let mut u = u0.clone();
    bar_drag(&mut u, &e, 0.125);
    near(u.px_per_sec, 50.0, "U-NAV-2 End pps with view 500");
    assert_eq!(u.scroll, 0);
}

#[test]
fn u_nav_2_bar_drag_ignores_current_state() {
    // Result depends only on grab, d and view_width: a ui already moved by a former drag frame
    // ends where a fresh one does.
    let (u0, t, s, e) = grab();
    for (g, d) in [(t, 0.2), (s, 0.1), (e, 0.15)] {
        let mut fresh = u0.clone();
        bar_drag(&mut fresh, &g, d);
        let mut moved = u0.clone();
        bar_drag(&mut moved, &g, 0.9);
        bar_drag(&mut moved, &g, d);
        assert_eq!(
            moved, fresh,
            "U-NAV-2 {:?}: same result from any prior state",
            g.part
        );
    }
}

#[test]
fn u_nav_2_bar_drag_every_frame_equals_once() {
    let (u0, t, s, e) = grab();
    for g in [t, s, e] {
        for end_d in [0.3, -0.3, 0.9, -0.9] {
            let mut frames = u0.clone();
            for i in 1..=10 {
                bar_drag(&mut frames, &g, end_d * i as f64 / 10.0);
            }
            let mut once = u0.clone();
            bar_drag(&mut once, &g, end_d);
            assert_eq!(
                frames, once,
                "U-NAV-2 {:?}: growing d every frame == one call with d {end_d}",
                g.part
            );
        }
    }
}
