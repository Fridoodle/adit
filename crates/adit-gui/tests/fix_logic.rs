//! Pure-logic rules added after the GUI review: U-FIX-1 (locked tracks) and U-FIX-2 (edge clicks).
//! docs/GUI.md "Rules added after the GUI review (U-FIX-*)". Expected values come from the rule
//! text and hand arithmetic (30 fps, `F` = 23_520_000 flicks), never from the code under test.
mod common;
use common::*;

// ------------------------------------------------------------------------------- U-FIX-1

/// V1 (unlocked): clip 101 `[1s, 5s)`, enabled. V2 (locked): clip 102 `[1s, 5s)`, disabled.
/// Selection `[101, 102]`, playhead 2 s.
fn locked_mix() -> (Project, UiState) {
    let p = B::std()
        .clip(V1, 101, S, 4 * S)
        .clip(V2, 102, S, 4 * S)
        .with(102, |c| c.enabled = false)
        .lock(V2)
        .build();
    let mut u = ui();
    u.selection = vec![101, 102];
    u.playhead = 2 * S;
    (p, u)
}

/// V1: clip 101 `[1s, 5s)` linked (link 7) with A1 clip 201 `[1s, 5s)`; A1 locked.
/// Selection `[101]`, linked selection on (U-STATE default), playhead 2 s.
fn locked_partner() -> (Project, UiState) {
    let p = B::std()
        .clip(V1, 101, S, 4 * S)
        .clip(A1, 201, S, 4 * S)
        .link(&[101, 201], 7)
        .lock(A1)
        .build();
    let mut u = ui();
    u.selection = vec![101];
    u.playhead = 2 * S;
    assert!(u.linked_selection);
    (p, u)
}

fn body_drag(clip_id: Id, track_id: Id, from: Time, to: Time) -> Drag {
    Drag {
        from: Hit::Clip {
            clip_id,
            track_id,
            part: ClipPart::Body,
            time: from,
        },
        to_time: to,
        to_track: Some(track_id),
        shift: false,
        alt: false,
    }
}

fn split(clip_id: Id, time: Time) -> Command {
    Command::SplitClip {
        clip_id,
        time,
        include_linked: false,
    }
}

fn mv(clip_id: Id, start: Time) -> Command {
    Command::MoveClip {
        clip_id,
        start,
        track_id: None,
        mode: PlaceMode::Overwrite,
        include_linked: false,
    }
}

fn enable(clip_id: Id, on: bool) -> Command {
    Command::SetClip {
        clip_id,
        name: None,
        enabled: Some(on),
        reverse: None,
        blend: None,
        audio_filter: None,
    }
}

/// Debug text of every command in `ops` (to check an id is never named).
fn ops_text(ops: &[Op]) -> String {
    format!("{ops:?}")
}

#[test]
fn u_fix_1_add_edit_leaves_out_locked_clip() {
    let (p, mut u) = locked_mix();
    let ops = act(Action::AddEdit, &p, &mut u);
    // U-ADDEDIT over the effective selection minus 102 (locked V2): only 101 contains 2 s.
    assert_eq!(ops, edit(vec![split(101, 2 * S)]));
    apply(&p, &ops);
}

#[test]
fn u_fix_1_delete_leaves_out_locked_clip() {
    let (p, mut u) = locked_mix();
    let ops = act(Action::Delete, &p, &mut u);
    assert_eq!(
        ops,
        edit(vec![Command::RemoveClips {
            clip_ids: vec![101],
            include_linked: true,
        }])
    );
    assert!(u.selection.is_empty(), "U-DEL empties the selection");
    let q = apply(&p, &ops);
    assert_eq!(clips_on(&q, V1), vec![]);
    assert_eq!(clips_on(&q, V2), vec![(102, S, 4 * S)]);
}

#[test]
fn u_fix_1_ripple_delete_leaves_out_locked_clip() {
    let (p, mut u) = locked_mix();
    let ops = act(Action::RippleDelete, &p, &mut u);
    assert_eq!(
        ops,
        edit(vec![Command::RippleDeleteClips {
            clip_ids: vec![101],
            include_linked: true,
        }])
    );
    let q = apply(&p, &ops);
    assert_eq!(clips_on(&q, V2), vec![(102, S, 4 * S)]);
}

#[test]
fn u_fix_1_nudge_leaves_out_locked_clip() {
    let (p, mut u) = locked_mix();
    let ops = act(Action::Nudge(1), &p, &mut u);
    // U-NUDGE: start 1 s + one frame, no track change.
    assert_eq!(ops, edit(vec![mv(101, S + F)]));
    let q = apply(&p, &ops);
    assert_eq!(clips_on(&q, V1), vec![(101, S + F, 4 * S)]);
    assert_eq!(clips_on(&q, V2), vec![(102, S, 4 * S)]);
}

#[test]
fn u_fix_1_toggle_enable_counts_only_unlocked_clips() {
    let (p, mut u) = locked_mix();
    let ops = act(Action::ToggleEnable, &p, &mut u);
    // Effective selection = [101] (enabled) → "all enabled" → new value false. Counting the
    // disabled locked 102 would give true instead.
    assert_eq!(ops, edit(vec![enable(101, false)]));
    apply(&p, &ops);
}

#[test]
fn u_fix_1_select_move_leaves_out_locked_clip() {
    let (p, mut u) = locked_mix();
    // Body drag of 101 (selected) from 2 s to 3 s on V1: dt = 1 s.
    let ops = gesture(body_drag(101, V1, 2 * S, 3 * S), &p, &mut u);
    assert_eq!(ops, edit(vec![mv(101, 2 * S)]));
    let q = apply(&p, &ops);
    assert_eq!(clips_on(&q, V1), vec![(101, 2 * S, 4 * S)]);
}

#[test]
fn u_fix_1_linked_partner_on_locked_track_left_out_of_add_edit() {
    let (p, mut u) = locked_partner();
    let ops = act(Action::AddEdit, &p, &mut u);
    assert_eq!(ops, edit(vec![split(101, 2 * S)]));
    apply(&p, &ops);
}

#[test]
fn u_fix_1_linked_partner_on_locked_track_left_out_of_nudge() {
    let (p, mut u) = locked_partner();
    let ops = act(Action::Nudge(1), &p, &mut u);
    assert_eq!(ops, edit(vec![mv(101, S + F)]));
    let q = apply(&p, &ops);
    assert_eq!(clips_on(&q, A1), vec![(201, S, 4 * S)]);
}

#[test]
fn u_fix_1_linked_partner_on_locked_track_left_out_of_toggle_enable() {
    let (p, mut u) = locked_partner();
    let ops = act(Action::ToggleEnable, &p, &mut u);
    assert_eq!(ops, edit(vec![enable(101, false)]));
    apply(&p, &ops);
}

#[test]
fn u_fix_1_linked_partner_on_locked_track_left_out_of_move() {
    let (p, mut u) = locked_partner();
    let ops = gesture(body_drag(101, V1, 2 * S, 3 * S), &p, &mut u);
    assert_eq!(ops, edit(vec![mv(101, 2 * S)]));
    let q = apply(&p, &ops);
    assert_eq!(clips_on(&q, A1), vec![(201, S, 4 * S)]);
}

/// The exact command shape for removing a clip whose linked partner is locked is left to the
/// implementation (U-DEL's `include_linked: true` would make the core touch the locked partner),
/// so this checks the outcome: the core accepts the batch, 101 goes, 201 stays.
#[test]
fn u_fix_1_linked_partner_on_locked_track_left_out_of_delete() {
    for action in [Action::Delete, Action::RippleDelete] {
        let (p, mut u) = locked_partner();
        let ops = act(action, &p, &mut u);
        assert!(
            !ops_text(&ops).contains("201"),
            "{action:?}: no command names the locked partner 201: {ops:?}"
        );
        let q = apply(&p, &ops);
        assert_eq!(clips_on(&q, V1), vec![], "{action:?} removes 101");
        assert_eq!(
            clips_on(&q, A1),
            vec![(201, S, 4 * S)],
            "{action:?} keeps 201"
        );
    }
}

#[test]
fn u_fix_1_everything_locked_is_no_op() {
    let (p, mut u) = locked_mix();
    u.selection = vec![102];
    for action in [
        Action::AddEdit,
        Action::Delete,
        Action::RippleDelete,
        Action::Nudge(1),
        Action::Nudge(-1),
        Action::ToggleEnable,
    ] {
        let mut v = u.clone();
        let ops = act(action, &p, &mut v);
        assert_eq!(
            ops,
            vec![],
            "{action:?}: only locked clips selected → no op"
        );
    }
    let mut v = u.clone();
    let ops = gesture(body_drag(102, V2, 2 * S, 3 * S), &p, &mut v);
    assert_eq!(ops, vec![], "move of a locked clip → no op");
}

// ------------------------------------------------------------------------------- U-FIX-2

/// V1: clip 101 `[1s, 5s)` (400 px wide at 100 px/s, so it has edge zones).
fn one_clip() -> Project {
    B::std().clip(V1, 101, S, 4 * S).build()
}

/// Inside the 6 px edge zones at 100 px/s: one frame (3.3 px) from each edge.
const START_T: Time = S + F;
const END_T: Time = 5 * S - F;

fn edge(part: ClipPart, from: Time, to: Time) -> Drag {
    Drag {
        from: Hit::Clip {
            clip_id: 101,
            track_id: V1,
            part,
            time: from,
        },
        to_time: to,
        to_track: Some(V1),
        shift: false,
        alt: false,
    }
}

fn edge_click(part: ClipPart) -> Drag {
    let t = if part == ClipPart::StartEdge {
        START_T
    } else {
        END_T
    };
    edge(part, t, t)
}

fn with_tool(tool: Tool) -> UiState {
    let mut u = ui();
    u.tool = tool;
    u
}

#[test]
fn u_fix_2_select_click_on_edge_selects_without_trim() {
    let p = one_clip();
    for part in [ClipPart::StartEdge, ClipPart::EndEdge] {
        let u0 = with_tool(Tool::Select);
        let mut u = u0.clone();
        let ops = gesture(edge_click(part), &p, &mut u);
        assert_eq!(ops, vec![], "{part:?} click with Select never trims");
        let mut want = u0.clone();
        want.selection = vec![101];
        assert_eq!(u, want, "{part:?} click with Select selects the clip only");
    }
}

#[test]
fn u_fix_2_ripple_roll_rate_click_on_edge_is_no_op() {
    let p = one_clip();
    for tool in [Tool::Ripple, Tool::Roll, Tool::RateStretch] {
        for part in [ClipPart::StartEdge, ClipPart::EndEdge] {
            let u = with_tool(tool);
            let mut v = u.clone();
            let ops = gesture(edge_click(part), &p, &mut v);
            assert_eq!(ops, vec![], "{tool:?} click on {part:?} → no op");
            assert_eq!(v, u, "{tool:?} click on {part:?} leaves ui unchanged");
        }
    }
}

/// Real drags still trim (U-DRAG-TRIM / U-DRAG-RATE). Passes today; guards the fix.
#[test]
fn u_fix_2_real_edge_drags_still_trim() {
    let p = one_clip();
    let trim = |edge, to, mode| {
        edit(vec![Command::TrimClip {
            clip_id: 101,
            edge,
            to,
            mode,
            include_linked: true,
        }])
    };
    let cases = [
        (
            Tool::Select,
            edge(ClipPart::StartEdge, START_T, 2 * S),
            trim(Edge::Start, 2 * S, TrimMode::Normal),
        ),
        (
            Tool::Select,
            edge(ClipPart::EndEdge, END_T, 4 * S),
            trim(Edge::End, 4 * S, TrimMode::Normal),
        ),
        (
            Tool::Ripple,
            edge(ClipPart::EndEdge, END_T, 4 * S),
            trim(Edge::End, 4 * S, TrimMode::Ripple),
        ),
        (
            Tool::Roll,
            edge(ClipPart::StartEdge, START_T, 2 * S),
            trim(Edge::Start, 2 * S, TrimMode::Roll),
        ),
        // new = 3 s - 1 s = 2 s; speed = 1 * 4 s / 2 s = 2.
        (
            Tool::RateStretch,
            edge(ClipPart::EndEdge, END_T, 3 * S),
            edit(vec![Command::SetClipSpeed {
                clip_id: 101,
                speed: 2.0,
                ripple: false,
                include_linked: true,
            }]),
        ),
    ];
    for (tool, d, want) in cases {
        let mut u = with_tool(tool);
        assert_eq!(gesture(d, &p, &mut u), want, "{tool:?} {d:?}");
    }
}
