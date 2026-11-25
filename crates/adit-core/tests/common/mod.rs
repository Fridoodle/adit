//! Shared fixtures and assertion helpers for the adit-core integration tests.
//! Fixtures are built as plain `Project` structs (public fields) so setup never depends on the
//! command layer under test. Expected values in the tests are literals from SPEC.md arithmetic.
#![allow(dead_code)]

pub use adit_core::*;
use std::collections::BTreeMap;

pub const S: Time = FLICKS_PER_SECOND;
pub const FPS24: Rational = Rational { num: 24, den: 1 };

/// Apply `cmd`, require `Ok`, require `validate()` to pass (G2) and check the G2 undo bookkeeping.
pub fn ok(doc: &mut Document, cmd: Command) -> Outcome {
    let before = doc.project().clone();
    let undo = doc.undo_depth();
    let redo = doc.redo_depth();
    let out = match doc.apply(cmd.clone()) {
        Ok(o) => o,
        Err(e) => panic!("expected Ok for {cmd:?}, got Err({e:?})"),
    };
    if let Err(why) = doc.project().validate() {
        panic!("G2: validate() failed after {cmd:?}: {why}");
    }
    if *doc.project() != before {
        assert_eq!(
            doc.undo_depth(),
            undo + 1,
            "G2: a changing command pushes one undo entry"
        );
        assert_eq!(doc.redo_depth(), 0, "G2: a changing command clears redo");
    } else {
        assert_eq!(doc.undo_depth(), undo, "G2: a no-op pushes nothing");
        assert_eq!(doc.redo_depth(), redo, "G2: a no-op keeps redo");
    }
    assert!(
        doc.project().next_id >= before.next_id,
        "G4: next_id never decreases"
    );
    out
}

/// Apply `cmd`, require `Err`, and check G1 (project and depths unchanged).
pub fn err(doc: &mut Document, cmd: Command) -> Error {
    let before = doc.project().clone();
    let undo = doc.undo_depth();
    let redo = doc.redo_depth();
    let e = match doc.apply(cmd.clone()) {
        Ok(o) => panic!("expected Err for {cmd:?}, got Ok({o:?})"),
        Err(e) => e,
    };
    assert_eq!(
        doc.project(),
        &before,
        "G1: project changed by failing {cmd:?}"
    );
    assert_eq!(doc.undo_depth(), undo, "G1: undo depth changed");
    assert_eq!(doc.redo_depth(), redo, "G1: redo depth changed");
    e
}

/// `assert_err!(&mut doc, cmd, Error::Overlap { .. })` — `err` plus a pattern check.
#[macro_export]
macro_rules! assert_err {
    ($doc:expr, $cmd:expr, $pat:pat $(if $guard:expr)?) => {{
        let e = $crate::common::err($doc, $cmd);
        assert!(
            matches!(e, $pat $(if $guard)?),
            "expected {}, got {:?}",
            stringify!($pat),
            e
        );
        e
    }};
}

pub fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * (1.0 + a.abs().max(b.abs()))
}

#[track_caller]
pub fn assert_approx(a: f64, b: f64) {
    assert!(approx(a, b), "{a} != {b}");
}

// ---------------------------------------------------------------------------------------------
// Fixture builder

pub fn mk_clip(id: Id, source: ClipSource, start: Time, duration: Time, source_in: Time) -> Clip {
    Clip {
        id,
        name: format!("c{id}"),
        start,
        duration,
        source,
        source_in,
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

pub fn media_src(media_id: Id) -> ClipSource {
    ClipSource::Media { media_id }
}

pub fn seq_src(sequence_id: Id) -> ClipSource {
    ClipSource::Sequence { sequence_id }
}

pub fn text_spec(content: &str) -> TextSpec {
    TextSpec {
        content: content.to_string(),
        font: String::new(),
        size: 48.0,
        color: [1.0, 1.0, 1.0, 1.0],
        align: TextAlign::Center,
        bold: false,
        italic: false,
    }
}

pub fn text_src(content: &str) -> ClipSource {
    ClipSource::Text {
        text: text_spec(content),
    }
}

pub fn solid_src() -> ClipSource {
    ClipSource::Solid {
        color: [0.0, 0.0, 0.0, 1.0],
    }
}

pub fn kf(time: Time, value: f64, easing: Easing) -> Keyframe {
    Keyframe {
        time,
        value,
        easing,
    }
}

pub fn param(value: f64, keyframes: Vec<Keyframe>) -> Param {
    Param { value, keyframes }
}

pub struct Fx {
    pub p: Project,
}

impl Fx {
    pub fn new() -> Self {
        Fx {
            p: Project {
                name: "test".to_string(),
                next_id: 1,
                media: Vec::new(),
                sequences: Vec::new(),
                effects: Vec::new(),
            },
        }
    }

    pub fn id(&mut self) -> Id {
        let id = self.p.next_id;
        self.p.next_id += 1;
        id
    }

    pub fn media_full(
        &mut self,
        name: &str,
        duration: Option<Time>,
        video: bool,
        audio: bool,
    ) -> Id {
        let id = self.id();
        self.p.media.push(Media {
            id,
            spec: MediaSpec {
                name: name.to_string(),
                path: format!("/media/{name}.mp4"),
                duration,
                video: video.then_some(VideoInfo {
                    width: 1920,
                    height: 1080,
                    fps: FPS24,
                }),
                audio: audio.then_some(AudioInfo {
                    sample_rate: 48000,
                    channels: 2,
                }),
            },
        });
        id
    }

    pub fn seq(&mut self, name: &str) -> Id {
        let id = self.id();
        self.p.sequences.push(Sequence {
            id,
            name: name.to_string(),
            width: 1920,
            height: 1080,
            fps: FPS24,
            sample_rate: 48000,
            tracks: Vec::new(),
            markers: Vec::new(),
        });
        id
    }

    pub fn track(&mut self, seq: Id, kind: TrackKind) -> Id {
        let id = self.id();
        let s = self.seq_mut(seq);
        let k = s.tracks.iter().filter(|t| t.kind == kind).count() + 1;
        let prefix = match kind {
            TrackKind::Video => "V",
            TrackKind::Audio => "A",
        };
        s.tracks.push(Track {
            id,
            kind,
            name: format!("{prefix}{k}"),
            locked: false,
            muted: false,
            solo: false,
            hidden: false,
            clips: Vec::new(),
            transitions: Vec::new(),
        });
        id
    }

    pub fn clip_src(
        &mut self,
        track: Id,
        source: ClipSource,
        start: Time,
        duration: Time,
        source_in: Time,
    ) -> Id {
        let id = self.id();
        let t = self.t(track);
        t.clips
            .push(mk_clip(id, source, start, duration, source_in));
        t.clips.sort_by_key(|c| c.start);
        id
    }

    pub fn clip(
        &mut self,
        track: Id,
        media: Id,
        start: Time,
        duration: Time,
        source_in: Time,
    ) -> Id {
        self.clip_src(track, media_src(media), start, duration, source_in)
    }

    pub fn text(&mut self, track: Id, start: Time, duration: Time) -> Id {
        self.clip_src(track, text_src("hello"), start, duration, 0)
    }

    /// Allocates a link id and gives it to every listed clip.
    pub fn link(&mut self, clips: &[Id]) -> Id {
        let l = self.id();
        for c in clips {
            self.c(*c).link = Some(l);
        }
        l
    }

    pub fn lock(&mut self, track: Id) {
        self.t(track).locked = true;
    }

    pub fn effect(&mut self, clip: Id, name: &str, params: &[(&str, Param)]) -> Id {
        let id = self.id();
        let params = params
            .iter()
            .map(|(n, p)| (n.to_string(), p.clone()))
            .collect();
        self.c(clip).effects.push(EffectInstance {
            id,
            effect: name.to_string(),
            enabled: true,
            params,
        });
        id
    }

    pub fn transition(
        &mut self,
        track: Id,
        left: Option<Id>,
        right: Option<Id>,
        duration: Time,
    ) -> Id {
        let id = self.id();
        self.t(track).transitions.push(Transition {
            id,
            left,
            right,
            effect: "dissolve".to_string(),
            duration,
            params: BTreeMap::new(),
        });
        id
    }

    /// Appends; the caller adds markers in time order.
    pub fn marker(&mut self, seq: Id, time: Time, duration: Time) -> Id {
        let id = self.id();
        self.seq_mut(seq).markers.push(Marker {
            id,
            time,
            duration,
            name: format!("mk{id}"),
            note: String::new(),
        });
        id
    }

    pub fn seq_mut(&mut self, id: Id) -> &mut Sequence {
        self.p
            .sequences
            .iter_mut()
            .find(|s| s.id == id)
            .expect("fixture: no such sequence")
    }

    pub fn t(&mut self, id: Id) -> &mut Track {
        self.p
            .sequences
            .iter_mut()
            .flat_map(|s| s.tracks.iter_mut())
            .find(|t| t.id == id)
            .expect("fixture: no such track")
    }

    pub fn c(&mut self, id: Id) -> &mut Clip {
        self.p
            .sequences
            .iter_mut()
            .flat_map(|s| s.tracks.iter_mut())
            .flat_map(|t| t.clips.iter_mut())
            .find(|c| c.id == id)
            .expect("fixture: no such clip")
    }

    /// `doc()` on a copy, so the fixture can be edited further.
    pub fn clone_doc(&self) -> Document {
        Fx { p: self.p.clone() }.doc()
    }

    /// The fixture must itself be valid; then wrap it in a fresh Document.
    pub fn doc(self) -> Document {
        if let Err(why) = self.p.validate() {
            panic!("fixture is invalid: {why}");
        }
        Document::new(self.p)
    }
}

/// Media 1 "M" (100 s, video+audio), 2 "Still" (no duration, video), 3 "Aud" (100 s, audio only),
/// 4 "Vid" (100 s, video only); sequence 5 "Main" with tracks V1=6, V2=7, A1=8, A2=9. next_id = 10.
pub struct Base {
    pub fx: Fx,
    pub m: Id,
    pub still: Id,
    pub aud: Id,
    pub vid: Id,
    pub seq: Id,
    pub v1: Id,
    pub v2: Id,
    pub a1: Id,
    pub a2: Id,
}

pub fn base() -> Base {
    let mut fx = Fx::new();
    let m = fx.media_full("M", Some(100 * S), true, true);
    let still = fx.media_full("Still", None, true, false);
    let aud = fx.media_full("Aud", Some(100 * S), false, true);
    let vid = fx.media_full("Vid", Some(100 * S), true, false);
    let seq = fx.seq("Main");
    let v1 = fx.track(seq, TrackKind::Video);
    let v2 = fx.track(seq, TrackKind::Video);
    let a1 = fx.track(seq, TrackKind::Audio);
    let a2 = fx.track(seq, TrackKind::Audio);
    assert_eq!(
        (m, still, aud, vid, seq, v1, v2, a1, a2),
        (1, 2, 3, 4, 5, 6, 7, 8, 9)
    );
    Base {
        fx,
        m,
        still,
        aud,
        vid,
        seq,
        v1,
        v2,
        a1,
        a2,
    }
}

// ---------------------------------------------------------------------------------------------
// Read helpers (plain field walks, independent of the `Project::clip` query under test)

pub fn find_clip(p: &Project, id: Id) -> Option<&Clip> {
    p.sequences
        .iter()
        .flat_map(|s| &s.tracks)
        .flat_map(|t| &t.clips)
        .find(|c| c.id == id)
}

#[track_caller]
pub fn clip(p: &Project, id: Id) -> &Clip {
    find_clip(p, id).unwrap_or_else(|| panic!("clip {id} not in project"))
}

#[track_caller]
pub fn trk(p: &Project, id: Id) -> &Track {
    p.sequences
        .iter()
        .flat_map(|s| &s.tracks)
        .find(|t| t.id == id)
        .unwrap_or_else(|| panic!("track {id} not in project"))
}

#[track_caller]
pub fn sq(p: &Project, id: Id) -> &Sequence {
    p.sequences
        .iter()
        .find(|s| s.id == id)
        .unwrap_or_else(|| panic!("sequence {id} not in project"))
}

/// Track that holds clip `id`.
#[track_caller]
pub fn track_of(p: &Project, id: Id) -> Id {
    p.sequences
        .iter()
        .flat_map(|s| &s.tracks)
        .find(|t| t.clips.iter().any(|c| c.id == id))
        .map(|t| t.id)
        .unwrap_or_else(|| panic!("clip {id} not in project"))
}

/// `(id, start, end)` of every clip on the track, in stored order.
pub fn spans(p: &Project, track: Id) -> Vec<(Id, Time, Time)> {
    trk(p, track)
        .clips
        .iter()
        .map(|c| (c.id, c.start, c.end()))
        .collect()
}

pub fn all_clip_ids(p: &Project) -> Vec<Id> {
    p.sequences
        .iter()
        .flat_map(|s| &s.tracks)
        .flat_map(|t| &t.clips)
        .map(|c| c.id)
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Command builders

/// `AddClip` with every field; use `..addb(..)` for the defaults.
pub struct AddB {
    pub track_id: Id,
    pub linked_track_id: Option<Id>,
    pub source: ClipSource,
    pub start: Time,
    pub duration: Time,
    pub source_in: Time,
    pub mode: PlaceMode,
    pub name: Option<String>,
}

impl AddB {
    pub fn cmd(self) -> Command {
        Command::AddClip {
            track_id: self.track_id,
            linked_track_id: self.linked_track_id,
            source: self.source,
            start: self.start,
            duration: self.duration,
            source_in: self.source_in,
            mode: self.mode,
            name: self.name,
        }
    }
}

pub fn addb(track_id: Id, source: ClipSource, start: Time, duration: Time) -> AddB {
    AddB {
        track_id,
        linked_track_id: None,
        source,
        start,
        duration,
        source_in: 0,
        mode: PlaceMode::Place,
        name: None,
    }
}

pub fn add(track_id: Id, source: ClipSource, start: Time, duration: Time) -> Command {
    addb(track_id, source, start, duration).cmd()
}

pub fn add_m(track_id: Id, media: Id, start: Time, duration: Time) -> Command {
    add(track_id, media_src(media), start, duration)
}

pub fn trim(clip_id: Id, edge: Edge, to: Time, mode: TrimMode) -> Command {
    Command::TrimClip {
        clip_id,
        edge,
        to,
        mode,
        include_linked: true,
    }
}

pub fn pt(clip_id: Id, name: &str) -> ParamTarget {
    ParamTarget {
        clip_id,
        effect_id: None,
        name: name.to_string(),
    }
}

pub fn pte(clip_id: Id, effect_id: Id, name: &str) -> ParamTarget {
    ParamTarget {
        clip_id,
        effect_id: Some(effect_id),
        name: name.to_string(),
    }
}

pub fn add_marker(sequence_id: Id, time: Time) -> Command {
    Command::AddMarker {
        sequence_id,
        time,
        duration: 0,
        name: String::new(),
        note: String::new(),
    }
}
