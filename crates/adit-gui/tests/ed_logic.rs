//! Pure-logic rules of "Editing comfort": U-ED-1 .. U-ED-9 (docs/GUI.md).
//! Every fixture is 30 fps: `S` = 705_600_000 flicks, `F` = 23_520_000 flicks. Expected commands,
//! times and ui values come from the rule text and hand arithmetic, never from the code under test.
mod common;
use common::*;
use eframe::egui::Key;
use std::collections::{BTreeMap, HashSet};

/// 0.1 s.
const TOL: Time = 70_560_000;

fn item(track_id: Id, clip: Clip) -> ClipboardItem {
    ClipboardItem { track_id, clip }
}

/// The clipboard item of an existing project clip.
fn item_of(p: &Project, track_id: Id, id: Id) -> ClipboardItem {
    item(track_id, clip(p, id).clone())
}

/// `it` as it must be pasted: the same clip with only `start` replaced.
fn pasted(it: &ClipboardItem, start: Time) -> PasteItem {
    PasteItem {
        track_id: it.track_id,
        clip: Clip {
            start,
            ..it.clip.clone()
        },
    }
}

fn paste_ops(items: Vec<PasteItem>) -> Vec<Op> {
    edit(vec![Command::PasteClips {
        items,
        mode: PlaceMode::Overwrite,
    }])
}

fn label(clip_id: Id, n: u8) -> Command {
    Command::SetClip {
        clip_id,
        name: None,
        enabled: None,
        reverse: None,
        blend: None,
        audio_filter: None,
        label: Some(n),
    }
}

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

fn clip_hit(clip_id: Id, track_id: Id, time: Time) -> Hit {
    Hit::Clip {
        clip_id,
        track_id,
        part: ClipPart::Body,
        time,
    }
}

fn drag_of(from: Hit, to_time: Time, to_track: Id) -> Drag {
    Drag {
        from,
        to_time,
        to_track: Some(to_track),
        shift: false,
        alt: false,
    }
}

/// `paste` on a clone of `ui`: no ops, `ui` unchanged.
#[track_caller]
fn assert_paste_noop(items: &[ClipboardItem], p: &Project, ui: &UiState) {
    let mut u = ui.clone();
    assert_eq!(paste(items, p, &mut u), vec![], "U-ED-2: no op expected");
    assert_eq!(
        &u, ui,
        "U-ED-2: a paste that does nothing leaves ui unchanged"
    );
}

/// `context_menu` on a clone of `ui`: returns the items and requires `ui == want(before)`.
#[track_caller]
fn menu(
    hit: Hit,
    has_clipboard: bool,
    p: &Project,
    ui: &UiState,
    want: impl FnOnce(&mut UiState),
) -> Vec<MenuItem> {
    let mut expected = ui.clone();
    want(&mut expected);
    let mut u = ui.clone();
    let items = context_menu(hit, has_clipboard, p, &mut u);
    assert_eq!(u, expected, "U-ED-9: ui after context_menu({hit:?})");
    items
}

fn acts(actions: &[Action]) -> Vec<MenuItem> {
    actions.iter().map(|&a| MenuItem::Do(a)).collect()
}

// ------------------------------------------------------------------------------------- U-ED-1

/// V1: 101 `[0, 2s)`, 102 `[4s, 6s)`. V2: 103 `[1s, 3s)`. A1: 201 `[0, 2s)`.
/// 101 and 201 are linked (link 7).
fn copy_project() -> B {
    B::std()
        .clip(V1, 101, 0, 2 * S)
        .clip(V1, 102, 4 * S, 2 * S)
        .clip(V2, 103, S, 2 * S)
        .clip(A1, 201, 0, 2 * S)
        .link(&[101, 201], 7)
}

#[test]
fn u_ed_1_copy_is_effective_selection_in_track_then_start_order() {
    let p = copy_project().build();
    let mut u = ui();
    u.selection = vec![103, 102, 101];
    assert!(u.linked_selection);
    assert_eq!(
        copy(&p, &u),
        vec![
            item_of(&p, V1, 101),
            item_of(&p, V1, 102),
            item_of(&p, V2, 103),
            item_of(&p, A1, 201),
        ],
        "U-ED-1: track index ascending (V1, V2, A1), then start; 201 comes in through its link"
    );
}

#[test]
fn u_ed_1_copy_without_linked_selection_leaves_partner_out() {
    let p = copy_project().build();
    let mut u = ui();
    u.selection = vec![103, 102, 101];
    u.linked_selection = false;
    assert_eq!(
        copy(&p, &u),
        vec![
            item_of(&p, V1, 101),
            item_of(&p, V1, 102),
            item_of(&p, V2, 103)
        ],
        "U-ED-1: linked partners only with linked_selection"
    );
}

#[test]
fn u_ed_1_copy_ignores_stale_ids_and_empty_selection() {
    let p = copy_project().build();
    let mut u = ui();
    assert_eq!(copy(&p, &u), vec![], "U-ED-1: empty selection");
    u.selection = vec![STALE, 102];
    assert_eq!(
        copy(&p, &u),
        vec![item_of(&p, V1, 102)],
        "U-ED-1: stale ids ignored"
    );
    u.selection = vec![STALE];
    assert_eq!(copy(&p, &u), vec![], "U-ED-1: only stale ids");
}

#[test]
fn u_ed_1_copy_leaves_out_clips_on_locked_tracks() {
    // U-FIX-1: selected clip on a locked track.
    let p = copy_project().lock(V2).build();
    let mut u = ui();
    u.selection = vec![103, 102];
    assert_eq!(
        copy(&p, &u),
        vec![item_of(&p, V1, 102)],
        "U-ED-1 / U-FIX-1: 103 is on locked V2"
    );
    // U-FIX-1: linked partner on a locked track.
    let p = copy_project().lock(A1).build();
    u.selection = vec![101];
    assert_eq!(
        copy(&p, &u),
        vec![item_of(&p, V1, 101)],
        "U-ED-1 / U-FIX-1: linked partner 201 is on locked A1"
    );
}

#[test]
fn u_ed_1_copy_ignores_clips_of_other_sequences() {
    let p = copy_project().other_seq_clip(900, 0, S).build();
    let mut u = ui();
    u.selection = vec![900, 102];
    assert_eq!(
        copy(&p, &u),
        vec![item_of(&p, V1, 102)],
        "U-ED-1: only clips of the current sequence"
    );
}

#[test]
fn u_ed_1_perform_copy_returns_op_copy() {
    let p = copy_project().build();
    let mut u = ui();
    u.selection = vec![102];
    let before = u.clone();
    assert_eq!(act(Action::Copy, &p, &mut u), vec![Op::Copy], "U-ED-1");
    assert_eq!(u, before, "U-ED-1: Copy does not change ui");
}

#[test]
fn u_ed_1_perform_copy_with_empty_effective_selection_is_no_op() {
    let p = copy_project().lock(V2).build();
    let mut u = ui();
    assert_noop(Action::Copy, &p, &u);
    // Only a clip on a locked track: the effective selection is empty (U-FIX-1).
    u.selection = vec![103];
    assert_eq!(
        act(Action::Copy, &p, &mut u.clone()),
        vec![],
        "U-ED-1: locked"
    );
    u.selection = vec![STALE];
    assert_eq!(act(Action::Copy, &p, &mut u), vec![], "U-ED-1: stale");
}

#[test]
fn u_ed_1_cut_is_copy_then_delete() {
    let p = copy_project().build();
    let mut u = ui();
    u.selection = vec![102, STALE, 101];
    let mut want = u.clone();
    want.selection = vec![];
    let ops = act(Action::Cut, &p, &mut u);
    // U-DEL: RemoveClips { clip_ids: selection (stale ids left out), include_linked }.
    assert_eq!(
        ops,
        vec![
            Op::Copy,
            Op::Edit(vec![Command::RemoveClips {
                clip_ids: vec![102, 101],
                include_linked: true,
            }]),
        ],
        "U-ED-1: Cut = [Op::Copy, the Edit of Delete]"
    );
    assert_eq!(u, want, "U-ED-1: Cut empties the selection as Delete does");
    let after = apply(&p, &ops[1..]);
    assert_eq!(clips_on(&after, V1), vec![]);
    assert_eq!(clips_on(&after, A1), vec![], "linked 201 goes with 101");
    assert_eq!(clips_on(&after, V2), vec![(103, S, 2 * S)]);
}

#[test]
fn u_ed_1_cut_without_linked_selection() {
    let p = copy_project().build();
    let mut u = ui();
    u.selection = vec![101];
    u.linked_selection = false;
    let ops = act(Action::Cut, &p, &mut u);
    assert_eq!(
        ops,
        vec![
            Op::Copy,
            Op::Edit(vec![Command::RemoveClips {
                clip_ids: vec![101],
                include_linked: false,
            }]),
        ],
        "U-ED-1 / U-DEL: include_linked = linked_selection"
    );
    assert!(u.selection.is_empty(), "U-ED-1: selection emptied");
}

#[test]
fn u_ed_1_cut_leaves_out_locked_clips_like_delete() {
    // U-FIX-1: 103 on locked V2 is left out.
    let p = copy_project().lock(V2).build();
    let mut u = ui();
    u.selection = vec![102, 103];
    assert_eq!(
        act(Action::Cut, &p, &mut u),
        vec![
            Op::Copy,
            Op::Edit(vec![Command::RemoveClips {
                clip_ids: vec![102],
                include_linked: true,
            }]),
        ],
        "U-ED-1 / U-FIX-1"
    );
    assert!(u.selection.is_empty(), "U-ED-1: selection emptied");
    // U-FIX-14: partner on a locked track -> remaining clips named, include_linked false.
    let p = copy_project().lock(A1).build();
    let mut u = ui();
    u.selection = vec![101];
    assert_eq!(
        act(Action::Cut, &p, &mut u),
        vec![
            Op::Copy,
            Op::Edit(vec![Command::RemoveClips {
                clip_ids: vec![101],
                include_linked: false,
            }]),
        ],
        "U-ED-1 / U-FIX-14"
    );
}

#[test]
fn u_ed_1_cut_with_empty_selection_is_no_op() {
    let p = copy_project().lock(V2).build();
    let mut u = ui();
    assert_noop(Action::Cut, &p, &u);
    u.selection = vec![103];
    assert_eq!(
        act(Action::Cut, &p, &mut u.clone()),
        vec![],
        "U-ED-1: only a locked clip selected"
    );
    u.selection = vec![STALE];
    assert_eq!(act(Action::Cut, &p, &mut u), vec![], "U-ED-1: stale");
}

#[test]
fn u_ed_1_perform_paste_always_returns_op_paste() {
    let p = copy_project().build();
    let mut u = ui();
    let before = u.clone();
    assert_eq!(
        act(Action::Paste, &p, &mut u),
        vec![Op::Paste],
        "U-ED-1: empty selection"
    );
    assert_eq!(u, before, "U-ED-1: Paste itself does not change ui");
    u.selection = vec![101];
    u.playhead = 3 * S;
    let before = u.clone();
    assert_eq!(act(Action::Paste, &p, &mut u), vec![Op::Paste], "U-ED-1");
    assert_eq!(u, before, "U-ED-1: Paste itself does not change ui");
    // No tracks at all.
    let empty = B::new(FPS30).build();
    assert_eq!(act(Action::Paste, &empty, &mut ui()), vec![Op::Paste]);
}

#[test]
fn u_ed_1_keymap_has_copy_cut_paste_and_no_duplicate_chord() {
    let got = default_keymap();
    let ctrl = |key| KeyChord {
        key,
        ctrl: true,
        shift: false,
        alt: false,
    };
    let plain = |key| KeyChord {
        key,
        ctrl: false,
        shift: false,
        alt: false,
    };
    for (chord, action) in [
        (ctrl(Key::C), Action::Copy),
        (ctrl(Key::X), Action::Cut),
        (ctrl(Key::V), Action::Paste),
        // U-KEYS: the plain keys keep their meaning.
        (plain(Key::C), Action::SetTool(Tool::Razor)),
        (plain(Key::X), Action::MarkClip),
        (plain(Key::V), Action::SetTool(Tool::Select)),
    ] {
        let found: Vec<_> = got.iter().filter(|(c, _)| *c == chord).collect();
        assert_eq!(
            found,
            vec![&(chord, action)],
            "U-ED-1 / U-KEYS: {chord:?} must map to {action:?}"
        );
    }
    for a in [Action::Copy, Action::Cut, Action::Paste] {
        assert_eq!(
            got.iter().filter(|(_, x)| *x == a).count(),
            1,
            "U-ED-1: exactly one key for {a:?}"
        );
    }
    let mut seen = HashSet::new();
    for (chord, _) in &got {
        assert!(seen.insert(*chord), "U-KEYS: chord {chord:?} appears twice");
    }
    // U-KEYS table (73 entries) plus the three of U-ED-1.
    assert_eq!(got.len(), 76, "U-ED-1: 73 + 3 keymap entries");
}

// ------------------------------------------------------------------------------------- U-ED-2

/// V1: 101 `[2s, 4s)`. A1: 201 `[2s, 4s)`. V2 and A2 empty.
fn paste_project() -> B {
    B::std()
        .clip(V1, 101, 2 * S, 2 * S)
        .clip(A1, 201, 2 * S, 2 * S)
}

#[test]
fn u_ed_2_paste_offsets_order_playhead_and_selection() {
    let p = paste_project().build();
    // Given order: the later item first. base = 2 s, span = 6 s - 2 s = 4 s.
    let a = item(A1, mk_clip(201, 3 * S, 3 * S));
    let v = item(V1, mk_clip(101, 2 * S, 2 * S));
    let mut u = ui();
    u.playhead = 10 * S;
    u.selection = vec![101];
    let mut want = u.clone();
    want.playhead = 14 * S; // 10 s + span 4 s, on a frame
    want.selection = vec![];
    let ops = paste(&[a.clone(), v.clone()], &p, &mut u);
    assert_eq!(
        ops,
        // 10 s + (3 s - 2 s) = 11 s; 10 s + (2 s - 2 s) = 10 s.
        paste_ops(vec![pasted(&a, 11 * S), pasted(&v, 10 * S)]),
        "U-ED-2: start = playhead + (clip.start - base), order as given, mode overwrite"
    );
    assert_eq!(
        u, want,
        "U-ED-2: playhead = at(frame(playhead + span)), selection emptied"
    );
    // C-PASTE-2: new clip ids in item order from next_id (100_000).
    let after = apply(&p, &ops);
    assert_eq!(
        clips_on(&after, A1),
        vec![(201, 2 * S, 2 * S), (100_000, 11 * S, 3 * S)]
    );
    assert_eq!(
        clips_on(&after, V1),
        vec![(101, 2 * S, 2 * S), (100_001, 10 * S, 2 * S)]
    );
}

#[test]
fn u_ed_2_paste_single_item_lands_on_playhead() {
    let p = paste_project().build();
    let v = item(V1, mk_clip(101, 7 * S, 2 * S));
    let mut u = ui();
    u.playhead = 3 * S;
    let ops = paste(std::slice::from_ref(&v), &p, &mut u);
    assert_eq!(
        ops,
        paste_ops(vec![pasted(&v, 3 * S)]),
        "U-ED-2: base = 7 s"
    );
    assert_eq!(u.playhead, 5 * S, "U-ED-2: 3 s + span 2 s");
    // C-PASTE-3 overwrite: 101 [2s, 4s) is cut back to [2s, 3s).
    let after = apply(&p, &ops);
    assert_eq!(
        clips_on(&after, V1),
        vec![(101, 2 * S, S), (100_000, 3 * S, 2 * S)]
    );
}

#[test]
fn u_ed_2_paste_playhead_lands_on_a_frame() {
    let p = paste_project().build();
    // Duration 2 s + 1000 flicks: playhead + span = 12 s + 1000 -> frame 360 -> 12 s.
    let v = item(V2, mk_clip(101, 2 * S, 2 * S + 1000));
    let mut u = ui();
    u.playhead = 10 * S;
    let ops = paste(std::slice::from_ref(&v), &p, &mut u);
    assert_eq!(ops, paste_ops(vec![pasted(&v, 10 * S)]), "U-ED-2");
    assert_eq!(
        u.playhead,
        12 * S,
        "U-ED-2: playhead = at(frame(playhead + span))"
    );
}

#[test]
fn u_ed_2_paste_leaves_out_locked_missing_and_foreign_tracks() {
    let p = paste_project().lock(V2).other_seq_clip(900, 0, S).build();
    let keep = item(V1, mk_clip(101, 2 * S, 2 * S));
    let locked = item(V2, mk_clip(103, 2 * S, 2 * S));
    let foreign = item(OTHER_TRACK, mk_clip(900, 2 * S, 2 * S));
    let gone = item(STALE, mk_clip(104, 2 * S, 2 * S));
    let mut u = ui();
    u.playhead = 6 * S;
    let ops = paste(&[locked, keep.clone(), foreign, gone], &p, &mut u);
    assert_eq!(
        ops,
        paste_ops(vec![pasted(&keep, 6 * S)]),
        "U-ED-2: items on locked tracks or tracks not in the sequence are left out"
    );
    assert_eq!(u.playhead, 8 * S, "U-ED-2: 6 s + span 2 s");
}

#[test]
fn u_ed_2_paste_base_and_span_cover_left_out_items() {
    let p = paste_project().lock(V2).build();
    // Left-out item is the earliest: base = 1 s, span = 5 s - 1 s = 4 s.
    let locked = item(V2, mk_clip(103, S, S));
    let keep = item(V1, mk_clip(101, 3 * S, 2 * S));
    let mut u = ui();
    u.playhead = 10 * S;
    let ops = paste(&[locked, keep.clone()], &p, &mut u);
    assert_eq!(
        ops,
        // 10 s + (3 s - 1 s) = 12 s.
        paste_ops(vec![pasted(&keep, 12 * S)]),
        "U-ED-2: base is the smallest start of all items"
    );
    assert_eq!(u.playhead, 14 * S, "U-ED-2: 10 s + span 4 s");

    // Left-out item is the latest: base = 2 s, span = 9 s - 2 s = 7 s.
    let locked = item(V2, mk_clip(103, 3 * S, 6 * S));
    let keep = item(V1, mk_clip(101, 2 * S, 2 * S));
    let mut u = ui();
    u.playhead = S;
    let ops = paste(&[keep.clone(), locked], &p, &mut u);
    assert_eq!(ops, paste_ops(vec![pasted(&keep, S)]), "U-ED-2");
    assert_eq!(
        u.playhead,
        8 * S,
        "U-ED-2: span is computed over all items (1 s + 7 s)"
    );
}

#[test]
fn u_ed_2_paste_nothing_left_is_no_op() {
    let p = paste_project().lock(V2).other_seq_clip(900, 0, S).build();
    let mut u = ui();
    u.playhead = 6 * S;
    u.selection = vec![101];
    assert_paste_noop(&[], &p, &u);
    assert_paste_noop(&[item(V2, mk_clip(103, 2 * S, 2 * S))], &p, &u);
    assert_paste_noop(
        &[
            item(V2, mk_clip(103, 2 * S, 2 * S)),
            item(OTHER_TRACK, mk_clip(900, 0, S)),
            item(STALE, mk_clip(104, 0, S)),
        ],
        &p,
        &u,
    );
}

#[test]
fn u_ed_2_paste_missing_sequence_is_no_op() {
    let p = paste_project().build();
    let mut u = ui();
    u.sequence_id = STALE;
    u.playhead = 6 * S;
    u.selection = vec![101];
    assert_paste_noop(&[item(V1, mk_clip(101, 2 * S, 2 * S))], &p, &u);
    assert_paste_noop(&[], &p, &u);
}

// ------------------------------------------------------------------------------------- U-ED-3

/// V1: 101 `[0, 2s)` (link 7), 102 `[4s, 6s)`. V2: 103 `[1s, 3s)` (link 8).
/// A1: 201 `[0, 2s)` (link 7). A2: 203 `[1s, 3s)` (link 8).
fn unlink_project() -> B {
    B::std()
        .clip(V1, 101, 0, 2 * S)
        .clip(V1, 102, 4 * S, 2 * S)
        .clip(V2, 103, S, 2 * S)
        .clip(A1, 201, 0, 2 * S)
        .clip(A2, 203, S, 2 * S)
        .link(&[101, 201], 7)
        .link(&[103, 203], 8)
}

#[test]
fn u_ed_3_unlink_names_linked_clips_in_track_order() {
    let p = unlink_project().build();
    let mut u = ui();
    u.selection = vec![103, 102, 101];
    let before = u.clone();
    let ops = act(Action::Unlink, &p, &mut u);
    assert_eq!(
        ops,
        edit(vec![Command::UnlinkClips {
            clip_ids: vec![101, 103, 201, 203],
        }]),
        "U-ED-3: effective-selection clips with a link, track order then start; 102 has none"
    );
    assert_eq!(u, before, "U-ED-3: selection (and ui) unchanged");
    let after = apply(&p, &ops);
    for id in [101, 103, 201, 203] {
        assert_eq!(clip(&after, id).link, None, "C-LINK-1: {id} unlinked");
    }
}

#[test]
fn u_ed_3_unlink_without_linked_selection() {
    let p = unlink_project().build();
    let mut u = ui();
    u.selection = vec![103, 101, 102];
    u.linked_selection = false;
    let before = u.clone();
    assert_eq!(
        act(Action::Unlink, &p, &mut u),
        edit(vec![Command::UnlinkClips {
            clip_ids: vec![101, 103],
        }]),
        "U-ED-3: only the selected clips that have a link"
    );
    assert_eq!(u, before, "U-ED-3: selection unchanged");
}

#[test]
fn u_ed_3_unlink_orders_by_start_within_a_track() {
    // V1: 101 [0, 2s) and 102 [4s, 6s), both linked to audio.
    let p = B::std()
        .clip(V1, 101, 0, 2 * S)
        .clip(V1, 102, 4 * S, 2 * S)
        .clip(A1, 201, 0, 2 * S)
        .clip(A1, 202, 4 * S, 2 * S)
        .link(&[101, 201], 7)
        .link(&[102, 202], 8)
        .build();
    let mut u = ui();
    u.selection = vec![202, 101];
    assert_eq!(
        act(Action::Unlink, &p, &mut u),
        edit(vec![Command::UnlinkClips {
            clip_ids: vec![101, 102, 201, 202],
        }]),
        "U-ED-3: track order, then start"
    );
}

#[test]
fn u_ed_3_unlink_leaves_out_locked_partner() {
    let p = unlink_project().lock(A1).build();
    let mut u = ui();
    u.selection = vec![101, 103];
    assert_eq!(
        act(Action::Unlink, &p, &mut u),
        edit(vec![Command::UnlinkClips {
            clip_ids: vec![101, 103, 203],
        }]),
        "U-ED-3 / U-FIX-1: 201 is on locked A1"
    );
}

#[test]
fn u_ed_3_unlink_nothing_linked_is_no_op() {
    let p = unlink_project().lock(V2).build();
    let mut u = ui();
    assert_noop(Action::Unlink, &p, &u);
    u.selection = vec![102];
    assert_noop(Action::Unlink, &p, &u);
    u.selection = vec![STALE];
    assert_eq!(act(Action::Unlink, &p, &mut u), vec![], "U-ED-3: stale");
}

// ------------------------------------------------------------------------------------- U-ED-4

#[test]
fn u_ed_4_set_label_one_set_clip_per_clip_in_one_batch() {
    let p = copy_project().build();
    let mut u = ui();
    u.selection = vec![103, 102, 101];
    let before = u.clone();
    let ops = act(Action::SetLabel(3), &p, &mut u);
    assert_eq!(
        ops,
        edit(vec![
            label(101, 3),
            label(102, 3),
            label(103, 3),
            label(201, 3)
        ]),
        "U-ED-4: one SetClip {{ label }} per effective-selection clip, track order then start"
    );
    assert_eq!(u, before, "U-ED-4: ui unchanged");
    let after = apply(&p, &ops);
    for id in [101, 102, 103, 201] {
        assert_eq!(clip(&after, id).label, 3, "C-LABEL-1: clip {id}");
    }
}

#[test]
fn u_ed_4_set_label_0_and_8_are_valid() {
    let p = copy_project().with(102, |c| c.label = 5).build();
    let mut u = ui();
    u.selection = vec![102];
    let ops = act(Action::SetLabel(0), &p, &mut u);
    assert_eq!(
        ops,
        edit(vec![label(102, 0)]),
        "U-ED-4: 0 = none is allowed"
    );
    assert_eq!(clip(&apply(&p, &ops), 102).label, 0);
    let ops = act(Action::SetLabel(8), &p, &mut u);
    assert_eq!(
        ops,
        edit(vec![label(102, 8)]),
        "U-ED-4: 8 is the last label"
    );
    assert_eq!(clip(&apply(&p, &ops), 102).label, 8);
}

#[test]
fn u_ed_4_set_label_above_8_is_no_op() {
    let p = copy_project().build();
    let mut u = ui();
    u.selection = vec![102];
    assert_noop(Action::SetLabel(9), &p, &u);
    assert_noop(Action::SetLabel(255), &p, &u);
}

#[test]
fn u_ed_4_set_label_empty_selection_is_no_op() {
    let p = copy_project().lock(V2).build();
    let mut u = ui();
    assert_noop(Action::SetLabel(1), &p, &u);
    u.selection = vec![103];
    assert_eq!(
        act(Action::SetLabel(1), &p, &mut u.clone()),
        vec![],
        "U-ED-4 / U-FIX-1: only a locked clip"
    );
    u.selection = vec![STALE];
    assert_eq!(
        act(Action::SetLabel(1), &p, &mut u),
        vec![],
        "U-ED-4: stale"
    );
}

#[test]
fn u_ed_4_set_label_leaves_out_locked_and_unlinked_clips() {
    let p = copy_project().lock(A1).build();
    let mut u = ui();
    u.selection = vec![101];
    assert_eq!(
        act(Action::SetLabel(2), &p, &mut u),
        edit(vec![label(101, 2)]),
        "U-ED-4 / U-FIX-1: partner 201 on locked A1 left out"
    );
    let p = copy_project().build();
    u.linked_selection = false;
    assert_eq!(
        act(Action::SetLabel(2), &p, &mut u),
        edit(vec![label(101, 2)]),
        "U-ED-4: no linked partner without linked_selection"
    );
}

// ------------------------------------------------------------------------------------- U-ED-5

/// V1: 101 `[2s, 4s)`, 102 `[6s, 8s)`. Other tracks empty.
fn gap_project() -> B {
    B::std()
        .clip(V1, 101, 2 * S, 2 * S)
        .clip(V1, 102, 6 * S, 2 * S)
}

fn close_gap(track_id: Id, time: Time) -> Action {
    Action::CloseGap { track_id, time }
}

#[test]
fn u_ed_5_close_gap_between_clips() {
    let p = gap_project().build();
    let mut u = ui();
    u.selection = vec![101];
    let before = u.clone();
    let ops = act(close_gap(V1, 5 * S), &p, &mut u);
    assert_eq!(
        ops,
        edit(vec![Command::CloseGap {
            track_id: V1,
            time: 5 * S,
        }]),
        "U-ED-5"
    );
    assert_eq!(u, before, "U-ED-5: ui unchanged");
    // C-GAP-1: gap [4s, 6s) -> 102 moves to 4 s.
    assert_eq!(
        clips_on(&apply(&p, &ops), V1),
        vec![(101, 2 * S, 2 * S), (102, 4 * S, 2 * S)]
    );
}

#[test]
fn u_ed_5_close_gap_before_first_clip_and_at_a_clip_end() {
    let p = gap_project().build();
    let ops = act(close_gap(V1, S), &p, &mut ui());
    assert_eq!(
        ops,
        edit(vec![Command::CloseGap {
            track_id: V1,
            time: S,
        }]),
        "U-ED-5: 1 s is inside no clip and 101 starts after it"
    );
    // C-GAP-1: gap [0, 2s).
    assert_eq!(
        clips_on(&apply(&p, &ops), V1),
        vec![(101, 0, 2 * S), (102, 4 * S, 2 * S)]
    );
    // 4 s is the (exclusive) end of 101: not inside a clip.
    assert_eq!(
        act(close_gap(V1, 4 * S), &p, &mut ui()),
        edit(vec![Command::CloseGap {
            track_id: V1,
            time: 4 * S,
        }]),
        "U-ED-5: `start <= time < end` excludes the clip end"
    );
    assert_eq!(
        act(close_gap(V1, 0), &p, &mut ui()),
        edit(vec![Command::CloseGap {
            track_id: V1,
            time: 0,
        }]),
        "U-ED-5: time 0"
    );
}

#[test]
fn u_ed_5_close_gap_inside_a_clip_is_no_op() {
    let p = gap_project().build();
    for t in [2 * S, 3 * S, 4 * S - 1, 6 * S, 7 * S] {
        assert_noop(close_gap(V1, t), &p, &ui());
    }
}

#[test]
fn u_ed_5_close_gap_without_clip_after_is_no_op() {
    let p = gap_project().build();
    // 8 s is the end of the last clip; nothing starts after it.
    assert_noop(close_gap(V1, 8 * S), &p, &ui());
    assert_noop(close_gap(V1, 9 * S), &p, &ui());
    // A track without clips.
    assert_noop(close_gap(V2, S), &p, &ui());
}

#[test]
fn u_ed_5_close_gap_locked_or_foreign_track_is_no_op() {
    let p = gap_project().lock(V1).build();
    assert_noop(close_gap(V1, 5 * S), &p, &ui());
    // The other sequence's track has a clip starting after 1 s, but is not in this sequence.
    let p = gap_project().other_seq_clip(900, 3 * S, S).build();
    assert_noop(close_gap(OTHER_TRACK, S), &p, &ui());
    assert_noop(close_gap(STALE, S), &p, &ui());
}

// ------------------------------------------------------------------------------------- U-ED-6

/// V1: P 101 `[0, 4s)`, Q 102 `[4s, 8s)`, gap `[8s, 10s)`, R 103 `[10s, 14s)`.
fn trans_project() -> B {
    B::std()
        .clip(V1, 101, 0, 4 * S)
        .clip(V1, 102, 4 * S, 4 * S)
        .clip(V1, 103, 10 * S, 4 * S)
}

const XD: &str = "cross_dissolve";

#[test]
fn u_ed_6_end_edge_with_next_clip() {
    let p = trans_project().build();
    // 3 s in P: 3 s from the start, 1 s from the end -> end edge; Q starts at P.end.
    let ops = transition_at(&p, SEQ, V1, 3 * S, XD);
    assert_eq!(
        ops,
        // min(1 s, 2 * 4 s, 2 * 4 s) = 1 s
        edit(vec![trans(V1, Some(101), Some(102), XD, S)]),
        "U-ED-6: end edge with a neighbour"
    );
    let after = apply(&p, &ops);
    assert_eq!(after.track(V1).expect("V1").1.transitions.len(), 1);
}

#[test]
fn u_ed_6_end_edge_without_next_clip() {
    let p = trans_project().build();
    // 7 s in Q: end edge; nothing starts at 8 s.
    assert_eq!(
        transition_at(&p, SEQ, V1, 7 * S, XD),
        // min(1 s, 4 s) = 1 s
        edit(vec![trans(V1, Some(102), None, XD, S)]),
        "U-ED-6: end edge without a neighbour -> left only"
    );
}

#[test]
fn u_ed_6_start_edge_with_previous_clip() {
    let p = trans_project().build();
    // 5 s in Q: 1 s from the start, 3 s from the end -> start edge; P ends at Q.start.
    assert_eq!(
        transition_at(&p, SEQ, V1, 5 * S, XD),
        edit(vec![trans(V1, Some(101), Some(102), XD, S)]),
        "U-ED-6: start edge with a neighbour"
    );
    // Exactly on the cut: 4 s belongs to Q (start <= time), distance 0 -> start edge.
    assert_eq!(
        transition_at(&p, SEQ, V1, 4 * S, XD),
        edit(vec![trans(V1, Some(101), Some(102), XD, S)]),
        "U-ED-6: time on the cut"
    );
}

#[test]
fn u_ed_6_start_edge_without_previous_clip() {
    let p = trans_project().build();
    // 1 s in P: start edge; nothing ends at 0.
    assert_eq!(
        transition_at(&p, SEQ, V1, S, XD),
        edit(vec![trans(V1, None, Some(101), XD, S)]),
        "U-ED-6: start edge without a neighbour -> right only"
    );
    // 11 s in R: start edge; Q ends at 8 s, not at 10 s.
    assert_eq!(
        transition_at(&p, SEQ, V1, 11 * S, XD),
        edit(vec![trans(V1, None, Some(103), XD, S)]),
        "U-ED-6: a clip ending before C.start is not a neighbour"
    );
}

#[test]
fn u_ed_6_tie_goes_to_end_edge() {
    let p = trans_project().build();
    // 2 s in P: 2 s from both edges; `<` is false -> end edge.
    assert_eq!(
        transition_at(&p, SEQ, V1, 2 * S, XD),
        edit(vec![trans(V1, Some(101), Some(102), XD, S)]),
        "U-ED-6: tie -> end edge (with Q)"
    );
    // 12 s in R: tie -> end edge, nothing after R -> left only (start edge would be right only).
    assert_eq!(
        transition_at(&p, SEQ, V1, 12 * S, XD),
        edit(vec![trans(V1, Some(103), None, XD, S)]),
        "U-ED-6: tie -> end edge"
    );
    // One flick earlier it is the start edge.
    assert_eq!(
        transition_at(&p, SEQ, V1, 12 * S - 1, XD),
        edit(vec![trans(V1, None, Some(103), XD, S)]),
        "U-ED-6: nearer to the start"
    );
}

#[test]
fn u_ed_6_duration_is_limited_by_short_clips() {
    // V1: 101 [0, 10F), 102 [10F, 10F + 4s). 10F = 235_200_000.
    let p = B::std()
        .clip(V1, 101, 0, 235_200_000)
        .clip(V1, 102, 235_200_000, 4 * S)
        .build();
    // 8F in 101: end edge; min(1 s, 2 * 10F, 2 * 4 s) = 20F = 470_400_000.
    let ops = transition_at(&p, SEQ, V1, 188_160_000, XD);
    assert_eq!(
        ops,
        edit(vec![trans(V1, Some(101), Some(102), XD, 470_400_000)]),
        "U-ED-6: two-sided duration limited by the left clip"
    );
    apply(&p, &ops);
    // 2F in 101: start edge, no previous clip; min(1 s, 10F) = 10F.
    assert_eq!(
        transition_at(&p, SEQ, V1, 47_040_000, XD),
        edit(vec![trans(V1, None, Some(101), XD, 235_200_000)]),
        "U-ED-6: one-sided start duration = min(1 s, C.duration)"
    );

    // V1: 101 [0, 4s), 102 [4s, 4s + 12F). 12F = 282_240_000.
    let p = B::std()
        .clip(V1, 101, 0, 4 * S)
        .clip(V1, 102, 4 * S, 282_240_000)
        .build();
    // 3.5 s in 101: end edge; min(1 s, 8 s, 2 * 12F) = 24F = 564_480_000.
    assert_eq!(
        transition_at(&p, SEQ, V1, 2_469_600_000, XD),
        edit(vec![trans(V1, Some(101), Some(102), XD, 564_480_000)]),
        "U-ED-6: two-sided duration limited by the right clip"
    );
    // 4 s + 1F in 102: start edge with P = 101; same 24F.
    assert_eq!(
        transition_at(&p, SEQ, V1, 4 * S + F, XD),
        edit(vec![trans(V1, Some(101), Some(102), XD, 564_480_000)]),
        "U-ED-6: mirrored start edge, duration limited by C"
    );
    // 4 s + 11F in 102: end edge, nothing after; min(1 s, 12F) = 12F.
    assert_eq!(
        transition_at(&p, SEQ, V1, 4 * S + 258_720_000, XD),
        edit(vec![trans(V1, Some(102), None, XD, 282_240_000)]),
        "U-ED-6: one-sided end duration = min(1 s, C.duration)"
    );
}

#[test]
fn u_ed_6_gap_with_both_sides_closes_gap_then_adds_transition() {
    let p = trans_project().build();
    let ops = transition_at(&p, SEQ, V1, 9 * S, XD);
    assert_eq!(
        ops,
        edit(vec![
            Command::CloseGap {
                track_id: V1,
                time: 9 * S,
            },
            // L = Q, R = R; min(1 s, 8 s, 8 s) = 1 s
            trans(V1, Some(102), Some(103), XD, S),
        ]),
        "U-ED-6: gap -> one batch [CloseGap, AddTransition]"
    );
    let after = apply(&p, &ops);
    assert_eq!(
        clips_on(&after, V1),
        vec![(101, 0, 4 * S), (102, 4 * S, 4 * S), (103, 8 * S, 4 * S)]
    );
    assert_eq!(after.track(V1).expect("V1").1.transitions.len(), 1);
    // 8 s: Q ends exactly at `time` ("ending at or before").
    assert_eq!(
        transition_at(&p, SEQ, V1, 8 * S, XD),
        edit(vec![
            Command::CloseGap {
                track_id: V1,
                time: 8 * S,
            },
            trans(V1, Some(102), Some(103), XD, S),
        ]),
        "U-ED-6: gap start"
    );
}

#[test]
fn u_ed_6_gap_duration_is_limited_by_short_clips() {
    // V1: L 101 [0, 10F), R 102 [2s, 6s).
    let p = B::std()
        .clip(V1, 101, 0, 235_200_000)
        .clip(V1, 102, 2 * S, 4 * S)
        .build();
    assert_eq!(
        transition_at(&p, SEQ, V1, S, XD),
        edit(vec![
            Command::CloseGap {
                track_id: V1,
                time: S,
            },
            // min(1 s, 2 * 10F, 8 s) = 20F
            trans(V1, Some(101), Some(102), XD, 470_400_000),
        ]),
        "U-ED-6: gap duration = min(1 s, 2 * L.duration, 2 * R.duration)"
    );
}

#[test]
fn u_ed_6_gap_without_both_sides_is_no_op() {
    // V1: 101 [2s, 4s), 102 [6s, 8s).
    let p = gap_project().build();
    assert_eq!(
        transition_at(&p, SEQ, V1, S, XD),
        vec![],
        "U-ED-6: gap before the first clip has no L"
    );
    assert_eq!(transition_at(&p, SEQ, V1, 0, XD), vec![], "U-ED-6: time 0");
    assert_eq!(
        transition_at(&p, SEQ, V1, 9 * S, XD),
        vec![],
        "U-ED-6: after the last clip there is no R"
    );
    assert_eq!(
        transition_at(&p, SEQ, V1, 8 * S, XD),
        vec![],
        "U-ED-6: at the end of the last clip there is no R"
    );
    assert_eq!(
        transition_at(&p, SEQ, V2, S, XD),
        vec![],
        "U-ED-6: empty track"
    );
}

#[test]
fn u_ed_6_locked_or_foreign_track_is_no_op() {
    let p = trans_project().lock(V1).build();
    for t in [S, 3 * S, 9 * S] {
        assert_eq!(
            transition_at(&p, SEQ, V1, t, XD),
            vec![],
            "U-ED-6: locked track, time {t}"
        );
    }
    let p = trans_project().other_seq_clip(900, 0, 4 * S).build();
    assert_eq!(
        transition_at(&p, SEQ, OTHER_TRACK, S, XD),
        vec![],
        "U-ED-6: track of another sequence"
    );
    assert_eq!(
        transition_at(&p, OTHER_SEQ, V1, S, XD),
        vec![],
        "U-ED-6: V1 is not in the other sequence"
    );
    assert_eq!(
        transition_at(&p, SEQ, STALE, S, XD),
        vec![],
        "U-ED-6: unknown track"
    );
    assert_eq!(
        transition_at(&p, STALE, V1, S, XD),
        vec![],
        "U-ED-6: missing sequence"
    );
}

#[test]
fn u_ed_6_effect_name_is_passed_through() {
    // A1: 201 [0, 4s), 202 [4s, 8s).
    let p = B::std()
        .clip(A1, 201, 0, 4 * S)
        .clip(A1, 202, 4 * S, 4 * S)
        .build();
    assert_eq!(
        transition_at(&p, SEQ, A1, 3 * S, "crossfade"),
        edit(vec![trans(A1, Some(201), Some(202), "crossfade", S)]),
        "U-ED-6: `effect` as given, on an audio track"
    );
}

// ------------------------------------------------------------------------------------- U-ED-7

/// V1: 101 `[2s, 4s)`, 102 `[5s, 8s)` (the example of U-ED-7). Playhead 0, no marks.
fn snap_project() -> B {
    B::std()
        .clip(V1, 101, 2 * S, 2 * S)
        .clip(V1, 102, 5 * S, 3 * S)
}

#[test]
fn u_ed_7_end_edge_snaps_to_next_clip_start() {
    let p = snap_project().build();
    // delta 0.95 s = 670_320_000: the end lands on 4.95 s, 0.05 s from 5 s -> 1 s.
    assert_eq!(
        snap_delta(670_320_000, &[101], &p, &ui(), TOL),
        S,
        "U-ED-7: worked example of the rule"
    );
    // From the other side: delta 1.05 s = 740_880_000 -> end at 5.05 s -> 1 s.
    assert_eq!(
        snap_delta(740_880_000, &[101], &p, &ui(), TOL),
        S,
        "U-ED-7: candidate below the edge"
    );
}

#[test]
fn u_ed_7_start_edge_snaps_to_previous_clip_end() {
    let p = snap_project().build();
    // 102 [5s, 8s) by -0.95 s: start at 4.05 s, 0.05 s from 101's end (4 s) -> -1 s.
    assert_eq!(
        snap_delta(-670_320_000, &[102], &p, &ui(), TOL),
        -S,
        "U-ED-7: start edge snaps, delta + (candidate - edge)"
    );
    // 101 [2s, 4s) by -1.95 s = -1_375_920_000: start at 0.05 s -> candidate 0 -> -2 s.
    assert_eq!(
        snap_delta(-1_375_920_000, &[101], &p, &ui(), TOL),
        -2 * S,
        "U-ED-7: 0 is a candidate (U-SNAP)"
    );
}

#[test]
fn u_ed_7_snap_off_rounds_to_frames_with_sign_kept() {
    let p = snap_project().build();
    let mut u = ui();
    u.snap = false;
    // The example delta would snap to 1 s; with snap off: 0.95 s = 28.5F is avoided here,
    // 670_000_000 / F = 28.49 -> 28F = 658_560_000.
    assert_eq!(
        snap_delta(670_000_000, &[101], &p, &u, TOL),
        658_560_000,
        "U-ED-7: snap off -> nearest multiple of F"
    );
    // 10F + 5_000_000 = 240_200_000 -> 10F; 10F + 20_000_000 = 255_200_000 -> 11F.
    assert_eq!(snap_delta(240_200_000, &[101], &p, &u, TOL), 235_200_000);
    assert_eq!(snap_delta(255_200_000, &[101], &p, &u, TOL), 258_720_000);
    assert_eq!(
        snap_delta(-240_200_000, &[102], &p, &u, TOL),
        -235_200_000,
        "U-ED-7: |delta| rounded, sign kept"
    );
    assert_eq!(
        snap_delta(-255_200_000, &[102], &p, &u, TOL),
        -258_720_000,
        "U-ED-7: |delta| rounded, sign kept"
    );
    assert_eq!(snap_delta(0, &[101], &p, &u, TOL), 0);
    assert_eq!(snap_delta(5_000_000, &[101], &p, &u, TOL), 0);
    assert_eq!(snap_delta(-5_000_000, &[102], &p, &u, TOL), 0);
}

#[test]
fn u_ed_7_no_candidate_in_range_rounds_to_frames() {
    let p = snap_project().build();
    // 101 by 10F + 5_000_000: edges at about 2.34 s and 4.34 s; candidates 0, 5 s, 8 s.
    assert_eq!(
        snap_delta(240_200_000, &[101], &p, &ui(), TOL),
        235_200_000,
        "U-ED-7: nothing within tolerance -> rounded to F"
    );
    assert_eq!(
        snap_delta(-255_200_000, &[102], &p, &ui(), TOL),
        -258_720_000,
        "U-ED-7: nothing within tolerance -> rounded, sign kept"
    );
}

#[test]
fn u_ed_7_tolerance_is_inclusive() {
    let p = snap_project().build();
    // delta 0.9 s = 635_040_000 (= 27F): the end is exactly 0.1 s from 5 s.
    assert_eq!(
        snap_delta(635_040_000, &[101], &p, &ui(), TOL),
        S,
        "U-ED-7: distance == tolerance snaps"
    );
    // One flick further away: out of range -> nearest frame = 27F.
    assert_eq!(
        snap_delta(635_039_999, &[101], &p, &ui(), TOL),
        635_040_000,
        "U-ED-7: distance == tolerance + 1 does not snap"
    );
    assert_eq!(
        snap_delta(670_320_000, &[101], &p, &ui(), 35_279_999),
        // 0.05 s away with tolerance 0.05 s - 1: no snap; 670_320_000 = 28.5F is a rounding tie,
        // so only "not snapped to 1 s" is checked through the two possible frames.
        snap_delta(670_320_000, &[101], &p, &ui(), 0),
        "U-ED-7: same result as with tolerance 0"
    );
    assert_ne!(
        snap_delta(670_320_000, &[101], &p, &ui(), 35_279_999),
        S,
        "U-ED-7: 0.05 s away is outside a tolerance of 0.05 s - 1"
    );
}

#[test]
fn u_ed_7_smallest_distance_wins() {
    // Marker at 2.97 s = 2_095_632_000; V2 holds 103 [5s, 6s).
    let p = B::std()
        .clip(V1, 101, 2 * S, 2 * S)
        .clip(V2, 103, 5 * S, S)
        .marker(500, 2_095_632_000)
        .build();
    // delta 0.95 s: start at 2.95 s is 0.02 s from the marker, end at 4.95 s is 0.05 s from 5 s.
    // Result 0.95 s + 0.02 s = 0.97 s = 684_432_000.
    assert_eq!(
        snap_delta(670_320_000, &[101], &p, &ui(), TOL),
        684_432_000,
        "U-ED-7: the pair with the smallest distance wins"
    );
}

#[test]
fn u_ed_7_tie_goes_to_smaller_delta() {
    // Markers at 2.95 s = 2_081_520_000 and 5.05 s = 3_563_280_000.
    let p = B::std()
        .clip(V1, 101, 2 * S, 2 * S)
        .marker(500, 2_081_520_000)
        .marker(501, 3_563_280_000)
        .build();
    // delta 1 s: start at 3 s and end at 5 s are both 0.05 s from a marker.
    // Resulting deltas 0.95 s (start) and 1.05 s (end) -> the smaller, 670_320_000.
    assert_eq!(
        snap_delta(S, &[101], &p, &ui(), TOL),
        670_320_000,
        "U-ED-7: tie -> the smaller resulting delta"
    );
}

#[test]
fn u_ed_7_tie_with_negative_delta_goes_to_smaller_signed_delta() {
    // Markers at 0.95 s = 670_320_000 and 3.05 s = 2_152_080_000.
    let p = B::std()
        .clip(V1, 101, 2 * S, 2 * S)
        .marker(500, 670_320_000)
        .marker(501, 2_152_080_000)
        .build();
    // delta -1 s: start at 1 s, end at 3 s, both 0.05 s from a marker.
    // Resulting deltas -1.05 s and -0.95 s -> the smaller number, -740_880_000.
    assert_eq!(
        snap_delta(-S, &[101], &p, &ui(), TOL),
        -740_880_000,
        "U-ED-7: tie -> the smaller resulting delta (literal reading: signed)"
    );
}

#[test]
fn u_ed_7_unknown_ids_are_ignored() {
    let p = snap_project().build();
    assert_eq!(
        snap_delta(670_320_000, &[STALE, 101], &p, &ui(), TOL),
        S,
        "U-ED-7: unknown id next to a real one"
    );
    // No moving clip at all: nothing to compare -> rounded (10F + 5_000_000 -> 10F).
    assert_eq!(
        snap_delta(240_200_000, &[STALE], &p, &ui(), TOL),
        235_200_000,
        "U-ED-7: only unknown ids"
    );
    assert_eq!(
        snap_delta(240_200_000, &[], &p, &ui(), TOL),
        235_200_000,
        "U-ED-7: no clips"
    );
}

#[test]
fn u_ed_7_candidates_include_playhead_marks_and_markers() {
    // Only clip: 101 [2s, 4s). delta 5.95 s = 4_198_320_000 puts its end at 9.95 s.
    let p = B::std().clip(V1, 101, 2 * S, 2 * S).build();
    let d = 4_198_320_000;
    // Nothing at 10 s: 5.95 s = 178.5F is a rounding tie, so only "not 6 s" is checked.
    assert_ne!(snap_delta(d, &[101], &p, &ui(), TOL), 6 * S);
    let mut u = ui();
    u.playhead = 10 * S;
    assert_eq!(
        snap_delta(d, &[101], &p, &u, TOL),
        6 * S,
        "U-ED-7: playhead"
    );
    let mut u = ui();
    u.mark_in = Some(10 * S);
    assert_eq!(snap_delta(d, &[101], &p, &u, TOL), 6 * S, "U-ED-7: mark_in");
    let mut u = ui();
    u.mark_out = Some(10 * S);
    assert_eq!(
        snap_delta(d, &[101], &p, &u, TOL),
        6 * S,
        "U-ED-7: mark_out"
    );
    let p = B::std()
        .clip(V1, 101, 2 * S, 2 * S)
        .marker(500, 10 * S)
        .build();
    assert_eq!(
        snap_delta(d, &[101], &p, &ui(), TOL),
        6 * S,
        "U-ED-7: marker"
    );
    // Candidates are returned as is: a marker off the frame grid at 10 s + 1000.
    let p = B::std()
        .clip(V1, 101, 2 * S, 2 * S)
        .marker(500, 10 * S + 1000)
        .build();
    assert_eq!(
        snap_delta(d, &[101], &p, &ui(), TOL),
        6 * S + 1000,
        "U-ED-7: result = delta + (candidate - edge), not rounded"
    );
}

#[test]
fn u_ed_7_candidates_include_clips_on_other_tracks_only_of_this_sequence() {
    // A1: 201 [5s, 8s); other sequence: 900 [10s, 11s).
    let p = B::std()
        .clip(V1, 101, 2 * S, 2 * S)
        .clip(A1, 201, 5 * S, 3 * S)
        .other_seq_clip(900, 10 * S, S)
        .build();
    assert_eq!(
        snap_delta(670_320_000, &[101], &p, &ui(), TOL),
        S,
        "U-ED-7: a clip start on another track is a candidate"
    );
    // End at 8 s - 0.05 s when moved by 3.95 s = 2_787_120_000 -> 201's end (8 s) -> 4 s.
    assert_eq!(
        snap_delta(2_787_120_000, &[101], &p, &ui(), TOL),
        4 * S,
        "U-ED-7: a clip end is a candidate"
    );
    // End at 9.95 s: 10 s is only a clip start in the other sequence -> no snap to 6 s.
    assert_ne!(
        snap_delta(4_198_320_000, &[101], &p, &ui(), TOL),
        6 * S,
        "U-ED-7: other sequences give no candidates"
    );
}

#[test]
fn u_ed_7_moving_clips_are_not_candidates() {
    let p = snap_project().build();
    // 101 by 30_000_000 (< tolerance): its own old start and end are in range but excluded.
    // 30_000_000 / F = 1.28 -> F.
    assert_eq!(
        snap_delta(30_000_000, &[101], &p, &ui(), TOL),
        F,
        "U-ED-7: a moving clip does not snap to itself"
    );
    // 101 and 102 both move by 1 s + 30_000_000 = 735_600_000: 101's end would be 30_000_000
    // past 102's old start, but 102 is moving too. 735_600_000 / F = 31.28 -> 31F.
    assert_eq!(
        snap_delta(735_600_000, &[101, 102], &p, &ui(), TOL),
        729_120_000,
        "U-ED-7: moving clips do not snap to each other"
    );
    // With only 101 moving the same delta snaps to 1 s.
    assert_eq!(snap_delta(735_600_000, &[101], &p, &ui(), TOL), S);
}

#[test]
fn u_ed_7_every_moving_clip_is_compared() {
    // V1: 101 [2s, 4s), 102 [5s, 8s) move; V2: 103 [10s, 12s) stays.
    let p = snap_project().clip(V2, 103, 10 * S, 2 * S).build();
    // delta 1.95 s = 1_375_920_000: only 102's end (9.95 s) is near a candidate (10 s) -> 2 s.
    assert_eq!(
        snap_delta(1_375_920_000, &[101, 102], &p, &ui(), TOL),
        2 * S,
        "U-ED-7: edges of every moving clip are compared"
    );
    assert_eq!(
        snap_delta(1_375_920_000, &[102, 101], &p, &ui(), TOL),
        2 * S,
        "U-ED-7: whatever the order of `clips`"
    );
}

#[test]
fn u_ed_7_missing_sequence_does_not_panic() {
    let p = snap_project().build();
    let mut u = ui();
    u.sequence_id = STALE;
    // The contract fixes no value here (there is no fps to round with); it must not panic.
    let _ = snap_delta(670_320_000, &[101], &p, &u, TOL);
    u.snap = false;
    let _ = snap_delta(-670_320_000, &[101], &p, &u, TOL);
}

// ------------------------------------------------------------------------------------- U-ED-8

#[test]
fn u_ed_8_preview_of_a_move_is_the_edited_project() {
    let p = B::std().clip(V1, 101, 2 * S, 2 * S).build();
    let before = p.clone();
    let u = ui();
    // U-DRAG-MOVE: body drag from 2.5 s to 3.5 s -> 101 moves by 1 s.
    let d = drag_of(clip_hit(101, V1, 1_764_000_000), 2_469_600_000, V1);
    let got = preview(d, &p, &u).expect("U-ED-8: a move gives Some");
    assert_eq!(clips_on(&got, V1), vec![(101, 3 * S, 2 * S)], "U-ED-8");
    assert_eq!(p, before, "U-ED-8: project unchanged");
    assert_eq!(u, ui(), "U-ED-8: ui unchanged");
    // The same as applying what `drag` returns.
    let ops = gesture(d, &p, &mut u.clone());
    assert_eq!(
        got,
        apply(&p, &ops),
        "U-ED-8: result = project after the edit"
    );
}

#[test]
fn u_ed_8_preview_of_a_trim() {
    let p = snap_project().build();
    // U-DRAG-TRIM: end edge of 101 from 4 s to 4.5 s = 3_175_200_000.
    let d = drag_of(
        Hit::Clip {
            clip_id: 101,
            track_id: V1,
            part: ClipPart::EndEdge,
            time: 4 * S,
        },
        3_175_200_000,
        V1,
    );
    let got = preview(d, &p, &ui()).expect("U-ED-8: a trim gives Some");
    assert_eq!(
        clips_on(&got, V1),
        // 2.5 s = 1_764_000_000
        vec![(101, 2 * S, 1_764_000_000), (102, 5 * S, 3 * S)],
        "U-ED-8"
    );
}

#[test]
fn u_ed_8_preview_without_an_edit_is_none() {
    let p = snap_project().build();
    let u = ui();
    // A click only selects (U-DRAG-SEL).
    assert_eq!(
        preview(drag_of(clip_hit(101, V1, 3 * S), 3 * S, V1), &p, &u),
        None,
        "U-ED-8: click -> no edit"
    );
    // A ruler drag only moves the playhead.
    assert_eq!(
        preview(drag_of(Hit::Ruler { time: S }, 2 * S, V1), &p, &u),
        None,
        "U-ED-8: ruler drag -> no edit"
    );
    // Ripple tool on a body is a no-op (U-DRAG-TRIM).
    let mut r = ui();
    r.tool = Tool::Ripple;
    assert_eq!(
        preview(drag_of(clip_hit(101, V1, 3 * S), 3 * S + S / 2, V1), &p, &r),
        None,
        "U-ED-8: gesture without ops"
    );
    // A move that would start before 0 is a no-op (U-DRAG-MOVE).
    assert_eq!(
        preview(drag_of(clip_hit(101, V1, 3 * S), 0, V1), &p, &u),
        None,
        "U-ED-8: move before 0"
    );
    // Wand: Op::WandPrompt only (U-DRAG-WAND).
    let mut w = ui();
    w.tool = Tool::Wand;
    assert_eq!(
        preview(drag_of(clip_hit(101, V1, 3 * S), 6 * S, V1), &p, &w),
        None,
        "U-ED-8: an op that is not an Edit"
    );
    assert_eq!(u, ui(), "U-ED-8: ui unchanged");
}

#[test]
fn u_ed_8_preview_of_an_edit_the_core_rejects_is_none() {
    let p = snap_project().build();
    let before = p.clone();
    // End edge of 101 dragged to 6 s, into 102 [5s, 8s): C-TRIM-2 -> Overlap.
    let d = drag_of(
        Hit::Clip {
            clip_id: 101,
            track_id: V1,
            part: ClipPart::EndEdge,
            time: 4 * S,
        },
        6 * S,
        V1,
    );
    assert_eq!(preview(d, &p, &ui()), None, "U-ED-8: rejected edit -> None");
    assert_eq!(p, before, "U-ED-8: project unchanged");
}

#[test]
fn u_ed_8_preview_missing_sequence_is_none() {
    let p = snap_project().build();
    let mut u = ui();
    u.sequence_id = STALE;
    let before = u.clone();
    let d = drag_of(clip_hit(101, V1, 3 * S), 3 * S + S / 2, V1);
    assert_eq!(preview(d, &p, &u), None, "U-ED-8: missing sequence");
    assert_eq!(u, before);
}

// ------------------------------------------------------------------------------------- U-ED-9

/// V1: 101 `[0, 2s)` (link 7), 102 `[4s, 6s)`. V2: 103 `[1s, 3s)` and 104 `[5s, 7s)` (group 5).
/// A1: 201 `[0, 2s)` (link 7).
fn menu_project() -> B {
    B::std()
        .clip(V1, 101, 0, 2 * S)
        .clip(V1, 102, 4 * S, 2 * S)
        .clip(V2, 103, S, 2 * S)
        .clip(V2, 104, 5 * S, 2 * S)
        .clip(A1, 201, 0, 2 * S)
        .link(&[101, 201], 7)
        .with(103, |c| c.group = Some(5))
        .with(104, |c| c.group = Some(5))
}

#[test]
fn u_ed_9_clip_menu_plain_clip_without_clipboard() {
    let p = menu_project().build();
    let items = menu(clip_hit(102, V1, 5 * S), false, &p, &ui(), |w| {
        w.selection = vec![102];
    });
    assert_eq!(
        items,
        vec![
            MenuItem::Do(Action::Cut),
            MenuItem::Do(Action::Copy),
            MenuItem::Do(Action::Delete),
            MenuItem::Do(Action::RippleDelete),
            MenuItem::Do(Action::ToggleEnable),
            MenuItem::Do(Action::SpeedDialog),
            MenuItem::Do(Action::GainDialog),
            MenuItem::Labels,
            MenuItem::Transitions,
            MenuItem::Do(Action::WandPrompt),
        ],
        "U-ED-9: no clipboard, no link, one clip, no group"
    );
}

#[test]
fn u_ed_9_clip_menu_paste_only_with_clipboard() {
    let p = menu_project().build();
    let items = menu(clip_hit(102, V1, 5 * S), true, &p, &ui(), |w| {
        w.selection = vec![102];
    });
    assert_eq!(
        items,
        vec![
            MenuItem::Do(Action::Cut),
            MenuItem::Do(Action::Copy),
            MenuItem::Do(Action::Paste),
            MenuItem::Do(Action::Delete),
            MenuItem::Do(Action::RippleDelete),
            MenuItem::Do(Action::ToggleEnable),
            MenuItem::Do(Action::SpeedDialog),
            MenuItem::Do(Action::GainDialog),
            MenuItem::Labels,
            MenuItem::Transitions,
            MenuItem::Do(Action::WandPrompt),
        ],
        "U-ED-9: Paste after Copy with has_clipboard"
    );
}

#[test]
fn u_ed_9_clip_menu_unlink_only_with_a_linked_clip() {
    let p = menu_project().build();
    // 101 is linked to 201.
    let items = menu(clip_hit(101, V1, S), false, &p, &ui(), |w| {
        w.selection = vec![101];
    });
    assert_eq!(
        items,
        vec![
            MenuItem::Do(Action::Cut),
            MenuItem::Do(Action::Copy),
            MenuItem::Do(Action::Delete),
            MenuItem::Do(Action::RippleDelete),
            MenuItem::Do(Action::Unlink),
            MenuItem::Do(Action::ToggleEnable),
            MenuItem::Do(Action::SpeedDialog),
            MenuItem::Do(Action::GainDialog),
            MenuItem::Labels,
            MenuItem::Transitions,
            MenuItem::Do(Action::WandPrompt),
        ],
        "U-ED-9: Unlink after RippleDelete when an effective-selection clip has a link"
    );
    // With linked selection off the clicked clip itself still has a link.
    let mut u = ui();
    u.linked_selection = false;
    let items = menu(clip_hit(201, A1, S), false, &p, &u, |w| {
        w.selection = vec![201];
    });
    assert!(
        items.contains(&MenuItem::Do(Action::Unlink)),
        "U-ED-9: 201 has a link: {items:?}"
    );
}

#[test]
fn u_ed_9_clip_menu_group_needs_two_selected_clips() {
    let p = menu_project().build();
    let mut u = ui();
    u.selection = vec![102, 101];
    // 102 is already selected: selection stays. 2 clips -> Group; 101 linked -> Unlink.
    let items = menu(clip_hit(102, V1, 5 * S), false, &p, &u, |_| {});
    assert_eq!(
        items,
        vec![
            MenuItem::Do(Action::Cut),
            MenuItem::Do(Action::Copy),
            MenuItem::Do(Action::Delete),
            MenuItem::Do(Action::RippleDelete),
            MenuItem::Do(Action::Unlink),
            MenuItem::Do(Action::Group),
            MenuItem::Do(Action::ToggleEnable),
            MenuItem::Do(Action::SpeedDialog),
            MenuItem::Do(Action::GainDialog),
            MenuItem::Labels,
            MenuItem::Transitions,
            MenuItem::Do(Action::WandPrompt),
        ],
        "U-ED-9: Group with at least 2 clips in ui.selection"
    );
    // One selected clip whose linked partner makes the effective selection 2 clips: the count
    // is over `ui.selection`, so no Group.
    let mut u = ui();
    u.selection = vec![101];
    let items = menu(clip_hit(101, V1, S), false, &p, &u, |_| {});
    assert!(
        !items.contains(&MenuItem::Do(Action::Group)),
        "U-ED-9: Group counts ui.selection, not the effective selection: {items:?}"
    );
}

#[test]
fn u_ed_9_clip_menu_ungroup_only_with_a_grouped_clip() {
    let p = menu_project().build();
    let items = menu(clip_hit(103, V2, 2 * S), false, &p, &ui(), |w| {
        w.selection = vec![103];
    });
    assert_eq!(
        items,
        vec![
            MenuItem::Do(Action::Cut),
            MenuItem::Do(Action::Copy),
            MenuItem::Do(Action::Delete),
            MenuItem::Do(Action::RippleDelete),
            MenuItem::Do(Action::Ungroup),
            MenuItem::Do(Action::ToggleEnable),
            MenuItem::Do(Action::SpeedDialog),
            MenuItem::Do(Action::GainDialog),
            MenuItem::Labels,
            MenuItem::Transitions,
            MenuItem::Do(Action::WandPrompt),
        ],
        "U-ED-9: Ungroup when an effective-selection clip has a group"
    );
}

#[test]
fn u_ed_9_clip_menu_full_list_in_order() {
    let p = menu_project().build();
    let mut u = ui();
    u.selection = vec![103, 101];
    let items = menu(clip_hit(101, V1, S), true, &p, &u, |_| {});
    assert_eq!(
        items,
        vec![
            MenuItem::Do(Action::Cut),
            MenuItem::Do(Action::Copy),
            MenuItem::Do(Action::Paste),
            MenuItem::Do(Action::Delete),
            MenuItem::Do(Action::RippleDelete),
            MenuItem::Do(Action::Unlink),
            MenuItem::Do(Action::Group),
            MenuItem::Do(Action::Ungroup),
            MenuItem::Do(Action::ToggleEnable),
            MenuItem::Do(Action::SpeedDialog),
            MenuItem::Do(Action::GainDialog),
            MenuItem::Labels,
            MenuItem::Transitions,
            MenuItem::Do(Action::WandPrompt),
        ],
        "U-ED-9: every item, in the order of the rule"
    );
}

#[test]
fn u_ed_9_right_click_on_unselected_clip_replaces_selection() {
    let p = menu_project().build();
    let mut u = ui();
    u.selection = vec![103, 101];
    // 102 is not in the effective selection {103, 101, 201}: selection = [102] first, so the
    // items are those of a plain single clip (no Unlink, Group, Ungroup).
    let items = menu(clip_hit(102, V1, 5 * S), false, &p, &u, |w| {
        w.selection = vec![102];
    });
    assert_eq!(
        items,
        vec![
            MenuItem::Do(Action::Cut),
            MenuItem::Do(Action::Copy),
            MenuItem::Do(Action::Delete),
            MenuItem::Do(Action::RippleDelete),
            MenuItem::Do(Action::ToggleEnable),
            MenuItem::Do(Action::SpeedDialog),
            MenuItem::Do(Action::GainDialog),
            MenuItem::Labels,
            MenuItem::Transitions,
            MenuItem::Do(Action::WandPrompt),
        ],
        "U-ED-9: the conditions are judged after the selection change"
    );
}

#[test]
fn u_ed_9_right_click_inside_effective_selection_keeps_selection() {
    let p = menu_project().build();
    let mut u = ui();
    u.selection = vec![101];
    // 201 is in the effective selection through its link with 101.
    let items = menu(clip_hit(201, A1, S), false, &p, &u, |_| {});
    assert!(
        items.contains(&MenuItem::Do(Action::Unlink)),
        "U-ED-9: {items:?}"
    );
    // Without linked selection 201 is not in it: selection = [201].
    u.linked_selection = false;
    menu(clip_hit(201, A1, S), false, &p, &u, |w| {
        w.selection = vec![201];
    });
    // Any clip part counts as a Clip hit.
    let edge = Hit::Clip {
        clip_id: 102,
        track_id: V1,
        part: ClipPart::EndEdge,
        time: 6 * S - 1,
    };
    menu(edge, false, &p, &u, |w| w.selection = vec![102]);
}

#[test]
fn u_ed_9_empty_hit_in_a_gap() {
    // V1: 101 [2s, 4s), 102 [6s, 8s).
    let p = gap_project().build();
    let mut u = ui();
    u.selection = vec![101];
    let hit = Hit::Empty {
        track_id: V1,
        time: 5 * S,
    };
    assert_eq!(
        menu(hit, true, &p, &u, |_| {}),
        vec![
            MenuItem::Do(Action::Paste),
            MenuItem::Do(close_gap(V1, 5 * S)),
            MenuItem::Transitions,
            MenuItem::Do(Action::WandPrompt),
        ],
        "U-ED-9: Empty hit in a gap with clips on both sides, with clipboard"
    );
    assert_eq!(
        menu(hit, false, &p, &u, |_| {}),
        vec![
            MenuItem::Do(close_gap(V1, 5 * S)),
            MenuItem::Transitions,
            MenuItem::Do(Action::WandPrompt),
        ],
        "U-ED-9: Empty hit in a gap, no clipboard"
    );
}

#[test]
fn u_ed_9_empty_hit_before_first_clip_has_close_gap_but_no_transitions() {
    let p = gap_project().build();
    let hit = Hit::Empty {
        track_id: V1,
        time: S,
    };
    assert_eq!(
        menu(hit, false, &p, &ui(), |_| {}),
        acts(&[close_gap(V1, S), Action::WandPrompt]),
        "U-ED-9: U-ED-5 gives an op, U-ED-6 does not (no clip before)"
    );
}

#[test]
fn u_ed_9_empty_hit_after_last_clip() {
    let p = gap_project().build();
    let hit = Hit::Empty {
        track_id: V1,
        time: 9 * S,
    };
    assert_eq!(
        menu(hit, false, &p, &ui(), |_| {}),
        acts(&[Action::WandPrompt]),
        "U-ED-9: no clip after -> neither CloseGap nor Transitions"
    );
    assert_eq!(
        menu(hit, true, &p, &ui(), |_| {}),
        acts(&[Action::Paste, Action::WandPrompt]),
        "U-ED-9: Paste only depends on has_clipboard"
    );
    // A track without clips.
    let hit = Hit::Empty {
        track_id: A2,
        time: S,
    };
    assert_eq!(
        menu(hit, false, &p, &ui(), |_| {}),
        acts(&[Action::WandPrompt]),
        "U-ED-9: empty track"
    );
}

#[test]
fn u_ed_9_empty_hit_on_locked_track() {
    let p = gap_project().lock(V1).build();
    let hit = Hit::Empty {
        track_id: V1,
        time: 5 * S,
    };
    assert_eq!(
        menu(hit, false, &p, &ui(), |_| {}),
        acts(&[Action::WandPrompt]),
        "U-ED-9: locked track -> U-ED-5 and U-ED-6 give no op"
    );
}

#[test]
fn u_ed_9_other_hits_give_an_empty_menu() {
    let p = menu_project().build();
    let mut u = ui();
    u.selection = vec![101];
    for hit in [
        Hit::Ruler { time: S },
        Hit::Header { track_id: V1 },
        Hit::Nothing,
    ] {
        for has_clipboard in [false, true] {
            assert_eq!(
                menu(hit, has_clipboard, &p, &u, |_| {}),
                vec![],
                "U-ED-9: {hit:?} -> empty"
            );
        }
    }
}

#[test]
fn u_ed_9_missing_sequence_gives_an_empty_menu() {
    let p = menu_project().build();
    let mut u = ui();
    u.sequence_id = STALE;
    u.selection = vec![101];
    for hit in [
        clip_hit(102, V1, 5 * S),
        clip_hit(101, V1, S),
        Hit::Empty {
            track_id: V1,
            time: 3 * S,
        },
        Hit::Ruler { time: S },
        Hit::Header { track_id: V1 },
        Hit::Nothing,
    ] {
        assert_eq!(
            menu(hit, true, &p, &u, |_| {}),
            vec![],
            "U-ED-9: missing sequence, {hit:?}"
        );
    }
}

#[test]
fn u_ed_9_unknown_ids_in_the_hit_do_not_panic() {
    let p = menu_project().build();
    let mut u = ui();
    u.selection = vec![101];
    // The contract fixes no result for a hit naming a clip that is gone; it must not panic.
    let _ = context_menu(clip_hit(STALE, V1, S), true, &p, &mut u.clone());
    // An Empty hit on an unknown track: U-ED-5 and U-ED-6 give no op.
    let hit = Hit::Empty {
        track_id: STALE,
        time: S,
    };
    assert_eq!(
        menu(hit, false, &p, &u, |_| {}),
        acts(&[Action::WandPrompt]),
        "U-ED-9: unknown track"
    );
}

// ------------------------------------------------------------------ missing sequence, U-ED-1..5

/// "Never panics, missing sequence -> nothing" for the new actions and `copy`.
/// `Paste` is left out: U-ED-1 says it returns `[Op::Paste]` always.
#[test]
fn u_ed_1_to_5_missing_sequence_is_no_op() {
    let p = unlink_project().build();
    let mut u = ui();
    u.sequence_id = STALE;
    u.selection = vec![101, 102];
    u.playhead = 3 * S;
    for a in [
        Action::Copy,
        Action::Cut,
        Action::Unlink,
        Action::SetLabel(1),
        Action::SetLabel(9),
        close_gap(V1, 3 * S),
    ] {
        assert_noop(a, &p, &u);
    }
    assert_eq!(copy(&p, &u), vec![], "U-ED-1: copy with a missing sequence");
    // A project without any sequence.
    let empty = Project::new("empty");
    let mut u = ui();
    u.selection = vec![101];
    for a in [
        Action::Copy,
        Action::Cut,
        Action::Unlink,
        Action::SetLabel(1),
        close_gap(V1, 3 * S),
    ] {
        assert_noop(a, &empty, &u);
    }
    assert_eq!(copy(&empty, &u), vec![]);
    assert_eq!(transition_at(&empty, SEQ, V1, S, XD), vec![]);
    assert_eq!(
        preview(drag_of(clip_hit(101, V1, S), 2 * S, V1), &empty, &u),
        None
    );
    assert_paste_noop(&[item(V1, mk_clip(101, 0, S))], &empty, &u);
}
