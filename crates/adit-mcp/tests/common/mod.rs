//! Shared helpers for the in-process `Session` tests. Contract: docs/MCP_TOOLS.md.
//! Every expected value in the tests is worked out by hand from the contract, never read back
//! from the code under test.
#![allow(dead_code)]

use adit_core::Rational;
use adit_mcp::{Session, ToolOutput};
use adit_media::Pattern;
use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

/// Flicks per second.
pub const S: i64 = 705_600_000;
/// One frame at 30 fps: 705_600_000 / 30.
pub const F30: i64 = 23_520_000;

/// The backend every in-process test uses: 64x64, 30 fps, 10 s (SPEC M-PAT).
pub fn session() -> Session {
    Session::new(Arc::new(Pattern {
        width: 64,
        height: 64,
        fps: Rational { num: 30, den: 1 },
        duration: Some(10 * S),
    }))
}

pub fn call(s: &Session, tool: &str, args: Value) -> ToolOutput {
    match s.call(tool, args.clone()) {
        Ok(o) => o,
        Err(e) => panic!("{tool} {args} failed: {e}"),
    }
}

pub fn call_err(s: &Session, tool: &str, args: Value) -> String {
    match s.call(tool, args.clone()) {
        Ok(o) => panic!("{tool} {args} should fail, got {}", o.json),
        Err(e) => {
            assert!(
                !e.trim().is_empty(),
                "{tool}: error message must not be empty"
            );
            e
        }
    }
}

pub struct Proj {
    pub id: String,
    pub seq: u64,
    /// In creation order: V1, V2, ..., A1, A2, ... (SPEC C-SEQ-1).
    pub tracks: Vec<u64>,
}

impl Proj {
    pub fn v1(&self) -> u64 {
        self.tracks[0]
    }
}

/// `project_new` with `name: "t"`, 64x48, 30 fps, 2+2 tracks; `extra` overrides fields.
pub fn new_project(s: &Session, extra: Value) -> Proj {
    let mut args = json!({ "name": "t", "width": 64, "height": 48 });
    for (k, v) in extra.as_object().expect("extra must be an object") {
        args[k] = v.clone();
    }
    let o = call(s, "project_new", args).json;
    Proj {
        id: o["project_id"]
            .as_str()
            .expect("project_id is a string")
            .to_string(),
        seq: o["sequence_id"].as_u64().expect("sequence_id"),
        tracks: o["track_ids"]
            .as_array()
            .expect("track_ids")
            .iter()
            .map(|v| v.as_u64().unwrap())
            .collect(),
    }
}

/// `project_get` detail full.
pub fn full(s: &Session, pid: &str) -> Value {
    call(
        s,
        "project_get",
        json!({ "project_id": pid, "detail": "full" }),
    )
    .json
}

pub fn project(s: &Session, pid: &str) -> Value {
    full(s, pid)["project"].clone()
}

pub fn summary(s: &Session, pid: &str) -> Value {
    call(s, "project_get", json!({ "project_id": pid })).json
}

pub fn depths(s: &Session, pid: &str) -> (u64, u64) {
    let f = full(s, pid);
    (
        f["undo_depth"].as_u64().unwrap(),
        f["redo_depth"].as_u64().unwrap(),
    )
}

pub fn next_id(s: &Session, pid: &str) -> u64 {
    project(s, pid)["next_id"].as_u64().unwrap()
}

/// `edit` that must succeed; returns the result JSON.
pub fn edit(s: &Session, pid: &str, commands: Value) -> Value {
    call(
        s,
        "edit",
        json!({ "project_id": pid, "commands": commands }),
    )
    .json
}

pub fn edit_err(s: &Session, pid: &str, commands: Value) -> String {
    call_err(
        s,
        "edit",
        json!({ "project_id": pid, "commands": commands }),
    )
}

pub fn solid(rgba: [f64; 4]) -> Value {
    json!({ "type": "solid", "color": rgba })
}

pub fn media_src(id: u64) -> Value {
    json!({ "type": "media", "media_id": id })
}

pub fn text_src(content: &str) -> Value {
    json!({ "type": "text", "text": {
        "content": content, "font": "", "size": 24.0, "color": [1.0, 1.0, 1.0, 1.0],
        "align": "center", "bold": false, "italic": false } })
}

/// Adds a solid clip and returns its id (`created[0][0]`).
pub fn add_solid(
    s: &Session,
    pid: &str,
    track: u64,
    start: Value,
    dur: Value,
    rgba: [f64; 4],
) -> u64 {
    let r = edit(
        s,
        pid,
        json!([{ "op": "add_clip", "track_id": track, "source": solid(rgba),
                 "start": start, "duration": dur }]),
    );
    r["created"][0][0].as_u64().expect("created[0][0]")
}

/// Finds a clip by id in a full `adit_core::Project` JSON.
pub fn find_clip(project: &Value, id: u64) -> Value {
    for seq in project["sequences"].as_array().unwrap() {
        for tr in seq["tracks"].as_array().unwrap() {
            for c in tr["clips"].as_array().unwrap() {
                if c["id"].as_u64() == Some(id) {
                    return c.clone();
                }
            }
        }
    }
    panic!("clip {id} not in project {project}");
}

pub fn find_track(project: &Value, id: u64) -> Value {
    for seq in project["sequences"].as_array().unwrap() {
        for tr in seq["tracks"].as_array().unwrap() {
            if tr["id"].as_u64() == Some(id) {
                return tr.clone();
            }
        }
    }
    panic!("track {id} not in project {project}");
}

/// Imports one fake file (Pattern ignores the content) and returns the media id.
pub fn import_pattern(s: &Session, pid: &str, dir: &Path, name: &str) -> u64 {
    let p = dir.join(name);
    std::fs::write(&p, b"pattern backend ignores this file").unwrap();
    let r = call(
        s,
        "media_import",
        json!({ "project_id": pid, "paths": [p.to_str().unwrap()] }),
    )
    .json;
    r["media"][0]["id"].as_u64().expect("media[0].id")
}

pub fn decode(bytes: &[u8]) -> image::RgbaImage {
    image::load_from_memory(bytes)
        .expect("image decodes")
        .to_rgba8()
}

/// `render_frame` as PNG at `max_size` 640; returns the decoded image and the text JSON.
pub fn frame_png(s: &Session, pid: &str, time: Value) -> (image::RgbaImage, Value) {
    let o = call(
        s,
        "render_frame",
        json!({ "project_id": pid, "time": time, "format": "png" }),
    );
    assert_eq!(o.images.len(), 1, "render_frame returns one image");
    assert_eq!(o.images[0].mime, "image/png");
    (decode(&o.images[0].bytes), o.json)
}

pub fn assert_px(img: &image::RgbaImage, x: u32, y: u32, rgb: [u8; 3], tol: i32, what: &str) {
    let p = img.get_pixel(x, y);
    for c in 0..3 {
        let d = (p[c] as i32 - rgb[c] as i32).abs();
        assert!(
            d <= tol,
            "{what}: pixel ({x},{y}) = {:?}, expected {rgb:?} ±{tol}",
            p.0
        );
    }
}

pub fn approx(a: f64, b: f64, tol: f64, what: &str) {
    assert!((a - b).abs() <= tol, "{what}: got {a}, expected {b} ±{tol}");
}

pub fn f(v: &Value) -> f64 {
    v.as_f64().unwrap_or_else(|| panic!("not a number: {v}"))
}

/// `ffprobe` of the first video stream: (nb_read_frames, width, height, r_frame_rate).
pub fn probe_video(path: &Path) -> (u64, u32, u32, String) {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-count_frames",
            "-show_entries",
            "stream=nb_read_frames,width,height,r_frame_rate",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .expect("ffprobe runs");
    assert!(
        out.status.success(),
        "ffprobe failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let st = &v["streams"][0];
    (
        st["nb_read_frames"].as_str().unwrap().parse().unwrap(),
        st["width"].as_u64().unwrap() as u32,
        st["height"].as_u64().unwrap() as u32,
        st["r_frame_rate"].as_str().unwrap().to_string(),
    )
}

/// Decodes video frame `n` of a file with ffmpeg to RGB24.
pub fn ffmpeg_frame(path: &Path, n: u32, w: u32, h: u32) -> Vec<u8> {
    let out = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args([
            "-vf",
            &format!("select=eq(n\\,{n})"),
            "-vframes",
            "1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-",
        ])
        .output()
        .expect("ffmpeg runs");
    assert!(
        out.status.success(),
        "ffmpeg decode failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(out.stdout.len() as u32, w * h * 3, "decoded frame size");
    out.stdout
}

pub fn rgb_at(buf: &[u8], w: u32, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * w + x) * 3) as usize;
    [buf[i], buf[i + 1], buf[i + 2]]
}

pub fn assert_rgb(got: [u8; 3], want: [u8; 3], tol: i32, what: &str) {
    for c in 0..3 {
        assert!(
            (got[c] as i32 - want[c] as i32).abs() <= tol,
            "{what}: got {got:?}, expected {want:?} ±{tol}"
        );
    }
}

/// Same file system object (paths may come back canonicalised or not).
pub fn same_path(a: &str, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => Path::new(a) == b,
    }
}

/// The E-5 example filter.
pub const SWAP_WGSL: &str = "fn effect(uv: vec2f) -> vec4f { let c = src(uv); \
    return vec4f(c.b, c.g, c.r, c.a) * vec4f(p.gain, p.gain, p.gain, 1.0); }";

pub fn swap_def(name: &str) -> Value {
    json!({ "name": name, "kind": "filter", "wgsl": SWAP_WGSL,
            "params": [{ "name": "gain", "default": 1.0, "min": 0.0, "max": 2.0 }] })
}

/// Requests list may come back as an array or as `{ requests: [...] }` (contract leaves it open).
pub fn request_list(s: &Session, args: Value) -> Vec<Value> {
    let j = call(s, "request_list", args).json;
    if let Some(a) = j.as_array() {
        return a.clone();
    }
    j["requests"]
        .as_array()
        .expect("request_list: array or {requests:[..]}")
        .clone()
}
