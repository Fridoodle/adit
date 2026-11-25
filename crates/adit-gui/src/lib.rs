//! adit-gui: the Premiere-style window. Everything the user does becomes `adit_core::Command`s sent
//! through the same `adit_mcp::Session` an MCP client uses, so both see each other's edits.
//!
//! Contract: docs/GUI.md. The pure part (state, key map, actions, geometry, drags) is specified
//! rule by rule and tested without a window; `App` is the egui shell around it. Public items and
//! signatures in this file are fixed; bodies and private modules are free.

use adit_core::{Command, Id, Project, Time};
use eframe::egui;

mod app;
mod logic;

// ------------------------------------------------------------------------------------- state

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tool {
    Select,
    TrackSelect,
    Ripple,
    Roll,
    RateStretch,
    Razor,
    Slip,
    Slide,
    Pen,
    Hand,
    Zoom,
    Type,
    Wand,
}

/// What is loaded in the Source monitor.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceState {
    pub media_id: Id,
    pub playhead: Time,
    pub mark_in: Option<Time>,
    pub mark_out: Option<Time>,
}

/// Per-sequence editing state of the window. Plain data; `perform` and `drag` are the only logic.
#[derive(Debug, Clone, PartialEq)]
pub struct UiState {
    pub sequence_id: Id,
    /// Always on a frame boundary of the sequence.
    pub playhead: Time,
    /// Selected clip ids.
    pub selection: Vec<Id>,
    pub mark_in: Option<Time>,
    pub mark_out: Option<Time>,
    pub tool: Tool,
    pub snap: bool,
    /// Selecting or editing a clip also takes its linked clips.
    pub linked_selection: bool,
    pub playing: bool,
    /// J/K/L speed: 0 stopped, 1 forward at normal speed, -2 double speed backwards, ... (-8..=8).
    pub shuttle: i32,
    /// Targeted track ids. Empty = every unlocked track.
    pub targets: Vec<Id>,
    pub source: Option<SourceState>,
    /// Timeline zoom in pixels per second.
    pub px_per_sec: f64,
    /// Sequence time at the left edge of the timeline area.
    pub scroll: Time,
    /// Width of the timeline area in pixels (set by the window each frame).
    pub view_width: f32,
}

impl UiState {
    /// Defaults (GUI.md U-STATE) for a sequence.
    pub fn new(sequence_id: Id) -> Self {
        logic::new_state(sequence_id)
    }
}

// ----------------------------------------------------------------------------------- actions

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    SetTool(Tool),
    PlayPause,
    ShuttleLeft,
    ShuttleStop,
    ShuttleRight,
    /// Frames, signed.
    Step(i64),
    GoToStart,
    GoToEnd,
    PrevEdit,
    NextEdit,
    MarkIn,
    MarkOut,
    MarkClip,
    ClearIn,
    ClearOut,
    ClearInOut,
    GoToIn,
    GoToOut,
    AddEdit,
    AddEditAll,
    RippleTrimPrev,
    RippleTrimNext,
    Delete,
    RippleDelete,
    Lift,
    Extract,
    Insert,
    Overwrite,
    SelectAtPlayhead,
    SelectAll,
    DeselectAll,
    /// Frames, signed.
    Nudge(i64),
    ToggleEnable,
    ToggleLink,
    Group,
    Ungroup,
    ApplyVideoTransition,
    ApplyAudioTransition,
    ApplyDefaultTransitions,
    AddMarker,
    ToggleSnap,
    ToggleLinkedSelection,
    MatchFrame,
    ZoomIn,
    ZoomOut,
    ZoomFit,
    Undo,
    Redo,
    Save,
    Import,
    Export,
    SpeedDialog,
    GainDialog,
    WandPrompt,
}

/// What the window must do after an action, in order.
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    /// Apply as one atomic batch = one undo step.
    Edit(Vec<Command>),
    Undo,
    Redo,
    Save,
    Import,
    Export,
    SpeedDialog,
    GainDialog,
    WandPrompt,
}

/// Runs an action against the current project (GUI.md U-*). Pure: changes only `ui`.
pub fn perform(action: Action, project: &Project, ui: &mut UiState) -> Vec<Op> {
    logic::perform(action, project, ui)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyChord {
    pub key: egui::Key,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

/// Premiere Pro default keys (GUI.md U-KEYS). No chord appears twice.
pub fn default_keymap() -> Vec<(KeyChord, Action)> {
    logic::default_keymap()
}

// ---------------------------------------------------------------------------------- geometry

/// Layout of the timeline panel in panel-local pixels (GUI.md U-GEO).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineLayout {
    pub px_per_sec: f64,
    pub scroll: Time,
    /// Width of the track header column at the left.
    pub header_width: f32,
    pub ruler_height: f32,
    pub track_height: f32,
    /// Half-width of the zone at a clip edge that grabs the edge instead of the body.
    pub edge_px: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipPart {
    Body,
    StartEdge,
    EndEdge,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hit {
    Ruler {
        time: Time,
    },
    Header {
        track_id: Id,
    },
    Clip {
        clip_id: Id,
        track_id: Id,
        part: ClipPart,
        time: Time,
    },
    Empty {
        track_id: Id,
        time: Time,
    },
    Nothing,
}

impl TimelineLayout {
    pub fn time_to_x(&self, time: Time) -> f32 {
        logic::time_to_x(self, time)
    }

    pub fn x_to_time(&self, x: f32) -> Time {
        logic::x_to_time(self, x)
    }

    /// Track ids top to bottom as drawn.
    pub fn rows(&self, project: &Project, sequence_id: Id) -> Vec<Id> {
        logic::rows(project, sequence_id)
    }

    pub fn hit(&self, project: &Project, sequence_id: Id, x: f32, y: f32) -> Hit {
        logic::hit(self, project, sequence_id, x, y)
    }
}

/// Snaps `time` (GUI.md U-SNAP). `exclude` = clips being dragged; `tolerance` in flicks.
pub fn snap(time: Time, project: &Project, ui: &UiState, exclude: &[Id], tolerance: Time) -> Time {
    logic::snap(time, project, ui, exclude, tolerance)
}

// ------------------------------------------------------------------------------------- drags

/// A finished mouse gesture on the timeline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Drag {
    /// What was under the pointer at press.
    pub from: Hit,
    /// Sequence time under the pointer at release (already snapped by the caller).
    pub to_time: Time,
    /// Track under the pointer at release, if any.
    pub to_track: Option<Id>,
    pub shift: bool,
    pub alt: bool,
}

/// Turns a gesture with the current tool into edits and state changes (GUI.md U-DRAG).
/// A press and release at the same time and track is a click.
pub fn drag(drag: Drag, project: &Project, ui: &mut UiState) -> Vec<Op> {
    logic::drag(drag, project, ui)
}

// ------------------------------------------------------------------------------------ window

#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Project file to open at start.
    pub open: Option<String>,
    /// Shown in the status bar so the user can connect Claude (GUI.md U-APP-6).
    pub mcp_url: Option<String>,
    pub mcp_token: Option<String>,
}

pub use app::{App, run};
