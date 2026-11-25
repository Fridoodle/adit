//! SPEC X-1..X-4: `export`. Uses the real adit-media encoder (ffmpeg); results are checked by
//! running the ffprobe / ffmpeg binaries directly.

mod common;
use adit_core::*;
use adit_media::{AudioCodec, ExportSettings, VideoCodec};
use adit_render::{RenderError, export};
use common::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

fn out_path(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("adit-render-tests");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let p = dir.join(format!("{}-{name}", std::process::id()));
    let _ = std::fs::remove_file(&p);
    p
}

fn settings(path: &Path, w: u32, h: u32, fps: Rational, audio: AudioCodec) -> ExportSettings {
    ExportSettings {
        path: path.to_string_lossy().into_owned(),
        width: w,
        height: h,
        fps,
        sample_rate: SR,
        video: VideoCodec::H264,
        audio,
        crf: Some(10),
    }
}

fn run_export(
    p: &Project,
    seq: Id,
    st: &ExportSettings,
    m: &Fake,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(u64, u64),
) -> Result<(), RenderError> {
    let mut r = renderer();
    export(&mut r, p, seq, st, m, on_progress, cancel)
}

fn tool(name: &str, args: &[&str]) -> String {
    let out = Command::new(name)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("run {name}: {e}"));
    assert!(
        out.status.success(),
        "{name} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// (width, height, r_frame_rate, counted frames) of the first video stream.
fn probe_video(path: &Path) -> (u32, u32, String, u64) {
    let p = path.to_string_lossy();
    let out = tool(
        "ffprobe",
        &[
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-count_frames",
            "-show_entries",
            "stream=width,height,r_frame_rate,nb_read_frames",
            "-of",
            "default=noprint_wrappers=1",
            &p,
        ],
    );
    let get = |k: &str| {
        out.lines()
            .find_map(|l| l.trim().strip_prefix(&format!("{k}=")).map(str::to_string))
            .unwrap_or_else(|| panic!("ffprobe lacks {k}: {out}"))
    };
    (
        get("width").parse().unwrap(),
        get("height").parse().unwrap(),
        get("r_frame_rate"),
        get("nb_read_frames").parse().unwrap(),
    )
}

/// Sample rates of all audio streams (empty = no audio stream).
fn probe_audio(path: &Path) -> Vec<u32> {
    let p = path.to_string_lossy();
    let out = tool(
        "ffprobe",
        &[
            "-v",
            "error",
            "-select_streams",
            "a",
            "-show_entries",
            "stream=sample_rate",
            "-of",
            "csv=p=0",
            &p,
        ],
    );
    out.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.trim().parse().unwrap())
        .collect()
}

/// All frames decoded to RGBA.
fn decode(path: &Path, w: u32, h: u32) -> Vec<Vec<u8>> {
    let p = path.to_string_lossy();
    let out = Command::new("ffmpeg")
        .args([
            "-v", "error", "-i", &p, "-f", "rawvideo", "-pix_fmt", "rgba", "-",
        ])
        .output()
        .expect("run ffmpeg");
    assert!(
        out.status.success(),
        "ffmpeg decode failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
        .chunks((w * h * 4) as usize)
        .map(|c| c.to_vec())
        .collect()
}

fn assert_rgb(frame: &[u8], w: u32, x: u32, y: u32, rgb: [u8; 3], tol: u8, what: &str) {
    let i = ((y * w + x) * 4) as usize;
    let p = &frame[i..i + 3];
    assert!(
        (0..3).all(|k| p[k].abs_diff(rgb[k]) <= tol),
        "{what}: ({x},{y}) = {p:?}, expected {rgb:?} ±{tol}"
    );
}

/// 64x64 30 fps: red [0, .5 s), blue [.5 s, 1 s); optional audio track with const "a".
fn red_blue(with_audio: bool) -> (Project, Id) {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v = b.vtrack(s);
    b.solid(v, 0, ms(500), RED);
    b.solid(v, ms(500), ms(500), BLUE);
    if with_audio {
        let a = b.atrack(s);
        let m = b.media("a", None, true);
        b.media_clip(a, 0, S, m);
    }
    (b.p, s)
}

fn audio_fake() -> Fake {
    const_audio(&[("a", 0.5, 0.25)], 1.0)
}

// ------------------------------------------------------------------ X-1

#[test]
fn x1_frame_count_for_non_integer_duration() {
    // Duration .52 s = 366_912_000; frames n with n * 23_520_000 < 366_912_000 → n <= 15 → 16.
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v = b.vtrack(s);
    b.solid(v, 0, ms(520), RED);
    let path = out_path("x1_count.mp4");
    let st = settings(&path, 64, 64, FPS30, AudioCodec::None);
    let mut total = 0;
    run_export(
        &b.p,
        s,
        &st,
        &Fake::new(),
        &AtomicBool::new(false),
        &mut |_, n| total = n,
    )
    .expect("export");
    assert_eq!(total, 16, "X-1 N = 16");
    let (w, h, _, n) = probe_video(&path);
    assert_eq!((w, h, n), (64, 64, 16), "X-1 file has 16 frames of 64x64");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn x1_scaled_retimed_with_resampled_audio() {
    let (p, s) = red_blue(true);
    let path = out_path("x1_scaled.mp4");
    let mut st = settings(&path, 32, 32, Rational { num: 10, den: 1 }, AudioCodec::Aac);
    st.sample_rate = 44_100;
    let mut total = 0;
    run_export(
        &p,
        s,
        &st,
        &audio_fake(),
        &AtomicBool::new(false),
        &mut |_, n| total = n,
    )
    .expect("export");
    // 10 fps over 1 s: n * 70_560_000 < S → n = 0..9 → 10 frames.
    assert_eq!(total, 10, "X-1 N at export fps");
    let (w, h, rate, n) = probe_video(&path);
    assert_eq!(
        (w, h, rate.as_str(), n),
        (32, 32, "10/1", 10),
        "X-1 scaled to settings"
    );
    assert_eq!(
        probe_audio(&path),
        vec![44_100],
        "X-1 one audio stream at settings.sample_rate"
    );
    let frames = decode(&path, 32, 32);
    assert_eq!(frames.len(), 10);
    // Frame 4 = render at .4 s (red), frame 5 = .5 s (blue).
    assert_rgb(&frames[4], 32, 16, 16, [255, 0, 0], 12, "X-1 frame 4 red");
    assert_rgb(&frames[5], 32, 16, 16, [0, 0, 255], 12, "X-1 frame 5 blue");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn x1_audio_codec_none_means_no_audio_stream() {
    let (p, s) = red_blue(true);
    let path = out_path("x1_noaudio.mp4");
    let st = settings(&path, 64, 64, FPS30, AudioCodec::None);
    run_export(
        &p,
        s,
        &st,
        &audio_fake(),
        &AtomicBool::new(false),
        &mut |_, _| {},
    )
    .expect("export");
    assert_eq!(
        probe_audio(&path),
        Vec::<u32>::new(),
        "X-1 AudioCodec::None → no audio stream"
    );
    let _ = std::fs::remove_file(&path);
}

// ------------------------------------------------------------------ X-2

#[test]
fn x2_progress_after_every_frame() {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v = b.vtrack(s);
    b.solid(v, 0, 10 * F30, GREEN); // exactly 10 frames
    let path = out_path("x2_progress.mp4");
    let st = settings(&path, 64, 64, FPS30, AudioCodec::None);
    let mut seen = vec![];
    run_export(
        &b.p,
        s,
        &st,
        &Fake::new(),
        &AtomicBool::new(false),
        &mut |d, n| seen.push((d, n)),
    )
    .expect("export");
    let want: Vec<(u64, u64)> = (1..=10).map(|d| (d, 10)).collect();
    assert_eq!(seen, want, "X-2 progress (1,10) .. (10,10)");
    assert!(path.exists(), "X-2 file written");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn x2_cancel_mid_export_removes_partial_file() {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v = b.vtrack(s);
    b.solid(v, 0, 2 * S, GREEN); // 60 frames
    let path = out_path("x2_cancel.mp4");
    let st = settings(&path, 64, 64, FPS30, AudioCodec::None);
    let cancel = AtomicBool::new(false);
    let mut seen = vec![];
    let r = run_export(&b.p, s, &st, &Fake::new(), &cancel, &mut |d, n| {
        seen.push((d, n));
        if d == 3 {
            cancel.store(true, Ordering::SeqCst);
        }
    });
    assert_eq!(r, Err(RenderError::Cancelled), "X-2 cancel → Cancelled");
    let last = seen.last().map(|x| x.0).unwrap_or(0);
    assert!(
        (3..=4).contains(&last),
        "X-2 stops within one frame of cancel: last progress {last}"
    );
    assert!(!path.exists(), "X-2 partial file removed");
}

#[test]
fn x2_cancel_before_start() {
    let (p, s) = red_blue(false);
    let path = out_path("x2_precancel.mp4");
    let st = settings(&path, 64, 64, FPS30, AudioCodec::None);
    let mut calls = 0;
    let r = run_export(
        &p,
        s,
        &st,
        &Fake::new(),
        &AtomicBool::new(true),
        &mut |_, _| calls += 1,
    );
    assert_eq!(r, Err(RenderError::Cancelled));
    assert!(calls <= 1, "X-2 stops within one frame");
    assert!(!path.exists(), "X-2 no file left");
}

// ------------------------------------------------------------------ X-3

#[test]
fn x3_empty_sequence_is_error_without_file() {
    let mut b = B::new();
    let s = b.seq(64, 64);
    b.vtrack(s);
    b.atrack(s);
    let path = out_path("x3_empty.mp4");
    let st = settings(&path, 64, 64, FPS30, AudioCodec::Aac);
    let r = run_export(
        &b.p,
        s,
        &st,
        &audio_fake(),
        &AtomicBool::new(false),
        &mut |_, _| {},
    );
    assert!(r.is_err(), "X-3 empty sequence → Err");
    assert!(!path.exists(), "X-3 no file left behind");
}

#[test]
fn x_missing_sequence_is_error() {
    let (p, _) = red_blue(false);
    let path = out_path("x_missing.mp4");
    let st = settings(&path, 64, 64, FPS30, AudioCodec::None);
    let r = run_export(
        &p,
        777,
        &st,
        &Fake::new(),
        &AtomicBool::new(false),
        &mut |_, _| {},
    );
    assert_eq!(r, Err(RenderError::SequenceNotFound(777)));
    assert!(!path.exists());
}

// ------------------------------------------------------------------ X-4

#[test]
fn x4_end_to_end_red_blue_h264() {
    let (p, s) = red_blue(false);
    let before = p.clone();
    let path = out_path("x4_e2e.mp4");
    // Audio requested but the sequence has no audio tracks → no audio stream (X-1).
    let st = settings(&path, 64, 64, FPS30, AudioCodec::Aac);
    let mut seen = vec![];
    run_export(
        &p,
        s,
        &st,
        &Fake::new(),
        &AtomicBool::new(false),
        &mut |d, n| seen.push((d, n)),
    )
    .expect("export");
    assert_eq!(seen.last(), Some(&(30, 30)), "X-2 ends with (N, N)");
    let (w, h, rate, n) = probe_video(&path);
    assert_eq!((w, h, rate.as_str(), n), (64, 64, "30/1", 30), "X-4 probe");
    assert_eq!(
        probe_audio(&path),
        Vec::<u32>::new(),
        "X-1 no audio tracks → no audio stream"
    );
    let frames = decode(&path, 64, 64);
    assert_eq!(frames.len(), 30);
    for (x, y) in [(32, 32), (5, 5), (60, 60)] {
        assert_rgb(&frames[5], 64, x, y, [255, 0, 0], 12, "X-4 frame 5 red");
        assert_rgb(&frames[20], 64, x, y, [0, 0, 255], 12, "X-4 frame 20 blue");
    }
    assert_eq!(p, before, "export does not modify the project");
    let _ = std::fs::remove_file(&path);
}
