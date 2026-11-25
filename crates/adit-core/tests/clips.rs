//! Clip placement and range edits: C-ADD-*, C-REM-1, C-RIP-*, C-LIFT-1, C-EXT-1, C-GAP-1,
//! C-SPLIT-*, C-MOVE-1, C-SET-1, C-LINK-1, with R1, R4, R5, R6, R7, G5, G6, G7, G8.
#[macro_use]
mod common;
use common::*;

// ============================================================================ C-ADD-1

#[test]
fn c_add_1_defaults() {
    let Base { fx, m, v1, .. } = base();
    let mut doc = fx.doc();
    let out = ok(
        &mut doc,
        AddB {
            source_in: 7 * S,
            ..addb(v1, media_src(m), 2 * S, 3 * S)
        }
        .cmd(),
    );
    assert_eq!(out.created, vec![10]);
    assert_eq!(doc.project().next_id, 11);
    let c = clip(doc.project(), 10);
    assert_eq!(track_of(doc.project(), 10), v1);
    assert_eq!((c.start, c.duration, c.source_in), (2 * S, 3 * S, 7 * S));
    assert_eq!(c.source, media_src(m));
    assert_eq!(c.name, "M");
    assert_eq!(c.speed, 1.0);
    assert!(!c.reverse && c.enabled);
    assert_eq!(c.blend, BlendMode::Normal);
    assert_eq!(c.audio_filter, "");
    assert!(c.params.is_empty() && c.effects.is_empty());
    assert_eq!((c.link, c.group), (None, None));
}

#[test]
fn c_add_1_default_names() {
    let Base { mut fx, m, v1, .. } = base();
    let inner = fx.seq("Inner"); // 10
    let it = fx.track(inner, TrackKind::Video); // 11
    fx.clip(it, m, 0, S, 0); // 12
    let mut doc = fx.doc();
    ok(&mut doc, add(v1, seq_src(inner), 0, S)); // 13
    ok(&mut doc, add(v1, text_src("hi"), S, S)); // 14
    ok(&mut doc, add(v1, solid_src(), 2 * S, S)); // 15
    ok(&mut doc, add(v1, ClipSource::Adjustment, 3 * S, S)); // 16
    ok(
        &mut doc,
        AddB {
            name: Some("custom".into()),
            ..addb(v1, media_src(m), 4 * S, S)
        }
        .cmd(),
    ); // 17
    let names: Vec<_> = trk(doc.project(), v1)
        .clips
        .iter()
        .map(|c| (c.id, c.name.as_str()))
        .collect();
    assert_eq!(
        names,
        vec![
            (13, "Inner"),
            (14, "Text"),
            (15, "Solid"),
            (16, "Adjustment"),
            (17, "custom")
        ]
    );
}

#[test]
fn c_add_1_r7_text_source_in_stays_as_is() {
    let Base { fx, v1, .. } = base();
    let mut doc = fx.doc();
    ok(
        &mut doc,
        AddB {
            source_in: 5 * S,
            ..addb(v1, text_src("t"), 0, S)
        }
        .cmd(),
    );
    assert_eq!(clip(doc.project(), 10).source_in, 5 * S);
}

#[test]
fn c_add_1_i4_kind_mismatch() {
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
    for cmd in [
        add_m(v1, aud, 0, S),
        add_m(a1, vid, 0, S),
        add_m(a1, still, 0, S),
        add(a1, text_src("t"), 0, S),
        add(a1, solid_src(), 0, S),
        add(a1, ClipSource::Adjustment, 0, S),
    ] {
        assert_err!(&mut doc, cmd, Error::KindMismatch(_));
    }
    ok(&mut doc, add_m(a1, aud, 0, S));
    ok(&mut doc, add_m(v1, vid, 0, S));
}

#[test]
fn c_add_1_sequence_on_audio_track_ok() {
    let Base { mut fx, m, a1, .. } = base();
    let inner = fx.seq("Inner");
    let it = fx.track(inner, TrackKind::Audio);
    fx.clip(it, m, 0, S, 0);
    let mut doc = fx.doc();
    ok(&mut doc, add(a1, seq_src(inner), 0, S));
}

#[test]
fn c_add_1_nest_cycle_direct_and_indirect() {
    let Base {
        mut fx, seq, v1, ..
    } = base();
    let inner = fx.seq("Inner");
    let it = fx.track(inner, TrackKind::Video);
    let deeper = fx.seq("Deeper");
    let dt = fx.track(deeper, TrackKind::Video);
    fx.clip_src(it, seq_src(seq), 0, S, 0); // Inner contains Main
    let mut doc = fx.doc();
    assert_err!(
        &mut doc,
        add(v1, seq_src(seq), 0, S),
        Error::NestCycle { .. }
    );
    assert_err!(
        &mut doc,
        add(v1, seq_src(inner), 0, S),
        Error::NestCycle { .. }
    );
    // Deeper -> Inner -> Main is fine, but then Main -> Deeper closes a cycle
    ok(&mut doc, add(dt, seq_src(inner), 0, S));
    assert_err!(
        &mut doc,
        add(v1, seq_src(deeper), 0, S),
        Error::NestCycle { .. }
    );
}

#[test]
fn c_add_1_r7_source_bounds_and_slack() {
    let Base { fx, m, v1, .. } = base();
    let mut doc = fx.doc();
    // M is 100 s long
    ok(
        &mut doc,
        AddB {
            source_in: SOURCE_SLACK,
            ..addb(v1, media_src(m), 0, 100 * S)
        }
        .cmd(),
    );
    assert_err!(
        &mut doc,
        AddB {
            source_in: SOURCE_SLACK + 1,
            ..addb(v1, media_src(m), 200 * S, 100 * S)
        }
        .cmd(),
        Error::SourceOutOfRange { .. }
    );
    assert_err!(
        &mut doc,
        AddB {
            source_in: 90 * S,
            ..addb(v1, media_src(m), 200 * S, 11 * S)
        }
        .cmd(),
        Error::SourceOutOfRange { .. }
    );
    // ambiguity (see report): negative source_in reported as SourceOutOfRange per R7
    assert_err!(
        &mut doc,
        AddB {
            source_in: -1,
            ..addb(v1, media_src(m), 300 * S, S)
        }
        .cmd(),
        Error::SourceOutOfRange { .. }
    );
}

#[test]
fn c_add_1_r7_stills_and_sequences_unbounded() {
    let Base {
        mut fx,
        m,
        still,
        v1,
        v2,
        ..
    } = base();
    let inner = fx.seq("Inner");
    let it = fx.track(inner, TrackKind::Video);
    fx.clip(it, m, 0, S, 0); // inner is 1 s long
    let mut doc = fx.doc();
    ok(
        &mut doc,
        AddB {
            source_in: 1000 * S,
            ..addb(v1, media_src(still), 0, 5000 * S)
        }
        .cmd(),
    );
    ok(
        &mut doc,
        AddB {
            source_in: 10 * S,
            ..addb(v2, seq_src(inner), 0, 50 * S)
        }
        .cmd(),
    );
}

#[test]
fn c_add_1_g5_g6_g7() {
    let Base {
        mut fx, m, v1, v2, ..
    } = base();
    fx.lock(v2);
    let mut doc = fx.doc();
    assert_err!(&mut doc, add_m(v2, m, 0, S), Error::TrackLocked { track_id } if track_id == v2);
    assert_err!(&mut doc, add_m(v1, m, 0, 0), Error::InvalidArgument(_));
    assert_err!(&mut doc, add_m(v1, m, 0, -S), Error::InvalidArgument(_));
    assert_err!(&mut doc, add_m(v1, m, -1, S), Error::InvalidArgument(_));
    assert_err!(
        &mut doc,
        add_m(999, m, 0, S),
        Error::NotFound { id: 999, .. }
    );
    assert_err!(
        &mut doc,
        add_m(v1, 999, 0, S),
        Error::NotFound { id: 999, .. }
    );
    assert_err!(
        &mut doc,
        add(v1, seq_src(998), 0, S),
        Error::NotFound { id: 998, .. }
    );
}

// ============================================================================ C-ADD-2

#[test]
fn c_add_2_linked_pair_id_order() {
    let Base { fx, m, v1, a1, .. } = base();
    let mut doc = fx.doc();
    let out = ok(
        &mut doc,
        AddB {
            linked_track_id: Some(a1),
            source_in: 3 * S,
            ..addb(v1, media_src(m), S, 2 * S)
        }
        .cmd(),
    );
    assert_eq!(out.created, vec![10, 11]);
    assert_eq!(doc.project().next_id, 13);
    let (v, a) = (clip(doc.project(), 10), clip(doc.project(), 11));
    assert_eq!(
        (track_of(doc.project(), 10), track_of(doc.project(), 11)),
        (v1, a1)
    );
    for c in [v, a] {
        assert_eq!(
            (c.start, c.duration, c.source_in, c.link),
            (S, 2 * S, 3 * S, Some(12))
        );
        assert_eq!(c.source, media_src(m));
    }
}

#[test]
fn c_add_2_unlinked_has_no_link() {
    let Base { fx, m, v1, .. } = base();
    let mut doc = fx.doc();
    ok(&mut doc, add_m(v1, m, 0, S));
    assert_eq!(clip(doc.project(), 10).link, None);
    assert_eq!(doc.project().next_id, 11);
}

#[test]
fn c_add_2_linked_sequence_source() {
    let Base {
        mut fx, m, v1, a1, ..
    } = base();
    let inner = fx.seq("Inner"); // 10
    let it = fx.track(inner, TrackKind::Video); // 11
    fx.clip(it, m, 0, S, 0); // 12
    let mut doc = fx.doc();
    let out = ok(
        &mut doc,
        AddB {
            linked_track_id: Some(a1),
            ..addb(v1, seq_src(inner), 0, S)
        }
        .cmd(),
    );
    assert_eq!(out.created, vec![13, 14]);
    assert_eq!(clip(doc.project(), 14).source, seq_src(inner));
    assert_eq!(clip(doc.project(), 13).link, Some(15));
}

#[test]
fn c_add_2_kind_mismatch_cases() {
    let Base {
        mut fx,
        m,
        vid,
        aud,
        v1,
        v2,
        a1,
        a2,
        ..
    } = base();
    let other = fx.seq("Other");
    let oa = fx.track(other, TrackKind::Audio);
    let mut doc = fx.doc();
    let linked = |t: Id, l: Id, src: ClipSource| {
        AddB {
            linked_track_id: Some(l),
            ..addb(t, src, 0, S)
        }
        .cmd()
    };
    for cmd in [
        linked(v1, a1, media_src(vid)), // no audio
        linked(v1, a1, media_src(aud)), // no video
        linked(v1, a1, text_src("t")),  // not media/sequence
        linked(v1, a1, solid_src()),
        linked(a1, a2, media_src(m)), // track_id must be video
        linked(v1, v2, media_src(m)), // linked track must be audio
        linked(v1, oa, media_src(m)), // other sequence
    ] {
        assert_err!(&mut doc, cmd, Error::KindMismatch(_));
    }
}

#[test]
fn c_add_2_c_add_4_linked_overlap_on_audio_side() {
    let Base {
        mut fx, m, v1, a1, ..
    } = base();
    fx.clip(a1, m, S, S, 0);
    let mut doc = fx.doc();
    assert_err!(
        &mut doc,
        AddB { linked_track_id: Some(a1), ..addb(v1, media_src(m), 0, 2 * S) }.cmd(),
        Error::Overlap { track_id } if track_id == a1
    );
}

#[test]
fn c_add_2_linked_track_locked() {
    let Base {
        mut fx, m, v1, a1, ..
    } = base();
    fx.lock(a1);
    let mut doc = fx.doc();
    assert_err!(
        &mut doc,
        AddB { linked_track_id: Some(a1), ..addb(v1, media_src(m), 0, S) }.cmd(),
        Error::TrackLocked { track_id } if track_id == a1
    );
}

// ============================================================================ C-ADD-4

#[test]
fn c_add_4_place_overlap_and_touching() {
    let Base { mut fx, m, v1, .. } = base();
    fx.clip(v1, m, 2 * S, 2 * S, 0); // [2s, 4s)
    let mut doc = fx.doc();
    assert_err!(&mut doc, add_m(v1, m, S, 2 * S), Error::Overlap { track_id } if track_id == v1);
    assert_err!(&mut doc, add_m(v1, m, 3 * S, 2 * S), Error::Overlap { track_id } if track_id == v1);
    assert_err!(&mut doc, add_m(v1, m, 2 * S + 1, 1), Error::Overlap { .. });
    assert_err!(&mut doc, add_m(v1, m, 0, 10 * S), Error::Overlap { .. });
    ok(&mut doc, add_m(v1, m, 0, 2 * S)); // ends exactly where X starts
    ok(&mut doc, add_m(v1, m, 4 * S, S)); // starts exactly at X's end
    let starts: Vec<_> = spans(doc.project(), v1)
        .iter()
        .map(|s| (s.1, s.2))
        .collect();
    assert_eq!(starts, vec![(0, 2 * S), (2 * S, 4 * S), (4 * S, 5 * S)]);
}

// ============================================================================ C-ADD-5 / R5

#[test]
fn c_add_5_overwrite_clears_range_on_target_only() {
    let Base {
        mut fx, m, v1, v2, ..
    } = base();
    let a = fx.clip(v1, m, 0, 2 * S, 0);
    let b = fx.clip(v1, m, 3 * S, S, 0);
    let c = fx.clip(v1, m, 5 * S, 5 * S, 0);
    let o = fx.clip(v2, m, 2 * S, S, 0);
    let mut doc = fx.doc();
    let n = doc.project().next_id;
    ok(
        &mut doc,
        AddB {
            mode: PlaceMode::Overwrite,
            ..addb(v1, media_src(m), S, 5 * S)
        }
        .cmd(),
    );
    let p = doc.project();
    assert_eq!(
        spans(p, v1),
        vec![(a, 0, S), (n, S, 6 * S), (c, 6 * S, 10 * S)]
    );
    assert!(find_clip(p, b).is_none());
    assert_eq!(clip(p, a).source_in, 0, "tail trim keeps source_in");
    assert_eq!(clip(p, c).source_in, S, "head trim by 1 s moves source_in");
    assert_eq!(spans(p, v2), vec![(o, 2 * S, 3 * S)]);
}

#[test]
fn c_add_5_overwrite_inside_one_clip_splits_it() {
    let Base { mut fx, m, v1, .. } = base();
    let x = fx.clip(v1, m, 0, 10 * S, 0);
    let mut doc = fx.doc();
    let n = doc.project().next_id;
    let out = ok(
        &mut doc,
        AddB {
            mode: PlaceMode::Overwrite,
            ..addb(v1, media_src(m), 4 * S, 2 * S)
        }
        .cmd(),
    );
    assert_eq!(out.created, vec![n], "C-ADD-3");
    let p = doc.project();
    assert_eq!(
        spans(p, v1),
        vec![(x, 0, 4 * S), (n, 4 * S, 6 * S), (n + 1, 6 * S, 10 * S)]
    );
    assert_eq!(clip(p, n + 1).source_in, 6 * S);
    assert_eq!(p.next_id, n + 2);
}

#[test]
fn c_add_5_overwrite_exact_fit_and_boundaries() {
    let Base { mut fx, m, v1, .. } = base();
    let a = fx.clip(v1, m, 0, 2 * S, 0);
    let b = fx.clip(v1, m, 2 * S, 2 * S, 0);
    let c = fx.clip(v1, m, 4 * S, 2 * S, 0);
    let mut doc = fx.doc();
    let n = doc.project().next_id;
    ok(
        &mut doc,
        AddB {
            mode: PlaceMode::Overwrite,
            ..addb(v1, media_src(m), 2 * S, 2 * S)
        }
        .cmd(),
    );
    let p = doc.project();
    assert_eq!(
        spans(p, v1),
        vec![(a, 0, 2 * S), (n, 2 * S, 4 * S), (c, 4 * S, 6 * S)]
    );
    assert!(find_clip(p, b).is_none());
    assert_eq!((clip(p, a).source_in, clip(p, c).source_in), (0, 0));
}

#[test]
fn c_add_5_r2_overwrite_head_trims_reversed_and_fast_clips() {
    let Base {
        mut fx, m, v1, v2, ..
    } = base();
    let r = fx.clip(v1, m, 5 * S, 5 * S, 0);
    fx.c(r).reverse = true;
    let f = fx.clip(v2, m, 5 * S, 5 * S, 0);
    fx.c(f).speed = 2.0;
    let mut doc = fx.doc();
    ok(
        &mut doc,
        AddB {
            mode: PlaceMode::Overwrite,
            ..addb(v1, media_src(m), 4 * S, 2 * S)
        }
        .cmd(),
    );
    ok(
        &mut doc,
        AddB {
            mode: PlaceMode::Overwrite,
            ..addb(v2, media_src(m), 4 * S, 2 * S)
        }
        .cmd(),
    );
    let p = doc.project();
    assert_eq!(
        (clip(p, r).start, clip(p, r).duration, clip(p, r).source_in),
        (6 * S, 4 * S, 0)
    );
    assert_eq!(
        (clip(p, f).start, clip(p, f).duration, clip(p, f).source_in),
        (6 * S, 4 * S, 2 * S)
    );
}

#[test]
fn c_add_5_r3_overwrite_tail_trims_reversed_clip() {
    let Base { mut fx, m, v1, .. } = base();
    let r = fx.clip(v1, m, 0, 5 * S, S);
    fx.c(r).reverse = true;
    let mut doc = fx.doc();
    ok(
        &mut doc,
        AddB {
            mode: PlaceMode::Overwrite,
            ..addb(v1, media_src(m), 3 * S, 4 * S)
        }
        .cmd(),
    );
    let c = clip(doc.project(), r);
    assert_eq!((c.start, c.duration, c.source_in), (0, 3 * S, 3 * S));
}

#[test]
fn c_add_5_linked_overwrite_clears_both_targets() {
    let Base {
        mut fx, m, v1, a1, ..
    } = base();
    let x = fx.clip(v1, m, 0, 4 * S, 0);
    let y = fx.clip(a1, m, S, 4 * S, 0);
    let mut doc = fx.doc();
    let n = doc.project().next_id;
    ok(
        &mut doc,
        AddB {
            mode: PlaceMode::Overwrite,
            linked_track_id: Some(a1),
            ..addb(v1, media_src(m), 2 * S, 2 * S)
        }
        .cmd(),
    );
    let p = doc.project();
    assert_eq!(spans(p, v1), vec![(x, 0, 2 * S), (n, 2 * S, 4 * S)]);
    assert_eq!(spans(p, a1), vec![(y, S, 2 * S), (n + 1, 2 * S, 4 * S)]);
}

#[test]
fn c_add_5_overwrite_locked_target() {
    let Base { mut fx, m, v1, .. } = base();
    fx.clip(v1, m, 0, 4 * S, 0);
    fx.lock(v1);
    let mut doc = fx.doc();
    assert_err!(
        &mut doc,
        AddB { mode: PlaceMode::Overwrite, ..addb(v1, media_src(m), 2 * S, 2 * S) }.cmd(),
        Error::TrackLocked { track_id } if track_id == v1
    );
}

// ============================================================================ C-ADD-3 / C-ADD-6 / R1

#[test]
fn c_add_6_spec_example_and_c_add_3_ids() {
    let Base { mut fx, m, v1, .. } = base();
    let x = fx.clip(v1, m, 0, 10 * S, S); // 10
    let mut doc = fx.doc();
    assert_eq!(doc.project().next_id, 11);
    let out = ok(
        &mut doc,
        AddB {
            mode: PlaceMode::Insert,
            source_in: 50 * S,
            ..addb(v1, media_src(m), 4 * S, 2 * S)
        }
        .cmd(),
    );
    assert_eq!(
        out.created,
        vec![11],
        "C-ADD-3: new clip id is next_id before the command"
    );
    let p = doc.project();
    assert_eq!(
        spans(p, v1),
        vec![(x, 0, 4 * S), (11, 4 * S, 6 * S), (12, 6 * S, 12 * S)]
    );
    assert_eq!(clip(p, x).source_in, S);
    assert_eq!(clip(p, 11).source_in, 50 * S);
    assert_eq!(clip(p, 12).source_in, 5 * S, "X.source_in + 4s");
    assert_eq!(clip(p, 12).name, clip(p, x).name);
    assert_eq!(p.next_id, 13);
}

#[test]
fn c_add_6_r1_insert_splits_and_shifts_all_unlocked_tracks() {
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
    let x = fx.clip(v1, m, 0, 10 * S, 0);
    let y = fx.clip(v2, m, 3 * S, 2 * S, 0); // crosses 4 s
    let z = fx.clip(v2, m, 7 * S, S, 0);
    let w = fx.clip(a1, m, 0, S, 0);
    let w2 = fx.clip(a1, m, 4 * S, S, 0); // starts exactly at T: shifts, not split
    let l = fx.clip(a2, m, 3 * S, 3 * S, 0);
    fx.lock(a2);
    let mk = fx.marker(seq, 5 * S, 0);
    let mut doc = fx.doc();
    let n = doc.project().next_id;
    let out = ok(
        &mut doc,
        AddB {
            mode: PlaceMode::Insert,
            ..addb(v1, media_src(m), 4 * S, 2 * S)
        }
        .cmd(),
    );
    assert_eq!(out.created, vec![n]);
    let p = doc.project();
    let v1s = spans(p, v1);
    let v2s = spans(p, v2);
    assert_eq!(v1s.len(), 3);
    assert_eq!((v1s[0], v1s[1]), ((x, 0, 4 * S), (n, 4 * S, 6 * S)));
    assert_eq!((v1s[2].1, v1s[2].2), (6 * S, 12 * S));
    assert_eq!(v2s.len(), 3);
    assert_eq!(v2s[0], (y, 3 * S, 4 * S));
    assert_eq!((v2s[1].1, v2s[1].2), (6 * S, 7 * S));
    assert_eq!(v2s[2], (z, 9 * S, 10 * S));
    assert_eq!(clip(p, v2s[1].0).source_in, S);
    let mut rights = vec![v1s[2].0, v2s[1].0];
    rights.sort();
    assert_eq!(
        rights,
        vec![n + 1, n + 2],
        "split ids come after the new clip id"
    );
    assert_eq!(p.next_id, n + 3);
    assert_eq!(spans(p, a1), vec![(w, 0, S), (w2, 6 * S, 7 * S)]);
    assert_eq!(
        spans(p, a2),
        vec![(l, 3 * S, 6 * S)],
        "locked track: no split, no shift"
    );
    assert_eq!(
        sq(p, seq)
            .markers
            .iter()
            .map(|k| (k.id, k.time))
            .collect::<Vec<_>>(),
        vec![(mk, 5 * S)]
    );
}

#[test]
fn c_add_6_linked_insert_ids() {
    let Base {
        mut fx, m, v1, a1, ..
    } = base();
    fx.clip(v1, m, 0, 10 * S, 0);
    let mut doc = fx.doc();
    let n = doc.project().next_id;
    let out = ok(
        &mut doc,
        AddB {
            mode: PlaceMode::Insert,
            linked_track_id: Some(a1),
            ..addb(v1, media_src(m), 4 * S, 2 * S)
        }
        .cmd(),
    );
    assert_eq!(out.created, vec![n, n + 1]);
    let p = doc.project();
    assert_eq!(clip(p, n).link, Some(n + 2));
    let v1s = spans(p, v1);
    assert_eq!(v1s[2], (n + 3, 6 * S, 12 * S));
    assert_eq!(spans(p, a1), vec![(n + 1, 4 * S, 6 * S)]);
}

#[test]
fn c_add_6_insert_on_locked_target() {
    let Base { mut fx, m, v1, .. } = base();
    fx.lock(v1);
    let mut doc = fx.doc();
    assert_err!(
        &mut doc,
        AddB { mode: PlaceMode::Insert, ..addb(v1, media_src(m), 0, S) }.cmd(),
        Error::TrackLocked { track_id } if track_id == v1
    );
}

// ============================================================================ C-REM-1 / R6 / G8

fn linked_pair() -> (Base, Id, Id, Id, Id) {
    let mut b = base();
    let (m, v1, a1) = (b.m, b.v1, b.a1);
    let x = b.fx.clip(v1, m, 0, 2 * S, 0);
    let xa = b.fx.clip(a1, m, 0, 2 * S, 0);
    let l = b.fx.link(&[x, xa]);
    let y = b.fx.clip(v1, m, 3 * S, 2 * S, 0);
    (b, x, xa, l, y)
}

#[test]
fn c_rem_1_removes_linked_set_others_stay() {
    let (b, x, xa, _, y) = linked_pair();
    let (v1, a1) = (b.v1, b.a1);
    let mut doc = b.fx.doc();
    ok(
        &mut doc,
        Command::RemoveClips {
            clip_ids: vec![x],
            include_linked: true,
        },
    );
    let p = doc.project();
    assert!(find_clip(p, x).is_none() && find_clip(p, xa).is_none());
    assert_eq!(spans(p, v1), vec![(y, 3 * S, 5 * S)]);
    assert!(trk(p, a1).clips.is_empty());
}

#[test]
fn c_rem_1_without_linked() {
    let (b, x, xa, _, _) = linked_pair();
    let a1 = b.a1;
    let mut doc = b.fx.doc();
    ok(
        &mut doc,
        Command::RemoveClips {
            clip_ids: vec![x],
            include_linked: false,
        },
    );
    let p = doc.project();
    assert!(find_clip(p, x).is_none());
    assert_eq!(spans(p, a1), vec![(xa, 0, 2 * S)]);
}

#[test]
fn c_rem_1_r6_linked_set_is_per_sequence() {
    let (mut b, x, xa, l, _) = linked_pair();
    let m = b.m;
    let other = b.fx.seq("Other");
    let ot = b.fx.track(other, TrackKind::Video);
    let far = b.fx.clip(ot, m, 0, S, 0);
    b.fx.c(far).link = Some(l);
    let mut doc = b.fx.doc();
    ok(
        &mut doc,
        Command::RemoveClips {
            clip_ids: vec![x],
            include_linked: true,
        },
    );
    assert!(find_clip(doc.project(), xa).is_none());
    assert!(
        find_clip(doc.project(), far).is_some(),
        "same link id in another sequence is not in the set"
    );
}

#[test]
fn c_rem_1_g8_prunes_transitions_of_removed_clip() {
    let Base { mut fx, m, v1, .. } = base();
    let p0 = fx.clip(v1, m, 0, 2 * S, 0);
    let q = fx.clip(v1, m, 2 * S, 2 * S, 0);
    fx.transition(v1, Some(p0), Some(q), S);
    let tq = fx.transition(v1, Some(q), None, S);
    let mut doc = fx.doc();
    ok(
        &mut doc,
        Command::RemoveClips {
            clip_ids: vec![p0],
            include_linked: true,
        },
    );
    let ts: Vec<_> = trk(doc.project(), v1)
        .transitions
        .iter()
        .map(|t| t.id)
        .collect();
    assert_eq!(ts, vec![tq]);
}

#[test]
fn c_rem_1_errors() {
    let (mut b, x, _, _, y) = linked_pair();
    let a1 = b.a1;
    b.fx.lock(a1);
    let mut doc = b.fx.doc();
    assert_err!(
        &mut doc,
        Command::RemoveClips {
            clip_ids: vec![],
            include_linked: true
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::RemoveClips {
            clip_ids: vec![y, 999],
            include_linked: true
        },
        Error::NotFound { id: 999, .. }
    );
    assert_err!(
        &mut doc,
        Command::RemoveClips { clip_ids: vec![x], include_linked: true },
        Error::TrackLocked { track_id } if track_id == a1
    );
    ok(
        &mut doc,
        Command::RemoveClips {
            clip_ids: vec![x],
            include_linked: false,
        },
    );
}

// ============================================================================ C-RIP-1 / C-RIP-2 / R1

struct Rip {
    b: Base,
    p: Id,
    q: Id,
    r: Id,
    m_: Id,
    l: Id,
}

fn rip(m_start: Time) -> Rip {
    let mut b = base();
    let (m, v1, a1, a2) = (b.m, b.v1, b.a1, b.a2);
    let p = b.fx.clip(v1, m, 0, 2 * S, 0);
    let q = b.fx.clip(v1, m, 2 * S, 3 * S, 0);
    let r = b.fx.clip(v1, m, 5 * S, 4 * S, 0);
    let m_ = b.fx.clip(a1, m, m_start, 9 * S - m_start, 0);
    let l = b.fx.clip(a2, m, 5 * S, 4 * S, 0);
    b.fx.lock(a2);
    Rip { b, p, q, r, m_, l }
}

#[test]
fn c_rip_2_spec_example() {
    let Rip { b, p, q, r, m_, l } = rip(5 * S);
    let (v1, a1, a2) = (b.v1, b.a1, b.a2);
    let mut doc = b.fx.doc();
    ok(
        &mut doc,
        Command::RippleDeleteClips {
            clip_ids: vec![q],
            include_linked: true,
        },
    );
    let pr = doc.project();
    assert_eq!(spans(pr, v1), vec![(p, 0, 2 * S), (r, 2 * S, 6 * S)]);
    assert_eq!(spans(pr, a1), vec![(m_, 2 * S, 6 * S)]);
    assert_eq!(spans(pr, a2), vec![(l, 5 * S, 9 * S)]);
}

#[test]
fn c_rip_2_blocked_example() {
    let Rip { b, q, .. } = rip(S);
    let mut doc = b.fx.doc();
    assert_err!(
        &mut doc,
        Command::RippleDeleteClips {
            clip_ids: vec![q],
            include_linked: true
        },
        Error::RippleBlocked { .. }
    );
}

#[test]
fn c_rip_1_touching_ranges_merge() {
    let Base { mut fx, m, v1, .. } = base();
    let q = fx.clip(v1, m, 2 * S, 3 * S, 0);
    let r = fx.clip(v1, m, 5 * S, 4 * S, 0);
    let s_ = fx.clip(v1, m, 9 * S, S, 0);
    let mut doc = fx.doc();
    ok(
        &mut doc,
        Command::RippleDeleteClips {
            clip_ids: vec![r, q],
            include_linked: true,
        },
    );
    assert_eq!(spans(doc.project(), v1), vec![(s_, 2 * S, 3 * S)]);
}

#[test]
fn c_rip_1_separate_ranges_latest_first() {
    let Base {
        mut fx, m, v1, v2, ..
    } = base();
    let p = fx.clip(v1, m, 0, 2 * S, 0);
    let q = fx.clip(v1, m, 2 * S, 2 * S, 0);
    let r = fx.clip(v1, m, 4 * S, 2 * S, 0);
    let s_ = fx.clip(v1, m, 6 * S, 2 * S, 0);
    let o = fx.clip(v2, m, 7 * S, S, 0);
    let mut doc = fx.doc();
    ok(
        &mut doc,
        Command::RippleDeleteClips {
            clip_ids: vec![p, r],
            include_linked: true,
        },
    );
    assert_eq!(
        spans(doc.project(), v1),
        vec![(q, 0, 2 * S), (s_, 2 * S, 4 * S)]
    );
    assert_eq!(spans(doc.project(), v2), vec![(o, 3 * S, 4 * S)]);
}

#[test]
fn c_rip_1_linked_set_and_include_linked_false() {
    let Rip {
        mut b, p, q, r, m_, ..
    } = rip(5 * S);
    let (m, v1, a1) = (b.m, b.v1, b.a1);
    let qa = b.fx.clip(a1, m, 2 * S, 3 * S, 0);
    b.fx.link(&[q, qa]);
    let mut doc = b.fx.clone_doc();
    // without the partner the partner sits in [2s, 5s) and blocks
    assert_err!(
        &mut doc,
        Command::RippleDeleteClips {
            clip_ids: vec![q],
            include_linked: false
        },
        Error::RippleBlocked { .. }
    );
    ok(
        &mut doc,
        Command::RippleDeleteClips {
            clip_ids: vec![q],
            include_linked: true,
        },
    );
    let pr = doc.project();
    assert!(find_clip(pr, qa).is_none());
    assert_eq!(spans(pr, v1), vec![(p, 0, 2 * S), (r, 2 * S, 6 * S)]);
    assert_eq!(spans(pr, a1), vec![(m_, 2 * S, 6 * S)]);
}

#[test]
fn c_rip_1_errors() {
    let Rip { mut b, q, l, .. } = rip(5 * S);
    let m = b.m;
    let other = b.fx.seq("Other");
    let ot = b.fx.track(other, TrackKind::Video);
    let far = b.fx.clip(ot, m, 0, S, 0);
    let mut doc = b.fx.doc();
    assert_err!(
        &mut doc,
        Command::RippleDeleteClips {
            clip_ids: vec![q, far],
            include_linked: true
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::RippleDeleteClips {
            clip_ids: vec![],
            include_linked: true
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::RippleDeleteClips {
            clip_ids: vec![l],
            include_linked: true
        },
        Error::TrackLocked { .. }
    );
    assert_err!(
        &mut doc,
        Command::RippleDeleteClips {
            clip_ids: vec![999],
            include_linked: true
        },
        Error::NotFound { id: 999, .. }
    );
}

// ============================================================================ C-LIFT-1 / R5

fn lift(seq: Id, start: Time, end: Time, track_ids: Option<Vec<Id>>) -> Command {
    Command::LiftRange {
        sequence_id: seq,
        start,
        end,
        track_ids,
    }
}

#[test]
fn c_lift_1_r5_cases() {
    let Base {
        mut fx, m, seq, v1, ..
    } = base();
    let a = fx.clip(v1, m, 0, 4 * S, 0);
    let b_ = fx.clip(v1, m, 5 * S, S, 0);
    let c = fx.clip(v1, m, 6 * S, 4 * S, 0);
    let d = fx.clip(v1, m, 10 * S, S, 0);
    let mut doc = fx.doc();
    ok(&mut doc, lift(seq, 3 * S, 7 * S, None));
    let p = doc.project();
    assert_eq!(
        spans(p, v1),
        vec![(a, 0, 3 * S), (c, 7 * S, 10 * S), (d, 10 * S, 11 * S)]
    );
    assert!(find_clip(p, b_).is_none());
    assert_eq!(
        (
            clip(p, a).source_in,
            clip(p, c).source_in,
            clip(p, d).source_in
        ),
        (0, S, 0)
    );
}

#[test]
fn c_lift_1_r5_half_open_boundaries() {
    let Base {
        mut fx, m, seq, v1, ..
    } = base();
    let a = fx.clip(v1, m, 0, 3 * S, 0);
    let b_ = fx.clip(v1, m, 3 * S, 4 * S, 0);
    let c = fx.clip(v1, m, 7 * S, 2 * S, 0);
    let mut doc = fx.doc();
    ok(&mut doc, lift(seq, 3 * S, 7 * S, None));
    let p = doc.project();
    assert_eq!(spans(p, v1), vec![(a, 0, 3 * S), (c, 7 * S, 9 * S)]);
    assert!(find_clip(p, b_).is_none());
    assert_eq!((clip(p, a).source_in, clip(p, c).source_in), (0, 0));
}

#[test]
fn c_lift_1_r5_split_containing_clip() {
    let Base {
        mut fx, m, seq, v1, ..
    } = base();
    let x = fx.clip(v1, m, 0, 10 * S, S);
    let mut doc = fx.doc();
    let n = doc.project().next_id;
    ok(&mut doc, lift(seq, 3 * S, 7 * S, None));
    let p = doc.project();
    assert_eq!(spans(p, v1), vec![(x, 0, 3 * S), (n, 7 * S, 10 * S)]);
    assert_eq!((clip(p, x).source_in, clip(p, n).source_in), (S, 8 * S));
    assert_eq!(p.next_id, n + 1);
}

#[test]
fn c_lift_1_r5_split_reversed_containing_clip() {
    let Base {
        mut fx, m, seq, v1, ..
    } = base();
    let x = fx.clip(v1, m, 0, 10 * S, 0);
    fx.c(x).reverse = true;
    let mut doc = fx.doc();
    let n = doc.project().next_id;
    ok(&mut doc, lift(seq, 3 * S, 7 * S, None));
    let p = doc.project();
    assert_eq!(spans(p, v1), vec![(x, 0, 3 * S), (n, 7 * S, 10 * S)]);
    assert_eq!(clip(p, x).source_in, 7 * S, "R3 reversed tail trim by 7s");
    assert_eq!(
        clip(p, n).source_in,
        0,
        "R2 reversed head trims keep source_in"
    );
    assert!(clip(p, n).reverse);
    // the same source frames are shown at the same sequence times (Q3)
    assert_eq!(clip(p, x).source_time(S), 9 * S);
    assert_eq!(clip(p, n).source_time(8 * S), 2 * S);
}

#[test]
fn c_lift_1_r2_speed_head_trim() {
    let Base {
        mut fx, m, seq, v1, ..
    } = base();
    let c = fx.clip(v1, m, 6 * S, 4 * S, 0);
    fx.c(c).speed = 2.0;
    let mut doc = fx.doc();
    ok(&mut doc, lift(seq, 3 * S, 7 * S, None));
    let cl = clip(doc.project(), c);
    assert_eq!((cl.start, cl.duration, cl.source_in), (7 * S, 3 * S, 2 * S));
}

#[test]
fn c_lift_1_r2_head_trim_moves_keyframes() {
    let Base {
        mut fx, m, seq, v1, ..
    } = base();
    let c = fx.clip(v1, m, 6 * S, 4 * S, 0);
    fx.c(c).params.insert(
        "opacity".into(),
        param(1.0, vec![kf(2 * S, 0.5, Easing::Linear)]),
    );
    fx.effect(
        c,
        "blur",
        &[(
            "amount",
            param(
                0.0,
                vec![kf(0, 1.0, Easing::Linear), kf(4 * S, 2.0, Easing::Hold)],
            ),
        )],
    );
    let mut doc = fx.doc();
    ok(&mut doc, lift(seq, 3 * S, 7 * S, None));
    let cl = clip(doc.project(), c);
    assert_eq!(
        cl.params["opacity"].keyframes,
        vec![kf(S, 0.5, Easing::Linear)]
    );
    assert_eq!(
        cl.effects[0].params["amount"].keyframes,
        vec![kf(-S, 1.0, Easing::Linear), kf(3 * S, 2.0, Easing::Hold)]
    );
}

#[test]
fn c_lift_1_track_selection_and_errors() {
    let Base {
        mut fx,
        m,
        seq,
        v1,
        v2,
        a2,
        ..
    } = base();
    let x = fx.clip(v1, m, 0, 4 * S, 0);
    let y = fx.clip(v2, m, 0, 4 * S, 0);
    let l = fx.clip(a2, m, 0, 4 * S, 0);
    fx.lock(a2);
    let other = fx.seq("Other");
    let ot = fx.track(other, TrackKind::Video);
    let mut doc = fx.doc();
    assert_err!(
        &mut doc,
        lift(seq, 2 * S, 2 * S, None),
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        lift(seq, 3 * S, 2 * S, None),
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        lift(seq, -S, 2 * S, None),
        Error::InvalidArgument(_)
    );
    assert_err!(&mut doc, lift(seq, S, 2 * S, Some(vec![v1, a2])), Error::TrackLocked { track_id } if track_id == a2);
    assert_err!(
        &mut doc,
        lift(seq, S, 2 * S, Some(vec![ot])),
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        lift(999, S, 2 * S, None),
        Error::NotFound { id: 999, .. }
    );
    // listed: only V2
    ok(&mut doc, lift(seq, 2 * S, 4 * S, Some(vec![v2])));
    assert_eq!(spans(doc.project(), v2), vec![(y, 0, 2 * S)]);
    assert_eq!(spans(doc.project(), v1), vec![(x, 0, 4 * S)]);
    // None: all unlocked; the locked track is skipped silently (G5)
    ok(&mut doc, lift(seq, 3 * S, 4 * S, None));
    assert_eq!(spans(doc.project(), v1), vec![(x, 0, 3 * S)]);
    assert_eq!(spans(doc.project(), a2), vec![(l, 0, 4 * S)]);
}

// ============================================================================ C-EXT-1

fn extract(seq: Id, start: Time, end: Time, track_ids: Option<Vec<Id>>) -> Command {
    Command::ExtractRange {
        sequence_id: seq,
        start,
        end,
        track_ids,
    }
}

#[test]
fn c_ext_1_all_unlocked_tracks() {
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
    let x = fx.clip(v1, m, 0, 10 * S, 0);
    let y = fx.clip(v2, m, 5 * S, S, 0);
    let z = fx.clip(a1, m, S, 2 * S, 0);
    let l = fx.clip(a2, m, 0, S, 0); // locked, but no material after start
    fx.lock(a2);
    let mut doc = fx.doc();
    let n = doc.project().next_id;
    ok(&mut doc, extract(seq, 2 * S, 4 * S, None));
    let p = doc.project();
    assert_eq!(spans(p, v1), vec![(x, 0, 2 * S), (n, 2 * S, 8 * S)]);
    assert_eq!(clip(p, n).source_in, 4 * S);
    assert_eq!(spans(p, v2), vec![(y, 3 * S, 4 * S)]);
    assert_eq!(spans(p, a1), vec![(z, S, 2 * S)]);
    assert_eq!(spans(p, a2), vec![(l, 0, S)]);
    assert_eq!(sq(p, seq).duration(), 8 * S, "10s - (4s - 2s)");
}

#[test]
fn c_ext_1_listed_tracks_only() {
    let Base {
        mut fx,
        m,
        seq,
        v1,
        v2,
        ..
    } = base();
    let x = fx.clip(v1, m, 0, 10 * S, 0);
    let y = fx.clip(v2, m, 5 * S, S, 0);
    let mut doc = fx.doc();
    let n = doc.project().next_id;
    ok(&mut doc, extract(seq, 2 * S, 4 * S, Some(vec![v1])));
    let p = doc.project();
    assert_eq!(spans(p, v1), vec![(x, 0, 2 * S), (n, 2 * S, 8 * S)]);
    assert_eq!(spans(p, v2), vec![(y, 5 * S, 6 * S)]);
}

#[test]
fn c_ext_1_locked_track_does_not_move() {
    let Base {
        mut fx,
        m,
        seq,
        v1,
        a2,
        ..
    } = base();
    let x = fx.clip(v1, m, 0, 3 * S, 0);
    let y = fx.clip(v1, m, 5 * S, S, 0);
    let l = fx.clip(a2, m, 5 * S, S, 0);
    fx.lock(a2);
    let mut doc = fx.doc();
    ok(&mut doc, extract(seq, 3 * S, 4 * S, None));
    assert_eq!(
        spans(doc.project(), v1),
        vec![(x, 0, 3 * S), (y, 4 * S, 5 * S)]
    );
    assert_eq!(spans(doc.project(), a2), vec![(l, 5 * S, 6 * S)]);
}

#[test]
fn c_ext_1_duration_when_range_passes_the_end() {
    let Base {
        mut fx, m, seq, v1, ..
    } = base();
    let a = fx.clip(v1, m, 0, S, 0);
    let b_ = fx.clip(v1, m, 2 * S, 3 * S, 0);
    let mut doc = fx.doc();
    ok(&mut doc, extract(seq, 3 * S, 8 * S, None));
    assert_eq!(
        spans(doc.project(), v1),
        vec![(a, 0, S), (b_, 2 * S, 3 * S)]
    );
    assert_eq!(
        sq(doc.project(), seq).duration(),
        3 * S,
        "5s - (min(5s, 8s) - 3s)"
    );
}

#[test]
fn c_ext_1_errors() {
    let Base {
        mut fx,
        m,
        seq,
        v1,
        a2,
        ..
    } = base();
    fx.clip(v1, m, 0, 4 * S, 0);
    fx.lock(a2);
    let mut doc = fx.doc();
    assert_err!(
        &mut doc,
        extract(seq, 2 * S, 2 * S, None),
        Error::InvalidArgument(_)
    );
    assert_err!(&mut doc, extract(seq, S, 2 * S, Some(vec![a2])), Error::TrackLocked { track_id } if track_id == a2);
}

// ============================================================================ C-GAP-1

fn gap_fx() -> (Base, [Id; 6]) {
    let mut b = base();
    let (m, v1, v2, a2) = (b.m, b.v1, b.v2, b.a2);
    let a = b.fx.clip(v1, m, 0, 2 * S, 0);
    let bb = b.fx.clip(v1, m, 5 * S, 2 * S, 0);
    let c = b.fx.clip(v1, m, 8 * S, S, 0);
    let d = b.fx.clip(v2, m, S, S, 0); // ends exactly at a = 2s
    let e = b.fx.clip(v2, m, 5 * S, S, 0); // starts exactly at b = 5s
    let l = b.fx.clip(a2, m, 3 * S, S, 0); // inside the gap but locked
    b.fx.lock(a2);
    (b, [a, bb, c, d, e, l])
}

#[test]
fn c_gap_1_closes_gap_across_unlocked_tracks() {
    for time in [3 * S, 2 * S, 5 * S - 1] {
        let (b, [a, bb, c, d, e, l]) = gap_fx();
        let (v1, v2, a2) = (b.v1, b.v2, b.a2);
        let mut doc = b.fx.doc();
        ok(&mut doc, Command::CloseGap { track_id: v1, time });
        let p = doc.project();
        assert_eq!(
            spans(p, v1),
            vec![(a, 0, 2 * S), (bb, 2 * S, 4 * S), (c, 5 * S, 6 * S)],
            "time {time}"
        );
        assert_eq!(spans(p, v2), vec![(d, S, 2 * S), (e, 2 * S, 3 * S)]);
        assert_eq!(spans(p, a2), vec![(l, 3 * S, 4 * S)]);
    }
}

#[test]
fn c_gap_1_gap_from_zero() {
    let Base { mut fx, m, v1, .. } = base();
    let a = fx.clip(v1, m, S, S, 0);
    let mut doc = fx.doc();
    ok(
        &mut doc,
        Command::CloseGap {
            track_id: v1,
            time: 0,
        },
    );
    assert_eq!(spans(doc.project(), v1), vec![(a, 0, S)]);
}

#[test]
fn c_gap_1_errors() {
    let (b, _) = gap_fx();
    let v1 = b.v1;
    let mut doc = b.fx.doc();
    assert_err!(
        &mut doc,
        Command::CloseGap {
            track_id: v1,
            time: S
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::CloseGap {
            track_id: v1,
            time: 5 * S
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::CloseGap {
            track_id: v1,
            time: 10 * S
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::CloseGap {
            track_id: v1,
            time: 9 * S
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::CloseGap {
            track_id: v1,
            time: -S
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::CloseGap {
            track_id: 999,
            time: 3 * S
        },
        Error::NotFound { id: 999, .. }
    );
}

#[test]
fn c_gap_1_blocked_by_other_unlocked_track() {
    let (mut b, _) = gap_fx();
    let (m, v1, a1) = (b.m, b.v1, b.a1);
    b.fx.clip(a1, m, 4 * S, 2 * S, 0); // intersects [2s, 5s)
    let mut doc = b.fx.doc();
    assert_err!(
        &mut doc,
        Command::CloseGap {
            track_id: v1,
            time: 3 * S
        },
        Error::RippleBlocked { .. }
    );
}

#[test]
fn c_gap_1_locked_target_track() {
    // ambiguity (see report): reading = changing clips on a locked track is TrackLocked (G5)
    let (mut b, _) = gap_fx();
    let v1 = b.v1;
    b.fx.lock(v1);
    let mut doc = b.fx.doc();
    assert_err!(&mut doc, Command::CloseGap { track_id: v1, time: 3 * S }, Error::TrackLocked { track_id } if track_id == v1);
}

// ============================================================================ C-SPLIT-1 / R4

struct SplitFx {
    b: Base,
    x: Id,
    xa: Id,
    y: Id,
    z: Id,
    w: Id,
    l: Id,
    e: Id,
    t_end: Id,
    t_start: Id,
}

fn split_fx() -> SplitFx {
    let mut b = base();
    let (m, v1, v2, a1, a2) = (b.m, b.v1, b.v2, b.a1, b.a2);
    let x = b.fx.clip(v1, m, 2 * S, 4 * S, S);
    let xa = b.fx.clip(a1, m, 2 * S, 4 * S, S);
    let l = b.fx.link(&[x, xa]);
    let w = b.fx.clip(v1, m, 7 * S, S, 0);
    let y = b.fx.clip(v2, m, 0, 10 * S, 0);
    let z = b.fx.clip(a2, m, 0, 10 * S, 0);
    b.fx.lock(a2);
    b.fx.c(x).params.insert(
        "opacity".into(),
        param(
            1.0,
            vec![kf(0, 0.0, Easing::Linear), kf(4 * S, 1.0, Easing::Linear)],
        ),
    );
    let e = b.fx.effect(
        x,
        "blur",
        &[(
            "amount",
            param(
                0.0,
                vec![kf(0, 10.0, Easing::EaseIn), kf(4 * S, 50.0, Easing::Linear)],
            ),
        )],
    );
    let t_end = b.fx.transition(v1, Some(x), None, S);
    let t_start = b.fx.transition(v1, None, Some(x), S);
    SplitFx {
        b,
        x,
        xa,
        y,
        z,
        w,
        l,
        e,
        t_end,
        t_start,
    }
}

#[test]
fn c_split_1_all_unlocked_tracks() {
    let SplitFx {
        b,
        x,
        xa,
        y,
        z,
        w,
        l,
        ..
    } = split_fx();
    let (seq, v1, v2, a1, a2) = (b.seq, b.v1, b.v2, b.a1, b.a2);
    let mut doc = b.fx.doc();
    let n = doc.project().next_id;
    let out = ok(
        &mut doc,
        Command::SplitAt {
            sequence_id: seq,
            time: 3 * S,
            track_ids: None,
        },
    );
    let p = doc.project();
    assert_eq!(out.created.len(), 3);
    let (xr, yr, xar) = (out.created[0], out.created[1], out.created[2]);
    assert_eq!(track_of(p, xr), v1, "created in track order");
    assert_eq!(track_of(p, yr), v2);
    assert_eq!(track_of(p, xar), a1);
    assert_eq!(
        spans(p, v1),
        vec![(x, 2 * S, 3 * S), (xr, 3 * S, 6 * S), (w, 7 * S, 8 * S)]
    );
    assert_eq!(spans(p, v2), vec![(y, 0, 3 * S), (yr, 3 * S, 10 * S)]);
    assert_eq!(spans(p, a1), vec![(xa, 2 * S, 3 * S), (xar, 3 * S, 6 * S)]);
    assert_eq!(spans(p, a2), vec![(z, 0, 10 * S)]);
    assert_eq!((clip(p, x).source_in, clip(p, xr).source_in), (S, 2 * S));
    assert_eq!((clip(p, y).source_in, clip(p, yr).source_in), (0, 3 * S));
    // link handling
    assert_eq!((clip(p, x).link, clip(p, xa).link), (Some(l), Some(l)));
    let l2 = clip(p, xr).link.expect("right halves are linked");
    assert_ne!(l2, l);
    assert!(l2 >= n);
    assert_eq!(clip(p, xar).link, Some(l2));
    assert_eq!(clip(p, yr).link, None);
    // 3 clip ids, 1 effect id, 1 link id
    assert_eq!(p.next_id, n + 5);
}

#[test]
fn c_split_1_r4_right_part_copies_and_new_effect_ids() {
    let SplitFx { b, x, e, .. } = split_fx();
    let seq = b.seq;
    let mut doc = b.fx.doc();
    let n = doc.project().next_id;
    let before = clip(doc.project(), x).clone();
    let out = ok(
        &mut doc,
        Command::SplitAt {
            sequence_id: seq,
            time: 3 * S,
            track_ids: None,
        },
    );
    let p = doc.project();
    let xr = clip(p, out.created[0]);
    assert!(xr.id >= n);
    assert_eq!(xr.effects.len(), 1);
    assert_ne!(xr.effects[0].id, e);
    assert!(xr.effects[0].id >= n);
    assert_eq!(
        clip(p, x).effects[0].id,
        e,
        "left part keeps its effect ids"
    );
    assert_eq!(xr.effects[0].effect, "blur");
    assert_eq!(
        (xr.name.as_str(), xr.source.clone(), xr.speed, xr.reverse),
        (before.name.as_str(), before.source.clone(), 1.0, false)
    );
    // keyframes: left untouched (tail trim), right shifted by -1s (head trim)
    assert_eq!(clip(p, x).params, before.params);
    assert_eq!(
        xr.params["opacity"].keyframes,
        vec![kf(-S, 0.0, Easing::Linear), kf(3 * S, 1.0, Easing::Linear)]
    );
    assert_eq!(
        xr.effects[0].params["amount"].keyframes,
        vec![
            kf(-S, 10.0, Easing::EaseIn),
            kf(3 * S, 50.0, Easing::Linear)
        ]
    );
    // same value at the same sequence time (4s): local 2s before, 1s in the right part
    assert_approx(before.param("opacity", 2 * S).unwrap(), 0.5);
    assert_approx(xr.param("opacity", S).unwrap(), 0.5);
    assert_approx(xr.effects[0].params["amount"].eval(S), 20.0); // 10 + 40 * 0.5^2
}

#[test]
fn c_split_1_r4_transitions_follow_their_edge() {
    let SplitFx {
        b,
        x,
        t_end,
        t_start,
        ..
    } = split_fx();
    let (seq, v1) = (b.seq, b.v1);
    let mut doc = b.fx.doc();
    let out = ok(
        &mut doc,
        Command::SplitAt {
            sequence_id: seq,
            time: 3 * S,
            track_ids: Some(vec![v1]),
        },
    );
    let p = doc.project();
    let xr = out.created[0];
    let ts = &trk(p, v1).transitions;
    let te = ts
        .iter()
        .find(|t| t.id == t_end)
        .expect("end transition kept");
    assert_eq!((te.left, te.right), (Some(xr), None));
    let tst = ts
        .iter()
        .find(|t| t.id == t_start)
        .expect("start transition kept");
    assert_eq!((tst.left, tst.right), (None, Some(x)));
}

#[test]
fn c_split_1_r4_speed_and_reverse_source_in() {
    let Base {
        mut fx,
        m,
        seq,
        v1,
        v2,
        ..
    } = base();
    let f = fx.clip(v1, m, 2 * S, 4 * S, S);
    fx.c(f).speed = 2.0;
    let r = fx.clip(v2, m, 2 * S, 4 * S, S);
    fx.c(r).speed = 2.0;
    fx.c(r).reverse = true;
    let mut doc = fx.doc();
    let before_f = clip(doc.project(), f).clone();
    let before_r = clip(doc.project(), r).clone();
    let out = ok(
        &mut doc,
        Command::SplitAt {
            sequence_id: seq,
            time: 3 * S,
            track_ids: None,
        },
    );
    let p = doc.project();
    let (fr, rr) = (out.created[0], out.created[1]);
    assert_eq!((clip(p, f).source_in, clip(p, f).duration), (S, S));
    assert_eq!(
        (
            clip(p, fr).source_in,
            clip(p, fr).start,
            clip(p, fr).duration
        ),
        (3 * S, 3 * S, 3 * S)
    );
    assert_eq!((clip(p, r).source_in, clip(p, r).duration), (7 * S, S));
    assert_eq!(
        (
            clip(p, rr).source_in,
            clip(p, rr).start,
            clip(p, rr).duration
        ),
        (S, 3 * S, 3 * S)
    );
    // Q3 unchanged at 4s and 2.5s
    assert_eq!(before_f.source_time(4 * S), 5 * S);
    assert_eq!(clip(p, fr).source_time(4 * S), 5 * S);
    assert_eq!(before_r.source_time(4 * S), 5 * S);
    assert_eq!(clip(p, rr).source_time(4 * S), 5 * S);
    assert_eq!(clip(p, r).source_time(2 * S + S / 2), 8 * S);
}

#[test]
fn c_split_1_boundaries_and_noop() {
    let SplitFx { b, x, y, .. } = split_fx();
    let (seq, v2) = (b.seq, b.v2);
    let mut doc = b.fx.doc();
    // X ends at 6s: not split; Y crosses
    let out = ok(
        &mut doc,
        Command::SplitAt {
            sequence_id: seq,
            time: 6 * S,
            track_ids: None,
        },
    );
    assert_eq!(out.created.len(), 1);
    assert_eq!(track_of(doc.project(), out.created[0]), v2);
    assert_eq!(clip(doc.project(), x).end(), 6 * S);
    // nothing crosses 10s (Y's right part ends there, Z is locked): Ok, unchanged, no undo entry
    let before = doc.project().clone();
    let depth = doc.undo_depth();
    let out = ok(
        &mut doc,
        Command::SplitAt {
            sequence_id: seq,
            time: 10 * S,
            track_ids: None,
        },
    );
    assert!(out.created.is_empty());
    assert_eq!(doc.project(), &before);
    assert_eq!(doc.undo_depth(), depth);
    let _ = y;
}

#[test]
fn c_split_1_listed_tracks_and_errors() {
    let SplitFx { b, x, .. } = split_fx();
    let (seq, v2, a2) = (b.seq, b.v2, b.a2);
    let mut doc = b.fx.doc();
    assert_err!(
        &mut doc,
        Command::SplitAt { sequence_id: seq, time: 3 * S, track_ids: Some(vec![a2]) },
        Error::TrackLocked { track_id } if track_id == a2
    );
    assert_err!(
        &mut doc,
        Command::SplitAt {
            sequence_id: seq,
            time: -S,
            track_ids: None
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::SplitAt {
            sequence_id: 999,
            time: S,
            track_ids: None
        },
        Error::NotFound { id: 999, .. }
    );
    let out = ok(
        &mut doc,
        Command::SplitAt {
            sequence_id: seq,
            time: 3 * S,
            track_ids: Some(vec![v2]),
        },
    );
    assert_eq!(out.created.len(), 1);
    assert_eq!(clip(doc.project(), x).duration, 4 * S);
}

// ============================================================================ C-SPLIT-2

#[test]
fn c_split_2_with_linked() {
    let SplitFx { b, x, xa, l, .. } = split_fx();
    let (v1, a1) = (b.v1, b.a1);
    let mut doc = b.fx.doc();
    let out = ok(
        &mut doc,
        Command::SplitClip {
            clip_id: x,
            time: 4 * S,
            include_linked: true,
        },
    );
    let p = doc.project();
    assert_eq!(out.created.len(), 2);
    assert_eq!(track_of(p, out.created[0]), v1);
    assert_eq!(track_of(p, out.created[1]), a1);
    assert_eq!((clip(p, x).end(), clip(p, xa).end()), (4 * S, 4 * S));
    let l2 = clip(p, out.created[0]).link;
    assert!(l2.is_some() && l2 != Some(l));
    assert_eq!(clip(p, out.created[1]).link, l2);
    assert_eq!(clip(p, out.created[0]).source_in, 3 * S);
}

#[test]
fn c_split_2_without_linked_or_partner_not_containing_time() {
    let SplitFx { mut b, x, xa, .. } = split_fx();
    let a1 = b.a1;
    let mut doc = b.fx.clone_doc();
    let out = ok(
        &mut doc,
        Command::SplitClip {
            clip_id: x,
            time: 4 * S,
            include_linked: false,
        },
    );
    assert_eq!(out.created.len(), 1);
    assert_eq!(spans(doc.project(), a1), vec![(xa, 2 * S, 6 * S)]);

    b.fx.c(xa).duration = S; // partner [2s, 3s) does not contain 4s
    let mut doc = b.fx.doc();
    let out = ok(
        &mut doc,
        Command::SplitClip {
            clip_id: x,
            time: 4 * S,
            include_linked: true,
        },
    );
    assert_eq!(out.created.len(), 1);
    assert_eq!(spans(doc.project(), a1), vec![(xa, 2 * S, 3 * S)]);
}

#[test]
fn c_split_2_errors() {
    let SplitFx { mut b, x, xa, .. } = split_fx();
    let mut doc = b.fx.clone_doc();
    for time in [2 * S, 6 * S, 7 * S, 0] {
        assert_err!(
            &mut doc,
            Command::SplitClip {
                clip_id: x,
                time,
                include_linked: true
            },
            Error::InvalidArgument(_)
        );
    }
    assert_err!(
        &mut doc,
        Command::SplitClip {
            clip_id: 999,
            time: 4 * S,
            include_linked: true
        },
        Error::NotFound { id: 999, .. }
    );
    let a1 = b.a1;
    b.fx.lock(a1);
    let mut doc = b.fx.doc();
    assert_err!(
        &mut doc,
        Command::SplitClip { clip_id: x, time: 4 * S, include_linked: true },
        Error::TrackLocked { track_id } if track_id == a1
    );
    let _ = xa;
}

// ============================================================================ C-MOVE-1

fn mv(
    clip_id: Id,
    start: Time,
    track_id: Option<Id>,
    mode: PlaceMode,
    include_linked: bool,
) -> Command {
    Command::MoveClip {
        clip_id,
        start,
        track_id,
        mode,
        include_linked,
    }
}

/// V1: X [0,2s) linked to XA on A1 [0,2s); V1: Y [5s,7s).
fn move_fx() -> (Base, Id, Id, Id) {
    let mut b = base();
    let (m, v1, a1) = (b.m, b.v1, b.a1);
    let x = b.fx.clip(v1, m, 0, 2 * S, 0);
    let xa = b.fx.clip(a1, m, 0, 2 * S, 0);
    b.fx.link(&[x, xa]);
    let y = b.fx.clip(v1, m, 5 * S, 2 * S, 0);
    (b, x, xa, y)
}

#[test]
fn c_move_1_moves_linked_set() {
    let (b, x, xa, y) = move_fx();
    let (v1, a1) = (b.v1, b.a1);
    let mut doc = b.fx.doc();
    ok(&mut doc, mv(x, 2 * S, None, PlaceMode::Place, true));
    let p = doc.project();
    assert_eq!(spans(p, v1), vec![(x, 2 * S, 4 * S), (y, 5 * S, 7 * S)]);
    assert_eq!(spans(p, a1), vec![(xa, 2 * S, 4 * S)]);
    assert_eq!((clip(p, x).source_in, clip(p, xa).source_in), (0, 0));
}

#[test]
fn c_move_1_without_linked() {
    let (b, x, xa, _) = move_fx();
    let a1 = b.a1;
    let mut doc = b.fx.doc();
    ok(&mut doc, mv(x, 2 * S, None, PlaceMode::Place, false));
    assert_eq!(clip(doc.project(), x).start, 2 * S);
    assert_eq!(spans(doc.project(), a1), vec![(xa, 0, 2 * S)]);
}

#[test]
fn c_move_1_overlapping_own_old_position_is_fine() {
    let (b, x, xa, _) = move_fx();
    let mut doc = b.fx.doc();
    ok(&mut doc, mv(x, S, None, PlaceMode::Place, true));
    assert_eq!(
        (clip(doc.project(), x).start, clip(doc.project(), xa).start),
        (S, S)
    );
}

#[test]
fn c_move_1_to_other_track_partner_keeps_track() {
    let (b, x, xa, _) = move_fx();
    let (v2, a1) = (b.v2, b.a1);
    let mut doc = b.fx.doc();
    ok(&mut doc, mv(x, 3 * S, Some(v2), PlaceMode::Place, true));
    let p = doc.project();
    assert_eq!(spans(p, v2), vec![(x, 3 * S, 5 * S)]);
    assert_eq!(spans(p, a1), vec![(xa, 3 * S, 5 * S)]);
}

#[test]
fn c_move_1_place_overlap() {
    let (mut b, x, _, _) = move_fx();
    let (m, v1, a1) = (b.m, b.v1, b.a1);
    let mut doc = b.fx.clone_doc();
    assert_err!(&mut doc, mv(x, 4 * S, None, PlaceMode::Place, true), Error::Overlap { track_id } if track_id == v1);
    // touching Y's start is fine
    ok(&mut doc, mv(x, 3 * S, None, PlaceMode::Place, true));
    // partner collides on A1
    b.fx.clip(a1, m, 3 * S, S, 0);
    let mut doc = b.fx.doc();
    assert_err!(&mut doc, mv(x, 2 * S, None, PlaceMode::Place, true), Error::Overlap { track_id } if track_id == a1);
}

#[test]
fn c_move_1_overwrite_clears_destination() {
    let (b, x, xa, y) = move_fx();
    let (v1, a1) = (b.v1, b.a1);
    let mut doc = b.fx.doc();
    ok(&mut doc, mv(x, 4 * S, None, PlaceMode::Overwrite, true));
    let p = doc.project();
    assert_eq!(spans(p, v1), vec![(x, 4 * S, 6 * S), (y, 6 * S, 7 * S)]);
    assert_eq!(clip(p, y).source_in, S);
    assert_eq!(spans(p, a1), vec![(xa, 4 * S, 6 * S)]);
}

#[test]
fn c_move_1_errors() {
    let (mut b, x, xa, _) = move_fx();
    let (m, vid, v1, v2, a1) = (b.m, b.vid, b.v1, b.v2, b.a1);
    let vo = b.fx.clip(v1, vid, 10 * S, S, 0);
    let t = b.fx.text(v1, 12 * S, S);
    let other = b.fx.seq("Other");
    let ot = b.fx.track(other, TrackKind::Video);
    let mut doc = b.fx.clone_doc();
    assert_err!(
        &mut doc,
        mv(x, 20 * S, None, PlaceMode::Insert, true),
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        mv(x, -1, None, PlaceMode::Place, true),
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        mv(x, 20 * S, Some(ot), PlaceMode::Place, true),
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        mv(vo, 20 * S, Some(a1), PlaceMode::Place, false),
        Error::KindMismatch(_)
    );
    assert_err!(
        &mut doc,
        mv(t, 20 * S, Some(a1), PlaceMode::Place, false),
        Error::KindMismatch(_)
    );
    assert_err!(
        &mut doc,
        mv(999, 0, None, PlaceMode::Place, true),
        Error::NotFound { id: 999, .. }
    );
    assert_err!(
        &mut doc,
        mv(x, 20 * S, Some(998), PlaceMode::Place, true),
        Error::NotFound { id: 998, .. }
    );
    // a linked clip would end up before 0
    b.fx.c(x).start = 3 * S;
    let mut doc = b.fx.clone_doc();
    assert_err!(
        &mut doc,
        mv(x, S, None, PlaceMode::Place, true),
        Error::InvalidArgument(_)
    );
    ok(&mut doc, mv(x, S, None, PlaceMode::Place, false));
    let _ = (m, xa);
    // G5: locked source, destination and partner tracks
    for locked in [v1, v2, a1] {
        let mut fx2 = Fx { p: b.fx.p.clone() };
        fx2.lock(locked);
        let mut doc = fx2.doc();
        assert_err!(
            &mut doc,
            mv(x, 20 * S, Some(v2), PlaceMode::Place, true),
            Error::TrackLocked { track_id } if track_id == locked
        );
    }
}

#[test]
fn c_trans_3_moving_q_prunes_transition_and_undo_restores_it() {
    let Base { mut fx, m, v1, .. } = base();
    let p0 = fx.clip(v1, m, 0, 2 * S, 0);
    let q = fx.clip(v1, m, 2 * S, 2 * S, 0);
    let t = fx.transition(v1, Some(p0), Some(q), S);
    let mut doc = fx.doc();
    let before = doc.project().clone();
    ok(&mut doc, mv(q, 3 * S, None, PlaceMode::Place, true));
    assert!(trk(doc.project(), v1).transitions.is_empty(), "G8");
    assert!(doc.undo());
    assert_eq!(doc.project(), &before);
    assert_eq!(
        trk(doc.project(), v1)
            .transitions
            .iter()
            .map(|x| x.id)
            .collect::<Vec<_>>(),
        vec![t]
    );
}

// ============================================================================ C-SET-1

#[test]
fn c_set_1_set_clip_fields() {
    let Base { mut fx, m, v1, .. } = base();
    let c = fx.clip(v1, m, 0, 4 * S, S);
    let mut doc = fx.doc();
    ok(
        &mut doc,
        Command::SetClip {
            clip_id: c,
            name: Some("Shot".into()),
            enabled: Some(false),
            reverse: None,
            blend: Some(BlendMode::Screen),
            audio_filter: None,
        },
    );
    let cl = clip(doc.project(), c);
    assert_eq!(
        (cl.name.as_str(), cl.enabled, cl.reverse, cl.blend),
        ("Shot", false, false, BlendMode::Screen)
    );
    assert_eq!((cl.start, cl.duration, cl.source_in), (0, 4 * S, S));
    ok(
        &mut doc,
        Command::SetClip {
            clip_id: c,
            name: None,
            enabled: None,
            reverse: Some(true),
            blend: None,
            audio_filter: Some("volume=0.5".into()),
        },
    );
    let cl = clip(doc.project(), c);
    assert_eq!(
        (cl.name.as_str(), cl.enabled, cl.reverse, cl.blend),
        ("Shot", false, true, BlendMode::Screen)
    );
    assert_eq!(cl.audio_filter, "volume=0.5");
    assert_eq!((cl.start, cl.duration, cl.source_in), (0, 4 * S, S));
}

#[test]
fn c_set_1_errors() {
    let Base {
        mut fx, m, v1, v2, ..
    } = base();
    let c = fx.clip(v2, m, 0, S, 0);
    fx.lock(v2);
    let _ = v1;
    let mut doc = fx.doc();
    assert_err!(
        &mut doc,
        Command::SetClip { clip_id: c, name: Some("x".into()), enabled: None, reverse: None, blend: None, audio_filter: None },
        Error::TrackLocked { track_id } if track_id == v2
    );
    assert_err!(
        &mut doc,
        Command::SetClip {
            clip_id: 999,
            name: Some("x".into()),
            enabled: None,
            reverse: None,
            blend: None,
            audio_filter: None
        },
        Error::NotFound { id: 999, .. }
    );
}

#[test]
fn c_set_1_set_text() {
    let Base { mut fx, m, v1, .. } = base();
    let t = fx.text(v1, 0, S);
    let c = fx.clip(v1, m, S, S, 0);
    let mut doc = fx.doc();
    let mut spec = text_spec("new words");
    spec.bold = true;
    spec.size = 72.0;
    ok(
        &mut doc,
        Command::SetText {
            clip_id: t,
            text: spec.clone(),
        },
    );
    assert_eq!(
        clip(doc.project(), t).source,
        ClipSource::Text { text: spec.clone() }
    );
    assert_err!(
        &mut doc,
        Command::SetText {
            clip_id: c,
            text: spec.clone()
        },
        Error::KindMismatch(_)
    );
    for size in [0.0, -1.0, f64::NAN] {
        let mut bad = spec.clone();
        bad.size = size;
        assert_err!(
            &mut doc,
            Command::SetText {
                clip_id: t,
                text: bad
            },
            Error::InvalidArgument(_)
        );
    }
}

// ============================================================================ C-LINK-1

#[test]
fn c_link_1_link_and_unlink() {
    let Base {
        mut fx,
        m,
        v1,
        v2,
        a1,
        ..
    } = base();
    let x = fx.clip(v1, m, 0, S, 0);
    let y = fx.clip(v2, m, 0, S, 0);
    let z = fx.clip(a1, m, 0, S, 0);
    let old = fx.link(&[x, y]);
    let mut doc = fx.doc();
    let n = doc.project().next_id;
    ok(
        &mut doc,
        Command::LinkClips {
            clip_ids: vec![y, z],
        },
    );
    let p = doc.project();
    assert_eq!(
        (clip(p, x).link, clip(p, y).link, clip(p, z).link),
        (Some(old), Some(n), Some(n))
    );
    assert_eq!(p.next_id, n + 1);
    ok(&mut doc, Command::UnlinkClips { clip_ids: vec![y] });
    let p = doc.project();
    assert_eq!(
        (clip(p, x).link, clip(p, y).link, clip(p, z).link),
        (Some(old), None, Some(n))
    );
}

#[test]
fn c_link_1_group_and_ungroup() {
    let Base {
        mut fx, m, v1, v2, ..
    } = base();
    let x = fx.clip(v1, m, 0, S, 0);
    let y = fx.clip(v2, m, 0, S, 0);
    let w = fx.clip(v2, m, S, S, 0);
    let mut doc = fx.doc();
    let n = doc.project().next_id;
    ok(
        &mut doc,
        Command::GroupClips {
            clip_ids: vec![x, y, w],
        },
    );
    let p = doc.project();
    assert_eq!(
        (clip(p, x).group, clip(p, y).group, clip(p, w).group),
        (Some(n), Some(n), Some(n))
    );
    assert_eq!(
        (clip(p, x).link, clip(p, y).link),
        (None, None),
        "grouping does not link"
    );
    ok(
        &mut doc,
        Command::UngroupClips {
            clip_ids: vec![x, y],
        },
    );
    let p = doc.project();
    assert_eq!(
        (clip(p, x).group, clip(p, y).group, clip(p, w).group),
        (None, None, Some(n))
    );
}

#[test]
fn c_link_1_errors() {
    let Base {
        mut fx, m, v1, v2, ..
    } = base();
    let x = fx.clip(v1, m, 0, S, 0);
    let y = fx.clip(v2, m, 0, S, 0);
    let other = fx.seq("Other");
    let ot = fx.track(other, TrackKind::Video);
    let far = fx.clip(ot, m, 0, S, 0);
    let mut doc = fx.doc();
    assert_err!(
        &mut doc,
        Command::LinkClips { clip_ids: vec![x] },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::LinkClips { clip_ids: vec![] },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::LinkClips {
            clip_ids: vec![x, far]
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::GroupClips { clip_ids: vec![x] },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::GroupClips {
            clip_ids: vec![x, far]
        },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::LinkClips {
            clip_ids: vec![x, 999]
        },
        Error::NotFound { id: 999, .. }
    );
    assert_err!(
        &mut doc,
        Command::UnlinkClips { clip_ids: vec![] },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::UngroupClips { clip_ids: vec![] },
        Error::InvalidArgument(_)
    );
    assert_err!(
        &mut doc,
        Command::UnlinkClips {
            clip_ids: vec![999]
        },
        Error::NotFound { id: 999, .. }
    );
    let _ = y;
}
