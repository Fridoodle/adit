//! U-STATE, U-KEYS, U-TOOL, U-TOGGLE, U-ZOOM, U-OPS, U-MARKER and the general `perform` rules
//! (missing sequence, empty project, never an empty Edit).
mod common;
use common::*;
use eframe::egui::Key;
use std::collections::HashSet;

// ------------------------------------------------------------------------------------ U-STATE

#[test]
fn u_state_defaults() {
    let u = UiState::new(42);
    let want = UiState {
        sequence_id: 42,
        playhead: 0,
        selection: vec![],
        mark_in: None,
        mark_out: None,
        tool: Tool::Select,
        snap: true,
        linked_selection: true,
        playing: false,
        shuttle: 0,
        targets: vec![],
        source: None,
        px_per_sec: 100.0,
        scroll: 0,
        view_width: 1000.0,
    };
    assert_eq!(u, want);
    assert_eq!(UiState::new(SEQ), ui(), "U-STATE: the test helper matches");
}

// ------------------------------------------------------------------------------------- U-KEYS

/// `m`: any of 'c' (Ctrl), 's' (Shift), 'a' (Alt).
fn k(m: &str, key: Key) -> KeyChord {
    KeyChord {
        key,
        ctrl: m.contains('c'),
        shift: m.contains('s'),
        alt: m.contains('a'),
    }
}

fn spec_keymap() -> Vec<(KeyChord, Action)> {
    use Action::*;
    use Tool as T;
    vec![
        (k("", Key::V), SetTool(T::Select)),
        (k("", Key::A), SetTool(T::TrackSelect)),
        (k("", Key::B), SetTool(T::Ripple)),
        (k("", Key::N), SetTool(T::Roll)),
        (k("", Key::R), SetTool(T::RateStretch)),
        (k("", Key::C), SetTool(T::Razor)),
        (k("", Key::Y), SetTool(T::Slip)),
        (k("", Key::U), SetTool(T::Slide)),
        (k("", Key::P), SetTool(T::Pen)),
        (k("", Key::H), SetTool(T::Hand)),
        (k("", Key::Z), SetTool(T::Zoom)),
        (k("", Key::T), SetTool(T::Type)),
        (k("s", Key::W), SetTool(T::Wand)),
        (k("cs", Key::W), WandPrompt),
        (k("", Key::Space), PlayPause),
        (k("", Key::K), ShuttleStop),
        (k("", Key::J), ShuttleLeft),
        (k("", Key::L), ShuttleRight),
        (k("", Key::ArrowLeft), Step(-1)),
        (k("", Key::ArrowRight), Step(1)),
        (k("s", Key::ArrowLeft), Step(-5)),
        (k("s", Key::ArrowRight), Step(5)),
        (k("", Key::Home), GoToStart),
        (k("", Key::End), GoToEnd),
        (k("", Key::ArrowUp), PrevEdit),
        (k("", Key::ArrowDown), NextEdit),
        (k("", Key::I), MarkIn),
        (k("", Key::O), MarkOut),
        (k("", Key::X), MarkClip),
        (k("cs", Key::X), ClearInOut),
        (k("cs", Key::I), ClearIn),
        (k("cs", Key::O), ClearOut),
        (k("s", Key::I), GoToIn),
        (k("s", Key::O), GoToOut),
        (k("c", Key::K), AddEdit),
        (k("cs", Key::K), AddEditAll),
        (k("", Key::Q), RippleTrimPrev),
        (k("", Key::W), RippleTrimNext),
        (k("", Key::Delete), Delete),
        (k("", Key::Backspace), Delete),
        (k("s", Key::Delete), RippleDelete),
        (k("s", Key::Backspace), RippleDelete),
        (k("", Key::Semicolon), Lift),
        (k("", Key::Quote), Extract),
        (k("", Key::Comma), Insert),
        (k("", Key::Period), Overwrite),
        (k("", Key::D), SelectAtPlayhead),
        (k("c", Key::A), SelectAll),
        (k("cs", Key::A), DeselectAll),
        (k("s", Key::E), ToggleEnable),
        (k("a", Key::ArrowLeft), Nudge(-1)),
        (k("a", Key::ArrowRight), Nudge(1)),
        (k("as", Key::ArrowLeft), Nudge(-5)),
        (k("as", Key::ArrowRight), Nudge(5)),
        (k("c", Key::L), ToggleLink),
        (k("c", Key::G), Group),
        (k("cs", Key::G), Ungroup),
        (k("c", Key::D), ApplyVideoTransition),
        (k("cs", Key::D), ApplyAudioTransition),
        (k("s", Key::D), ApplyDefaultTransitions),
        (k("", Key::M), AddMarker),
        (k("", Key::S), ToggleSnap),
        (k("", Key::F), MatchFrame),
        (k("", Key::Equals), ZoomIn),
        (k("", Key::Minus), ZoomOut),
        (k("", Key::Backslash), ZoomFit),
        (k("c", Key::Z), Undo),
        (k("cs", Key::Z), Redo),
        (k("c", Key::S), Save),
        (k("c", Key::I), Import),
        (k("c", Key::M), Export),
        (k("c", Key::R), SpeedDialog),
        (k("", Key::G), GainDialog),
        (k("c", Key::C), Copy),
        (k("c", Key::X), Cut),
        (k("c", Key::V), Paste),
    ]
}

#[test]
fn u_keys_spec_table_is_well_formed() {
    // Guards the literal table above: 38 full rows of two.
    let spec = spec_keymap();
    assert_eq!(spec.len(), 76);
    let chords: HashSet<KeyChord> = spec.iter().map(|(c, _)| *c).collect();
    assert_eq!(chords.len(), 76);
}

#[test]
fn u_keys_keymap_is_exactly_the_table() {
    let got = default_keymap();
    let spec = spec_keymap();
    let mut seen = HashSet::new();
    for (chord, _) in &got {
        assert!(seen.insert(*chord), "U-KEYS: chord {chord:?} appears twice");
    }
    for (chord, action) in &spec {
        let found: Vec<_> = got.iter().filter(|(c, _)| c == chord).collect();
        assert_eq!(
            found,
            vec![&(*chord, *action)],
            "U-KEYS: {chord:?} must map to {action:?}"
        );
    }
    for entry in &got {
        assert!(spec.contains(entry), "U-KEYS: extra entry {entry:?}");
    }
    assert_eq!(got.len(), spec.len());
}

// ------------------------------------------------------------------------------------- U-TOOL

#[test]
fn u_tool_sets_tool_only() {
    let p = B::std().clip(V1, 100, 0, S).build();
    let tools = [
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
    for from in [Tool::Select, Tool::Wand] {
        for t in tools {
            let mut u = ui();
            u.tool = from;
            u.selection = vec![100];
            u.playhead = 3 * F;
            let mut want = u.clone();
            want.tool = t;
            assert_eq!(act(Action::SetTool(t), &p, &mut u), vec![]);
            assert_eq!(u, want);
        }
    }
}

// ----------------------------------------------------------------------------------- U-TOGGLE

#[test]
fn u_toggle_snap_flips_both_ways() {
    let p = B::std().build();
    let mut u = ui();
    let mut want = u.clone();
    want.snap = false;
    assert_eq!(act(Action::ToggleSnap, &p, &mut u), vec![]);
    assert_eq!(u, want);
    want.snap = true;
    assert_eq!(act(Action::ToggleSnap, &p, &mut u), vec![]);
    assert_eq!(u, want);
}

#[test]
fn u_toggle_linked_selection_flips_both_ways() {
    let p = B::std().build();
    let mut u = ui();
    let mut want = u.clone();
    want.linked_selection = false;
    assert_eq!(act(Action::ToggleLinkedSelection, &p, &mut u), vec![]);
    assert_eq!(u, want);
    want.linked_selection = true;
    assert_eq!(act(Action::ToggleLinkedSelection, &p, &mut u), vec![]);
    assert_eq!(u, want);
}

// ------------------------------------------------------------------------------------- U-ZOOM

fn zoom(action: Action, p: &Project, px: f64, scroll: Time, width: f32) -> UiState {
    let mut u = ui();
    u.px_per_sec = px;
    u.scroll = scroll;
    u.view_width = width;
    u.playhead = 2 * F;
    let before = u.clone();
    assert_eq!(act(action, p, &mut u), vec![]);
    let mut rest = u.clone();
    rest.px_per_sec = before.px_per_sec;
    rest.scroll = before.scroll;
    assert_eq!(rest, before, "U-ZOOM changes only px_per_sec and scroll");
    u
}

#[test]
fn u_zoom_in_and_out_with_caps() {
    let p = B::std().clip(V1, 100, 0, 4 * S).build();
    let u = zoom(Action::ZoomIn, &p, 100.0, 3 * S, 1000.0);
    assert_eq!((u.px_per_sec, u.scroll), (150.0, 3 * S), "100 * 1.5");
    assert_eq!(
        zoom(Action::ZoomIn, &p, 1500.0, 0, 1000.0).px_per_sec,
        2000.0
    );
    assert_eq!(
        zoom(Action::ZoomIn, &p, 2000.0, 0, 1000.0).px_per_sec,
        2000.0
    );
    let u = zoom(Action::ZoomOut, &p, 150.0, 3 * S, 1000.0);
    assert_eq!((u.px_per_sec, u.scroll), (100.0, 3 * S), "150 / 1.5");
    assert_eq!(zoom(Action::ZoomOut, &p, 1.2, 0, 1000.0).px_per_sec, 1.0);
    assert_eq!(zoom(Action::ZoomOut, &p, 1.0, 0, 1000.0).px_per_sec, 1.0);
}

#[test]
fn u_zoom_fit() {
    // duration 4 s, width 1000 -> 250 px/s, scroll reset.
    let p = B::std().clip(V1, 100, 0, 4 * S).build();
    let u = zoom(Action::ZoomFit, &p, 100.0, 3 * S, 1000.0);
    assert_eq!((u.px_per_sec, u.scroll), (250.0, 0));
    // duration under 1 s counts as 1 s.
    let short = B::std().clip(V1, 100, 0, S / 4).build();
    assert_eq!(
        zoom(Action::ZoomFit, &short, 5.0, 0, 800.0).px_per_sec,
        800.0
    );
    // empty sequence: max(0, 1) = 1.
    let empty = B::std().build();
    assert_eq!(
        zoom(Action::ZoomFit, &empty, 5.0, S, 600.0).px_per_sec,
        600.0
    );
    // clamps: 500 / 1000 s = 0.5 -> 1; 5000 / 1 s -> 2000. (The fixture media is 1000 s long.)
    let long = B::std().clip(V1, 100, 0, 1000 * S).build();
    assert_eq!(zoom(Action::ZoomFit, &long, 50.0, 0, 500.0).px_per_sec, 1.0);
    assert_eq!(
        zoom(Action::ZoomFit, &short, 50.0, 0, 5000.0).px_per_sec,
        2000.0
    );
}

// -------------------------------------------------------------------------------------- U-OPS

#[test]
fn u_ops_pass_through_and_change_nothing() {
    let p = B::std().clip(V1, 100, 0, S).build();
    let mut u = ui();
    u.selection = vec![100];
    u.playhead = F;
    for (a, op) in [
        (Action::Undo, Op::Undo),
        (Action::Redo, Op::Redo),
        (Action::Save, Op::Save),
        (Action::Import, Op::Import),
        (Action::Export, Op::Export),
        (Action::SpeedDialog, Op::SpeedDialog),
        (Action::GainDialog, Op::GainDialog),
        (Action::WandPrompt, Op::WandPrompt),
    ] {
        let mut v = u.clone();
        assert_eq!(act(a, &p, &mut v), vec![op], "{a:?}");
        assert_eq!(v, u, "{a:?} changes nothing");
    }
}

#[test]
fn u_ops_dialogs_need_a_selection() {
    let p = B::std().clip(V1, 100, 0, S).build();
    assert_noop(Action::SpeedDialog, &p, &ui());
    assert_noop(Action::GainDialog, &p, &ui());
    // Undo etc. do not depend on the selection.
    let mut u = ui();
    assert_eq!(act(Action::Undo, &p, &mut u), vec![Op::Undo]);
    assert_eq!(act(Action::WandPrompt, &p, &mut u), vec![Op::WandPrompt]);
}

// ----------------------------------------------------------------------------------- U-MARKER

#[test]
fn u_marker_adds_marker_at_playhead() {
    let p = B::std().clip(V1, 100, 0, 4 * S).marker(500, S).build();
    let mut u = ui();
    u.playhead = 2 * S + 3 * F;
    u.selection = vec![100];
    let before = u.clone();
    let ops = act(Action::AddMarker, &p, &mut u);
    assert_eq!(
        ops,
        edit(vec![Command::AddMarker {
            sequence_id: SEQ,
            time: 2 * S + 3 * F,
            duration: 0,
            name: String::new(),
            note: String::new(),
        }])
    );
    assert_eq!(u, before);
    let after = apply(&p, &ops);
    let times: Vec<Time> = after.sequences[0].markers.iter().map(|m| m.time).collect();
    assert_eq!(times, vec![S, 2 * S + 3 * F]);
}

// --------------------------------------------------------------- general: missing / empty state

fn busy_project() -> Project {
    B::std()
        .clip(V1, 100, 0, 4 * S)
        .clip(V1, 101, 4 * S, 4 * S)
        .clip(A1, 102, 0, 4 * S)
        .link(&[100, 102], 900)
        .build()
}

fn busy_ui(seq: Id) -> UiState {
    let mut u = ui();
    u.sequence_id = seq;
    u.playhead = 2 * S;
    u.selection = vec![100, 101];
    u.mark_in = Some(S);
    u.mark_out = Some(3 * S);
    u.shuttle = 2;
    u.playing = true;
    u.source = Some(SourceState {
        media_id: MEDIA,
        playhead: 0,
        mark_in: None,
        mark_out: None,
    });
    u
}

/// GUI.md: "missing sequence -> no ops, `ui` unchanged", for every action.
#[test]
fn missing_sequence_every_action_is_noop() {
    let p = busy_project();
    for &a in ALL_ACTIONS {
        assert_noop(a, &p, &busy_ui(STALE));
    }
    let empty = Project::new("empty");
    for &a in ALL_ACTIONS {
        assert_noop(a, &empty, &busy_ui(SEQ));
    }
}

/// An empty sequence: only the actions that need no clips produce ops; no Edit is empty.
#[test]
fn empty_sequence_every_action() {
    let p = B::std().build();
    for &a in ALL_ACTIONS {
        let mut u = ui();
        let ops = act(a, &p, &mut u);
        let want = match a {
            Action::AddMarker => edit(vec![Command::AddMarker {
                sequence_id: SEQ,
                time: 0,
                duration: 0,
                name: String::new(),
                note: String::new(),
            }]),
            Action::Undo => vec![Op::Undo],
            Action::Redo => vec![Op::Redo],
            Action::Save => vec![Op::Save],
            Action::Import => vec![Op::Import],
            Action::Export => vec![Op::Export],
            Action::WandPrompt => vec![Op::WandPrompt],
            Action::Paste => vec![Op::Paste],
            _ => vec![],
        };
        assert_eq!(ops, want, "{a:?} on an empty sequence");
        assert_eq!(
            u.playhead, 0,
            "{a:?}: playhead stays at 0 on an empty sequence"
        );
    }
    // A sequence with no tracks at all.
    let bare = B::new(FPS30).build();
    for &a in ALL_ACTIONS {
        let mut u = ui();
        let ops = act(a, &bare, &mut u);
        assert!(
            ops.iter()
                .all(|o| !matches!(o, Op::Edit(_)) || a == Action::AddMarker),
            "{a:?} on a sequence without tracks: {ops:?}"
        );
    }
}
