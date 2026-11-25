//! U-ADDEDIT, U-QW, U-DEL, U-LIFT, U-INSERT, U-NUDGE, U-ENABLE, U-LINK, U-TRANS: actions that
//! return `Op::Edit`. Each checks the exact commands, the `ui` change, and (for the key cases) the
//! project after the window applies the batch.
mod common;
use common::*;
use std::collections::BTreeMap;

/// Runs `a`, requires `ops == want_ops` and `ui == want(before)`.
#[track_caller]
fn check(
    a: Action,
    p: &Project,
    u: &mut UiState,
    want_ops: Vec<Op>,
    want: impl FnOnce(&mut UiState),
) -> Vec<Op> {
    let mut expected = u.clone();
    want(&mut expected);
    let ops = act(a, p, u);
    assert_eq!(ops, want_ops, "{a:?}");
    assert_eq!(*u, expected, "{a:?}: ui");
    ops
}

// ---------------------------------------------------------------------------------- U-ADDEDIT

/// V1 c100 [0,4s), c101 [4s,8s); A1 c102 [0,4s) linked to c100; V2 c103 [6s,7s).
fn split_project() -> Project {
    B::std()
        .clip(V1, 100, 0, 4 * S)
        .clip(V1, 101, 4 * S, 4 * S)
        .clip(A1, 102, 0, 4 * S)
        .link(&[100, 102], 900)
        .clip(V2, 103, 6 * S, S)
        .build()
}

fn split_clip(clip_id: Id, time: Time) -> Command {
    Command::SplitClip {
        clip_id,
        time,
        include_linked: false,
    }
}

#[test]
fn u_addedit_selection_strictly_containing() {
    let p = split_project();
    let mut u = ui();
    u.playhead = 2 * S;
    u.selection = vec![100, 101];
    u.linked_selection = false;
    let ops = check(
        Action::AddEdit,
        &p,
        &mut u,
        edit(vec![split_clip(100, 2 * S)]),
        |_| {},
    );
    let after = apply(&p, &ops);
    assert_eq!(
        clips_on(&after, V1),
        vec![
            (100, 0, 2 * S),
            (100_000, 2 * S, 2 * S),
            (101, 4 * S, 4 * S)
        ]
    );
    assert_eq!(
        clips_on(&after, A1),
        vec![(102, 0, 4 * S)],
        "include_linked false"
    );
}

#[test]
fn u_addedit_selection_boundary_is_not_strict() {
    let p = split_project();
    // playhead 4 s = c101 start = c100 end: neither strictly contains it -> no op, even though
    // the SplitAt branch is not used when the selection is non-empty.
    let mut u = ui();
    u.playhead = 4 * S;
    u.selection = vec![101, 100];
    assert_noop(Action::AddEdit, &p, &u);
    u.playhead = 2 * S;
    u.selection = vec![101];
    assert_noop(Action::AddEdit, &p, &u);
}

#[test]
fn u_addedit_effective_selection_in_track_then_time_order() {
    let p = split_project();
    let mut u = ui();
    u.playhead = 2 * S;
    u.selection = vec![102]; // linked set adds c100 (V1, before A1)
    check(
        Action::AddEdit,
        &p,
        &mut u,
        edit(vec![split_clip(100, 2 * S), split_clip(102, 2 * S)]),
        |_| {},
    );
    let mut u = ui();
    u.playhead = 6 * S + S / 2;
    u.selection = vec![103, 101]; // V1 before V2
    check(
        Action::AddEdit,
        &p,
        &mut u,
        edit(vec![
            split_clip(101, 6 * S + S / 2),
            split_clip(103, 6 * S + S / 2),
        ]),
        |_| {},
    );
    let mut u = ui();
    u.playhead = 2 * S;
    u.selection = vec![102];
    u.linked_selection = false;
    check(
        Action::AddEdit,
        &p,
        &mut u,
        edit(vec![split_clip(102, 2 * S)]),
        |_| {},
    );
}

#[test]
fn u_addedit_no_selection_splits_targets() {
    let p = split_project();
    let mut u = ui();
    u.playhead = 2 * S;
    let ops = check(
        Action::AddEdit,
        &p,
        &mut u,
        edit(vec![Command::SplitAt {
            sequence_id: SEQ,
            time: 2 * S,
            track_ids: Some(vec![V1, V2, A1, A2]),
        }]),
        |_| {},
    );
    let after = apply(&p, &ops);
    assert_eq!(clips_on(&after, V1).len(), 3);
    assert_eq!(clips_on(&after, A1).len(), 2);
    // Stale-only selection counts as empty.
    let mut u = ui();
    u.playhead = 2 * S;
    u.selection = vec![STALE];
    u.targets = vec![V1, A1];
    check(
        Action::AddEdit,
        &p,
        &mut u,
        edit(vec![Command::SplitAt {
            sequence_id: SEQ,
            time: 2 * S,
            track_ids: Some(vec![V1, A1]),
        }]),
        |_| {},
    );
}

#[test]
fn u_addedit_targets_skip_locked() {
    let p = B { p: split_project() }.lock(V1).build();
    let mut u = ui();
    u.playhead = 2 * S;
    check(
        Action::AddEdit,
        &p,
        &mut u,
        edit(vec![Command::SplitAt {
            sequence_id: SEQ,
            time: 2 * S,
            track_ids: Some(vec![V2, A1, A2]),
        }]),
        |_| {},
    );
    let mut u = ui();
    u.playhead = 2 * S;
    u.targets = vec![V1, A1];
    check(
        Action::AddEdit,
        &p,
        &mut u,
        edit(vec![Command::SplitAt {
            sequence_id: SEQ,
            time: 2 * S,
            track_ids: Some(vec![A1]),
        }]),
        |_| {},
    );
}

#[test]
fn u_addedit_no_selection_nothing_to_split() {
    let p = split_project();
    let mut u = ui();
    u.playhead = 4 * S; // on a cut everywhere
    assert_noop(Action::AddEdit, &p, &u);
    u.playhead = 9 * S; // after everything
    assert_noop(Action::AddEdit, &p, &u);
    u.playhead = 2 * S;
    u.targets = vec![V2, A2]; // nothing there
    assert_noop(Action::AddEdit, &p, &u);
    // Only a locked track contains it.
    let locked = B::std().clip(V1, 100, 0, 4 * S).lock(V1).build();
    let mut u = ui();
    u.playhead = 2 * S;
    assert_noop(Action::AddEdit, &locked, &u);
    assert_noop(Action::AddEditAll, &locked, &u);
}

#[test]
fn u_addedit_all() {
    let p = split_project();
    let mut u = ui();
    u.playhead = 6 * S + S / 2;
    u.targets = vec![A2]; // ignored
    u.selection = vec![100]; // ignored
    let ops = check(
        Action::AddEditAll,
        &p,
        &mut u,
        edit(vec![Command::SplitAt {
            sequence_id: SEQ,
            time: 6 * S + S / 2,
            track_ids: None,
        }]),
        |_| {},
    );
    let after = apply(&p, &ops);
    assert_eq!(clips_on(&after, V1).len(), 3);
    assert_eq!(clips_on(&after, V2).len(), 2);
    let mut u = ui();
    u.playhead = 4 * S;
    assert_noop(Action::AddEditAll, &p, &u);
}

// -------------------------------------------------------------------------------------- U-QW

#[test]
fn u_qw_example() {
    // One clip [0, 10s), playhead 4 s.
    let p = B::std().clip(V1, 100, 0, 10 * S).build();
    let mut u = ui();
    u.playhead = 4 * S;
    let ops = check(
        Action::RippleTrimPrev,
        &p,
        &mut u,
        edit(vec![Command::ExtractRange {
            sequence_id: SEQ,
            start: 0,
            end: 4 * S,
            track_ids: None,
        }]),
        |w| w.playhead = 0,
    );
    let after = apply(&p, &ops);
    assert_eq!(clips_on(&after, V1), vec![(100, 0, 6 * S)]);
    assert_eq!(clip(&after, 100).source_in, 4 * S, "shows source from 4 s");

    let mut u = ui();
    u.playhead = 4 * S;
    let ops = check(
        Action::RippleTrimNext,
        &p,
        &mut u,
        edit(vec![Command::ExtractRange {
            sequence_id: SEQ,
            start: 4 * S,
            end: 10 * S,
            track_ids: None,
        }]),
        |_| {},
    );
    let after = apply(&p, &ops);
    assert_eq!(clips_on(&after, V1), vec![(100, 0, 4 * S)]);
    assert_eq!(clip(&after, 100).source_in, 0);
}

#[test]
fn u_qw_on_edit_point_and_locked_points() {
    // V1 [0,2s), [2s,5s); A2 (locked) [3s,4s) still provides edit points.
    let p = B::std()
        .clip(V1, 100, 0, 2 * S)
        .clip(V1, 101, 2 * S, 3 * S)
        .clip(A2, 102, 3 * S, S)
        .lock(A2)
        .build();
    let mut u = ui();
    u.playhead = 2 * S;
    check(
        Action::RippleTrimPrev,
        &p,
        &mut u,
        edit(vec![Command::ExtractRange {
            sequence_id: SEQ,
            start: 0,
            end: 2 * S,
            track_ids: None,
        }]),
        |w| w.playhead = 0,
    );
    let mut u = ui();
    u.playhead = 2 * S;
    check(
        Action::RippleTrimNext,
        &p,
        &mut u,
        edit(vec![Command::ExtractRange {
            sequence_id: SEQ,
            start: 2 * S,
            end: 3 * S,
            track_ids: None,
        }]),
        |_| {},
    );
    let mut u = ui();
    u.playhead = 4 * S + S / 2;
    check(
        Action::RippleTrimPrev,
        &p,
        &mut u,
        edit(vec![Command::ExtractRange {
            sequence_id: SEQ,
            start: 4 * S,
            end: 4 * S + S / 2,
            track_ids: None,
        }]),
        |w| w.playhead = 4 * S,
    );
}

#[test]
fn u_qw_no_point_no_op() {
    let p = B::std().clip(V1, 100, 0, 10 * S).build();
    let mut u = ui();
    assert_noop(Action::RippleTrimPrev, &p, &u);
    u.playhead = 10 * S;
    assert_noop(Action::RippleTrimNext, &p, &u);
    u.playhead = 12 * S;
    assert_noop(Action::RippleTrimNext, &p, &u);
    let empty = B::std().build();
    assert_noop(Action::RippleTrimNext, &empty, &ui());
}

// ------------------------------------------------------------------------------------- U-DEL

/// C-RIP-2 layout: V1 P [0,2s) Q [2s,5s) R [5s,9s); A1 M [5s,9s); Q linked to A2 q [2s,5s).
fn del_project() -> Project {
    B::std()
        .clip(V1, 100, 0, 2 * S)
        .clip(V1, 101, 2 * S, 3 * S)
        .clip(V1, 102, 5 * S, 4 * S)
        .clip(A1, 103, 5 * S, 4 * S)
        .clip(A2, 104, 2 * S, 3 * S)
        .link(&[101, 104], 900)
        .build()
}

#[test]
fn u_del_delete() {
    let p = del_project();
    let mut u = ui();
    u.selection = vec![101];
    let ops = check(
        Action::Delete,
        &p,
        &mut u,
        edit(vec![Command::RemoveClips {
            clip_ids: vec![101],
            include_linked: true,
        }]),
        |w| w.selection = vec![],
    );
    let after = apply(&p, &ops);
    assert_eq!(
        clips_on(&after, V1),
        vec![(100, 0, 2 * S), (102, 5 * S, 4 * S)]
    );
    assert_eq!(clips_on(&after, A2), vec![]);

    let mut u = ui();
    u.selection = vec![101, STALE, 100];
    u.linked_selection = false;
    check(
        Action::Delete,
        &p,
        &mut u,
        edit(vec![Command::RemoveClips {
            clip_ids: vec![101, 100],
            include_linked: false,
        }]),
        |w| w.selection = vec![],
    );
}

#[test]
fn u_del_ripple_delete() {
    let p = del_project();
    let mut u = ui();
    u.selection = vec![101];
    let ops = check(
        Action::RippleDelete,
        &p,
        &mut u,
        edit(vec![Command::RippleDeleteClips {
            clip_ids: vec![101],
            include_linked: true,
        }]),
        |w| w.selection = vec![],
    );
    let after = apply(&p, &ops);
    assert_eq!(
        clips_on(&after, V1),
        vec![(100, 0, 2 * S), (102, 2 * S, 4 * S)]
    );
    assert_eq!(clips_on(&after, A1), vec![(103, 2 * S, 4 * S)]);
    let mut u = ui();
    u.selection = vec![100];
    u.linked_selection = false;
    check(
        Action::RippleDelete,
        &p,
        &mut u,
        edit(vec![Command::RippleDeleteClips {
            clip_ids: vec![100],
            include_linked: false,
        }]),
        |w| w.selection = vec![],
    );
}

#[test]
fn u_del_empty_selection_no_op() {
    let p = del_project();
    assert_noop(Action::Delete, &p, &ui());
    assert_noop(Action::RippleDelete, &p, &ui());
    // Only stale ids: nothing to delete (ui not checked: the contract does not say whether the
    // stale ids are dropped here).
    let mut u = ui();
    u.selection = vec![STALE];
    assert_eq!(act(Action::Delete, &p, &mut u.clone()), vec![]);
    assert_eq!(act(Action::RippleDelete, &p, &mut u), vec![]);
}

// ------------------------------------------------------------------------------------ U-LIFT

/// `(start, duration)` of every clip on a track (ids of split-off parts are not asserted).
fn spans(p: &Project, track: Id) -> Vec<(Time, Time)> {
    clips_on(p, track).iter().map(|c| (c.1, c.2)).collect()
}

#[test]
fn u_lift_and_extract() {
    let p = B::std()
        .clip(V1, 100, 0, 10 * S)
        .clip(A1, 101, 0, 10 * S)
        .build();
    let mut base = ui();
    base.playhead = 7 * S;
    base.mark_in = Some(S);
    base.mark_out = Some(3 * S);
    let after_ui = |w: &mut UiState| {
        w.playhead = S;
        w.mark_in = None;
        w.mark_out = None;
    };
    let mut u = base.clone();
    let ops = check(
        Action::Lift,
        &p,
        &mut u,
        edit(vec![Command::LiftRange {
            sequence_id: SEQ,
            start: S,
            end: 3 * S,
            track_ids: None,
        }]),
        after_ui,
    );
    let after = apply(&p, &ops);
    assert_eq!(spans(&after, V1), vec![(0, S), (3 * S, 7 * S)]);

    let mut u = base.clone();
    let ops = check(
        Action::Extract,
        &p,
        &mut u,
        edit(vec![Command::ExtractRange {
            sequence_id: SEQ,
            start: S,
            end: 3 * S,
            track_ids: None,
        }]),
        after_ui,
    );
    let after = apply(&p, &ops);
    assert_eq!(spans(&after, V1), vec![(0, S), (S, 7 * S)]);
    assert_eq!(clip(&after, clips_on(&after, V1)[1].0).source_in, 3 * S);

    let mut u = base.clone();
    u.targets = vec![V1];
    let ops = check(
        Action::Extract,
        &p,
        &mut u,
        edit(vec![Command::ExtractRange {
            sequence_id: SEQ,
            start: S,
            end: 3 * S,
            track_ids: Some(vec![V1]),
        }]),
        after_ui,
    );
    let after = apply(&p, &ops);
    assert_eq!(
        clips_on(&after, A1),
        vec![(101, 0, 10 * S)],
        "A1 not targeted"
    );
    let mut u = base.clone();
    u.targets = vec![A1, V1];
    check(
        Action::Lift,
        &p,
        &mut u,
        edit(vec![Command::LiftRange {
            sequence_id: SEQ,
            start: S,
            end: 3 * S,
            track_ids: Some(vec![A1, V1]),
        }]),
        after_ui,
    );
}

#[test]
fn u_lift_needs_both_marks() {
    let p = B::std().clip(V1, 100, 0, 10 * S).build();
    for (i, o) in [(Some(S), None), (None, Some(3 * S)), (None, None)] {
        let mut u = ui();
        u.playhead = 5 * S;
        u.mark_in = i;
        u.mark_out = o;
        assert_noop(Action::Lift, &p, &u);
        assert_noop(Action::Extract, &p, &u);
    }
}

// ---------------------------------------------------------------------------------- U-INSERT

const AV10: Id = 3; // video + audio, 10 s
const VID: Id = 4; // video only, 6 s
const AUD: Id = 5; // audio only, 4 s
const STILL: Id = 6; // still image (duration None)

fn insert_project() -> Project {
    B::std()
        .media(AV10, Some(10 * S), true, true)
        .media(VID, Some(6 * S), true, false)
        .media(AUD, Some(4 * S), false, true)
        .media(STILL, None, true, false)
        .build()
}

fn src(media_id: Id, mark_in: Option<Time>, mark_out: Option<Time>) -> Option<SourceState> {
    Some(SourceState {
        media_id,
        playhead: 0,
        mark_in,
        mark_out,
    })
}

#[allow(clippy::too_many_arguments)]
fn add(
    track_id: Id,
    linked: Option<Id>,
    media_id: Id,
    start: Time,
    duration: Time,
    source_in: Time,
    mode: PlaceMode,
) -> Command {
    Command::AddClip {
        track_id,
        linked_track_id: linked,
        source: ClipSource::Media { media_id },
        start,
        duration,
        source_in,
        mode,
        name: None,
    }
}

/// Insert ui: playhead 1 s, a selection, the given source.
fn ins_ui(source: Option<SourceState>) -> UiState {
    let mut u = ui();
    u.playhead = S;
    u.selection = vec![STALE];
    u.source = source;
    u
}

fn after_insert(playhead: Time) -> impl FnOnce(&mut UiState) {
    move |w| {
        w.playhead = playhead;
        w.selection = vec![];
        w.mark_in = None;
        w.mark_out = None;
    }
}

#[test]
fn u_insert_video_and_audio() {
    let p = insert_project();
    let mut u = ins_ui(src(AV10, None, None));
    check(
        Action::Insert,
        &p,
        &mut u,
        edit(vec![add(
            V1,
            Some(A1),
            AV10,
            S,
            10 * S,
            0,
            PlaceMode::Insert,
        )]),
        after_insert(11 * S),
    );
    let mut u = ins_ui(src(AV10, None, None));
    check(
        Action::Overwrite,
        &p,
        &mut u,
        edit(vec![add(
            V1,
            Some(A1),
            AV10,
            S,
            10 * S,
            0,
            PlaceMode::Overwrite,
        )]),
        after_insert(11 * S),
    );
}

#[test]
fn u_insert_applied_splits_and_pushes() {
    // V1 X [0,4s) linked to A1 Xa [0,4s); insert AV10 (10 s) at 1 s.
    let p = B {
        p: insert_project(),
    }
    .clip(V1, 100, 0, 4 * S)
    .clip(A1, 101, 0, 4 * S)
    .link(&[100, 101], 900)
    .build();
    let mut u = ins_ui(src(AV10, None, None));
    let ops = act(Action::Insert, &p, &mut u);
    let after = apply(&p, &ops);
    assert_eq!(
        spans(&after, V1),
        vec![(0, S), (S, 10 * S), (11 * S, 3 * S)]
    );
    assert_eq!(
        spans(&after, A1),
        vec![(0, S), (S, 10 * S), (11 * S, 3 * S)]
    );
    assert_eq!(clips_on(&after, V1)[1].0, 100_000, "the new clip");
}

#[test]
fn u_insert_source_marks() {
    let p = insert_project();
    let mut u = ins_ui(src(AV10, Some(2 * S), Some(5 * S)));
    check(
        Action::Insert,
        &p,
        &mut u,
        edit(vec![add(
            V1,
            Some(A1),
            AV10,
            S,
            3 * S,
            2 * S,
            PlaceMode::Insert,
        )]),
        after_insert(4 * S),
    );
    // Only mark_in: out = media duration.
    let mut u = ins_ui(src(AV10, Some(8 * S), None));
    check(
        Action::Insert,
        &p,
        &mut u,
        edit(vec![add(
            V1,
            Some(A1),
            AV10,
            S,
            2 * S,
            8 * S,
            PlaceMode::Insert,
        )]),
        after_insert(3 * S),
    );
    // Only mark_out: in = 0.
    let mut u = ins_ui(src(AV10, None, Some(3 * S)));
    check(
        Action::Overwrite,
        &p,
        &mut u,
        edit(vec![add(
            V1,
            Some(A1),
            AV10,
            S,
            3 * S,
            0,
            PlaceMode::Overwrite,
        )]),
        after_insert(4 * S),
    );
}

#[test]
fn u_insert_non_positive_duration_no_op() {
    let p = insert_project();
    for s in [
        src(AV10, Some(5 * S), Some(5 * S)),
        src(AV10, Some(6 * S), Some(5 * S)),
        src(AV10, Some(10 * S), None),
    ] {
        assert_noop(Action::Insert, &p, &ins_ui(s.clone()));
        assert_noop(Action::Overwrite, &p, &ins_ui(s));
    }
}

#[test]
fn u_insert_uses_timeline_mark_in() {
    let p = insert_project();
    let mut u = ins_ui(src(AV10, Some(2 * S), Some(5 * S)));
    u.mark_in = Some(3 * S);
    u.mark_out = Some(4 * S);
    check(
        Action::Overwrite,
        &p,
        &mut u,
        edit(vec![add(
            V1,
            Some(A1),
            AV10,
            3 * S,
            3 * S,
            2 * S,
            PlaceMode::Overwrite,
        )]),
        after_insert(6 * S),
    );
}

#[test]
fn u_insert_single_kind_media() {
    let p = insert_project();
    let mut u = ins_ui(src(VID, None, None));
    check(
        Action::Insert,
        &p,
        &mut u,
        edit(vec![add(V1, None, VID, S, 6 * S, 0, PlaceMode::Insert)]),
        after_insert(7 * S),
    );
    let mut u = ins_ui(src(AUD, None, None));
    check(
        Action::Insert,
        &p,
        &mut u,
        edit(vec![add(A1, None, AUD, S, 4 * S, 0, PlaceMode::Insert)]),
        after_insert(5 * S),
    );
}

#[test]
fn u_insert_still_is_five_seconds() {
    let p = insert_project();
    let mut u = ins_ui(src(STILL, None, None));
    check(
        Action::Overwrite,
        &p,
        &mut u,
        edit(vec![add(
            V1,
            None,
            STILL,
            S,
            5 * S,
            0,
            PlaceMode::Overwrite,
        )]),
        after_insert(6 * S),
    );
    let mut u = ins_ui(src(STILL, Some(S), None));
    check(
        Action::Insert,
        &p,
        &mut u,
        edit(vec![add(V1, None, STILL, S, 5 * S, S, PlaceMode::Insert)]),
        after_insert(6 * S),
    );
    let mut u = ins_ui(src(STILL, Some(S), Some(2 * S)));
    check(
        Action::Insert,
        &p,
        &mut u,
        edit(vec![add(V1, None, STILL, S, S, S, PlaceMode::Insert)]),
        after_insert(2 * S),
    );
}

#[test]
fn u_insert_target_tracks() {
    let p = insert_project();
    let mut u = ins_ui(src(AV10, None, None));
    u.targets = vec![A2, V2];
    check(
        Action::Insert,
        &p,
        &mut u,
        edit(vec![add(
            V2,
            Some(A2),
            AV10,
            S,
            10 * S,
            0,
            PlaceMode::Insert,
        )]),
        after_insert(11 * S),
    );
    // No audio target: video only.
    let mut u = ins_ui(src(AV10, None, None));
    u.targets = vec![V2];
    check(
        Action::Insert,
        &p,
        &mut u,
        edit(vec![add(V2, None, AV10, S, 10 * S, 0, PlaceMode::Insert)]),
        after_insert(11 * S),
    );
    // Video-only media, only audio targeted: nothing.
    let mut u = ins_ui(src(VID, None, None));
    u.targets = vec![A1];
    assert_noop(Action::Insert, &p, &u);
    // Locked lowest tracks are skipped.
    let locked = B { p: p.clone() }.lock(V1).lock(A1).build();
    let mut u = ins_ui(src(AV10, None, None));
    check(
        Action::Overwrite,
        &locked,
        &mut u,
        edit(vec![add(
            V2,
            Some(A2),
            AV10,
            S,
            10 * S,
            0,
            PlaceMode::Overwrite,
        )]),
        after_insert(11 * S),
    );
    // Locked track listed in targets is ignored.
    let mut u = ins_ui(src(AV10, None, None));
    u.targets = vec![V1, V2, A1];
    check(
        Action::Overwrite,
        &locked,
        &mut u,
        edit(vec![add(
            V2,
            None,
            AV10,
            S,
            10 * S,
            0,
            PlaceMode::Overwrite,
        )]),
        after_insert(11 * S),
    );
}

#[test]
fn u_insert_needs_source_in_project() {
    let p = insert_project();
    assert_noop(Action::Insert, &p, &ins_ui(None));
    assert_noop(Action::Overwrite, &p, &ins_ui(None));
    assert_noop(Action::Insert, &p, &ins_ui(src(STALE, None, None)));
}

#[test]
fn u_insert_playhead_floors_to_frame() {
    let p = B::std()
        .media(7, Some(S + 100), true, false)
        .media(8, Some(S - 100), true, false)
        .build();
    let mut u = ins_ui(src(7, None, None));
    u.playhead = 0;
    check(
        Action::Overwrite,
        &p,
        &mut u,
        edit(vec![add(V1, None, 7, 0, S + 100, 0, PlaceMode::Overwrite)]),
        after_insert(S),
    );
    let mut u = ins_ui(src(8, None, None));
    u.playhead = 0;
    check(
        Action::Overwrite,
        &p,
        &mut u,
        edit(vec![add(V1, None, 8, 0, S - 100, 0, PlaceMode::Overwrite)]),
        after_insert(29 * F),
    );
}

// ----------------------------------------------------------------------------------- U-NUDGE

fn mv(clip_id: Id, start: Time) -> Command {
    Command::MoveClip {
        clip_id,
        start,
        track_id: None,
        mode: PlaceMode::Overwrite,
        include_linked: false,
    }
}

#[test]
fn u_nudge_single_and_linked() {
    let p = B::std()
        .clip(V1, 100, 2 * S, S)
        .clip(A1, 101, 2 * S, S)
        .link(&[100, 101], 900)
        .build();
    let mut u = ui();
    u.selection = vec![100];
    u.linked_selection = false;
    check(
        Action::Nudge(1),
        &p,
        &mut u,
        edit(vec![mv(100, 2 * S + F)]),
        |_| {},
    );
    // Linked: both move (same start, so their order is not fixed by the rule).
    let mut u = ui();
    u.selection = vec![100];
    let before = u.clone();
    let ops = act(Action::Nudge(-5), &p, &mut u);
    assert_eq!(u, before);
    let [Op::Edit(cmds)] = &ops[..] else {
        panic!("one Edit expected: {ops:?}")
    };
    assert_eq!(
        sorted(cmds.clone()),
        sorted(vec![mv(100, 2 * S - 5 * F), mv(101, 2 * S - 5 * F)])
    );
    let after = apply(&p, &ops);
    assert_eq!(clips_on(&after, V1), vec![(100, 2 * S - 5 * F, S)]);
    assert_eq!(clips_on(&after, A1), vec![(101, 2 * S - 5 * F, S)]);
}

#[test]
fn u_nudge_order_and_result() {
    // Adjacent clips on one track: moving right goes from the last, left from the first.
    let p = B::std()
        .clip(V1, 100, S, S)
        .clip(V1, 101, 2 * S, S)
        .clip(V1, 102, 3 * S, S)
        .build();
    let mut u = ui();
    u.selection = vec![101, 100, 102];
    let ops = check(
        Action::Nudge(1),
        &p,
        &mut u,
        edit(vec![mv(102, 3 * S + F), mv(101, 2 * S + F), mv(100, S + F)]),
        |_| {},
    );
    let after = apply(&p, &ops);
    assert_eq!(
        clips_on(&after, V1),
        vec![(100, S + F, S), (101, 2 * S + F, S), (102, 3 * S + F, S)]
    );
    let mut u = ui();
    u.selection = vec![102, 101, 100];
    let ops = check(
        Action::Nudge(-5),
        &p,
        &mut u,
        edit(vec![
            mv(100, S - 5 * F),
            mv(101, 2 * S - 5 * F),
            mv(102, 3 * S - 5 * F),
        ]),
        |_| {},
    );
    let after = apply(&p, &ops);
    assert_eq!(
        clips_on(&after, V1),
        vec![
            (100, S - 5 * F, S),
            (101, 2 * S - 5 * F, S),
            (102, 3 * S - 5 * F, S)
        ]
    );
}

#[test]
fn u_nudge_no_op_cases() {
    let p = B::std()
        .clip(V1, 100, 5 * F, S)
        .clip(V2, 101, 4 * F, S)
        .build();
    assert_noop(Action::Nudge(1), &p, &ui());
    let mut u = ui();
    u.selection = vec![STALE];
    assert_eq!(act(Action::Nudge(1), &p, &mut u), vec![]);
    let mut u = ui();
    u.selection = vec![100, 101];
    assert_noop(Action::Nudge(-5), &p, &u); // c101 would start at -1 frame
    u.selection = vec![100];
    check(
        Action::Nudge(-5),
        &p,
        &mut u,
        edit(vec![mv(100, 0)]),
        |_| {},
    );
}

// ---------------------------------------------------------------------------------- U-ENABLE

fn set_enabled(clip_id: Id, enabled: bool) -> Command {
    Command::SetClip {
        clip_id,
        name: None,
        enabled: Some(enabled),
        reverse: None,
        blend: None,
        audio_filter: None,
        label: None,
    }
}

#[track_caller]
fn enable_cmds(p: &Project, u: &mut UiState) -> Vec<Command> {
    let before = u.clone();
    let ops = act(Action::ToggleEnable, p, u);
    assert_eq!(*u, before, "U-ENABLE changes no ui");
    match &ops[..] {
        [Op::Edit(c)] => sorted(c.clone()),
        _ => panic!("one Edit expected: {ops:?}"),
    }
}

#[test]
fn u_enable_toggles() {
    // c100 enabled linked to c101 disabled; c102 enabled; c103 disabled.
    let p = B::std()
        .clip(V1, 100, 0, S)
        .clip(A1, 101, 0, S)
        .link(&[100, 101], 900)
        .with(101, |c| c.enabled = false)
        .clip(V1, 102, S, S)
        .clip(V2, 103, 0, S)
        .with(103, |c| c.enabled = false)
        .build();
    let mut u = ui();
    u.selection = vec![100, 102];
    u.linked_selection = false;
    assert_eq!(
        enable_cmds(&p, &mut u),
        sorted(vec![set_enabled(100, false), set_enabled(102, false)])
    );
    // Linked selection pulls in the disabled c101: mixed -> all true.
    let mut u = ui();
    u.selection = vec![100, 102];
    let cmds = enable_cmds(&p, &mut u);
    assert_eq!(
        cmds,
        sorted(vec![
            set_enabled(100, true),
            set_enabled(101, true),
            set_enabled(102, true)
        ])
    );
    let mut u = ui();
    u.selection = vec![103, STALE];
    assert_eq!(enable_cmds(&p, &mut u), vec![set_enabled(103, true)]);
    // Applied: all disabled -> enabled.
    let mut u = ui();
    u.selection = vec![102];
    let ops = act(Action::ToggleEnable, &p, &mut u);
    assert!(!clip(&apply(&p, &ops), 102).enabled);
}

#[test]
fn u_enable_empty_no_op() {
    let p = B::std().clip(V1, 100, 0, S).build();
    assert_noop(Action::ToggleEnable, &p, &ui());
    let mut u = ui();
    u.selection = vec![STALE];
    assert_eq!(act(Action::ToggleEnable, &p, &mut u), vec![]);
}

// ------------------------------------------------------------------------------------ U-LINK

/// c100/c101 share link 900; c102 has link 901; c103, c104 unlinked; c105 alone with link 902.
fn link_project() -> Project {
    B::std()
        .clip(V1, 100, 0, S)
        .clip(A1, 101, 0, S)
        .link(&[100, 101], 900)
        .clip(V1, 102, S, S)
        .link(&[102], 901)
        .clip(V2, 103, 0, S)
        .clip(V2, 104, S, S)
        .clip(V1, 105, 2 * S, S)
        .link(&[105], 902)
        .build()
}

#[test]
fn u_link_toggle() {
    let p = link_project();
    let cases: Vec<(Vec<Id>, Vec<Op>)> = vec![
        (
            vec![100, 101],
            edit(vec![Command::UnlinkClips {
                clip_ids: vec![100, 101],
            }]),
        ),
        // One clip whose link is Some: all selected share it -> unlink.
        (
            vec![105],
            edit(vec![Command::UnlinkClips {
                clip_ids: vec![105],
            }]),
        ),
        // Not the effective selection: c101 is not added.
        (
            vec![100],
            edit(vec![Command::UnlinkClips {
                clip_ids: vec![100],
            }]),
        ),
        (
            vec![103, 104],
            edit(vec![Command::LinkClips {
                clip_ids: vec![103, 104],
            }]),
        ),
        (
            vec![100, 102],
            edit(vec![Command::LinkClips {
                clip_ids: vec![100, 102],
            }]),
        ),
        (
            vec![104, 100],
            edit(vec![Command::LinkClips {
                clip_ids: vec![104, 100],
            }]),
        ),
        (
            vec![STALE, 102],
            edit(vec![Command::UnlinkClips {
                clip_ids: vec![102],
            }]),
        ),
    ];
    for (sel, want) in cases {
        let mut u = ui();
        u.selection = sel.clone();
        let before = u.clone();
        assert_eq!(
            act(Action::ToggleLink, &p, &mut u),
            want,
            "selection {sel:?}"
        );
        assert_eq!(u, before);
    }
    for sel in [vec![], vec![103], vec![103, STALE]] {
        let mut u = ui();
        u.selection = sel;
        assert_eq!(act(Action::ToggleLink, &p, &mut u), vec![]);
    }
}

#[test]
fn u_link_applied() {
    let p = link_project();
    let mut u = ui();
    u.selection = vec![103, 104];
    let after = apply(&p, &act(Action::ToggleLink, &p, &mut u));
    let (a, b) = (clip(&after, 103).link, clip(&after, 104).link);
    assert!(a.is_some() && a == b);
    let mut u = ui();
    u.selection = vec![100, 101];
    let after = apply(&p, &act(Action::ToggleLink, &p, &mut u));
    assert_eq!(
        (clip(&after, 100).link, clip(&after, 101).link),
        (None, None)
    );
}

#[test]
fn u_link_group_and_ungroup() {
    let p = link_project();
    let mut u = ui();
    u.selection = vec![103, 100];
    let before = u.clone();
    assert_eq!(
        act(Action::Group, &p, &mut u),
        edit(vec![Command::GroupClips {
            clip_ids: vec![103, 100],
        }])
    );
    assert_eq!(
        act(Action::Ungroup, &p, &mut u),
        edit(vec![Command::UngroupClips {
            clip_ids: vec![103, 100],
        }])
    );
    assert_eq!(u, before);
    let mut u = ui();
    u.selection = vec![103];
    assert_noop(Action::Group, &p, &u);
    assert_eq!(
        act(Action::Ungroup, &p, &mut u),
        edit(vec![Command::UngroupClips {
            clip_ids: vec![103],
        }])
    );
    assert_noop(Action::Ungroup, &p, &ui());
    assert_noop(Action::Group, &p, &ui());
    let mut u = ui();
    u.selection = vec![103, STALE];
    assert_eq!(act(Action::Group, &p, &mut u), vec![]);
}

// ----------------------------------------------------------------------------------- U-TRANS

fn trans(track_id: Id, left: Option<Id>, right: Option<Id>, effect: &str, d: Time) -> Command {
    Command::AddTransition {
        track_id,
        left,
        right,
        effect: effect.into(),
        duration: d,
        params: BTreeMap::new(),
    }
}

/// Cut at 2 s. V1: P [0,2s) | Q [2s,6s). V2: R [2s-12F, 2s) ends only. A1: AP [0,2s) |
/// AQ [2s, 2s+10F). A2: AR [2s, 2s+15F) starts only.
fn trans_project() -> Project {
    B::std()
        .clip(V1, 100, 0, 2 * S)
        .clip(V1, 101, 2 * S, 4 * S)
        .clip(V2, 102, 2 * S - 12 * F, 12 * F)
        .clip(A1, 103, 0, 2 * S)
        .clip(A1, 104, 2 * S, 10 * F)
        .clip(A2, 105, 2 * S, 15 * F)
        .build()
}

fn video_cmds() -> Vec<Command> {
    vec![
        trans(V1, Some(100), Some(101), "cross_dissolve", S),
        trans(V2, Some(102), None, "cross_dissolve", 12 * F),
    ]
}

fn audio_cmds() -> Vec<Command> {
    vec![
        // min(1 s, 2 * 2 s, 2 * 10F) = 20F
        trans(A1, Some(103), Some(104), "crossfade", 20 * F),
        trans(A2, None, Some(105), "crossfade", 15 * F),
    ]
}

#[test]
fn u_trans_video_audio_default() {
    let p = trans_project();
    let mut u = ui();
    u.playhead = 2 * S;
    check(
        Action::ApplyVideoTransition,
        &p,
        &mut u,
        edit(video_cmds()),
        |_| {},
    );
    check(
        Action::ApplyAudioTransition,
        &p,
        &mut u,
        edit(audio_cmds()),
        |_| {},
    );
    let mut all = video_cmds();
    all.extend(audio_cmds());
    let ops = check(
        Action::ApplyDefaultTransitions,
        &p,
        &mut u,
        edit(all),
        |_| {},
    );
    let after = apply(&p, &ops);
    for (t, n) in [(V1, 1), (V2, 1), (A1, 1), (A2, 1)] {
        assert_eq!(after.track(t).unwrap().1.transitions.len(), n);
    }
}

#[test]
fn u_trans_short_clips_and_one_sided() {
    // Two-sided: min(1 s, 2 * 12F, 2 * 20F) = 24F. One-sided left: min(1 s, 7F) = 7F.
    let p = B::std()
        .clip(V1, 100, S, 12 * F)
        .clip(V1, 101, S + 12 * F, 20 * F)
        .clip(V2, 102, S + 5 * F, 7 * F)
        .clip(A1, 103, 0, S + 12 * F) // one-sided left, 1 s cap
        .clip(A2, 104, S + 12 * F, 4 * S) // one-sided right, 1 s cap
        .build();
    let mut u = ui();
    u.playhead = S + 12 * F;
    let ops = check(
        Action::ApplyDefaultTransitions,
        &p,
        &mut u,
        edit(vec![
            trans(V1, Some(100), Some(101), "cross_dissolve", 24 * F),
            trans(V2, Some(102), None, "cross_dissolve", 7 * F),
            trans(A1, Some(103), None, "crossfade", S),
            trans(A2, None, Some(104), "crossfade", S),
        ]),
        |_| {},
    );
    apply(&p, &ops);
}

#[test]
fn u_trans_targets_and_locked() {
    let p = trans_project();
    let mut u = ui();
    u.playhead = 2 * S;
    u.targets = vec![A2, V2];
    check(
        Action::ApplyDefaultTransitions,
        &p,
        &mut u,
        edit(vec![
            trans(V2, Some(102), None, "cross_dissolve", 12 * F),
            trans(A2, None, Some(105), "crossfade", 15 * F),
        ]),
        |_| {},
    );
    let locked = B { p: p.clone() }.lock(V1).lock(A1).build();
    let mut u = ui();
    u.playhead = 2 * S;
    u.targets = vec![V1, V2, A1];
    check(
        Action::ApplyVideoTransition,
        &locked,
        &mut u,
        edit(vec![trans(V2, Some(102), None, "cross_dissolve", 12 * F)]),
        |_| {},
    );
    assert_noop(Action::ApplyAudioTransition, &locked, &u);
}

#[test]
fn u_trans_no_cut_no_op() {
    let p = trans_project();
    let mut u = ui();
    u.playhead = S; // inside P, AP; nothing starts or ends
    for a in [
        Action::ApplyVideoTransition,
        Action::ApplyAudioTransition,
        Action::ApplyDefaultTransitions,
    ] {
        assert_noop(a, &p, &u);
    }
    u.playhead = 2 * S;
    u.targets = vec![V2];
    assert_noop(Action::ApplyAudioTransition, &p, &u);
    let empty = B::std().build();
    assert_noop(Action::ApplyDefaultTransitions, &empty, &ui());
}
