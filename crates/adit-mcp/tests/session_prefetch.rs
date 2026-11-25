//! GUI.md S-6: `Session::prefetch` forwards `media_frames` (SPEC V-11) to `MediaBackend::prefetch`.
//! Expected calls are worked out by hand from the project below.
use adit_core::*;
use adit_mcp::Session;
use adit_media::{Frame, MediaBackend, MediaError, Pattern};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

const S: Time = 705_600_000;

/// 1 ms in flicks is 705_600.
const fn ms(n: i64) -> Time {
    n * 705_600
}

struct Rec {
    inner: Pattern,
    calls: Mutex<Vec<(String, Time)>>,
}

impl MediaBackend for Rec {
    fn probe(&self, path: &str) -> Result<MediaSpec, MediaError> {
        self.inner.probe(path)
    }
    fn video_frame(&self, path: &str, time: Time) -> Result<Arc<Frame>, MediaError> {
        self.inner.video_frame(path, time)
    }
    fn prefetch(&self, path: &str, time: Time) {
        self.calls.lock().unwrap().push((path.to_string(), time));
    }
    fn audio(
        &self,
        path: &str,
        start: Time,
        duration: Time,
        sample_rate: u32,
        filter: &str,
    ) -> Result<Vec<f32>, MediaError> {
        self.inner.audio(path, start, duration, sample_rate, filter)
    }
}

fn rec() -> (Arc<Rec>, Session) {
    let r = Arc::new(Rec {
        inner: Pattern {
            width: 64,
            height: 64,
            fps: Rational { num: 30, den: 1 },
            duration: Some(10 * S),
        },
        calls: Mutex::new(vec![]),
    });
    let s = Session::new(r.clone());
    (r, s)
}

fn media(id: Id, path: &str) -> Media {
    Media {
        id,
        spec: MediaSpec {
            name: path.into(),
            path: path.into(),
            duration: Some(10 * S),
            video: Some(VideoInfo {
                width: 64,
                height: 64,
                fps: Rational { num: 30, den: 1 },
            }),
            audio: None,
        },
    }
}

fn clip(id: Id, start: Time, duration: Time, source_in: Time, source: ClipSource) -> Clip {
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
        effects: vec![],
        label: 0,
    }
}

fn track(id: Id, clips: Vec<Clip>) -> Track {
    Track {
        id,
        kind: TrackKind::Video,
        name: format!("V{id}"),
        locked: false,
        muted: false,
        solo: false,
        hidden: false,
        clips,
        transitions: vec![],
    }
}

/// Sequence 100: V (id 10) a.mp4 [0,4s) source_in 2 s; V (id 11) b.mp4 [1s,5s) source_in 0, plus a
/// solid on track 12 that must not be prefetched.
fn project() -> Project {
    Project {
        name: "p".into(),
        next_id: 1000,
        media: vec![media(1, "a.mp4"), media(2, "b.mp4")],
        sequences: vec![Sequence {
            id: 100,
            name: "S".into(),
            width: 64,
            height: 64,
            fps: Rational { num: 30, den: 1 },
            sample_rate: 48_000,
            tracks: vec![
                track(
                    10,
                    vec![clip(
                        20,
                        0,
                        ms(4000),
                        ms(2000),
                        ClipSource::Media { media_id: 1 },
                    )],
                ),
                track(
                    11,
                    vec![clip(
                        21,
                        ms(1000),
                        ms(4000),
                        0,
                        ClipSource::Media { media_id: 2 },
                    )],
                ),
                track(
                    12,
                    vec![clip(
                        22,
                        0,
                        ms(4000),
                        0,
                        ClipSource::Solid {
                            color: [1.0, 0.0, 0.0, 1.0],
                        },
                    )],
                ),
            ],
            markers: vec![],
        }],
        effects: vec![],
    }
}

#[test]
fn s6_prefetch_calls_backend_for_each_media_frame_in_order() {
    let (r, s) = rec();
    let p = project();
    // 3 s: a.mp4 at 2 s + 3 s = 5 s; b.mp4 at 0 + (3 s - 1 s) = 2 s; solid contributes nothing.
    s.prefetch(&p, 100, ms(3000));
    assert_eq!(
        *r.calls.lock().unwrap(),
        vec![
            ("a.mp4".to_string(), ms(5000)),
            ("b.mp4".to_string(), ms(2000))
        ]
    );
}

#[test]
fn s6_prefetch_only_clips_at_time_and_accumulates_across_calls() {
    let (r, s) = rec();
    let p = project();
    s.prefetch(&p, 100, ms(500)); // only a.mp4: 2 s + 0.5 s
    s.prefetch(&p, 100, ms(4500)); // only b.mp4: 3.5 s
    s.prefetch(&p, 100, ms(8000)); // nothing
    assert_eq!(
        *r.calls.lock().unwrap(),
        vec![
            ("a.mp4".to_string(), ms(2500)),
            ("b.mp4".to_string(), ms(3500))
        ]
    );
}

#[test]
fn s6_prefetch_unknown_sequence_does_nothing_and_does_not_panic() {
    let (r, s) = rec();
    s.prefetch(&project(), 999, ms(3000));
    assert!(r.calls.lock().unwrap().is_empty());
}

#[test]
fn s6_prefetch_works_without_the_project_being_open_in_the_session() {
    // The project is passed in; the session holds none. Must still work and not touch state.
    let (r, s) = rec();
    assert!(s.snapshot("nope").is_none());
    s.prefetch(&project(), 100, ms(1000));
    assert_eq!(
        *r.calls.lock().unwrap(),
        vec![("a.mp4".to_string(), ms(3000)), ("b.mp4".to_string(), 0)]
    );
}
