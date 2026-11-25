//! `Project::validate` against hand-built projects (I1-I6).
mod common;
use common::*;

#[track_caller]
fn valid(p: &Project) {
    if let Err(why) = p.validate() {
        panic!("expected valid, got Err({why})");
    }
}

#[track_caller]
fn invalid(p: &Project) {
    assert!(
        p.validate().is_err(),
        "expected validate() to reject the project"
    );
}

/// Uses every kind of entity once: nested sequence, text/solid/adjustment, still, link, group,
/// effect with keyframes, markers, transition, effect def.
fn rich() -> Fx {
    let Base {
        mut fx,
        m,
        still,
        seq,
        v1,
        v2,
        a1,
        ..
    } = base();
    let inner = fx.seq("Inner");
    let iv = fx.track(inner, TrackKind::Video);
    fx.clip(iv, m, 0, 2 * S, 0);
    let p = fx.clip(v1, m, 0, 2 * S, 0);
    let q = fx.clip(v1, m, 2 * S, 2 * S, 10 * S);
    fx.transition(v1, Some(p), Some(q), S);
    fx.clip_src(v2, seq_src(inner), 0, 5 * S, S);
    fx.clip_src(v2, text_src("t"), 5 * S, S, 0);
    fx.clip_src(v2, solid_src(), 6 * S, S, 0);
    fx.clip_src(v2, ClipSource::Adjustment, 7 * S, S, 0);
    fx.clip(v2, still, 8 * S, 500 * S, 0);
    let pa = fx.clip(a1, m, 0, 2 * S, 0);
    fx.clip_src(a1, seq_src(inner), 2 * S, S, 0);
    fx.link(&[p, pa]);
    let g = fx.id();
    fx.c(q).group = Some(g);
    fx.c(p).params.insert(
        "opacity".into(),
        param(
            1.0,
            vec![kf(-S, 0.0, Easing::Linear), kf(S, 1.0, Easing::EaseIn)],
        ),
    );
    fx.effect(
        p,
        "blur",
        &[("amount", param(3.0, vec![kf(0, 1.0, Easing::Hold)]))],
    );
    fx.marker(seq, 0, 0);
    fx.marker(seq, 0, S);
    fx.marker(seq, 3 * S, 0);
    fx.p.effects.push(EffectDef {
        name: "glow".into(),
        kind: EffectKind::Filter,
        wgsl: "fn main() {}".into(),
        params: vec![ParamDef {
            name: "k".into(),
            default: 0.5,
            min: 0.0,
            max: 1.0,
        }],
    });
    fx
}

#[test]
fn rich_project_is_valid() {
    valid(&rich().p);
}

// --- I1 ---

#[test]
fn i1_touching_clips_are_fine_half_open() {
    let Base { mut fx, m, v1, .. } = base();
    fx.clip(v1, m, 0, 2 * S, 0);
    fx.clip(v1, m, 2 * S, S, 0);
    valid(&fx.p);
}

#[test]
fn i1_overlap_rejected() {
    let Base { mut fx, m, v1, .. } = base();
    fx.clip(v1, m, 0, 2 * S, 0);
    fx.clip(v1, m, 2 * S - 1, S, 0);
    invalid(&fx.p);
}

#[test]
fn i1_unsorted_rejected() {
    let Base { mut fx, m, v1, .. } = base();
    fx.clip(v1, m, 0, S, 0);
    fx.clip(v1, m, 2 * S, S, 0);
    fx.t(v1).clips.swap(0, 1);
    invalid(&fx.p);
}

#[test]
fn i1_negative_start_rejected() {
    let Base { mut fx, m, v1, .. } = base();
    let c = fx.clip(v1, m, 0, S, 0);
    fx.c(c).start = -1;
    invalid(&fx.p);
}

#[test]
fn i1_zero_and_negative_duration_rejected() {
    for d in [0, -S] {
        let Base { mut fx, m, v1, .. } = base();
        let c = fx.clip(v1, m, 0, S, 0);
        fx.c(c).duration = d;
        invalid(&fx.p);
    }
}

// --- I2 ---

#[test]
fn i2_id_not_below_next_id_rejected() {
    let Base { mut fx, m, v1, .. } = base();
    let c = fx.clip(v1, m, 0, S, 0);
    fx.p.next_id = c; // the clip id is no longer < next_id
    invalid(&fx.p);
}

#[test]
fn i2_duplicate_clip_ids_rejected() {
    let Base {
        mut fx, m, v1, v2, ..
    } = base();
    let c = fx.clip(v1, m, 0, S, 0);
    let d = fx.clip(v2, m, 0, S, 0);
    fx.c(d).id = c;
    invalid(&fx.p);
}

#[test]
fn i2_ids_unique_across_entity_kinds() {
    // clip id equal to a track id
    let Base {
        mut fx, m, v1, v2, ..
    } = base();
    let c = fx.clip(v1, m, 0, S, 0);
    fx.c(c).id = v2;
    invalid(&fx.p);
    // marker id equal to a media id
    let Base { mut fx, m, seq, .. } = base();
    let mk = fx.marker(seq, 0, 0);
    fx.seq_mut(seq)
        .markers
        .iter_mut()
        .find(|x| x.id == mk)
        .unwrap()
        .id = m;
    invalid(&fx.p);
    // effect instance id equal to its clip id
    let Base { mut fx, m, v1, .. } = base();
    let c = fx.clip(v1, m, 0, S, 0);
    fx.effect(c, "blur", &[]);
    fx.c(c).effects[0].id = c;
    invalid(&fx.p);
    // transition id equal to a sequence id
    let Base {
        mut fx, m, seq, v1, ..
    } = base();
    let c = fx.clip(v1, m, 0, S, 0);
    fx.transition(v1, Some(c), None, S / 2);
    fx.t(v1).transitions[0].id = seq;
    invalid(&fx.p);
}

#[test]
fn i2_link_and_group_ids() {
    // shared link id is fine (also in rich()), but it must not equal an entity id
    let Base {
        mut fx, m, v1, a1, ..
    } = base();
    let c = fx.clip(v1, m, 0, S, 0);
    let d = fx.clip(a1, m, 0, S, 0);
    fx.link(&[c, d]);
    valid(&fx.p);
    fx.c(c).link = Some(m);
    fx.c(d).link = Some(m);
    invalid(&fx.p);

    let Base { mut fx, m, v1, .. } = base();
    let c = fx.clip(v1, m, 0, S, 0);
    fx.c(c).link = Some(fx.p.next_id);
    invalid(&fx.p);

    let Base { mut fx, m, v1, .. } = base();
    let c = fx.clip(v1, m, 0, S, 0);
    fx.c(c).group = Some(fx.p.next_id + 5);
    invalid(&fx.p);

    let Base { mut fx, m, v1, .. } = base();
    let c = fx.clip(v1, m, 0, S, 0);
    fx.c(c).group = Some(v1);
    invalid(&fx.p);
}

// --- I3 ---

fn short_media() -> (Fx, Id, Id) {
    let Base { mut fx, v1, .. } = base();
    let short = fx.media_full("Short", Some(10 * S), true, true);
    (fx, short, v1)
}

#[test]
fn i3_source_slack_boundary() {
    let (mut fx, short, v1) = short_media();
    let c = fx.clip(v1, short, 0, 10 * S, 0);
    valid(&fx.p);
    fx.c(c).source_in = SOURCE_SLACK; // 10 s + 2 <= 10 s + 2
    valid(&fx.p);
    fx.c(c).source_in = SOURCE_SLACK + 1;
    invalid(&fx.p);
}

#[test]
fn i3_speed_scales_source_use() {
    let (mut fx, short, v1) = short_media();
    let c = fx.clip(v1, short, 0, 5 * S, 0);
    fx.c(c).speed = 2.0; // uses exactly 10 s
    valid(&fx.p);
    fx.c(c).source_in = 3;
    invalid(&fx.p);
}

#[test]
fn i3_rounds_duration_times_speed() {
    let Base { mut fx, v1, .. } = base();
    let tiny = fx.media_full("Tiny", Some(3), true, false);
    let c = fx.clip(v1, tiny, 0, 3, 0);
    fx.c(c).speed = 1.5; // round(4.5) = 5 <= 3 + 2
    valid(&fx.p);
    fx.p.media
        .iter_mut()
        .find(|x| x.id == tiny)
        .unwrap()
        .spec
        .duration = Some(2); // 5 > 4
    invalid(&fx.p);
}

#[test]
fn i3_negative_source_in_rejected_for_every_clip_kind() {
    let Base { mut fx, m, v1, .. } = base();
    let c = fx.clip(v1, m, 0, S, 0);
    fx.c(c).source_in = -1;
    invalid(&fx.p);
    let Base { mut fx, v1, .. } = base();
    let t = fx.text(v1, 0, S);
    fx.c(t).source_in = -1;
    invalid(&fx.p);
}

#[test]
fn i3_still_is_unbounded() {
    let Base {
        mut fx, still, v1, ..
    } = base();
    fx.clip(v1, still, 0, 10_000 * S, 5_000 * S);
    valid(&fx.p);
}

#[test]
fn i3_bad_speed_rejected() {
    for speed in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let Base { mut fx, m, v1, .. } = base();
        let c = fx.clip(v1, m, 0, S, 0);
        fx.c(c).speed = speed;
        invalid(&fx.p);
        let Base { mut fx, v1, .. } = base();
        let t = fx.text(v1, 0, S);
        fx.c(t).speed = speed;
        invalid(&fx.p);
    }
}

// --- I4 ---

type SrcFn = fn(&Base) -> ClipSource;

#[test]
fn i4_track_kinds() {
    let ok_cases: Vec<(bool, SrcFn)> = vec![
        (true, |b| media_src(b.m)),
        (true, |b| media_src(b.vid)),
        (true, |b| media_src(b.still)),
        (true, |_| text_src("x")),
        (true, |_| solid_src()),
        (true, |_| ClipSource::Adjustment),
        (false, |b| media_src(b.m)),
        (false, |b| media_src(b.aud)),
    ];
    for (video, f) in ok_cases {
        let mut b = base();
        let src = f(&b);
        let t = if video { b.v1 } else { b.a1 };
        b.fx.clip_src(t, src, 0, S, 0);
        valid(&b.fx.p);
    }
    let bad_cases: Vec<(bool, SrcFn)> = vec![
        (true, |b| media_src(b.aud)),
        (false, |b| media_src(b.vid)),
        (false, |b| media_src(b.still)),
        (false, |_| text_src("x")),
        (false, |_| solid_src()),
        (false, |_| ClipSource::Adjustment),
    ];
    for (video, f) in bad_cases {
        let mut b = base();
        let src = f(&b);
        let t = if video { b.v1 } else { b.a1 };
        b.fx.clip_src(t, src, 0, S, 0);
        invalid(&b.fx.p);
    }
}

#[test]
fn i4_sequence_clip_on_both_kinds() {
    let Base {
        mut fx, m, v1, a1, ..
    } = base();
    let inner = fx.seq("Inner");
    let it = fx.track(inner, TrackKind::Audio);
    fx.clip(it, m, 0, S, 0);
    fx.clip_src(v1, seq_src(inner), 0, S, 0);
    fx.clip_src(a1, seq_src(inner), 0, S, 0);
    valid(&fx.p);
}

// --- I5 ---

/// V1: P [0, pl), Q [pl, pl+ql).
fn pq(pl: Time, ql: Time) -> (Fx, Id, Id, Id, Id) {
    let Base {
        mut fx, m, v1, v2, ..
    } = base();
    let p = fx.clip(v1, m, 0, pl, 0);
    let q = fx.clip(v1, m, pl, ql, 0);
    (fx, v1, v2, p, q)
}

#[test]
fn i5_valid_two_sided_and_one_sided() {
    let (mut fx, v1, _, p, q) = pq(2 * S, 2 * S);
    fx.transition(v1, Some(p), Some(q), S);
    valid(&fx.p);
    let (mut fx, v1, _, p, _) = pq(2 * S, 2 * S);
    fx.transition(v1, Some(p), None, 2 * S);
    valid(&fx.p);
    let (mut fx, v1, _, _, q) = pq(2 * S, 2 * S);
    fx.transition(v1, None, Some(q), 2 * S);
    valid(&fx.p);
}

#[test]
fn i5_bad_duration_or_no_side() {
    let (mut fx, v1, _, p, q) = pq(2 * S, 2 * S);
    fx.transition(v1, Some(p), Some(q), 0);
    invalid(&fx.p);
    let (mut fx, v1, _, _, _) = pq(2 * S, 2 * S);
    fx.transition(v1, None, None, S);
    invalid(&fx.p);
}

#[test]
fn i5_clip_must_be_on_that_track_and_exist() {
    let (mut fx, v1, v2, _, _) = pq(2 * S, 2 * S);
    let m = fx.p.media[0].id;
    let other = fx.clip(v2, m, 0, 2 * S, 0);
    fx.transition(v1, Some(other), None, S);
    invalid(&fx.p);
    let (mut fx, v1, _, _, _) = pq(2 * S, 2 * S);
    let ghost = fx.id();
    fx.transition(v1, Some(ghost), None, S);
    invalid(&fx.p);
}

#[test]
fn i5_two_sided_needs_adjacent_clips() {
    let Base { mut fx, m, v1, .. } = base();
    let p = fx.clip(v1, m, 0, 2 * S, 0);
    let q = fx.clip(v1, m, 3 * S, 2 * S, 0);
    fx.transition(v1, Some(p), Some(q), S);
    invalid(&fx.p);
}

#[test]
fn i5_two_sided_split_uses_integer_division() {
    // left 1 s, right 2 s: duration/2 must fit left, duration - duration/2 must fit right
    let (mut fx, v1, _, p, q) = pq(S, 2 * S);
    fx.transition(v1, Some(p), Some(q), 2 * S + 1); // S on the left, S+1 on the right
    valid(&fx.p);
    let (mut fx, v1, _, p, q) = pq(S, 2 * S);
    fx.transition(v1, Some(p), Some(q), 2 * S + 2); // S+1 on the left
    invalid(&fx.p);
    // left 2 s, right 1 s
    let (mut fx, v1, _, p, q) = pq(2 * S, S);
    fx.transition(v1, Some(p), Some(q), 2 * S);
    valid(&fx.p);
    let (mut fx, v1, _, p, q) = pq(2 * S, S);
    fx.transition(v1, Some(p), Some(q), 2 * S + 1); // S+1 on the right
    invalid(&fx.p);
}

#[test]
fn i5_one_sided_must_fit_clip() {
    let (mut fx, v1, _, p, _) = pq(2 * S, 2 * S);
    fx.transition(v1, Some(p), None, 2 * S + 1);
    invalid(&fx.p);
    let (mut fx, v1, _, _, q) = pq(2 * S, 2 * S);
    fx.transition(v1, None, Some(q), 2 * S + 1);
    invalid(&fx.p);
}

#[test]
fn i5_one_transition_per_edge() {
    let (mut fx, v1, _, p, q) = pq(2 * S, 2 * S);
    fx.transition(v1, Some(p), Some(q), S);
    fx.transition(v1, Some(p), None, S);
    invalid(&fx.p);
    let (mut fx, v1, _, p, q) = pq(2 * S, 2 * S);
    fx.transition(v1, Some(p), Some(q), S);
    fx.transition(v1, None, Some(q), S);
    invalid(&fx.p);
    // P's start and Q's end are different edges
    let (mut fx, v1, _, p, q) = pq(2 * S, 2 * S);
    fx.transition(v1, Some(p), Some(q), S);
    fx.transition(v1, None, Some(p), S / 2);
    fx.transition(v1, Some(q), None, S / 2);
    valid(&fx.p);
}

// --- I6 ---

#[test]
fn i6_dangling_media_and_sequence_ids() {
    let Base { mut fx, v1, .. } = base();
    fx.clip(v1, 999, 0, S, 0);
    invalid(&fx.p);
    let Base { mut fx, v1, .. } = base();
    fx.clip_src(v1, seq_src(999), 0, S, 0);
    invalid(&fx.p);
}

#[test]
fn i6_nest_cycles() {
    let Base {
        mut fx, seq, v1, ..
    } = base();
    fx.clip_src(v1, seq_src(seq), 0, S, 0);
    invalid(&fx.p);

    let Base {
        mut fx, seq, v1, ..
    } = base();
    let b = fx.seq("B");
    let bt = fx.track(b, TrackKind::Video);
    fx.clip_src(v1, seq_src(b), 0, S, 0);
    fx.clip_src(bt, seq_src(seq), 0, S, 0);
    invalid(&fx.p);

    let Base {
        mut fx, seq, v1, ..
    } = base();
    let b = fx.seq("B");
    let c = fx.seq("C");
    let bt = fx.track(b, TrackKind::Video);
    let ct = fx.track(c, TrackKind::Audio);
    fx.clip_src(v1, seq_src(b), 0, S, 0);
    fx.clip_src(bt, seq_src(c), 0, S, 0);
    fx.clip_src(ct, seq_src(seq), 0, S, 0);
    invalid(&fx.p);

    // a diamond (two paths to the same sequence) is not a cycle
    let Base { mut fx, v1, v2, .. } = base();
    let b = fx.seq("B");
    let c = fx.seq("C");
    let bt = fx.track(b, TrackKind::Video);
    fx.clip_src(v1, seq_src(b), 0, S, 0);
    fx.clip_src(v2, seq_src(c), 0, S, 0);
    fx.clip_src(bt, seq_src(c), 0, S, 0);
    valid(&fx.p);
}

#[test]
fn i6_sequence_format_fields_positive() {
    let edits: Vec<fn(&mut Sequence)> = vec![
        |s| s.width = 0,
        |s| s.height = 0,
        |s| s.sample_rate = 0,
        |s| s.fps.num = 0,
        |s| s.fps.den = 0,
    ];
    for e in edits {
        let Base { mut fx, seq, .. } = base();
        e(fx.seq_mut(seq));
        invalid(&fx.p);
    }
}

#[test]
fn i6_markers() {
    let Base { mut fx, seq, .. } = base();
    fx.marker(seq, 2 * S, 0);
    fx.marker(seq, S, 0);
    invalid(&fx.p);
    let Base { mut fx, seq, .. } = base();
    fx.marker(seq, -1, 0);
    invalid(&fx.p);
    let Base { mut fx, seq, .. } = base();
    fx.marker(seq, 0, -1);
    invalid(&fx.p);
    let Base { mut fx, seq, .. } = base();
    fx.marker(seq, S, 0);
    fx.marker(seq, S, 0);
    valid(&fx.p);
}

#[test]
fn i6_keyframes_sorted_unique() {
    let Base { mut fx, m, v1, .. } = base();
    let c = fx.clip(v1, m, 0, S, 0);
    fx.c(c).params.insert(
        "opacity".into(),
        param(
            1.0,
            vec![kf(S, 0.0, Easing::Linear), kf(0, 1.0, Easing::Linear)],
        ),
    );
    invalid(&fx.p);
    let Base { mut fx, m, v1, .. } = base();
    let c = fx.clip(v1, m, 0, S, 0);
    fx.c(c).params.insert(
        "opacity".into(),
        param(
            1.0,
            vec![kf(0, 0.0, Easing::Linear), kf(0, 1.0, Easing::Linear)],
        ),
    );
    invalid(&fx.p);
    // effect params too
    let Base { mut fx, m, v1, .. } = base();
    let c = fx.clip(v1, m, 0, S, 0);
    fx.effect(
        c,
        "blur",
        &[(
            "amount",
            param(
                0.0,
                vec![kf(S, 0.0, Easing::Linear), kf(0, 1.0, Easing::Linear)],
            ),
        )],
    );
    invalid(&fx.p);
}

#[test]
fn i6_every_number_finite() {
    let edits: Vec<fn(&mut Fx, Id, Id)> = vec![
        |fx, c, _| {
            fx.c(c)
                .params
                .insert("opacity".into(), param(f64::NAN, vec![]));
        },
        |fx, c, _| {
            fx.c(c).params.insert(
                "opacity".into(),
                param(1.0, vec![kf(0, f64::INFINITY, Easing::Linear)]),
            );
        },
        |fx, c, _| {
            fx.effect(c, "blur", &[("amount", param(f64::NAN, vec![]))]);
        },
        |fx, c, v1| {
            fx.transition(v1, Some(c), None, S / 2);
            fx.t(v1).transitions[0]
                .params
                .insert("k".into(), f64::NEG_INFINITY);
        },
        |fx, _, v1| {
            fx.clip_src(
                v1,
                ClipSource::Solid {
                    color: [f32::NAN, 0.0, 0.0, 1.0],
                },
                5 * S,
                S,
                0,
            );
        },
        |fx, _, v1| {
            let mut t = text_spec("x");
            t.size = f64::NAN;
            fx.clip_src(v1, ClipSource::Text { text: t }, 5 * S, S, 0);
        },
        |fx, _, _| {
            fx.p.effects.push(EffectDef {
                name: "e".into(),
                kind: EffectKind::Filter,
                wgsl: String::new(),
                params: vec![ParamDef {
                    name: "k".into(),
                    default: f64::NAN,
                    min: 0.0,
                    max: 1.0,
                }],
            });
        },
    ];
    for e in edits {
        let Base { mut fx, m, v1, .. } = base();
        let c = fx.clip(v1, m, 0, S, 0);
        e(&mut fx, c, v1);
        invalid(&fx.p);
    }
}
