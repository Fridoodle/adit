//! Self-check of the test fixtures, using only the ffmpeg/ffprobe binaries.
//! These do not touch adit-media and pass against the `todo!()` stubs by design: they prove the
//! fixtures are what the other tests assume, so a failure elsewhere is the crate's, not the recipe's.
mod common;

use adit_media::Frame;
use common::*;

fn frame(path: &str, k: usize, w: u32, h: u32) -> Frame {
    let rgba = ffmpeg_decode_frame(path, k);
    assert_eq!(rgba.len(), (w * h * 4) as usize, "{path} frame {k}");
    Frame {
        width: w,
        height: h,
        rgba,
    }
}

#[test]
fn fixture_id_videos_are_pixel_exact() {
    let d = TempDir::new("fx-id");
    for (name, fps) in [("id30.mov", "30/1"), ("id2997.mov", "30000/1001")] {
        let p = id_video(d.path(), name, fps, 60);
        let v = probe_video_stream(&p);
        assert_eq!(v["nb_read_frames"], "60");
        assert_eq!(v["avg_frame_rate"], fps);
        for k in [0, 1, 37, 59] {
            assert_id_frame(&frame(&p, k, 64, 64), k as u32);
        }
    }
}

#[test]
fn fixture_gray_h264_encodes_frame_number() {
    let d = TempDir::new("fx-gray");
    for (name, fps) in [("g30.mp4", "30/1"), ("g2997.mkv", "30000/1001")] {
        let p = gray_h264(d.path(), name, fps, 60);
        let v = probe_video_stream(&p);
        assert_eq!(v["nb_read_frames"], "60");
        assert_eq!(v["avg_frame_rate"], fps);
        assert_eq!(v["pix_fmt"], "yuv420p");
        for k in 0..60 {
            assert_eq!(gray_frame_number(&frame(&p, k, 64, 64)), k as u32, "{name}");
        }
    }
}

#[test]
fn fixture_still_png_has_straight_alpha() {
    let d = TempDir::new("fx-png");
    let p = still_png(d.path());
    let f = frame(&p, 0, 16, 16);
    for y in 0..16usize {
        for x in 0..16usize {
            let i = (y * 16 + x) * 4;
            assert_eq!(f.rgba[i + 3] as usize, x * 16 + y);
            assert_eq!(&f.rgba[i..i + 3], &[200, 100, 50]);
        }
    }
}

#[test]
fn fixture_audio_and_av_streams() {
    let d = TempDir::new("fx-audio");
    let st = ramp_stereo_wav(d.path());
    assert_eq!(
        probe_audio_streams(&st),
        vec![("pcm_f32le".to_string(), 48000)]
    );
    let mono = ramp_mono_wav(d.path());
    assert_eq!(
        probe_audio_streams(&mono),
        vec![("pcm_f32le".to_string(), 44100)]
    );
    let av = av_mp4(d.path(), "av.mp4");
    assert_eq!(probe_audio_streams(&av), vec![("aac".to_string(), 44100)]);
    let v = probe_video_stream(&av);
    assert_eq!((v["width"].as_str(), v["height"].as_str()), ("64", "48"));
}

#[test]
fn fixture_unicode_path_works_with_ffmpeg() {
    let d = TempDir::new("fx-uni");
    let sub = d.unicode_subdir();
    let p = id_video(&sub, "clip ä.mov", "30", 6);
    assert_eq!(probe_video_stream(&p)["nb_read_frames"], "6");
    assert_id_frame(&frame(&p, 2, 64, 64), 2);
}

#[test]
fn fixture_detect_sources() {
    let d = TempDir::new("fx-det");
    // Build every detect fixture and confirm ffmpeg's own detectors agree with the recipe.
    let gap = tone_with_gap_wav(d.path(), "gap.wav", 1.0, 2.0);
    let out = std::process::Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-i",
            &gap,
            "-af",
            "silencedetect=n=-30dB:d=0.5",
        ])
        .args(["-f", "null", "-"])
        .output()
        .unwrap();
    let log = String::from_utf8_lossy(&out.stderr);
    assert_eq!(log.matches("silence_start").count(), 1, "{log}");
    let _ = tone_with_gap_wav(d.path(), "short.wav", 1.0, 1.3);
    let _ = tone_wav(d.path());
    let cut = cut_mp4(d.path());
    let red = frame(&cut, 29, 64, 64);
    let blue = frame(&cut, 30, 64, 64);
    assert!(
        red.rgba[0] > 200 && red.rgba[2] < 50,
        "frame 29 red: {:?}",
        &red.rgba[..4]
    );
    assert!(
        blue.rgba[2] > 200 && blue.rgba[0] < 50,
        "frame 30 blue: {:?}",
        &blue.rgba[..4]
    );
    assert_eq!(
        probe_video_stream(&nocut_mp4(d.path()))["nb_read_frames"],
        "60"
    );
    let _ = still_jpg(d.path());
}
