//! Media, sequences, tracks, markers, effect definitions (C-MEDIA-*, C-SEQ-*, C-TRACK-*,
//! C-MARK-1, C-FX-1) plus the G5/G6/G7 cases they raise.
#[macro_use]
mod common;
use common::*;

fn spec(name: &str, duration: Option<Time>, video: bool, audio: bool) -> MediaSpec {
    MediaSpec {
        name: name.into(),
        path: format!("/x/{name}.mov"),
        duration,
        video: video.then_some(VideoInfo {
            width: 640,
            height: 480,
            fps: FPS24,
        }),
        audio: audio.then_some(AudioInfo {
            sample_rate: 44100,
            channels: 1,
        }),
    }
}

// --- C-MEDIA-1 ---

#[test]
fn c_media_1_add_appends_and_allocates_next_id() {
    let mut doc = base().fx.doc();
    let out = ok(
        &mut doc,
        Command::AddMedia {
            spec: spec("New", Some(5 * S), true, false),
        },
    );
    assert_eq!(out.created, vec![10]);
    assert_eq!(doc.project().next_id, 11);
    let last = doc.project().media.last().unwrap();
    assert_eq!(last.id, 10);
    assert_eq!(last.spec, spec("New", Some(5 * S), true, false));
    assert_eq!(doc.project().media.len(), 5);
    assert_eq!(
        doc.project().media[0].id,
        1,
        "existing media keep their place"
    );
}

#[test]
fn c_media_1_still_and_audio_only_are_fine() {
    let mut doc = base().fx.doc();
    ok(
        &mut doc,
        Command::AddMedia {
            spec: spec("Png", None, true, false),
        },
    );
    ok(
        &mut doc,
        Command::AddMedia {
            spec: spec("Wav", Some(1), false, true),
        },
    );
    assert_eq!(doc.project().media.len(), 6);
}

#[test]
fn c_media_1_errors() {
    let mut doc = base().fx.doc();
    assert_err!(
        &mut doc,
        Command::AddMedia {
            spec: spec("Z", Some(0), true, true)
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::AddMedia {
            spec: spec("N", Some(-S), true, true)
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::AddMedia {
            spec: spec("E", Some(S), false, false)
        },
        Error::InvalidArgument(_)
    );
}

// --- C-MEDIA-2 ---

#[test]
fn c_media_2_remove_unused() {
    let Base { fx, vid, .. } = base();
    let mut doc = fx.doc();
    ok(&mut doc, Command::RemoveMedia { media_id: vid });
    assert!(doc.project().media.iter().all(|m| m.id != vid));
    assert_eq!(doc.project().media.len(), 3);
}

#[test]
fn c_media_2_in_use_and_not_found() {
    let Base { mut fx, m, seq, .. } = base();
    let inner = fx.seq("Inner");
    let t = fx.track(inner, TrackKind::Audio);
    fx.clip(t, m, 0, S, 0); // used only inside a different sequence
    let _ = seq;
    let mut doc = fx.doc();
    assert_err!(&mut doc, Command::RemoveMedia { media_id: m }, Error::InUse { id, .. } if id == m);
    assert_err!(
        &mut doc,
        Command::RemoveMedia { media_id: 999 },
        Error::NotFound { id: 999, .. }
    );
}

// --- C-MEDIA-3 ---

#[test]
fn c_media_3_set_fields() {
    let Base { fx, m, .. } = base();
    let mut doc = fx.doc();
    ok(
        &mut doc,
        Command::SetMedia {
            media_id: m,
            name: None,
            path: Some("/new/place.mp4".into()),
        },
    );
    let md = &doc.project().media[0];
    assert_eq!(
        (md.spec.name.as_str(), md.spec.path.as_str()),
        ("M", "/new/place.mp4")
    );
    assert_eq!(md.spec.duration, Some(100 * S));
    ok(
        &mut doc,
        Command::SetMedia {
            media_id: m,
            name: Some("Renamed".into()),
            path: None,
        },
    );
    let md = &doc.project().media[0];
    assert_eq!(
        (md.spec.name.as_str(), md.spec.path.as_str()),
        ("Renamed", "/new/place.mp4")
    );
    assert_err!(
        &mut doc,
        Command::SetMedia {
            media_id: 999,
            name: Some("x".into()),
            path: None
        },
        Error::NotFound { id: 999, .. }
    );
}

// --- C-SEQ-1 ---

fn add_seq(w: u32, h: u32, fps: Rational, sr: u32, v: u32, a: u32) -> Command {
    Command::AddSequence {
        name: "Edit".into(),
        width: w,
        height: h,
        fps,
        sample_rate: sr,
        video_tracks: v,
        audio_tracks: a,
    }
}

#[test]
fn c_seq_1_creates_sequence_then_tracks() {
    let mut doc = Document::new(Project::new("p"));
    let fps = Rational {
        num: 30000,
        den: 1001,
    };
    let out = ok(&mut doc, add_seq(1280, 720, fps, 48000, 2, 1));
    assert_eq!(out.created, vec![1, 2, 3, 4]);
    assert_eq!(doc.project().next_id, 5);
    let s = &doc.project().sequences[0];
    assert_eq!(
        (
            s.id,
            s.name.as_str(),
            s.width,
            s.height,
            s.fps,
            s.sample_rate
        ),
        (1, "Edit", 1280, 720, fps, 48000)
    );
    assert!(s.markers.is_empty());
    let tracks: Vec<_> = s
        .tracks
        .iter()
        .map(|t| (t.id, t.kind, t.name.as_str()))
        .collect();
    assert_eq!(
        tracks,
        vec![
            (2, TrackKind::Video, "V1"),
            (3, TrackKind::Video, "V2"),
            (4, TrackKind::Audio, "A1")
        ]
    );
    for t in &s.tracks {
        assert!(
            !t.locked
                && !t.muted
                && !t.solo
                && !t.hidden
                && t.clips.is_empty()
                && t.transitions.is_empty()
        );
    }
}

#[test]
fn c_seq_1_zero_tracks_is_fine() {
    let mut doc = Document::new(Project::new("p"));
    let out = ok(&mut doc, add_seq(10, 10, FPS24, 8000, 0, 0));
    assert_eq!(out.created, vec![1]);
    assert!(doc.project().sequences[0].tracks.is_empty());
}

#[test]
fn c_seq_1_zero_fields_rejected() {
    let mut doc = Document::new(Project::new("p"));
    for cmd in [
        add_seq(0, 720, FPS24, 48000, 1, 1),
        add_seq(1280, 0, FPS24, 48000, 1, 1),
        add_seq(1280, 720, FPS24, 0, 1, 1),
        add_seq(1280, 720, Rational { num: 0, den: 1 }, 48000, 1, 1),
        add_seq(1280, 720, Rational { num: 24, den: 0 }, 48000, 1, 1),
    ] {
        assert_err!(&mut doc, cmd, Error::InvalidArgument(_));
    }
}

// --- C-SEQ-2 ---

#[test]
fn c_seq_2_remove_sequence() {
    let Base {
        mut fx, m, seq, v1, ..
    } = base();
    fx.clip(v1, m, 0, S, 0);
    let other = fx.seq("Other");
    let mut doc = fx.doc();
    ok(&mut doc, Command::RemoveSequence { sequence_id: seq });
    assert_eq!(
        doc.project()
            .sequences
            .iter()
            .map(|s| s.id)
            .collect::<Vec<_>>(),
        vec![other]
    );
    assert_err!(&mut doc, Command::RemoveSequence { sequence_id: seq }, Error::NotFound { id, .. } if id == seq);
}

#[test]
fn c_seq_2_in_use_by_another_sequence() {
    let Base {
        mut fx, seq, v1, ..
    } = base();
    let inner = fx.seq("Inner");
    fx.clip_src(v1, seq_src(inner), 0, S, 0);
    let mut doc = fx.doc();
    assert_err!(&mut doc, Command::RemoveSequence { sequence_id: inner }, Error::InUse { id, .. } if id == inner);
    // the outer one is not nested anywhere
    ok(&mut doc, Command::RemoveSequence { sequence_id: seq });
}

// --- C-SEQ-3 ---

fn set_seq(id: Id) -> Command {
    Command::SetSequence {
        sequence_id: id,
        name: None,
        width: None,
        height: None,
        fps: None,
        sample_rate: None,
    }
}

#[test]
fn c_seq_3_set_fields() {
    let Base { fx, seq, .. } = base();
    let mut doc = fx.doc();
    let fps = Rational { num: 25, den: 1 };
    ok(
        &mut doc,
        Command::SetSequence {
            sequence_id: seq,
            name: Some("Cut".into()),
            width: Some(3840),
            height: None,
            fps: Some(fps),
            sample_rate: None,
        },
    );
    let s = sq(doc.project(), seq);
    assert_eq!(
        (s.name.as_str(), s.width, s.height, s.fps, s.sample_rate),
        ("Cut", 3840, 1080, fps, 48000)
    );
}

#[test]
fn c_seq_3_zero_values_rejected() {
    let Base { fx, seq, .. } = base();
    let mut doc = fx.doc();
    for (w, h, fps, sr) in [
        (Some(0), None, None, None),
        (None, Some(0), None, None),
        (None, None, Some(Rational { num: 0, den: 1 }), None),
        (None, None, Some(Rational { num: 1, den: 0 }), None),
        (None, None, None, Some(0)),
    ] {
        assert_err!(
            &mut doc,
            Command::SetSequence {
                sequence_id: seq,
                name: None,
                width: w,
                height: h,
                fps,
                sample_rate: sr
            },
            Error::InvalidArgument(_)
        );
    }
    assert_err!(&mut doc, set_seq(999), Error::NotFound { id: 999, .. });
}

// --- C-TRACK-1 ---

fn add_track(seq: Id, kind: TrackKind, name: Option<&str>, index: Option<usize>) -> Command {
    Command::AddTrack {
        sequence_id: seq,
        kind,
        name: name.map(Into::into),
        index,
    }
}

fn track_names(p: &Project, seq: Id) -> Vec<(Id, String)> {
    sq(p, seq)
        .tracks
        .iter()
        .map(|t| (t.id, t.name.clone()))
        .collect()
}

#[test]
fn c_track_1_append_and_default_name() {
    let Base { fx, seq, .. } = base();
    let mut doc = fx.doc();
    let out = ok(&mut doc, add_track(seq, TrackKind::Audio, None, None));
    assert_eq!(out.created, vec![10]);
    let t = sq(doc.project(), seq).tracks.last().unwrap();
    assert_eq!(
        (t.id, t.kind, t.name.as_str()),
        (10, TrackKind::Audio, "A3")
    );
    assert!(!t.locked && !t.muted && !t.solo && !t.hidden);
}

#[test]
fn c_track_1_insert_at_index_counts_kind_after_adding() {
    let Base {
        fx,
        seq,
        v1,
        v2,
        a1,
        a2,
        ..
    } = base();
    let mut doc = fx.doc();
    ok(&mut doc, add_track(seq, TrackKind::Video, None, Some(0)));
    assert_eq!(
        track_names(doc.project(), seq),
        vec![
            (10, "V3".to_string()),
            (v1, "V1".to_string()),
            (v2, "V2".to_string()),
            (a1, "A1".to_string()),
            (a2, "A2".to_string())
        ]
    );
    // index == len appends
    ok(
        &mut doc,
        add_track(seq, TrackKind::Video, Some("Top"), Some(5)),
    );
    assert_eq!(track_names(doc.project(), seq)[5], (11, "Top".to_string()));
}

#[test]
fn c_track_1_first_track_of_kind_is_1() {
    let mut fx = Fx::new();
    let s = fx.seq("S");
    let mut doc = fx.doc();
    ok(&mut doc, add_track(s, TrackKind::Audio, None, None));
    ok(&mut doc, add_track(s, TrackKind::Video, None, None));
    assert_eq!(
        track_names(doc.project(), s),
        vec![(2, "A1".to_string()), (3, "V1".to_string())]
    );
}

#[test]
fn c_track_1_errors() {
    let Base { fx, seq, v1, .. } = base();
    let mut doc = fx.doc();
    assert_err!(
        &mut doc,
        add_track(seq, TrackKind::Video, None, Some(5)),
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        add_track(999, TrackKind::Video, None, None),
        Error::NotFound { id: 999, .. }
    );
    assert_err!(
        &mut doc,
        add_track(v1, TrackKind::Video, None, None),
        Error::NotFound { .. }
    );
}

// --- C-TRACK-2 (and G5 for track commands) ---

#[test]
fn c_track_2_remove_track_with_clips_and_transitions() {
    let Base {
        mut fx,
        m,
        seq,
        v1,
        v2,
        a1,
        a2,
        ..
    } = base();
    let c = fx.clip(v1, m, 0, S, 0);
    fx.transition(v1, Some(c), None, S / 2);
    let mut doc = fx.doc();
    ok(&mut doc, Command::RemoveTrack { track_id: v1 });
    assert_eq!(
        sq(doc.project(), seq)
            .tracks
            .iter()
            .map(|t| t.id)
            .collect::<Vec<_>>(),
        vec![v2, a1, a2]
    );
    assert!(find_clip(doc.project(), c).is_none());
    assert_err!(&mut doc, Command::RemoveTrack { track_id: v1 }, Error::NotFound { id, .. } if id == v1);
}

#[test]
fn c_track_2_g5_remove_locked_track_fails() {
    let Base { mut fx, a2, .. } = base();
    fx.lock(a2);
    let mut doc = fx.doc();
    assert_err!(&mut doc, Command::RemoveTrack { track_id: a2 }, Error::TrackLocked { track_id } if track_id == a2);
}

#[test]
fn c_track_2_move_track() {
    let Base {
        mut fx,
        seq,
        v1,
        v2,
        a1,
        a2,
        ..
    } = base();
    fx.lock(v1); // G5: MoveTrack works on locked tracks
    let mut doc = fx.doc();
    ok(
        &mut doc,
        Command::MoveTrack {
            track_id: v1,
            index: 3,
        },
    );
    let ids = |d: &Document| {
        sq(d.project(), seq)
            .tracks
            .iter()
            .map(|t| t.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&doc), vec![v2, a1, a2, v1]);
    ok(
        &mut doc,
        Command::MoveTrack {
            track_id: a2,
            index: 0,
        },
    );
    assert_eq!(ids(&doc), vec![a2, v2, a1, v1]);
    assert_err!(
        &mut doc,
        Command::MoveTrack {
            track_id: a2,
            index: 4
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::MoveTrack {
            track_id: 999,
            index: 0
        },
        Error::NotFound { id: 999, .. }
    );
}

#[test]
fn c_track_2_set_track_fields_even_when_locked() {
    let Base { mut fx, v1, .. } = base();
    fx.lock(v1);
    let mut doc = fx.doc();
    ok(
        &mut doc,
        Command::SetTrack {
            track_id: v1,
            name: Some("Main".into()),
            locked: None,
            muted: Some(true),
            solo: None,
            hidden: Some(true),
        },
    );
    let t = trk(doc.project(), v1);
    assert_eq!(
        (t.name.as_str(), t.locked, t.muted, t.solo, t.hidden),
        ("Main", true, true, false, true)
    );
    ok(
        &mut doc,
        Command::SetTrack {
            track_id: v1,
            name: None,
            locked: Some(false),
            muted: None,
            solo: Some(true),
            hidden: None,
        },
    );
    let t = trk(doc.project(), v1);
    assert_eq!(
        (t.name.as_str(), t.locked, t.muted, t.solo, t.hidden),
        ("Main", false, true, true, true)
    );
    assert_err!(
        &mut doc,
        Command::SetTrack {
            track_id: 999,
            name: None,
            locked: Some(true),
            muted: None,
            solo: None,
            hidden: None
        },
        Error::NotFound { id: 999, .. }
    );
}

// --- C-MARK-1 ---

fn marker(seq: Id, time: Time, duration: Time, name: &str) -> Command {
    Command::AddMarker {
        sequence_id: seq,
        time,
        duration,
        name: name.into(),
        note: "n".into(),
    }
}

#[test]
fn c_mark_1_sorted_insert_after_equal_times() {
    let Base { mut fx, seq, .. } = base();
    let a = fx.marker(seq, S, 0); // 10
    let b = fx.marker(seq, 3 * S, 0); // 11
    let mut doc = fx.doc();
    let out = ok(&mut doc, marker(seq, S, 2 * S, "same-time"));
    assert_eq!(out.created, vec![12]);
    ok(&mut doc, marker(seq, 0, 0, "first"));
    ok(&mut doc, marker(seq, 5 * S, 0, "last"));
    let ms: Vec<_> = sq(doc.project(), seq)
        .markers
        .iter()
        .map(|m| (m.id, m.time))
        .collect();
    assert_eq!(ms, vec![(13, 0), (a, S), (12, S), (b, 3 * S), (14, 5 * S)]);
    let m12 = sq(doc.project(), seq)
        .markers
        .iter()
        .find(|m| m.id == 12)
        .unwrap();
    assert_eq!(
        (m12.duration, m12.name.as_str(), m12.note.as_str()),
        (2 * S, "same-time", "n")
    );
}

#[test]
fn c_mark_1_remove() {
    let Base { mut fx, seq, .. } = base();
    let a = fx.marker(seq, S, 0);
    let b = fx.marker(seq, 2 * S, 0);
    let mut doc = fx.doc();
    ok(&mut doc, Command::RemoveMarker { marker_id: a });
    assert_eq!(
        sq(doc.project(), seq)
            .markers
            .iter()
            .map(|m| m.id)
            .collect::<Vec<_>>(),
        vec![b]
    );
    assert_err!(&mut doc, Command::RemoveMarker { marker_id: a }, Error::NotFound { id, .. } if id == a);
}

#[test]
fn c_mark_1_g6_bad_values() {
    let Base { fx, seq, .. } = base();
    let mut doc = fx.doc();
    assert_err!(&mut doc, marker(seq, -1, 0, ""), Error::InvalidArgument(_));
    assert_err!(&mut doc, marker(seq, 0, -1, ""), Error::InvalidArgument(_));
    assert_err!(
        &mut doc,
        marker(999, 0, 0, ""),
        Error::NotFound { id: 999, .. }
    );
    ok(&mut doc, marker(seq, 0, 0, "zero duration is allowed"));
}

// --- C-FX-1 ---

fn def(name: &str, wgsl: &str) -> EffectDef {
    EffectDef {
        name: name.into(),
        kind: EffectKind::Filter,
        wgsl: wgsl.into(),
        params: vec![ParamDef {
            name: "amount".into(),
            default: 0.5,
            min: 0.0,
            max: 1.0,
        }],
    }
}

#[test]
fn c_fx_1_define_appends_or_replaces_in_place() {
    let mut doc = base().fx.doc();
    ok(&mut doc, Command::DefineEffect { def: def("a", "1") });
    ok(&mut doc, Command::DefineEffect { def: def("b", "2") });
    ok(&mut doc, Command::DefineEffect { def: def("a", "3") });
    assert_eq!(doc.project().effects, vec![def("a", "3"), def("b", "2")]);
    ok(&mut doc, Command::RemoveEffectDef { name: "a".into() });
    assert_eq!(doc.project().effects, vec![def("b", "2")]);
}

#[test]
fn c_fx_1_errors() {
    let mut doc = base().fx.doc();
    assert_err!(
        &mut doc,
        Command::DefineEffect { def: def("", "x") },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::RemoveEffectDef {
            name: "nope".into()
        },
        Error::InvalidArgument(_)
    );
}

#[test]
fn c_fx_1_core_does_not_check_effect_names() {
    let Base { mut fx, m, v1, .. } = base();
    let c = fx.clip(v1, m, 0, S, 0);
    let mut doc = fx.doc();
    ok(
        &mut doc,
        Command::DefineEffect {
            def: def("glow", ""),
        },
    );
    ok(
        &mut doc,
        Command::AddEffect {
            clip_id: c,
            effect: "glow".into(),
            index: None,
            params: Default::default(),
        },
    );
    // removing a def that a clip uses is allowed
    ok(
        &mut doc,
        Command::RemoveEffectDef {
            name: "glow".into(),
        },
    );
    ok(
        &mut doc,
        Command::AddEffect {
            clip_id: c,
            effect: "never-defined".into(),
            index: None,
            params: Default::default(),
        },
    );
    assert_eq!(clip(doc.project(), c).effects.len(), 2);
}

// --- zero / negative fields, one at a time, inside a batch ---
// The final validate() would also reject the bad value, so each bad command is followed in the
// batch by one that repairs it: only the command's own check (C-BATCH-1: first Err fails the
// batch) can make the batch fail.

#[test]
fn c_seq_1_c_batch_1_each_zero_field_fails_even_if_repaired_later() {
    let fix = |fps: Option<Rational>, w: Option<u32>, h: Option<u32>, sr: Option<u32>| {
        Command::SetSequence {
            sequence_id: 1, // the sequence the batch's AddSequence creates
            name: None,
            width: w,
            height: h,
            fps,
            sample_rate: sr,
        }
    };
    for (bad, repair) in [
        (
            add_seq(0, 720, FPS24, 48000, 0, 0),
            fix(None, Some(1280), None, None),
        ),
        (
            add_seq(1280, 0, FPS24, 48000, 0, 0),
            fix(None, None, Some(720), None),
        ),
        (
            add_seq(1280, 720, FPS24, 0, 0, 0),
            fix(None, None, None, Some(48000)),
        ),
        (
            add_seq(1280, 720, Rational { num: 0, den: 1 }, 48000, 0, 0),
            fix(Some(FPS24), None, None, None),
        ),
        (
            add_seq(1280, 720, Rational { num: 24, den: 0 }, 48000, 0, 0),
            fix(Some(FPS24), None, None, None),
        ),
    ] {
        let mut doc = Document::new(Project::new("p"));
        assert_err!(
            &mut doc,
            Command::Batch {
                commands: vec![bad, repair]
            },
            Error::InvalidArgument(_)
        );
    }
}

#[test]
fn c_seq_3_c_batch_1_each_zero_value_fails_even_if_repaired_later() {
    let Base { fx, seq, .. } = base();
    let mut doc = fx.doc();
    let set = |w: Option<u32>, h: Option<u32>, fps: Option<Rational>, sr: Option<u32>| {
        Command::SetSequence {
            sequence_id: seq,
            name: None,
            width: w,
            height: h,
            fps,
            sample_rate: sr,
        }
    };
    for (bad, repair) in [
        (
            set(Some(0), None, None, None),
            set(Some(1920), None, None, None),
        ),
        (
            set(None, Some(0), None, None),
            set(None, Some(1080), None, None),
        ),
        (
            set(None, None, None, Some(0)),
            set(None, None, None, Some(48000)),
        ),
        (
            set(None, None, Some(Rational { num: 0, den: 1 }), None),
            set(None, None, Some(FPS24), None),
        ),
        (
            set(None, None, Some(Rational { num: 24, den: 0 }), None),
            set(None, None, Some(FPS24), None),
        ),
    ] {
        assert_err!(
            &mut doc,
            Command::Batch {
                commands: vec![bad, repair]
            },
            Error::InvalidArgument(_)
        );
    }
}

#[test]
fn c_mark_1_g6_c_batch_1_negative_time_or_duration_fails_even_if_removed_later() {
    let Base { fx, seq, .. } = base();
    let mut doc = fx.doc();
    // base() has next_id 10, so the marker would get id 10
    for (time, duration) in [(-1, 0), (0, -1)] {
        assert_err!(
            &mut doc,
            Command::Batch {
                commands: vec![
                    marker(seq, time, duration, ""),
                    Command::RemoveMarker { marker_id: 10 }
                ]
            },
            Error::InvalidArgument(_)
        );
    }
}
