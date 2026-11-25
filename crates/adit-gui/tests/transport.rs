//! U-PLAY, U-JKL, U-STEP, U-EDITNAV, U-MARK, U-SELECT, U-MATCH: actions that change `ui` only.
mod common;
use common::*;

/// Runs `a`, requires no ops, and requires `ui` == `want(before)`.
#[track_caller]
fn only(a: Action, p: &Project, u: &mut UiState, want: impl FnOnce(&mut UiState)) {
    let mut expected = u.clone();
    want(&mut expected);
    assert_eq!(act(a, p, u), vec![], "{a:?} returns no op");
    assert_eq!(*u, expected, "{a:?}");
}

fn two_sec() -> Project {
    B::std().clip(V1, 100, 0, 2 * S).build()
}

// ------------------------------------------------------------------------------------- U-PLAY

#[test]
fn u_play_starts_and_stops() {
    let p = two_sec();
    let mut u = ui();
    u.playhead = S;
    only(Action::PlayPause, &p, &mut u, |w| {
        w.playing = true;
        w.shuttle = 1;
    });
    u.shuttle = 4;
    only(Action::PlayPause, &p, &mut u, |w| {
        w.playing = false;
        w.shuttle = 0;
    });
}

#[test]
fn u_play_at_or_after_end_restarts_from_zero() {
    let p = two_sec();
    for at in [2 * S, 3 * S] {
        let mut u = ui();
        u.playhead = at;
        only(Action::PlayPause, &p, &mut u, |w| {
            w.playing = true;
            w.shuttle = 1;
            w.playhead = 0;
        });
    }
    let mut u = ui();
    u.playhead = 2 * S - F;
    only(Action::PlayPause, &p, &mut u, |w| {
        w.playing = true;
        w.shuttle = 1;
    });
}

// -------------------------------------------------------------------------------------- U-JKL

#[test]
fn u_jkl_shuttle_right_doubles_and_caps() {
    let p = two_sec();
    for (from, to) in [
        (0, 1),
        (-4, 1),
        (-1, 1),
        (1, 2),
        (2, 4),
        (4, 8),
        (8, 8),
        (5, 8),
    ] {
        let mut u = ui();
        u.shuttle = from;
        u.playing = from != 0;
        u.playhead = S;
        only(Action::ShuttleRight, &p, &mut u, |w| {
            w.shuttle = to;
            w.playing = true;
        });
    }
}

#[test]
fn u_jkl_shuttle_left_doubles_and_caps() {
    let p = two_sec();
    for (from, to) in [
        (0, -1),
        (3, -1),
        (1, -1),
        (-1, -2),
        (-2, -4),
        (-4, -8),
        (-8, -8),
        (-5, -8),
    ] {
        let mut u = ui();
        u.shuttle = from;
        u.playing = from != 0;
        u.playhead = S;
        only(Action::ShuttleLeft, &p, &mut u, |w| {
            w.shuttle = to;
            w.playing = true;
        });
    }
}

#[test]
fn u_jkl_shuttle_stop() {
    let p = two_sec();
    let mut u = ui();
    u.shuttle = -4;
    u.playing = true;
    u.playhead = S;
    only(Action::ShuttleStop, &p, &mut u, |w| {
        w.shuttle = 0;
        w.playing = false;
    });
}

// ------------------------------------------------------------------------------------- U-STEP

#[track_caller]
fn step(a: Action, p: &Project, from: Time, to: Time) {
    let mut u = ui();
    u.playhead = from;
    u.playing = true;
    u.shuttle = 2;
    only(a, p, &mut u, |w| {
        w.playhead = to;
        w.playing = false;
        w.shuttle = 0;
    });
}

#[test]
fn u_step_moves_by_frames_and_stops_playback() {
    let p = two_sec();
    step(Action::Step(1), &p, 0, F);
    step(Action::Step(5), &p, S, S + 5 * F);
    step(Action::Step(-1), &p, S, S - F);
    step(Action::Step(-5), &p, S, S - 5 * F);
}

#[test]
fn u_step_clamps_at_zero_and_at_end() {
    let p = two_sec();
    step(Action::Step(-1), &p, 0, 0);
    step(Action::Step(-5), &p, 2 * F, 0);
    step(Action::Step(1), &p, 2 * S, 2 * S);
    step(Action::Step(5), &p, 2 * S - 2 * F, 2 * S);
    step(Action::Step(1), &p, 3 * S, 2 * S); // beyond the end: clamped back
    let empty = B::std().build();
    step(Action::Step(1), &empty, 0, 0);
}

#[test]
fn u_step_end_not_on_frame_boundary() {
    // duration 2 s + 100 flicks: frame(duration) = 60 -> last playhead 2 s.
    let p = B::std().clip(V1, 100, 0, 2 * S + 100).build();
    step(Action::Step(1), &p, 2 * S - F, 2 * S);
    step(Action::Step(1), &p, 2 * S, 2 * S);
    step(Action::GoToEnd, &p, 0, 2 * S);
    // duration 2 s - 100 flicks: frame(duration) = 59.
    let p = B::std().clip(V1, 100, 0, 2 * S - 100).build();
    step(Action::Step(5), &p, 57 * F, 59 * F);
    step(Action::GoToEnd, &p, 0, 59 * F);
    assert_eq!(59 * F, 1_387_680_000);
}

#[test]
fn u_step_ntsc() {
    // 10 s at 30000/1001: frame(10 s) = floor(7_056_000_000 / 23_543_520) = 299.
    let p = B::new(NTSC)
        .track(V1, TrackKind::Video)
        .clip(V1, 100, 0, 10 * S)
        .build();
    step(Action::Step(1), &p, 0, 23_543_520);
    step(Action::Step(5), &p, 297 * NF, 7_039_512_480);
    step(Action::Step(-1), &p, 297 * NF, 6_968_881_920);
    step(Action::GoToEnd, &p, 0, 7_039_512_480);
    step(Action::GoToStart, &p, 297 * NF, 0);
}

#[test]
fn u_step_go_to_start_and_end() {
    let p = two_sec();
    step(Action::GoToStart, &p, S, 0);
    step(Action::GoToEnd, &p, S, 2 * S);
    let empty = B::std().build();
    step(Action::GoToEnd, &empty, 0, 0);
}

// ---------------------------------------------------------------------------------- U-EDITNAV

/// Edit points {0, 1, 2, 3, 4, 5, 7} s; A2 is locked (still counts); the other sequence's clip
/// at 1.5 s does not count.
fn nav_project() -> Project {
    B::std()
        .clip(V1, 100, S, 2 * S)
        .clip(V1, 101, 3 * S, 2 * S)
        .clip(A1, 102, 2 * S, 2 * S)
        .clip(A2, 103, 5 * S, 2 * S)
        .lock(A2)
        .other_seq_clip(200, S + S / 2, S / 4)
        .build()
}

#[track_caller]
fn nav(a: Action, p: &Project, from: Time, to: Time) {
    step(a, p, from, to);
}

#[test]
fn u_editnav_next_and_prev() {
    let p = nav_project();
    nav(Action::NextEdit, &p, 0, S);
    nav(Action::NextEdit, &p, S, 2 * S); // not 1.5 s (other sequence)
    nav(Action::NextEdit, &p, 3 * S, 4 * S); // exactly on an edit point: strictly greater
    nav(Action::NextEdit, &p, 2 * S + S / 2, 3 * S);
    nav(Action::NextEdit, &p, 5 * S, 7 * S); // locked track counts
    nav(Action::PrevEdit, &p, 3 * S, 2 * S); // exactly on an edit point: strictly less
    nav(Action::PrevEdit, &p, 2 * S + F, 2 * S);
    nav(Action::PrevEdit, &p, S, 0);
    nav(Action::PrevEdit, &p, 10 * S, 7 * S);
}

#[test]
fn u_editnav_none_leaves_playhead_but_stops() {
    let p = nav_project();
    nav(Action::NextEdit, &p, 7 * S, 7 * S);
    nav(Action::NextEdit, &p, 9 * S, 9 * S);
    nav(Action::PrevEdit, &p, 0, 0);
    let empty = B::std().build();
    nav(Action::NextEdit, &empty, 0, 0);
    nav(Action::PrevEdit, &empty, 2 * S, 0); // 0 is always an edit point
}

// ------------------------------------------------------------------------------------- U-MARK

#[test]
fn u_mark_in() {
    let p = two_sec();
    let mut u = ui();
    u.playhead = S;
    u.mark_out = Some(S + F);
    only(Action::MarkIn, &p, &mut u, |w| w.mark_in = Some(S));
    // mark_out <= mark_in -> cleared (equal and before).
    for out in [S, S - F] {
        let mut u = ui();
        u.playhead = S;
        u.mark_out = Some(out);
        u.mark_in = Some(0);
        only(Action::MarkIn, &p, &mut u, |w| {
            w.mark_in = Some(S);
            w.mark_out = None;
        });
    }
}

#[test]
fn u_mark_out() {
    let p = two_sec();
    let mut u = ui();
    u.playhead = S;
    u.mark_in = Some(S - F);
    only(Action::MarkOut, &p, &mut u, |w| w.mark_out = Some(S));
    for inn in [S, S + F] {
        let mut u = ui();
        u.playhead = S;
        u.mark_in = Some(inn);
        u.mark_out = Some(2 * S);
        only(Action::MarkOut, &p, &mut u, |w| {
            w.mark_out = Some(S);
            w.mark_in = None;
        });
    }
}

/// V1 c100 [1s,3s) linked to A1 c102 [2s,6s); V2 c101 [0,2s); A2 (locked) c103 [0,9s).
fn mark_project() -> Project {
    B::std()
        .clip(V1, 100, S, 2 * S)
        .clip(A1, 102, 2 * S, 4 * S)
        .link(&[100, 102], 900)
        .clip(V2, 101, 0, 2 * S)
        .clip(A2, 103, 0, 9 * S)
        .lock(A2)
        .build()
}

#[test]
fn u_mark_clip_over_effective_selection() {
    let p = mark_project();
    let mut u = ui();
    u.playhead = 8 * S;
    u.selection = vec![100];
    only(Action::MarkClip, &p, &mut u, |w| {
        w.mark_in = Some(S);
        w.mark_out = Some(6 * S);
    });
    let mut u = ui();
    u.selection = vec![100];
    u.linked_selection = false;
    only(Action::MarkClip, &p, &mut u, |w| {
        w.mark_in = Some(S);
        w.mark_out = Some(3 * S);
    });
    // The selection wins over the playhead, and stale ids are ignored.
    let mut u = ui();
    u.selection = vec![STALE, 101];
    u.playhead = 2 * S + F;
    only(Action::MarkClip, &p, &mut u, |w| {
        w.mark_in = Some(0);
        w.mark_out = Some(2 * S);
    });
}

#[test]
fn u_mark_clip_under_playhead_on_targets() {
    let p = mark_project();
    // playhead 2 s: c100 [1,3) yes, c102 [2,6) yes (start inclusive), c101 [0,2) no (end
    // exclusive), c103 on locked A2 no.
    let mut u = ui();
    u.playhead = 2 * S;
    only(Action::MarkClip, &p, &mut u, |w| {
        w.mark_in = Some(S);
        w.mark_out = Some(6 * S);
    });
    let mut u = ui();
    u.playhead = 2 * S;
    u.targets = vec![V1];
    only(Action::MarkClip, &p, &mut u, |w| {
        w.mark_in = Some(S);
        w.mark_out = Some(3 * S);
    });
    // Only stale ids selected -> treated as empty -> clips under the playhead.
    let mut u = ui();
    u.playhead = 2 * S;
    u.selection = vec![STALE];
    u.targets = vec![V1, A2]; // A2 is locked: not a target
    only(Action::MarkClip, &p, &mut u, |w| {
        w.mark_in = Some(S);
        w.mark_out = Some(3 * S);
    });
}

#[test]
fn u_mark_clip_nothing_leaves_marks() {
    let p = mark_project();
    let mut u = ui();
    u.playhead = 7 * S; // only the locked A2 clip is there
    u.mark_in = Some(F);
    u.mark_out = Some(2 * F);
    only(Action::MarkClip, &p, &mut u, |_| {});
    let mut u = ui();
    u.playhead = 2 * S;
    u.targets = vec![A2];
    only(Action::MarkClip, &p, &mut u, |_| {});
}

#[test]
fn u_mark_clear_and_go_to() {
    let p = two_sec();
    let mut base = ui();
    base.playhead = F;
    base.mark_in = Some(S);
    base.mark_out = Some(S + 6 * F);
    let mut u = base.clone();
    only(Action::ClearIn, &p, &mut u, |w| w.mark_in = None);
    let mut u = base.clone();
    only(Action::ClearOut, &p, &mut u, |w| w.mark_out = None);
    let mut u = base.clone();
    only(Action::ClearInOut, &p, &mut u, |w| {
        w.mark_in = None;
        w.mark_out = None;
    });
    let mut u = base.clone();
    only(Action::GoToIn, &p, &mut u, |w| w.playhead = S);
    let mut u = base.clone();
    only(Action::GoToOut, &p, &mut u, |w| w.playhead = S + 6 * F);
    let mut u = ui();
    u.playhead = F;
    only(Action::GoToIn, &p, &mut u, |_| {});
    only(Action::GoToOut, &p, &mut u, |_| {});
}

// ----------------------------------------------------------------------------------- U-SELECT

/// Tracks V1, V2, A1, A2 (A2 locked). Clips under 2 s: c103 (A1), c101 (V2), c100 (V1),
/// c104 (A2, locked). c105 on V1 at [5s,6s); c106 on V1 [3s,4s) (time order differs from id).
fn select_project() -> Project {
    B::std()
        .clip(A1, 103, S, 2 * S)
        .clip(V2, 101, 0, 3 * S)
        .clip(V1, 105, 5 * S, S)
        .clip(V1, 100, S, 2 * S)
        .clip(V1, 106, 3 * S, S)
        .clip(A2, 104, 0, 3 * S)
        .lock(A2)
        .other_seq_clip(200, 0, 9 * S)
        .build()
}

#[test]
fn u_select_at_playhead_in_track_order() {
    let p = select_project();
    let mut u = ui();
    u.playhead = 2 * S;
    u.selection = vec![105];
    only(Action::SelectAtPlayhead, &p, &mut u, |w| {
        w.selection = vec![100, 101, 103]
    });
    // half-open: at 3 s c100 [1,3) and c103 [1,3) end, c106 [3,4) starts, c101 [0,3) ends.
    let mut u = ui();
    u.playhead = 3 * S;
    only(Action::SelectAtPlayhead, &p, &mut u, |w| {
        w.selection = vec![106]
    });
    // targets restrict; locked A2 listed in targets is ignored.
    let mut u = ui();
    u.playhead = 2 * S;
    u.targets = vec![A1, A2];
    only(Action::SelectAtPlayhead, &p, &mut u, |w| {
        w.selection = vec![103]
    });
    // nothing there -> empty selection.
    let mut u = ui();
    u.playhead = 8 * S;
    u.selection = vec![100];
    only(Action::SelectAtPlayhead, &p, &mut u, |w| {
        w.selection = vec![]
    });
}

#[test]
fn u_select_all_and_deselect_all() {
    let p = select_project();
    let mut u = ui();
    u.targets = vec![V1]; // SelectAll ignores targets
    only(Action::SelectAll, &p, &mut u, |w| {
        w.selection = vec![100, 106, 105, 101, 103, 104]
    });
    only(Action::DeselectAll, &p, &mut u, |w| w.selection = vec![]);
    let empty = B::std().build();
    let mut u = ui();
    only(Action::SelectAll, &empty, &mut u, |_| {});
}

// ------------------------------------------------------------------------------------ U-MATCH

/// V1 c100 [1s,5s) media 2, source_in 2 s, speed 2. V2 c101 [0,4s) is a Solid. A1 c102 media.
fn match_project() -> Project {
    B::std()
        .media(7, Some(100 * S), true, true)
        .clip(V1, 100, S, 4 * S)
        .with(100, |c| {
            c.source_in = 2 * S;
            c.speed = 2.0;
        })
        .clip(V2, 101, 0, 4 * S)
        .with(101, |c| {
            c.source = ClipSource::Solid {
                color: [0.0, 0.0, 0.0, 1.0],
            }
        })
        .clip(A1, 102, 0, 4 * S)
        .with(102, |c| c.source = ClipSource::Media { media_id: 7 })
        .build()
}

#[test]
fn u_match_frame_highest_target_video_with_media() {
    let p = match_project();
    let mut u = ui();
    u.playhead = 3 * S;
    u.source = Some(SourceState {
        media_id: 7,
        playhead: S,
        mark_in: Some(0),
        mark_out: Some(S),
    });
    // V2 has a Solid under the playhead, so V1's media clip: 2 s + (3 s - 1 s) * 2 = 6 s.
    only(Action::MatchFrame, &p, &mut u, |w| {
        w.source = Some(SourceState {
            media_id: MEDIA,
            playhead: 6 * S,
            mark_in: None,
            mark_out: None,
        })
    });
}

#[test]
fn u_match_frame_prefers_higher_track() {
    let p = B::std()
        .media(7, Some(100 * S), true, true)
        .clip(V1, 100, 0, 4 * S)
        .clip(V2, 101, S, 2 * S)
        .with(101, |c| {
            c.source = ClipSource::Media { media_id: 7 };
            c.source_in = S;
        })
        .build();
    let mut u = ui();
    u.playhead = 2 * S;
    only(Action::MatchFrame, &p, &mut u, |w| {
        w.source = Some(SourceState {
            media_id: 7,
            playhead: 2 * S,
            mark_in: None,
            mark_out: None,
        })
    });
    // V2 not targeted -> V1 (source_in 0, start 0): 2 s.
    let mut u = ui();
    u.playhead = 2 * S;
    u.targets = vec![V1, A1];
    only(Action::MatchFrame, &p, &mut u, |w| {
        w.source = Some(SourceState {
            media_id: MEDIA,
            playhead: 2 * S,
            mark_in: None,
            mark_out: None,
        })
    });
    // V2 locked -> V1.
    let locked = B { p: p.clone() }.lock(V2).build();
    let mut u = ui();
    u.playhead = 2 * S;
    only(Action::MatchFrame, &locked, &mut u, |w| {
        w.source = Some(SourceState {
            media_id: MEDIA,
            playhead: 2 * S,
            mark_in: None,
            mark_out: None,
        })
    });
}

#[test]
fn u_match_frame_none_unchanged() {
    let p = match_project();
    // 5 s: c100 [1,5) ended (half-open); only audio has media -> unchanged.
    let mut u = ui();
    u.playhead = 4 * S + S / 2;
    u.targets = vec![V2, A1];
    only(Action::MatchFrame, &p, &mut u, |_| {});
    let mut u = ui();
    u.playhead = 5 * S;
    only(Action::MatchFrame, &p, &mut u, |_| {});
}
