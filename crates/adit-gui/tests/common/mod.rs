//! Shared fixtures for the adit-gui pure-logic tests (docs/GUI.md U-*).
//! Projects are built as plain structs with fixed ids so expected values can be written as
//! literals. Expected values come from GUI.md rule text and hand arithmetic, never from the code
//! under test.
#![allow(dead_code)]

pub use adit_core::*;
pub use adit_gui::*;
use std::collections::BTreeMap;

/// Flicks per second.
pub const S: Time = FLICKS_PER_SECOND;
/// One frame at 30 fps: 705_600_000 / 30.
pub const F: Time = 23_520_000;
pub const FPS30: Rational = Rational { num: 30, den: 1 };
pub const NTSC: Rational = Rational {
    num: 30000,
    den: 1001,
};
/// One frame at 30000/1001 fps: 705_600_000 * 1001 / 30000 (exact).
pub const NF: Time = 23_543_520;

pub const SEQ: Id = 1;
/// Default media of every fixture clip: video + audio, 1000 s.
pub const MEDIA: Id = 2;
/// A second sequence used to check that other sequences are ignored.
pub const OTHER_SEQ: Id = 3;
pub const OTHER_TRACK: Id = 4;
pub const V1: Id = 11;
pub const V2: Id = 12;
pub const V3: Id = 13;
pub const A1: Id = 21;
pub const A2: Id = 22;
/// An id that exists nowhere (stale selection entries, missing sequences).
pub const STALE: Id = 9_999;

pub struct B {
    pub p: Project,
}

impl B {
    /// Empty project with sequence `SEQ` (no tracks) and media `MEDIA`.
    pub fn new(fps: Rational) -> Self {
        let mut b = B {
            p: Project {
                name: "t".into(),
                next_id: 100_000,
                media: Vec::new(),
                sequences: vec![Sequence {
                    id: SEQ,
                    name: "Seq".into(),
                    width: 1920,
                    height: 1080,
                    fps,
                    sample_rate: 48000,
                    tracks: Vec::new(),
                    markers: Vec::new(),
                }],
                effects: Vec::new(),
            },
        };
        b = b.media(MEDIA, Some(1000 * S), true, true);
        b
    }

    /// 30 fps, tracks V1, V2, A1, A2 (in that `tracks` order).
    pub fn std() -> Self {
        B::new(FPS30)
            .track(V1, TrackKind::Video)
            .track(V2, TrackKind::Video)
            .track(A1, TrackKind::Audio)
            .track(A2, TrackKind::Audio)
    }

    pub fn media(mut self, id: Id, duration: Option<Time>, video: bool, audio: bool) -> Self {
        self.p.media.push(Media {
            id,
            spec: MediaSpec {
                name: format!("m{id}"),
                path: format!("/m/{id}.mp4"),
                duration,
                video: video.then_some(VideoInfo {
                    width: 1920,
                    height: 1080,
                    fps: FPS30,
                }),
                audio: audio.then_some(AudioInfo {
                    sample_rate: 48000,
                    channels: 2,
                }),
            },
        });
        self
    }

    pub fn track(mut self, id: Id, kind: TrackKind) -> Self {
        self.seq_mut().tracks.push(Track {
            id,
            kind,
            name: format!("t{id}"),
            locked: false,
            muted: false,
            solo: false,
            hidden: false,
            clips: Vec::new(),
            transitions: Vec::new(),
        });
        self
    }

    /// A `Media(MEDIA)` clip, `source_in` 0, speed 1.
    pub fn clip(mut self, track: Id, id: Id, start: Time, duration: Time) -> Self {
        let t = self.track_mut(track);
        t.clips.push(mk_clip(id, start, duration));
        t.clips.sort_by_key(|c| c.start);
        self
    }

    pub fn with(mut self, clip: Id, f: impl FnOnce(&mut Clip)) -> Self {
        let c = self
            .p
            .sequences
            .iter_mut()
            .flat_map(|s| &mut s.tracks)
            .flat_map(|t| &mut t.clips)
            .find(|c| c.id == clip)
            .expect("fixture clip exists");
        f(c);
        self
    }

    pub fn link(mut self, ids: &[Id], link: Id) -> Self {
        for &id in ids {
            self = self.with(id, |c| c.link = Some(link));
        }
        self
    }

    pub fn lock(mut self, track: Id) -> Self {
        self.track_mut(track).locked = true;
        self
    }

    pub fn marker(mut self, id: Id, time: Time) -> Self {
        let s = self.seq_mut();
        s.markers.push(Marker {
            id,
            time,
            duration: 0,
            name: String::new(),
            note: String::new(),
        });
        s.markers.sort_by_key(|m| m.time);
        self
    }

    /// Adds sequence `OTHER_SEQ` with one video track `OTHER_TRACK` holding clip `id`.
    pub fn other_seq_clip(mut self, id: Id, start: Time, duration: Time) -> Self {
        let mut s = self.p.sequences[0].clone();
        s.id = OTHER_SEQ;
        s.markers.clear();
        s.tracks = vec![Track {
            id: OTHER_TRACK,
            kind: TrackKind::Video,
            name: "o".into(),
            locked: false,
            muted: false,
            solo: false,
            hidden: false,
            clips: vec![mk_clip(id, start, duration)],
            transitions: Vec::new(),
        }];
        self.p.sequences.push(s);
        self
    }

    pub fn build(self) -> Project {
        if let Err(e) = self.p.validate() {
            panic!("fixture is invalid: {e}");
        }
        self.p
    }

    fn seq_mut(&mut self) -> &mut Sequence {
        &mut self.p.sequences[0]
    }

    fn track_mut(&mut self, id: Id) -> &mut Track {
        self.p.sequences[0]
            .tracks
            .iter_mut()
            .find(|t| t.id == id)
            .expect("fixture track exists")
    }
}

pub fn mk_clip(id: Id, start: Time, duration: Time) -> Clip {
    Clip {
        id,
        name: format!("c{id}"),
        start,
        duration,
        source: ClipSource::Media { media_id: MEDIA },
        source_in: 0,
        speed: 1.0,
        reverse: false,
        enabled: true,
        link: None,
        group: None,
        blend: BlendMode::Normal,
        audio_filter: String::new(),
        params: BTreeMap::new(),
        effects: Vec::new(),
        label: 0,
    }
}

/// U-STATE defaults written out by hand, so tests of other rules do not depend on `UiState::new`.
pub fn ui() -> UiState {
    UiState {
        sequence_id: SEQ,
        playhead: 0,
        selection: Vec::new(),
        mark_in: None,
        mark_out: None,
        tool: Tool::Select,
        snap: true,
        linked_selection: true,
        playing: false,
        shuttle: 0,
        targets: Vec::new(),
        source: None,
        px_per_sec: 100.0,
        scroll: 0,
        view_width: 1000.0,
    }
}

#[track_caller]
pub fn no_empty_edit(ops: &[Op]) {
    for op in ops {
        if let Op::Edit(cmds) = op {
            assert!(!cmds.is_empty(), "an Op::Edit must never be empty: {ops:?}");
        }
    }
}

/// `perform` plus the "never an empty Edit" check.
#[track_caller]
pub fn act(action: Action, p: &Project, ui: &mut UiState) -> Vec<Op> {
    let ops = perform(action, p, ui);
    no_empty_edit(&ops);
    ops
}

/// `perform` on a clone of `ui`; asserts no ops and that `ui` did not change at all.
#[track_caller]
pub fn assert_noop(action: Action, p: &Project, ui: &UiState) {
    let mut u = ui.clone();
    let ops = act(action, p, &mut u);
    assert_eq!(ops, vec![], "{action:?} must return no op");
    assert_eq!(&u, ui, "{action:?} must not change ui");
}

/// `drag` plus the "never an empty Edit" check.
#[track_caller]
pub fn gesture(d: Drag, p: &Project, ui: &mut UiState) -> Vec<Op> {
    let ops = drag(d, p, ui);
    no_empty_edit(&ops);
    ops
}

#[track_caller]
pub fn assert_drag_noop(d: Drag, p: &Project, ui: &UiState) {
    let mut u = ui.clone();
    let ops = gesture(d, p, &mut u);
    assert_eq!(ops, vec![], "{d:?} must return no op");
    assert_eq!(&u, ui, "{d:?} must not change ui");
}

pub fn edit(cmds: Vec<Command>) -> Vec<Op> {
    vec![Op::Edit(cmds)]
}

/// Applies the single `Op::Edit` in `ops` to `p` as one `Batch`, as the window does.
#[track_caller]
pub fn apply(p: &Project, ops: &[Op]) -> Project {
    let [Op::Edit(cmds)] = ops else {
        panic!("expected exactly one Op::Edit, got {ops:?}");
    };
    let mut doc = Document::new(p.clone());
    if let Err(e) = doc.apply(Command::Batch {
        commands: cmds.clone(),
    }) {
        panic!("core rejected {cmds:?}: {e:?}");
    }
    doc.project().clone()
}

/// `(id, start, duration)` of every clip on a track, in order.
pub fn clips_on(p: &Project, track: Id) -> Vec<(Id, Time, Time)> {
    p.track(track)
        .expect("track exists")
        .1
        .clips
        .iter()
        .map(|c| (c.id, c.start, c.duration))
        .collect()
}

pub fn clip(p: &Project, id: Id) -> &Clip {
    p.clip(id).expect("clip exists").2
}

/// Sorts commands by their debug text so tests can compare where GUI.md leaves the order open.
pub fn sorted(mut cmds: Vec<Command>) -> Vec<Command> {
    cmds.sort_by_key(|c| format!("{c:?}"));
    cmds
}

/// Every `Action` value used by the keymap plus the ones without a key.
pub const ALL_ACTIONS: &[Action] = &[
    Action::SetTool(Tool::Select),
    Action::SetTool(Tool::TrackSelect),
    Action::SetTool(Tool::Ripple),
    Action::SetTool(Tool::Roll),
    Action::SetTool(Tool::RateStretch),
    Action::SetTool(Tool::Razor),
    Action::SetTool(Tool::Slip),
    Action::SetTool(Tool::Slide),
    Action::SetTool(Tool::Pen),
    Action::SetTool(Tool::Hand),
    Action::SetTool(Tool::Zoom),
    Action::SetTool(Tool::Type),
    Action::SetTool(Tool::Wand),
    Action::PlayPause,
    Action::ShuttleLeft,
    Action::ShuttleStop,
    Action::ShuttleRight,
    Action::Step(-1),
    Action::Step(1),
    Action::Step(-5),
    Action::Step(5),
    Action::GoToStart,
    Action::GoToEnd,
    Action::PrevEdit,
    Action::NextEdit,
    Action::MarkIn,
    Action::MarkOut,
    Action::MarkClip,
    Action::ClearIn,
    Action::ClearOut,
    Action::ClearInOut,
    Action::GoToIn,
    Action::GoToOut,
    Action::AddEdit,
    Action::AddEditAll,
    Action::RippleTrimPrev,
    Action::RippleTrimNext,
    Action::Delete,
    Action::RippleDelete,
    Action::Lift,
    Action::Extract,
    Action::Insert,
    Action::Overwrite,
    Action::SelectAtPlayhead,
    Action::SelectAll,
    Action::DeselectAll,
    Action::Nudge(-1),
    Action::Nudge(1),
    Action::Nudge(-5),
    Action::Nudge(5),
    Action::ToggleEnable,
    Action::ToggleLink,
    Action::Group,
    Action::Ungroup,
    Action::ApplyVideoTransition,
    Action::ApplyAudioTransition,
    Action::ApplyDefaultTransitions,
    Action::AddMarker,
    Action::ToggleSnap,
    Action::ToggleLinkedSelection,
    Action::MatchFrame,
    Action::ZoomIn,
    Action::ZoomOut,
    Action::ZoomFit,
    Action::Undo,
    Action::Redo,
    Action::Save,
    Action::Import,
    Action::Export,
    Action::SpeedDialog,
    Action::GainDialog,
    Action::WandPrompt,
];
