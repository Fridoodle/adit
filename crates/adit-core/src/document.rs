//! Public behaviour API. Signatures are the contract (docs/SPEC.md); command execution lives in
//! `engine.rs`.

use crate::engine::{edges_unique, end_ok, execute, nests_itself, too_deep, transition_ok};
use crate::{
    CLIP_PARAMS, Clip, ClipSource, Command, Easing, Error, FLICKS_PER_SECOND, Id, MAX_TIME,
    Outcome, Param, Project, Rational, SOURCE_SLACK, Sequence, Time, Track, TrackKind,
};
use std::collections::HashSet;

/// A project plus its undo/redo history. The only way to change a project.
#[derive(Debug, Clone)]
pub struct Document {
    project: Project,
    undo: Vec<Project>,
    redo: Vec<Project>,
}

impl Document {
    pub fn new(project: Project) -> Self {
        Self {
            project,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn apply(&mut self, command: Command) -> Result<Outcome, Error> {
        let mut next = self.project.clone();
        let created = execute(&mut next, command)?;
        next.validate().map_err(Error::InvalidArgument)?;
        if next != self.project {
            // shortcut: whole-project snapshots per undo step, switch to inverse commands if projects get huge
            self.undo.push(std::mem::replace(&mut self.project, next));
            self.redo.clear();
        }
        Ok(Outcome { created })
    }

    /// Returns false (and changes nothing) when there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.undo.pop() else {
            return false;
        };
        self.redo.push(std::mem::replace(&mut self.project, prev));
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(std::mem::replace(&mut self.project, next));
        true
    }

    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_depth(&self) -> usize {
        self.redo.len()
    }
}

fn finite(x: f64, what: &str) -> Result<(), String> {
    if x.is_finite() {
        Ok(())
    } else {
        Err(format!("{what} is not finite"))
    }
}

fn check_param(p: &Param, what: &str) -> Result<(), String> {
    finite(p.value, what)?;
    for k in &p.keyframes {
        finite(k.value, what)?;
        if !(-MAX_TIME..=MAX_TIME).contains(&k.time) {
            return Err(format!("H-3: {what}: keyframe time beyond ±MAX_TIME"));
        }
    }
    if !p.keyframes.windows(2).all(|w| w[0].time < w[1].time) {
        return Err(format!("{what}: keyframes not sorted with unique times"));
    }
    Ok(())
}

impl Project {
    /// Empty project, `next_id` = 1.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            next_id: 1,
            media: Vec::new(),
            sequences: Vec::new(),
            effects: Vec::new(),
        }
    }

    /// Checks invariants I1-I6. `Err` carries a human-readable reason.
    pub fn validate(&self) -> Result<(), String> {
        let mut ids: HashSet<Id> = HashSet::new();
        let mut entity = |what: &str, id: Id| -> Result<(), String> {
            if id >= self.next_id {
                Err(format!("I2: {what} id {id} >= next_id"))
            } else if !ids.insert(id) {
                Err(format!("I2: duplicate id {id} ({what})"))
            } else {
                Ok(())
            }
        };
        for m in &self.media {
            entity("media", m.id)?;
        }
        for s in &self.sequences {
            entity("sequence", s.id)?;
            for m in &s.markers {
                entity("marker", m.id)?;
            }
            for t in &s.tracks {
                entity("track", t.id)?;
                for c in &t.clips {
                    entity("clip", c.id)?;
                    for e in &c.effects {
                        entity("effect instance", e.id)?;
                    }
                }
                for tr in &t.transitions {
                    entity("transition", tr.id)?;
                }
            }
        }
        for s in &self.sequences {
            if s.width == 0
                || s.height == 0
                || s.sample_rate == 0
                || s.fps.num == 0
                || s.fps.den == 0
            {
                return Err(format!("I6: sequence {} has a zero parameter", s.id));
            }
            if s.markers.iter().any(|m| m.time < 0 || m.duration < 0) {
                return Err("I6: negative marker time or duration".into());
            }
            if s.markers
                .iter()
                .any(|m| m.time > MAX_TIME || m.duration > MAX_TIME)
            {
                return Err("H-3: marker beyond MAX_TIME".into());
            }
            if !s.markers.windows(2).all(|w| w[0].time <= w[1].time) {
                return Err("I6: markers not sorted".into());
            }
            if nests_itself(self, s.id) {
                return Err(format!("I6: sequence {} contains itself", s.id));
            }
            for t in &s.tracks {
                self.validate_track(t, &ids)?;
            }
        }
        if too_deep(self) {
            return Err("H-4: sequences nested deeper than MAX_NEST_DEPTH".into());
        }
        for m in &self.media {
            if m.spec.duration.is_some_and(|d| d <= 0) {
                return Err(format!("media {} has non-positive duration", m.id));
            }
        }
        for pd in self.effects.iter().flat_map(|e| &e.params) {
            for x in [pd.default, pd.min, pd.max] {
                finite(x, "effect parameter definition")?;
            }
        }
        Ok(())
    }

    fn validate_track(&self, t: &Track, ids: &HashSet<Id>) -> Result<(), String> {
        // Before anything calls `Clip::end`, which would overflow.
        if let Some(c) = t
            .clips
            .iter()
            .find(|c| !end_ok(c) || c.source_in > MAX_TIME)
        {
            return Err(format!("H-3: clip {} ends or reads beyond MAX_TIME", c.id));
        }
        for w in t.clips.windows(2) {
            if w[0].end() > w[1].start {
                return Err(format!(
                    "I1: clips {} and {} overlap or are unsorted",
                    w[0].id, w[1].id
                ));
            }
        }
        for c in &t.clips {
            if c.start < 0 || c.duration <= 0 {
                return Err(format!("I1: clip {} has bad start/duration", c.id));
            }
            if !c.speed.is_finite() || c.speed <= 0.0 || c.source_in < 0 {
                return Err(format!("I3: clip {} has bad speed or source_in", c.id));
            }
            if c.label > 8 {
                return Err(format!("C-LABEL-1: clip {} has label > 8", c.id));
            }
            for l in c.link.iter().chain(&c.group) {
                if *l >= self.next_id || ids.contains(l) {
                    return Err(format!("I2: clip {} link/group id {l} invalid", c.id));
                }
            }
            let fits = match &c.source {
                ClipSource::Media { media_id } => {
                    let Some(m) = self.media.iter().find(|m| m.id == *media_id) else {
                        return Err(format!("I6: clip {} references missing media", c.id));
                    };
                    if let Some(d) = m.spec.duration {
                        let used = (c.duration as f64 * c.speed).round() as Time;
                        if c.source_in.saturating_add(used) > d.saturating_add(SOURCE_SLACK) {
                            return Err(format!("I3: clip {} reads past its media", c.id));
                        }
                    }
                    match t.kind {
                        TrackKind::Video => m.spec.video.is_some(),
                        TrackKind::Audio => m.spec.audio.is_some(),
                    }
                }
                ClipSource::Sequence { sequence_id } => {
                    if self.sequence(*sequence_id).is_none() {
                        return Err(format!("I6: clip {} references missing sequence", c.id));
                    }
                    true
                }
                ClipSource::Text { text } => {
                    finite(text.size, "text size")?;
                    text.color
                        .iter()
                        .try_for_each(|x| finite(*x as f64, "text colour"))?;
                    t.kind == TrackKind::Video
                }
                ClipSource::Solid { color } => {
                    color
                        .iter()
                        .try_for_each(|x| finite(*x as f64, "solid colour"))?;
                    t.kind == TrackKind::Video
                }
                ClipSource::Adjustment => t.kind == TrackKind::Video,
            };
            if !fits {
                return Err(format!("I4: clip {} does not fit track {}", c.id, t.id));
            }
            let effect_params = c.effects.iter().flat_map(|e| &e.params);
            for (n, p) in c.params.iter().chain(effect_params) {
                check_param(p, n)?;
            }
        }
        for tr in &t.transitions {
            if !transition_ok(&t.clips, tr) {
                return Err(format!("I5: transition {} is invalid", tr.id));
            }
            tr.params
                .values()
                .try_for_each(|v| finite(*v, "transition parameter"))?;
        }
        if !edges_unique(&t.transitions) {
            return Err(format!(
                "I5: track {} has two transitions on one edge",
                t.id
            ));
        }
        Ok(())
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("project serialises")
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }

    pub fn sequence(&self, id: Id) -> Option<&Sequence> {
        self.sequences.iter().find(|s| s.id == id)
    }

    pub fn track(&self, id: Id) -> Option<(&Sequence, &Track)> {
        self.sequences
            .iter()
            .find_map(|s| s.tracks.iter().find(|t| t.id == id).map(|t| (s, t)))
    }

    pub fn clip(&self, id: Id) -> Option<(&Sequence, &Track, &Clip)> {
        self.sequences.iter().find_map(|s| {
            s.tracks
                .iter()
                .find_map(|t| t.clips.iter().find(|c| c.id == id).map(|c| (s, t, c)))
        })
    }
}

impl Sequence {
    /// Largest clip end over all tracks; 0 when empty.
    pub fn duration(&self) -> Time {
        self.tracks
            .iter()
            .flat_map(|t| &t.clips)
            .map(Clip::end)
            .max()
            .unwrap_or(0)
    }
}

impl Clip {
    pub fn end(&self) -> Time {
        self.start + self.duration
    }

    /// Value of a built-in property at `local` (time relative to clip start). Falls back to the
    /// default from `CLIP_PARAMS`; `None` for an unknown name.
    pub fn param(&self, name: &str, local: Time) -> Option<f64> {
        let default = CLIP_PARAMS.iter().find(|(n, _)| *n == name)?.1;
        Some(self.params.get(name).map_or(default, |p| p.eval(local)))
    }

    /// Source time shown at sequence time `at` (SPEC Q3). `at` may lie outside the clip.
    pub fn source_time(&self, at: Time) -> Time {
        let local = at - self.start;
        let span = if self.reverse {
            self.duration - local
        } else {
            local
        };
        // Saturating: duration * speed is not bounded by MAX_TIME (sequence clips at speed 100).
        self.source_in
            .saturating_add((span as f64 * self.speed).round() as Time)
    }
}

impl Param {
    /// SPEC Q2.
    pub fn eval(&self, local: Time) -> f64 {
        let ks = &self.keyframes;
        let (Some(first), Some(last)) = (ks.first(), ks.last()) else {
            return self.value;
        };
        if local <= first.time {
            return first.value;
        }
        if local >= last.time {
            return last.value;
        }
        let i = ks.partition_point(|k| k.time <= local) - 1;
        let (k0, k1) = (ks[i], ks[i + 1]);
        let u = (local - k0.time) as f64 / (k1.time - k0.time) as f64;
        let f = match k0.easing {
            Easing::Linear => u,
            Easing::Hold => 0.0,
            Easing::EaseIn => u * u,
            Easing::EaseOut => 1.0 - (1.0 - u) * (1.0 - u),
            Easing::EaseInOut => 3.0 * u * u - 2.0 * u * u * u,
        };
        k0.value + (k1.value - k0.value) * f
    }
}

/// Start time of frame `frame`: floor(frame * FLICKS_PER_SECOND * den / num).
pub fn frame_to_time(frame: i64, fps: Rational) -> Time {
    let n = frame as i128 * FLICKS_PER_SECOND as i128 * fps.den as i128;
    n.div_euclid(fps.num as i128) as Time
}

/// Index of the frame that contains `time` (floor).
pub fn time_to_frame(time: Time, fps: Rational) -> i64 {
    // largest n with floor(n*S*den/num) <= t  <=>  n*S*den < (t+1)*num
    let d = FLICKS_PER_SECOND as i128 * fps.den as i128;
    ((time as i128 + 1) * fps.num as i128 - 1).div_euclid(d) as i64
}
