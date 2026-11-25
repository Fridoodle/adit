//! U-DRAG-*: timeline gestures.
mod common;
use common::*;

/// V1: c100 [1s,3s), c101 [4s,6s). V2: c102 [1s,2s). A1: c103 [1s,3s) linked to c100.
/// A2: c104 [4s,5s).
fn dp() -> Project {
    B::std()
        .clip(V1, 100, S, 2 * S)
        .clip(V1, 101, 4 * S, 2 * S)
        .clip(V2, 102, S, S)
        .clip(A1, 103, S, 2 * S)
        .link(&[100, 103], 900)
        .clip(A2, 104, 4 * S, S)
        .build()
}

fn track_of(clip_id: Id) -> Id {
    match clip_id {
        100 | 101 => V1,
        102 => V2,
        103 => A1,
        _ => A2,
    }
}

fn hit(clip_id: Id, part: ClipPart, time: Time) -> Hit {
    Hit::Clip {
        clip_id,
        track_id: track_of(clip_id),
        part,
        time,
    }
}

fn body(clip_id: Id, time: Time) -> Hit {
    hit(clip_id, ClipPart::Body, time)
}

fn empty(track_id: Id, time: Time) -> Hit {
    Hit::Empty { track_id, time }
}

/// A gesture released at `to_time` over `to_track`.
fn g(from: Hit, to_time: Time, to_track: Option<Id>) -> Drag {
    Drag {
        from,
        to_time,
        to_track,
        shift: false,
        alt: false,
    }
}

/// A click: released where pressed, on the pressed track.
fn click(from: Hit) -> Drag {
    let (time, track) = match from {
        Hit::Clip { time, track_id, .. } | Hit::Empty { time, track_id } => (time, Some(track_id)),
        Hit::Ruler { time } => (time, None),
        Hit::Header { track_id } => (0, Some(track_id)),
        Hit::Nothing => (0, None),
    };
    g(from, time, track)
}

fn shifted(mut d: Drag) -> Drag {
    d.shift = true;
    d
}

fn with_tool(tool: Tool) -> UiState {
    let mut u = ui();
    u.tool = tool;
    u
}

/// Runs the gesture; requires `ops == want_ops` and `ui == want(before)`.
#[track_caller]
fn check(
    d: Drag,
    p: &Project,
    u: &mut UiState,
    want_ops: Vec<Op>,
    want: impl FnOnce(&mut UiState),
) -> Vec<Op> {
    let mut expected = u.clone();
    want(&mut expected);
    let ops = gesture(d, p, u);
    assert_eq!(ops, want_ops, "{d:?}");
    assert_eq!(*u, expected, "{d:?}: ui");
    ops
}

const ALL_TOOLS: [Tool; 13] = [
    Tool::Select,
    Tool::TrackSelect,
    Tool::Ripple,
    Tool::Roll,
    Tool::RateStretch,
    Tool::Razor,
    Tool::Slip,
    Tool::Slide,
    Tool::Pen,
    Tool::Hand,
    Tool::Zoom,
    Tool::Type,
    Tool::Wand,
];

// ---------------------------------------------------------------------------------- general

#[test]
fn drag_missing_sequence_is_noop() {
    let p = dp();
    for tool in ALL_TOOLS {
        let mut u = with_tool(tool);
        u.sequence_id = STALE;
        u.selection = vec![101];
        u.playing = true;
        u.shuttle = 2;
        for d in [
            click(body(100, 2 * S)),
            g(body(100, 2 * S), 2 * S + 10 * F, Some(V1)),
            g(hit(100, ClipPart::EndEdge, 3 * S - 1), 2 * S, Some(V1)),
            click(empty(V1, 7 * S)),
            click(Hit::Ruler { time: 2 * S }),
            g(Hit::Ruler { time: 2 * S }, 3 * S, None),
        ] {
            assert_drag_noop(d, &p, &u);
        }
    }
}

#[test]
fn drag_header_and_nothing_do_nothing() {
    let p = dp();
    for tool in ALL_TOOLS {
        let mut u = with_tool(tool);
        u.selection = vec![101];
        u.playhead = F;
        for from in [Hit::Header { track_id: V1 }, Hit::Nothing] {
            assert_drag_noop(click(from), &p, &u);
            assert_drag_noop(g(from, 3 * S, Some(V2)), &p, &u);
        }
    }
}

// --------------------------------------------------------------------------------- U-DRAG-SEL

#[test]
fn u_drag_sel_click_selects() {
    let p = dp();
    let mut u = with_tool(Tool::Select);
    u.selection = vec![101];
    check(click(body(100, 2 * S)), &p, &mut u, vec![], |w| {
        w.selection = vec![100]
    });
    // Clicking an edge zone is still a click (dt == 0): selects.
    let mut u = with_tool(Tool::Select);
    check(
        click(hit(101, ClipPart::StartEdge, 4 * S)),
        &p,
        &mut u,
        vec![],
        |w| w.selection = vec![101],
    );
}

#[test]
fn u_drag_sel_shift_toggles() {
    let p = dp();
    let mut u = with_tool(Tool::Select);
    u.selection = vec![101];
    check(shifted(click(body(100, 2 * S))), &p, &mut u, vec![], |w| {
        w.selection = vec![101, 100]
    });
    check(shifted(click(body(101, 5 * S))), &p, &mut u, vec![], |w| {
        w.selection = vec![100]
    });
}

#[test]
fn u_drag_sel_click_empty_or_ruler() {
    let p = dp();
    let mut u = with_tool(Tool::Select);
    u.selection = vec![100, 101];
    u.playhead = F;
    check(click(empty(V1, 3 * S + S / 2)), &p, &mut u, vec![], |w| {
        w.selection = vec![]
    });
    let mut u = with_tool(Tool::Select);
    u.selection = vec![100];
    u.playing = true;
    u.shuttle = 2;
    check(
        click(Hit::Ruler { time: 2 * S + 100 }),
        &p,
        &mut u,
        vec![],
        |w| {
            w.selection = vec![];
            w.playhead = 2 * S;
            w.playing = false;
            w.shuttle = 0;
        },
    );
}

#[test]
fn u_drag_sel_ruler_with_any_tool() {
    let p = dp();
    for tool in ALL_TOOLS {
        // Click: playhead to the frame, playback stops (selection not checked for other tools).
        let mut u = with_tool(tool);
        u.playing = true;
        u.shuttle = -4;
        let ops = gesture(
            click(Hit::Ruler {
                time: 3 * S + F - 1,
            }),
            &p,
            &mut u,
        );
        assert_eq!(ops, vec![], "{tool:?}");
        let mut want = with_tool(tool);
        want.playhead = 3 * S;
        // Selection is specified for `Select` only (checked above); Zoom's click is not checked
        // against the ruler rule.
        want.selection = u.selection.clone();
        want.px_per_sec = u.px_per_sec;
        assert_eq!(u, want, "{tool:?}: ruler click");
        // Drag: playhead to the frame of the release time; nothing else.
        let mut u = with_tool(tool);
        u.selection = vec![100];
        let mut want = u.clone();
        want.playhead = 4 * S;
        let ops = gesture(g(Hit::Ruler { time: S }, 4 * S + 100, None), &p, &mut u);
        assert_eq!(ops, vec![], "{tool:?}");
        assert_eq!(u, want, "{tool:?}: ruler drag");
    }
}

// -------------------------------------------------------------------------------- U-DRAG-MOVE

fn mv(clip_id: Id, start: Time, track_id: Option<Id>) -> Command {
    Command::MoveClip {
        clip_id,
        start,
        track_id,
        mode: PlaceMode::Overwrite,
        include_linked: false,
    }
}

#[test]
fn u_drag_move_unselected_clip_selects_it_first() {
    let p = dp();
    let mut u = with_tool(Tool::Select);
    let ops = check(
        g(body(101, 5 * S), 5 * S + 10 * F, Some(V1)),
        &p,
        &mut u,
        edit(vec![mv(101, 4 * S + 10 * F, None)]),
        |w| w.selection = vec![101],
    );
    let after = apply(&p, &ops);
    assert_eq!(clips_on(&after, V1)[1], (101, 4 * S + 10 * F, 2 * S));
    // Selection [101] (not containing c100): becomes [100]; linked c103 moves too.
    let mut u = with_tool(Tool::Select);
    u.selection = vec![101];
    let ops = gesture(g(body(100, 2 * S), 2 * S + 3 * F, Some(V1)), &p, &mut u);
    assert_eq!(u.selection, vec![100]);
    let [Op::Edit(cmds)] = &ops[..] else {
        panic!("one Edit: {ops:?}")
    };
    // Same start: the order between them is not fixed by the rule.
    assert_eq!(
        sorted(cmds.clone()),
        sorted(vec![mv(100, S + 3 * F, None), mv(103, S + 3 * F, None)])
    );
}

#[test]
fn u_drag_move_track_change() {
    let p = dp();
    // dt == 0 with a track change is not a click.
    let mut u = with_tool(Tool::Select);
    let ops = check(
        g(body(101, 5 * S), 5 * S, Some(V2)),
        &p,
        &mut u,
        edit(vec![mv(101, 4 * S, Some(V2))]),
        |w| w.selection = vec![101],
    );
    let after = apply(&p, &ops);
    assert_eq!(clips_on(&after, V2), vec![(102, S, S), (101, 4 * S, 2 * S)]);
    assert_eq!(clips_on(&after, V1), vec![(100, S, 2 * S)]);
    // Different kind -> no track change.
    let mut u = with_tool(Tool::Select);
    check(
        g(body(101, 5 * S), 5 * S + 10 * F, Some(A1)),
        &p,
        &mut u,
        edit(vec![mv(101, 4 * S + 10 * F, None)]),
        |w| w.selection = vec![101],
    );
    // Released off the tracks.
    let mut u = with_tool(Tool::Select);
    check(
        g(body(101, 5 * S), 5 * S - 10 * F, None),
        &p,
        &mut u,
        edit(vec![mv(101, 4 * S - 10 * F, None)]),
        |w| w.selection = vec![101],
    );
    // Stale ids do not count: effective selection is exactly c101.
    let mut u = with_tool(Tool::Select);
    u.selection = vec![101, STALE];
    check(
        g(body(101, 5 * S), 5 * S + F, Some(V2)),
        &p,
        &mut u,
        edit(vec![mv(101, 4 * S + F, Some(V2))]),
        |_| {},
    );
}

#[test]
fn u_drag_move_linked_blocks_track_change() {
    let p = dp();
    let mut u = with_tool(Tool::Select);
    let ops = gesture(g(body(100, 2 * S), 2 * S + F, Some(V2)), &p, &mut u);
    let [Op::Edit(cmds)] = &ops[..] else {
        panic!("one Edit: {ops:?}")
    };
    assert_eq!(
        sorted(cmds.clone()),
        sorted(vec![mv(100, S + F, None), mv(103, S + F, None)])
    );
    // linked_selection off: c100 alone may change track (and overwrites c102 there).
    let mut u = with_tool(Tool::Select);
    u.linked_selection = false;
    let ops = check(
        g(body(100, 2 * S), 2 * S, Some(V2)),
        &p,
        &mut u,
        edit(vec![mv(100, S, Some(V2))]),
        |w| w.selection = vec![100],
    );
    let after = apply(&p, &ops);
    assert_eq!(clips_on(&after, V2), vec![(100, S, 2 * S)]);
    assert_eq!(clips_on(&after, A1), vec![(103, S, 2 * S)]);
    // Two selected clips: no track change.
    let mut u = with_tool(Tool::Select);
    u.linked_selection = false;
    u.selection = vec![100, 101];
    check(
        g(body(101, 5 * S), 5 * S + F, Some(V2)),
        &p,
        &mut u,
        edit(vec![mv(101, 4 * S + F, None), mv(100, S + F, None)]),
        |_| {},
    );
}

#[test]
fn u_drag_move_order_by_direction() {
    let p = dp();
    let mut u = with_tool(Tool::Select);
    u.linked_selection = false;
    u.selection = vec![100, 101];
    check(
        g(body(100, 2 * S), 2 * S + 10 * F, Some(V1)),
        &p,
        &mut u,
        edit(vec![
            mv(101, 4 * S + 10 * F, None),
            mv(100, S + 10 * F, None),
        ]),
        |_| {},
    );
    let mut u = with_tool(Tool::Select);
    u.linked_selection = false;
    u.selection = vec![101, 100];
    check(
        g(body(101, 5 * S), 5 * S - 10 * F, Some(V1)),
        &p,
        &mut u,
        edit(vec![
            mv(100, S - 10 * F, None),
            mv(101, 4 * S - 10 * F, None),
        ]),
        |_| {},
    );
}

#[test]
fn u_drag_move_adjacent_clips_all_survive() {
    // d1 [1,2) d2 [2,3) d3 [3,4) on V1, all selected.
    let p = B::std()
        .clip(V1, 100, S, S)
        .clip(V1, 101, 2 * S, S)
        .clip(V1, 102, 3 * S, S)
        .build();
    let mut u = with_tool(Tool::Select);
    u.selection = vec![101, 102, 100];
    let ops = check(
        g(body(101, 2 * S + S / 2), 2 * S + S / 2 + F, Some(V1)),
        &p,
        &mut u,
        edit(vec![
            mv(102, 3 * S + F, None),
            mv(101, 2 * S + F, None),
            mv(100, S + F, None),
        ]),
        |_| {},
    );
    assert_eq!(
        clips_on(&apply(&p, &ops), V1),
        vec![(100, S + F, S), (101, 2 * S + F, S), (102, 3 * S + F, S)]
    );
    let mut u = with_tool(Tool::Select);
    u.selection = vec![102, 100, 101];
    let ops = check(
        g(body(101, 2 * S + S / 2), 2 * S + S / 2 - F, Some(V1)),
        &p,
        &mut u,
        edit(vec![
            mv(100, S - F, None),
            mv(101, 2 * S - F, None),
            mv(102, 3 * S - F, None),
        ]),
        |_| {},
    );
    assert_eq!(
        clips_on(&apply(&p, &ops), V1),
        vec![(100, S - F, S), (101, 2 * S - F, S), (102, 3 * S - F, S)]
    );
}

#[test]
fn u_drag_move_before_zero_no_op() {
    let p = dp();
    let mut u = with_tool(Tool::Select);
    u.linked_selection = false;
    u.selection = vec![100, 101];
    assert_drag_noop(g(body(101, 5 * S), 4 * S - F, Some(V1)), &p, &u);
    // Exactly to 0 is fine.
    check(
        g(body(101, 5 * S), 4 * S, Some(V1)),
        &p,
        &mut u,
        edit(vec![mv(100, 0, None), mv(101, 3 * S, None)]),
        |_| {},
    );
}

#[test]
fn u_drag_move_select_empty_drag_no_op() {
    let p = dp();
    let mut u = with_tool(Tool::Select);
    u.selection = vec![100];
    assert_eq!(
        gesture(g(empty(V1, 7 * S), 8 * S, Some(V1)), &p, &mut u),
        vec![]
    );
}

// -------------------------------------------------------------------------------- U-DRAG-TRIM

fn trim(clip_id: Id, edge: Edge, to: Time, mode: TrimMode, linked: bool) -> Command {
    Command::TrimClip {
        clip_id,
        edge,
        to,
        mode,
        include_linked: linked,
    }
}

#[test]
fn u_drag_trim_modes() {
    let p = dp();
    for (tool, mode) in [
        (Tool::Select, TrimMode::Normal),
        (Tool::Ripple, TrimMode::Ripple),
        (Tool::Roll, TrimMode::Roll),
    ] {
        let mut u = with_tool(tool);
        u.selection = vec![102];
        check(
            g(hit(100, ClipPart::StartEdge, S), S + 5 * F, Some(V1)),
            &p,
            &mut u,
            edit(vec![trim(100, Edge::Start, S + 5 * F, mode, true)]),
            |_| {},
        );
        let mut u = with_tool(tool);
        u.linked_selection = false;
        check(
            g(hit(101, ClipPart::EndEdge, 6 * S - F), 5 * S, Some(V2)),
            &p,
            &mut u,
            edit(vec![trim(101, Edge::End, 5 * S, mode, false)]),
            |_| {},
        );
    }
}

#[test]
fn u_drag_trim_applied() {
    let p = dp();
    let mut u = with_tool(Tool::Select);
    let ops = gesture(
        g(hit(101, ClipPart::EndEdge, 6 * S - F), 5 * S, Some(V1)),
        &p,
        &mut u,
    );
    assert_eq!(clips_on(&apply(&p, &ops), V1)[1], (101, 4 * S, S));
    // Linked start trim moves both c100 and c103.
    let mut u = with_tool(Tool::Select);
    let ops = gesture(
        g(hit(100, ClipPart::StartEdge, S), 2 * S, Some(V1)),
        &p,
        &mut u,
    );
    let after = apply(&p, &ops);
    assert_eq!(clips_on(&after, V1)[0], (100, 2 * S, S));
    assert_eq!(clips_on(&after, A1), vec![(103, 2 * S, S)]);
}

#[test]
fn u_drag_trim_ripple_roll_body_no_op() {
    let p = dp();
    for tool in [Tool::Ripple, Tool::Roll] {
        let mut u = with_tool(tool);
        u.selection = vec![100];
        assert_drag_noop(g(body(100, 2 * S), 2 * S + 5 * F, Some(V1)), &p, &u);
    }
}

// -------------------------------------------------------------------------------- U-DRAG-RATE

fn speed(clip_id: Id, speed: f64, linked: bool) -> Command {
    Command::SetClipSpeed {
        clip_id,
        speed,
        ripple: false,
        include_linked: linked,
    }
}

#[test]
fn u_drag_rate_stretch() {
    let p = dp();
    let end = hit(100, ClipPart::EndEdge, 3 * S - F);
    // c100 [1s,3s) speed 1: new = to - 1 s; speed = 1 * 2 s / new.
    for (to, sp) in [
        (2 * S, 2.0),
        (5 * S, 0.5),
        (S + 14_112_000, 100.0),
        (S + 200 * S, 0.01),
    ] {
        let mut u = with_tool(Tool::RateStretch);
        check(
            g(end, to, Some(V1)),
            &p,
            &mut u,
            edit(vec![speed(100, sp, true)]),
            |_| {},
        );
    }
    let mut u = with_tool(Tool::RateStretch);
    u.linked_selection = false;
    let ops = check(
        g(end, 2 * S, Some(V1)),
        &p,
        &mut u,
        edit(vec![speed(100, 2.0, false)]),
        |_| {},
    );
    assert_eq!(clips_on(&apply(&p, &ops), V1)[0], (100, S, S));
    // Non-unit speed: 2 * 2 s / 1 s = 4.
    let fast = B { p: dp() }.with(101, |c| c.speed = 2.0).build();
    let mut u = with_tool(Tool::RateStretch);
    check(
        g(hit(101, ClipPart::EndEdge, 6 * S - F), 5 * S, Some(V1)),
        &fast,
        &mut u,
        edit(vec![speed(101, 4.0, true)]),
        |_| {},
    );
}

#[test]
fn u_drag_rate_stretch_no_op_cases() {
    let p = dp();
    let end = hit(100, ClipPart::EndEdge, 3 * S - F);
    let u = with_tool(Tool::RateStretch);
    for to in [S, S / 2, S + 10_000_000, S + 201 * S] {
        // new <= 0, new < 0, speed 141.12, speed 0.00995
        assert_drag_noop(g(end, to, Some(V1)), &p, &u);
    }
    assert_drag_noop(g(body(100, 2 * S), 3 * S, Some(V1)), &p, &u);
    assert_drag_noop(g(hit(100, ClipPart::StartEdge, S), 2 * S, Some(V1)), &p, &u);
}

// ------------------------------------------------------------------------------- U-DRAG-RAZOR

#[test]
fn u_drag_razor() {
    let p = dp();
    let mut u = with_tool(Tool::Razor);
    u.selection = vec![101];
    let ops = check(
        click(body(100, 2 * S)),
        &p,
        &mut u,
        edit(vec![Command::SplitClip {
            clip_id: 100,
            time: 2 * S,
            include_linked: true,
        }]),
        |_| {},
    );
    let after = apply(&p, &ops);
    assert_eq!(clips_on(&after, V1).len(), 3);
    assert_eq!(clips_on(&after, A1).len(), 2);
    let mut u = with_tool(Tool::Razor);
    u.linked_selection = false;
    check(
        click(body(100, 2 * S)),
        &p,
        &mut u,
        edit(vec![Command::SplitClip {
            clip_id: 100,
            time: 2 * S,
            include_linked: false,
        }]),
        |_| {},
    );
    let mut u = with_tool(Tool::Razor);
    check(
        shifted(click(body(101, 5 * S))),
        &p,
        &mut u,
        edit(vec![Command::SplitAt {
            sequence_id: SEQ,
            time: 5 * S,
            track_ids: None,
        }]),
        |_| {},
    );
}

#[test]
fn u_drag_razor_not_strictly_inside_no_op() {
    let p = dp();
    let u = with_tool(Tool::Razor);
    let at_start = click(hit(100, ClipPart::StartEdge, S));
    assert_drag_noop(at_start, &p, &u);
    assert_drag_noop(shifted(at_start), &p, &u);
    assert_drag_noop(click(empty(V1, 7 * S)), &p, &u);
}

// -------------------------------------------------------------------------------- U-DRAG-SLIP

#[test]
fn u_drag_slip_and_slide() {
    let p = dp();
    let mut u = with_tool(Tool::Slip);
    check(
        g(body(100, 2 * S), 2 * S + 10 * F, Some(V1)),
        &p,
        &mut u,
        edit(vec![Command::SlipClip {
            clip_id: 100,
            delta: -10 * F,
            include_linked: true,
        }]),
        |_| {},
    );
    let mut u = with_tool(Tool::Slip);
    u.linked_selection = false;
    check(
        g(body(101, 5 * S), 5 * S - 3 * F, Some(V1)),
        &p,
        &mut u,
        edit(vec![Command::SlipClip {
            clip_id: 101,
            delta: 3 * F,
            include_linked: false,
        }]),
        |_| {},
    );
    for dt in [10 * F, -4 * F] {
        let mut u = with_tool(Tool::Slide);
        check(
            g(body(100, 2 * S), 2 * S + dt, Some(V1)),
            &p,
            &mut u,
            edit(vec![Command::SlideClip {
                clip_id: 100,
                delta: dt,
            }]),
            |_| {},
        );
    }
    assert_drag_noop(click(body(100, 2 * S)), &p, &with_tool(Tool::Slip));
    assert_drag_noop(click(body(100, 2 * S)), &p, &with_tool(Tool::Slide));
}

// ---------------------------------------------------------------------------- U-DRAG-TRACKSEL

#[test]
fn u_drag_tracksel_single_track() {
    let p = dp();
    for (from, want) in [
        (body(101, 5 * S), vec![101]),
        (body(100, 2 * S), vec![100, 101]), // T = clip start, not the hit time
        (empty(V1, 3 * S + S / 2), vec![101]),
        (empty(V1, S / 2), vec![100, 101]),
        (empty(V1, 7 * S), vec![]),
        (empty(V2, S), vec![102]),
    ] {
        let mut u = with_tool(Tool::TrackSelect);
        u.selection = vec![104];
        check(click(from), &p, &mut u, vec![], |w| w.selection = want);
    }
}

#[test]
fn u_drag_tracksel_shift_all_tracks() {
    let p = B { p: dp() }.lock(A2).build(); // locked clips are still selected
    for (from, mut want) in [
        (empty(V1, S / 2), vec![100, 101, 102, 103, 104]),
        (body(101, 5 * S), vec![101, 104]), // c104 starts exactly at T = 4 s
        (empty(V2, 4 * S + 1), vec![]),
    ] {
        let mut u = with_tool(Tool::TrackSelect);
        u.selection = vec![102];
        let ops = gesture(shifted(click(from)), &p, &mut u);
        assert_eq!(ops, vec![]);
        let mut got = u.selection.clone();
        got.sort();
        want.sort();
        assert_eq!(got, want, "{from:?}");
        let mut want_ui = with_tool(Tool::TrackSelect);
        want_ui.selection = u.selection.clone();
        assert_eq!(u, want_ui, "only the selection changes");
    }
}

// -------------------------------------------------------------------------------- U-DRAG-VIEW

#[test]
fn u_drag_view_hand_scrolls() {
    let p = dp();
    for (scroll, from, to, want) in [
        (0, empty(V1, 3 * S), 2 * S, S),         // dt -1 s -> scroll + 1 s
        (5 * S, body(100, 2 * S), 4 * S, 3 * S), // dt +2 s
        (0, empty(V1, 3 * S), 4 * S, 0),         // clamped at 0
        (S, body(101, 5 * S), 5 * S + 3 * S, 0), // 1 s - 3 s -> 0
    ] {
        let mut u = with_tool(Tool::Hand);
        u.scroll = scroll;
        check(g(from, to, Some(V1)), &p, &mut u, vec![], |w| {
            w.scroll = want
        });
    }
}

#[test]
fn u_drag_view_zoom_click() {
    let p = dp();
    let mut u = with_tool(Tool::Zoom);
    check(click(empty(V1, 7 * S)), &p, &mut u, vec![], |w| {
        w.px_per_sec = 150.0
    });
    let mut u = with_tool(Tool::Zoom);
    let mut d = click(body(100, 2 * S));
    d.alt = true;
    check(d, &p, &mut u, vec![], |w| w.px_per_sec = 100.0 / 1.5);
    let mut u = with_tool(Tool::Zoom);
    u.px_per_sec = 1500.0;
    check(click(body(100, 2 * S)), &p, &mut u, vec![], |w| {
        w.px_per_sec = 2000.0
    });
}

// --------------------------------------------------------------------------------- U-DRAG-PEN

fn opacity_kf(clip_id: Id, time: Time, value: f64) -> Command {
    Command::SetKeyframe {
        target: ParamTarget {
            clip_id,
            effect_id: None,
            name: "opacity".into(),
        },
        time,
        value,
        easing: Easing::Linear,
    }
}

#[test]
fn u_drag_pen_keyframes_opacity() {
    // c100 opacity: 0 at local 0 -> 1 at local 2 s, linear.
    let p = B { p: dp() }
        .with(100, |c| {
            c.params.insert(
                "opacity".into(),
                Param {
                    value: 1.0,
                    keyframes: vec![
                        Keyframe {
                            time: 0,
                            value: 0.0,
                            easing: Easing::Linear,
                        },
                        Keyframe {
                            time: 2 * S,
                            value: 1.0,
                            easing: Easing::Linear,
                        },
                    ],
                },
            );
        })
        .build();
    let mut u = with_tool(Tool::Pen);
    let ops = check(
        click(body(100, 2 * S)),
        &p,
        &mut u,
        edit(vec![opacity_kf(100, S, 0.5)]),
        |_| {},
    );
    let after = apply(&p, &ops);
    let kfs: Vec<(Time, f64)> = clip(&after, 100).params["opacity"]
        .keyframes
        .iter()
        .map(|k| (k.time, k.value))
        .collect();
    assert_eq!(kfs, vec![(0, 0.0), (S, 0.5), (2 * S, 1.0)]);
    // No stored param: the default 1.0.
    let mut u = with_tool(Tool::Pen);
    check(
        click(body(101, 5 * S + F)),
        &p,
        &mut u,
        edit(vec![opacity_kf(101, S + F, 1.0)]),
        |_| {},
    );
    assert_drag_noop(click(empty(V1, 7 * S)), &p, &with_tool(Tool::Pen));
}

// -------------------------------------------------------------------------------- U-DRAG-WAND

#[test]
fn u_drag_wand_marks_and_prompts() {
    let p = dp();
    for (d, i, o, sel) in [
        (
            g(empty(V1, S / 2), 4 * S + S / 2, Some(V1)),
            S / 2,
            4 * S + S / 2,
            vec![100, 101],
        ),
        (
            g(empty(V1, 3 * S + S / 2), S / 2, Some(V2)),
            S / 2,
            3 * S + S / 2,
            vec![100],
        ),
        (
            g(body(101, 5 * S), 6 * S + S / 2, Some(V1)),
            5 * S,
            6 * S + S / 2,
            vec![101],
        ),
        (g(empty(V1, 3 * S), 4 * S, Some(V1)), 3 * S, 4 * S, vec![]), // half-open: touches only
        (
            g(empty(V2, S / 2), 4 * S + S / 2, Some(V2)),
            S / 2,
            4 * S + S / 2,
            vec![102],
        ),
        (click(body(100, 2 * S)), S, 3 * S, vec![100]), // a click marks the clip's range
    ] {
        let mut u = with_tool(Tool::Wand);
        u.selection = vec![104];
        check(d, &p, &mut u, vec![Op::WandPrompt], |w| {
            w.mark_in = Some(i);
            w.mark_out = Some(o);
            w.selection = sel;
        });
    }
}

#[test]
fn u_drag_wand_click_on_empty_does_nothing() {
    let p = dp();
    let mut u = with_tool(Tool::Wand);
    u.mark_in = Some(F);
    u.selection = vec![101];
    assert_drag_noop(click(empty(V1, 7 * S)), &p, &u);
}

// -------------------------------------------------------------------------------- U-DRAG-TYPE

#[test]
fn u_drag_type_does_nothing() {
    let p = dp();
    let mut u = with_tool(Tool::Type);
    u.selection = vec![101];
    for d in [
        click(body(100, 2 * S)),
        g(body(100, 2 * S), 2 * S + 5 * F, Some(V2)),
        g(hit(100, ClipPart::EndEdge, 3 * S - 1), 2 * S, Some(V1)),
        click(empty(V1, 7 * S)),
        g(empty(V1, 7 * S), 8 * S, Some(V1)),
    ] {
        assert_drag_noop(d, &p, &u);
    }
}
