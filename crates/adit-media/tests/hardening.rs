//! SPEC Part 5 hardening rules for adit-media: H-6 (audio filter allow-list), H-7 (start
//! offsets), H-8 (variable frame rate), H-14 (detection through `MediaBackend`).
//! Fixtures are made with the ffmpeg binary; every recipe was checked with ffprobe by hand
//! (offset files: start_time=10; VFR file: one frame lasting 2.033 s).
mod common;

use adit_core::Rational;
use adit_media::{Ffmpeg, MediaBackend, MediaError, Pattern, check_audio_filter};
use common::*;

fn ff() -> Ffmpeg {
    Ffmpeg::new().expect("Ffmpeg::new with ffmpeg/ffprobe on PATH")
}

/// SPEC Q4 `frame_to_time`: floor(n * s * den / num).
fn frame_to_time(n: i64, fps: Rational) -> i64 {
    (i128::from(n) * i128::from(S) * i128::from(fps.den) / i128::from(fps.num)) as i64
}

// ------------------------------------------------------------------------------------- H-6

const ALLOWED: &[&str] = &[
    "volume",
    "atempo",
    "areverse",
    "highpass",
    "lowpass",
    "bandpass",
    "bandreject",
    "equalizer",
    "bass",
    "treble",
    "acompressor",
    "alimiter",
    "agate",
    "aecho",
    "chorus",
    "flanger",
    "aphaser",
    "tremolo",
    "vibrato",
    "afade",
    "adelay",
    "loudnorm",
    "dynaudnorm",
    "compand",
    "anequalizer",
    "afftdn",
    "anlmdn",
    "deesser",
    "crystalizer",
    "stereowiden",
    "extrastereo",
    "pan",
    "aresample",
    "asetrate",
    "silenceremove",
    "apad",
    "atrim",
    "asetpts",
    "acontrast",
    "asoftclip",
    "speechnorm",
    "channelmap",
    "aformat",
];

/// The filters H-6 names as forbidden.
const FORBIDDEN: &[&str] = &[
    "amovie",
    "movie",
    "ametadata",
    "asendcmd",
    "azmq",
    "afir",
    "arnndn",
    "ladspa",
    "lv2",
];

#[test]
fn h_6_empty_filter_ok() {
    assert_eq!(check_audio_filter(""), Ok(()));
}

#[test]
fn h_6_every_allowed_name_alone_ok() {
    for name in ALLOWED {
        assert_eq!(check_audio_filter(name), Ok(()), "{name}");
    }
}

#[test]
fn h_6_accepted_chains() {
    for f in [
        "volume=0.5",
        "atempo=2.0",
        "areverse,atempo=1.5,volume=-3dB",
        "highpass=f=200,lowpass=f=3000",
        "equalizer=f=1000:t=q:w=1:g=2",
        // `|` is not forbidden; the renderer's own mono upmix looks like this.
        "pan=stereo|c0=c0|c1=c0",
        "afade=t=in:st=0:d=1",
        "aformat=sample_fmts=fltp:channel_layouts=stereo",
        "atrim=start=0.5,asetpts=PTS-STARTPTS",
        "atempo=2.0,atempo=2.0,atempo=1.25",
    ] {
        assert_eq!(check_audio_filter(f), Ok(()), "{f:?}");
    }
}

#[test]
fn h_6_named_forbidden_filters_rejected() {
    for name in FORBIDDEN {
        for f in [
            name.to_string(),
            format!("{name}=x"),
            format!("volume=0.5,{name}=x"),
        ] {
            let e = check_audio_filter(&f).expect_err(&f);
            assert!(e.contains(name), "{f:?}: error {e:?} should name {name}");
        }
    }
}

#[test]
fn h_6_forbidden_with_realistic_args_rejected() {
    for f in [
        "amovie=secret.wav",
        "movie=clip.mp4",
        "ametadata=mode=print:file=leak.txt",
        "asendcmd=f=cmds.txt",
        "azmq=bind_address=tcp\\://*\\:5555",
        "afir=gtype=gn",
        "arnndn=m=model.rnnn",
        "ladspa=file=plugin.dll",
        "lv2=p=urn\\:x",
        "atempo=2,ametadata=mode=print",
    ] {
        assert!(check_audio_filter(f).is_err(), "{f:?} must be rejected");
    }
}

#[test]
fn h_6_semicolon_rejected() {
    for f in [
        "volume=1;volume=2",
        "volume=1;",
        ";",
        "volume=1;amovie=x.wav",
    ] {
        assert!(check_audio_filter(f).is_err(), "{f:?} must be rejected");
    }
}

#[test]
fn h_6_brackets_rejected() {
    for f in [
        "[a]volume=1",
        "volume=1[out]",
        "[in]volume=1[out]",
        "pan=stereo[x]",
        "volume=1,[x]atempo=2",
        "[",
        "]",
    ] {
        assert!(check_audio_filter(f).is_err(), "{f:?} must be rejected");
    }
}

#[test]
fn h_6_names_that_only_start_with_an_allowed_name_rejected() {
    for f in [
        "volumedetect",
        "volumedetect=x",
        "volumes=1",
        "atempox=2",
        "pane",
        "apadx",
        "aformats",
        "highpass2=f=1",
        "volume=1,volumedetect",
    ] {
        assert!(check_audio_filter(f).is_err(), "{f:?} must be rejected");
    }
}

#[test]
fn h_6_names_not_in_allow_list_rejected() {
    for f in [
        "aevalsrc=0",
        "sine",
        "amix",
        "anullsrc",
        "astats",
        "showwavespic",
        "vol=1",
        // ffmpeg filter names are case-sensitive; `VOLUME` is not `volume`.
        "VOLUME=1",
    ] {
        assert!(check_audio_filter(f).is_err(), "{f:?} must be rejected");
    }
}

#[test]
fn h_6_empty_filter_names_rejected() {
    // Each comma-separated part must be `name` or `name=args` with an allowed name; an empty
    // name is not in the allow-list.
    for f in ["volume=1,", ",volume=1", "volume=1,,atempo=2", "=1", ","] {
        assert!(check_audio_filter(f).is_err(), "{f:?} must be rejected");
    }
}

#[test]
fn h_6_whitespace_does_not_hide_forbidden_filters() {
    // Whether or not whitespace around names is tolerated, a forbidden name stays forbidden.
    for f in [
        " amovie=x.wav",
        "amovie =x.wav",
        "volume=1, amovie=x.wav",
        "volume=1,\tametadata=mode=print",
        "volume=1,\namovie=x.wav",
        "volume=1 ,ametadata",
    ] {
        assert!(check_audio_filter(f).is_err(), "{f:?} must be rejected");
    }
}

/// `ametadata` filter that writes to `file`, escaped for an ffmpeg filter option.
fn ametadata_to(file: &std::path::Path) -> String {
    let p = s(file).replace('\\', "/").replace(':', "\\:");
    format!("ametadata=mode=print:file='{p}'")
}

#[test]
fn h_6_ffmpeg_audio_refuses_forbidden_filter_and_writes_nothing() {
    timed(60, || {
        let d = TempDir::new("h6-leak");
        let wav = ramp_stereo_wav(d.path());
        // Fixture check: run by ffmpeg itself, this filter does create its file.
        let probe = d.path().join("probe.txt");
        ffmpeg(&["-i", &wav, "-af", &ametadata_to(&probe), "-f", "null", "-"]);
        assert!(probe.exists(), "fixture: ffmpeg should create {probe:?}");

        let leak = d.path().join("leak.txt");
        let r = ff().audio(&wav, 0, S, 48000, &ametadata_to(&leak));
        assert!(
            matches!(r, Err(MediaError::Ffmpeg(_))),
            "H-6: expected MediaError::Ffmpeg, got {:?}",
            r.map(|v| v.len())
        );
        assert!(!leak.exists(), "H-6: {leak:?} must not be created");
    })
}

#[test]
fn h_6_ffmpeg_audio_allowed_chain_still_works() {
    timed(60, || {
        let d = TempDir::new("h6-ok");
        let wav = ramp_stereo_wav(d.path());
        // Ramp: left = t/4, right = -t/8. From 1 s with volume 0.5: (0.125, -0.0625).
        let v = ff().audio(&wav, S, S / 2, 48000, "volume=0.5").unwrap();
        assert!(
            v.len().abs_diff(48000) <= 4,
            "{} samples, expected 48000 ±4",
            v.len()
        );
        assert!((v[0] - 0.125).abs() < 2e-3, "first left = {}", v[0]);
        assert!((v[1] + 0.0625).abs() < 2e-3, "first right = {}", v[1]);
    })
}

// ------------------------------------------------------------------------------------- H-7

/// 60 frames of 30 fps H.264 (qp 0), frame N flat grey 4N, first timestamp 10 s.
fn gray_offset(d: &TempDir, name: &str) -> String {
    let p = d.file(name);
    ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        "nullsrc=s=64x64:r=30,geq=r='N*4':g='N*4':b='N*4'",
        "-frames:v",
        "60",
        "-c:v",
        "libx264",
        "-qp",
        "0",
        "-pix_fmt",
        "yuv420p",
        "-output_ts_offset",
        "10",
        &p,
    ]);
    let st = ffprobe(&["-show_entries", "format=start_time", "-of", "csv=p=0", &p]);
    let st: f64 = st.trim().parse().unwrap();
    assert!((st - 10.0).abs() < 0.01, "fixture start_time {st}, want 10");
    p
}

fn h7_cold(ext: &'static str) {
    timed(180, move || {
        let d = TempDir::new(&format!("h7-cold-{ext}"));
        let p = gray_offset(&d, &format!("off.{ext}"));
        for n in [0u32, 1, 17, 45, 59] {
            // Fresh backend per read: nothing cached.
            let f = ff().video_frame(&p, t30(n as i64)).unwrap();
            let got = gray_frame_number(&f);
            assert_eq!(got, n, "cold read of frame {n} ({ext}) gave frame {got}");
        }
    })
}

fn h7_order(ext: &'static str, order: Vec<u32>, tag: &'static str) {
    timed(180, move || {
        let d = TempDir::new(&format!("h7-{tag}-{ext}"));
        let p = gray_offset(&d, &format!("off.{ext}"));
        let ff = ff();
        for (i, &n) in order.iter().enumerate() {
            let f = ff.video_frame(&p, t30(n as i64)).unwrap();
            let got = gray_frame_number(&f);
            assert_eq!(got, n, "{tag} request #{i} ({ext}): frame {n}, got {got}");
        }
    })
}

fn random() -> Vec<u32> {
    vec![37, 5, 58, 6, 0, 59, 1, 30, 29, 31, 12, 44, 2, 3, 50, 49, 17]
}

/// M-FRAME-2 measured from the first frame: `t >=` the 2 s length gives frame 59, whatever the
/// container says about its absolute end.
fn h7_past_end(ext: &'static str) {
    timed(180, move || {
        let d = TempDir::new(&format!("h7-end-{ext}"));
        let p = gray_offset(&d, &format!("off.{ext}"));
        for t in [2 * S, 5 * S, 11 * S, 20 * S] {
            let f = ff().video_frame(&p, t).unwrap();
            let got = gray_frame_number(&f);
            assert_eq!(got, 59, "t = {} s ({ext}) gave frame {got}", t / S);
        }
    })
}

#[test]
fn h_7_offset_past_end_returns_last_frame_mp4() {
    h7_past_end("mp4");
}
#[test]
fn h_7_offset_past_end_returns_last_frame_mkv() {
    h7_past_end("mkv");
}

#[test]
fn h_7_offset_cold_reads_mp4() {
    h7_cold("mp4");
}
#[test]
fn h_7_offset_cold_reads_mkv() {
    h7_cold("mkv");
}
#[test]
fn h_7_offset_random_reads_mp4() {
    h7_order("mp4", random(), "random");
}
#[test]
fn h_7_offset_random_reads_mkv() {
    h7_order("mkv", random(), "random");
}
#[test]
fn h_7_offset_sequential_reads_mp4() {
    h7_order("mp4", (0..60).collect(), "seq");
}
#[test]
fn h_7_offset_sequential_reads_mkv() {
    h7_order("mkv", (0..60).collect(), "seq");
}

fn h7_probe(ext: &'static str) {
    timed(60, move || {
        let d = TempDir::new(&format!("h7-probe-{ext}"));
        let p = gray_offset(&d, &format!("off.{ext}"));
        let spec = ff().probe(&p).unwrap();
        let dur = spec.duration.expect("video file has a duration");
        // 60 frames at 30 fps = 2 s, measured from the start time (not 12 s).
        assert!(
            (dur - 2 * S).abs() <= t30(1),
            "H-7: duration {dur} ({} s), expected 2 s within one frame",
            dur as f64 / S as f64
        );
    })
}

#[test]
fn h_7_offset_probe_duration_mp4() {
    h7_probe("mp4");
}
#[test]
fn h_7_offset_probe_duration_mkv() {
    h7_probe("mkv");
}

/// 3 s stereo f32 ramp (left t/4, right -t/8) in mkv, first timestamp 10 s.
fn ramp_offset_mkv(d: &TempDir) -> String {
    let p = d.file("aoff.mkv");
    ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        "aevalsrc='t/4|-t/8':s=48000:d=3",
        "-c:a",
        "pcm_f32le",
        "-output_ts_offset",
        "10",
        &p,
    ]);
    p
}

#[test]
fn h_7_offset_audio_start_counts_from_audio_beginning() {
    timed(60, || {
        let d = TempDir::new("h7-audio");
        let p = ramp_offset_mkv(&d);
        let v = ff().audio(&p, S, S / 2, 48000, "").unwrap();
        // 0.5 s at 48 kHz stereo is about 48000 samples (mkv cuts on 1 ms blocks, so the
        // exact length is M-AUDIO-1's business, not this rule's).
        assert!(
            v.len() > 47_000,
            "{} samples, expected about 48000",
            v.len()
        );
        // Frame 0 is source t = 1 s: (0.25, -0.125); frame 12000 is t = 1.25 s: left 0.3125.
        assert!((v[0] - 0.25).abs() < 2e-3, "first left = {}", v[0]);
        assert!((v[1] + 0.125).abs() < 2e-3, "first right = {}", v[1]);
        assert!(
            (v[24_000] - 0.3125).abs() < 2e-3,
            "left at 1.25 s = {}",
            v[24_000]
        );
    })
}

#[test]
fn h_7_offset_audio_probe_duration() {
    timed(60, || {
        let d = TempDir::new("h7-aprobe");
        let p = ramp_offset_mkv(&d);
        let dur = ff().probe(&p).unwrap().duration.expect("duration");
        // 3 s of audio, not 13 s.
        assert!(
            (dur - 3 * S).abs() <= S / 20,
            "H-7: duration {} s, expected 3 s",
            dur as f64 / S as f64
        );
    })
}

// ------------------------------------------------------------------------------------- H-8

/// 60 frames, frame N flat grey 4N; frames 0..29 at 30 fps, then a 2 s gap (frame 29 lasts
/// 2.033 s), frames 30..59 at 30 fps from 3 s. Total 4 s.
fn vfr(d: &TempDir, name: &str) -> String {
    let p = d.file(name);
    ffmpeg(&[
        "-f",
        "lavfi",
        "-i",
        "nullsrc=s=64x64:r=30,geq=r='N*4':g='N*4':b='N*4',setpts='if(gte(N,30),PTS+60,PTS)'",
        "-frames:v",
        "60",
        "-fps_mode",
        "passthrough",
        "-c:v",
        "libx264",
        "-qp",
        "0",
        "-pix_fmt",
        "yuv420p",
        &p,
    ]);
    // Fixture check: frame 30 starts at 3 s.
    let pts = ffprobe(&[
        "-select_streams",
        "v",
        "-show_entries",
        "frame=pts_time",
        "-of",
        "csv=p=0",
        &p,
    ]);
    let t30: f64 = pts.lines().nth(30).unwrap().trim().parse().unwrap();
    assert!(
        (t30 - 3.0).abs() < 0.01,
        "fixture frame 30 at {t30}, want 3"
    );
    p
}

fn h8(ext: &'static str) {
    timed(300, move || {
        let d = TempDir::new(&format!("h8-{ext}"));
        let p = vfr(&d, &format!("vfr.{ext}"));
        let targets = [
            S / 2,
            S * 98 / 100,
            2 * S,
            3 * S + S / 20,
            3 * S + S / 2,
            3 * S + S * 9 / 10,
        ];
        // Backend B: a sequential run from 0, then the targets.
        let seq = ff();
        let fps = seq.probe(&p).unwrap().video.expect("video").fps;
        let last = *targets.iter().max().unwrap();
        let mut n = 0;
        while frame_to_time(n, fps) <= last {
            seq.video_frame(&p, frame_to_time(n, fps)).unwrap();
            n += 1;
        }
        for &t in &targets {
            let after_run = seq.video_frame(&p, t).unwrap();
            // A fresh backend per target: a cold read.
            let cold = ff().video_frame(&p, t).unwrap();
            let (a, b) = (gray_frame_number(&cold), gray_frame_number(&after_run));
            assert!(
                cold.rgba == after_run.rgba,
                "H-8 ({ext}): t = {} s cold gave frame {a}, after a sequential run frame {b}",
                t as f64 / S as f64
            );
        }
    })
}

#[test]
fn h_8_vfr_cold_equals_sequential_mp4() {
    h8("mp4");
}
#[test]
fn h_8_vfr_cold_equals_sequential_mkv() {
    h8("mkv");
}

// ------------------------------------------------------------------------------------- H-14

#[test]
fn h_14_pattern_detection_unsupported() {
    let p = Pattern {
        width: 64,
        height: 64,
        fps: Rational { num: 30, den: 1 },
        duration: Some(10 * S),
    };
    let b: &dyn MediaBackend = &p;
    assert!(matches!(
        b.detect_silence("x.wav", -30.0, S / 2),
        Err(MediaError::Unsupported(_))
    ));
    assert!(matches!(
        b.detect_scenes("x.mp4", 0.3),
        Err(MediaError::Unsupported(_))
    ));
}

#[test]
fn h_14_ffmpeg_detect_silence_through_trait_object() {
    timed(60, || {
        let d = TempDir::new("h14-sil");
        let p = tone_with_gap_wav(d.path(), "gap.wav", 1.0, 2.0);
        let ff = ff();
        let b: &dyn MediaBackend = &ff;
        let r = b.detect_silence(&p, -30.0, S / 2).unwrap();
        assert_eq!(r.len(), 1, "{r:?}");
        assert!((r[0].0 - S).abs() <= S / 10, "start {}", r[0].0);
        assert!((r[0].1 - 2 * S).abs() <= S / 10, "end {}", r[0].1);
    })
}

#[test]
fn h_14_ffmpeg_detect_scenes_through_trait_object() {
    timed(60, || {
        let d = TempDir::new("h14-cut");
        let p = cut_mp4(d.path());
        let ff = ff();
        let b: &dyn MediaBackend = &ff;
        let r = b.detect_scenes(&p, 0.3).unwrap();
        assert_eq!(r.len(), 1, "{r:?}");
        assert!((r[0] - S).abs() <= t30(1), "cut at {}", r[0]);
    })
}
