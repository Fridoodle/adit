//! SPEC Part 5 hardening rules for adit-core: H-3 (time bounds), H-4 (nesting depth),
//! H-10 (pruning inside batches), H-15 (RemoveSequence with a locked track).
//! Expected values are SPEC literals and hand arithmetic; G1 is checked by `err`.
mod common;
use common::*;

const MT: Time = MAX_TIME;

fn solid_add(track: Id, start: Time, duration: Time) -> Command {
    add(track, solid_src(), start, duration)
}

fn move_to(clip_id: Id, start: Time) -> Command {
    Command::MoveClip {
        clip_id,
        start,
        track_id: None,
        mode: PlaceMode::Place,
        include_linked: true,
    }
}

fn speed(clip_id: Id, speed: f64, ripple: bool) -> Command {
    Command::SetClipSpeed {
        clip_id,
        speed,
        ripple,
        include_linked: true,
    }
}

fn kf_cmd(clip_id: Id, time: Time) -> Command {
    Command::SetKeyframe {
        target: pt(clip_id, "opacity"),
        time,
        value: 0.5,
        easing: Easing::Linear,
    }
}

fn marker_cmd(sequence_id: Id, time: Time, duration: Time) -> Command {
    Command::AddMarker {
        sequence_id,
        time,
        duration,
        name: String::new(),
        note: String::new(),
    }
}

// ------------------------------------------------------------------------------------- H-3

#[test]
fn h_3_max_time_is_3_600_000_seconds() {
    assert_eq!(MAX_TIME, 3_600_000 * 705_600_000);
}

#[test]
fn h_3_add_clip_duration_max_time_ok() {
    let b = base();
    let mut doc = b.fx.doc();
    ok(&mut doc, solid_add(b.v1, 0, MT));
    assert_eq!(spans(doc.project(), b.v1)[0].2, MT);
}

#[test]
fn h_3_add_clip_duration_above_max_time_rejected() {
    let b = base();
    let mut doc = b.fx.doc();
    assert_err!(
        &mut doc,
        solid_add(b.v1, 0, MT + 1),
        Error::InvalidArgument(_)
    );
}

#[test]
fn h_3_add_clip_ending_exactly_at_max_time_ok() {
    let b = base();
    let mut doc = b.fx.doc();
    ok(&mut doc, solid_add(b.v1, MT - S, S));
}

#[test]
fn h_3_add_clip_ending_past_max_time_rejected() {
    let b = base();
    let mut doc = b.fx.doc();
    // end = MT - S + 1 + S = MT + 1
    assert_err!(
        &mut doc,
        solid_add(b.v1, MT - S + 1, S),
        Error::InvalidArgument(_)
    );
    // start == MAX_TIME is allowed as a value, but the clip would end at MT + 1.
    assert_err!(&mut doc, solid_add(b.v1, MT, 1), Error::InvalidArgument(_));
}

#[test]
fn h_3_add_clip_start_above_max_time_rejected() {
    let b = base();
    let mut doc = b.fx.doc();
    assert_err!(
        &mut doc,
        solid_add(b.v1, MT + 1, S),
        Error::InvalidArgument(_)
    );
}

#[test]
fn h_3_add_clip_near_i64_max_start_rejected() {
    let b = base();
    let mut doc = b.fx.doc();
    assert_err!(
        &mut doc,
        solid_add(b.v1, i64::MAX - S, 1),
        Error::InvalidArgument(_)
    );
}

#[test]
fn h_3_add_clip_overflowing_end_rejected() {
    let b = base();
    let mut doc = b.fx.doc();
    // start + duration overflows i64: must be an ordinary InvalidArgument.
    assert_err!(
        &mut doc,
        solid_add(b.v1, i64::MAX - 10, 100),
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        solid_add(b.v1, i64::MAX, i64::MAX),
        Error::InvalidArgument(_)
    );
}

#[test]
fn h_3_add_clip_source_in_bound() {
    let b = base();
    let mut doc = b.fx.doc();
    ok(
        &mut doc,
        AddB {
            source_in: MT,
            ..addb(b.v1, solid_src(), 0, S)
        }
        .cmd(),
    );
    assert_err!(
        &mut doc,
        AddB {
            source_in: MT + 1,
            ..addb(b.v2, solid_src(), 0, S)
        }
        .cmd(),
        Error::InvalidArgument(_)
    );
    // A still (no duration) has no I3 bound, but H-3 still caps source_in.
    assert_err!(
        &mut doc,
        AddB {
            source_in: MT + 1,
            ..addb(b.v2, media_src(b.still), 0, S)
        }
        .cmd(),
        Error::InvalidArgument(_)
    );
}

#[test]
fn h_3_add_media_duration_bound() {
    let b = base();
    let mut doc = b.fx.doc();
    let spec = |d: Time| MediaSpec {
        name: "long".into(),
        path: "/media/long.mp4".into(),
        duration: Some(d),
        video: None,
        audio: Some(AudioInfo {
            sample_rate: 48000,
            channels: 2,
        }),
    };
    ok(&mut doc, Command::AddMedia { spec: spec(MT) });
    assert_err!(
        &mut doc,
        Command::AddMedia { spec: spec(MT + 1) },
        Error::InvalidArgument(_)
    );
}

#[test]
fn h_3_marker_time_bound() {
    let b = base();
    let mut doc = b.fx.doc();
    ok(&mut doc, marker_cmd(b.seq, MT, 0));
    assert_err!(
        &mut doc,
        marker_cmd(b.seq, MT + 1, 0),
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        marker_cmd(b.seq, i64::MAX, 0),
        Error::InvalidArgument(_)
    );
}

#[test]
fn h_3_marker_duration_bound() {
    let b = base();
    let mut doc = b.fx.doc();
    assert_err!(
        &mut doc,
        marker_cmd(b.seq, 0, MT + 1),
        Error::InvalidArgument(_)
    );
}

#[test]
fn h_3_move_clip_bound() {
    let mut b = base();
    let c = b.fx.clip_src(b.v1, solid_src(), 0, S, 0);
    let mut doc = b.fx.clone_doc();
    assert_err!(&mut doc, move_to(c, MT - S + 1), Error::InvalidArgument(_));
    assert_err!(&mut doc, move_to(c, MT + 1), Error::InvalidArgument(_));
    assert_err!(&mut doc, move_to(c, i64::MAX), Error::InvalidArgument(_));
    ok(&mut doc, move_to(c, MT - S));
    assert_eq!(spans(doc.project(), b.v1), vec![(c, MT - S, MT)]);
}

#[test]
fn h_3_trim_to_bound() {
    let mut b = base();
    let c = b.fx.clip_src(b.v1, solid_src(), 0, S, 0);
    let mut doc = b.fx.clone_doc();
    assert_err!(
        &mut doc,
        trim(c, Edge::End, MT + 1, TrimMode::Normal),
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        trim(c, Edge::End, i64::MAX, TrimMode::Normal),
        Error::InvalidArgument(_)
    );
    ok(&mut doc, trim(c, Edge::End, MT, TrimMode::Normal));
    assert_eq!(spans(doc.project(), b.v1), vec![(c, 0, MT)]);
}

#[test]
fn h_3_split_at_time_bound() {
    let mut b = base();
    b.fx.clip_src(b.v1, solid_src(), 0, S, 0);
    let mut doc = b.fx.clone_doc();
    assert_err!(
        &mut doc,
        Command::SplitAt {
            sequence_id: b.seq,
            time: MT + 1,
            track_ids: None,
        },
        Error::InvalidArgument(_)
    );
    // At MAX_TIME nothing is crossed: Ok, no change (C-SPLIT-1).
    ok(
        &mut doc,
        Command::SplitAt {
            sequence_id: b.seq,
            time: MT,
            track_ids: None,
        },
    );
}

#[test]
fn h_3_lift_and_extract_range_bound() {
    let mut b = base();
    b.fx.clip_src(b.v1, solid_src(), 0, S, 0);
    let mut doc = b.fx.clone_doc();
    let lift = |start, end| Command::LiftRange {
        sequence_id: b.seq,
        start,
        end,
        track_ids: None,
    };
    let extract = |start, end| Command::ExtractRange {
        sequence_id: b.seq,
        start,
        end,
        track_ids: None,
    };
    assert_err!(&mut doc, lift(2 * S, MT + 1), Error::InvalidArgument(_));
    assert_err!(&mut doc, lift(MT + 1, MT + 2), Error::InvalidArgument(_));
    assert_err!(&mut doc, extract(2 * S, MT + 1), Error::InvalidArgument(_));
    ok(&mut doc, lift(2 * S, MT));
    ok(&mut doc, extract(2 * S, MT));
}

#[test]
fn h_3_keyframe_time_bound() {
    let mut b = base();
    let c = b.fx.clip_src(b.v1, solid_src(), 0, S, 0);
    let mut doc = b.fx.clone_doc();
    assert_err!(&mut doc, kf_cmd(c, MT + 1), Error::InvalidArgument(_));
    assert_err!(&mut doc, kf_cmd(c, -MT - 1), Error::InvalidArgument(_));
    assert_err!(&mut doc, kf_cmd(c, i64::MIN), Error::InvalidArgument(_));
    ok(&mut doc, kf_cmd(c, MT));
    ok(&mut doc, kf_cmd(c, -MT));
    let times: Vec<Time> = clip(doc.project(), c).params["opacity"]
        .keyframes
        .iter()
        .map(|k| k.time)
        .collect();
    assert_eq!(times, vec![-MT, MT]);
}

#[test]
fn h_3_slip_delta_bound() {
    let mut b = base();
    // A still has no source length, so only H-3 limits the slip.
    let c = b.fx.clip(b.v1, b.still, 0, S, 0);
    let mut doc = b.fx.clone_doc();
    let slip = |delta| Command::SlipClip {
        clip_id: c,
        delta,
        include_linked: true,
    };
    assert_err!(&mut doc, slip(MT + 1), Error::InvalidArgument(_));
    assert_err!(&mut doc, slip(-MT - 1), Error::InvalidArgument(_));
    ok(&mut doc, slip(MT));
    assert_eq!(clip(doc.project(), c).source_in, MT);
}

#[test]
fn h_3_slide_delta_bound() {
    let mut b = base();
    b.fx.clip_src(b.v1, solid_src(), 0, S, 0);
    let x = b.fx.clip_src(b.v1, solid_src(), S, S, 0);
    b.fx.clip_src(b.v1, solid_src(), 2 * S, S, 0);
    let mut doc = b.fx.clone_doc();
    let slide = |delta| Command::SlideClip { clip_id: x, delta };
    assert_err!(&mut doc, slide(MT + 1), Error::InvalidArgument(_));
    assert_err!(&mut doc, slide(-MT - 1), Error::InvalidArgument(_));
}

#[test]
fn h_3_set_clip_speed_end_bound() {
    let mut b = base();
    let c = b.fx.clip(b.v1, b.still, 0, MT / 2, 0);
    let mut doc = b.fx.clone_doc();
    // new_duration = round((MT / 2) * 1 / 0.5) = MT: ends exactly at MAX_TIME.
    ok(&mut doc, speed(c, 0.5, false));
    assert_eq!(spans(doc.project(), b.v1), vec![(c, 0, MT)]);
    // round(MT * 0.5 / 0.25) = 2 MT: past MAX_TIME.
    assert_err!(&mut doc, speed(c, 0.25, false), Error::InvalidArgument(_));
}

/// V1: A solid [0, 1s), B solid [MT - 1s, MT).
fn edge_fixture() -> (Base, Id, Id) {
    let mut b = base();
    let a = b.fx.clip_src(b.v1, solid_src(), 0, S, 0);
    let bb = b.fx.clip_src(b.v1, solid_src(), MT - S, S, 0);
    (b, a, bb)
}

#[test]
fn h_3_insert_ripple_past_max_time_blocked() {
    let (b, _, _) = edge_fixture();
    let mut doc = b.fx.clone_doc();
    // Insert 1s at 0 on V2 shifts B to [MT, MT + 1s).
    assert_err!(
        &mut doc,
        AddB {
            mode: PlaceMode::Insert,
            ..addb(b.v2, solid_src(), 0, S)
        }
        .cmd(),
        Error::RippleBlocked { .. }
    );
}

#[test]
fn h_3_ripple_trim_past_max_time_blocked() {
    let (b, a, _) = edge_fixture();
    let mut doc = b.fx.clone_doc();
    // A's end 1s -> 2s ripples B by +1s past MAX_TIME.
    assert_err!(
        &mut doc,
        trim(a, Edge::End, 2 * S, TrimMode::Ripple),
        Error::RippleBlocked { .. }
    );
}

#[test]
fn h_3_speed_ripple_past_max_time_blocked() {
    let mut b = base();
    let a = b.fx.clip(b.v1, b.still, 0, S, 0);
    b.fx.clip_src(b.v1, solid_src(), MT - S, S, 0);
    let mut doc = b.fx.clone_doc();
    // A: 1s at speed 1 -> speed 0.5 = 2s; B shifts by +1s past MAX_TIME.
    assert_err!(&mut doc, speed(a, 0.5, true), Error::RippleBlocked { .. });
}

#[test]
fn h_3_validate_accepts_values_at_the_bounds() {
    let mut b = base();
    let c = b.fx.clip_src(b.v1, solid_src(), MT - S, S, MT);
    b.fx.c(c).params.insert(
        "opacity".into(),
        param(
            1.0,
            vec![kf(-MT, 0.0, Easing::Linear), kf(MT, 1.0, Easing::Linear)],
        ),
    );
    b.fx.marker(b.seq, MT, 0);
    assert_eq!(b.fx.p.validate(), Ok(()));
}

#[test]
fn h_3_validate_rejects_clip_ending_past_max_time() {
    let mut b = base();
    b.fx.clip_src(b.v1, solid_src(), MT - S + 1, S, 0);
    assert!(b.fx.p.validate().is_err());
}

#[test]
fn h_3_validate_rejects_overflowing_clip_end() {
    let mut b = base();
    b.fx.clip_src(b.v1, solid_src(), i64::MAX - 10, 100, 0);
    assert!(b.fx.p.validate().is_err());
}

#[test]
fn h_3_validate_rejects_source_in_past_max_time() {
    let mut b = base();
    b.fx.clip_src(b.v1, solid_src(), 0, S, MT + 1);
    assert!(b.fx.p.validate().is_err());
}

#[test]
fn h_3_validate_rejects_marker_past_max_time() {
    let mut b = base();
    b.fx.marker(b.seq, MT + 1, 0);
    assert!(b.fx.p.validate().is_err());
}

#[test]
fn h_3_validate_rejects_keyframe_past_max_time() {
    for t in [MT + 1, -MT - 1] {
        let mut b = base();
        let c = b.fx.clip_src(b.v1, solid_src(), 0, S, 0);
        b.fx.c(c).params.insert(
            "opacity".into(),
            param(1.0, vec![kf(t, 0.0, Easing::Linear)]),
        );
        assert!(b.fx.p.validate().is_err(), "keyframe at {t}");
    }
}

// ------------------------------------------------------------------------------------- H-4

/// `n` sequences, each with one video track; sequence i nests sequence i + 1 (1s clip at 0).
/// The first sequence has nesting depth `n`. Returns (fixture, sequence ids, track ids).
fn chain(n: usize) -> (Fx, Vec<Id>, Vec<Id>) {
    let mut fx = Fx::new();
    let mut seqs = Vec::new();
    let mut tracks = Vec::new();
    for i in 0..n {
        let s = fx.seq(&format!("S{i}"));
        seqs.push(s);
        tracks.push(fx.track(s, TrackKind::Video));
    }
    for i in 0..n - 1 {
        fx.clip_src(tracks[i], seq_src(seqs[i + 1]), 0, S, 0);
    }
    (fx, seqs, tracks)
}

#[test]
fn h_4_max_nest_depth_is_16() {
    assert_eq!(MAX_NEST_DEPTH, 16);
}

#[test]
fn h_4_add_clip_reaching_depth_16_ok() {
    let (mut fx, _, tracks) = chain(15);
    let leaf = fx.seq("leaf");
    fx.track(leaf, TrackKind::Video);
    let mut doc = fx.doc();
    // S14 nests leaf: S0 depth 15 -> 16.
    ok(&mut doc, add(tracks[14], seq_src(leaf), 0, S));
}

#[test]
fn h_4_add_clip_reaching_depth_17_rejected() {
    let (mut fx, _, tracks) = chain(16);
    let leaf = fx.seq("leaf");
    fx.track(leaf, TrackKind::Video);
    let mut doc = fx.doc();
    // S15 nests leaf: S0 depth 16 -> 17.
    assert_err!(
        &mut doc,
        add(tracks[15], seq_src(leaf), 0, S),
        Error::InvalidArgument(_)
    );
}

#[test]
fn h_4_add_clip_on_top_of_depth_16_rejected() {
    let (mut fx, seqs, _) = chain(16);
    let top = fx.seq("top");
    let tt = fx.track(top, TrackKind::Video);
    let mut doc = fx.doc();
    // top would nest S0 (depth 16): top depth 17.
    assert_err!(
        &mut doc,
        add(tt, seq_src(seqs[0]), 0, S),
        Error::InvalidArgument(_)
    );
}

#[test]
fn h_4_add_clip_in_batch_reaching_depth_17_rejected() {
    let (mut fx, _, tracks) = chain(16);
    let leaf = fx.seq("leaf");
    fx.track(leaf, TrackKind::Video);
    let mut doc = fx.doc();
    assert_err!(
        &mut doc,
        Command::Batch {
            commands: vec![add(tracks[15], seq_src(leaf), 0, S)],
        },
        Error::InvalidArgument(_)
    );
}

#[test]
fn h_4_validate_depth_16_ok() {
    let (fx, _, _) = chain(16);
    assert_eq!(fx.p.validate(), Ok(()));
}

#[test]
fn h_4_validate_depth_17_rejected() {
    let (fx, _, _) = chain(17);
    assert!(fx.p.validate().is_err());
}

// ------------------------------------------------------------------------------------- H-10

#[test]
fn h_10_batch_prunes_after_each_command() {
    let mut b = base();
    let p = b.fx.clip(b.v1, b.m, 0, 2 * S, 0);
    let q = b.fx.clip(b.v1, b.m, 2 * S, 2 * S, 2 * S);
    b.fx.transition(b.v1, Some(p), Some(q), S);
    let before = b.fx.p.clone();
    let mut doc = b.fx.doc();
    ok(
        &mut doc,
        Command::Batch {
            commands: vec![move_to(q, 3 * S), move_to(q, 2 * S)],
        },
    );
    // Q is back where it was, but the transition was pruned after the first move.
    assert_eq!(
        spans(doc.project(), b.v1),
        vec![(p, 0, 2 * S), (q, 2 * S, 4 * S)]
    );
    assert!(
        trk(doc.project(), b.v1).transitions.is_empty(),
        "H-10: transition must be pruned inside the batch"
    );
    assert_eq!(doc.undo_depth(), 1, "one undo entry for the batch");
    assert!(doc.undo());
    assert_eq!(doc.project(), &before, "undo brings the transition back");
    assert_eq!(doc.undo_depth(), 0);
}

// ------------------------------------------------------------------------------------- H-15

#[test]
fn h_15_remove_sequence_with_locked_track_rejected() {
    let mut b = base();
    b.fx.lock(b.v2);
    let v2 = b.v2;
    let seq = b.seq;
    let mut doc = b.fx.doc();
    assert_err!(
        &mut doc,
        Command::RemoveSequence { sequence_id: seq },
        Error::TrackLocked { track_id } if track_id == v2
    );
}

#[test]
fn h_15_remove_sequence_without_locked_track_ok() {
    let b = base();
    let seq = b.seq;
    let mut doc = b.fx.doc();
    ok(&mut doc, Command::RemoveSequence { sequence_id: seq });
    assert!(doc.project().sequences.is_empty());
}
