//! `Ffmpeg::new` contract from lib.rs: env `ADIT_FFMPEG` / `ADIT_FFPROBE`, else PATH; checks they run.
//! Own test binary with a single test, because it changes process environment variables.
mod common;

use adit_media::{Ffmpeg, MediaError};

fn set(k: &str, v: Option<&str>) {
    // SAFETY: the only test in this binary; no other thread reads the environment meanwhile.
    unsafe {
        match v {
            Some(v) => std::env::set_var(k, v),
            None => std::env::remove_var(k),
        }
    }
}

#[test]
fn ffmpeg_new_env_overrides_and_missing_binaries() {
    let d = common::TempDir::new("new-env");
    let bogus = d.file("no_such_ffmpeg.exe");
    let old = (
        std::env::var("ADIT_FFMPEG").ok(),
        std::env::var("ADIT_FFPROBE").ok(),
    );

    set("ADIT_FFMPEG", None);
    set("ADIT_FFPROBE", None);
    let ok = Ffmpeg::new();

    set("ADIT_FFMPEG", Some(&bogus));
    let bad_ffmpeg = Ffmpeg::new();
    set("ADIT_FFMPEG", None);

    set("ADIT_FFPROBE", Some(&bogus));
    let bad_ffprobe = Ffmpeg::new();

    set("ADIT_FFMPEG", old.0.as_deref());
    set("ADIT_FFPROBE", old.1.as_deref());

    assert!(ok.is_ok(), "PATH lookup: {:?}", ok.err());
    assert!(
        matches!(bad_ffmpeg, Err(MediaError::FfmpegMissing(_))),
        "{:?}",
        bad_ffmpeg.err()
    );
    assert!(
        matches!(bad_ffprobe, Err(MediaError::FfmpegMissing(_))),
        "{:?}",
        bad_ffprobe.err()
    );
}
