//! SPEC V-11 (`media_frames`). Media paths are made-up strings: nothing is decoded.
//! Times are written with `ms()` (1 ms = 705_600 flicks); results are hand-computed.
mod common;
use adit_core::*;
use adit_render::media_frames;
use common::*;

fn e(path: &str, t: Time) -> (String, Time) {
    (path.to_string(), t)
}

#[test]
fn v11_single_clip_source_in_and_speed() {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v = b.vtrack(s);
    let m = b.media("a.mp4", Some((64, 64)), true);
    let c = b.media_clip(v, ms(1000), ms(4000), m);
    b.clip_mut(c).source_in = ms(10_000);
    b.clip_mut(c).speed = 2.0;
    // local 1 s -> 10 s + 2 s
    assert_eq!(
        media_frames(&b.p, s, ms(2000)),
        vec![e("a.mp4", ms(12_000))]
    );
    // clip start (inclusive)
    assert_eq!(
        media_frames(&b.p, s, ms(1000)),
        vec![e("a.mp4", ms(10_000))]
    );
    // local 2.5 s -> 10 s + 5 s
    assert_eq!(
        media_frames(&b.p, s, ms(3500)),
        vec![e("a.mp4", ms(15_000))]
    );
    // no clip there
    assert!(media_frames(&b.p, s, ms(500)).is_empty());
    assert!(media_frames(&b.p, s, ms(9000)).is_empty());
}

#[test]
fn v11_reversed_clip() {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v = b.vtrack(s);
    let m = b.media("a.mp4", Some((64, 64)), false);
    let c = b.media_clip(v, 0, ms(4000), m);
    b.clip_mut(c).source_in = ms(1000);
    b.clip_mut(c).reverse = true;
    // Q3: 1 s + (4 s - 1 s) = 4 s
    assert_eq!(media_frames(&b.p, s, ms(1000)), vec![e("a.mp4", ms(4000))]);
}

#[test]
fn v11_hidden_track_and_disabled_clip_contribute_nothing() {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v0 = b.vtrack(s);
    let v1 = b.vtrack(s);
    let v2 = b.vtrack(s);
    let m = b.media("a.mp4", Some((64, 64)), true);
    b.media_clip(v0, 0, ms(4000), m);
    b.track_mut(v0).hidden = true;
    let c = b.media_clip(v1, 0, ms(4000), m);
    b.clip_mut(c).enabled = false;
    let m2 = b.media("b.mp4", Some((64, 64)), true);
    b.media_clip(v2, 0, ms(4000), m2);
    assert_eq!(media_frames(&b.p, s, ms(1000)), vec![e("b.mp4", ms(1000))]);
}

#[test]
fn v11_text_solid_adjustment_and_no_video_media_contribute_nothing() {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let tracks: Vec<Id> = (0..5).map(|_| b.vtrack(s)).collect();
    b.solid(tracks[0], 0, ms(4000), RED);
    b.adjustment(tracks[1], 0, ms(4000));
    b.clip(
        tracks[2],
        0,
        ms(4000),
        ClipSource::Text {
            text: TextSpec {
                content: "hi".into(),
                font: String::new(),
                size: 12.0,
                color: WHITE,
                align: TextAlign::Center,
                bold: false,
                italic: false,
            },
        },
    );
    let wav = b.media("a.wav", None, true);
    b.media_clip(tracks[3], 0, ms(4000), wav);
    let m = b.media("v.mp4", Some((64, 64)), true);
    b.media_clip(tracks[4], 0, ms(4000), m);
    assert_eq!(media_frames(&b.p, s, ms(1000)), vec![e("v.mp4", ms(1000))]);
}

#[test]
fn v11_missing_media_and_missing_sequence_references_contribute_nothing() {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v0 = b.vtrack(s);
    let v1 = b.vtrack(s);
    b.media_clip(v0, 0, ms(4000), 9999);
    b.seq_clip(v1, 0, ms(4000), 8888);
    assert!(media_frames(&b.p, s, ms(1000)).is_empty());
}

#[test]
fn v11_unknown_sequence_id_is_empty() {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v = b.vtrack(s);
    let m = b.media("a.mp4", Some((64, 64)), true);
    b.media_clip(v, 0, ms(4000), m);
    assert!(media_frames(&b.p, s + 1000, ms(1000)).is_empty());
}

#[test]
fn v11_two_tracks_bottom_first() {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v0 = b.vtrack(s);
    let v1 = b.vtrack(s);
    let ma = b.media("a.mp4", Some((64, 64)), true);
    let mb = b.media("b.mp4", Some((64, 64)), true);
    // Top track filled first on purpose: order follows `tracks`, not insertion.
    let cb = b.media_clip(v1, 0, ms(4000), mb);
    b.clip_mut(cb).source_in = ms(3000);
    b.media_clip(v0, 0, ms(4000), ma);
    assert_eq!(
        media_frames(&b.p, s, ms(1000)),
        vec![e("a.mp4", ms(1000)), e("b.mp4", ms(4000))]
    );
}

#[test]
fn v11_duplicates_removed_but_different_times_kept() {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v0 = b.vtrack(s);
    let v1 = b.vtrack(s);
    let v2 = b.vtrack(s);
    let m = b.media("a.mp4", Some((64, 64)), true);
    b.media_clip(v0, 0, ms(4000), m);
    b.media_clip(v1, 0, ms(4000), m); // same path, same time as v0
    let c = b.media_clip(v2, 0, ms(4000), m);
    b.clip_mut(c).source_in = ms(5000); // same path, other time
    assert_eq!(
        media_frames(&b.p, s, ms(1000)),
        vec![e("a.mp4", ms(1000)), e("a.mp4", ms(6000))]
    );
}

#[test]
fn v11_nested_sequence_maps_time_through_clip() {
    let mut b = B::new();
    let inner = b.seq(32, 32);
    let iv = b.vtrack(inner);
    let n = b.media("n.mp4", Some((32, 32)), true);
    let ic = b.media_clip(iv, 0, ms(20_000), n);
    b.clip_mut(ic).source_in = ms(5000);

    let outer = b.seq(64, 64);
    let ov0 = b.vtrack(outer);
    let ov1 = b.vtrack(outer);
    let o = b.media("o.mp4", Some((64, 64)), true);
    b.media_clip(ov0, 0, ms(20_000), o);
    let sc = b.seq_clip(ov1, ms(2000), ms(10_000), inner);
    b.clip_mut(sc).source_in = ms(1000);

    // outer 3 s: nested time = 1 s + (3 s - 2 s) = 2 s; inner clip: 5 s + 2 s = 7 s
    assert_eq!(
        media_frames(&b.p, outer, ms(3000)),
        vec![e("o.mp4", ms(3000)), e("n.mp4", ms(7000))]
    );
    // before the nested clip starts only the bottom track is drawn
    assert_eq!(
        media_frames(&b.p, outer, ms(1000)),
        vec![e("o.mp4", ms(1000))]
    );
    // the inner sequence on its own
    assert_eq!(
        media_frames(&b.p, inner, ms(2000)),
        vec![e("n.mp4", ms(7000))]
    );
}

#[test]
fn v11_nested_sequence_with_speed() {
    let mut b = B::new();
    let inner = b.seq(32, 32);
    let iv = b.vtrack(inner);
    let n = b.media("n.mp4", Some((32, 32)), true);
    b.media_clip(iv, 0, ms(60_000), n);
    let outer = b.seq(64, 64);
    let ov = b.vtrack(outer);
    let sc = b.seq_clip(ov, ms(1000), ms(10_000), inner);
    b.clip_mut(sc).speed = 2.0;
    // outer 3 s: local 2 s -> nested 4 s -> n.mp4 at 4 s
    assert_eq!(
        media_frames(&b.p, outer, ms(3000)),
        vec![e("n.mp4", ms(4000))]
    );
}

fn transition_project() -> (B, Id) {
    let mut b = B::new();
    let s = b.seq(64, 64);
    let v = b.vtrack(s);
    let ml = b.media("l.mp4", Some((64, 64)), true);
    let mr = b.media("r.mp4", Some((64, 64)), true);
    let l = b.media_clip(v, 0, ms(2000), ml);
    b.clip_mut(l).source_in = ms(10_000);
    let r = b.media_clip(v, ms(2000), ms(2000), mr);
    b.clip_mut(r).source_in = ms(20_000);
    // two-sided 1 s: window [1.5 s, 2.5 s)
    b.transition(v, Some(l), Some(r), "cross_dissolve", ms(1000), &[]);
    (b, s)
}

#[test]
fn v11_transition_window_lists_both_sides_left_then_right() {
    let (b, s) = transition_project();
    // 1.75 s: left 10 + 1.75; right continues before its start: 20 + (1.75 - 2) = 19.75
    assert_eq!(
        media_frames(&b.p, s, ms(1750)),
        vec![e("l.mp4", ms(11_750)), e("r.mp4", ms(19_750))]
    );
    // 2.25 s: left continues past its end: 10 + 2.25; right 20 + 0.25
    assert_eq!(
        media_frames(&b.p, s, ms(2250)),
        vec![e("l.mp4", ms(12_250)), e("r.mp4", ms(20_250))]
    );
    // window start is inclusive
    assert_eq!(
        media_frames(&b.p, s, ms(1500)),
        vec![e("l.mp4", ms(11_500)), e("r.mp4", ms(19_500))]
    );
}

#[test]
fn v11_outside_transition_window_only_the_clip_at_time() {
    let (b, s) = transition_project();
    assert_eq!(
        media_frames(&b.p, s, ms(1400)),
        vec![e("l.mp4", ms(11_400))]
    );
    // window end is exclusive
    assert_eq!(
        media_frames(&b.p, s, ms(2500)),
        vec![e("r.mp4", ms(20_500))]
    );
    assert_eq!(
        media_frames(&b.p, s, ms(3000)),
        vec![e("r.mp4", ms(21_000))]
    );
}

#[test]
fn v11_transition_with_disabled_side_skips_that_side() {
    let (mut b, s) = transition_project();
    let r = b.p.sequences[0].tracks[0].clips[1].id;
    b.clip_mut(r).enabled = false;
    assert_eq!(
        media_frames(&b.p, s, ms(1750)),
        vec![e("l.mp4", ms(11_750))]
    );
}
