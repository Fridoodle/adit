//! Audio mix (SPEC A-1..A-7): pure CPU, interleaved stereo f32.

use crate::RenderError;
use crate::compose::{param, window};
use adit_core::*;
use adit_media::{MediaBackend, MediaError};
use std::f64::consts::FRAC_PI_2;

const S: i128 = FLICKS_PER_SECOND as i128;

/// round(d * sr / s), half away from zero; 0 for d <= 0.
fn frame_count(d: Time, sr: u32) -> usize {
    let d = i128::from(d.max(0));
    ((2 * d * i128::from(sr) + S) / (2 * S)) as usize
}

/// First sample frame at or after offset `t` (flicks, >= 0).
fn frame_at(t: Time, sr: u32) -> usize {
    (i128::from(t) * i128::from(sr) + S - 1)
        .div_euclid(S)
        .max(0) as usize
}

/// Offset in flicks of sample frame `i`.
fn time_of(i: usize, sr: u32) -> Time {
    (i as i128 * S / i128::from(sr)) as Time
}

/// ffmpeg filter chain for a media clip (A-2).
fn filter_chain(clip: &Clip) -> String {
    let mut steps = Vec::new();
    if clip.reverse {
        steps.push("areverse".to_string());
    }
    // atempo accepts 0.5..100 per instance; chain steps for anything outside.
    let mut s = clip.speed;
    if s.is_finite() && s > 0.0 {
        while s > 100.0 {
            steps.push("atempo=100".into());
            s /= 100.0;
        }
        while s < 0.5 {
            steps.push("atempo=0.5".into());
            s /= 0.5;
        }
        if s != 1.0 {
            // `{}` prints the shortest string that parses back to the same f64.
            steps.push(format!("atempo={s}"));
        }
    }
    if !clip.audio_filter.is_empty() {
        steps.push(clip.audio_filter.clone());
    }
    steps.join(",")
}

/// Source range `[ss, se)` played over sequence range `[a, b)` (Q3 mapping).
fn source_range(clip: &Clip, a: Time, b: Time) -> (Time, Time) {
    let at = |t: Time| clip.source_in + ((t - clip.start) as f64 * clip.speed).round() as Time;
    if clip.reverse {
        let back =
            |t: Time| clip.source_in + ((clip.end() - t) as f64 * clip.speed).round() as Time;
        (back(b), back(a))
    } else {
        (at(a), at(b))
    }
}

/// The clip's raw audio for sequence range `[a, b)`, starting at frame `a`; may be shorter or
/// longer than the range (the caller cuts and pads). `None` = silence.
fn fetch(
    project: &Project,
    clip: &Clip,
    a: Time,
    b: Time,
    sr: u32,
    media: &dyn MediaBackend,
    depth: usize,
) -> Option<Vec<f32>> {
    match &clip.source {
        ClipSource::Media { media_id } => {
            let m = project.media.iter().find(|m| m.id == *media_id)?;
            let chain = filter_chain(clip);
            let get = |a: Time, b: Time| {
                let (ss, se) = source_range(clip, a, b);
                media.audio(&m.spec.path, ss, se - ss, sr, &chain)
            };
            // A backend yields less, not padding, for source time outside the file (before 0, or
            // past the end for a reversed clip, which comes out first). That leading part is
            // fetched on its own and right-aligned so the rest still lines up with `a`.
            let (ss, se) = source_range(clip, a, b);
            let over = if clip.reverse {
                m.spec.duration.map_or(0, |d| (se - d).max(0))
            } else {
                (-ss).max(0)
            };
            let skip = (over as f64 / clip.speed).round() as Time;
            let lead = frame_at(skip, sr).min(frame_at(b - a, sr));
            let a2 = a + time_of(lead, sr);
            let mut v = Vec::new();
            if lead > 0 {
                let got = get(a, a2).unwrap_or_default();
                let got = &got[got.len().saturating_sub(2 * lead)..];
                v.resize(2 * lead - got.len(), 0.0);
                v.extend_from_slice(got);
            }
            if a2 < b {
                v.extend(get(a2, b).ok()?);
            }
            Some(v)
        }
        ClipSource::Sequence { sequence_id } => {
            // validate() caps nesting (H-4); an unvalidated project is cut off here, silent.
            if depth >= MAX_NEST_DEPTH {
                return None;
            }
            // A-6 shortcut: nested audio ignores speed and reverse.
            let nested = project.sequence(*sequence_id)?;
            let from = clip.source_in + (a - clip.start);
            let v = mix_seq(project, nested, from, b - a, media, depth + 1).ok()?;
            Some(resample(&v, nested.sample_rate, sr))
        }
        _ => None,
    }
}

pub(crate) fn mix_audio(
    project: &Project,
    sequence_id: Id,
    start: Time,
    duration: Time,
    media: &dyn MediaBackend,
) -> Result<Vec<f32>, RenderError> {
    let seq = project
        .sequence(sequence_id)
        .ok_or(RenderError::SequenceNotFound(sequence_id))?;
    // H-5: the buffer is sized from `duration`.
    if !(0..=MAX_TIME).contains(&duration) {
        return Err(MediaError::Unsupported(format!(
            "audio duration {duration} flicks outside 0..={MAX_TIME}"
        ))
        .into());
    }
    mix_seq(project, seq, start, duration, media, 1)
}

/// `depth` = nesting depth of `seq` (1 = the sequence asked for).
fn mix_seq(
    project: &Project,
    seq: &Sequence,
    start: Time,
    duration: Time,
    media: &dyn MediaBackend,
    depth: usize,
) -> Result<Vec<f32>, RenderError> {
    let sr = seq.sample_rate;
    let n = frame_count(duration, sr);
    // Fallible: MAX_TIME at a high sample rate is still far more than any machine has.
    let mut out = Vec::new();
    n.checked_mul(2)
        .and_then(|len| out.try_reserve_exact(len).ok())
        .ok_or_else(|| MediaError::Unsupported(format!("cannot allocate {n} audio frames")))?;
    out.resize(2 * n, 0.0f32);
    let end = start.saturating_add(duration.max(0));
    let audio = || seq.tracks.iter().filter(|t| t.kind == TrackKind::Audio);
    let solo = audio().any(|t| t.solo);
    for track in audio().filter(|t| !t.muted && (!solo || t.solo)) {
        for clip in track.clips.iter().filter(|c| c.enabled) {
            // A-5: (window start, window end, fading out) for each transition the clip is a side of.
            let mut fades = Vec::new();
            let (mut lo, mut hi) = (clip.start, clip.end());
            for tr in &track.transitions {
                let Some((ws, l, r)) = window(track, tr) else {
                    continue;
                };
                for (side, out) in [(l, true), (r, false)] {
                    if side.is_some_and(|c| c.id == clip.id) {
                        fades.push((ws, ws + tr.duration, out));
                        lo = lo.min(ws);
                        hi = hi.max(ws + tr.duration);
                    }
                }
            }
            let (a, b) = (lo.max(start), hi.min(end));
            if a >= b {
                continue;
            }
            let first = frame_at(a - start, sr);
            let last = frame_at(b - start, sr).min(n);
            if first >= last {
                continue;
            }
            let (a, b) = (start + time_of(first, sr), start + time_of(last, sr));
            let Some(src) = fetch(project, clip, a, b, sr, media, depth) else {
                continue;
            };
            for k in 0..last - first {
                let i = first + k;
                let t = start + time_of(i, sr);
                let mut g = if clip.start <= t && t < clip.end() {
                    1.0
                } else {
                    0.0
                };
                for &(ws, we, fade_out) in &fades {
                    if ws <= t && t < we {
                        let p = (t - ws) as f64 / (we - ws) as f64 * FRAC_PI_2;
                        g = if fade_out { p.cos() } else { p.sin() };
                    }
                }
                if g == 0.0 {
                    continue;
                }
                let local = t - clip.start;
                let g = g * 10f64.powf(param(clip, "volume", local) / 20.0);
                let pan = param(clip, "pan", local);
                let (l, r) = (
                    src.get(2 * k).copied().unwrap_or(0.0),
                    src.get(2 * k + 1).copied().unwrap_or(0.0),
                );
                out[2 * i] += (f64::from(l) * g * (1.0 - pan).min(1.0)) as f32;
                out[2 * i + 1] += (f64::from(r) * g * (1.0 + pan).min(1.0)) as f32;
            }
        }
    }
    Ok(out)
}

/// Linear resampling of interleaved stereo.
pub(crate) fn resample(v: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || from == 0 || to == 0 {
        return v.to_vec();
    }
    let n_in = v.len() / 2;
    let n_out =
        ((n_in as u128 * u128::from(to) * 2 + u128::from(from)) / (u128::from(from) * 2)) as usize;
    let mut out = Vec::with_capacity(2 * n_out);
    for j in 0..n_out {
        let pos = j as f64 * f64::from(from) / f64::from(to);
        let i = pos.floor() as usize;
        let f = (pos - i as f64) as f32;
        for ch in 0..2 {
            let x0 = v.get(2 * i + ch).copied().unwrap_or(0.0);
            let x1 = v.get(2 * (i + 1) + ch).copied().unwrap_or(x0);
            out.push(x0 + (x1 - x0) * f);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atempo_steps_multiply_back() {
        let mut c = Clip {
            id: 1,
            name: String::new(),
            start: 0,
            duration: 1,
            source: ClipSource::Adjustment,
            source_in: 0,
            speed: 0.1,
            reverse: true,
            enabled: true,
            link: None,
            group: None,
            blend: BlendMode::Normal,
            audio_filter: "volume=2".into(),
            params: Default::default(),
            effects: vec![],
            label: 0,
        };
        assert_eq!(
            filter_chain(&c),
            "areverse,atempo=0.5,atempo=0.5,atempo=0.5,atempo=0.8,volume=2"
        );
        c.speed = 250.0;
        c.reverse = false;
        c.audio_filter.clear();
        assert_eq!(filter_chain(&c), "atempo=100,atempo=2.5");
    }

    #[test]
    fn resample_lengths_and_values() {
        let v = [0.0f32, 1.0, 1.0, 1.0].repeat(24_000); // 48000 frames
        let r = resample(&v, 48_000, 44_100);
        assert_eq!(r.len(), 2 * 44_100);
        assert!(r.iter().skip(1).step_by(2).all(|x| (*x - 1.0).abs() < 1e-6));
    }
}
