//! Trim, slip, slide, speed: C-TRIM-*, C-SLIP-1, C-SLIDE-1, C-SPEED-1 with R1, R2, R3, R7.
#[macro_use]
mod common;
use common::*;

fn tr(clip_id: Id, edge: Edge, to: Time, mode: TrimMode, include_linked: bool) -> Command {
    Command::TrimClip {
        clip_id,
        edge,
        to,
        mode,
        include_linked,
    }
}

fn sdr(c: &Clip) -> (Time, Time, Time) {
    (c.start, c.duration, c.source_in)
}

/// V1: X [2s,6s) source_in 1s (media M, 100 s).
fn one(speed: f64, reverse: bool) -> (Base, Id) {
    let mut b = base();
    let (m, v1) = (b.m, b.v1);
    let x = b.fx.clip(v1, m, 2 * S, 4 * S, S);
    b.fx.c(x).speed = speed;
    b.fx.c(x).reverse = reverse;
    (b, x)
}

// ============================================================================ C-TRIM-1/2 normal, R2

#[test]
fn c_trim_2_r2_head_trim() {
    let (b, x) = one(1.0, false);
    let mut doc = b.fx.doc();
    ok(&mut doc, trim(x, Edge::Start, 3 * S, TrimMode::Normal));
    assert_eq!(sdr(clip(doc.project(), x)), (3 * S, 3 * S, 2 * S));
}

#[test]
fn c_trim_2_r2_head_trim_speed_2() {
    let (b, x) = one(2.0, false);
    let mut doc = b.fx.doc();
    ok(&mut doc, trim(x, Edge::Start, 3 * S, TrimMode::Normal));
    assert_eq!(sdr(clip(doc.project(), x)), (3 * S, 3 * S, 3 * S));
    assert_eq!(clip(doc.project(), x).speed, 2.0);
}

#[test]
fn c_trim_2_r2_head_trim_reversed_keeps_source_in() {
    let (b, x) = one(2.0, true);
    let mut doc = b.fx.doc();
    ok(&mut doc, trim(x, Edge::Start, 3 * S, TrimMode::Normal));
    assert_eq!(sdr(clip(doc.project(), x)), (3 * S, 3 * S, S));
}

#[test]
fn c_trim_2_r2_head_extend() {
    let (b, x) = one(1.0, false);
    let mut doc = b.fx.doc();
    ok(&mut doc, trim(x, Edge::Start, S, TrimMode::Normal));
    assert_eq!(sdr(clip(doc.project(), x)), (S, 5 * S, 0));
}

#[test]
fn c_trim_2_r2_head_extend_reversed() {
    let (b, x) = one(1.0, true);
    let mut doc = b.fx.doc();
    ok(&mut doc, trim(x, Edge::Start, 0, TrimMode::Normal));
    assert_eq!(sdr(clip(doc.project(), x)), (0, 6 * S, S));
}

#[test]
fn c_trim_2_r2_rounding_with_speed_1_5() {
    let Base { mut fx, m, v1, .. } = base();
    let x = fx.clip(v1, m, 2 * S, 4 * S, 10);
    fx.c(x).speed = 1.5;
    let mut doc = fx.clone_doc();
    ok(&mut doc, trim(x, Edge::Start, 2 * S + 1, TrimMode::Normal));
    assert_eq!(clip(doc.project(), x).source_in, 12, "10 + round(1.5)");
    let mut doc = fx.doc();
    ok(&mut doc, trim(x, Edge::Start, 2 * S - 1, TrimMode::Normal));
    assert_eq!(clip(doc.project(), x).source_in, 8, "10 + round(-1.5)");
}

#[test]
fn c_trim_2_r2_head_trim_shifts_keyframes() {
    let (mut b, x) = one(1.0, false);
    b.fx.c(x).params.insert(
        "opacity".into(),
        param(
            1.0,
            vec![kf(0, 0.0, Easing::Linear), kf(4 * S, 1.0, Easing::EaseOut)],
        ),
    );
    b.fx.effect(
        x,
        "blur",
        &[("amount", param(2.0, vec![kf(S, 5.0, Easing::Hold)]))],
    );
    let mut doc = b.fx.doc();
    ok(&mut doc, trim(x, Edge::Start, 3 * S, TrimMode::Normal));
    let c = clip(doc.project(), x);
    assert_eq!(
        c.params["opacity"].keyframes,
        vec![kf(-S, 0.0, Easing::Linear), kf(3 * S, 1.0, Easing::EaseOut)]
    );
    assert_eq!(c.params["opacity"].value, 1.0);
    assert_eq!(
        c.effects[0].params["amount"].keyframes,
        vec![kf(0, 5.0, Easing::Hold)]
    );
}

#[test]
fn c_trim_2_r7_head_extend_past_source_start() {
    let (b, x) = one(1.0, false);
    let mut doc = b.fx.doc();
    assert_err!(&mut doc, trim(x, Edge::Start, 0, TrimMode::Normal), Error::SourceOutOfRange { clip_id } if clip_id == x);
}

#[test]
fn c_trim_2_overlap_and_duration_errors() {
    let (mut b, x) = one(1.0, false);
    let (m, v1) = (b.m, b.v1);
    b.fx.clip(v1, m, 0, 2 * S - S / 2, 0); // P [0, 1.5s)
    b.fx.clip(v1, m, 6 * S, 2 * S, 0); // N [6s, 8s)
    let mut doc = b.fx.doc();
    assert_err!(&mut doc, trim(x, Edge::Start, S, TrimMode::Normal), Error::Overlap { track_id } if track_id == v1);
    assert_err!(&mut doc, trim(x, Edge::End, 7 * S, TrimMode::Normal), Error::Overlap { track_id } if track_id == v1);
    assert_err!(
        &mut doc,
        trim(x, Edge::Start, 6 * S, TrimMode::Normal),
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        trim(x, Edge::Start, 7 * S, TrimMode::Normal),
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        trim(x, Edge::End, 2 * S, TrimMode::Normal),
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        trim(999, Edge::End, 2 * S, TrimMode::Normal),
        Error::NotFound { id: 999, .. }
    );
    // touching neighbours exactly is fine
    ok(
        &mut doc,
        trim(x, Edge::Start, 2 * S - S / 2, TrimMode::Normal),
    );
    ok(&mut doc, trim(x, Edge::End, 6 * S, TrimMode::Normal));
}

// ---------------------------------------------------------------------------- R3

#[test]
fn c_trim_2_r3_tail_trim() {
    let (b, x) = one(1.0, false);
    let mut doc = b.fx.doc();
    ok(&mut doc, trim(x, Edge::End, 5 * S, TrimMode::Normal));
    assert_eq!(sdr(clip(doc.project(), x)), (2 * S, 3 * S, S));
}

#[test]
fn c_trim_2_r3_tail_trim_reversed() {
    let (b, x) = one(1.0, true);
    let mut doc = b.fx.doc();
    ok(&mut doc, trim(x, Edge::End, 5 * S, TrimMode::Normal));
    assert_eq!(sdr(clip(doc.project(), x)), (2 * S, 3 * S, 2 * S));
}

#[test]
fn c_trim_2_r3_tail_trim_reversed_speed_2() {
    let (b, x) = one(2.0, true);
    let mut doc = b.fx.doc();
    ok(&mut doc, trim(x, Edge::End, 5 * S, TrimMode::Normal));
    assert_eq!(sdr(clip(doc.project(), x)), (2 * S, 3 * S, 3 * S));
}

#[test]
fn c_trim_2_r3_tail_trim_keeps_keyframes() {
    let (mut b, x) = one(1.0, false);
    b.fx.c(x).params.insert(
        "opacity".into(),
        param(1.0, vec![kf(3 * S, 0.0, Easing::Linear)]),
    );
    let mut doc = b.fx.doc();
    ok(&mut doc, trim(x, Edge::End, 3 * S, TrimMode::Normal));
    assert_eq!(
        clip(doc.project(), x).params["opacity"].keyframes,
        vec![kf(3 * S, 0.0, Easing::Linear)]
    );
}

#[test]
fn c_trim_2_r7_tail_extend_slack_boundary() {
    let Base { mut fx, v1, .. } = base();
    let short = fx.media_full("Short", Some(10 * S), true, true);
    let x = fx.clip(v1, short, 2 * S, 4 * S, 5 * S); // uses [5s, 9s)
    let mut doc = fx.clone_doc();
    ok(
        &mut doc,
        trim(x, Edge::End, 7 * S + SOURCE_SLACK, TrimMode::Normal),
    );
    assert_eq!(clip(doc.project(), x).duration, 5 * S + 2);
    let mut doc = fx.doc();
    assert_err!(
        &mut doc,
        trim(x, Edge::End, 7 * S + SOURCE_SLACK + 1, TrimMode::Normal),
        Error::SourceOutOfRange { clip_id } if clip_id == x
    );
}

#[test]
fn c_trim_2_r3_r7_reversed_tail_extend_below_zero() {
    let (b, x) = one(1.0, true);
    let mut doc = b.fx.doc();
    // t = -2s -> source_in = 1s - 2s < 0
    assert_err!(
        &mut doc,
        trim(x, Edge::End, 8 * S, TrimMode::Normal),
        Error::SourceOutOfRange { .. }
    );
}

#[test]
fn c_trim_2_r7_still_and_generated_clips() {
    let Base {
        mut fx,
        still,
        v1,
        v2,
        ..
    } = base();
    let st = fx.clip(v1, still, 2 * S, S, 0);
    let t = fx.text(v2, 2 * S, 2 * S);
    let mut doc = fx.doc();
    ok(&mut doc, trim(st, Edge::End, 1000 * S, TrimMode::Normal));
    assert_eq!(clip(doc.project(), st).duration, 998 * S);
    // R7: Text keeps its source_in (no source bounds), so extending its head is fine
    ok(&mut doc, trim(t, Edge::Start, 0, TrimMode::Normal));
    assert_eq!(sdr(clip(doc.project(), t)), (0, 4 * S, 0));
}

#[test]
fn c_trim_2_r7_sequence_clip_needs_only_nonnegative_source_in() {
    let Base { mut fx, m, v1, .. } = base();
    let inner = fx.seq("Inner");
    let it = fx.track(inner, TrackKind::Video);
    fx.clip(it, m, 0, S, 0);
    let c = fx.clip_src(v1, seq_src(inner), 2 * S, S, S);
    let mut doc = fx.doc();
    ok(&mut doc, trim(c, Edge::End, 50 * S, TrimMode::Normal));
    ok(&mut doc, trim(c, Edge::Start, S, TrimMode::Normal));
    assert_eq!(sdr(clip(doc.project(), c)), (S, 49 * S, 0));
    assert_err!(
        &mut doc,
        trim(c, Edge::Start, S - 1, TrimMode::Normal),
        Error::SourceOutOfRange { .. }
    );
}

#[test]
fn c_trim_1_linked_same_edge_only() {
    let (mut b, x) = one(1.0, false);
    let (m, a1, a2) = (b.m, b.a1, b.a2);
    let xa = b.fx.clip(a1, m, 2 * S, 4 * S, S);
    let xb = b.fx.clip(a2, m, 2 * S, 5 * S, S); // same start, different end
    b.fx.link(&[x, xa, xb]);
    let mut doc = b.fx.clone_doc();
    ok(&mut doc, trim(x, Edge::End, 5 * S, TrimMode::Normal));
    let p = doc.project();
    assert_eq!(
        (clip(p, x).end(), clip(p, xa).end(), clip(p, xb).end()),
        (5 * S, 5 * S, 7 * S)
    );
    ok(&mut doc, trim(x, Edge::Start, 3 * S, TrimMode::Normal));
    let p = doc.project();
    assert_eq!(
        (sdr(clip(p, x)), sdr(clip(p, xa)), sdr(clip(p, xb))),
        (
            (3 * S, 2 * S, 2 * S),
            (3 * S, 2 * S, 2 * S),
            (3 * S, 4 * S, 2 * S)
        )
    );
    let mut doc = b.fx.doc();
    ok(&mut doc, tr(x, Edge::End, 5 * S, TrimMode::Normal, false));
    assert_eq!(
        (clip(doc.project(), x).end(), clip(doc.project(), xa).end()),
        (5 * S, 6 * S)
    );
}

#[test]
fn c_trim_1_r7_applies_to_linked_clips() {
    let (mut b, x) = one(1.0, false);
    let (m, a1) = (b.m, b.a1);
    let xa = b.fx.clip(a1, m, 2 * S, 4 * S, 0); // partner has no head room
    b.fx.link(&[x, xa]);
    let mut doc = b.fx.doc();
    assert_err!(&mut doc, trim(x, Edge::Start, S, TrimMode::Normal), Error::SourceOutOfRange { clip_id } if clip_id == xa);
    ok(&mut doc, tr(x, Edge::Start, S, TrimMode::Normal, false));
}

#[test]
fn c_trim_g5_locked() {
    let (mut b, x) = one(1.0, false);
    let (m, v1, a1) = (b.m, b.v1, b.a1);
    let xa = b.fx.clip(a1, m, 2 * S, 4 * S, S);
    b.fx.link(&[x, xa]);
    b.fx.lock(a1);
    let mut doc = b.fx.clone_doc();
    assert_err!(&mut doc, trim(x, Edge::End, 5 * S, TrimMode::Normal), Error::TrackLocked { track_id } if track_id == a1);
    b.fx.lock(v1);
    let mut doc = b.fx.doc();
    assert_err!(&mut doc, tr(x, Edge::End, 5 * S, TrimMode::Normal, false), Error::TrackLocked { track_id } if track_id == v1);
}

// ============================================================================ C-TRIM-3 ripple

/// V1: X [2s,6s) src 1s, Y [6s,8s); V2: Z [6s,7s), Z2 [1s,3s), Z3 [5s,6s); A2 locked: L [6s,7s).
fn ripple_fx() -> (Base, [Id; 6]) {
    let (mut b, x) = one(1.0, false);
    let (m, v1, v2, a2) = (b.m, b.v1, b.v2, b.a2);
    let y = b.fx.clip(v1, m, 6 * S, 2 * S, 0);
    let z = b.fx.clip(v2, m, 6 * S, S, 0);
    let z2 = b.fx.clip(v2, m, S, 2 * S, 0);
    let z3 = b.fx.clip(v2, m, 5 * S, S, 0);
    let l = b.fx.clip(a2, m, 6 * S, S, 0);
    b.fx.lock(a2);
    (b, [x, y, z, z2, z3, l])
}

#[test]
fn c_trim_3_spec_example_ripple_start() {
    let (mut b, x) = one(1.0, false);
    let (m, v1) = (b.m, b.v1);
    let y = b.fx.clip(v1, m, 6 * S, 2 * S, 0);
    let mut doc = b.fx.doc();
    ok(&mut doc, trim(x, Edge::Start, 3 * S, TrimMode::Ripple));
    let p = doc.project();
    assert_eq!(sdr(clip(p, x)), (2 * S, 3 * S, 2 * S));
    assert_eq!(spans(p, v1), vec![(x, 2 * S, 5 * S), (y, 5 * S, 7 * S)]);
}

#[test]
fn c_trim_3_r1_ripple_end_shifts_unlocked_tracks() {
    let (b, [x, y, z, z2, z3, l]) = ripple_fx();
    let (v1, v2, a2) = (b.v1, b.v2, b.a2);
    let mut doc = b.fx.doc();
    ok(&mut doc, trim(x, Edge::End, 7 * S, TrimMode::Ripple));
    let p = doc.project();
    assert_eq!(sdr(clip(p, x)), (2 * S, 5 * S, S));
    assert_eq!(spans(p, v1), vec![(x, 2 * S, 7 * S), (y, 7 * S, 9 * S)]);
    assert_eq!(
        spans(p, v2),
        vec![(z2, S, 3 * S), (z3, 5 * S, 6 * S), (z, 7 * S, 8 * S)]
    );
    assert_eq!(spans(p, a2), vec![(l, 6 * S, 7 * S)]);
}

#[test]
fn c_trim_3_r1_ripple_end_shorten_blocked_on_other_track() {
    // Z [6s,7s) would move to [5.5s,6.5s) onto Z3 [5s,6s), which starts before the old end
    let (b, [x, ..]) = ripple_fx();
    let mut doc = b.fx.doc();
    assert_err!(
        &mut doc,
        trim(x, Edge::End, 5 * S + S / 2, TrimMode::Ripple),
        Error::RippleBlocked { .. }
    );
}

#[test]
fn c_trim_3_ripple_end_shorten() {
    let (mut b, [x, y, z, z2, z3, l]) = ripple_fx();
    let (v1, v2, a2) = (b.v1, b.v2, b.a2);
    b.fx.c(z3).duration = S / 2; // Z3 [5s, 5.5s)
    let mut doc = b.fx.doc();
    ok(
        &mut doc,
        trim(x, Edge::End, 5 * S + S / 2, TrimMode::Ripple),
    );
    let p = doc.project();
    let h = S / 2;
    assert_eq!(
        spans(p, v1),
        vec![(x, 2 * S, 5 * S + h), (y, 5 * S + h, 7 * S + h)]
    );
    assert_eq!(
        spans(p, v2),
        vec![
            (z2, S, 3 * S),
            (z3, 5 * S, 5 * S + h),
            (z, 5 * S + h, 6 * S + h)
        ]
    );
    assert_eq!(spans(p, a2), vec![(l, 6 * S, 7 * S)]);
}

#[test]
fn c_trim_3_ripple_start_extend_and_keyframes() {
    let (mut b, [x, y, ..]) = ripple_fx();
    let v1 = b.v1;
    b.fx.c(x).params.insert(
        "rotation".into(),
        param(0.0, vec![kf(2 * S, 90.0, Easing::Linear)]),
    );
    let mut doc = b.fx.doc();
    ok(&mut doc, trim(x, Edge::Start, S, TrimMode::Ripple)); // d = -1s
    let p = doc.project();
    assert_eq!(sdr(clip(p, x)), (2 * S, 5 * S, 0));
    assert_eq!(spans(p, v1), vec![(x, 2 * S, 7 * S), (y, 7 * S, 9 * S)]);
    assert_eq!(
        clip(p, x).params["rotation"].keyframes,
        vec![kf(3 * S, 90.0, Easing::Linear)]
    );
    // d = +1s (fixture without the V2 clips that would block the left shift)
    let (mut b, x) = one(1.0, false);
    b.fx.c(x).params.insert(
        "rotation".into(),
        param(0.0, vec![kf(2 * S, 90.0, Easing::Linear)]),
    );
    let mut doc = b.fx.doc();
    ok(&mut doc, trim(x, Edge::Start, 3 * S, TrimMode::Ripple));
    assert_eq!(
        clip(doc.project(), x).params["rotation"].keyframes,
        vec![kf(S, 90.0, Easing::Linear)]
    );
}

#[test]
fn c_trim_3_ripple_start_reversed_and_speed() {
    let (b, x) = one(2.0, true);
    let mut doc = b.fx.doc();
    ok(&mut doc, trim(x, Edge::Start, 3 * S, TrimMode::Ripple));
    assert_eq!(sdr(clip(doc.project(), x)), (2 * S, 3 * S, S));
    let (b, x) = one(2.0, false);
    let mut doc = b.fx.doc();
    ok(&mut doc, trim(x, Edge::Start, 3 * S, TrimMode::Ripple));
    assert_eq!(sdr(clip(doc.project(), x)), (2 * S, 3 * S, 3 * S));
}

#[test]
fn c_trim_3_ripple_errors() {
    let (b, [x, ..]) = ripple_fx();
    let mut doc = b.fx.doc();
    assert_err!(
        &mut doc,
        trim(x, Edge::Start, 0, TrimMode::Ripple),
        Error::SourceOutOfRange { .. }
    );
    assert_err!(
        &mut doc,
        trim(x, Edge::End, 2 * S, TrimMode::Ripple),
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        trim(x, Edge::Start, 6 * S, TrimMode::Ripple),
        Error::InvalidArgument(_)
    );
}

// ============================================================================ C-TRIM-4 roll

#[test]
fn c_trim_4_roll_end() {
    let (mut b, x) = one(1.0, false);
    let (m, v1, v2) = (b.m, b.v1, b.v2);
    let n = b.fx.clip(v1, m, 6 * S, 2 * S, 2 * S);
    let o = b.fx.clip(v2, m, 7 * S, S, 0);
    let mut doc = b.fx.clone_doc();
    ok(&mut doc, trim(x, Edge::End, 7 * S, TrimMode::Roll));
    let p = doc.project();
    assert_eq!(spans(p, v1), vec![(x, 2 * S, 7 * S), (n, 7 * S, 8 * S)]);
    assert_eq!((clip(p, x).source_in, clip(p, n).source_in), (S, 3 * S));
    assert_eq!(spans(p, v2), vec![(o, 7 * S, 8 * S)]);
    let mut doc = b.fx.doc();
    ok(&mut doc, trim(x, Edge::End, 5 * S, TrimMode::Roll));
    let p = doc.project();
    assert_eq!(spans(p, v1), vec![(x, 2 * S, 5 * S), (n, 5 * S, 8 * S)]);
    assert_eq!(clip(p, n).source_in, S);
}

#[test]
fn c_trim_4_roll_start() {
    let (mut b, x) = one(1.0, false);
    let (m, v1) = (b.m, b.v1);
    let p0 = b.fx.clip(v1, m, 0, 2 * S, 0);
    let mut doc = b.fx.clone_doc();
    ok(&mut doc, trim(x, Edge::Start, 3 * S, TrimMode::Roll));
    let p = doc.project();
    assert_eq!(spans(p, v1), vec![(p0, 0, 3 * S), (x, 3 * S, 6 * S)]);
    assert_eq!((clip(p, p0).source_in, clip(p, x).source_in), (0, 2 * S));
    let mut doc = b.fx.doc();
    ok(&mut doc, trim(x, Edge::Start, S, TrimMode::Roll));
    let p = doc.project();
    assert_eq!(spans(p, v1), vec![(p0, 0, S), (x, S, 6 * S)]);
    assert_eq!(clip(p, x).source_in, 0);
}

#[test]
fn c_trim_4_roll_with_reversed_neighbour() {
    let (mut b, x) = one(1.0, false);
    let (m, v1) = (b.m, b.v1);
    let n = b.fx.clip(v1, m, 6 * S, 2 * S, 2 * S);
    b.fx.c(n).reverse = true;
    let mut doc = b.fx.doc();
    ok(&mut doc, trim(x, Edge::End, 7 * S, TrimMode::Roll));
    assert_eq!(
        sdr(clip(doc.project(), n)),
        (7 * S, S, 2 * S),
        "R2 reversed head trim keeps source_in"
    );
}

#[test]
fn c_trim_4_roll_errors() {
    let (mut b, x) = one(1.0, false);
    let (m, v1) = (b.m, b.v1);
    let n = b.fx.clip(v1, m, 6 * S + 1, 2 * S, 0); // 1 flick gap
    let mut doc = b.fx.clone_doc();
    assert_err!(&mut doc, trim(x, Edge::End, 7 * S, TrimMode::Roll), Error::NoAdjacentClip { clip_id } if clip_id == x);
    assert_err!(&mut doc, trim(x, Edge::Start, 3 * S, TrimMode::Roll), Error::NoAdjacentClip { clip_id } if clip_id == x);
    b.fx.c(n).start = 6 * S; // now adjacent, N [6s, 8s) source_in 0
    let mut doc = b.fx.doc();
    assert_err!(&mut doc, trim(x, Edge::End, 5 * S, TrimMode::Roll), Error::SourceOutOfRange { clip_id } if clip_id == n);
    assert_err!(
        &mut doc,
        trim(x, Edge::End, 8 * S, TrimMode::Roll),
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        trim(x, Edge::End, 2 * S, TrimMode::Roll),
        Error::InvalidArgument(_)
    );
}

// ============================================================================ C-SLIP-1

#[test]
fn c_slip_1_moves_source_only_with_linked() {
    let (mut b, x) = one(1.0, false);
    let (m, a1) = (b.m, b.a1);
    let xa = b.fx.clip(a1, m, 2 * S, 4 * S, S);
    b.fx.link(&[x, xa]);
    let mut doc = b.fx.clone_doc();
    ok(
        &mut doc,
        Command::SlipClip {
            clip_id: x,
            delta: S,
            include_linked: true,
        },
    );
    assert_eq!(
        (sdr(clip(doc.project(), x)), sdr(clip(doc.project(), xa))),
        ((2 * S, 4 * S, 2 * S), (2 * S, 4 * S, 2 * S))
    );
    ok(
        &mut doc,
        Command::SlipClip {
            clip_id: x,
            delta: -2 * S,
            include_linked: false,
        },
    );
    assert_eq!(
        (sdr(clip(doc.project(), x)), sdr(clip(doc.project(), xa))),
        ((2 * S, 4 * S, 0), (2 * S, 4 * S, 2 * S))
    );
}

#[test]
fn c_slip_1_r7_bounds() {
    let Base { mut fx, v1, .. } = base();
    let short = fx.media_full("Short", Some(10 * S), true, true);
    let x = fx.clip(v1, short, 2 * S, 4 * S, S); // uses [1s, 5s)
    let mut doc = fx.doc();
    assert_err!(
        &mut doc,
        Command::SlipClip {
            clip_id: x,
            delta: -S - 1,
            include_linked: true
        },
        Error::SourceOutOfRange { .. }
    );
    assert_err!(
        &mut doc,
        Command::SlipClip {
            clip_id: x,
            delta: 5 * S + SOURCE_SLACK + 1,
            include_linked: true
        },
        Error::SourceOutOfRange { .. }
    );
    ok(
        &mut doc,
        Command::SlipClip {
            clip_id: x,
            delta: 5 * S + SOURCE_SLACK,
            include_linked: true,
        },
    );
    assert_eq!(clip(doc.project(), x).source_in, 6 * S + 2);
}

#[test]
fn c_slip_1_kinds_and_errors() {
    let Base {
        mut fx, m, v1, v2, ..
    } = base();
    let inner = fx.seq("Inner");
    let it = fx.track(inner, TrackKind::Video);
    fx.clip(it, m, 0, S, 0);
    let sc = fx.clip_src(v1, seq_src(inner), 0, S, 0);
    let t = fx.text(v1, S, S);
    let so = fx.clip_src(v1, solid_src(), 2 * S, S, 0);
    let ad = fx.clip_src(v1, ClipSource::Adjustment, 3 * S, S, 0);
    let lk = fx.clip(v2, m, 0, S, S);
    fx.lock(v2);
    let mut doc = fx.doc();
    ok(
        &mut doc,
        Command::SlipClip {
            clip_id: sc,
            delta: 30 * S,
            include_linked: true,
        },
    );
    assert_eq!(clip(doc.project(), sc).source_in, 30 * S);
    for c in [t, so, ad] {
        assert_err!(
            &mut doc,
            Command::SlipClip {
                clip_id: c,
                delta: S,
                include_linked: true
            },
            Error::InvalidArgument(_)
        );
    }
    assert_err!(
        &mut doc,
        Command::SlipClip {
            clip_id: lk,
            delta: S,
            include_linked: true
        },
        Error::TrackLocked { .. }
    );
    assert_err!(
        &mut doc,
        Command::SlipClip {
            clip_id: 999,
            delta: S,
            include_linked: true
        },
        Error::NotFound { id: 999, .. }
    );
}

// ============================================================================ C-SLIDE-1

/// V1: P [0,2s) src 0, X [2s,4s) src 10s, N [4s,6s) src 1s.
fn slide_fx() -> (Base, Id, Id, Id) {
    let mut b = base();
    let (m, v1) = (b.m, b.v1);
    let p = b.fx.clip(v1, m, 0, 2 * S, 0);
    let x = b.fx.clip(v1, m, 2 * S, 2 * S, 10 * S);
    let n = b.fx.clip(v1, m, 4 * S, 2 * S, S);
    (b, p, x, n)
}

#[test]
fn c_slide_1_forward_and_back() {
    let (b, p, x, n) = slide_fx();
    let v1 = b.v1;
    let mut doc = b.fx.doc();
    ok(
        &mut doc,
        Command::SlideClip {
            clip_id: x,
            delta: S,
        },
    );
    let pr = doc.project();
    assert_eq!(
        spans(pr, v1),
        vec![(p, 0, 3 * S), (x, 3 * S, 5 * S), (n, 5 * S, 6 * S)]
    );
    assert_eq!(
        (
            clip(pr, p).source_in,
            clip(pr, x).source_in,
            clip(pr, n).source_in
        ),
        (0, 10 * S, 2 * S)
    );
    ok(
        &mut doc,
        Command::SlideClip {
            clip_id: x,
            delta: -2 * S,
        },
    );
    let pr = doc.project();
    assert_eq!(
        spans(pr, v1),
        vec![(p, 0, S), (x, S, 3 * S), (n, 3 * S, 6 * S)]
    );
    assert_eq!(
        (
            clip(pr, p).source_in,
            clip(pr, x).source_in,
            clip(pr, n).source_in
        ),
        (0, 10 * S, 0)
    );
}

#[test]
fn c_slide_1_errors() {
    let (mut b, _, x, n) = slide_fx();
    let mut doc = b.fx.clone_doc();
    assert_err!(
        &mut doc,
        Command::SlideClip {
            clip_id: x,
            delta: -2 * S
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::SlideClip {
            clip_id: x,
            delta: 2 * S
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::SlideClip {
            clip_id: 999,
            delta: S
        },
        Error::NotFound { id: 999, .. }
    );
    b.fx.c(n).source_in = 0;
    let mut doc = b.fx.clone_doc();
    assert_err!(&mut doc, Command::SlideClip { clip_id: x, delta: -S }, Error::SourceOutOfRange { clip_id } if clip_id == n);
    b.fx.c(n).start = 4 * S + 1;
    b.fx.c(n).duration = 2 * S - 1;
    let mut doc = b.fx.doc();
    assert_err!(&mut doc, Command::SlideClip { clip_id: x, delta: S }, Error::NoAdjacentClip { clip_id } if clip_id == x);
}

#[test]
fn c_slide_1_needs_previous_clip() {
    let Base { mut fx, m, v1, .. } = base();
    let x = fx.clip(v1, m, 0, 2 * S, 0);
    fx.clip(v1, m, 2 * S, 2 * S, 0);
    let mut doc = fx.doc();
    assert_err!(&mut doc, Command::SlideClip { clip_id: x, delta: S }, Error::NoAdjacentClip { clip_id } if clip_id == x);
}

// ============================================================================ C-SPEED-1

fn speed(clip_id: Id, speed: f64, ripple: bool) -> Command {
    Command::SetClipSpeed {
        clip_id,
        speed,
        ripple,
        include_linked: true,
    }
}

/// V1: X [0,4s) src 1s, Y [5s,7s); V2: Z [4s,5s); A2 locked: L [5s,6s).
fn speed_fx() -> (Base, [Id; 4]) {
    let mut b = base();
    let (m, v1, v2, a2) = (b.m, b.v1, b.v2, b.a2);
    let x = b.fx.clip(v1, m, 0, 4 * S, S);
    let y = b.fx.clip(v1, m, 5 * S, 2 * S, 0);
    let z = b.fx.clip(v2, m, 4 * S, S, 0);
    let l = b.fx.clip(a2, m, 5 * S, S, 0);
    b.fx.lock(a2);
    (b, [x, y, z, l])
}

#[test]
fn c_speed_1_spec_example_no_ripple() {
    let (b, [x, y, z, l]) = speed_fx();
    let (v1, v2, a2) = (b.v1, b.v2, b.a2);
    let mut doc = b.fx.doc();
    ok(&mut doc, speed(x, 2.0, false));
    let p = doc.project();
    assert_eq!(sdr(clip(p, x)), (0, 2 * S, S));
    assert_eq!(clip(p, x).speed, 2.0);
    assert_eq!(spans(p, v1), vec![(x, 0, 2 * S), (y, 5 * S, 7 * S)]);
    assert_eq!(spans(p, v2), vec![(z, 4 * S, 5 * S)]);
    assert_eq!(spans(p, a2), vec![(l, 5 * S, 6 * S)]);
}

#[test]
fn c_speed_1_r1_ripple() {
    let (b, [x, y, z, l]) = speed_fx();
    let (v1, v2, a2) = (b.v1, b.v2, b.a2);
    let mut doc = b.fx.clone_doc();
    ok(&mut doc, speed(x, 2.0, true));
    let p = doc.project();
    assert_eq!(spans(p, v1), vec![(x, 0, 2 * S), (y, 3 * S, 5 * S)]);
    assert_eq!(
        spans(p, v2),
        vec![(z, 2 * S, 3 * S)],
        "start >= old end (4s) shifts"
    );
    assert_eq!(spans(p, a2), vec![(l, 5 * S, 6 * S)]);
    let mut doc = b.fx.doc();
    ok(&mut doc, speed(x, 0.5, true));
    let p = doc.project();
    assert_eq!(spans(p, v1), vec![(x, 0, 8 * S), (y, 9 * S, 11 * S)]);
    assert_eq!(spans(p, v2), vec![(z, 8 * S, 9 * S)]);
}

#[test]
fn c_speed_1_overlap_without_ripple() {
    let (b, [x, ..]) = speed_fx();
    let v1 = b.v1;
    let mut doc = b.fx.doc();
    assert_err!(&mut doc, speed(x, 0.5, false), Error::Overlap { track_id } if track_id == v1);
}

#[test]
fn c_speed_1_uses_old_speed() {
    let (mut b, [x, ..]) = speed_fx();
    b.fx.c(x).duration = 2 * S;
    b.fx.c(x).speed = 2.0;
    let mut doc = b.fx.doc();
    ok(&mut doc, speed(x, 1.0, false));
    assert_eq!(sdr(clip(doc.project(), x)), (0, 4 * S, S));
}

#[test]
fn c_speed_1_rounding_and_min_duration() {
    let Base {
        mut fx, still, v1, ..
    } = base();
    let a = fx.clip(v1, still, 0, 3, 0);
    let b_ = fx.clip(v1, still, S, 1, 0);
    let mut doc = fx.doc();
    ok(&mut doc, speed(a, 2.0, false));
    assert_eq!(clip(doc.project(), a).duration, 2, "round(1.5) = 2");
    ok(&mut doc, speed(b_, 100.0, false));
    assert_eq!(clip(doc.project(), b_).duration, 1, "max(1, round(0.01))");
}

#[test]
fn c_speed_1_r7_source_bounds() {
    let Base { mut fx, v1, .. } = base();
    let tiny = fx.media_full("Tiny", Some(1), true, false);
    let c = fx.clip(v1, tiny, 0, 1, 0);
    let mut doc = fx.doc();
    // new duration 1, source use round(1 * 100) = 100 > 1 + 2
    assert_err!(&mut doc, speed(c, 100.0, false), Error::SourceOutOfRange { clip_id } if clip_id == c);
}

#[test]
fn c_speed_1_range_and_kinds() {
    let (mut b, [x, ..]) = speed_fx();
    let v2 = b.v2;
    let t = b.fx.text(v2, 10 * S, S);
    let mut doc = b.fx.clone_doc();
    for s in [0.0099, 100.5, 0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert_err!(&mut doc, speed(x, s, true), Error::InvalidArgument(_));
    }
    assert_err!(&mut doc, speed(t, 2.0, true), Error::InvalidArgument(_));
    assert_err!(
        &mut doc,
        speed(999, 2.0, true),
        Error::NotFound { id: 999, .. }
    );
    ok(&mut doc, speed(x, 100.0, true));
    assert_eq!(clip(doc.project(), x).duration, 4 * S / 100);
    let mut doc = b.fx.doc();
    ok(&mut doc, speed(x, 0.01, true));
    assert_eq!(clip(doc.project(), x).duration, 400 * S);
}

#[test]
fn c_speed_1_linked() {
    let (mut b, [x, ..]) = speed_fx();
    let (m, a1) = (b.m, b.a1);
    let xa = b.fx.clip(a1, m, 0, 4 * S, S);
    b.fx.link(&[x, xa]);
    let mut doc = b.fx.clone_doc();
    ok(&mut doc, speed(x, 2.0, false));
    assert_eq!(
        (
            clip(doc.project(), xa).duration,
            clip(doc.project(), xa).speed
        ),
        (2 * S, 2.0)
    );
    let mut doc = b.fx.doc();
    ok(
        &mut doc,
        Command::SetClipSpeed {
            clip_id: x,
            speed: 2.0,
            ripple: false,
            include_linked: false,
        },
    );
    assert_eq!(
        (
            clip(doc.project(), xa).duration,
            clip(doc.project(), xa).speed
        ),
        (4 * S, 1.0)
    );
}

#[test]
fn c_speed_1_sequence_clip_ok() {
    let Base { mut fx, m, v1, .. } = base();
    let inner = fx.seq("Inner");
    let it = fx.track(inner, TrackKind::Video);
    fx.clip(it, m, 0, S, 0);
    let c = fx.clip_src(v1, seq_src(inner), 0, 4 * S, 0);
    let mut doc = fx.doc();
    ok(&mut doc, speed(c, 4.0, false));
    assert_eq!(clip(doc.project(), c).duration, S);
}
