//! Parameters, keyframes, effects, transitions: C-PARAM-*, C-EFF-*, C-TRANS-*, G8.
#[macro_use]
mod common;
use common::*;
use std::collections::BTreeMap;

/// V1: X [0,4s) with effect "blur" (no params), Y [4s,6s); V2 locked: L [0,1s).
fn fx1() -> (Base, Id, Id, Id, Id) {
    let mut b = base();
    let (m, v1, v2) = (b.m, b.v1, b.v2);
    let x = b.fx.clip(v1, m, 0, 4 * S, 0);
    let e = b.fx.effect(x, "blur", &[]);
    let y = b.fx.clip(v1, m, 4 * S, 2 * S, 0);
    let l = b.fx.clip(v2, m, 0, S, 0);
    b.fx.lock(v2);
    (b, x, e, y, l)
}

// ============================================================================ C-PARAM-1/2

#[test]
fn c_param_2_set_param_replaces_keyframes() {
    let (mut b, x, ..) = fx1();
    b.fx.c(x).params.insert(
        "opacity".into(),
        param(0.2, vec![kf(0, 0.0, Easing::Linear)]),
    );
    let mut doc = b.fx.doc();
    ok(
        &mut doc,
        Command::SetParam {
            target: pt(x, "opacity"),
            value: 0.7,
        },
    );
    assert_eq!(clip(doc.project(), x).params["opacity"], param(0.7, vec![]));
    ok(
        &mut doc,
        Command::SetParam {
            target: pt(x, "scale_x"),
            value: 2.0,
        },
    );
    assert_eq!(clip(doc.project(), x).params["scale_x"], param(2.0, vec![]));
    assert_eq!(clip(doc.project(), x).param("scale_x", S), Some(2.0));
}

#[test]
fn c_param_2_effect_param() {
    let (b, x, e, ..) = fx1();
    let mut doc = b.fx.doc();
    ok(
        &mut doc,
        Command::SetParam {
            target: pte(x, e, "radius"),
            value: 5.0,
        },
    );
    assert_eq!(
        clip(doc.project(), x).effects[0].params["radius"],
        param(5.0, vec![])
    );
    assert!(
        clip(doc.project(), x).params.is_empty(),
        "clip params untouched"
    );
}

#[test]
fn c_param_1_errors() {
    let (b, x, e, y, l) = fx1();
    let mut doc = b.fx.doc();
    assert_err!(
        &mut doc,
        Command::SetParam {
            target: pt(x, "bogus"),
            value: 1.0
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::SetParam {
            target: pt(x, ""),
            value: 1.0
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::SetParam {
            target: pte(x, e, ""),
            value: 1.0
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::SetParam {
            target: pte(x, 999, "r"),
            value: 1.0
        },
        Error::NotFound { id: 999, .. }
    );
    // the effect exists, but on another clip
    assert_err!(&mut doc, Command::SetParam { target: pte(y, e, "r"), value: 1.0 }, Error::NotFound { id, .. } if id == e);
    assert_err!(
        &mut doc,
        Command::SetParam {
            target: pt(999, "opacity"),
            value: 1.0
        },
        Error::NotFound { id: 999, .. }
    );
    assert_err!(
        &mut doc,
        Command::SetParam {
            target: pt(x, "opacity"),
            value: f64::NAN
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::SetParam {
            target: pt(x, "opacity"),
            value: f64::INFINITY
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::SetParam {
            target: pt(l, "opacity"),
            value: 0.5
        },
        Error::TrackLocked { .. }
    );
}

// ============================================================================ C-PARAM-3

fn set_kf(target: ParamTarget, time: Time, value: f64, easing: Easing) -> Command {
    Command::SetKeyframe {
        target,
        time,
        value,
        easing,
    }
}

#[test]
fn c_param_3_creates_param_with_default_value() {
    let (b, x, e, ..) = fx1();
    let mut doc = b.fx.doc();
    ok(&mut doc, set_kf(pt(x, "opacity"), S, 0.5, Easing::Linear));
    ok(&mut doc, set_kf(pt(x, "scale_y"), 0, 3.0, Easing::Hold));
    ok(&mut doc, set_kf(pt(x, "rotation"), 0, 3.0, Easing::Hold));
    ok(
        &mut doc,
        set_kf(pte(x, e, "amount"), 0, 9.0, Easing::EaseIn),
    );
    let c = clip(doc.project(), x);
    assert_eq!(
        c.params["opacity"],
        param(1.0, vec![kf(S, 0.5, Easing::Linear)])
    );
    assert_eq!(
        c.params["scale_y"],
        param(1.0, vec![kf(0, 3.0, Easing::Hold)])
    );
    assert_eq!(
        c.params["rotation"],
        param(0.0, vec![kf(0, 3.0, Easing::Hold)])
    );
    assert_eq!(
        c.effects[0].params["amount"],
        param(0.0, vec![kf(0, 9.0, Easing::EaseIn)])
    );
}

#[test]
fn c_param_3_keeps_existing_value_and_orders_keyframes() {
    let (mut b, x, ..) = fx1();
    b.fx.c(x)
        .params
        .insert("opacity".into(), param(0.25, vec![]));
    let mut doc = b.fx.doc();
    ok(
        &mut doc,
        set_kf(pt(x, "opacity"), 2 * S, 0.2, Easing::Linear),
    );
    ok(&mut doc, set_kf(pt(x, "opacity"), S, 0.1, Easing::Linear));
    ok(
        &mut doc,
        set_kf(pt(x, "opacity"), 3 * S, 0.3, Easing::Linear),
    );
    ok(
        &mut doc,
        set_kf(pt(x, "opacity"), 2 * S, 0.9, Easing::EaseOut),
    ); // replaces
    ok(&mut doc, set_kf(pt(x, "opacity"), -S, 0.0, Easing::Linear)); // before the clip
    ok(
        &mut doc,
        set_kf(pt(x, "opacity"), 10 * S, 1.0, Easing::Linear),
    ); // beyond the clip
    assert_eq!(
        clip(doc.project(), x).params["opacity"],
        param(
            0.25,
            vec![
                kf(-S, 0.0, Easing::Linear),
                kf(S, 0.1, Easing::Linear),
                kf(2 * S, 0.9, Easing::EaseOut),
                kf(3 * S, 0.3, Easing::Linear),
                kf(10 * S, 1.0, Easing::Linear),
            ]
        )
    );
}

#[test]
fn c_param_3_errors() {
    let (b, x, e, _, l) = fx1();
    let mut doc = b.fx.doc();
    assert_err!(
        &mut doc,
        set_kf(pt(x, "bogus"), 0, 1.0, Easing::Linear),
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        set_kf(pt(x, "opacity"), 0, f64::NAN, Easing::Linear),
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        set_kf(pte(x, e + 1000, "a"), 0, 1.0, Easing::Linear),
        Error::NotFound { .. }
    );
    assert_err!(
        &mut doc,
        set_kf(pt(l, "opacity"), 0, 1.0, Easing::Linear),
        Error::TrackLocked { .. }
    );
}

// ============================================================================ C-PARAM-4

#[test]
fn c_param_4_remove_and_clear_keyframes() {
    let (mut b, x, e, ..) = fx1();
    b.fx.c(x).params.insert(
        "opacity".into(),
        param(
            0.4,
            vec![
                kf(0, 0.0, Easing::Linear),
                kf(S, 1.0, Easing::Linear),
                kf(2 * S, 0.5, Easing::Linear),
            ],
        ),
    );
    b.fx.c(x).effects[0].params.insert(
        "amount".into(),
        param(3.0, vec![kf(0, 1.0, Easing::Linear)]),
    );
    let mut doc = b.fx.doc();
    assert_err!(
        &mut doc,
        Command::RemoveKeyframe {
            target: pt(x, "opacity"),
            time: S + 1
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::RemoveKeyframe {
            target: pt(x, "rotation"),
            time: 0
        },
        Error::InvalidArgument(_)
    );
    ok(
        &mut doc,
        Command::RemoveKeyframe {
            target: pt(x, "opacity"),
            time: S,
        },
    );
    assert_eq!(
        clip(doc.project(), x).params["opacity"],
        param(
            0.4,
            vec![kf(0, 0.0, Easing::Linear), kf(2 * S, 0.5, Easing::Linear)]
        )
    );
    ok(
        &mut doc,
        Command::ClearKeyframes {
            target: pt(x, "opacity"),
        },
    );
    assert_eq!(clip(doc.project(), x).params["opacity"], param(0.4, vec![]));
    ok(
        &mut doc,
        Command::ClearKeyframes {
            target: pte(x, e, "amount"),
        },
    );
    assert_eq!(
        clip(doc.project(), x).effects[0].params["amount"],
        param(3.0, vec![])
    );
}

// ============================================================================ C-EFF-1

fn add_fx(clip_id: Id, effect: &str, index: Option<usize>, params: &[(&str, f64)]) -> Command {
    Command::AddEffect {
        clip_id,
        effect: effect.into(),
        index,
        params: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
    }
}

#[test]
fn c_eff_1_add_effect() {
    let (b, x, e, ..) = fx1();
    let mut doc = b.fx.doc();
    let n = doc.project().next_id;
    let out = ok(&mut doc, add_fx(x, "glow", None, &[("k", 0.5), ("r", 2.0)]));
    assert_eq!(out.created, vec![n]);
    let out2 = ok(&mut doc, add_fx(x, "first", Some(0), &[]));
    assert_eq!(out2.created, vec![n + 1]);
    ok(&mut doc, add_fx(x, "end", Some(3), &[])); // index == len
    let c = clip(doc.project(), x);
    let order: Vec<_> = c
        .effects
        .iter()
        .map(|i| (i.id, i.effect.as_str()))
        .collect();
    assert_eq!(
        order,
        vec![(n + 1, "first"), (e, "blur"), (n, "glow"), (n + 2, "end")]
    );
    let glow = &c.effects[2];
    assert!(glow.enabled);
    let mut want = BTreeMap::new();
    want.insert("k".to_string(), param(0.5, vec![]));
    want.insert("r".to_string(), param(2.0, vec![]));
    assert_eq!(glow.params, want);
}

#[test]
fn c_eff_1_errors() {
    let (b, x, _, _, l) = fx1();
    let mut doc = b.fx.doc();
    assert_err!(
        &mut doc,
        add_fx(x, "late", Some(2), &[]),
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        add_fx(x, "", None, &[]),
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        add_fx(x, "nan", None, &[("k", f64::NAN)]),
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        add_fx(999, "a", None, &[]),
        Error::NotFound { id: 999, .. }
    );
    assert_err!(
        &mut doc,
        add_fx(l, "a", None, &[]),
        Error::TrackLocked { .. }
    );
}

// ============================================================================ C-EFF-2

#[test]
fn c_eff_2_remove_move_enable() {
    let (mut b, x, e, ..) = fx1();
    let e2 = b.fx.effect(x, "two", &[]);
    let e3 = b.fx.effect(x, "three", &[]);
    let mut doc = b.fx.doc();
    let ids = |d: &Document| {
        clip(d.project(), x)
            .effects
            .iter()
            .map(|i| i.id)
            .collect::<Vec<_>>()
    };
    ok(
        &mut doc,
        Command::MoveEffect {
            clip_id: x,
            effect_id: e,
            index: 2,
        },
    );
    assert_eq!(ids(&doc), vec![e2, e3, e]);
    ok(
        &mut doc,
        Command::MoveEffect {
            clip_id: x,
            effect_id: e3,
            index: 0,
        },
    );
    assert_eq!(ids(&doc), vec![e3, e2, e]);
    assert_err!(
        &mut doc,
        Command::MoveEffect {
            clip_id: x,
            effect_id: e3,
            index: 3
        },
        Error::InvalidArgument(_)
    );
    ok(
        &mut doc,
        Command::SetEffectEnabled {
            clip_id: x,
            effect_id: e2,
            enabled: false,
        },
    );
    let c = clip(doc.project(), x);
    assert_eq!(
        c.effects.iter().map(|i| i.enabled).collect::<Vec<_>>(),
        vec![true, false, true]
    );
    ok(
        &mut doc,
        Command::RemoveEffect {
            clip_id: x,
            effect_id: e3,
        },
    );
    assert_eq!(ids(&doc), vec![e2, e]);
}

#[test]
fn c_eff_2_unknown_effect_on_clip() {
    let (b, x, e, y, _) = fx1();
    let mut doc = b.fx.doc();
    assert_err!(&mut doc, Command::RemoveEffect { clip_id: y, effect_id: e }, Error::NotFound { id, .. } if id == e);
    assert_err!(&mut doc, Command::MoveEffect { clip_id: y, effect_id: e, index: 0 }, Error::NotFound { id, .. } if id == e);
    assert_err!(
        &mut doc,
        Command::SetEffectEnabled { clip_id: y, effect_id: e, enabled: false },
        Error::NotFound { id, .. } if id == e
    );
    assert_err!(
        &mut doc,
        Command::RemoveEffect {
            clip_id: x,
            effect_id: 999
        },
        Error::NotFound { id: 999, .. }
    );
}

// ============================================================================ C-TRANS-1/2

fn add_tr(track_id: Id, left: Option<Id>, right: Option<Id>, duration: Time) -> Command {
    Command::AddTransition {
        track_id,
        left,
        right,
        effect: "dissolve".into(),
        duration,
        params: BTreeMap::new(),
    }
}

/// V1: P [0,2s), Q [2s,4s), R [5s,6s); V2: O [0,2s).
fn tr_fx() -> (Base, [Id; 4]) {
    let mut b = base();
    let (m, v1, v2) = (b.m, b.v1, b.v2);
    let p = b.fx.clip(v1, m, 0, 2 * S, 0);
    let q = b.fx.clip(v1, m, 2 * S, 2 * S, 0);
    let r = b.fx.clip(v1, m, 5 * S, S, 0);
    let o = b.fx.clip(v2, m, 0, 2 * S, 0);
    (b, [p, q, r, o])
}

#[test]
fn c_trans_1_add_two_sided_and_one_sided() {
    let (b, [p, q, r, _]) = tr_fx();
    let v1 = b.v1;
    let mut doc = b.fx.doc();
    let n = doc.project().next_id;
    let mut params = BTreeMap::new();
    params.insert("softness".to_string(), 0.5);
    let out = ok(
        &mut doc,
        Command::AddTransition {
            track_id: v1,
            left: Some(p),
            right: Some(q),
            effect: "wipe".into(),
            duration: S,
            params: params.clone(),
        },
    );
    assert_eq!(out.created, vec![n]);
    let out = ok(&mut doc, add_tr(v1, Some(r), None, S));
    assert_eq!(out.created, vec![n + 1]);
    let ts = &trk(doc.project(), v1).transitions;
    assert_eq!(ts.len(), 2);
    let t = ts.iter().find(|t| t.id == n).unwrap();
    assert_eq!(
        t,
        &Transition {
            id: n,
            left: Some(p),
            right: Some(q),
            effect: "wipe".into(),
            duration: S,
            params
        }
    );
}

#[test]
fn c_trans_1_replaces_transition_on_same_edge() {
    let (b, [p, q, ..]) = tr_fx();
    let v1 = b.v1;
    let mut doc = b.fx.doc();
    let t1 = ok(&mut doc, add_tr(v1, Some(p), None, S)).created[0];
    let t2 = ok(&mut doc, add_tr(v1, None, Some(p), S)).created[0]; // P's start: other edge, kept
    let t3 = ok(&mut doc, add_tr(v1, Some(p), Some(q), S)).created[0]; // replaces t1
    let mut ids: Vec<_> = trk(doc.project(), v1)
        .transitions
        .iter()
        .map(|t| t.id)
        .collect();
    ids.sort();
    assert_eq!(ids, vec![t2, t3]);
    assert!(!ids.contains(&t1));
    let t4 = ok(&mut doc, add_tr(v1, None, Some(q), S / 2)).created[0]; // replaces t3 (Q's start)
    let mut ids: Vec<_> = trk(doc.project(), v1)
        .transitions
        .iter()
        .map(|t| t.id)
        .collect();
    ids.sort();
    assert_eq!(ids, vec![t2, t4]);
}

#[test]
fn c_trans_1_i5_errors() {
    let (mut b, [p, q, r, o]) = tr_fx();
    let (v1, v2) = (b.v1, b.v2);
    let mut doc = b.fx.clone_doc();
    for cmd in [
        add_tr(v1, None, None, S),
        add_tr(v1, Some(q), Some(r), S),         // not adjacent
        add_tr(v1, Some(p), Some(q), 4 * S + 2), // duration/2 > left
        add_tr(v1, Some(r), None, S + 1),        // longer than R
        add_tr(v1, Some(o), None, S),            // O is on V2
        add_tr(v1, Some(p), Some(q), 0),
        add_tr(v1, Some(p), Some(q), -S),
    ] {
        assert_err!(&mut doc, cmd, Error::InvalidArgument(_));
    }
    assert_err!(
        &mut doc,
        add_tr(999, Some(p), None, S),
        Error::NotFound { id: 999, .. }
    );
    b.fx.lock(v2);
    let mut doc = b.fx.doc();
    assert_err!(&mut doc, add_tr(v2, Some(o), None, S), Error::TrackLocked { track_id } if track_id == v2);
}

#[test]
fn c_trans_2_remove_and_set() {
    let (mut b, [p, q, ..]) = tr_fx();
    let v1 = b.v1;
    let t = b.fx.transition(v1, Some(p), Some(q), S);
    let mut doc = b.fx.doc();
    let mut params = BTreeMap::new();
    params.insert("angle".to_string(), 45.0);
    ok(
        &mut doc,
        Command::SetTransition {
            transition_id: t,
            effect: Some("wipe".into()),
            duration: Some(2 * S),
            params: Some(params.clone()),
        },
    );
    let tt = &trk(doc.project(), v1).transitions[0];
    assert_eq!(
        (tt.effect.as_str(), tt.duration, &tt.params),
        ("wipe", 2 * S, &params)
    );
    // 4s + 1: duration/2 = 2s fits P, remainder 2s + 1 does not fit Q
    assert_err!(
        &mut doc,
        Command::SetTransition {
            transition_id: t,
            effect: None,
            duration: Some(4 * S + 1),
            params: None
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::SetTransition {
            transition_id: 999,
            effect: None,
            duration: Some(S),
            params: None
        },
        Error::NotFound { id: 999, .. }
    );
    ok(&mut doc, Command::RemoveTransition { transition_id: t });
    assert!(trk(doc.project(), v1).transitions.is_empty());
    assert_err!(&mut doc, Command::RemoveTransition { transition_id: t }, Error::NotFound { id, .. } if id == t);
}

// ============================================================================ G8 / C-TRANS-3

#[test]
fn g8_trim_breaking_adjacency_prunes_transition() {
    let (mut b, [p, q, ..]) = tr_fx();
    let v1 = b.v1;
    let t = b.fx.transition(v1, Some(p), Some(q), S);
    let keep = b.fx.transition(v1, None, Some(p), S);
    let mut doc = b.fx.doc();
    ok(&mut doc, trim(p, Edge::End, S + S / 2, TrimMode::Normal));
    let ids: Vec<_> = trk(doc.project(), v1)
        .transitions
        .iter()
        .map(|x| x.id)
        .collect();
    assert_eq!(ids, vec![keep]);
    assert!(doc.undo());
    assert!(trk(doc.project(), v1).transitions.iter().any(|x| x.id == t));
}

#[test]
fn g8_trim_making_clip_too_short_prunes_one_sided() {
    let (mut b, [_, _, r, _]) = tr_fx();
    let v1 = b.v1;
    b.fx.transition(v1, Some(r), None, S);
    let mut doc = b.fx.doc();
    ok(
        &mut doc,
        trim(r, Edge::End, 5 * S + S / 2, TrimMode::Normal),
    );
    assert!(trk(doc.project(), v1).transitions.is_empty());
}

#[test]
fn g8_roll_keeps_valid_transition() {
    let (mut b, [p, q, ..]) = tr_fx();
    let v1 = b.v1;
    let t = b.fx.transition(v1, Some(p), Some(q), S);
    let mut doc = b.fx.doc();
    ok(&mut doc, trim(p, Edge::End, 3 * S, TrimMode::Roll)); // P [0,3s), Q [3s,4s): 1s - 0.5s fits
    let ts = &trk(doc.project(), v1).transitions;
    assert_eq!(ts.iter().map(|x| x.id).collect::<Vec<_>>(), vec![t]);
}

#[test]
fn c_trans_3_spec_example() {
    let Base { mut fx, m, v1, .. } = base();
    let p = fx.clip(v1, m, 0, 2 * S, 0);
    let q = fx.clip(v1, m, 2 * S, 2 * S, 0);
    let t = fx.transition(v1, Some(p), Some(q), S);
    let mut doc = fx.doc();
    let before = doc.project().clone();
    ok(
        &mut doc,
        Command::MoveClip {
            clip_id: q,
            start: 3 * S,
            track_id: None,
            mode: PlaceMode::Place,
            include_linked: true,
        },
    );
    assert!(trk(doc.project(), v1).transitions.is_empty());
    assert!(doc.undo());
    assert_eq!(doc.project(), &before);
    assert_eq!(trk(doc.project(), v1).transitions[0].id, t);
}
