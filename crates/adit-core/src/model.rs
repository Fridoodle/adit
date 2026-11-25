//! Plain data. Public fields on purpose: clients read the model directly and change it only
//! through `Document::apply`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Time in flicks. Exact for every common frame rate and sample rate.
pub type Time = i64;
pub type Id = u64;
pub const FLICKS_PER_SECOND: Time = 705_600_000;
/// Upper bound for every time and for `start + duration` of every clip (1000 hours). Keeps all
/// time arithmetic far from i64 overflow (SPEC H-3).
pub const MAX_TIME: Time = 3_600_000 * FLICKS_PER_SECOND;
/// Deepest allowed chain of nested sequences (SPEC H-4).
pub const MAX_NEST_DEPTH: usize = 16;
/// Rounding slack (flicks) allowed when checking a clip against its source length (SPEC I3).
pub const SOURCE_SLACK: Time = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rational {
    pub num: u32,
    pub den: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub next_id: Id,
    pub media: Vec<Media>,
    pub sequences: Vec<Sequence>,
    /// Effects defined at runtime (by Claude or the user). Built-in effects are not stored here.
    pub effects: Vec<EffectDef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaSpec {
    pub name: String,
    pub path: String,
    /// `None` for stills: unbounded.
    pub duration: Option<Time>,
    pub video: Option<VideoInfo>,
    pub audio: Option<AudioInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Media {
    pub id: Id,
    #[serde(flatten)]
    pub spec: MediaSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VideoInfo {
    pub width: u32,
    pub height: u32,
    pub fps: Rational,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AudioInfo {
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sequence {
    pub id: Id,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub fps: Rational,
    pub sample_rate: u32,
    /// Index 0 is the bottom layer. Video tracks with a higher index draw on top.
    pub tracks: Vec<Track>,
    /// Sorted by time.
    pub markers: Vec<Marker>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackKind {
    Video,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: Id,
    pub kind: TrackKind,
    pub name: String,
    pub locked: bool,
    pub muted: bool,
    pub solo: bool,
    pub hidden: bool,
    /// Sorted by start, never overlapping.
    pub clips: Vec<Clip>,
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Marker {
    pub id: Id,
    pub time: Time,
    pub duration: Time,
    pub name: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    pub id: Id,
    pub name: String,
    /// Sequence time of the first frame.
    pub start: Time,
    pub duration: Time,
    pub source: ClipSource,
    /// Source time of the earliest source instant the clip uses.
    pub source_in: Time,
    pub speed: f64,
    pub reverse: bool,
    pub enabled: bool,
    pub link: Option<Id>,
    pub group: Option<Id>,
    pub blend: BlendMode,
    /// ffmpeg `-af` chain applied to this clip's audio. Empty = none.
    pub audio_filter: String,
    /// Built-in properties (see `CLIP_PARAMS`). Absent = default.
    pub params: BTreeMap<String, Param>,
    pub effects: Vec<EffectInstance>,
    /// Colour label, 0 = none, 1..=8 (SPEC C-LABEL-1).
    #[serde(default)]
    pub label: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClipSource {
    Media {
        media_id: Id,
    },
    /// Nested sequence.
    Sequence {
        sequence_id: Id,
    },
    Text {
        text: TextSpec,
    },
    /// RGBA, straight alpha, 0..1. A transparent solid plus effects is how generators are made.
    Solid {
        color: [f32; 4],
    },
    /// Applies its effect stack to everything below it.
    Adjustment,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextSpec {
    pub content: String,
    /// Font family. Empty = default.
    pub font: String,
    /// Pixels at sequence resolution.
    pub size: f64,
    pub color: [f32; 4],
    pub align: TextAlign,
    pub bold: bool,
    pub italic: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlendMode {
    Normal,
    Add,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    Difference,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Param {
    /// Used when there are no keyframes.
    pub value: f64,
    /// Sorted by time, unique times. Times are relative to the clip start.
    pub keyframes: Vec<Keyframe>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Keyframe {
    pub time: Time,
    pub value: f64,
    /// Shapes the segment from this keyframe to the next.
    pub easing: Easing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Easing {
    Linear,
    Hold,
    EaseIn,
    EaseOut,
    EaseInOut,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectInstance {
    pub id: Id,
    /// Name of a built-in effect or of an `EffectDef` in the project.
    pub effect: String,
    pub enabled: bool,
    pub params: BTreeMap<String, Param>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transition {
    pub id: Id,
    /// Clip whose end the transition sits on.
    pub left: Option<Id>,
    /// Clip whose start the transition sits on.
    pub right: Option<Id>,
    pub effect: String,
    pub duration: Time,
    pub params: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectKind {
    Filter,
    Transition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectDef {
    pub name: String,
    pub kind: EffectKind,
    pub wgsl: String,
    pub params: Vec<ParamDef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamDef {
    pub name: String,
    pub default: f64,
    pub min: f64,
    pub max: f64,
}

/// Built-in clip properties and their defaults (SPEC P1).
pub const CLIP_PARAMS: &[(&str, f64)] = &[
    ("position_x", 0.0),
    ("position_y", 0.0),
    ("scale_x", 1.0),
    ("scale_y", 1.0),
    ("rotation", 0.0),
    ("anchor_x", 0.0),
    ("anchor_y", 0.0),
    ("opacity", 1.0),
    ("crop_left", 0.0),
    ("crop_top", 0.0),
    ("crop_right", 0.0),
    ("crop_bottom", 0.0),
    ("volume", 0.0),
    ("pan", 0.0),
];
