//! SPEC Part 5 hardening cases that kill the process today (stack overflow, allocation abort).
//! Each case is its own test so a crash names the culprit; the ordinary cases are in
//! `hardening.rs`. After the fix every test here must pass in-process.
mod common;
use adit_core::*;
use adit_render::{mix_audio, validate_effect};
use common::*;

fn filter(wgsl: &str) -> EffectDef {
    def("h", EffectKind::Filter, wgsl, &[])
}

/// Function body plus `depth - 1` nested blocks: `depth` braces deep.
fn braces(depth: usize) -> String {
    format!(
        "fn effect(uv: vec2f) -> vec4f {{ {}{} return src(uv); }}\n",
        "{".repeat(depth - 1),
        "}".repeat(depth - 1)
    )
}

// ------------------------------------------------------------------------------------- H-1

#[test]
fn h_1_brace_depth_64_ok() {
    assert_eq!(validate_effect(&filter(&braces(64))), Ok(()));
}

#[test]
fn h_1_brace_depth_65_rejected() {
    assert!(validate_effect(&filter(&braces(65))).is_err());
}

// ------------------------------------------------------------------------------------- H-5

#[test]
fn h_5_mix_audio_duration_above_max_time_rejected() {
    let (p, s) = audio_project();
    assert!(mix_audio(&p, s, 0, MAX_TIME + 1, &backend()).is_err());
}

#[test]
fn h_5_mix_audio_duration_i64_max_rejected() {
    let (p, s) = audio_project();
    assert!(mix_audio(&p, s, 0, i64::MAX, &backend()).is_err());
}

/// 64x64 sequence with one audio track holding a 1 s clip of constant (0.5, 0.25) audio.
fn audio_project() -> (Project, Id) {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let a = b.atrack(s);
    let m = b.media("a.wav", None, true);
    b.media_clip(a, 0, S, m);
    (b.p, s)
}

fn backend() -> Fake {
    const_audio(&[("a.wav", 0.5, 0.25)], 1.0)
}
