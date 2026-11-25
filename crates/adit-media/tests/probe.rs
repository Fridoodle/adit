//! M-PROBE-1..3, M-PROC-1 (probe side), `Ffmpeg::new`.
mod common;

use adit_core::{AudioInfo, Rational, VideoInfo};
use adit_media::{Ffmpeg, MediaBackend, MediaError};
use common::*;

const R30: Rational = Rational { num: 30, den: 1 };
const R2997: Rational = Rational {
    num: 30000,
    den: 1001,
};

fn ff() -> Ffmpeg {
    Ffmpeg::new().expect("Ffmpeg::new with ffmpeg/ffprobe on PATH")
}

fn assert_within(actual: i64, expected: i64, tol: i64, what: &str) {
    assert!(
        (actual - expected).abs() <= tol,
        "{what}: {actual} not within {tol} of {expected}"
    );
}

#[test]
fn ffmpeg_new_finds_binaries_on_path() {
    timed(30, || {
        assert!(Ffmpeg::new().is_ok());
    })
}

#[test]
fn m_probe_1_video_and_audio_file() {
    timed(60, || {
        let d = TempDir::new("probe-av");
        let p = av_mp4(d.path(), "av.mp4");
        let m = ff().probe(&p).unwrap();
        assert_eq!(m.name, "av.mp4");
        assert_eq!(m.path, p);
        // 2 s, within one frame (1/30 s).
        assert_within(m.duration.expect("duration"), 2 * S, 23_520_000, "duration");
        assert_eq!(
            m.video,
            Some(VideoInfo {
                width: 64,
                height: 48,
                fps: R30
            })
        );
        assert_eq!(
            m.audio,
            Some(AudioInfo {
                sample_rate: 44100,
                channels: 2
            })
        );
    })
}

#[test]
fn m_probe_1_fps_30000_1001_exact_rational_video_only() {
    timed(60, || {
        let d = TempDir::new("probe-2997");
        let p = id_video(d.path(), "id2997.mov", "30000/1001", 60);
        let m = ff().probe(&p).unwrap();
        assert_eq!(m.name, "id2997.mov");
        // 60 frames at 30000/1001 = 2.002 s = 1_412_611_200 flicks; within one frame.
        assert_within(
            m.duration.expect("duration"),
            1_412_611_200,
            23_543_521,
            "duration",
        );
        assert_eq!(
            m.video,
            Some(VideoInfo {
                width: 64,
                height: 64,
                fps: R2997
            })
        );
        assert_eq!(m.audio, None, "video-only file has audio: None");
    })
}

#[test]
fn m_probe_1_fps_30_video_only() {
    timed(60, || {
        let d = TempDir::new("probe-30");
        let p = id_video(d.path(), "id30.mov", "30", 60);
        let m = ff().probe(&p).unwrap();
        assert_within(m.duration.expect("duration"), 2 * S, 23_520_000, "duration");
        assert_eq!(m.video.expect("video").fps, R30);
        assert_eq!(m.audio, None);
    })
}

#[test]
fn m_probe_1_audio_only_stereo() {
    timed(60, || {
        let d = TempDir::new("probe-wav");
        let p = ramp_stereo_wav(d.path());
        let m = ff().probe(&p).unwrap();
        assert_eq!(m.name, "ramp_st.wav");
        assert_eq!(m.video, None, "audio-only file has video: None");
        assert_eq!(
            m.audio,
            Some(AudioInfo {
                sample_rate: 48000,
                channels: 2
            })
        );
        // Exactly 3 s of PCM; allow 1/30 s.
        assert_within(m.duration.expect("duration"), 3 * S, 23_520_000, "duration");
    })
}

#[test]
fn m_probe_1_mono_source() {
    timed(60, || {
        let d = TempDir::new("probe-mono");
        let p = ramp_mono_wav(d.path());
        let m = ff().probe(&p).unwrap();
        assert_eq!(
            m.audio,
            Some(AudioInfo {
                sample_rate: 44100,
                channels: 1
            })
        );
        assert_eq!(m.video, None);
    })
}

#[test]
fn m_probe_1_rotation_metadata_not_applied() {
    timed(60, || {
        let d = TempDir::new("probe-rot");
        let src = av_mp4(d.path(), "av.mp4");
        let rot = d.file("rot.mp4");
        ffmpeg(&["-display_rotation:v", "90", "-i", &src, "-c", "copy", &rot]);
        let v = ff().probe(&rot).unwrap().video.expect("video");
        assert_eq!(
            (v.width, v.height),
            (64, 48),
            "coded size, not rotated 48x64"
        );
    })
}

#[test]
fn m_probe_1_m_proc_1_path_with_space_and_non_ascii() {
    timed(60, || {
        let d = TempDir::new("probe-uni");
        let sub = d.unicode_subdir();
        let p = id_video(&sub, "clip ä.mov", "30", 6);
        let m = ff().probe(&p).unwrap();
        assert_eq!(m.name, "clip ä.mov");
        assert_eq!(m.path, p);
        assert_eq!(
            m.video,
            Some(VideoInfo {
                width: 64,
                height: 64,
                fps: R30
            })
        );
        // 6 frames = 0.2 s.
        assert_within(
            m.duration.expect("duration"),
            t30(6),
            23_520_000,
            "duration",
        );
    })
}

#[test]
fn m_probe_2_png_still() {
    timed(60, || {
        let d = TempDir::new("probe-png");
        let p = still_png(d.path());
        let m = ff().probe(&p).unwrap();
        assert_eq!(m.name, "still.png");
        assert_eq!(m.duration, None);
        assert_eq!(
            m.video,
            Some(VideoInfo {
                width: 16,
                height: 16,
                fps: Rational { num: 25, den: 1 }
            })
        );
        assert_eq!(m.audio, None);
    })
}

#[test]
fn m_probe_2_jpg_still() {
    timed(60, || {
        let d = TempDir::new("probe-jpg");
        let p = still_jpg(d.path());
        let m = ff().probe(&p).unwrap();
        assert_eq!(m.duration, None);
        assert_eq!(
            m.video,
            Some(VideoInfo {
                width: 32,
                height: 24,
                fps: Rational { num: 25, den: 1 }
            })
        );
        assert_eq!(m.audio, None);
    })
}

#[test]
fn m_probe_3_missing_file_is_not_found() {
    timed(30, || {
        let d = TempDir::new("probe-missing");
        let r = ff().probe(&d.file("does not exist.mp4"));
        assert!(matches!(r, Err(MediaError::NotFound(_))), "{r:?}");
    })
}

#[test]
fn m_probe_3_unreadable_text_file() {
    timed(30, || {
        let d = TempDir::new("probe-text");
        let ff = ff();
        for name in ["notes.txt", "fake.mp4"] {
            let p = d.file(name);
            std::fs::write(&p, "hello, this is not media\n").unwrap();
            let r = ff.probe(&p);
            assert!(
                matches!(
                    r,
                    Err(MediaError::Ffmpeg(_)) | Err(MediaError::Unsupported(_))
                ),
                "{name}: {r:?}"
            );
        }
    })
}
