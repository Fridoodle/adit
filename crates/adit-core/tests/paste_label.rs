//! Clip colour labels and pasting: C-LABEL-1, C-PASTE-1, C-PASTE-2, C-PASTE-3, with G1, G2, G3,
//! G4, G5, G7, G8, G9, G10, R4, R5, R7.
//! Written from docs/SPEC.md only. Ids are literals: `base()` ends with `next_id = 10`.
#[macro_use]
mod common;
use common::*;
use std::collections::BTreeMap;

fn it(track_id: Id, clip: Clip) -> PasteItem {
    PasteItem { track_id, clip }
}

fn place(items: Vec<PasteItem>) -> Command {
    Command::PasteClips {
        items,
        mode: PlaceMode::Place,
    }
}

fn over(items: Vec<PasteItem>) -> Command {
    Command::PasteClips {
        items,
        mode: PlaceMode::Overwrite,
    }
}

/// A clip to paste. Id 999 does not exist anywhere: C-PASTE-1 says it is ignored.
fn pc(source: ClipSource, start: Time, duration: Time) -> Clip {
    mk_clip(999, source, start, duration, 0)
}

fn fxi(id: Id, effect: &str, enabled: bool, params: &[(&str, Param)]) -> EffectInstance {
    EffectInstance {
        id,
        effect: effect.to_string(),
        enabled,
        params: params
            .iter()
            .map(|(n, p)| (n.to_string(), p.clone()))
            .collect(),
    }
}

fn set_label(clip_id: Id, label: Option<u8>) -> Command {
    Command::SetClip {
        clip_id,
        name: None,
        enabled: None,
        reverse: None,
        blend: None,
        audio_filter: None,
        label,
    }
}

// ============================================================================ C-LABEL-1

#[test]
fn c_label_1_add_clip_creates_label_0() {
    let Base { fx, m, v1, a1, .. } = base();
    let mut doc = fx.doc();
    ok(&mut doc, add_m(v1, m, 0, S)); // 10
    ok(&mut doc, add(v1, text_src("t"), S, S)); // 11
    ok(
        &mut doc,
        AddB {
            linked_track_id: Some(a1),
            ..addb(v1, media_src(m), 2 * S, S)
        }
        .cmd(),
    ); // 12, 13, link 14
    for id in [10, 11, 12, 13] {
        assert_eq!(
            clip(doc.project(), id).label,
            0,
            "C-LABEL-1: AddClip creates label 0 (clip {id})"
        );
    }
}

#[test]
fn c_label_1_set_clip_sets_label_and_nothing_else() {
    let Base { mut fx, m, v1, .. } = base();
    fx.clip(v1, m, 0, 10 * S, 0); // 10
    let mut doc = fx.doc();
    let before = clip(doc.project(), 10).clone();
    assert_eq!(before.label, 0, "C-LABEL-1: fixture starts unlabelled");
    for n in [3u8, 1, 2, 4, 5, 6, 7, 8, 0] {
        ok(&mut doc, set_label(10, Some(n)));
        assert_eq!(
            clip(doc.project(), 10),
            &Clip {
                label: n,
                ..before.clone()
            },
            "C-LABEL-1: SetClip {{ label: Some({n}) }} sets only the label"
        );
    }
    assert_eq!(
        doc.project().next_id,
        11,
        "C-LABEL-1/G4: no id is allocated"
    );
}

#[test]
fn c_label_1_label_8_ok_9_invalid_argument() {
    let Base { mut fx, m, v1, .. } = base();
    fx.clip(v1, m, 0, 10 * S, 0); // 10
    let mut doc = fx.doc();
    let out = ok(&mut doc, set_label(10, Some(8)));
    assert!(out.created.is_empty(), "C-LABEL-1: created is empty");
    assert_eq!(clip(doc.project(), 10).label, 8, "C-LABEL-1: 8 is a colour");
    for n in [9u8, 10, 200, 255] {
        assert_err!(&mut doc, set_label(10, Some(n)), Error::InvalidArgument(_));
        assert_eq!(
            clip(doc.project(), 10).label,
            8,
            "C-LABEL-1/G1: label {n} > 8 is rejected, label unchanged"
        );
    }
}

#[test]
fn c_label_1_set_clip_label_none_keeps_label() {
    let Base { mut fx, m, v1, .. } = base();
    let x = fx.clip(v1, m, 0, 10 * S, 0); // 10
    fx.c(x).label = 5;
    let mut doc = fx.doc();
    ok(
        &mut doc,
        Command::SetClip {
            clip_id: 10,
            name: Some("renamed".into()),
            enabled: None,
            reverse: None,
            blend: None,
            audio_filter: None,
            label: None,
        },
    );
    let c = clip(doc.project(), 10);
    assert_eq!(c.name, "renamed", "C-SET-1: name set");
    assert_eq!(c.label, 5, "C-LABEL-1: label None leaves the label alone");
}

#[test]
fn c_label_1_set_clip_label_omitted_in_json_is_none() {
    let c: Command = serde_json::from_str(r#"{"op":"set_clip","clip_id":10}"#).unwrap();
    assert_eq!(
        c,
        set_label(10, None),
        "C-LABEL-1/G9: SetClip.label defaults to None when omitted"
    );
    let c: Command = serde_json::from_str(r#"{"op":"set_clip","clip_id":10,"label":3}"#).unwrap();
    assert_eq!(
        c,
        set_label(10, Some(3)),
        "C-LABEL-1/G9: SetClip.label reads from JSON"
    );
    let back: Command = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
    assert_eq!(back, c, "C-LABEL-1/G9: SetClip with label round-trips");
}

#[test]
fn c_label_1_label_absent_in_clip_json_is_0() {
    let json = r#"{"id":10,"name":"c10","start":0,"duration":705600000,
        "source":{"type":"adjustment"},"source_in":0,"speed":1.0,"reverse":false,
        "enabled":true,"link":null,"group":null,"blend":"normal","audio_filter":"",
        "params":{},"effects":[]}"#;
    let c: Clip = serde_json::from_str(json).unwrap();
    assert_eq!(
        c,
        mk_clip(10, ClipSource::Adjustment, 0, 705_600_000, 0),
        "C-LABEL-1: label absent in JSON reads as 0"
    );
    assert_eq!(c.label, 0, "C-LABEL-1: label absent in JSON reads as 0");
}

#[test]
fn c_label_1_label_absent_in_project_json_is_0() {
    let Base { mut fx, m, v1, .. } = base();
    fx.clip(v1, m, 0, 10 * S, 0); // 10
    let p = fx.p;
    assert!(p.validate().is_ok());
    let mut v: serde_json::Value = serde_json::from_str(&p.to_json()).unwrap();
    let removed = v["sequences"][0]["tracks"][0]["clips"][0]
        .as_object_mut()
        .expect("clip is a JSON object")
        .remove("label");
    assert_eq!(
        removed,
        Some(serde_json::json!(0)),
        "C-LABEL-1/G9: label is written to JSON as a number"
    );
    assert_eq!(
        Project::from_json(&v.to_string()),
        Ok(p),
        "C-LABEL-1: a project file without label loads with label 0"
    );
}

#[test]
fn c_label_1_g9_clip_with_label_round_trips() {
    let Base { mut fx, m, v1, .. } = base();
    let x = fx.clip(v1, m, 0, 10 * S, 0); // 10
    fx.c(x).label = 7;
    let p = fx.p;
    assert!(
        p.validate().is_ok(),
        "C-LABEL-1: label 7 is a valid project"
    );
    let json = p.to_json();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        v["sequences"][0]["tracks"][0]["clips"][0]["label"],
        serde_json::json!(7),
        "C-LABEL-1/G9: label is stored in the project JSON"
    );
    let back = Project::from_json(&json);
    assert_eq!(
        back,
        Ok(p),
        "C-LABEL-1/G9: project with a label round-trips"
    );
    assert_eq!(
        clip(&back.unwrap(), 10).label,
        7,
        "C-LABEL-1/G9: label survives the round trip"
    );
}

#[test]
fn c_label_1_split_clip_copies_label() {
    let Base { mut fx, m, v1, .. } = base();
    let x = fx.clip(v1, m, 0, 10 * S, 0); // 10
    fx.c(x).label = 4;
    let mut doc = fx.doc();
    let out = ok(
        &mut doc,
        Command::SplitClip {
            clip_id: 10,
            time: 4 * S,
            include_linked: true,
        },
    );
    assert_eq!(out.created, vec![11], "C-SPLIT-2: right part id");
    assert_eq!(
        (clip(doc.project(), 10).label, clip(doc.project(), 11).label),
        (4, 4),
        "C-LABEL-1/R4: both parts of a split keep the label"
    );
}

#[test]
fn c_label_1_split_at_copies_label() {
    let Base {
        mut fx, m, seq, v1, ..
    } = base();
    let x = fx.clip(v1, m, 0, 10 * S, 0); // 10
    fx.c(x).label = 8;
    let mut doc = fx.doc();
    let out = ok(
        &mut doc,
        Command::SplitAt {
            sequence_id: seq,
            time: 2 * S,
            track_ids: None,
        },
    );
    assert_eq!(out.created, vec![11], "C-SPLIT-1: right part id");
    assert_eq!(
        (clip(doc.project(), 10).label, clip(doc.project(), 11).label),
        (8, 8),
        "C-LABEL-1/R4: both parts of a split keep the label"
    );
}

#[test]
fn c_label_1_split_by_overwrite_paste_copies_label() {
    let Base { mut fx, m, v1, .. } = base();
    let x = fx.clip(v1, m, 0, 10 * S, 0); // 10
    fx.c(x).label = 4;
    let mut doc = fx.doc();
    let out = ok(&mut doc, over(vec![it(v1, pc(media_src(m), 4 * S, 2 * S))]));
    assert_eq!(out.created, vec![11], "C-PASTE-2: created");
    assert_eq!(
        spans(doc.project(), v1),
        vec![(10, 0, 4 * S), (11, 4 * S, 6 * S), (12, 6 * S, 10 * S)],
        "C-PASTE-3/R5: X is split around the copy"
    );
    assert_eq!(
        (
            clip(doc.project(), 10).label,
            clip(doc.project(), 11).label,
            clip(doc.project(), 12).label
        ),
        (4, 0, 4),
        "C-LABEL-1/R4: split parts keep label 4, the copy keeps its own label 0"
    );
}

#[test]
fn c_label_1_g5_locked_track_is_track_locked() {
    let Base { mut fx, m, v1, .. } = base();
    fx.clip(v1, m, 0, 10 * S, 0); // 10
    fx.lock(v1);
    let mut doc = fx.doc();
    assert_err!(
        &mut doc,
        set_label(10, Some(1)),
        Error::TrackLocked { track_id: 6 }
    );
    assert_eq!(
        clip(doc.project(), 10).label,
        0,
        "C-LABEL-1/G5/G10: label on a locked track does not change"
    );
}

#[test]
fn c_label_1_g7_unknown_clip_is_not_found() {
    let Base { fx, .. } = base();
    let mut doc = fx.doc();
    assert_err!(
        &mut doc,
        set_label(99, Some(1)),
        Error::NotFound { id: 99, .. }
    );
}

#[test]
fn c_label_1_g3_undo_restores_label() {
    let Base { mut fx, m, v1, .. } = base();
    let x = fx.clip(v1, m, 0, 10 * S, 0); // 10
    fx.c(x).label = 2;
    let mut doc = fx.doc();
    let before = doc.project().clone();
    ok(&mut doc, set_label(10, Some(6)));
    assert_eq!(doc.undo_depth(), 1, "C-LABEL-1/G2: one undo entry");
    assert_eq!(clip(doc.project(), 10).label, 6, "C-LABEL-1: label set");
    assert!(doc.undo(), "C-LABEL-1/G3: undo available");
    assert_eq!(
        doc.project(),
        &before,
        "C-LABEL-1/G3: undo restores label 2"
    );
    assert!(doc.redo(), "C-LABEL-1/G3: redo available");
    assert_eq!(clip(doc.project(), 10).label, 6, "C-LABEL-1/G3: redo");
}

// ============================================================================ C-PASTE-1

#[test]
fn c_paste_1_copy_equals_clip_except_id() {
    let Base { fx, m, v1, .. } = base();
    let mut doc = fx.doc();
    let mut c = mk_clip(999, media_src(m), 2 * S, 3 * S, 7 * S);
    c.name = "orig".into();
    c.speed = 2.0;
    c.reverse = true;
    c.enabled = false;
    c.blend = BlendMode::Screen;
    c.audio_filter = "volume=0.5".into();
    c.label = 3;
    c.params.insert(
        "opacity".into(),
        param(
            0.5,
            vec![kf(-S, 0.0, Easing::Linear), kf(S, 1.0, Easing::Hold)],
        ),
    );
    c.params.insert("volume".into(), param(-6.0, vec![]));
    let out = ok(&mut doc, place(vec![it(v1, c.clone())]));
    assert_eq!(out.created, vec![10], "C-PASTE-2: created = [next_id]");
    assert_eq!(doc.project().next_id, 11, "C-PASTE-2/G4: one id used");
    assert_eq!(
        clip(doc.project(), 10),
        &Clip { id: 10, ..c },
        "C-PASTE-1/C-PASTE-2: the copy equals its clip except the id"
    );
    assert_eq!(
        spans(doc.project(), v1),
        vec![(10, 2 * S, 5 * S)],
        "C-PASTE-1: clip.start is where the copy goes"
    );
    assert_eq!(doc.undo_depth(), 1, "C-PASTE-1/G2: one undo entry");
}

#[test]
fn c_paste_1_clip_id_is_ignored() {
    let Base { mut fx, m, v1, .. } = base();
    fx.clip(v1, m, 0, S, 0); // 10
    let mut doc = fx.doc();
    let original = clip(doc.project(), 10).clone();
    // Same id as an existing clip.
    let mut c = original.clone();
    c.start = 5 * S;
    let out = ok(&mut doc, place(vec![it(v1, c)]));
    assert_eq!(out.created, vec![11], "C-PASTE-1: clip.id 10 is ignored");
    // Id 0 and an id of another entity kind (media 1).
    let mut z = original.clone();
    z.id = 0;
    z.start = 7 * S;
    let mut e = original.clone();
    e.id = 1;
    e.start = 9 * S;
    let out = ok(&mut doc, place(vec![it(v1, z), it(v1, e)]));
    assert_eq!(out.created, vec![12, 13], "C-PASTE-1: clip.id is ignored");
    assert_eq!(
        clip(doc.project(), 10),
        &original,
        "C-PASTE-1: the clip that had the same id is untouched"
    );
    assert_eq!(
        spans(doc.project(), v1),
        vec![
            (10, 0, S),
            (11, 5 * S, 6 * S),
            (12, 7 * S, 8 * S),
            (13, 9 * S, 10 * S)
        ],
        "C-PASTE-1: copies are added, nothing else moves"
    );
}

#[test]
fn c_paste_1_mode_defaults_to_place_in_json() {
    let c: Command = serde_json::from_str(r#"{"op":"paste_clips","items":[]}"#).unwrap();
    assert_eq!(
        c,
        Command::PasteClips {
            items: vec![],
            mode: PlaceMode::Place
        },
        "C-PASTE-1/G9: mode defaults to place"
    );
    let c: Command =
        serde_json::from_str(r#"{"op":"paste_clips","items":[],"mode":"overwrite"}"#).unwrap();
    assert_eq!(
        c,
        Command::PasteClips {
            items: vec![],
            mode: PlaceMode::Overwrite
        },
        "C-PASTE-1/G9: mode reads from JSON"
    );
}

#[test]
fn c_paste_1_item_clip_without_label_in_json_is_0() {
    let json = r#"{"op":"paste_clips","items":[{"track_id":6,"clip":
        {"id":0,"name":"c0","start":1411200000,"duration":705600000,
        "source":{"type":"media","media_id":1},"source_in":0,"speed":1.0,"reverse":false,
        "enabled":true,"link":null,"group":null,"blend":"normal","audio_filter":"",
        "params":{},"effects":[]}}]}"#;
    let c: Command = serde_json::from_str(json).unwrap();
    assert_eq!(
        c,
        place(vec![it(6, mk_clip(0, media_src(1), 2 * S, S, 0))]),
        "C-PASTE-1/C-LABEL-1/G9: item clip without label reads as label 0, mode place"
    );
}

#[test]
fn c_paste_1_g9_command_round_trips() {
    let mut a = mk_clip(42, media_src(1), 2 * S, 3 * S, 7 * S);
    a.label = 8;
    a.link = Some(100);
    a.group = Some(200);
    a.speed = 0.5;
    a.reverse = true;
    a.blend = BlendMode::Multiply;
    a.params.insert(
        "opacity".into(),
        param(
            0.25,
            vec![kf(0, 0.0, Easing::EaseInOut), kf(S, 1.0, Easing::Linear)],
        ),
    );
    a.effects.push(fxi(
        43,
        "blur",
        false,
        &[("amount", param(2.0, vec![kf(0, 1.0, Easing::Hold)]))],
    ));
    let b = mk_clip(44, text_src("hi"), 0, S, 0);
    for mode in [PlaceMode::Place, PlaceMode::Overwrite, PlaceMode::Insert] {
        let c = Command::PasteClips {
            items: vec![it(6, a.clone()), it(7, b.clone())],
            mode,
        };
        let json = serde_json::to_string(&c).unwrap();
        let back: Command = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c, "C-PASTE-1/G9: PasteClips round-trips ({mode:?})");
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            v["op"],
            serde_json::json!("paste_clips"),
            "C-PASTE-1/G9: op tag"
        );
        assert_eq!(
            v["items"][0]["clip"]["label"],
            serde_json::json!(8),
            "C-PASTE-1/G9: label is carried in the command JSON"
        );
    }
}

#[test]
fn c_paste_1_empty_items_invalid_argument() {
    let Base { fx, .. } = base();
    let mut doc = fx.doc();
    assert_err!(&mut doc, place(vec![]), Error::InvalidArgument(_));
    assert_err!(&mut doc, over(vec![]), Error::InvalidArgument(_));
}

#[test]
fn c_paste_1_tracks_in_two_sequences_invalid_argument() {
    let Base { mut fx, m, v1, .. } = base();
    let other = fx.seq("Other"); // 10
    let ov = fx.track(other, TrackKind::Video); // 11
    let mut doc = fx.doc();
    for cmd in [
        place(vec![
            it(v1, pc(media_src(m), 0, S)),
            it(ov, pc(media_src(m), 0, S)),
        ]),
        over(vec![
            it(ov, pc(media_src(m), 0, S)),
            it(v1, pc(media_src(m), 5 * S, S)),
        ]),
    ] {
        assert_err!(&mut doc, cmd, Error::InvalidArgument(_));
    }
    // One sequence at a time is fine, whichever it is.
    let out = ok(&mut doc, place(vec![it(ov, pc(media_src(m), 0, S))]));
    assert_eq!(
        out.created,
        vec![12],
        "C-PASTE-1: all items in one (other) sequence is ok"
    );
    assert_eq!(track_of(doc.project(), 12), ov, "C-PASTE-1: on its track");
}

#[test]
fn c_paste_1_mode_insert_invalid_argument() {
    let Base { fx, m, v1, .. } = base();
    let mut doc = fx.doc();
    assert_err!(
        &mut doc,
        Command::PasteClips {
            items: vec![it(v1, pc(media_src(m), 0, S))],
            mode: PlaceMode::Insert,
        },
        Error::InvalidArgument(_)
    );
}

#[test]
fn c_paste_1_unknown_track_not_found() {
    let Base { fx, m, v1, .. } = base();
    let mut doc = fx.doc();
    assert_err!(
        &mut doc,
        place(vec![it(99, pc(media_src(m), 0, S))]),
        Error::NotFound { id: 99, .. }
    );
    // A good first item does not get through (G1).
    assert_err!(
        &mut doc,
        over(vec![
            it(v1, pc(media_src(m), 0, S)),
            it(98, pc(media_src(m), 0, S)),
        ]),
        Error::NotFound { id: 98, .. }
    );
    // An id of another entity kind is not a track.
    assert_err!(
        &mut doc,
        place(vec![it(5, pc(media_src(m), 0, S))]),
        Error::NotFound { id: 5, .. }
    );
}

#[test]
fn c_paste_1_g5_locked_track_is_track_locked() {
    let Base {
        mut fx, m, v1, v2, ..
    } = base();
    fx.lock(v2);
    let mut doc = fx.doc();
    assert_err!(
        &mut doc,
        place(vec![it(v2, pc(media_src(m), 0, S))]),
        Error::TrackLocked { track_id: 7 }
    );
    assert_err!(
        &mut doc,
        over(vec![it(v2, pc(media_src(m), 0, S))]),
        Error::TrackLocked { track_id: 7 }
    );
    assert_err!(
        &mut doc,
        place(vec![
            it(v1, pc(media_src(m), 0, S)),
            it(v2, pc(media_src(m), 0, S)),
        ]),
        Error::TrackLocked { track_id: 7 }
    );
    assert!(
        all_clip_ids(doc.project()).is_empty(),
        "C-PASTE-1/G1: nothing was pasted"
    );
}

#[test]
fn c_paste_1_i4_kind_mismatch() {
    let Base {
        fx,
        aud,
        vid,
        still,
        v1,
        a1,
        ..
    } = base();
    let mut doc = fx.doc();
    for (track, source) in [
        (v1, media_src(aud)),
        (a1, media_src(vid)),
        (a1, media_src(still)),
        (a1, text_src("t")),
        (a1, solid_src()),
        (a1, ClipSource::Adjustment),
    ] {
        assert_err!(
            &mut doc,
            place(vec![it(track, pc(source.clone(), 0, S))]),
            Error::KindMismatch(_)
        );
        assert_err!(
            &mut doc,
            over(vec![it(track, pc(source, 0, S))]),
            Error::KindMismatch(_)
        );
    }
}

#[test]
fn c_paste_1_i4_allowed_kinds() {
    let Base {
        mut fx,
        m,
        still,
        aud,
        v1,
        a1,
        ..
    } = base();
    let inner = fx.seq("Inner"); // 10
    let itrack = fx.track(inner, TrackKind::Video); // 11
    fx.clip(itrack, m, 0, S, 0); // 12
    let mut doc = fx.doc();
    let sources_v = [
        media_src(still),
        seq_src(inner),
        text_src("t"),
        solid_src(),
        ClipSource::Adjustment,
    ];
    let sources_a = [media_src(aud), seq_src(inner), media_src(m)];
    let mut items = Vec::new();
    for (i, s) in sources_v.iter().enumerate() {
        items.push(it(v1, pc(s.clone(), i as Time * S, S)));
    }
    for (i, s) in sources_a.iter().enumerate() {
        items.push(it(a1, pc(s.clone(), i as Time * S, S)));
    }
    let out = ok(&mut doc, place(items));
    assert_eq!(
        out.created,
        vec![13, 14, 15, 16, 17, 18, 19, 20],
        "C-PASTE-2: created in item order"
    );
    assert_eq!(doc.project().next_id, 21, "C-PASTE-2/G4: eight ids used");
    for (i, s) in sources_v.iter().enumerate() {
        let c = clip(doc.project(), 13 + i as Id);
        assert_eq!(&c.source, s, "C-PASTE-1/I4: source kept on a video track");
        assert_eq!(track_of(doc.project(), c.id), v1, "C-PASTE-1: track");
    }
    for (i, s) in sources_a.iter().enumerate() {
        let c = clip(doc.project(), 18 + i as Id);
        assert_eq!(&c.source, s, "C-PASTE-1/I4: source kept on an audio track");
        assert_eq!(track_of(doc.project(), c.id), a1, "C-PASTE-1: track");
    }
}

#[test]
fn c_paste_1_unknown_media_or_sequence_not_found() {
    let Base { fx, m, v1, .. } = base();
    let mut doc = fx.doc();
    assert_err!(
        &mut doc,
        place(vec![it(v1, pc(media_src(77), 0, S))]),
        Error::NotFound { id: 77, .. }
    );
    assert_err!(
        &mut doc,
        place(vec![it(v1, pc(seq_src(78), 0, S))]),
        Error::NotFound { id: 78, .. }
    );
    // Sequence id 5 is not a media id; media id 1 is not a sequence id.
    assert_err!(
        &mut doc,
        over(vec![it(v1, pc(media_src(5), 0, S))]),
        Error::NotFound { id: 5, .. }
    );
    assert_err!(
        &mut doc,
        over(vec![it(v1, pc(seq_src(1), 0, S))]),
        Error::NotFound { id: 1, .. }
    );
    // Second item bad: nothing from the first stays (G1).
    assert_err!(
        &mut doc,
        place(vec![
            it(v1, pc(media_src(m), 0, S)),
            it(v1, pc(media_src(77), 5 * S, S)),
        ]),
        Error::NotFound { id: 77, .. }
    );
}

#[test]
fn c_paste_1_nest_cycle_direct_and_indirect() {
    let Base {
        mut fx, seq, v1, ..
    } = base();
    let inner = fx.seq("Inner"); // 10
    let itrack = fx.track(inner, TrackKind::Video); // 11
    fx.clip_src(itrack, seq_src(seq), 0, S, 0); // 12: Inner nests Main
    let mut doc = fx.doc();
    assert_err!(
        &mut doc,
        place(vec![it(v1, pc(seq_src(seq), 0, S))]),
        Error::NestCycle { .. }
    );
    assert_err!(
        &mut doc,
        over(vec![it(v1, pc(seq_src(inner), 0, S))]),
        Error::NestCycle { .. }
    );
}

/// Still image on V1: no source bounds, so only the named field is wrong.
fn bad(change: impl Fn(&mut Clip)) -> Clip {
    let mut c = pc(media_src(2), 0, S);
    change(&mut c);
    c
}

#[test]
fn c_paste_1_bad_start_duration_speed_label_invalid_argument() {
    let Base { fx, v1, .. } = base();
    let mut doc = fx.doc();
    let cases = [
        ("start < 0", bad(|c| c.start = -1)),
        ("start < 0", bad(|c| c.start = -S)),
        ("duration 0", bad(|c| c.duration = 0)),
        ("duration < 0", bad(|c| c.duration = -S)),
        ("speed below 0.01", bad(|c| c.speed = 0.009)),
        ("speed above 100", bad(|c| c.speed = 100.5)),
        ("speed 0", bad(|c| c.speed = 0.0)),
        ("speed negative", bad(|c| c.speed = -1.0)),
        ("speed NaN", bad(|c| c.speed = f64::NAN)),
        ("speed inf", bad(|c| c.speed = f64::INFINITY)),
        ("label 9", bad(|c| c.label = 9)),
        ("label 255", bad(|c| c.label = 255)),
    ];
    for (what, c) in cases {
        for cmd in [
            place(vec![it(v1, c.clone())]),
            over(vec![it(v1, c.clone())]),
        ] {
            let e = err(&mut doc, cmd);
            assert!(
                matches!(e, Error::InvalidArgument(_)),
                "C-PASTE-1: {what} must be InvalidArgument, got {e:?}"
            );
        }
    }
}

#[test]
fn c_paste_1_speed_and_label_bounds_are_inclusive() {
    let Base { fx, v1, .. } = base();
    let mut doc = fx.doc();
    let out = ok(
        &mut doc,
        place(vec![
            it(v1, bad(|c| c.speed = 0.01)),
            it(
                v1,
                bad(|c| {
                    c.start = S;
                    c.speed = 100.0;
                    c.label = 8;
                }),
            ),
        ]),
    );
    assert_eq!(out.created, vec![10, 11], "C-PASTE-2: created");
    assert_eq!(
        clip(doc.project(), 10).speed,
        0.01,
        "C-PASTE-1: speed 0.01 is allowed and kept"
    );
    assert_eq!(
        clip(doc.project(), 11).speed,
        100.0,
        "C-PASTE-1: speed 100 is allowed and kept"
    );
    assert_eq!(
        clip(doc.project(), 11).label,
        8,
        "C-PASTE-1: label 8 is allowed and kept"
    );
}

#[test]
fn c_paste_1_bad_params_and_keyframes_invalid_argument() {
    let Base { fx, v1, .. } = base();
    let mut doc = fx.doc();
    let p = |c: &mut Clip, name: &str, v: Param| {
        c.params.insert(name.to_string(), v);
    };
    let cases = [
        (
            "param name not in CLIP_PARAMS",
            bad(|c| p(c, "bogus", param(1.0, vec![]))),
        ),
        (
            "param name not in CLIP_PARAMS (empty)",
            bad(|c| p(c, "", param(1.0, vec![]))),
        ),
        (
            "unsorted keyframe times",
            bad(|c| {
                p(
                    c,
                    "opacity",
                    param(
                        1.0,
                        vec![kf(S, 0.0, Easing::Linear), kf(0, 1.0, Easing::Linear)],
                    ),
                )
            }),
        ),
        (
            "duplicate keyframe times",
            bad(|c| {
                p(
                    c,
                    "opacity",
                    param(
                        1.0,
                        vec![kf(S, 0.0, Easing::Linear), kf(S, 1.0, Easing::Linear)],
                    ),
                )
            }),
        ),
        (
            "non-finite param value",
            bad(|c| p(c, "opacity", param(f64::NAN, vec![]))),
        ),
        (
            "non-finite keyframe value",
            bad(|c| {
                p(
                    c,
                    "opacity",
                    param(1.0, vec![kf(0, f64::INFINITY, Easing::Linear)]),
                )
            }),
        ),
        (
            "non-finite effect param value",
            bad(|c| {
                c.effects.push(fxi(
                    1,
                    "blur",
                    true,
                    &[("amount", param(f64::NEG_INFINITY, vec![]))],
                ))
            }),
        ),
        (
            "non-finite effect keyframe value",
            bad(|c| {
                c.effects.push(fxi(
                    1,
                    "blur",
                    true,
                    &[("amount", param(1.0, vec![kf(0, f64::NAN, Easing::Linear)]))],
                ))
            }),
        ),
        (
            "unsorted effect keyframe times",
            bad(|c| {
                c.effects.push(fxi(
                    1,
                    "blur",
                    true,
                    &[(
                        "amount",
                        param(
                            1.0,
                            vec![kf(S, 0.0, Easing::Linear), kf(0, 1.0, Easing::Linear)],
                        ),
                    )],
                ))
            }),
        ),
        (
            "duplicate effect keyframe times",
            bad(|c| {
                c.effects.push(fxi(
                    1,
                    "blur",
                    true,
                    &[(
                        "amount",
                        param(
                            1.0,
                            vec![kf(0, 0.0, Easing::Linear), kf(0, 1.0, Easing::Linear)],
                        ),
                    )],
                ))
            }),
        ),
    ];
    for (what, c) in cases {
        for cmd in [
            place(vec![it(v1, c.clone())]),
            over(vec![it(v1, c.clone())]),
        ] {
            let e = err(&mut doc, cmd);
            assert!(
                matches!(e, Error::InvalidArgument(_)),
                "C-PASTE-1: {what} must be InvalidArgument, got {e:?}"
            );
        }
    }
}

#[test]
fn c_paste_1_non_finite_number_in_source_invalid_argument() {
    let Base { fx, v1, .. } = base();
    let mut doc = fx.doc();
    let solid = pc(
        ClipSource::Solid {
            color: [0.0, f32::NAN, 0.0, 1.0],
        },
        0,
        S,
    );
    let mut spec = text_spec("t");
    spec.size = f64::INFINITY;
    let text_size = pc(ClipSource::Text { text: spec }, 0, S);
    let mut spec = text_spec("t");
    spec.color = [1.0, 1.0, f32::INFINITY, 1.0];
    let text_color = pc(ClipSource::Text { text: spec }, 0, S);
    for (what, c) in [
        ("solid colour NaN", solid),
        ("text size inf", text_size),
        ("text colour inf", text_color),
    ] {
        let e = err(&mut doc, place(vec![it(v1, c)]));
        assert!(
            matches!(e, Error::InvalidArgument(_)),
            "C-PASTE-1: any non-finite number ({what}) must be InvalidArgument, got {e:?}"
        );
    }
}

#[test]
fn c_paste_1_error_in_a_later_item_is_atomic() {
    let Base { mut fx, m, v1, .. } = base();
    fx.clip(v1, m, 0, 10 * S, 0); // 10
    let mut doc = fx.doc();
    ok(&mut doc, set_label(10, Some(1))); // undo depth 1
    assert!(doc.undo(), "setup: make a redo entry"); // undo 0, redo 1
    let good = pc(media_src(m), 4 * S, 2 * S);
    for (what, second) in [
        ("label 9", bad(|c| c.label = 9)),
        ("duration 0", bad(|c| c.duration = 0)),
    ] {
        let mut second = second;
        second.start = 20 * S;
        // `err` checks G1: project ==, undo and redo depths unchanged.
        let e = err(&mut doc, over(vec![it(v1, good.clone()), it(v1, second)]));
        assert!(
            matches!(e, Error::InvalidArgument(_)),
            "C-PASTE-1/G1: {what} in item 2 must be InvalidArgument, got {e:?}"
        );
    }
    assert_eq!(
        spans(doc.project(), v1),
        vec![(10, 0, 10 * S)],
        "C-PASTE-1/G1: the first item's overwrite did not happen"
    );
    assert_eq!(doc.project().next_id, 11, "C-PASTE-1/G1: next_id unchanged");
    assert_eq!(
        (doc.undo_depth(), doc.redo_depth()),
        (0, 1),
        "C-PASTE-1/G1: undo and redo depths unchanged"
    );
}

#[test]
fn c_paste_1_r7_source_out_of_range() {
    let Base { fx, m, v1, a1, .. } = base();
    let mut doc = fx.doc();
    // Media 1 is 100 s long.
    let mut past_end = pc(media_src(m), 0, 2 * S);
    past_end.source_in = 99 * S;
    let mut fast = pc(media_src(m), 0, 51 * S);
    fast.speed = 2.0; // reads 102 s
    let mut negative = pc(media_src(m), 0, S);
    negative.source_in = -1;
    for (what, c) in [
        ("source_in 99 s + 2 s > 100 s", past_end),
        ("51 s at speed 2 > 100 s", fast),
        ("negative source_in", negative),
    ] {
        for cmd in [
            place(vec![it(v1, c.clone())]),
            over(vec![it(a1, c.clone())]),
        ] {
            let e = err(&mut doc, cmd);
            assert!(
                matches!(e, Error::SourceOutOfRange { .. }),
                "C-PASTE-1/R7: {what} must be SourceOutOfRange, got {e:?}"
            );
        }
    }
}

#[test]
fn c_paste_1_r7_sequence_clip_negative_source_in() {
    let Base { mut fx, m, v1, .. } = base();
    let inner = fx.seq("Inner"); // 10
    let itrack = fx.track(inner, TrackKind::Video); // 11
    fx.clip(itrack, m, 0, S, 0); // 12
    let mut doc = fx.doc();
    let mut c = pc(seq_src(inner), 0, S);
    c.source_in = -1;
    assert_err!(
        &mut doc,
        place(vec![it(v1, c)]),
        Error::SourceOutOfRange { .. }
    );
}

#[test]
fn c_paste_1_r7_bounds_that_hold() {
    let Base {
        fx, m, still, v1, ..
    } = base();
    let mut doc = fx.doc();
    // Exactly to the end of the 100 s media.
    let mut exact = pc(media_src(m), 0, 2 * S);
    exact.source_in = 98 * S;
    // 50 s at speed 2 reads exactly 100 s.
    let mut fast = pc(media_src(m), 2 * S, 50 * S);
    fast.speed = 2.0;
    // A still has no duration: unbounded.
    let mut unbounded = pc(media_src(still), 52 * S, S);
    unbounded.source_in = 5000 * S;
    // Text has no source bounds; source_in stays as is.
    let mut text = pc(text_src("t"), 53 * S, S);
    text.source_in = 5 * S;
    let out = ok(
        &mut doc,
        place(vec![
            it(v1, exact),
            it(v1, fast),
            it(v1, unbounded),
            it(v1, text),
        ]),
    );
    assert_eq!(out.created, vec![10, 11, 12, 13], "C-PASTE-2: created");
    assert_eq!(
        [10, 11, 12, 13].map(|id| clip(doc.project(), id).source_in),
        [98 * S, 0, 5000 * S, 5 * S],
        "C-PASTE-1/R7: source_in is copied unchanged"
    );
}

// ============================================================================ C-PASTE-2

#[test]
fn c_paste_2_effect_instances_get_fresh_ids_after_all_clip_ids() {
    let Base { fx, m, v1, .. } = base();
    let mut doc = fx.doc();
    let amount = param(
        2.0,
        vec![kf(0, 1.0, Easing::Linear), kf(S, 3.0, Easing::EaseIn)],
    );
    let mut a = pc(media_src(m), 0, S);
    a.effects = vec![
        fxi(500, "blur", true, &[("amount", amount.clone())]),
        fxi(500, "glow", false, &[]),
    ];
    let mut b = pc(media_src(m), S, S);
    b.effects = vec![fxi(3, "blur", true, &[])];
    let out = ok(&mut doc, place(vec![it(v1, a), it(v1, b)]));
    assert_eq!(
        out.created,
        vec![10, 11],
        "C-PASTE-2: clip ids come first, in item order"
    );
    assert_eq!(
        clip(doc.project(), 10).effects,
        vec![
            fxi(12, "blur", true, &[("amount", amount)]),
            fxi(13, "glow", false, &[]),
        ],
        "C-PASTE-2: effect ids in item order then effect order; the rest of the instance is copied"
    );
    assert_eq!(
        clip(doc.project(), 11).effects,
        vec![fxi(14, "blur", true, &[])],
        "C-PASTE-2: second item's effect comes after the first item's effects"
    );
    assert_eq!(doc.project().next_id, 15, "C-PASTE-2/G4: five ids used");
}

#[test]
fn c_paste_2_shared_link_gets_one_fresh_id_single_becomes_none() {
    let Base { fx, m, v1, .. } = base();
    let mut doc = fx.doc();
    let links = [Some(300), Some(100), Some(300), Some(100), Some(77), None];
    let items = links
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let mut c = pc(media_src(m), i as Time * S, S);
            c.link = *l;
            it(v1, c)
        })
        .collect();
    let out = ok(&mut doc, place(items));
    assert_eq!(
        out.created,
        vec![10, 11, 12, 13, 14, 15],
        "C-PASTE-2: created[i] = next_id + i"
    );
    assert_eq!(
        [10, 11, 12, 13, 14, 15].map(|id| clip(doc.project(), id).link),
        [Some(16), Some(17), Some(16), Some(17), None, None],
        "C-PASTE-2: shared link value -> one fresh id (order of first appearance), single -> None"
    );
    assert_eq!(
        [10, 11, 12, 13, 14, 15].map(|id| clip(doc.project(), id).group),
        [None; 6],
        "C-PASTE-2: group None stays None"
    );
    assert_eq!(doc.project().next_id, 18, "C-PASTE-2/G4: 6 clips + 2 links");
}

#[test]
fn c_paste_2_shared_group_gets_one_fresh_id_single_becomes_none() {
    let Base { fx, m, v1, .. } = base();
    let mut doc = fx.doc();
    let groups = [Some(9), Some(4), Some(4), Some(9), Some(2)];
    let items = groups
        .iter()
        .enumerate()
        .map(|(i, g)| {
            let mut c = pc(media_src(m), i as Time * S, S);
            c.group = *g;
            it(v1, c)
        })
        .collect();
    let out = ok(&mut doc, place(items));
    assert_eq!(
        out.created,
        vec![10, 11, 12, 13, 14],
        "C-PASTE-2: created[i] = next_id + i"
    );
    assert_eq!(
        [10, 11, 12, 13, 14].map(|id| clip(doc.project(), id).group),
        [Some(15), Some(16), Some(16), Some(15), None],
        "C-PASTE-2: shared group value -> one fresh id (order of first appearance), single -> None"
    );
    assert_eq!(
        [10, 11, 12, 13, 14].map(|id| clip(doc.project(), id).link),
        [None; 5],
        "C-PASTE-2: link None stays None"
    );
    assert_eq!(
        doc.project().next_id,
        17,
        "C-PASTE-2/G4: 5 clips + 2 groups"
    );
}

#[test]
fn c_paste_2_id_order_clips_effects_links_groups() {
    let Base { fx, m, v1, v2, .. } = base();
    let mut doc = fx.doc();
    let mut a = pc(media_src(m), 0, S);
    a.effects = vec![fxi(1, "blur", true, &[])];
    a.link = Some(50);
    a.group = Some(50);
    let mut b = pc(media_src(m), 0, S);
    b.effects = vec![fxi(1, "blur", true, &[])];
    b.link = Some(50);
    b.group = Some(50);
    let mut c = pc(media_src(m), S, S);
    c.link = Some(60);
    c.group = Some(50);
    let out = ok(&mut doc, place(vec![it(v1, a), it(v2, b), it(v1, c)]));
    assert_eq!(out.created, vec![10, 11, 12], "C-PASTE-2: clip ids first");
    let p = doc.project();
    assert_eq!(
        (clip(p, 10).effects[0].id, clip(p, 11).effects[0].id),
        (13, 14),
        "C-PASTE-2: effect-instance ids second"
    );
    assert!(
        clip(p, 12).effects.is_empty(),
        "C-PASTE-2: no effects added"
    );
    assert_eq!(
        [10, 11, 12].map(|id| clip(p, id).link),
        [Some(15), Some(15), None],
        "C-PASTE-2: link ids third; a link carried by one item becomes None"
    );
    assert_eq!(
        [10, 11, 12].map(|id| clip(p, id).group),
        [Some(16), Some(16), Some(16)],
        "C-PASTE-2: group ids fourth; link and group values are separate namespaces"
    );
    assert_eq!(p.next_id, 17, "C-PASTE-2/G4: 3 + 2 + 1 + 1 ids used");
}

#[test]
fn c_paste_2_split_ids_come_last() {
    let Base {
        mut fx,
        m,
        aud,
        v1,
        a1,
        ..
    } = base();
    let x = fx.clip(v1, m, 0, 10 * S, 0); // 10
    fx.effect(x, "blur", &[]); // 11
    let mut doc = fx.doc();
    let mut a = pc(media_src(m), 4 * S, 2 * S);
    a.effects = vec![fxi(1, "glow", true, &[])];
    a.link = Some(1);
    let mut b = pc(media_src(aud), 4 * S, 2 * S);
    b.effects = vec![fxi(1, "eq", true, &[])];
    b.link = Some(1);
    let out = ok(&mut doc, over(vec![it(v1, a), it(a1, b)]));
    assert_eq!(out.created, vec![12, 13], "C-PASTE-2: created = clip ids");
    let p = doc.project();
    assert_eq!(
        (clip(p, 12).effects[0].id, clip(p, 13).effects[0].id),
        (14, 15),
        "C-PASTE-2: effect ids after clip ids"
    );
    assert_eq!(
        (clip(p, 12).link, clip(p, 13).link),
        (Some(16), Some(16)),
        "C-PASTE-2: link id after effect ids"
    );
    assert_eq!(
        spans(p, v1),
        vec![(10, 0, 4 * S), (12, 4 * S, 6 * S), (17, 6 * S, 10 * S)],
        "C-PASTE-2: ids needed for splitting other clips come last (right part = 17)"
    );
    assert_eq!(
        clip(p, 10).effects[0].id,
        11,
        "R4: the left part keeps its effect id"
    );
    assert_eq!(
        clip(p, 17).effects[0].id,
        18,
        "C-PASTE-2/R4: the right part's effect gets a new id after its clip id"
    );
    assert_eq!(p.next_id, 19, "C-PASTE-2/G4: 2 + 2 + 1 + 2 ids used");
}

#[test]
fn c_paste_2_link_value_of_existing_clips_is_not_joined() {
    let Base {
        mut fx, m, v1, a1, ..
    } = base();
    let a = fx.clip(v1, m, 0, S, 0); // 10
    let b = fx.clip(a1, m, 0, S, 0); // 11
    let l = fx.link(&[a, b]); // 12
    let g = fx.id(); // 13
    fx.c(a).group = Some(g);
    fx.c(b).group = Some(g);
    let mut doc = fx.doc();
    assert_eq!((l, g), (12, 13));
    let mut ca = clip(doc.project(), 10).clone();
    ca.start = 5 * S;
    let mut cb = clip(doc.project(), 11).clone();
    cb.start = 5 * S;

    // One carrier of link 12 / group 13 in the command: both become None.
    let out = ok(&mut doc, place(vec![it(v1, ca.clone())]));
    assert_eq!(out.created, vec![14], "C-PASTE-2: created");
    assert_eq!(
        (clip(doc.project(), 14).link, clip(doc.project(), 14).group),
        (None, None),
        "C-PASTE-2: only items of this command count; a single carrier becomes None"
    );
    assert_eq!(doc.project().next_id, 15, "C-PASTE-2/G4: one id used");

    // Both carriers: one fresh link id and one fresh group id, not the old ones.
    ca.start = 8 * S;
    cb.start = 8 * S;
    let out = ok(&mut doc, place(vec![it(v1, ca), it(a1, cb)]));
    assert_eq!(out.created, vec![15, 16], "C-PASTE-2: created");
    let p = doc.project();
    assert_eq!(
        (clip(p, 15).link, clip(p, 16).link),
        (Some(17), Some(17)),
        "C-PASTE-2: copies share a fresh link id"
    );
    assert_eq!(
        (clip(p, 15).group, clip(p, 16).group),
        (Some(18), Some(18)),
        "C-PASTE-2: copies share a fresh group id"
    );
    assert_eq!(
        [10, 11].map(|id| (clip(p, id).link, clip(p, id).group)),
        [(Some(12), Some(13)); 2],
        "C-PASTE-2: the originals keep their link and group"
    );
    assert_eq!(p.next_id, 19, "C-PASTE-2/G4: 2 clips + link + group");
}

#[test]
fn c_paste_2_created_is_item_order_not_timeline_order() {
    let Base { fx, m, v1, v2, .. } = base();
    let mut doc = fx.doc();
    let out = ok(
        &mut doc,
        place(vec![
            it(v2, pc(media_src(m), 5 * S, S)),
            it(v1, pc(media_src(m), 5 * S, S)),
            it(v1, pc(media_src(m), 0, S)),
        ]),
    );
    assert_eq!(
        out.created,
        vec![10, 11, 12],
        "C-PASTE-2: created = new clip ids in item order"
    );
    assert_eq!(
        spans(doc.project(), v1),
        vec![(12, 0, S), (11, 5 * S, 6 * S)],
        "C-PASTE-2/I1: ids follow item order, storage follows start"
    );
    assert_eq!(
        spans(doc.project(), v2),
        vec![(10, 5 * S, 6 * S)],
        "C-PASTE-2: first item on V2"
    );
}

#[test]
fn c_paste_2_transitions_are_not_copied() {
    let Base {
        mut fx, m, v1, v2, ..
    } = base();
    let p = fx.clip(v1, m, 0, 2 * S, 0); // 10
    let q = fx.clip(v1, m, 2 * S, 2 * S, 0); // 11
    fx.transition(v1, Some(p), Some(q), S); // 12
    let mut doc = fx.doc();
    let v1_before = trk(doc.project(), v1).clone();
    let cp = clip(doc.project(), 10).clone();
    let cq = clip(doc.project(), 11).clone();
    let out = ok(&mut doc, place(vec![it(v2, cp), it(v2, cq)]));
    assert_eq!(out.created, vec![13, 14], "C-PASTE-2: created");
    assert_eq!(
        doc.project().next_id,
        15,
        "C-PASTE-2: no id is spent on a transition"
    );
    assert_eq!(
        spans(doc.project(), v2),
        vec![(13, 0, 2 * S), (14, 2 * S, 4 * S)],
        "C-PASTE-2: both copies placed"
    );
    assert!(
        trk(doc.project(), v2).transitions.is_empty(),
        "C-PASTE-2: transitions are not copied"
    );
    assert_eq!(
        trk(doc.project(), v1),
        &v1_before,
        "C-PASTE-2: the source track, with its transition, is untouched"
    );
}

#[test]
fn c_paste_2_g3_undo_restores_exactly_and_redo_reapplies() {
    let Base {
        mut fx, m, v1, a1, ..
    } = base();
    let x = fx.clip(v1, m, 0, 10 * S, 0); // 10
    fx.effect(x, "blur", &[]); // 11
    let mut doc = fx.doc();
    let before = doc.project().clone();
    let mut a = pc(media_src(m), 4 * S, 2 * S);
    a.effects = vec![fxi(1, "glow", true, &[])];
    a.link = Some(1);
    a.group = Some(2);
    a.label = 5;
    let mut b = pc(media_src(m), 4 * S, 2 * S);
    b.link = Some(1);
    b.group = Some(2);
    ok(&mut doc, over(vec![it(v1, a), it(a1, b)]));
    let after = doc.project().clone();
    assert_ne!(after, before, "C-PASTE-1: the paste changed the project");
    assert_eq!(
        (doc.undo_depth(), doc.redo_depth()),
        (1, 0),
        "C-PASTE-1/G2: exactly one undo entry"
    );
    assert!(doc.undo(), "C-PASTE-1/G3: undo available");
    assert_eq!(
        doc.project(),
        &before,
        "C-PASTE-1/G3: undo restores the project exactly"
    );
    assert_eq!(
        doc.project().next_id,
        12,
        "C-PASTE-1/G4: undo restores next_id"
    );
    assert_eq!(
        (doc.undo_depth(), doc.redo_depth()),
        (0, 1),
        "C-PASTE-1/G3: undone command is redoable"
    );
    assert!(doc.redo(), "C-PASTE-1/G3: redo available");
    assert_eq!(doc.project(), &after, "C-PASTE-1/G3: redo restores exactly");
    assert!(
        doc.project().validate().is_ok(),
        "C-PASTE-1/G2: validate passes"
    );
}

// ============================================================================ C-PASTE-3

#[test]
fn c_paste_3_place_overlap_with_existing_clip() {
    let Base {
        mut fx, m, v1, v2, ..
    } = base();
    fx.clip(v1, m, 2 * S, 2 * S, 0); // 10: [2s, 4s)
    let mut doc = fx.doc();
    for (start, duration) in [
        (3 * S, 2 * S),     // crosses the end
        (0, 2 * S + 1),     // one flick into the start
        (S, 4 * S),         // contains it
        (2 * S + 1, S / 2), // inside it
        (2 * S, 2 * S),     // same range
        (4 * S - 1, S),     // one flick before the end
    ] {
        assert_err!(
            &mut doc,
            place(vec![it(v1, pc(media_src(m), start, duration))]),
            Error::Overlap { track_id: 6 }
        );
    }
    // A good item on another track does not save it; nothing is placed (G1).
    assert_err!(
        &mut doc,
        place(vec![
            it(v2, pc(media_src(m), 0, S)),
            it(v1, pc(media_src(m), 3 * S, S)),
        ]),
        Error::Overlap { track_id: 6 }
    );
    assert_eq!(
        all_clip_ids(doc.project()),
        vec![10],
        "C-PASTE-3/G1: nothing was placed"
    );
}

#[test]
fn c_paste_3_place_touching_and_other_track_ok() {
    let Base {
        mut fx, m, v1, v2, ..
    } = base();
    fx.clip(v1, m, 2 * S, 2 * S, 0); // 10: [2s, 4s)
    let mut doc = fx.doc();
    let x_before = clip(doc.project(), 10).clone();
    let out = ok(
        &mut doc,
        place(vec![
            it(v1, pc(media_src(m), 0, 2 * S)),
            it(v1, pc(media_src(m), 4 * S, S)),
            it(v2, pc(media_src(m), 2 * S, 2 * S)),
        ]),
    );
    assert_eq!(out.created, vec![11, 12, 13], "C-PASTE-2: created");
    assert_eq!(
        spans(doc.project(), v1),
        vec![(11, 0, 2 * S), (10, 2 * S, 4 * S), (12, 4 * S, 5 * S)],
        "C-PASTE-3: half-open ranges that touch do not overlap"
    );
    assert_eq!(
        spans(doc.project(), v2),
        vec![(13, 2 * S, 4 * S)],
        "C-PASTE-3: overlap is per track"
    );
    assert_eq!(
        clip(doc.project(), 10),
        &x_before,
        "C-PASTE-3: place changes no existing clip"
    );
    assert_eq!(doc.project().next_id, 14, "C-PASTE-3/G4: no split ids");
}

#[test]
fn c_paste_3_place_items_overlapping_each_other() {
    let Base { fx, m, v1, v2, .. } = base();
    let mut doc = fx.doc();
    for (s2, d2) in [(S, 2 * S), (0, 2 * S), (2 * S - 1, S), (S / 2, S)] {
        assert_err!(
            &mut doc,
            place(vec![
                it(v1, pc(media_src(m), 0, 2 * S)),
                it(v1, pc(media_src(m), s2, d2)),
            ]),
            Error::Overlap { track_id: 6 }
        );
    }
    // Not adjacent in the item list, on V2.
    assert_err!(
        &mut doc,
        place(vec![
            it(v2, pc(media_src(m), 4 * S, 2 * S)),
            it(v1, pc(media_src(m), 0, S)),
            it(v2, pc(media_src(m), 3 * S, 2 * S)),
        ]),
        Error::Overlap { track_id: 7 }
    );
    // Same range on two tracks, and touching on one track: fine.
    let out = ok(
        &mut doc,
        place(vec![
            it(v1, pc(media_src(m), 0, 2 * S)),
            it(v2, pc(media_src(m), 0, 2 * S)),
            it(v1, pc(media_src(m), 2 * S, S)),
        ]),
    );
    assert_eq!(out.created, vec![10, 11, 12], "C-PASTE-2: created");
    assert_eq!(
        spans(doc.project(), v1),
        vec![(10, 0, 2 * S), (12, 2 * S, 3 * S)],
        "C-PASTE-3: touching items on one track are placed"
    );
}

#[test]
fn c_paste_3_overwrite_items_overlapping_each_other() {
    let Base {
        mut fx, m, v1, v2, ..
    } = base();
    fx.clip(v1, m, 0, 10 * S, 0); // 10
    let mut doc = fx.doc();
    for (s2, d2) in [(5 * S, 2 * S), (4 * S, 2 * S), (6 * S - 1, S)] {
        assert_err!(
            &mut doc,
            over(vec![
                it(v1, pc(media_src(m), 4 * S, 2 * S)),
                it(v1, pc(media_src(m), s2, d2)),
            ]),
            Error::Overlap { track_id: 6 }
        );
    }
    assert_err!(
        &mut doc,
        over(vec![
            it(v2, pc(media_src(m), 20 * S, 2 * S)),
            it(v2, pc(media_src(m), 21 * S, 2 * S)),
        ]),
        Error::Overlap { track_id: 7 }
    );
    assert_eq!(
        spans(doc.project(), v1),
        vec![(10, 0, 10 * S)],
        "C-PASTE-3/G1: nothing was cleared"
    );
}

#[test]
fn c_paste_3_overwrite_touching_items_ok() {
    let Base { mut fx, m, v1, .. } = base();
    fx.clip(v1, m, 0, 10 * S, 0); // 10
    let mut doc = fx.doc();
    let out = ok(
        &mut doc,
        over(vec![
            it(v1, pc(media_src(m), 4 * S, S)),
            it(v1, pc(media_src(m), 5 * S, S)),
        ]),
    );
    assert_eq!(out.created, vec![11, 12], "C-PASTE-2: created");
    assert_eq!(
        spans(doc.project(), v1),
        vec![
            (10, 0, 4 * S),
            (11, 4 * S, 5 * S),
            (12, 5 * S, 6 * S),
            (13, 6 * S, 10 * S)
        ],
        "C-PASTE-3: touching items do not overlap; X is split once (right part id 13)"
    );
    assert_eq!(doc.project().next_id, 14, "C-PASTE-2/G4: one split id");
}

#[test]
fn c_paste_3_overwrite_worked_example() {
    // V1 = X [0, 10s) with id 5, next_id 9.
    let mut fx = Fx::new();
    let m = fx.media_full("M", Some(100 * S), true, true); // 1
    let seq = fx.seq("Main"); // 2
    let v1 = fx.track(seq, TrackKind::Video); // 3
    fx.track(seq, TrackKind::Audio); // 4
    let x = fx.clip(v1, m, 0, 10 * S, S); // 5
    fx.p.next_id = 9;
    assert_eq!(x, 5);
    let mut doc = fx.doc();
    let mut copy = pc(media_src(m), 4 * S, 2 * S);
    copy.source_in = 20 * S;
    let out = ok(&mut doc, over(vec![it(v1, copy.clone())]));
    assert_eq!(out.created, vec![9], "C-PASTE-3: created = [9]");
    let p = doc.project();
    assert_eq!(
        spans(p, v1),
        vec![
            (5, 0, 2_822_400_000),
            (9, 2_822_400_000, 4_233_600_000),
            (10, 4_233_600_000, 7_056_000_000)
        ],
        "C-PASTE-3: X [0,4s), the copy [4s,6s) id 9, X's right part [6s,10s) id 10"
    );
    assert_eq!(p.next_id, 11, "C-PASTE-3/G4: ids 9 and 10 used");
    assert_eq!(
        clip(p, 9),
        &Clip { id: 9, ..copy },
        "C-PASTE-2: the copy equals its clip except the id"
    );
    assert_eq!(
        (clip(p, 5).source_in, clip(p, 5).duration),
        (705_600_000, 2_822_400_000),
        "R5/R3: X is tail-trimmed, source_in stays 1 s"
    );
    assert_eq!(
        (clip(p, 10).source_in, clip(p, 10).duration),
        (4_939_200_000, 2_822_400_000),
        "R5/R4/R2: the right part starts 6 s into X: source_in 1 s + 6 s = 7 s"
    );
    assert_eq!(doc.undo_depth(), 1, "C-PASTE-3/G2: one undo entry");
}

#[test]
fn c_paste_3_overwrite_clears_range_r5() {
    let Base {
        mut fx, m, v1, v2, ..
    } = base();
    fx.clip(v1, m, 0, 3 * S, 0); // 10: A [0, 3s)
    fx.clip(v1, m, 3 * S, S, 0); // 11: B [3s, 4s)
    let c = fx.clip(v1, m, 4 * S, 4 * S, 10 * S); // 12: C [4s, 8s)
    fx.c(c).params.insert(
        "opacity".into(),
        param(
            1.0,
            vec![kf(0, 0.0, Easing::Linear), kf(2 * S, 1.0, Easing::Linear)],
        ),
    );
    fx.clip(v2, m, 0, 10 * S, 0); // 13: Y on V2
    let mut doc = fx.doc();
    let v2_before = trk(doc.project(), v2).clone();
    let out = ok(&mut doc, over(vec![it(v1, pc(media_src(m), 2 * S, 3 * S))]));
    assert_eq!(out.created, vec![14], "C-PASTE-2: created");
    let p = doc.project();
    assert_eq!(
        spans(p, v1),
        vec![(10, 0, 2 * S), (14, 2 * S, 5 * S), (12, 5 * S, 8 * S)],
        "C-PASTE-3/R5: A tail-trimmed, B removed, C head-trimmed"
    );
    assert_eq!(clip(p, 10).source_in, 0, "R3: tail trim keeps source_in");
    assert_eq!(
        clip(p, 12).source_in,
        11 * S,
        "R2: head trim by 1 s moves source_in from 10 s to 11 s"
    );
    assert_eq!(
        clip(p, 12).params["opacity"]
            .keyframes
            .iter()
            .map(|k| k.time)
            .collect::<Vec<_>>(),
        vec![-S, S],
        "R2: head trim by 1 s shifts keyframe times by -1 s"
    );
    assert_eq!(
        trk(p, v2),
        &v2_before,
        "C-PASTE-3: only the item's own track is cleared"
    );
    assert_eq!(p.next_id, 15, "C-PASTE-3/G4: no split, one id used");
}

#[test]
fn c_paste_3_overwrite_clears_every_items_range_on_its_track() {
    let Base {
        mut fx,
        m,
        aud,
        v1,
        v2,
        a1,
        ..
    } = base();
    fx.clip(v1, m, 0, 10 * S, 0); // 10
    fx.clip(a1, aud, 0, 10 * S, 0); // 11
    fx.clip(v2, m, 0, 10 * S, 0); // 12 (no item on V2)
    let mut doc = fx.doc();
    let out = ok(
        &mut doc,
        over(vec![
            it(v1, pc(media_src(m), 4 * S, 2 * S)),
            it(a1, pc(media_src(aud), S, S)),
        ]),
    );
    assert_eq!(out.created, vec![13, 14], "C-PASTE-2: created");
    let p = doc.project();
    assert_eq!(
        spans(p, v1),
        vec![(10, 0, 4 * S), (13, 4 * S, 6 * S), (15, 6 * S, 10 * S)],
        "C-PASTE-3: item 1's range cleared on V1 (split ids in item order)"
    );
    assert_eq!(
        spans(p, a1),
        vec![(11, 0, S), (14, S, 2 * S), (16, 2 * S, 10 * S)],
        "C-PASTE-3: item 2's range cleared on A1 (split ids in item order)"
    );
    assert_eq!(
        (clip(p, 15).source_in, clip(p, 16).source_in),
        (6 * S, 2 * S),
        "R5/R2: right parts read on from where they start"
    );
    assert_eq!(
        spans(p, v2),
        vec![(12, 0, 10 * S)],
        "C-PASTE-3: a track without items is untouched"
    );
    assert_eq!(p.next_id, 17, "C-PASTE-2/G4: 2 clips + 2 split ids");
}

#[test]
fn c_paste_3_overwrite_on_free_space_only_places() {
    let Base { mut fx, m, v1, .. } = base();
    fx.clip(v1, m, 0, 2 * S, 0); // 10
    let mut doc = fx.doc();
    let x_before = clip(doc.project(), 10).clone();
    let out = ok(&mut doc, over(vec![it(v1, pc(media_src(m), 2 * S, 2 * S))]));
    assert_eq!(out.created, vec![11], "C-PASTE-2: created");
    assert_eq!(
        spans(doc.project(), v1),
        vec![(10, 0, 2 * S), (11, 2 * S, 4 * S)],
        "C-PASTE-3: nothing to clear, the copy is placed"
    );
    assert_eq!(
        clip(doc.project(), 10),
        &x_before,
        "C-PASTE-3: a touching clip is not changed"
    );
    assert_eq!(doc.project().next_id, 12, "C-PASTE-3/G4: one id used");
}

#[test]
fn c_paste_3_g8_overwrite_prunes_transition_and_undo_restores_it() {
    let Base { mut fx, m, v1, .. } = base();
    let p = fx.clip(v1, m, 0, 2 * S, 0); // 10
    let q = fx.clip(v1, m, 2 * S, 2 * S, 0); // 11
    fx.transition(v1, Some(p), Some(q), S); // 12
    let mut doc = fx.doc();
    let before = doc.project().clone();
    let out = ok(&mut doc, over(vec![it(v1, pc(media_src(m), 2 * S, 2 * S))]));
    assert_eq!(out.created, vec![13], "C-PASTE-2: created");
    assert_eq!(
        spans(doc.project(), v1),
        vec![(10, 0, 2 * S), (13, 2 * S, 4 * S)],
        "C-PASTE-3/R5: Q is removed, the copy takes its place"
    );
    assert!(
        trk(doc.project(), v1).transitions.is_empty(),
        "C-PASTE-3/G8: the transition onto the removed clip is pruned"
    );
    assert!(doc.undo(), "C-PASTE-3/G3: undo available");
    assert_eq!(
        doc.project(),
        &before,
        "C-PASTE-3/G3: undo brings Q and the transition back"
    );
}

#[test]
fn c_paste_3_g1_overwrite_failing_late_changes_nothing() {
    let Base { mut fx, m, v1, .. } = base();
    fx.clip(v1, m, 0, 10 * S, 0); // 10
    let mut doc = fx.doc();
    let mut out_of_range = pc(media_src(m), 20 * S, 2 * S);
    out_of_range.source_in = 99 * S;
    // `err` checks G1: project ==, undo and redo depths unchanged.
    assert_err!(
        &mut doc,
        over(vec![
            it(v1, pc(media_src(m), 4 * S, 2 * S)),
            it(v1, out_of_range),
        ]),
        Error::SourceOutOfRange { .. }
    );
    assert_eq!(
        spans(doc.project(), v1),
        vec![(10, 0, 10 * S)],
        "C-PASTE-3/G1: the first item's range was not cleared"
    );
    assert_eq!(
        (doc.project().next_id, doc.undo_depth()),
        (11, 0),
        "C-PASTE-3/G1: next_id and undo depth unchanged"
    );
}

#[test]
fn c_paste_3_place_and_overwrite_agree_on_free_space() {
    let Base { fx, m, v1, a1, .. } = base();
    let mut a = pc(media_src(m), S, 2 * S);
    a.link = Some(4);
    a.label = 2;
    let mut b = pc(media_src(m), S, 2 * S);
    b.link = Some(4);
    let items = vec![it(v1, a), it(a1, b)];
    let mut d1 = fx.clone_doc();
    let mut d2 = fx.doc();
    let o1 = ok(&mut d1, place(items.clone()));
    let o2 = ok(&mut d2, over(items));
    assert_eq!(o1.created, vec![10, 11], "C-PASTE-2: created");
    assert_eq!(o1, o2, "C-PASTE-3: same outcome when nothing is in the way");
    assert_eq!(
        d1.project(),
        d2.project(),
        "C-PASTE-3: overwrite with nothing to clear equals place"
    );
    assert_eq!(
        (clip(d1.project(), 10).link, clip(d1.project(), 11).link),
        (Some(12), Some(12)),
        "C-PASTE-2: fresh shared link id"
    );
}

#[test]
fn c_paste_2_empty_params_map_is_kept_empty() {
    let Base { fx, m, v1, .. } = base();
    let mut doc = fx.doc();
    let c = pc(media_src(m), 0, S);
    ok(&mut doc, place(vec![it(v1, c)]));
    assert_eq!(
        clip(doc.project(), 10).params,
        BTreeMap::new(),
        "C-PASTE-2: the copy equals its clip: no params are invented"
    );
}
