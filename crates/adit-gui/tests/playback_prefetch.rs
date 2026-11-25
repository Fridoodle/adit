//! U-APP-7, last paragraph: while playing forward a background thread calls `Session::prefetch`
//! for the sequence frames from the playhead up to 0.6 s ahead, so a cut is decoded early.
//!
//! Layout: V1 has clip A `[0, 9s)` of clipA.mov and clip B `[5s, 9s)` of clipB.mov (V2, on top)
//! with `source_in` 3 s. Playhead starts at 4.7 s, so B's first frame (source time 3 s) is 0.3 s
//! ahead, inside the 0.6 s look-ahead; its frames are source times 3 s .. 3.3 s at most, and
//! always >= 3 s. Timing asserts are bounds only, never counts.
mod app_common;
use adit_core::{MediaSpec, Time};
use adit_gui::Options;
use adit_media::{Frame, MediaBackend, MediaError, Pattern};
use app_common::*;
use serde_json::json;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct Rec {
    inner: Pattern,
    calls: Mutex<Vec<(String, Time)>>,
}

impl Rec {
    fn calls_for(&self, name: &str) -> Vec<Time> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|(p, _)| p.ends_with(name))
            .map(|(_, t)| *t)
            .collect()
    }
    fn total(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
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

struct Setup {
    rec: Arc<Rec>,
    s: adit_mcp::Session,
    h: egui_kittest::Harness<'static, adit_gui::App>,
    dir: std::path::PathBuf,
}

impl Drop for Setup {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn setup() -> Setup {
    let rec = Arc::new(Rec {
        inner: pattern(),
        calls: Mutex::new(vec![]),
    });
    let s = session_with(rec.clone());
    let mut h = harness(&s, Options::default());
    let dir = std::env::temp_dir().join(format!(
        "adit-gui-prefetch-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let (pid, _, v1) = active(&h, &s);
    let mut ids = vec![];
    for name in ["clipA.mov", "clipB.mov"] {
        let p = dir.join(name);
        std::fs::write(&p, b"pattern backend ignores this file").unwrap();
        let o = call(
            &s,
            "media_import",
            json!({ "project_id": pid, "paths": [p.to_str().unwrap()] }),
        )
        .json;
        ids.push(o["media"][0]["id"].as_u64().expect("media id"));
    }
    let v2 = snapshot(&s, &pid).sequences[0]
        .tracks
        .iter()
        .filter(|t| t.kind == adit_core::TrackKind::Video)
        .map(|t| t.id)
        .find(|id| *id != v1)
        .expect("default project has a second video track");
    edit(
        &s,
        &pid,
        json!([
            { "op": "add_clip", "track_id": v1, "source": { "type": "media", "media_id": ids[0] },
              "start": 0, "duration": "9s" },
            { "op": "add_clip", "track_id": v2, "source": { "type": "media", "media_id": ids[1] },
              "start": "5s", "duration": "4s", "source_in": "3s" }
        ]),
    );
    h.run_steps(3);
    // Playhead 0.3 s before the cut.
    call_stepping(&mut h, &s, "ui_set", json!({ "playhead": "141f" })).expect("ui_set playhead");
    h.run_steps(3);
    Setup { rec, s, h, dir }
}

fn pump(h: &mut egui_kittest::Harness<'static, adit_gui::App>, ms: u64) {
    let end = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < end {
        h.step();
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn u_app_7_no_prefetch_while_not_playing() {
    let mut t = setup();
    pump(&mut t.h, 800);
    assert_eq!(t.rec.total(), 0, "prefetch calls while stopped");
    let _ = &t.s;
}

#[test]
fn u_app_7_forward_playback_prefetches_the_cut_ahead_before_the_playhead_reaches_it() {
    let mut t = setup();
    assert_eq!(t.h.state().ui_state().unwrap().playhead, 141 * F30);
    t.h.state_mut().run_action(adit_gui::Action::PlayPause);
    let deadline = Instant::now() + Duration::from_secs(3);
    let (playhead, calls) = loop {
        t.h.step();
        let calls = t.rec.calls_for("clipB.mov");
        if !calls.is_empty() {
            break (t.h.state().ui_state().unwrap().playhead, calls);
        }
        assert!(
            Instant::now() < deadline,
            "no prefetch of clipB.mov within 3 s of forward playback"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    // B frames inside the look-ahead window have source times 3 s .. 3.6 s (cut at 5 s, window
    // end at most 4.7 s + 0.6 s of play time ahead of a playhead that has moved a little).
    for c in &calls {
        assert!(
            (3 * S..=3 * S + 2 * S).contains(c),
            "prefetched clipB.mov source time {c} outside [3 s, 5 s]"
        );
    }
    assert!(
        calls.iter().any(|c| *c < 3 * S + 18 * F30),
        "an early frame of B (source time < 3.6 s) was prefetched: {calls:?}"
    );
    assert!(
        playhead < 5 * S,
        "playhead {playhead} had already reached the cut when the prefetch was first seen"
    );
    t.h.state_mut().run_action(adit_gui::Action::PlayPause);
    pump(&mut t.h, 100);
}

#[test]
fn u_app_7_prefetch_stops_after_pause() {
    let mut t = setup();
    t.h.state_mut().run_action(adit_gui::Action::PlayPause);
    let deadline = Instant::now() + Duration::from_secs(3);
    while t.rec.total() == 0 {
        t.h.step();
        assert!(Instant::now() < deadline, "no prefetch while playing");
        std::thread::sleep(Duration::from_millis(5));
    }
    t.h.state_mut().run_action(adit_gui::Action::PlayPause);
    assert!(!t.h.state().ui_state().unwrap().playing);
    pump(&mut t.h, 400); // let an in-flight batch finish
    let n = t.rec.total();
    pump(&mut t.h, 600);
    assert_eq!(t.rec.total(), n, "prefetch calls continued after pause");
}
