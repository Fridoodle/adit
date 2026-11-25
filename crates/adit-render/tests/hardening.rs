//! SPEC Part 5 hardening rules for adit-render: H-1 (shader size and nesting limits), H-2
//! (`override` rejected), H-5 (export longer than 4 hours), H-12 (export never destroys an
//! existing file). Process-killing inputs live in `hardening_crash.rs`.
mod common;
use adit_core::*;
use adit_media::{AudioCodec, ExportSettings, VideoCodec};
use adit_render::{RenderError, Renderer, export, validate_effect};
use common::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::Duration;

fn filter(wgsl: &str) -> EffectDef {
    def("h", EffectKind::Filter, wgsl, &[])
}

// ------------------------------------------------------------------------------------- H-1

const BODY: &str = "fn effect(uv: vec2f) -> vec4f { return src(uv); }\n";

/// A valid filter of exactly `len` bytes: BODY plus a comment line padded with `x`.
fn padded(len: usize) -> String {
    let mut s = String::from(BODY);
    s.push_str("//");
    while s.len() < len {
        s.push('x');
    }
    assert_eq!(s.len(), len);
    s
}

#[test]
fn h_1_wgsl_of_65536_bytes_ok() {
    assert_eq!(validate_effect(&filter(&padded(65_536))), Ok(()));
}

#[test]
fn h_1_wgsl_of_65537_bytes_rejected() {
    assert!(validate_effect(&filter(&padded(65_537))).is_err());
}

/// Module-scope constant nested `depth` parentheses deep; nothing else nests deeper than 2.
fn parens(depth: usize) -> String {
    format!(
        "const K: f32 = {}1.0{};\nfn effect(uv: vec2f) -> vec4f {{ return src(uv) * K; }}\n",
        "(".repeat(depth),
        ")".repeat(depth)
    )
}

/// Module-scope constant indexing `depth` brackets deep (`A[A[...A[0]...]]`, all zero).
fn brackets(depth: usize) -> String {
    format!(
        "const A = array<i32, 1>(0);\nconst K = {}0{};\n\
         fn effect(uv: vec2f) -> vec4f {{ return src(uv) * f32(K + 1); }}\n",
        "A[".repeat(depth),
        "]".repeat(depth)
    )
}

#[test]
fn h_1_paren_depth_64_ok() {
    assert_eq!(validate_effect(&filter(&parens(64))), Ok(()));
}

#[test]
fn h_1_paren_depth_65_rejected() {
    assert!(validate_effect(&filter(&parens(65))).is_err());
}

#[test]
fn h_1_bracket_depth_64_ok() {
    assert_eq!(validate_effect(&filter(&brackets(64))), Ok(()));
}

#[test]
fn h_1_bracket_depth_65_rejected() {
    assert!(validate_effect(&filter(&brackets(65))).is_err());
}

// Brace depth 64 / 65 overflow the stack today: see hardening_crash.rs.

/// `depth` nested parentheses inside the effect.
fn deep_parens(depth: usize) -> String {
    format!(
        "fn effect(uv: vec2f) -> vec4f {{ return src(uv) * {}1.0{}; }}\n",
        "(".repeat(depth),
        ")".repeat(depth)
    )
}

#[test]
fn h_1_500_nested_parens_rejected() {
    assert!(validate_effect(&filter(&deep_parens(500))).is_err());
}

#[test]
fn h_1_100000_nested_parens_rejected() {
    assert!(validate_effect(&filter(&deep_parens(100_000))).is_err());
}

#[test]
fn h_1_30000_nested_braces_rejected() {
    // About 60 KB: under the size limit, so only the depth limit has to reject it.
    let w = format!(
        "fn effect(uv: vec2f) -> vec4f {{ {}{} return src(uv); }}",
        "{".repeat(29_999),
        "}".repeat(29_999)
    );
    assert!(w.len() <= 65_536);
    assert!(validate_effect(&filter(&w)).is_err());
}

// ------------------------------------------------------------------------------------- H-2

#[test]
fn h_2_override_without_default_rejected() {
    let w = "override k: f32;\nfn effect(uv: vec2f) -> vec4f { return src(uv) * k; }";
    assert!(validate_effect(&filter(w)).is_err());
}

#[test]
fn h_2_override_with_default_rejected() {
    let w = "override k: f32 = 1.0;\nfn effect(uv: vec2f) -> vec4f { return src(uv) * k; }";
    assert!(validate_effect(&filter(w)).is_err());
}

// ------------------------------------------------------------------------------------- export helpers

static DIRS: AtomicU64 = AtomicU64::new(0);

/// Unique empty directory per test; removed on drop.
struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Self {
        let n = DIRS.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!(
            "adit-render-hardening-{tag}-{}-{n}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        Dir(p)
    }
    /// Names of everything in the directory, sorted.
    fn names(&self) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(&self.0)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        v.sort();
        v
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        for _ in 0..20 {
            if std::fs::remove_dir_all(&self.0).is_ok() || !self.0.exists() {
                return;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

fn settings(path: &Path) -> ExportSettings {
    ExportSettings {
        path: path.to_string_lossy().into_owned(),
        width: 64,
        height: 64,
        fps: FPS30,
        sample_rate: SR,
        video: VideoCodec::H264,
        audio: AudioCodec::None,
        crf: Some(10),
    }
}

/// Runs `f` on a thread; a hang longer than `secs` fails the test.
fn timed<T: Send + 'static>(secs: u64, f: impl FnOnce() -> T + Send + 'static) -> T {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        let _ = tx.send(r);
    });
    match rx.recv_timeout(Duration::from_secs(secs)) {
        Ok(Ok(v)) => v,
        Ok(Err(p)) => std::panic::resume_unwind(p),
        Err(_) => panic!("test hung: no result after {secs} s"),
    }
}

/// 64x64 30 fps sequence with one red solid of `duration`; returns (project, sequence).
fn red(duration: Time) -> (Project, Id) {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v = b.vtrack(s);
    b.solid(v, 0, duration, RED);
    (b.p, s)
}

const PRECIOUS: &[u8] = b"an existing file that must survive";

// ------------------------------------------------------------------------------------- H-5

/// One audio track with a 1 s clip of constant (0.5, 0.25) audio. (Above-MAX_TIME and
/// i64::MAX durations abort today: see hardening_crash.rs.)
fn audio_project() -> (Project, Id, Fake) {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let a = b.atrack(s);
    let m = b.media("a.wav", None, true);
    b.media_clip(a, 0, S, m);
    (b.p, s, const_audio(&[("a.wav", 0.5, 0.25)], 1.0))
}

#[test]
fn h_5_mix_audio_negative_duration_rejected() {
    let (p, s, m) = audio_project();
    let r = adit_render::mix_audio(&p, s, 0, -1, &m);
    assert!(r.is_err(), "H-5: negative duration must be Err, got {r:?}");
}

#[test]
fn h_5_mix_audio_ordinary_duration_still_works() {
    let (p, s, m) = audio_project();
    // A-1: 2 * round(0.5 s * 48000) = 48000 samples of (0.5, 0.25).
    let v = adit_render::mix_audio(&p, s, 0, S / 2, &m).unwrap();
    assert_eq!(v.len(), 48_000);
    assert_lr(&v, 100, 0.5, 0.25, 1e-6, "mix");
}

#[test]
fn h_5_export_longer_than_4_hours_rejected() {
    timed(60, || {
        let d = Dir::new("h5-long");
        let out = d.0.join("long.mp4");
        // 4 h + one 30 fps frame.
        let (p, s) = red(4 * 3600 * S + F30);
        // Own renderer: if this hangs, the shared one stays free for the other tests.
        let mut r = Renderer::new().expect("Renderer::new");
        let res = export(
            &mut r,
            &p,
            s,
            &settings(&out),
            &Fake::new(),
            &mut |_, _| {},
            &AtomicBool::new(false),
        );
        assert!(res.is_err(), "H-5: export of > 4 h must fail");
        assert_eq!(d.names(), Vec::<String>::new(), "no file left behind");
    })
}

// ------------------------------------------------------------------------------------- H-12

#[test]
fn h_12_failing_export_keeps_existing_file() {
    timed(120, || {
        let d = Dir::new("h12-fail");
        let out = d.0.join("out.mp4");
        std::fs::write(&out, PRECIOUS).unwrap();
        let (mut p, s) = red(S);
        // The unknown effect makes render_frame fail (V-4).
        p.sequences[0].tracks[0].clips[0]
            .effects
            .push(EffectInstance {
                id: p.next_id,
                effect: "no_such_effect".into(),
                enabled: true,
                params: Default::default(),
            });
        p.next_id += 1;
        let res = {
            let mut r = renderer();
            export(
                &mut r,
                &p,
                s,
                &settings(&out),
                &Fake::new(),
                &mut |_, _| {},
                &AtomicBool::new(false),
            )
        };
        assert!(
            matches!(res, Err(RenderError::UnknownEffect(_))),
            "expected UnknownEffect, got {res:?}"
        );
        let now = std::fs::read(&out).expect("H-12: the existing file was deleted");
        assert_eq!(now, PRECIOUS, "H-12: file changed");
        assert_eq!(d.names(), vec!["out.mp4".to_string()], "temp file left");
    })
}

#[test]
fn h_12_cancelled_export_keeps_existing_file() {
    timed(120, || {
        let d = Dir::new("h12-cancel");
        let out = d.0.join("out.mp4");
        std::fs::write(&out, PRECIOUS).unwrap();
        let (p, s) = red(2 * S);
        let cancel = AtomicBool::new(false);
        let res = {
            let mut r = renderer();
            // Cancel after the third frame: the encoder has already started.
            export(
                &mut r,
                &p,
                s,
                &settings(&out),
                &Fake::new(),
                &mut |done, _| {
                    if done >= 3 {
                        cancel.store(true, Ordering::SeqCst);
                    }
                },
                &cancel,
            )
        };
        assert_eq!(res, Err(RenderError::Cancelled));
        let now = std::fs::read(&out).expect("H-12: the existing file was deleted");
        assert_eq!(now, PRECIOUS, "H-12: file changed");
        assert_eq!(d.names(), vec!["out.mp4".to_string()], "temp file left");
    })
}

#[test]
fn h_12_successful_export_replaces_existing_file() {
    timed(120, || {
        let d = Dir::new("h12-ok");
        let out = d.0.join("out.mp4");
        std::fs::write(&out, PRECIOUS).unwrap();
        let (p, s) = red(S);
        let res = {
            let mut r = renderer();
            export(
                &mut r,
                &p,
                s,
                &settings(&out),
                &Fake::new(),
                &mut |_, _| {},
                &AtomicBool::new(false),
            )
        };
        assert_eq!(res, Ok(()));
        let bytes = std::fs::read(&out).unwrap();
        assert_ne!(bytes, PRECIOUS, "the export must replace the old file");
        // X-1: 1 s at 30 fps = 30 frames.
        let o = std::process::Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-count_frames",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=nb_read_frames",
                "-of",
                "csv=p=0",
            ])
            .arg(&out)
            .output()
            .expect("ffprobe runs");
        assert_eq!(String::from_utf8_lossy(&o.stdout).trim(), "30");
        assert_eq!(d.names(), vec!["out.mp4".to_string()], "temp file left");
    })
}
