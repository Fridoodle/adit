//! Command execution on a scratch copy of the project. `Document::apply` owns atomicity, pruning,
//! validation and history; everything here may leave the project half-edited on `Err`.

use crate::*;
use std::collections::{BTreeMap, HashMap, HashSet};

type R<T = ()> = Result<T, Error>;

fn nf<T>(what: &'static str, id: Id) -> R<T> {
    Err(Error::NotFound { what, id })
}

fn bad<T>(why: &str) -> R<T> {
    Err(Error::InvalidArgument(why.to_string()))
}

fn fin(x: f64) -> R {
    if x.is_finite() {
        Ok(())
    } else {
        bad("non-finite number")
    }
}

/// H-3: a time, duration or source position given in a command.
fn upto(t: Time) -> R {
    if t > MAX_TIME {
        bad("time beyond MAX_TIME")
    } else {
        Ok(())
    }
}

/// H-3: a keyframe time or delta given in a command.
fn within(t: Time) -> R {
    if (-MAX_TIME..=MAX_TIME).contains(&t) {
        Ok(())
    } else {
        bad("time beyond ±MAX_TIME")
    }
}

/// H-3: `start + duration` without overflow.
pub(crate) fn end_ok(c: &Clip) -> bool {
    c.start
        .checked_add(c.duration)
        .is_some_and(|e| e <= MAX_TIME)
}

fn alloc(p: &mut Project) -> Id {
    p.next_id += 1;
    p.next_id - 1
}

fn rnd(t: Time, speed: f64) -> Time {
    (t as f64 * speed).round() as Time
}

// ---------------------------------------------------------------------------------------------
// Lookup

fn seq_ix(p: &Project, id: Id) -> R<usize> {
    p.sequences
        .iter()
        .position(|s| s.id == id)
        .map_or_else(|| nf("sequence", id), Ok)
}

fn track_ix(p: &Project, id: Id) -> R<(usize, usize)> {
    for (si, s) in p.sequences.iter().enumerate() {
        if let Some(ti) = s.tracks.iter().position(|t| t.id == id) {
            return Ok((si, ti));
        }
    }
    nf("track", id)
}

fn clip_ix(p: &Project, id: Id) -> R<(usize, usize, usize)> {
    for (si, s) in p.sequences.iter().enumerate() {
        for (ti, t) in s.tracks.iter().enumerate() {
            if let Some(ci) = t.clips.iter().position(|c| c.id == id) {
                return Ok((si, ti, ci));
            }
        }
    }
    nf("clip", id)
}

fn clip_ref(p: &Project, id: Id) -> R<&Clip> {
    let (s, t, c) = clip_ix(p, id)?;
    Ok(&p.sequences[s].tracks[t].clips[c])
}

fn clip_mut(p: &mut Project, id: Id) -> R<&mut Clip> {
    let (s, t, c) = clip_ix(p, id)?;
    Ok(&mut p.sequences[s].tracks[t].clips[c])
}

/// R6, plus `ids` themselves; deduplicated, `ids` first. Errors if any id is unknown.
fn expand(p: &Project, ids: &[Id], linked: bool) -> R<Vec<Id>> {
    let mut out: Vec<Id> = Vec::new();
    for &id in ids {
        let (si, _, _) = clip_ix(p, id)?;
        let mut add = vec![id];
        if let (true, Some(l)) = (linked, clip_ref(p, id)?.link) {
            let s = &p.sequences[si];
            add.extend(
                s.tracks
                    .iter()
                    .flat_map(|t| &t.clips)
                    .filter(|c| c.link == Some(l))
                    .map(|c| c.id),
            );
        }
        for a in add {
            if !out.contains(&a) {
                out.push(a);
            }
        }
    }
    Ok(out)
}

fn unlocked(p: &Project, id: Id) -> R {
    let (si, ti) = track_ix(p, id)?;
    if p.sequences[si].tracks[ti].locked {
        Err(Error::TrackLocked { track_id: id })
    } else {
        Ok(())
    }
}

fn has_source_time(c: &Clip) -> bool {
    matches!(
        c.source,
        ClipSource::Media { .. } | ClipSource::Sequence { .. }
    )
}

// ---------------------------------------------------------------------------------------------
// Primitives (SPEC R1-R7)

fn head_trim(c: &mut Clip, h: Time) {
    c.start += h;
    c.duration -= h;
    let effect_params = c.effects.iter_mut().flat_map(|e| e.params.values_mut());
    for k in c
        .params
        .values_mut()
        .chain(effect_params)
        .flat_map(|pm| pm.keyframes.iter_mut())
    {
        // Saturating: ripple-start trims move keyframes without bound; validate() rejects H-3.
        k.time = k.time.saturating_sub(h);
    }
    if has_source_time(c) && !c.reverse {
        // rnd saturates at speed 100; the step check rejects the result (H-3).
        c.source_in = c.source_in.saturating_add(rnd(h, c.speed));
    }
}

fn tail_trim(c: &mut Clip, t: Time) {
    c.duration -= t;
    if has_source_time(c) && c.reverse {
        c.source_in = c.source_in.saturating_add(rnd(t, c.speed));
    }
}

/// R1. Fails with `RippleBlocked` if an unlocked track ends up overlapping, negative or past
/// `MAX_TIME` (H-3).
fn shift(p: &mut Project, si: usize, from: Time, d: Time) -> R {
    for t in p.sequences[si].tracks.iter_mut().filter(|t| !t.locked) {
        for c in t.clips.iter_mut().filter(|c| c.start >= from) {
            c.start += d;
        }
        if !track_ok(t) || !t.clips.iter().all(end_ok) {
            return Err(Error::RippleBlocked { track_id: t.id });
        }
    }
    Ok(())
}

fn track_ok(t: &Track) -> bool {
    t.clips.iter().all(|c| c.start >= 0) && t.clips.windows(2).all(|w| w[0].end() <= w[1].start)
}

fn remove_clip(p: &mut Project, id: Id) -> R<Clip> {
    let (s, t, c) = clip_ix(p, id)?;
    let track = &mut p.sequences[s].tracks[t];
    if track.locked {
        return Err(Error::TrackLocked { track_id: track.id });
    }
    Ok(track.clips.remove(c))
}

/// R4. Returns the right part's id.
fn split(p: &mut Project, id: Id, at: Time) -> R<Id> {
    let (si, ti, ci) = clip_ix(p, id)?;
    let new = alloc(p);
    let mut right = p.sequences[si].tracks[ti].clips[ci].clone();
    for e in &mut right.effects {
        e.id = alloc(p);
    }
    let t = &mut p.sequences[si].tracks[ti];
    let c = &mut t.clips[ci];
    let (start, end) = (c.start, c.end());
    tail_trim(c, end - at);
    head_trim(&mut right, at - start);
    right.id = new;
    for tr in t.transitions.iter_mut().filter(|tr| tr.left == Some(id)) {
        tr.left = Some(new);
    }
    t.clips.insert(ci + 1, right);
    Ok(new)
}

/// R5 on one track.
fn clear(p: &mut Project, si: usize, ti: usize, a: Time, b: Time) -> R {
    let hit: Vec<Id> = p.sequences[si].tracks[ti]
        .clips
        .iter()
        .filter(|c| c.start < b && c.end() > a)
        .map(|c| c.id)
        .collect();
    for id in hit {
        let (start, end) = {
            let c = clip_ref(p, id)?;
            (c.start, c.end())
        };
        if start >= a && end <= b {
            remove_clip(p, id)?;
        } else if start < a && end <= b {
            tail_trim(clip_mut(p, id)?, end - a);
        } else if start >= a {
            head_trim(clip_mut(p, id)?, b - start);
        } else {
            let right = split(p, id, a)?;
            head_trim(clip_mut(p, right)?, b - a);
        }
    }
    Ok(())
}

/// R7, for one clip.
fn bounds_ok(p: &Project, c: &Clip) -> bool {
    if c.source_in < 0 {
        return false;
    }
    let ClipSource::Media { media_id } = c.source else {
        return true;
    };
    let len = p
        .media
        .iter()
        .find(|m| m.id == media_id)
        .and_then(|m| m.spec.duration);
    len.is_none_or(|d| {
        c.source_in.saturating_add(rnd(c.duration, c.speed)) <= d.saturating_add(SOURCE_SLACK)
    })
}

/// Is `target` nested, directly or indirectly, inside sequence `from`?
fn reaches(p: &Project, from: Id, target: Id) -> bool {
    let kids = |id: Id| -> Vec<Id> {
        p.sequences
            .iter()
            .filter(|s| s.id == id)
            .flat_map(|s| s.tracks.iter().flat_map(|t| &t.clips))
            .filter_map(|c| match c.source {
                ClipSource::Sequence { sequence_id } => Some(sequence_id),
                _ => None,
            })
            .collect()
    };
    let mut seen = HashSet::new();
    let mut stack = kids(from);
    while let Some(id) = stack.pop() {
        if id == target {
            return true;
        }
        if seen.insert(id) {
            stack.extend(kids(id));
        }
    }
    false
}

// ---------------------------------------------------------------------------------------------
// Transitions (I5, G8)

fn edges(t: &Transition) -> Vec<(Id, bool)> {
    t.left
        .map(|l| (l, true))
        .into_iter()
        .chain(t.right.map(|r| (r, false)))
        .collect()
}

/// I5 for a single transition against its track's clips.
pub(crate) fn transition_ok(clips: &[Clip], t: &Transition) -> bool {
    let find = |id: Id| clips.iter().find(|c| c.id == id);
    if t.duration <= 0 {
        return false;
    }
    match (t.left.map(find), t.right.map(find)) {
        (Some(Some(l)), Some(Some(r))) => {
            l.end() == r.start
                && t.duration / 2 <= l.duration
                && t.duration - t.duration / 2 <= r.duration
        }
        (Some(Some(c)), None) | (None, Some(Some(c))) => t.duration <= c.duration,
        _ => false,
    }
}

/// True if no clip edge carries two transitions.
pub(crate) fn edges_unique(ts: &[Transition]) -> bool {
    let mut seen = HashSet::new();
    ts.iter().flat_map(edges).all(|e| seen.insert(e))
}

pub(crate) fn prune(p: &mut Project) {
    for t in p.sequences.iter_mut().flat_map(|s| s.tracks.iter_mut()) {
        let mut seen = HashSet::new();
        let clips = &t.clips;
        t.transitions.retain(|tr| {
            let es = edges(tr);
            let keep = transition_ok(clips, tr) && es.iter().all(|e| !seen.contains(e));
            if keep {
                seen.extend(es);
            }
            keep
        });
    }
}

// ---------------------------------------------------------------------------------------------
// Entry

type Locked = Vec<(Id, Track)>;

fn snapshot_locked(p: &Project) -> Locked {
    p.sequences
        .iter()
        .flat_map(|s| s.tracks.iter().map(move |t| (s.id, t)))
        .filter(|(_, t)| t.locked)
        .map(|(s, t)| (s, t.clone()))
        .collect()
}

/// G5: a locked track keeps its clips and transitions.
fn check_locked(p: &Project, before: &Locked) -> R {
    for (sid, old) in before {
        if p.sequence(*sid).is_none() {
            continue;
        }
        match p.track(old.id) {
            Some((_, t)) if t.clips == old.clips && t.transitions == old.transitions => {}
            _ => return Err(Error::TrackLocked { track_id: old.id }),
        }
    }
    Ok(())
}

pub(crate) fn execute(p: &mut Project, cmd: Command) -> R<Vec<Id>> {
    let mut created = Vec::new();
    step(p, cmd, &mut created)?;
    Ok(created)
}

fn step(p: &mut Project, cmd: Command, out: &mut Vec<Id>) -> R {
    if let Command::Batch { commands } = cmd {
        return commands.into_iter().try_for_each(|c| step(p, c, out));
    }
    let locked = snapshot_locked(p);
    run(p, cmd, out)?;
    check_locked(p, &locked)?;
    let clips = p
        .sequences
        .iter()
        .flat_map(|s| &s.tracks)
        .flat_map(|t| &t.clips);
    for c in clips {
        if !end_ok(c) || c.source_in > MAX_TIME {
            return bad("clip ends or reads beyond MAX_TIME");
        }
    }
    for t in p.sequences.iter_mut().flat_map(|s| s.tracks.iter_mut()) {
        t.clips.sort_by_key(|c| c.start);
        if !track_ok(t) {
            return Err(Error::Overlap { track_id: t.id });
        }
    }
    for c in p
        .sequences
        .iter()
        .flat_map(|s| &s.tracks)
        .flat_map(|t| &t.clips)
    {
        if !bounds_ok(p, c) {
            return Err(Error::SourceOutOfRange { clip_id: c.id });
        }
    }
    // H-10: G8 after every command, batch members included.
    prune(p);
    Ok(())
}

fn fits(p: &Project, kind: TrackKind, s: &ClipSource) -> R<bool> {
    Ok(match s {
        ClipSource::Media { media_id } => {
            let Some(m) = p.media.iter().find(|m| m.id == *media_id) else {
                return nf("media", *media_id);
            };
            match kind {
                TrackKind::Video => m.spec.video.is_some(),
                TrackKind::Audio => m.spec.audio.is_some(),
            }
        }
        ClipSource::Sequence { sequence_id } => {
            seq_ix(p, *sequence_id)?;
            true
        }
        _ => kind == TrackKind::Video,
    })
}

fn new_track(id: Id, kind: TrackKind, name: String) -> Track {
    Track {
        id,
        kind,
        name,
        locked: false,
        muted: false,
        solo: false,
        hidden: false,
        clips: Vec::new(),
        transitions: Vec::new(),
    }
}

/// Target track indexes for range commands: listed (must be unlocked, in the sequence) or all
/// unlocked.
fn range_targets(p: &Project, si: usize, ids: &Option<Vec<Id>>) -> R<Vec<usize>> {
    let s = &p.sequences[si];
    match ids {
        None => Ok((0..s.tracks.len())
            .filter(|&i| !s.tracks[i].locked)
            .collect()),
        Some(v) => v
            .iter()
            .map(|&id| {
                let (tsi, ti) = track_ix(p, id)?;
                if tsi != si {
                    return bad("track belongs to another sequence");
                }
                unlocked(p, id)?;
                Ok(ti)
            })
            .collect(),
    }
}

/// Params map of a target plus the default for a newly created entry.
fn target_params<'a>(
    p: &'a mut Project,
    t: &ParamTarget,
) -> R<(&'a mut BTreeMap<String, Param>, f64)> {
    let c = clip_mut(p, t.clip_id)?;
    match t.effect_id {
        None => {
            let Some(d) = CLIP_PARAMS.iter().find(|(n, _)| *n == t.name) else {
                return bad("unknown clip property");
            };
            Ok((&mut c.params, d.1))
        }
        Some(e) => {
            let Some(inst) = c.effects.iter_mut().find(|x| x.id == e) else {
                return nf("effect", e);
            };
            if t.name.is_empty() {
                return bad("empty parameter name");
            }
            Ok((&mut inst.params, 0.0))
        }
    }
}

fn new_clip(
    id: Id,
    name: String,
    source: ClipSource,
    start: Time,
    duration: Time,
    source_in: Time,
) -> Clip {
    Clip {
        id,
        name,
        start,
        duration,
        source,
        source_in,
        speed: 1.0,
        reverse: false,
        enabled: true,
        link: None,
        group: None,
        blend: BlendMode::Normal,
        audio_filter: String::new(),
        params: BTreeMap::new(),
        effects: Vec::new(),
    }
}

/// Shared by `SplitAt` / `SplitClip`: fresh shared link id per old link among the right parts.
fn relink(p: &mut Project, rights: &[Id]) -> R {
    let mut map: Vec<(Id, Id)> = Vec::new();
    for &r in rights {
        let Some(old) = clip_ref(p, r)?.link else {
            continue;
        };
        let new = match map.iter().find(|(o, _)| *o == old) {
            Some(&(_, n)) => n,
            None => {
                let n = alloc(p);
                map.push((old, n));
                n
            }
        };
        clip_mut(p, r)?.link = Some(new);
    }
    Ok(())
}

fn run(p: &mut Project, cmd: Command, out: &mut Vec<Id>) -> R {
    use Command::*;
    match cmd {
        Batch { .. } => unreachable!("handled in step"),

        // --- media, sequences, tracks, markers, effect definitions ---
        AddMedia { spec } => {
            if spec.duration.is_some_and(|d| d <= 0)
                || (spec.video.is_none() && spec.audio.is_none())
            {
                return bad("media needs a positive duration and a video or audio stream");
            }
            spec.duration.map_or(Ok(()), upto)?;
            let id = alloc(p);
            p.media.push(Media { id, spec });
            out.push(id);
        }
        RemoveMedia { media_id } => {
            let Some(i) = p.media.iter().position(|m| m.id == media_id) else {
                return nf("media", media_id);
            };
            let used = p
                .sequences
                .iter()
                .flat_map(|s| &s.tracks)
                .flat_map(|t| &t.clips)
                .any(|c| c.source == ClipSource::Media { media_id });
            if used {
                return Err(Error::InUse {
                    what: "media",
                    id: media_id,
                });
            }
            p.media.remove(i);
        }
        SetMedia {
            media_id,
            name,
            path,
        } => {
            let Some(m) = p.media.iter_mut().find(|m| m.id == media_id) else {
                return nf("media", media_id);
            };
            if let Some(n) = name {
                m.spec.name = n;
            }
            if let Some(x) = path {
                m.spec.path = x;
            }
        }
        AddSequence {
            name,
            width,
            height,
            fps,
            sample_rate,
            video_tracks,
            audio_tracks,
        } => {
            if width == 0 || height == 0 || sample_rate == 0 || fps.num == 0 || fps.den == 0 {
                return bad("zero sequence parameter");
            }
            let id = alloc(p);
            out.push(id);
            let mut tracks = Vec::new();
            for (kind, n, prefix) in [
                (TrackKind::Video, video_tracks, "V"),
                (TrackKind::Audio, audio_tracks, "A"),
            ] {
                for k in 1..=n {
                    let tid = alloc(p);
                    out.push(tid);
                    tracks.push(new_track(tid, kind, format!("{prefix}{k}")));
                }
            }
            p.sequences.push(Sequence {
                id,
                name,
                width,
                height,
                fps,
                sample_rate,
                tracks,
                markers: Vec::new(),
            });
        }
        RemoveSequence { sequence_id } => {
            let si = seq_ix(p, sequence_id)?;
            let nested = p
                .sequences
                .iter()
                .filter(|s| s.id != sequence_id)
                .flat_map(|s| &s.tracks)
                .flat_map(|t| &t.clips)
                .any(|c| c.source == ClipSource::Sequence { sequence_id });
            if nested {
                return Err(Error::InUse {
                    what: "sequence",
                    id: sequence_id,
                });
            }
            if let Some(t) = p.sequences[si].tracks.iter().find(|t| t.locked) {
                return Err(Error::TrackLocked { track_id: t.id });
            }
            p.sequences.remove(si);
        }
        SetSequence {
            sequence_id,
            name,
            width,
            height,
            fps,
            sample_rate,
        } => {
            let si = seq_ix(p, sequence_id)?;
            if width == Some(0)
                || height == Some(0)
                || sample_rate == Some(0)
                || fps.is_some_and(|f| f.num == 0 || f.den == 0)
            {
                return bad("zero sequence parameter");
            }
            let s = &mut p.sequences[si];
            if let Some(x) = name {
                s.name = x;
            }
            s.width = width.unwrap_or(s.width);
            s.height = height.unwrap_or(s.height);
            s.fps = fps.unwrap_or(s.fps);
            s.sample_rate = sample_rate.unwrap_or(s.sample_rate);
        }
        AddTrack {
            sequence_id,
            kind,
            name,
            index,
        } => {
            let si = seq_ix(p, sequence_id)?;
            let len = p.sequences[si].tracks.len();
            let index = index.unwrap_or(len);
            if index > len {
                return bad("track index out of range");
            }
            let id = alloc(p);
            let s = &mut p.sequences[si];
            let k = s.tracks.iter().filter(|t| t.kind == kind).count() + 1;
            let name = name.unwrap_or_else(|| {
                format!("{}{k}", if kind == TrackKind::Video { "V" } else { "A" })
            });
            s.tracks.insert(index, new_track(id, kind, name));
            out.push(id);
        }
        RemoveTrack { track_id } => {
            unlocked(p, track_id)?;
            let (si, ti) = track_ix(p, track_id)?;
            p.sequences[si].tracks.remove(ti);
        }
        MoveTrack { track_id, index } => {
            let (si, ti) = track_ix(p, track_id)?;
            let tracks = &mut p.sequences[si].tracks;
            if index >= tracks.len() {
                return bad("track index out of range");
            }
            let t = tracks.remove(ti);
            tracks.insert(index, t);
        }
        SetTrack {
            track_id,
            name,
            locked,
            muted,
            solo,
            hidden,
        } => {
            let (si, ti) = track_ix(p, track_id)?;
            let t = &mut p.sequences[si].tracks[ti];
            if let Some(x) = name {
                t.name = x;
            }
            t.locked = locked.unwrap_or(t.locked);
            t.muted = muted.unwrap_or(t.muted);
            t.solo = solo.unwrap_or(t.solo);
            t.hidden = hidden.unwrap_or(t.hidden);
        }
        AddMarker {
            sequence_id,
            time,
            duration,
            name,
            note,
        } => {
            let si = seq_ix(p, sequence_id)?;
            if time < 0 || duration < 0 {
                return bad("negative marker time or duration");
            }
            upto(time)?;
            upto(duration)?;
            let id = alloc(p);
            let ms = &mut p.sequences[si].markers;
            let at = ms.partition_point(|m| m.time <= time);
            ms.insert(
                at,
                Marker {
                    id,
                    time,
                    duration,
                    name,
                    note,
                },
            );
            out.push(id);
        }
        RemoveMarker { marker_id } => {
            for s in &mut p.sequences {
                if let Some(i) = s.markers.iter().position(|m| m.id == marker_id) {
                    s.markers.remove(i);
                    return Ok(());
                }
            }
            return nf("marker", marker_id);
        }
        DefineEffect { def } => {
            if def.name.is_empty() {
                return bad("empty effect name");
            }
            for pd in &def.params {
                [pd.default, pd.min, pd.max].into_iter().try_for_each(fin)?;
            }
            match p.effects.iter_mut().find(|e| e.name == def.name) {
                Some(e) => *e = def,
                None => p.effects.push(def),
            }
        }
        RemoveEffectDef { name } => {
            let Some(i) = p.effects.iter().position(|e| e.name == name) else {
                return bad("unknown effect definition");
            };
            p.effects.remove(i);
        }

        // --- clips ---
        AddClip {
            track_id,
            linked_track_id,
            source,
            start,
            duration,
            source_in,
            mode,
            name,
        } => {
            if start < 0 || duration <= 0 {
                return bad("negative start or non-positive duration");
            }
            [start, duration, source_in]
                .into_iter()
                .try_for_each(upto)?;
            upto(start + duration)?;
            let (si, ti) = track_ix(p, track_id)?;
            unlocked(p, track_id)?;
            if !fits(p, p.sequences[si].tracks[ti].kind, &source)? {
                return Err(Error::KindMismatch("source does not fit track kind".into()));
            }
            let nests = matches!(source, ClipSource::Sequence { .. });
            if let ClipSource::Sequence { sequence_id } = source {
                let own = p.sequences[si].id;
                if sequence_id == own || reaches(p, sequence_id, own) {
                    return Err(Error::NestCycle { sequence_id });
                }
            }
            let linked = match linked_track_id {
                None => None,
                Some(lt) => {
                    let (lsi, lti) = track_ix(p, lt)?;
                    unlocked(p, lt)?;
                    let both = match &source {
                        ClipSource::Media { media_id } => p
                            .media
                            .iter()
                            .find(|m| m.id == *media_id)
                            .is_some_and(|m| m.spec.video.is_some() && m.spec.audio.is_some()),
                        ClipSource::Sequence { .. } => true,
                        _ => false,
                    };
                    if !both
                        || lsi != si
                        || p.sequences[si].tracks[ti].kind != TrackKind::Video
                        || p.sequences[lsi].tracks[lti].kind != TrackKind::Audio
                    {
                        return Err(Error::KindMismatch("bad linked placement".into()));
                    }
                    Some(lt)
                }
            };
            let name = name.unwrap_or_else(|| match &source {
                ClipSource::Media { media_id } => p
                    .media
                    .iter()
                    .find(|m| m.id == *media_id)
                    .map(|m| m.spec.name.clone())
                    .unwrap_or_default(),
                ClipSource::Sequence { sequence_id } => p
                    .sequence(*sequence_id)
                    .map(|s| s.name.clone())
                    .unwrap_or_default(),
                ClipSource::Text { .. } => "Text".into(),
                ClipSource::Solid { .. } => "Solid".into(),
                ClipSource::Adjustment => "Adjustment".into(),
            });
            let id = alloc(p);
            let mut new = vec![(
                track_id,
                new_clip(id, name.clone(), source.clone(), start, duration, source_in),
            )];
            if let Some(lt) = linked {
                let aid = alloc(p);
                let l = alloc(p);
                new.push((lt, new_clip(aid, name, source, start, duration, source_in)));
                new[0].1.link = Some(l);
                new[1].1.link = Some(l);
            }
            out.extend(new.iter().map(|(_, c)| c.id));
            match mode {
                PlaceMode::Place => {}
                PlaceMode::Overwrite => {
                    for (t, _) in &new {
                        let (si, ti) = track_ix(p, *t)?;
                        clear(p, si, ti, start, start + duration)?;
                    }
                }
                PlaceMode::Insert => {
                    let crossing: Vec<Id> = p.sequences[si]
                        .tracks
                        .iter()
                        .filter(|t| !t.locked)
                        .flat_map(|t| &t.clips)
                        .filter(|c| c.start < start && start < c.end())
                        .map(|c| c.id)
                        .collect();
                    for c in crossing {
                        split(p, c, start)?;
                    }
                    shift(p, si, start, duration)?;
                }
            }
            for (t, c) in new {
                let (si, ti) = track_ix(p, t)?;
                p.sequences[si].tracks[ti].clips.push(c);
            }
            if nests && too_deep(p) {
                return bad("sequences nested deeper than MAX_NEST_DEPTH");
            }
        }
        RemoveClips {
            clip_ids,
            include_linked,
        } => {
            if clip_ids.is_empty() {
                return bad("no clips given");
            }
            for id in expand(p, &clip_ids, include_linked)? {
                remove_clip(p, id)?;
            }
        }
        RippleDeleteClips {
            clip_ids,
            include_linked,
        } => {
            if clip_ids.is_empty() {
                return bad("no clips given");
            }
            let ids = expand(p, &clip_ids, include_linked)?;
            let si = clip_ix(p, ids[0])?.0;
            let mut ranges = Vec::new();
            for &id in &ids {
                let (s, _, _) = clip_ix(p, id)?;
                if s != si {
                    return bad("clips span several sequences");
                }
                let c = clip_ref(p, id)?;
                ranges.push((c.start, c.end()));
            }
            for id in ids {
                remove_clip(p, id)?;
            }
            ranges.sort();
            let mut merged: Vec<(Time, Time)> = Vec::new();
            for (a, b) in ranges {
                match merged.last_mut() {
                    Some(last) if a <= last.1 => last.1 = last.1.max(b),
                    _ => merged.push((a, b)),
                }
            }
            for (a, b) in merged.into_iter().rev() {
                for t in p.sequences[si].tracks.iter().filter(|t| !t.locked) {
                    if t.clips.iter().any(|c| c.start < b && c.end() > a) {
                        return Err(Error::RippleBlocked { track_id: t.id });
                    }
                }
                shift(p, si, b, a - b)?;
            }
        }
        LiftRange {
            sequence_id,
            start,
            end,
            track_ids,
        } => {
            let si = seq_ix(p, sequence_id)?;
            if start < 0 || end <= start {
                return bad("bad range");
            }
            upto(end)?;
            for ti in range_targets(p, si, &track_ids)? {
                clear(p, si, ti, start, end)?;
            }
        }
        ExtractRange {
            sequence_id,
            start,
            end,
            track_ids,
        } => {
            let si = seq_ix(p, sequence_id)?;
            if start < 0 || end <= start {
                return bad("bad range");
            }
            upto(end)?;
            for ti in range_targets(p, si, &track_ids)? {
                clear(p, si, ti, start, end)?;
                for c in p.sequences[si].tracks[ti]
                    .clips
                    .iter_mut()
                    .filter(|c| c.start >= end)
                {
                    c.start -= end - start;
                }
            }
        }
        CloseGap { track_id, time } => {
            let (si, ti) = track_ix(p, track_id)?;
            unlocked(p, track_id)?;
            if time < 0 {
                return bad("negative time");
            }
            upto(time)?;
            let clips = &p.sequences[si].tracks[ti].clips;
            let next = clips
                .iter()
                .filter(|c| c.start > time)
                .map(|c| c.start)
                .min();
            let inside = clips.iter().any(|c| c.start <= time && time < c.end());
            let (false, Some(b)) = (inside, next) else {
                return bad("no gap at that time");
            };
            let a = clips
                .iter()
                .filter(|c| c.end() <= time)
                .map(|c| c.end())
                .max()
                .unwrap_or(0);
            for t in p.sequences[si].tracks.iter().filter(|t| !t.locked) {
                if t.clips.iter().any(|c| c.start < b && c.end() > a) {
                    return Err(Error::RippleBlocked { track_id: t.id });
                }
            }
            shift(p, si, b, a - b)?;
        }
        SplitAt {
            sequence_id,
            time,
            track_ids,
        } => {
            let si = seq_ix(p, sequence_id)?;
            if time < 0 {
                return bad("negative time");
            }
            upto(time)?;
            let targets = range_targets(p, si, &track_ids)?;
            let mut ordered = targets;
            ordered.sort();
            let crossing: Vec<Id> = ordered
                .iter()
                .flat_map(|&ti| &p.sequences[si].tracks[ti].clips)
                .filter(|c| c.start < time && time < c.end())
                .map(|c| c.id)
                .collect();
            let mut rights = Vec::new();
            for c in crossing {
                rights.push(split(p, c, time)?);
            }
            relink(p, &rights)?;
            out.extend(rights);
        }
        SplitClip {
            clip_id,
            time,
            include_linked,
        } => {
            let c = clip_ref(p, clip_id)?;
            if !(c.start < time && time < c.end()) {
                return bad("time not inside clip");
            }
            let set: Vec<Id> = expand(p, &[clip_id], include_linked)?
                .into_iter()
                .filter(|&id| clip_ref(p, id).is_ok_and(|c| c.start < time && time < c.end()))
                .collect();
            let mut rights = Vec::new();
            for id in set {
                rights.push(split(p, id, time)?);
            }
            if include_linked {
                relink(p, &rights)?;
            } else {
                clip_mut(p, rights[0])?.link = None;
            }
            out.extend(rights);
        }
        MoveClip {
            clip_id,
            start,
            track_id,
            mode,
            include_linked,
        } => {
            let c = clip_ref(p, clip_id)?;
            let (si, _, _) = clip_ix(p, clip_id)?;
            if start < 0 {
                return bad("negative start");
            }
            upto(start)?;
            if mode == PlaceMode::Insert {
                return bad("insert is not supported for moves");
            }
            let delta = start - c.start;
            let dest = match track_id {
                None => None,
                Some(t) => {
                    let (tsi, ti) = track_ix(p, t)?;
                    if tsi != si {
                        return bad("track belongs to another sequence");
                    }
                    if !fits(p, p.sequences[tsi].tracks[ti].kind, &c.source)? {
                        return Err(Error::KindMismatch("source does not fit track kind".into()));
                    }
                    Some(t)
                }
            };
            let mut moving = Vec::new();
            for id in expand(p, &[clip_id], include_linked)? {
                let (_, ti, _) = clip_ix(p, id)?;
                let from = p.sequences[si].tracks[ti].id;
                let mut c = remove_clip(p, id)?;
                c.start += delta;
                if c.start < 0 {
                    return bad("negative start");
                }
                let to = if id == clip_id {
                    dest.unwrap_or(from)
                } else {
                    from
                };
                moving.push((to, c));
            }
            if mode == PlaceMode::Overwrite {
                for (t, c) in &moving {
                    let (si, ti) = track_ix(p, *t)?;
                    clear(p, si, ti, c.start, c.end())?;
                }
            }
            for (t, c) in moving {
                let (si, ti) = track_ix(p, t)?;
                p.sequences[si].tracks[ti].clips.push(c);
            }
        }
        TrimClip {
            clip_id,
            edge,
            to,
            mode,
            include_linked,
        } => {
            if to < 0 {
                return bad("negative trim position");
            }
            upto(to)?;
            let c0 = clip_ref(p, clip_id)?.clone();
            let (si, ti, _) = clip_ix(p, clip_id)?;
            let cur = if edge == Edge::Start {
                c0.start
            } else {
                c0.end()
            };
            let d = to - cur;
            if mode == TrimMode::Roll {
                let t = &p.sequences[si].tracks[ti];
                let nb = t.clips.iter().find(|c| {
                    if edge == Edge::End {
                        c.start == c0.end()
                    } else {
                        c.end() == c0.start
                    }
                });
                let Some(nb) = nb.map(|c| c.id) else {
                    return Err(Error::NoAdjacentClip { clip_id });
                };
                if edge == Edge::End {
                    tail_trim(clip_mut(p, clip_id)?, -d);
                    head_trim(clip_mut(p, nb)?, d);
                } else {
                    head_trim(clip_mut(p, clip_id)?, d);
                    tail_trim(clip_mut(p, nb)?, -d);
                }
                for id in [clip_id, nb] {
                    if clip_ref(p, id)?.duration <= 0 {
                        return bad("trim leaves no duration");
                    }
                }
                return Ok(());
            }
            let affected: Vec<Id> = expand(p, &[clip_id], include_linked)?
                .into_iter()
                .filter(|&id| {
                    clip_ref(p, id).is_ok_and(|c| {
                        id == clip_id
                            || (if edge == Edge::Start {
                                c.start
                            } else {
                                c.end()
                            }) == cur
                    })
                })
                .collect();
            for &id in &affected {
                let c = clip_mut(p, id)?;
                if edge == Edge::Start {
                    head_trim(c, d);
                    if mode == TrimMode::Ripple {
                        c.start -= d;
                    }
                } else {
                    tail_trim(c, -d);
                }
                if c.duration <= 0 {
                    return bad("trim leaves no duration");
                }
            }
            if mode == TrimMode::Ripple {
                shift(p, si, c0.end(), if edge == Edge::End { d } else { -d })?;
            }
        }
        SlipClip {
            clip_id,
            delta,
            include_linked,
        } => {
            within(delta)?;
            for id in expand(p, &[clip_id], include_linked)? {
                let c = clip_mut(p, id)?;
                if !has_source_time(c) {
                    return bad("only media and sequence clips have a source range");
                }
                c.source_in += delta;
            }
        }
        SlideClip { clip_id, delta } => {
            within(delta)?;
            let c = clip_ref(p, clip_id)?.clone();
            let (si, ti, _) = clip_ix(p, clip_id)?;
            let t = &p.sequences[si].tracks[ti];
            let prev = t.clips.iter().find(|x| x.end() == c.start).map(|x| x.id);
            let next = t.clips.iter().find(|x| x.start == c.end()).map(|x| x.id);
            let (Some(pr), Some(nx)) = (prev, next) else {
                return Err(Error::NoAdjacentClip { clip_id });
            };
            clip_mut(p, clip_id)?.start += delta;
            tail_trim(clip_mut(p, pr)?, -delta);
            head_trim(clip_mut(p, nx)?, delta);
            for id in [pr, nx] {
                if clip_ref(p, id)?.duration <= 0 {
                    return bad("slide leaves a neighbour with no duration");
                }
            }
        }
        SetClipSpeed {
            clip_id,
            speed,
            ripple,
            include_linked,
        } => {
            if !(0.01..=100.0).contains(&speed) {
                return bad("speed out of range");
            }
            let (si, _, _) = clip_ix(p, clip_id)?;
            let old_end = clip_ref(p, clip_id)?.end();
            let mut d = 0;
            for id in expand(p, &[clip_id], include_linked)? {
                let c = clip_mut(p, id)?;
                if !has_source_time(c) {
                    return bad("only media and sequence clips have a speed");
                }
                let nd = ((c.duration as f64 * c.speed / speed).round() as Time).max(1);
                upto(nd)?;
                if id == clip_id {
                    d = nd - c.duration;
                }
                c.duration = nd;
                c.speed = speed;
            }
            if ripple {
                shift(p, si, old_end, d)?;
            }
        }
        SetClip {
            clip_id,
            name,
            enabled,
            reverse,
            blend,
            audio_filter,
        } => {
            let c = clip_mut(p, clip_id)?;
            if let Some(x) = name {
                c.name = x;
            }
            c.enabled = enabled.unwrap_or(c.enabled);
            c.reverse = reverse.unwrap_or(c.reverse);
            c.blend = blend.unwrap_or(c.blend);
            if let Some(x) = audio_filter {
                c.audio_filter = x;
            }
        }
        SetText { clip_id, text } => {
            let c = clip_mut(p, clip_id)?;
            if !matches!(c.source, ClipSource::Text { .. }) {
                return Err(Error::KindMismatch("not a text clip".into()));
            }
            if text.size.is_nan() || text.size <= 0.0 || text.color.iter().any(|x| !x.is_finite()) {
                return bad("bad text size or colour");
            }
            c.source = ClipSource::Text { text };
        }
        LinkClips { clip_ids } => {
            let ids = same_sequence(p, &clip_ids)?;
            let l = alloc(p);
            for id in ids {
                clip_mut(p, id)?.link = Some(l);
            }
        }
        GroupClips { clip_ids } => {
            let ids = same_sequence(p, &clip_ids)?;
            let g = alloc(p);
            for id in ids {
                clip_mut(p, id)?.group = Some(g);
            }
        }
        UnlinkClips { clip_ids } => {
            if clip_ids.is_empty() {
                return bad("no clips given");
            }
            for id in expand(p, &clip_ids, false)? {
                clip_mut(p, id)?.link = None;
            }
        }
        UngroupClips { clip_ids } => {
            if clip_ids.is_empty() {
                return bad("no clips given");
            }
            for id in expand(p, &clip_ids, false)? {
                clip_mut(p, id)?.group = None;
            }
        }

        // --- parameters, keyframes, effects, transitions ---
        SetParam { target, value } => {
            fin(value)?;
            let (m, _) = target_params(p, &target)?;
            m.insert(
                target.name,
                Param {
                    value,
                    keyframes: Vec::new(),
                },
            );
        }
        SetKeyframe {
            target,
            time,
            value,
            easing,
        } => {
            fin(value)?;
            within(time)?;
            let (m, default) = target_params(p, &target)?;
            let pm = m.entry(target.name).or_insert(Param {
                value: default,
                keyframes: Vec::new(),
            });
            let k = Keyframe {
                time,
                value,
                easing,
            };
            match pm.keyframes.binary_search_by_key(&time, |k| k.time) {
                Ok(i) => pm.keyframes[i] = k,
                Err(i) => pm.keyframes.insert(i, k),
            }
        }
        RemoveKeyframe { target, time } => {
            let (m, _) = target_params(p, &target)?;
            let Some(pm) = m.get_mut(&target.name) else {
                return bad("no such parameter");
            };
            let Some(i) = pm.keyframes.iter().position(|k| k.time == time) else {
                return bad("no keyframe at that time");
            };
            pm.keyframes.remove(i);
        }
        ClearKeyframes { target } => {
            let (m, _) = target_params(p, &target)?;
            if let Some(pm) = m.get_mut(&target.name) {
                pm.keyframes.clear();
            }
        }
        AddEffect {
            clip_id,
            effect,
            index,
            params,
        } => {
            params.values().try_for_each(|&v| fin(v))?;
            let len = clip_ref(p, clip_id)?.effects.len();
            let index = index.unwrap_or(len);
            if effect.is_empty() || index > len {
                return bad("empty effect name or index out of range");
            }
            let id = alloc(p);
            let params = params
                .into_iter()
                .map(|(k, value)| {
                    (
                        k,
                        Param {
                            value,
                            keyframes: Vec::new(),
                        },
                    )
                })
                .collect();
            clip_mut(p, clip_id)?.effects.insert(
                index,
                EffectInstance {
                    id,
                    effect,
                    enabled: true,
                    params,
                },
            );
            out.push(id);
        }
        RemoveEffect { clip_id, effect_id } => {
            let c = clip_mut(p, clip_id)?;
            let i = effect_pos(c, effect_id)?;
            c.effects.remove(i);
        }
        MoveEffect {
            clip_id,
            effect_id,
            index,
        } => {
            let c = clip_mut(p, clip_id)?;
            let i = effect_pos(c, effect_id)?;
            if index >= c.effects.len() {
                return bad("effect index out of range");
            }
            let e = c.effects.remove(i);
            c.effects.insert(index, e);
        }
        SetEffectEnabled {
            clip_id,
            effect_id,
            enabled,
        } => {
            let c = clip_mut(p, clip_id)?;
            let i = effect_pos(c, effect_id)?;
            c.effects[i].enabled = enabled;
        }
        AddTransition {
            track_id,
            left,
            right,
            effect,
            duration,
            params,
        } => {
            let (si, ti) = track_ix(p, track_id)?;
            for id in left.into_iter().chain(right) {
                clip_ref(p, id)?;
            }
            params.values().try_for_each(|&v| fin(v))?;
            let id = alloc(p);
            let tr = Transition {
                id,
                left,
                right,
                effect,
                duration,
                params,
            };
            let t = &mut p.sequences[si].tracks[ti];
            if !transition_ok(&t.clips, &tr) {
                return bad("transition violates I5");
            }
            let new_edges = edges(&tr);
            t.transitions
                .retain(|o| !edges(o).iter().any(|e| new_edges.contains(e)));
            t.transitions.push(tr);
            out.push(id);
        }
        RemoveTransition { transition_id } => {
            let (si, ti, i) = transition_ix(p, transition_id)?;
            p.sequences[si].tracks[ti].transitions.remove(i);
        }
        SetTransition {
            transition_id,
            effect,
            duration,
            params,
        } => {
            let (si, ti, i) = transition_ix(p, transition_id)?;
            if let Some(m) = &params {
                m.values().try_for_each(|&v| fin(v))?;
            }
            let t = &mut p.sequences[si].tracks[ti];
            let tr = &mut t.transitions[i];
            tr.effect = effect.unwrap_or_else(|| tr.effect.clone());
            tr.duration = duration.unwrap_or(tr.duration);
            tr.params = params.unwrap_or_else(|| tr.params.clone());
            if !transition_ok(&t.clips, &t.transitions[i]) {
                return bad("transition violates I5");
            }
        }
    }
    Ok(())
}

fn effect_pos(c: &Clip, id: Id) -> R<usize> {
    c.effects
        .iter()
        .position(|e| e.id == id)
        .map_or_else(|| nf("effect", id), Ok)
}

fn transition_ix(p: &Project, id: Id) -> R<(usize, usize, usize)> {
    for (si, s) in p.sequences.iter().enumerate() {
        for (ti, t) in s.tracks.iter().enumerate() {
            if let Some(i) = t.transitions.iter().position(|x| x.id == id) {
                return Ok((si, ti, i));
            }
        }
    }
    nf("transition", id)
}

/// Link/group argument check: at least 2 clips, all in one sequence.
fn same_sequence(p: &Project, ids: &[Id]) -> R<Vec<Id>> {
    let ids = expand(p, ids, false)?;
    if ids.len() < 2 {
        return bad("need at least two clips");
    }
    let si = clip_ix(p, ids[0])?.0;
    for &id in &ids {
        if clip_ix(p, id)?.0 != si {
            return bad("clips span several sequences");
        }
    }
    Ok(ids)
}

pub(crate) fn nests_itself(p: &Project, id: Id) -> bool {
    reaches(p, id, id)
}

/// H-4: does any sequence nest deeper than `MAX_NEST_DEPTH`? Needs an acyclic nesting graph.
/// Relaxation instead of recursion: no stack depth or blow-up on hostile projects. After `k`
/// rounds every depth is at least min(true depth, k + 1) and never above the true depth.
pub(crate) fn too_deep(p: &Project) -> bool {
    let mut depth: HashMap<Id, usize> = p.sequences.iter().map(|s| (s.id, 1)).collect();
    for _ in 0..MAX_NEST_DEPTH {
        for s in &p.sequences {
            let kids = s.tracks.iter().flat_map(|t| &t.clips);
            let d = 1 + kids
                .filter_map(|c| match c.source {
                    ClipSource::Sequence { sequence_id } => depth.get(&sequence_id).copied(),
                    _ => None,
                })
                .max()
                .unwrap_or(0);
            depth.insert(s.id, d.min(MAX_NEST_DEPTH + 1));
        }
    }
    depth.values().any(|&d| d > MAX_NEST_DEPTH)
}
