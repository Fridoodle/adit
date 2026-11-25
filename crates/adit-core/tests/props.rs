//! Property tests: random command sequences (G1, G2, G3, G5, G9) and targeted properties for
//! C-EXT-1 and R4 / C-SPLIT-1.
mod common;
use common::*;
use proptest::prelude::*;
use std::collections::BTreeMap;

const H: Time = S / 2;

#[derive(Debug, Clone)]
struct Op {
    kind: u8,
    a: u32,
    b: u32,
    t1: i64,
    t2: i64,
    flag: bool,
}

fn op() -> impl Strategy<Value = Op> {
    (
        0u8..24,
        any::<u32>(),
        any::<u32>(),
        -4i64..40,
        -4i64..40,
        any::<bool>(),
    )
        .prop_map(|(kind, a, b, t1, t2, flag)| Op {
            kind,
            a,
            b,
            t1,
            t2,
            flag,
        })
}

struct World {
    seq: Id,
    sources: Vec<ClipSource>,
    locked: Id,
}

/// V1: X [0,4s) linked to XA on A1, Y [4s,6s), transition X|Y, effect on X; V2: text [1s,3s),
/// still [5s,8s); A2 (locked): LA [2s,5s).
fn world() -> (Project, World) {
    let Base {
        mut fx,
        m,
        still,
        aud,
        vid,
        seq,
        v1,
        v2,
        a1,
        a2,
    } = base();
    let x = fx.clip(v1, m, 0, 4 * S, 0);
    let y = fx.clip(v1, m, 4 * S, 2 * S, 2 * S);
    let xa = fx.clip(a1, m, 0, 4 * S, 0);
    fx.link(&[x, xa]);
    fx.transition(v1, Some(x), Some(y), S);
    fx.effect(
        x,
        "blur",
        &[(
            "amount",
            param(
                1.0,
                vec![kf(0, 0.0, Easing::Linear), kf(2 * S, 4.0, Easing::Linear)],
            ),
        )],
    );
    fx.text(v2, S, 2 * S);
    fx.clip(v2, still, 5 * S, 3 * S, 0);
    fx.clip(a2, m, 2 * S, 3 * S, 0);
    fx.lock(a2);
    fx.marker(seq, S, 0);
    let sources = vec![
        media_src(m),
        media_src(still),
        media_src(aud),
        media_src(vid),
        text_src("p"),
        solid_src(),
    ];
    let p = fx.p;
    assert!(p.validate().is_ok());
    (
        p,
        World {
            seq,
            sources,
            locked: a2,
        },
    )
}

fn clip_ids(p: &Project) -> Vec<Id> {
    all_clip_ids(p)
}

fn track_ids(p: &Project) -> Vec<Id> {
    p.sequences
        .iter()
        .flat_map(|s| &s.tracks)
        .map(|t| t.id)
        .collect()
}

fn effect_ids(p: &Project) -> Vec<(Id, Id)> {
    p.sequences
        .iter()
        .flat_map(|s| &s.tracks)
        .flat_map(|t| &t.clips)
        .flat_map(|c| c.effects.iter().map(move |e| (c.id, e.id)))
        .collect()
}

fn transition_ids(p: &Project) -> Vec<Id> {
    p.sequences
        .iter()
        .flat_map(|s| &s.tracks)
        .flat_map(|t| &t.transitions)
        .map(|t| t.id)
        .collect()
}

/// Mostly existing ids; sometimes a missing one so NotFound paths are exercised too.
fn pick(v: &[Id], i: u32) -> Id {
    if v.is_empty() || i.is_multiple_of(17) {
        999_999
    } else {
        v[i as usize % v.len()]
    }
}

fn to_cmd(p: &Project, w: &World, o: &Op) -> Command {
    let clips = clip_ids(p);
    let tracks = track_ids(p);
    let c = pick(&clips, o.a);
    let c2 = pick(&clips, o.b);
    let t = pick(&tracks, o.b);
    let (t1, t2) = (o.t1 * H, o.t2 * H);
    let tracks_opt = if o.flag { None } else { Some(vec![t]) };
    let mode = [PlaceMode::Place, PlaceMode::Overwrite, PlaceMode::Insert][o.a as usize % 3];
    match o.kind {
        0 | 1 => AddB {
            linked_track_id: if o.a.is_multiple_of(4) {
                Some(pick(&tracks, o.a / 4))
            } else {
                None
            },
            source_in: (o.b % 5) as Time * H,
            mode,
            ..addb(t, w.sources[o.a as usize % w.sources.len()].clone(), t1, t2)
        }
        .cmd(),
        2 => Command::RemoveClips {
            clip_ids: vec![c],
            include_linked: o.flag,
        },
        3 => Command::RippleDeleteClips {
            clip_ids: vec![c, c2],
            include_linked: o.flag,
        },
        4 => Command::LiftRange {
            sequence_id: w.seq,
            start: t1,
            end: t1 + t2,
            track_ids: tracks_opt,
        },
        5 => Command::ExtractRange {
            sequence_id: w.seq,
            start: t1,
            end: t1 + t2,
            track_ids: tracks_opt,
        },
        6 => Command::CloseGap {
            track_id: t,
            time: t1,
        },
        7 => Command::SplitAt {
            sequence_id: w.seq,
            time: t1 + 1,
            track_ids: tracks_opt,
        },
        8 => Command::SplitClip {
            clip_id: c,
            time: t1,
            include_linked: o.flag,
        },
        9 => Command::MoveClip {
            clip_id: c,
            start: t1,
            track_id: if o.b.is_multiple_of(2) { Some(t) } else { None },
            mode,
            include_linked: o.flag,
        },
        10 | 11 => Command::TrimClip {
            clip_id: c,
            edge: if o.flag { Edge::Start } else { Edge::End },
            to: t1,
            mode: [TrimMode::Normal, TrimMode::Ripple, TrimMode::Roll][o.b as usize % 3],
            include_linked: o.a.is_multiple_of(2),
        },
        12 => Command::SlipClip {
            clip_id: c,
            delta: t1 - t2,
            include_linked: o.flag,
        },
        13 => Command::SlideClip {
            clip_id: c,
            delta: (t1 - t2) / 2,
        },
        14 => Command::SetClipSpeed {
            clip_id: c,
            speed: [0.5, 1.0, 2.0, 1.5, 0.005, 3.0][o.b as usize % 6],
            ripple: o.flag,
            include_linked: o.a.is_multiple_of(2),
        },
        15 => Command::SetKeyframe {
            target: pt(c, ["opacity", "scale_x", "bogus"][o.b as usize % 3]),
            time: t1,
            value: o.t2 as f64 / 4.0,
            easing: [
                Easing::Linear,
                Easing::Hold,
                Easing::EaseIn,
                Easing::EaseOut,
                Easing::EaseInOut,
            ][o.b as usize % 5],
        },
        16 => {
            let effs = effect_ids(p);
            if o.flag && !effs.is_empty() {
                let (cl, e) = effs[o.b as usize % effs.len()];
                Command::SetParam {
                    target: pte(cl, e, "amount"),
                    value: o.t1 as f64,
                }
            } else {
                let mut params = BTreeMap::new();
                params.insert("amount".to_string(), o.t1 as f64);
                Command::AddEffect {
                    clip_id: c,
                    effect: "glow".into(),
                    index: None,
                    params,
                }
            }
        }
        17 => Command::AddTransition {
            track_id: t,
            left: Some(c),
            right: if o.flag { Some(c2) } else { None },
            effect: "dissolve".into(),
            duration: (o.t2.abs() + 1) * H / 2,
            params: BTreeMap::new(),
        },
        18 => Command::AddMarker {
            sequence_id: w.seq,
            time: t1,
            duration: 0,
            name: String::new(),
            note: String::new(),
        },
        19 => Command::SetClip {
            clip_id: c,
            name: None,
            enabled: None,
            reverse: Some(o.flag),
            blend: None,
            audio_filter: None,
            label: None,
        },
        20 => Command::LinkClips {
            clip_ids: vec![c, c2],
        },
        21 => Command::Batch {
            commands: vec![
                Command::AddMarker {
                    sequence_id: w.seq,
                    time: t2,
                    duration: 0,
                    name: "b".into(),
                    note: String::new(),
                },
                Command::RemoveClips {
                    clip_ids: vec![c],
                    include_linked: o.flag,
                },
            ],
        },
        22 => {
            let trs = transition_ids(p);
            if o.flag {
                Command::RemoveTransition {
                    transition_id: pick(&trs, o.a),
                }
            } else {
                Command::SetTransition {
                    transition_id: pick(&trs, o.a),
                    effect: None,
                    duration: Some(t2),
                    params: None,
                }
            }
        }
        _ => {
            if o.flag {
                Command::MoveTrack {
                    track_id: t,
                    index: o.a as usize % 5,
                }
            } else {
                // never touches `locked`, so the locked track stays locked
                Command::SetTrack {
                    track_id: t,
                    name: None,
                    locked: None,
                    muted: Some(o.flag),
                    solo: None,
                    hidden: Some(true),
                }
            }
        }
    }
}

fn locked_clips(p: &Project, track: Id) -> Vec<Clip> {
    trk(p, track).clips.clone()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// G1, G2, G3, G5 (locked clips never change), G9 (project and Command JSON).
    #[test]
    fn prop_random_commands(ops in prop::collection::vec(op(), 1..40)) {
        let (p0, w) = world();
        let locked0 = locked_clips(&p0, w.locked);
        let mut doc = Document::new(p0.clone());
        let mut states = vec![p0.clone()];
        for o in &ops {
            let cmd = to_cmd(doc.project(), &w, o);
            let json = serde_json::to_string(&cmd).unwrap();
            prop_assert_eq!(&serde_json::from_str::<Command>(&json).unwrap(), &cmd);
            let before = doc.project().clone();
            let (u, r) = (doc.undo_depth(), doc.redo_depth());
            match doc.apply(cmd.clone()) {
                Ok(_) => {
                    let v = doc.project().validate();
                    prop_assert!(v.is_ok(), "G2: {:?} after {:?}", v, cmd);
                    if doc.project() != &before {
                        prop_assert_eq!(doc.undo_depth(), u + 1, "G2 push after {:?}", cmd);
                        prop_assert_eq!(doc.redo_depth(), 0);
                        states.push(doc.project().clone());
                    } else {
                        prop_assert_eq!((doc.undo_depth(), doc.redo_depth()), (u, r), "G2 no-op {:?}", cmd);
                    }
                    prop_assert!(doc.project().next_id >= before.next_id, "G4");
                }
                Err(e) => {
                    prop_assert_eq!(doc.project(), &before, "G1 after {:?} -> {:?}", cmd, e);
                    prop_assert_eq!((doc.undo_depth(), doc.redo_depth()), (u, r), "G1 depths");
                }
            }
            prop_assert_eq!(&locked_clips(doc.project(), w.locked), &locked0, "G5 after {:?}", cmd);
        }
        let last = doc.project().clone();
        prop_assert_eq!(Project::from_json(&last.to_json()), Ok(last.clone()), "G9");
        let n = doc.undo_depth();
        prop_assert_eq!(n, states.len() - 1);
        for k in (0..n).rev() {
            prop_assert!(doc.undo());
            prop_assert_eq!(doc.project(), &states[k], "G3 undo to state {}", k);
        }
        prop_assert!(!doc.undo());
        prop_assert_eq!(doc.project(), &p0);
        for (k, st) in states.iter().enumerate().skip(1) {
            prop_assert!(doc.redo());
            prop_assert_eq!(doc.project(), st, "G3 redo to state {}", k);
        }
        prop_assert!(!doc.redo());
        prop_assert_eq!(doc.project(), &last);
    }

    /// C-EXT-1 with track_ids None and no locked tracks.
    #[test]
    fn prop_extract_duration(
        placements in prop::collection::vec((0usize..3, 0i64..24, 1i64..10), 0..10),
        start in 0i64..28,
        len in 1i64..12,
    ) {
        let Base { mut fx, m, seq, v1, v2, a1, .. } = base();
        let tracks = [v1, v2, a1];
        for (ti, st, du) in placements {
            let (st, du) = (st * H, du * H);
            let free = fx.t(tracks[ti]).clips.iter().all(|c| st + du <= c.start || st >= c.end());
            if free {
                fx.clip(tracks[ti], m, st, du, 0);
            }
        }
        let (s, e) = (start * H, (start + len) * H);
        let crosses_start = fx.p.sequences[0].tracks.iter().flat_map(|t| &t.clips).any(|c| c.start < s && s < c.end());
        let mut doc = fx.doc();
        let old = sq(doc.project(), seq).duration();
        ok(&mut doc, Command::ExtractRange { sequence_id: seq, start: s, end: e, track_ids: None });
        let new = sq(doc.project(), seq).duration();
        if old > e {
            prop_assert_eq!(new, old - (e - s));
        } else if old <= s {
            prop_assert_eq!(new, old);
        } else {
            // s < old <= e: everything after s is gone (spec formula, see report)
            prop_assert!(new <= s);
            if crosses_start {
                prop_assert_eq!(new, s);
            }
        }
    }

    /// R4 / C-SPLIT-1: source_time (within 1 flick) and every parameter evaluate the same at every
    /// sampled sequence time before and after SplitAt.
    #[test]
    fn prop_split_preserves_time_and_params(
        start in 0i64..20,
        dur in 2i64..30,
        src in 0i64..20,
        speed_i in 0usize..6,
        reverse in any::<bool>(),
        op_kfs in prop::collection::btree_map(-10i64..40, (-50i32..50, 0usize..5), 0..5),
        fx_kfs in prop::collection::btree_map(-10i64..40, (-50i32..50, 0usize..5), 0..5),
        frac in 0.0f64..1.0,
        flick_off in -3i64..4,
    ) {
        let easings = [Easing::Linear, Easing::Hold, Easing::EaseIn, Easing::EaseOut, Easing::EaseInOut];
        let to_kfs = |m: &BTreeMap<i64, (i32, usize)>| -> Vec<Keyframe> {
            m.iter().map(|(t, (v, e))| kf(*t * H / 3, *v as f64 / 10.0, easings[*e])).collect()
        };
        let Base { mut fx, still, seq, v1, .. } = base();
        let (start, dur, src) = (start * H, dur * H / 2 + 7, src * H);
        let x = fx.clip(v1, still, start, dur, src);
        fx.c(x).speed = [1.0, 2.0, 0.5, 1.5, 3.0, 0.75][speed_i];
        fx.c(x).reverse = reverse;
        fx.c(x).params.insert("opacity".into(), param(0.3, to_kfs(&op_kfs)));
        fx.effect(x, "blur", &[("amount", param(2.0, to_kfs(&fx_kfs)))]);
        let before = fx.p.clone();
        let bx = clip(&before, x).clone();
        let time = start - H + (frac * (dur + 2 * H) as f64).round() as Time + flick_off;
        let mut doc = fx.doc();
        if time < 0 {
            return Ok(());
        }
        let out = ok(&mut doc, Command::SplitAt { sequence_id: seq, time, track_ids: None });
        let inside = start < time && time < start + dur;
        prop_assert_eq!(out.created.len(), if inside { 1 } else { 0 });
        let after = doc.project();
        let mut samples: Vec<Time> = (0..40).map(|k| start + dur * k / 40).collect();
        samples.extend([time - 1, time, time + 1, start + dur - 1]);
        for at in samples.into_iter().filter(|&t| start <= t && t < start + dur) {
            let c = trk(after, v1).clips.iter().find(|c| c.start <= at && at < c.end());
            prop_assert!(c.is_some(), "no clip covers {} after split", at);
            let c = c.unwrap();
            let d = c.source_time(at) - bx.source_time(at);
            prop_assert!(d.abs() <= 1, "source_time at {}: {} vs {}", at, c.source_time(at), bx.source_time(at));
            let (pb, pa) = (bx.param("opacity", at - bx.start).unwrap(), c.param("opacity", at - c.start).unwrap());
            prop_assert!(approx(pb, pa), "opacity at {}: {} vs {}", at, pb, pa);
            let (eb, ea) = (
                bx.effects[0].params["amount"].eval(at - bx.start),
                c.effects[0].params["amount"].eval(at - c.start),
            );
            prop_assert!(approx(eb, ea), "effect param at {}: {} vs {}", at, eb, ea);
        }
    }
}
