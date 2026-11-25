//! Window rules for timeline navigation (docs/GUI.md U-NAV-3 .. U-NAV-6), headless through
//! egui_kittest. Written from the contract only; the implementation was not read. Expected values
//! are worked out by hand.
//!
//! Geometry for mouse input, as in tests/ed_app.rs: the track header button `V1` sits 4 px right
//! of and below the top-left corner of the V1 row, the header is 150 px wide, a row middle is
//! 22 px down, and a fresh tab shows 100 px per second from time 0 (U-STATE). The sequence is
//! 30 fps (`F30` = one frame).
//!
//! Readings taken where the contract names no seam:
//! - the strip's width is compared with `ui_state().view_width` within 2 px, and used as `W`;
//! - "sequence duration" for the zoom bar is the end of the last clip (a single 40 s clip at 0);
//! - a wheel event is `egui::Event::MouseWheel` (unit Point, positive y = wheel up) with the
//!   modifier set in both `ModifiersChanged` and the event; egui smooths it over several frames,
//!   so only directions and the keep-under-pointer invariant are asserted;
//! - `Hand` is the tool button labelled `Hand` (U-ED-11).
//!
//! Not tested: the resize cursor (U-NAV-6, drawing only), vertical track scroll of U-NAV-5 and
//! the plain-wheel / Shift-wheel behaviour of U-NAV-4 (tracks do not overflow in a fresh tab).
mod app_common;
use adit_core::{Id, Time};
use adit_gui::{App, Options, Tool};
use app_common::*;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use serde_json::json;

// ------------------------------------------------------------------------------- helpers

fn v1_left(h: &Harness<App>) -> f32 {
    h.get_by_label("V1").rect().min.x - 4.0
}

/// Screen point `px` pixels right of time 0 (100 px/s, scroll 0) in the middle of the V1 row.
fn v1_px(h: &Harness<App>, px: f32) -> egui::Pos2 {
    let r = h.get_by_label("V1").rect();
    egui::pos2(r.min.x - 4.0 + 150.0 + px, r.min.y - 4.0 + 22.0)
}

fn button(h: &Harness<'static, App>, p: egui::Pos2, b: egui::PointerButton, pressed: bool) {
    h.event(egui::Event::PointerButton {
        pos: p,
        button: b,
        pressed,
        modifiers: egui::Modifiers::NONE,
    });
}

/// Press, move to `to` (two frames of motion); the button stays down.
fn press_move(
    h: &mut Harness<'static, App>,
    b: egui::PointerButton,
    a: egui::Pos2,
    to: egui::Pos2,
) {
    h.hover_at(a);
    h.step();
    button(h, a, b, true);
    h.step();
    h.hover_at(a + (to - a) * 0.5);
    h.step();
    h.hover_at(to);
    h.run_steps(2);
}

fn release(h: &mut Harness<'static, App>, b: egui::PointerButton, at: egui::Pos2) {
    button(h, at, b, false);
    h.run_steps(3);
}

/// Primary drag from `a` to `b`, one event per frame.
fn drag(h: &mut Harness<'static, App>, a: egui::Pos2, b: egui::Pos2) {
    press_move(h, egui::PointerButton::Primary, a, b);
    release(h, egui::PointerButton::Primary, b);
}

fn click_at(h: &mut Harness<'static, App>, p: egui::Pos2) {
    h.hover_at(p);
    h.step();
    h.drag_at(p);
    h.step();
    h.drop_at(p);
    h.run_steps(3);
}

fn ui_scroll(h: &Harness<App>) -> Time {
    h.state().ui_state().unwrap().scroll
}

fn ui_pps(h: &Harness<App>) -> f64 {
    h.state().ui_state().unwrap().px_per_sec
}

/// `(start, duration)` of a clip.
fn span(s: &adit_mcp::Session, pid: &str, clip: Id) -> (Time, Time) {
    let c = snapshot(s, pid).clip(clip).expect("clip").2.clone();
    (c.start, c.duration)
}

/// Untitled with `Ocelot5` `[1s, 3s)` on V1, drawn. Returns (pid, clip).
fn one_clip(h: &mut Harness<'static, App>, s: &adit_mcp::Session) -> (String, Id) {
    let (pid, _, v1) = active(h, s);
    let c = add_named(s, &pid, v1, json!("1s"), json!("2s"), "Ocelot5");
    step_until(h, "clip Ocelot5 in the timeline", |h| {
        has_label(h, "Ocelot5")
    });
    (pid, c)
}

fn select_clip(h: &mut Harness<'static, App>, c: Id) {
    h.get_by_label("Ocelot5").click();
    h.run_steps(3);
    assert_eq!(
        h.state().ui_state().unwrap().selection,
        vec![c],
        "setup: clip selected"
    );
}

fn pick_tool(h: &mut Harness<'static, App>, name: &str, tool: Tool) {
    h.get_by_label(name).click();
    h.run_steps(3);
    assert_eq!(
        h.state().ui_state().unwrap().tool,
        tool,
        "setup: tool {name}"
    );
}

// ------------------------------------------------------------------------------- U-NAV-6

#[test]
fn u_nav_6_select_drag_near_end_trims_end() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, c) = one_clip(&mut h, &s);
    let d0 = undo_depth(&s, &pid);
    // Press 7 px left of the end (2.93 s), release 50 px right (3.43 s).
    let (a, b) = (v1_px(&h, 293.0), v1_px(&h, 343.0));
    drag(&mut h, a, b);
    let (start, dur) = span(&s, &pid, c);
    assert_eq!(start, S, "U-NAV-6: start unchanged");
    assert_eq!(
        start + dur,
        103 * F30,
        "U-NAV-6: end at frame 103 (3.4333 s)"
    );
    assert_eq!(undo_depth(&s, &pid), d0 + 1, "U-NAV-6: one undo step");
}

#[test]
fn u_nav_6_select_drag_near_start_trims_start() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, c) = one_clip(&mut h, &s);
    let d0 = undo_depth(&s, &pid);
    // Press 7 px inside the start (1.07 s), release 50 px right (1.57 s -> frame 47).
    let (a, b) = (v1_px(&h, 107.0), v1_px(&h, 157.0));
    drag(&mut h, a, b);
    let (start, dur) = span(&s, &pid, c);
    assert_eq!(start, 47 * F30, "U-NAV-6: start trimmed to frame 47");
    assert_eq!(start + dur, 3 * S, "U-NAV-6: end unchanged");
    assert_eq!(undo_depth(&s, &pid), d0 + 1, "U-NAV-6: one undo step");
}

#[test]
fn u_nav_6_select_drag_in_body_still_moves() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, c) = one_clip(&mut h, &s);
    let d0 = undo_depth(&s, &pid);
    // Press at 2 s (middle), release 50 px right: moved by 0.5 s = 15 frames.
    let (a, b) = (v1_px(&h, 200.0), v1_px(&h, 250.0));
    drag(&mut h, a, b);
    let (start, dur) = span(&s, &pid, c);
    assert_eq!(
        start, 1_058_400_000,
        "U-NAV-6: body drag moves the clip to 1.5 s"
    );
    assert_eq!(dur, 2 * S, "U-NAV-6: length unchanged");
    assert_eq!(undo_depth(&s, &pid), d0 + 1);
}

// ------------------------------------------------------------------------------- U-NAV-5

#[test]
fn u_nav_5_select_drag_on_empty_pans_live_without_edit() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, c) = one_clip(&mut h, &s);
    select_clip(&mut h, c);
    let (d0, before) = (undo_depth(&s, &pid), snapshot(&s, &pid));
    assert_eq!(ui_scroll(&h), 0);
    // Empty space at 6 s, drag left by 100 px: scroll 0 - round(-100 * S / 100) = 1 s.
    let (a, b) = (v1_px(&h, 600.0), v1_px(&h, 500.0));
    press_move(&mut h, egui::PointerButton::Primary, a, b);
    assert_eq!(
        ui_scroll(&h),
        S,
        "U-NAV-5: the view pans before the release"
    );
    release(&mut h, egui::PointerButton::Primary, b);
    assert_eq!(ui_scroll(&h), S, "U-NAV-5: scroll 1 s after the release");
    assert_eq!(undo_depth(&s, &pid), d0, "U-NAV-5: no undo step");
    assert_eq!(snapshot(&s, &pid), before, "U-NAV-5: project unchanged");
    assert_eq!(
        h.state().ui_state().unwrap().selection,
        vec![c],
        "U-NAV-5: selection unchanged"
    );
    assert_eq!(
        h.state().ui_state().unwrap().playhead,
        0,
        "U-NAV-5: playhead unchanged"
    );
}

#[test]
fn u_nav_5_scroll_never_below_zero() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _) = one_clip(&mut h, &s);
    let d0 = undo_depth(&s, &pid);
    // Drag right from scroll 0: 0 - round(100 * S / 100) < 0 -> 0.
    let (a, b) = (v1_px(&h, 500.0), v1_px(&h, 600.0));
    drag(&mut h, a, b);
    assert_eq!(ui_scroll(&h), 0, "U-NAV-5: scroll clamps at 0");
    assert_eq!(undo_depth(&s, &pid), d0);
}

#[test]
fn u_nav_5_hand_drag_on_clip_pans_and_does_not_move_it() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, c) = one_clip(&mut h, &s);
    pick_tool(&mut h, "Hand", Tool::Hand);
    let (d0, before) = (undo_depth(&s, &pid), span(&s, &pid, c));
    // On the clip body (2 s), 200 px left: scroll 2 s.
    let (a, b) = (v1_px(&h, 200.0), v1_px(&h, 0.0));
    drag(&mut h, a, b);
    assert_eq!(ui_scroll(&h), 2 * S, "U-NAV-5: Hand drag pans");
    assert_eq!(span(&s, &pid, c), before, "U-NAV-5: the clip did not move");
    assert_eq!(undo_depth(&s, &pid), d0, "U-NAV-5: no undo step");
}

#[test]
fn u_nav_5_middle_drag_on_clip_pans_and_never_edits() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _) = one_clip(&mut h, &s);
    let (d0, before) = (undo_depth(&s, &pid), snapshot(&s, &pid));
    let (a, b) = (v1_px(&h, 200.0), v1_px(&h, 100.0));
    press_move(&mut h, egui::PointerButton::Middle, a, b);
    assert_eq!(
        ui_scroll(&h),
        S,
        "U-NAV-5: middle drag pans live (100 px left = 1 s)"
    );
    release(&mut h, egui::PointerButton::Middle, b);
    assert_eq!(ui_scroll(&h), S);
    assert_eq!(
        snapshot(&s, &pid),
        before,
        "U-NAV-5: a middle press never edits"
    );
    assert_eq!(undo_depth(&s, &pid), d0);
}

#[test]
fn u_nav_5_middle_drag_near_edge_does_not_trim() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, c) = one_clip(&mut h, &s);
    let before = span(&s, &pid, c);
    // 7 px inside the end: a primary drag would trim; the middle button pans.
    let (a, b) = (v1_px(&h, 293.0), v1_px(&h, 243.0));
    press_move(&mut h, egui::PointerButton::Middle, a, b);
    release(&mut h, egui::PointerButton::Middle, b);
    assert_eq!(
        span(&s, &pid, c),
        before,
        "U-NAV-5: no trim from the middle button"
    );
    assert_eq!(ui_scroll(&h), S / 2, "U-NAV-5: 50 px left = 0.5 s");
}

#[test]
fn u_nav_5_select_click_on_empty_still_empties_selection() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, c) = one_clip(&mut h, &s);
    select_clip(&mut h, c);
    let d0 = undo_depth(&s, &pid);
    let p = v1_px(&h, 600.0);
    click_at(&mut h, p);
    assert!(
        h.state().ui_state().unwrap().selection.is_empty(),
        "U-NAV-5 / U-DRAG-SEL: a click on empty space empties the selection"
    );
    assert_eq!(ui_scroll(&h), 0, "U-NAV-5: a click does not pan");
    assert_eq!(undo_depth(&s, &pid), d0);
}

// ------------------------------------------------------------------------------- U-NAV-3

/// Untitled with a 40 s clip on V1 (a partial thumb). Returns the pid.
fn long_seq(h: &mut Harness<'static, App>, s: &adit_mcp::Session) -> String {
    let (pid, _, v1) = active(h, s);
    add_named(s, &pid, v1, json!(0), json!("40s"), "Long40");
    step_until(h, "clip Long40 in the timeline", |h| has_label(h, "Long40"));
    pid
}

/// (rect of the strip, view_width).
fn strip(h: &Harness<App>) -> (egui::Rect, f64) {
    let r = h.get_by_label("Zoom bar").rect();
    (r, h.state().ui_state().unwrap().view_width as f64)
}

#[test]
fn u_nav_3_strip_exists_with_geometry() {
    let s = session();
    let mut h = harness(&s, Options::default());
    long_seq(&mut h, &s);
    let (r, vw) = strip(&h);
    assert!(
        (r.height() - 14.0).abs() < 0.5,
        "U-NAV-3: 14 px high, got {}",
        r.height()
    );
    assert!(
        (r.min.x - (v1_left(&h) + 150.0)).abs() < 0.5,
        "U-NAV-3: starts at the header's right edge: {} vs {}",
        r.min.x,
        v1_left(&h) + 150.0
    );
    assert!(
        (r.width() as f64 - vw).abs() < 2.0,
        "U-NAV-3: spans the timeline area: width {} vs view_width {vw}",
        r.width()
    );
    let v1 = h.get_by_label("V1").rect();
    assert!(
        r.min.y > v1.max.y,
        "U-NAV-3: the strip is below the track rows"
    );
}

/// Thumb of a fresh 40 s sequence at 100 px/s, scroll 0, in screen x: (left, right).
fn thumb_px(h: &Harness<App>) -> (f32, f32) {
    let (r, vw) = strip(h);
    // visible = vw / 100 s, total = 40 s: a = 0, b = vw / 4000 of the strip.
    let b = vw / 100.0 / 40.0;
    (r.min.x, r.min.x + (b * r.width() as f64) as f32)
}

#[test]
fn u_nav_3_thumb_drag_scrolls_without_zoom_or_edit() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let pid = long_seq(&mut h, &s);
    let (r, _) = strip(&h);
    let (l, rt) = thumb_px(&h);
    assert!(
        rt - l > 100.0,
        "setup: the thumb is partial and wide ({} px)",
        rt - l
    );
    let (d0, before) = (undo_depth(&s, &pid), snapshot(&s, &pid));
    let y = r.center().y;
    let mid = (l + rt) / 2.0;
    let (a, b) = (egui::pos2(mid, y), egui::pos2(mid + 100.0, y));
    press_move(&mut h, egui::PointerButton::Primary, a, b);
    // d = 100 / W, dt = d * 40 s: scroll = 4000 / W seconds.
    let want = 4000.0 / r.width() as f64;
    let got = ui_scroll(&h) as f64 / S as f64;
    assert!(
        (got - want).abs() < 0.05,
        "U-NAV-3: live scroll {got} s, want {want} s"
    );
    release(&mut h, egui::PointerButton::Primary, b);
    let got = ui_scroll(&h) as f64 / S as f64;
    assert!(
        (got - want).abs() < 0.05,
        "U-NAV-3: scroll {got} s, want {want} s"
    );
    assert_eq!(ui_pps(&h), 100.0, "U-NAV-3: a thumb drag keeps the zoom");
    assert_eq!(
        undo_depth(&s, &pid),
        d0,
        "U-NAV-3: nothing in the project changes"
    );
    assert_eq!(snapshot(&s, &pid), before);
}

#[test]
fn u_nav_3_right_end_drag_zooms_with_scroll_unchanged() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let pid = long_seq(&mut h, &s);
    let (r, vw) = strip(&h);
    let (_, rt) = thumb_px(&h);
    let d0 = undo_depth(&s, &pid);
    let y = r.center().y;
    // 2 px inside the right end grabs `End`. dx = -0.1 W: d = -0.1, dt = -4 s.
    let dx = -0.1 * r.width();
    let a = egui::pos2(rt - 2.0, y);
    drag(&mut h, a, egui::pos2(a.x + dx, y));
    // pps = vw / (visible - 4 s), visible = vw / 100 s.
    let want = vw / (vw / 100.0 - 4.0);
    let got = ui_pps(&h);
    assert!(
        (got - want).abs() / want < 0.01,
        "U-NAV-3: End drag pps {got}, want {want}"
    );
    assert_eq!(
        ui_scroll(&h),
        0,
        "U-NAV-3: End drag keeps the left end (scroll)"
    );
    assert_eq!(
        undo_depth(&s, &pid),
        d0,
        "U-NAV-3: nothing in the project changes"
    );
}

#[test]
fn u_nav_3_press_outside_thumb_does_nothing() {
    let s = session();
    let mut h = harness(&s, Options::default());
    let pid = long_seq(&mut h, &s);
    let (r, _) = strip(&h);
    let (_, rt) = thumb_px(&h);
    let d0 = undo_depth(&s, &pid);
    let y = r.center().y;
    let a = egui::pos2(rt + 60.0, y);
    assert!(a.x < r.max.x, "setup: press point on the strip");
    drag(&mut h, a, egui::pos2(a.x + 100.0, y));
    assert_eq!(
        ui_scroll(&h),
        0,
        "U-NAV-3: a press outside the thumb does nothing"
    );
    assert_eq!(ui_pps(&h), 100.0);
    assert_eq!(undo_depth(&s, &pid), d0);
}

// ------------------------------------------------------------------------------- U-NAV-4

fn ctrl() -> egui::Modifiers {
    egui::Modifiers {
        ctrl: true,
        command: true,
        ..Default::default()
    }
}

fn alt() -> egui::Modifiers {
    egui::Modifiers {
        alt: true,
        ..Default::default()
    }
}

/// Wheel `y` points (positive = up) with modifiers held, over several frames.
fn wheel(h: &mut Harness<'static, App>, m: egui::Modifiers, y: f32) {
    h.event(egui::Event::ModifiersChanged(m));
    h.step();
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, y),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
        modifiers: m,
    });
    h.run_steps(8);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run_steps(2);
}

/// Sequence time (seconds) under screen x, from the live view.
fn time_under(h: &Harness<App>, x: f32) -> f64 {
    let u = h.state().ui_state().unwrap();
    u.scroll as f64 / S as f64 + (x - (v1_left(h) + 150.0)) as f64 / u.px_per_sec
}

fn wheel_zoom_case(m: egui::Modifiers, name: &str) {
    let s = session();
    let mut h = harness(&s, Options::default());
    let (pid, _) = one_clip(&mut h, &s);
    let d0 = undo_depth(&s, &pid);
    let p = v1_px(&h, 400.0);
    h.hover_at(p);
    h.run_steps(2);
    let t0 = time_under(&h, p.x);
    assert!((t0 - 4.0).abs() < 1e-6, "setup: pointer at 4 s");

    wheel(&mut h, m, 60.0);
    let pps_in = ui_pps(&h);
    assert!(
        pps_in > 100.0,
        "U-NAV-4 {name}+wheel up zooms in: pps {pps_in}"
    );
    let drift_px = (time_under(&h, p.x) - t0).abs() * pps_in;
    assert!(
        drift_px <= 1.0,
        "U-NAV-4 {name}: time under pointer drifts {drift_px} px"
    );
    assert!(
        ui_scroll(&h) > 0,
        "U-NAV-4 {name}: zooming in at 4 s scrolls"
    );

    let t1 = time_under(&h, p.x);
    wheel(&mut h, m, -30.0);
    let pps_out = ui_pps(&h);
    assert!(
        pps_out < pps_in,
        "U-NAV-4 {name}+wheel down zooms out: {pps_out} < {pps_in}"
    );
    assert!(pps_out >= 1.0);
    let drift_px = (time_under(&h, p.x) - t1).abs() * pps_out;
    assert!(
        drift_px <= 1.0,
        "U-NAV-4 {name}: zoom out drifts {drift_px} px"
    );
    assert_eq!(undo_depth(&s, &pid), d0, "U-NAV-4: zooming edits nothing");
}

#[test]
fn u_nav_4_ctrl_wheel_zooms_at_pointer() {
    wheel_zoom_case(ctrl(), "Ctrl");
}

#[test]
fn u_nav_4_alt_wheel_zooms_at_pointer() {
    wheel_zoom_case(alt(), "Alt");
}

#[test]
fn u_nav_4_wheel_over_header_does_not_zoom() {
    let s = session();
    let mut h = harness(&s, Options::default());
    one_clip(&mut h, &s);
    // Over the header (75 px right of the row's left edge), not the timeline area.
    let p = egui::pos2(v1_left(&h) + 75.0, h.get_by_label("V1").rect().center().y);
    h.hover_at(p);
    h.run_steps(2);
    wheel(&mut h, ctrl(), 60.0);
    assert_eq!(
        ui_pps(&h),
        100.0,
        "U-NAV-4: only the timeline area (right of the headers) zooms"
    );
}
