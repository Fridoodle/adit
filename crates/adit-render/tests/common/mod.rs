//! Shared fixtures for the adit-render tests: project builders, fake media backends, pixel and
//! sample asserts. Projects are built by filling the public structs directly (adit-core logic is
//! not used). Expected values in the tests are hand-computed from docs/SPEC.md.
#![allow(dead_code)]

use adit_core::*;
use adit_media::{Frame, MediaBackend, MediaError};
use adit_render::{RenderError, Renderer};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

/// Flicks per second.
pub const S: Time = 705_600_000;
/// One frame at 30 fps: S / 30.
pub const F30: Time = 23_520_000;
/// One sample frame at 48 kHz: S / 48000.
pub const SF: Time = 14_700;
pub const FPS30: Rational = Rational { num: 30, den: 1 };
pub const SR: u32 = 48_000;

pub const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];
pub const GREEN: [f32; 4] = [0.0, 1.0, 0.0, 1.0];
pub const BLUE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];
pub const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

/// Milliseconds to flicks (S / 1000 = 705_600, exact).
pub const fn ms(n: i64) -> Time {
    n * 705_600
}

// ---------------------------------------------------------------- renderer

static RENDERER: OnceLock<Mutex<Renderer>> = OnceLock::new();

/// One renderer per test binary; tests take turns on it (parallel-safe).
pub fn renderer() -> MutexGuard<'static, Renderer> {
    RENDERER
        .get_or_init(|| Mutex::new(Renderer::new().expect("Renderer::new")))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

pub fn try_render(
    p: &Project,
    seq: Id,
    t: Time,
    m: &dyn MediaBackend,
) -> Result<Frame, RenderError> {
    renderer().render_frame(p, seq, t, m)
}

pub fn render(p: &Project, seq: Id, t: Time, m: &dyn MediaBackend) -> Frame {
    match try_render(p, seq, t, m) {
        Ok(f) => f,
        Err(e) => panic!("render_frame(seq {seq}, t {t}) failed: {e:?}"),
    }
}

pub fn mix(p: &Project, seq: Id, start: Time, dur: Time, m: &dyn MediaBackend) -> Vec<f32> {
    match adit_render::mix_audio(p, seq, start, dur, m) {
        Ok(v) => v,
        Err(e) => panic!("mix_audio(seq {seq}) failed: {e:?}"),
    }
}

// ---------------------------------------------------------------- project builder

pub struct B {
    pub p: Project,
}

impl B {
    pub fn new() -> Self {
        B {
            p: Project {
                name: "test".into(),
                next_id: 1,
                media: vec![],
                sequences: vec![],
                effects: vec![],
            },
        }
    }

    pub fn id(&mut self) -> Id {
        let id = self.p.next_id;
        self.p.next_id += 1;
        id
    }

    /// 30 fps, 48 kHz.
    pub fn seq(&mut self, width: u32, height: u32) -> Id {
        let id = self.id();
        self.p.sequences.push(Sequence {
            id,
            name: format!("Seq{id}"),
            width,
            height,
            fps: FPS30,
            sample_rate: SR,
            tracks: vec![],
            markers: vec![],
        });
        id
    }

    pub fn seq_mut(&mut self, id: Id) -> &mut Sequence {
        self.p
            .sequences
            .iter_mut()
            .find(|s| s.id == id)
            .expect("sequence")
    }

    fn track(&mut self, seq: Id, kind: TrackKind) -> Id {
        let id = self.id();
        let s = self.seq_mut(seq);
        let name = format!("T{}", s.tracks.len());
        s.tracks.push(Track {
            id,
            kind,
            name,
            locked: false,
            muted: false,
            solo: false,
            hidden: false,
            clips: vec![],
            transitions: vec![],
        });
        id
    }

    pub fn vtrack(&mut self, seq: Id) -> Id {
        self.track(seq, TrackKind::Video)
    }

    pub fn atrack(&mut self, seq: Id) -> Id {
        self.track(seq, TrackKind::Audio)
    }

    pub fn track_mut(&mut self, id: Id) -> &mut Track {
        self.p
            .sequences
            .iter_mut()
            .flat_map(|s| s.tracks.iter_mut())
            .find(|t| t.id == id)
            .expect("track")
    }

    /// Media entry with a 1000 s duration. `size: None` = no video stream.
    pub fn media(&mut self, path: &str, size: Option<(u32, u32)>, audio: bool) -> Id {
        let id = self.id();
        self.p.media.push(Media {
            id,
            spec: MediaSpec {
                name: path.into(),
                path: path.into(),
                duration: Some(1000 * S),
                video: size.map(|(width, height)| VideoInfo {
                    width,
                    height,
                    fps: FPS30,
                }),
                audio: audio.then_some(AudioInfo {
                    sample_rate: SR,
                    channels: 2,
                }),
            },
        });
        id
    }

    pub fn clip(&mut self, track: Id, start: Time, duration: Time, source: ClipSource) -> Id {
        let id = self.id();
        let t = self.track_mut(track);
        t.clips.push(Clip {
            id,
            name: format!("c{id}"),
            start,
            duration,
            source,
            source_in: 0,
            speed: 1.0,
            reverse: false,
            enabled: true,
            link: None,
            group: None,
            blend: BlendMode::Normal,
            audio_filter: String::new(),
            params: BTreeMap::new(),
            effects: vec![],
        });
        t.clips.sort_by_key(|c| c.start);
        id
    }

    pub fn solid(&mut self, track: Id, start: Time, duration: Time, color: [f32; 4]) -> Id {
        self.clip(track, start, duration, ClipSource::Solid { color })
    }

    pub fn media_clip(&mut self, track: Id, start: Time, duration: Time, media_id: Id) -> Id {
        self.clip(track, start, duration, ClipSource::Media { media_id })
    }

    pub fn seq_clip(&mut self, track: Id, start: Time, duration: Time, sequence_id: Id) -> Id {
        self.clip(track, start, duration, ClipSource::Sequence { sequence_id })
    }

    pub fn adjustment(&mut self, track: Id, start: Time, duration: Time) -> Id {
        self.clip(track, start, duration, ClipSource::Adjustment)
    }

    pub fn clip_mut(&mut self, id: Id) -> &mut Clip {
        self.p
            .sequences
            .iter_mut()
            .flat_map(|s| s.tracks.iter_mut())
            .flat_map(|t| t.clips.iter_mut())
            .find(|c| c.id == id)
            .expect("clip")
    }

    /// Clip property without keyframes.
    pub fn set(&mut self, clip: Id, name: &str, value: f64) {
        self.clip_mut(clip).params.insert(name.into(), param(value));
    }

    pub fn keys(&mut self, clip: Id, name: &str, kfs: &[(Time, f64, Easing)]) {
        self.clip_mut(clip).params.insert(name.into(), keyed(kfs));
    }

    /// Appends an enabled effect instance; returns its id.
    pub fn effect(&mut self, clip: Id, name: &str, params: &[(&str, f64)]) -> Id {
        let id = self.id();
        self.clip_mut(clip).effects.push(EffectInstance {
            id,
            effect: name.into(),
            enabled: true,
            params: params
                .iter()
                .map(|(k, v)| (k.to_string(), param(*v)))
                .collect(),
        });
        id
    }

    pub fn effect_mut(&mut self, clip: Id, effect: Id) -> &mut EffectInstance {
        self.clip_mut(clip)
            .effects
            .iter_mut()
            .find(|e| e.id == effect)
            .expect("effect instance")
    }

    pub fn transition(
        &mut self,
        track: Id,
        left: Option<Id>,
        right: Option<Id>,
        effect: &str,
        duration: Time,
        params: &[(&str, f64)],
    ) -> Id {
        let id = self.id();
        self.track_mut(track).transitions.push(Transition {
            id,
            left,
            right,
            effect: effect.into(),
            duration,
            params: params.iter().map(|(k, v)| (k.to_string(), *v)).collect(),
        });
        id
    }

    pub fn def(&mut self, name: &str, kind: EffectKind, wgsl: &str, params: &[(&str, f64)]) {
        self.p.effects.push(def(name, kind, wgsl, params));
    }
}

pub fn param(value: f64) -> Param {
    Param {
        value,
        keyframes: vec![],
    }
}

pub fn keyed(kfs: &[(Time, f64, Easing)]) -> Param {
    Param {
        value: 0.0,
        keyframes: kfs
            .iter()
            .map(|&(time, value, easing)| Keyframe {
                time,
                value,
                easing,
            })
            .collect(),
    }
}

/// Effect definition with wide min/max so no test depends on clamping to the declared range.
pub fn def(name: &str, kind: EffectKind, wgsl: &str, params: &[(&str, f64)]) -> EffectDef {
    EffectDef {
        name: name.into(),
        kind,
        wgsl: wgsl.into(),
        params: params
            .iter()
            .map(|(n, d)| ParamDef {
                name: n.to_string(),
                default: *d,
                min: -1.0e6,
                max: 1.0e6,
            })
            .collect(),
    }
}

// ---------------------------------------------------------------- frames

pub fn frame_fn(w: u32, h: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Frame {
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            rgba.extend_from_slice(&f(x, y));
        }
    }
    Frame {
        width: w,
        height: h,
        rgba,
    }
}

pub fn solid_frame(w: u32, h: u32, c: [u8; 4]) -> Frame {
    frame_fn(w, h, |_, _| c)
}

/// Quadrants: top-left red, top-right green, bottom-left blue, bottom-right white.
pub fn quadrants(w: u32, h: u32) -> Frame {
    frame_fn(w, h, |x, y| match (x < w / 2, y < h / 2) {
        (true, true) => [255, 0, 0, 255],
        (false, true) => [0, 255, 0, 255],
        (true, false) => [0, 0, 255, 255],
        (false, false) => [255, 255, 255, 255],
    })
}

/// Channel `ch` = floor(t / F30) % 256 (the 30 fps frame index of the requested time).
pub fn time_frame(t: Time, ch: usize, w: u32, h: u32) -> Frame {
    let v = (t.div_euclid(F30) % 256) as u8;
    frame_fn(w, h, |_, _| {
        let mut c = [0, 0, 0, 255];
        c[ch] = v;
        c
    })
}

pub fn px(f: &Frame, x: u32, y: u32) -> [u8; 4] {
    assert!(x < f.width && y < f.height, "pixel ({x},{y}) outside {}x{}", f.width, f.height);
    let i = ((y * f.width + x) * 4) as usize;
    [f.rgba[i], f.rgba[i + 1], f.rgba[i + 2], f.rgba[i + 3]]
}

/// Top-level frames are always opaque (V-1 canvas starts opaque, V-7 keeps out.a = 1).
pub fn assert_px_tol(f: &Frame, x: u32, y: u32, rgb: [u8; 3], tol: u8, what: &str) {
    let p = px(f, x, y);
    let ok = (0..3).all(|i| p[i].abs_diff(rgb[i]) <= tol) && p[3] >= 253;
    assert!(
        ok,
        "{what}: pixel ({x},{y}) = {p:?}, expected rgb {rgb:?} ±{tol} with alpha 255"
    );
}

pub fn assert_px(f: &Frame, x: u32, y: u32, rgb: [u8; 3], what: &str) {
    assert_px_tol(f, x, y, rgb, 2, what);
}

pub fn assert_size(f: &Frame, w: u32, h: u32) {
    assert_eq!((f.width, f.height), (w, h), "frame size");
    assert_eq!(f.rgba.len(), (w * h * 4) as usize, "rgba length");
}

// ---------------------------------------------------------------- fake media

type VideoFn = dyn Fn(&str, Time) -> Result<Frame, MediaError> + Send + Sync;
type AudioFn = dyn Fn(&str, Time, Time, u32, &str) -> Result<Vec<f32>, MediaError> + Send + Sync;

#[derive(Debug, Clone, PartialEq)]
pub struct AudioCall {
    pub path: String,
    pub start: Time,
    pub duration: Time,
    pub sample_rate: u32,
    pub filter: String,
}

/// Test backend: closures for pictures and sound; records every call. Default: everything errors.
pub struct Fake {
    video: Box<VideoFn>,
    audio: Box<AudioFn>,
    video_calls: Mutex<Vec<(String, Time)>>,
    audio_calls: Mutex<Vec<AudioCall>>,
}

impl Fake {
    pub fn new() -> Self {
        Fake {
            video: Box::new(|p, _| Err(MediaError::NotFound(p.to_string()))),
            audio: Box::new(|p, _, _, _, _| Err(MediaError::NotFound(p.to_string()))),
            video_calls: Mutex::new(vec![]),
            audio_calls: Mutex::new(vec![]),
        }
    }

    pub fn with_video(
        mut self,
        f: impl Fn(&str, Time) -> Result<Frame, MediaError> + Send + Sync + 'static,
    ) -> Self {
        self.video = Box::new(f);
        self
    }

    pub fn with_audio(
        mut self,
        f: impl Fn(&str, Time, Time, u32, &str) -> Result<Vec<f32>, MediaError>
        + Send
        + Sync
        + 'static,
    ) -> Self {
        self.audio = Box::new(f);
        self
    }

    /// Fixed picture per path; other paths → NotFound.
    pub fn images(list: Vec<(&str, Frame)>) -> Self {
        let list: Vec<(String, Frame)> = list.into_iter().map(|(p, f)| (p.into(), f)).collect();
        Fake::new().with_video(move |path, _| {
            list.iter()
                .find(|(p, _)| p == path)
                .map(|(_, f)| f.clone())
                .ok_or_else(|| MediaError::NotFound(path.into()))
        })
    }

    pub fn video_calls(&self) -> Vec<(String, Time)> {
        self.video_calls
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn audio_calls(&self) -> Vec<AudioCall> {
        self.audio_calls
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

impl MediaBackend for Fake {
    fn probe(&self, path: &str) -> Result<MediaSpec, MediaError> {
        let f = (self.video)(path, 0)?;
        Ok(MediaSpec {
            name: path.into(),
            path: path.into(),
            duration: None,
            video: Some(VideoInfo {
                width: f.width,
                height: f.height,
                fps: FPS30,
            }),
            audio: Some(AudioInfo {
                sample_rate: SR,
                channels: 2,
            }),
        })
    }

    fn video_frame(&self, path: &str, time: Time) -> Result<Arc<Frame>, MediaError> {
        self.video_calls
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((path.into(), time));
        (self.video)(path, time).map(Arc::new)
    }

    fn audio(
        &self,
        path: &str,
        start: Time,
        duration: Time,
        sample_rate: u32,
        filter: &str,
    ) -> Result<Vec<f32>, MediaError> {
        self.audio_calls
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(AudioCall {
                path: path.into(),
                start,
                duration,
                sample_rate,
                filter: filter.into(),
            });
        (self.audio)(path, start, duration, sample_rate, filter)
    }
}

// ---------------------------------------------------------------- audio helpers

/// Sample frames a backend yields for `duration` (M-AUDIO-1 / M-PAT rule, used by the fakes only).
pub fn n_frames(duration: Time, sr: u32) -> usize {
    (duration as f64 * sr as f64 / S as f64).round().max(0.0) as usize
}

/// Product of every `atempo=x` step in a comma-joined chain (1 when there is none).
pub fn tempo_product(filter: &str) -> f64 {
    filter
        .split(',')
        .filter_map(|f| f.trim().strip_prefix("atempo="))
        .map(|v| v.parse::<f64>().expect("atempo value"))
        .product()
}

/// Constant L/R per path; length emulates atempo and is scaled by `len_factor`
/// (to test cut/pad). Unknown path → NotFound.
pub fn const_audio(levels: &[(&str, f32, f32)], len_factor: f64) -> Fake {
    let levels: Vec<(String, f32, f32)> =
        levels.iter().map(|(p, l, r)| (p.to_string(), *l, *r)).collect();
    Fake::new().with_audio(move |path, _start, duration, sr, filter| {
        let &(_, l, r) = levels
            .iter()
            .find(|x| x.0 == path)
            .ok_or_else(|| MediaError::NotFound(path.into()))?;
        let n = (n_frames(duration, sr) as f64 / tempo_product(filter) * len_factor).round();
        Ok([l, r].repeat(n as usize))
    })
}

/// `path`: L = (source sample frame index) / 100_000, R = 0, where the index of the first
/// returned frame is round(start * sr / S). Other paths: silence of the requested length.
pub fn ramp_audio(path: &'static str) -> Fake {
    Fake::new().with_audio(move |p, start, duration, sr, filter| {
        let n = (n_frames(duration, sr) as f64 / tempo_product(filter)).round() as usize;
        if p != path {
            return Ok(vec![0.0; 2 * n]);
        }
        let first = (start as f64 * sr as f64 / S as f64).round() as i64;
        Ok((0..n)
            .flat_map(|i| [(first + i as i64) as f32 / 100_000.0, 0.0])
            .collect())
    })
}

pub fn lr(buf: &[f32], frame: usize) -> (f32, f32) {
    assert!(2 * frame + 1 < buf.len(), "frame {frame} beyond {} samples", buf.len());
    (buf[2 * frame], buf[2 * frame + 1])
}

pub fn assert_lr(buf: &[f32], frame: usize, l: f32, r: f32, tol: f32, what: &str) {
    let (a, b) = lr(buf, frame);
    assert!(
        (a - l).abs() <= tol && (b - r).abs() <= tol,
        "{what}: frame {frame} = ({a}, {b}), expected ({l}, {r}) ±{tol}"
    );
}
