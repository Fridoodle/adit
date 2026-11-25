//! Every edit there is. Semantics: docs/SPEC.md, section "Commands".

use crate::{
    BlendMode, Clip, ClipSource, Easing, EffectDef, Id, MediaSpec, Rational, TextSpec, Time,
    TrackKind,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaceMode {
    /// Fail with `Overlap` if the space is taken.
    #[default]
    Place,
    /// Remove whatever is in the way.
    Overwrite,
    /// Split all unlocked tracks at the start and push everything later to the right.
    Insert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Edge {
    Start,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrimMode {
    #[default]
    Normal,
    Ripple,
    Roll,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamTarget {
    pub clip_id: Id,
    /// `None` = a built-in clip property; `Some` = a parameter of that effect instance.
    #[serde(default)]
    pub effect_id: Option<Id>,
    pub name: String,
}

/// One clip to paste: `clip.start` is where the copy goes, `clip.id` is ignored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PasteItem {
    pub track_id: Id,
    pub clip: Clip,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Command {
    // --- media, sequences, tracks, markers, effect definitions ---
    AddMedia {
        spec: MediaSpec,
    },
    RemoveMedia {
        media_id: Id,
    },
    SetMedia {
        media_id: Id,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        path: Option<String>,
    },
    AddSequence {
        name: String,
        width: u32,
        height: u32,
        fps: Rational,
        sample_rate: u32,
        #[serde(default)]
        video_tracks: u32,
        #[serde(default)]
        audio_tracks: u32,
    },
    RemoveSequence {
        sequence_id: Id,
    },
    SetSequence {
        sequence_id: Id,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        width: Option<u32>,
        #[serde(default)]
        height: Option<u32>,
        #[serde(default)]
        fps: Option<Rational>,
        #[serde(default)]
        sample_rate: Option<u32>,
    },
    AddTrack {
        sequence_id: Id,
        kind: TrackKind,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        index: Option<usize>,
    },
    RemoveTrack {
        track_id: Id,
    },
    MoveTrack {
        track_id: Id,
        index: usize,
    },
    SetTrack {
        track_id: Id,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        locked: Option<bool>,
        #[serde(default)]
        muted: Option<bool>,
        #[serde(default)]
        solo: Option<bool>,
        #[serde(default)]
        hidden: Option<bool>,
    },
    AddMarker {
        sequence_id: Id,
        time: Time,
        #[serde(default)]
        duration: Time,
        #[serde(default)]
        name: String,
        #[serde(default)]
        note: String,
    },
    RemoveMarker {
        marker_id: Id,
    },
    DefineEffect {
        def: EffectDef,
    },
    RemoveEffectDef {
        name: String,
    },

    // --- clips ---
    AddClip {
        track_id: Id,
        /// Also place a linked audio clip of the same source on this audio track.
        #[serde(default)]
        linked_track_id: Option<Id>,
        source: ClipSource,
        start: Time,
        duration: Time,
        #[serde(default)]
        source_in: Time,
        #[serde(default)]
        mode: PlaceMode,
        #[serde(default)]
        name: Option<String>,
    },
    RemoveClips {
        clip_ids: Vec<Id>,
        #[serde(default = "yes")]
        include_linked: bool,
    },
    RippleDeleteClips {
        clip_ids: Vec<Id>,
        #[serde(default = "yes")]
        include_linked: bool,
    },
    LiftRange {
        sequence_id: Id,
        start: Time,
        end: Time,
        /// `None` = all unlocked tracks.
        #[serde(default)]
        track_ids: Option<Vec<Id>>,
    },
    ExtractRange {
        sequence_id: Id,
        start: Time,
        end: Time,
        #[serde(default)]
        track_ids: Option<Vec<Id>>,
    },
    CloseGap {
        track_id: Id,
        time: Time,
    },
    SplitAt {
        sequence_id: Id,
        time: Time,
        #[serde(default)]
        track_ids: Option<Vec<Id>>,
    },
    SplitClip {
        clip_id: Id,
        time: Time,
        #[serde(default = "yes")]
        include_linked: bool,
    },
    MoveClip {
        clip_id: Id,
        start: Time,
        #[serde(default)]
        track_id: Option<Id>,
        #[serde(default)]
        mode: PlaceMode,
        #[serde(default = "yes")]
        include_linked: bool,
    },
    TrimClip {
        clip_id: Id,
        edge: Edge,
        /// New sequence time of that edge.
        to: Time,
        #[serde(default)]
        mode: TrimMode,
        #[serde(default = "yes")]
        include_linked: bool,
    },
    SlipClip {
        clip_id: Id,
        delta: Time,
        #[serde(default = "yes")]
        include_linked: bool,
    },
    SlideClip {
        clip_id: Id,
        delta: Time,
    },
    SetClipSpeed {
        clip_id: Id,
        speed: f64,
        #[serde(default)]
        ripple: bool,
        #[serde(default = "yes")]
        include_linked: bool,
    },
    SetClip {
        clip_id: Id,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        enabled: Option<bool>,
        #[serde(default)]
        reverse: Option<bool>,
        #[serde(default)]
        blend: Option<BlendMode>,
        #[serde(default)]
        audio_filter: Option<String>,
        #[serde(default)]
        label: Option<u8>,
    },
    /// Adds copies of whole clips (SPEC C-PASTE-*).
    PasteClips {
        items: Vec<PasteItem>,
        #[serde(default)]
        mode: PlaceMode,
    },
    SetText {
        clip_id: Id,
        text: TextSpec,
    },
    LinkClips {
        clip_ids: Vec<Id>,
    },
    UnlinkClips {
        clip_ids: Vec<Id>,
    },
    GroupClips {
        clip_ids: Vec<Id>,
    },
    UngroupClips {
        clip_ids: Vec<Id>,
    },

    // --- parameters, keyframes, effects, transitions ---
    SetParam {
        target: ParamTarget,
        value: f64,
    },
    SetKeyframe {
        target: ParamTarget,
        /// Relative to the clip start.
        time: Time,
        value: f64,
        easing: Easing,
    },
    RemoveKeyframe {
        target: ParamTarget,
        time: Time,
    },
    ClearKeyframes {
        target: ParamTarget,
    },
    AddEffect {
        clip_id: Id,
        effect: String,
        #[serde(default)]
        index: Option<usize>,
        #[serde(default)]
        params: BTreeMap<String, f64>,
    },
    RemoveEffect {
        clip_id: Id,
        effect_id: Id,
    },
    MoveEffect {
        clip_id: Id,
        effect_id: Id,
        index: usize,
    },
    SetEffectEnabled {
        clip_id: Id,
        effect_id: Id,
        enabled: bool,
    },
    AddTransition {
        track_id: Id,
        #[serde(default)]
        left: Option<Id>,
        #[serde(default)]
        right: Option<Id>,
        effect: String,
        duration: Time,
        #[serde(default)]
        params: BTreeMap<String, f64>,
    },
    RemoveTransition {
        transition_id: Id,
    },
    SetTransition {
        transition_id: Id,
        #[serde(default)]
        effect: Option<String>,
        #[serde(default)]
        duration: Option<Time>,
        #[serde(default)]
        params: Option<BTreeMap<String, f64>>,
    },

    /// All or nothing, one undo step.
    Batch {
        commands: Vec<Command>,
    },
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Outcome {
    /// Ids of the entities the command created, in the order SPEC.md states for that command.
    pub created: Vec<Id>,
}
