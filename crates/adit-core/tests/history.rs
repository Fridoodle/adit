//! General rules: G1-G4, G6, G7, G9 and C-BATCH-1.
#[macro_use]
mod common;
use common::*;

fn doc_with_clip() -> (Document, Base, Id) {
    let mut b = base();
    let (m, v1) = (b.m, b.v1);
    let x = b.fx.clip(v1, m, 0, 4 * S, 0);
    let doc = b.fx.clone_doc();
    (doc, b, x)
}

// ============================================================================ G1

#[test]
fn g1_error_keeps_project_and_both_stacks() {
    let (mut doc, b, x) = doc_with_clip();
    let seq = b.seq;
    ok(&mut doc, add_marker(seq, S));
    ok(&mut doc, add_marker(seq, 2 * S));
    assert!(doc.undo());
    assert_eq!((doc.undo_depth(), doc.redo_depth()), (1, 1));
    // a failure in the middle of a multi-step command must not leave partial edits
    assert_err!(
        &mut doc,
        Command::Batch {
            commands: vec![
                Command::SplitClip {
                    clip_id: x,
                    time: 2 * S,
                    include_linked: true
                },
                add_marker(seq, 3 * S),
                Command::RemoveClips {
                    clip_ids: vec![999],
                    include_linked: true
                },
            ]
        },
        Error::NotFound { id: 999, .. }
    );
    assert_eq!((doc.undo_depth(), doc.redo_depth()), (1, 1));
    assert!(doc.redo(), "redo still available after an Err");
    assert_eq!(sq(doc.project(), seq).markers.len(), 2);
}

#[test]
fn g1_failing_ripple_after_partial_shift() {
    // Two ranges; the later one shifts fine, the earlier one is blocked. Nothing may change.
    let Base {
        mut fx, m, v1, v2, ..
    } = base();
    let p = fx.clip(v1, m, 0, 2 * S, 0);
    fx.clip(v1, m, 2 * S, 2 * S, 0);
    let r = fx.clip(v1, m, 4 * S, 2 * S, 0);
    fx.clip(v1, m, 6 * S, 2 * S, 0);
    fx.clip(v2, m, S, S / 2, 0); // intersects P's range only
    let mut doc = fx.doc();
    assert_err!(
        &mut doc,
        Command::RippleDeleteClips {
            clip_ids: vec![p, r],
            include_linked: true
        },
        Error::RippleBlocked { .. }
    );
}

// ============================================================================ G2

#[test]
fn g2_change_pushes_one_entry_and_clears_redo() {
    let (mut doc, b, _) = doc_with_clip();
    let seq = b.seq;
    ok(&mut doc, add_marker(seq, S));
    ok(&mut doc, add_marker(seq, 2 * S));
    assert_eq!(doc.undo_depth(), 2);
    assert!(doc.undo());
    assert_eq!(doc.redo_depth(), 1);
    ok(&mut doc, add_marker(seq, 3 * S));
    assert_eq!((doc.undo_depth(), doc.redo_depth()), (2, 0));
    assert!(!doc.redo());
}

#[test]
fn g2_noop_pushes_nothing_and_keeps_redo() {
    let (mut doc, b, x) = doc_with_clip();
    let (seq, v1) = (b.seq, b.v1);
    ok(&mut doc, add_marker(seq, S));
    assert!(doc.undo());
    let before = doc.project().clone();
    // same name as now -> project == before
    ok(
        &mut doc,
        Command::SetTrack {
            track_id: v1,
            name: Some("V1".into()),
            locked: None,
            muted: None,
            solo: None,
            hidden: None,
        },
    );
    ok(
        &mut doc,
        Command::SplitAt {
            sequence_id: seq,
            time: 10 * S,
            track_ids: None,
        },
    );
    ok(&mut doc, Command::Batch { commands: vec![] });
    ok(
        &mut doc,
        Command::SetClip {
            clip_id: x,
            name: None,
            enabled: None,
            reverse: None,
            blend: None,
            audio_filter: None,
            label: None,
        },
    );
    assert_eq!(doc.project(), &before);
    assert_eq!((doc.undo_depth(), doc.redo_depth()), (0, 1));
    assert!(doc.redo());
}

// ============================================================================ G3

#[test]
fn g3_undo_redo_exact() {
    let (mut doc, b, x) = doc_with_clip();
    let seq = b.seq;
    let s0 = doc.project().clone();
    ok(
        &mut doc,
        Command::SplitClip {
            clip_id: x,
            time: S,
            include_linked: true,
        },
    );
    let s1 = doc.project().clone();
    ok(&mut doc, add_marker(seq, S));
    let s2 = doc.project().clone();
    assert!(doc.undo());
    assert_eq!(doc.project(), &s1);
    assert!(doc.undo());
    assert_eq!(doc.project(), &s0);
    assert!(!doc.undo());
    assert_eq!(doc.project(), &s0);
    assert_eq!((doc.undo_depth(), doc.redo_depth()), (0, 2));
    assert!(doc.redo());
    assert_eq!(doc.project(), &s1);
    assert!(doc.redo());
    assert_eq!(doc.project(), &s2);
    assert!(!doc.redo());
    assert_eq!(doc.project(), &s2);
    assert_eq!((doc.undo_depth(), doc.redo_depth()), (2, 0));
}

#[test]
fn g3_empty_document_has_nothing_to_undo() {
    let mut doc = Document::new(Project::new("p"));
    let before = doc.project().clone();
    assert!(!doc.undo());
    assert!(!doc.redo());
    assert_eq!(doc.project(), &before);
}

#[test]
fn g3_history_is_unlimited() {
    let (mut doc, b, _) = doc_with_clip();
    let seq = b.seq;
    let s0 = doc.project().clone();
    for i in 0..300 {
        ok(&mut doc, add_marker(seq, i * S));
    }
    assert_eq!(doc.undo_depth(), 300);
    let end = doc.project().clone();
    for _ in 0..300 {
        assert!(doc.undo());
    }
    assert!(!doc.undo());
    assert_eq!(doc.project(), &s0);
    for _ in 0..300 {
        assert!(doc.redo());
    }
    assert_eq!(doc.project(), &end);
}

// ============================================================================ G4

#[test]
fn g4_ids_from_next_id_and_undo_restores_it() {
    let (mut doc, b, x) = doc_with_clip();
    let seq = b.seq;
    let n = doc.project().next_id;
    assert_eq!(n, 11);
    assert_eq!(ok(&mut doc, add_marker(seq, 0)).created, vec![11]);
    assert_eq!(ok(&mut doc, add_marker(seq, 0)).created, vec![12]);
    assert_eq!(doc.project().next_id, 13);
    // removing things never lowers next_id
    ok(
        &mut doc,
        Command::RemoveClips {
            clip_ids: vec![x],
            include_linked: true,
        },
    );
    assert_eq!(doc.project().next_id, 13);
    assert!(doc.undo());
    assert!(doc.undo());
    assert_eq!(doc.project().next_id, 12, "undo restores the old next_id");
    assert_eq!(ok(&mut doc, add_marker(seq, S)).created, vec![12]);
}

// ============================================================================ G6 / G7 (cross-command)

#[test]
fn g6_empty_clip_ids_everywhere() {
    let (mut doc, ..) = doc_with_clip();
    for cmd in [
        Command::RemoveClips {
            clip_ids: vec![],
            include_linked: true,
        },
        Command::RippleDeleteClips {
            clip_ids: vec![],
            include_linked: false,
        },
        Command::LinkClips { clip_ids: vec![] },
        Command::UnlinkClips { clip_ids: vec![] },
        Command::GroupClips { clip_ids: vec![] },
        Command::UngroupClips { clip_ids: vec![] },
    ] {
        assert_err!(&mut doc, cmd, Error::InvalidArgument(_));
    }
}

#[test]
fn g6_non_finite_and_negative_values() {
    let (mut doc, b, x) = doc_with_clip();
    let (seq, v1) = (b.seq, b.v1);
    let mut nan_params = std::collections::BTreeMap::new();
    nan_params.insert("k".to_string(), f64::NAN);
    for cmd in [
        Command::SetParam {
            target: pt(x, "opacity"),
            value: f64::NEG_INFINITY,
        },
        Command::SetKeyframe {
            target: pt(x, "opacity"),
            time: 0,
            value: f64::NAN,
            easing: Easing::Linear,
        },
        Command::SetClipSpeed {
            clip_id: x,
            speed: f64::INFINITY,
            ripple: false,
            include_linked: true,
        },
        Command::AddTransition {
            track_id: v1,
            left: Some(x),
            right: None,
            effect: "d".into(),
            duration: S,
            params: nan_params,
        },
        add_m(v1, b.m, 10 * S, 0),
        add_m(v1, b.m, -S, S),
        Command::AddMarker {
            sequence_id: seq,
            time: -1,
            duration: 0,
            name: String::new(),
            note: String::new(),
        },
        Command::SplitAt {
            sequence_id: seq,
            time: -1,
            track_ids: None,
        },
        Command::LiftRange {
            sequence_id: seq,
            start: -S,
            end: S,
            track_ids: None,
        },
        Command::ExtractRange {
            sequence_id: seq,
            start: -S,
            end: S,
            track_ids: None,
        },
        Command::MoveClip {
            clip_id: x,
            start: -1,
            track_id: None,
            mode: PlaceMode::Place,
            include_linked: true,
        },
    ] {
        assert_err!(&mut doc, cmd, Error::InvalidArgument(_));
    }
}

#[test]
fn g7_not_found_carries_the_id() {
    let (mut doc, ..) = doc_with_clip();
    for (cmd, id) in [
        (Command::RemoveMedia { media_id: 500 }, 500),
        (Command::RemoveSequence { sequence_id: 501 }, 501),
        (Command::RemoveTrack { track_id: 502 }, 502),
        (Command::RemoveMarker { marker_id: 503 }, 503),
        (
            Command::SlipClip {
                clip_id: 504,
                delta: S,
                include_linked: true,
            },
            504,
        ),
        (
            Command::SlideClip {
                clip_id: 505,
                delta: S,
            },
            505,
        ),
        (Command::RemoveTransition { transition_id: 506 }, 506),
        (
            Command::SplitClip {
                clip_id: 507,
                time: S,
                include_linked: true,
            },
            507,
        ),
        (
            Command::CloseGap {
                track_id: 508,
                time: S,
            },
            508,
        ),
        (
            Command::TrimClip {
                clip_id: 509,
                edge: Edge::End,
                to: S,
                mode: TrimMode::Normal,
                include_linked: true,
            },
            509,
        ),
    ] {
        assert_err!(&mut doc, cmd, Error::NotFound { id: got, .. } if got == id);
    }
}

// ============================================================================ C-BATCH-1

#[test]
fn c_batch_1_runs_in_order_one_undo_entry() {
    let (mut doc, b, x) = doc_with_clip();
    let (seq, v1) = (b.seq, b.v1);
    let s0 = doc.project().clone();
    let out = ok(
        &mut doc,
        Command::Batch {
            commands: vec![
                add_marker(seq, 0), // 11
                Command::SplitClip {
                    clip_id: x,
                    time: 2 * S,
                    include_linked: true,
                }, // 12
                // the second command's result is visible to the third: clip 12 exists now
                Command::TrimClip {
                    clip_id: 12,
                    edge: Edge::End,
                    to: 3 * S,
                    mode: TrimMode::Normal,
                    include_linked: true,
                },
                Command::Batch {
                    commands: vec![add_marker(seq, S), Command::Batch { commands: vec![] }],
                }, // 13
            ],
        },
    );
    assert_eq!(out.created, vec![11, 12, 13]);
    assert_eq!(doc.undo_depth(), 1);
    assert_eq!(
        spans(doc.project(), v1),
        vec![(x, 0, 2 * S), (12, 2 * S, 3 * S)]
    );
    assert_eq!(
        sq(doc.project(), seq)
            .markers
            .iter()
            .map(|m| m.id)
            .collect::<Vec<_>>(),
        vec![11, 13]
    );
    assert!(doc.undo());
    assert_eq!(doc.project(), &s0);
}

#[test]
fn c_batch_1_first_error_fails_all() {
    let (mut doc, b, x) = doc_with_clip();
    let (seq, v1) = (b.seq, b.v1);
    let e = assert_err!(
        &mut doc,
        Command::Batch {
            commands: vec![
                add_marker(seq, 0),
                Command::Batch {
                    commands: vec![add_m(v1, b.m, 0, S)]
                }, // overlaps X
                Command::RemoveClips {
                    clip_ids: vec![999],
                    include_linked: true
                },
            ]
        },
        Error::Overlap { .. }
    );
    assert_eq!(e, Error::Overlap { track_id: v1 });
    let _ = x;
}

#[test]
fn c_batch_1_empty_batch_is_ok_noop() {
    let (mut doc, ..) = doc_with_clip();
    let before = doc.project().clone();
    let out = ok(&mut doc, Command::Batch { commands: vec![] });
    assert!(out.created.is_empty());
    assert_eq!(doc.project(), &before);
    assert_eq!(doc.undo_depth(), 0);
}

// ============================================================================ G9

/// Every kind of entity, so the JSON round-trip covers all serde shapes.
fn rich_project() -> Project {
    let Base {
        mut fx,
        m,
        still,
        seq,
        v1,
        v2,
        a1,
        ..
    } = base();
    let inner = fx.seq("Inner");
    let iv = fx.track(inner, TrackKind::Video);
    fx.clip(iv, m, 0, 2 * S, 0);
    let p = fx.clip(v1, m, 0, 2 * S, 0);
    let q = fx.clip(v1, m, 2 * S, 2 * S, 10 * S);
    fx.c(q).speed = 1.5;
    fx.c(q).reverse = true;
    fx.c(q).blend = BlendMode::Multiply;
    fx.c(q).audio_filter = "volume=2".into();
    fx.c(q).enabled = false;
    fx.transition(v1, Some(p), Some(q), S);
    fx.t(v1).transitions[0]
        .params
        .insert("softness".into(), 0.25);
    fx.clip_src(v2, seq_src(inner), 0, 5 * S, S);
    let mut t = text_spec("Title \"quoted\" \u{e9}");
    t.align = TextAlign::Right;
    t.italic = true;
    fx.clip_src(v2, ClipSource::Text { text: t }, 5 * S, S, 0);
    fx.clip_src(
        v2,
        ClipSource::Solid {
            color: [0.1, 0.2, 0.3, 0.5],
        },
        6 * S,
        S,
        0,
    );
    fx.clip_src(v2, ClipSource::Adjustment, 7 * S, S, 0);
    fx.clip(v2, still, 8 * S, 500 * S, 0);
    let pa = fx.clip(a1, m, 0, 2 * S, 0);
    fx.link(&[p, pa]);
    let g = fx.id();
    fx.c(q).group = Some(g);
    fx.c(p).params.insert(
        "opacity".into(),
        param(
            0.5,
            vec![kf(-S, 0.0, Easing::EaseInOut), kf(S, 1.0, Easing::EaseOut)],
        ),
    );
    fx.effect(
        p,
        "blur",
        &[("amount", param(3.0, vec![kf(0, 1.0, Easing::Hold)]))],
    );
    fx.marker(seq, 0, 0);
    fx.marker(seq, 3 * S, S);
    fx.t(v2).muted = true;
    fx.t(v2).hidden = true;
    fx.t(a1).solo = true;
    fx.lock(a1);
    fx.p.effects.push(EffectDef {
        name: "glow".into(),
        kind: EffectKind::Transition,
        wgsl: "@fragment fn main() {}".into(),
        params: vec![ParamDef {
            name: "k".into(),
            default: 0.5,
            min: -1.0,
            max: 1.0,
        }],
    });
    fx.p
}

#[test]
fn g9_project_json_round_trip() {
    let p = rich_project();
    assert!(p.validate().is_ok());
    let json = p.to_json();
    assert_eq!(Project::from_json(&json), Ok(p.clone()));
    assert_eq!(
        Project::from_json(&Project::new("x").to_json()),
        Ok(Project::new("x"))
    );
}

#[test]
fn g9_from_json_rejects_garbage() {
    assert!(Project::from_json("").is_err());
    assert!(Project::from_json("{\"name\": 3}").is_err());
    assert!(Project::from_json("[]").is_err());
}

/// Passes against the stubs (serde derives only).
#[test]
fn g9_command_serde_round_trip() {
    let cmds = vec![
        add_m(1, 2, 3, 4),
        AddB {
            mode: PlaceMode::Insert,
            linked_track_id: Some(9),
            name: Some("n".into()),
            source_in: 5,
            ..addb(1, text_src("t"), 0, S)
        }
        .cmd(),
        Command::TrimClip {
            clip_id: 1,
            edge: Edge::Start,
            to: 5,
            mode: TrimMode::Roll,
            include_linked: false,
        },
        Command::SetKeyframe {
            target: pte(1, 2, "amount"),
            time: -3,
            value: 0.5,
            easing: Easing::EaseInOut,
        },
        Command::Batch {
            commands: vec![
                Command::Batch { commands: vec![] },
                Command::RemoveClips {
                    clip_ids: vec![1, 2],
                    include_linked: false,
                },
            ],
        },
        Command::SetClip {
            clip_id: 1,
            name: None,
            enabled: Some(true),
            reverse: None,
            blend: Some(BlendMode::Difference),
            audio_filter: None,
            label: None,
        },
    ];
    for c in cmds {
        let json = serde_json::to_string(&c).unwrap();
        let back: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c, "{json}");
    }
}

/// Passes against the stubs (serde defaults only).
#[test]
fn g9_command_defaults() {
    let c: Command = serde_json::from_str(r#"{"op":"remove_clips","clip_ids":[1]}"#).unwrap();
    assert_eq!(
        c,
        Command::RemoveClips {
            clip_ids: vec![1],
            include_linked: true
        }
    );
    let c: Command =
        serde_json::from_str(r#"{"op":"trim_clip","clip_id":1,"edge":"start","to":5}"#).unwrap();
    assert_eq!(
        c,
        Command::TrimClip {
            clip_id: 1,
            edge: Edge::Start,
            to: 5,
            mode: TrimMode::Normal,
            include_linked: true
        }
    );
    let c: Command = serde_json::from_str(
        r#"{"op":"add_clip","track_id":1,"source":{"type":"media","media_id":2},"start":0,"duration":10}"#,
    )
    .unwrap();
    assert_eq!(c, add_m(1, 2, 0, 10));
    let c: Command = serde_json::from_str(r#"{"op":"move_clip","clip_id":1,"start":0}"#).unwrap();
    assert_eq!(
        c,
        Command::MoveClip {
            clip_id: 1,
            start: 0,
            track_id: None,
            mode: PlaceMode::Place,
            include_linked: true
        }
    );
    let c: Command =
        serde_json::from_str(r#"{"op":"set_clip_speed","clip_id":1,"speed":2.0}"#).unwrap();
    assert_eq!(
        c,
        Command::SetClipSpeed {
            clip_id: 1,
            speed: 2.0,
            ripple: false,
            include_linked: true
        }
    );
}
