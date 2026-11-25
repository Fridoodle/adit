//! M-ENC-1..4, M-PROC-1 (encoder side). Output is checked with the ffprobe/ffmpeg binaries only.
mod common;

use std::time::{Duration, Instant};

use adit_core::Rational;
use adit_media::{AudioCodec, Encoder, ExportSettings, MediaError, VideoCodec};
use common::*;

const R30: Rational = Rational { num: 30, den: 1 };
const R2997: Rational = Rational {
    num: 30000,
    den: 1001,
};

fn settings(
    path: &str,
    w: u32,
    h: u32,
    fps: Rational,
    video: VideoCodec,
    audio: AudioCodec,
) -> ExportSettings {
    ExportSettings {
        path: path.to_string(),
        width: w,
        height: h,
        fps,
        sample_rate: 48000,
        video,
        audio,
        crf: None,
    }
}

/// Distinct flat colours, away from the clipping ends.
const COLOURS: [[u8; 4]; 6] = [
    [200, 30, 60, 255],
    [20, 180, 90, 255],
    [90, 90, 220, 255],
    [240, 230, 20, 255],
    [128, 128, 128, 255],
    [40, 40, 40, 255],
];

fn encode(st: &ExportSettings, audio: Option<&[f32]>, frames: usize) {
    let mut e = Encoder::start(st, audio).unwrap();
    for k in 0..frames {
        e.write_frame(&flat_rgba(st.width, st.height, COLOURS[k % COLOURS.len()]))
            .unwrap();
    }
    e.finish().unwrap();
}

fn assert_video(path: &str, w: u32, h: u32, fps: &str, frames: usize, codec: &str) {
    let v = probe_video_stream(path);
    assert_eq!(v["codec_name"], codec, "{v:?}");
    assert_eq!(v["width"], w.to_string(), "{v:?}");
    assert_eq!(v["height"], h.to_string(), "{v:?}");
    assert_eq!(v["avg_frame_rate"], fps, "{v:?}");
    assert_eq!(v["nb_read_frames"], frames.to_string(), "{v:?}");
}

#[test]
fn m_enc_1_h264_mp4_with_aac() {
    timed(120, || {
        let d = TempDir::new("enc-h264");
        let p = d.file("out.mp4");
        let st = settings(&p, 64, 48, R30, VideoCodec::H264, AudioCodec::Aac);
        let audio = stereo_sine(48000, 0.4);
        encode(&st, Some(&audio), 12);
        assert_video(&p, 64, 48, "30/1", 12, "h264");
        assert_eq!(probe_video_stream(&p)["pix_fmt"], "yuv420p");
        let a = probe_audio_streams(&p);
        assert_eq!(a.len(), 1, "{a:?}");
        assert_eq!(a[0], ("aac".to_string(), 48000));
    })
}

#[test]
fn m_enc_1_h264_2997_without_audio() {
    timed(120, || {
        let d = TempDir::new("enc-2997");
        let p = d.file("out.mp4");
        let st = settings(&p, 64, 48, R2997, VideoCodec::H264, AudioCodec::Aac);
        encode(&st, None, 15);
        assert_video(&p, 64, 48, "30000/1001", 15, "h264");
        assert!(
            probe_audio_streams(&p).is_empty(),
            "no audio given -> no audio stream"
        );
    })
}

#[test]
fn m_enc_1_h265() {
    timed(120, || {
        let d = TempDir::new("enc-h265");
        let p = d.file("out.mp4");
        let st = settings(&p, 64, 48, R30, VideoCodec::H265, AudioCodec::None);
        encode(&st, None, 9);
        assert_video(&p, 64, 48, "30/1", 9, "hevc");
    })
}

#[test]
fn m_enc_1_prores_mov_with_pcm() {
    timed(120, || {
        let d = TempDir::new("enc-prores");
        let p = d.file("out.mov");
        let mut st = settings(&p, 64, 48, R2997, VideoCodec::Prores, AudioCodec::Pcm);
        st.sample_rate = 44100;
        let audio = stereo_sine(44100, 0.5);
        encode(&st, Some(&audio), 10);
        assert_video(&p, 64, 48, "30000/1001", 10, "prores");
        let a = probe_audio_streams(&p);
        assert_eq!(a.len(), 1, "{a:?}");
        assert!(a[0].0.starts_with("pcm_"), "{a:?}");
        assert_eq!(a[0].1, 44100);
    })
}

#[test]
fn m_enc_1_gif() {
    timed(120, || {
        let d = TempDir::new("enc-gif");
        let p = d.file("out.gif");
        let st = settings(
            &p,
            64,
            48,
            Rational { num: 25, den: 1 },
            VideoCodec::Gif,
            AudioCodec::None,
        );
        encode(&st, None, 10);
        assert_video(&p, 64, 48, "25/1", 10, "gif");
    })
}

#[test]
fn m_enc_1_audio_codec_none_drops_given_audio() {
    timed(120, || {
        let d = TempDir::new("enc-anone");
        let p = d.file("out.mp4");
        let st = settings(&p, 64, 48, R30, VideoCodec::H264, AudioCodec::None);
        let audio = stereo_sine(48000, 0.4);
        encode(&st, Some(&audio), 12);
        assert_video(&p, 64, 48, "30/1", 12, "h264");
        assert!(probe_audio_streams(&p).is_empty());
    })
}

#[test]
fn m_enc_1_h264_odd_size_is_unsupported() {
    timed(60, || {
        let d = TempDir::new("enc-odd");
        for (w, h) in [(63, 48), (64, 47), (1, 1)] {
            let p = d.file(&format!("odd{w}x{h}.mp4"));
            let st = settings(&p, w, h, R30, VideoCodec::H264, AudioCodec::None);
            let r = Encoder::start(&st, None);
            assert!(
                matches!(r, Err(MediaError::Unsupported(_))),
                "{w}x{h}: {:?}",
                r.err()
            );
        }
    })
}

fn mean_abs_error(a: &[u8], c: [u8; 4]) -> [f64; 3] {
    let mut e = [0f64; 3];
    for px in a.chunks_exact(4) {
        for ch in 0..3 {
            e[ch] += (px[ch] as f64 - c[ch] as f64).abs();
        }
    }
    let n = (a.len() / 4) as f64;
    e.map(|x| x / n)
}

fn check_roundtrip(codec: VideoCodec, file: &'static str, tag: &'static str) {
    timed(120, move || {
        let d = TempDir::new(tag);
        let p = d.file(file);
        let mut st = settings(&p, 64, 48, R30, codec, AudioCodec::None);
        st.crf = Some(10);
        encode(&st, None, COLOURS.len());
        for (k, c) in COLOURS.iter().enumerate() {
            let px = ffmpeg_decode_frame(&p, k);
            assert_eq!(px.len(), 64 * 48 * 4, "frame {k} size");
            let e = mean_abs_error(&px, *c);
            assert!(
                e.iter().all(|x| *x < 8.0),
                "frame {k}: MAE {e:?} for colour {c:?}"
            );
        }
    })
}

#[test]
fn m_enc_2_h264_frame_k_is_kth_written() {
    check_roundtrip(VideoCodec::H264, "rt.mp4", "enc-rt264");
}

#[test]
fn m_enc_2_h265_frame_k_is_kth_written() {
    check_roundtrip(VideoCodec::H265, "rt.mp4", "enc-rt265");
}

#[test]
fn m_enc_2_prores_frame_k_is_kth_written() {
    check_roundtrip(VideoCodec::Prores, "rt.mov", "enc-rtpro");
}

#[test]
fn m_enc_3_wrong_buffer_length_is_io_and_writes_nothing() {
    timed(120, || {
        let d = TempDir::new("enc-len");
        let p = d.file("out.mp4");
        let st = settings(&p, 64, 48, R30, VideoCodec::H264, AudioCodec::None);
        let mut e = Encoder::start(&st, None).unwrap();
        let good = flat_rgba(64, 48, COLOURS[0]);
        for bad in [
            vec![],
            vec![0u8; 64 * 48 * 4 - 1],
            vec![0u8; 64 * 48 * 4 + 4],
            vec![0u8; 64 * 48 * 3],
        ] {
            let r = e.write_frame(&bad);
            assert!(
                matches!(r, Err(MediaError::Io(_))),
                "len {}: {r:?}",
                bad.len()
            );
        }
        for _ in 0..5 {
            e.write_frame(&good).unwrap();
        }
        e.finish().unwrap();
        // Rejected buffers wrote nothing: exactly the 5 good frames.
        assert_video(&p, 64, 48, "30/1", 5, "h264");
    })
}

#[test]
fn m_enc_3_unwritable_output_errors_with_stderr_tail_and_no_hang() {
    timed(60, || {
        let d = TempDir::new("enc-unwr");
        let p = d.file("missing subdir/out.mp4");
        let st = settings(&p, 64, 48, R30, VideoCodec::H264, AudioCodec::None);
        let e = match Encoder::start(&st, None) {
            // Failing up front is allowed.
            Err(_) => return,
            Ok(e) => e,
        };
        let mut e = e;
        let frame = flat_rgba(64, 48, COLOURS[0]);
        let mut err: Option<MediaError> = None;
        for _ in 0..30 {
            if let Err(x) = e.write_frame(&frame) {
                err = Some(x);
                break;
            }
        }
        let err = match err {
            Some(x) => x,
            None => e
                .finish()
                .expect_err("finish must fail when ffmpeg cannot open the output"),
        };
        // ffmpeg's stderr says: "Error opening output ...: No such file or directory".
        let msg = err.to_string();
        assert!(
            msg.contains("No such file or directory"),
            "error must carry ffmpeg's stderr tail, got: {msg}"
        );
        assert!(!std::path::Path::new(&p).exists());
    })
}

#[test]
fn m_enc_3_drop_without_finish_kills_ffmpeg() {
    timed(60, || {
        let d = TempDir::new("enc-drop");
        let p = d.file("out.mp4");
        let st = settings(&p, 64, 48, R30, VideoCodec::H264, AudioCodec::None);
        let mut e = Encoder::start(&st, None).unwrap();
        for k in 0..3 {
            e.write_frame(&flat_rgba(64, 48, COLOURS[k])).unwrap();
        }
        drop(e);
        // Once ffmpeg is gone the file is not held open (Windows refuses to delete open files).
        // A killed mp4 has no moov atom, so ffprobe must not find a finished 3-frame file.
        let t0 = Instant::now();
        loop {
            if !std::path::Path::new(&p).exists() {
                break;
            }
            let probed = std::process::Command::new("ffprobe")
                .args(["-v", "error", "-count_frames", "-select_streams", "v:0"])
                .args([
                    "-show_entries",
                    "stream=nb_read_frames",
                    "-of",
                    "csv=p=0",
                    &p,
                ])
                .output()
                .unwrap();
            assert!(
                !(probed.status.success() && String::from_utf8_lossy(&probed.stdout).trim() == "3"),
                "dropped Encoder produced a finished file: ffmpeg was not killed"
            );
            if std::fs::remove_file(&p).is_ok() {
                break;
            }
            assert!(
                t0.elapsed() < Duration::from_secs(5),
                "output still held open 5 s after drop"
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    })
}

#[test]
fn m_enc_4_short_audio_does_not_cut_video() {
    timed(120, || {
        let d = TempDir::new("enc-short");
        let p = d.file("out.mp4");
        let st = settings(&p, 64, 48, R30, VideoCodec::H264, AudioCodec::Aac);
        let audio = stereo_sine(48000, 0.2); // 0.2 s audio, 1 s video
        encode(&st, Some(&audio), 30);
        assert_video(&p, 64, 48, "30/1", 30, "h264");
        assert_eq!(probe_audio_streams(&p).len(), 1);
    })
}

#[test]
fn m_proc_1_encode_to_path_with_space_and_non_ascii() {
    timed(120, || {
        let d = TempDir::new("enc-uni");
        let sub = d.unicode_subdir();
        let p = s(&sub.join("out ä ü.mp4"));
        let st = settings(&p, 64, 48, R30, VideoCodec::H264, AudioCodec::Aac);
        let audio = stereo_sine(48000, 0.2);
        encode(&st, Some(&audio), 5);
        assert_video(&p, 64, 48, "30/1", 5, "h264");
    })
}

#[test]
fn m_proc_1_long_encode_does_not_deadlock_on_stderr() {
    // Partial check: 900 frames produce plenty of progress/log output if stderr is not drained.
    timed(240, || {
        let d = TempDir::new("enc-long");
        let p = d.file("long.mp4");
        let st = settings(&p, 64, 48, R30, VideoCodec::H264, AudioCodec::Aac);
        let audio = stereo_sine(48000, 30.0);
        encode(&st, Some(&audio), 900);
        assert_video(&p, 64, 48, "30/1", 900, "h264");
    })
}
