//! Public behaviour API. Signatures are the contract (docs/SPEC.md); bodies are filled in by the
//! implementation. Private helpers and extra private modules are free.

use crate::{Clip, Command, Error, Id, Outcome, Param, Project, Rational, Sequence, Time, Track};

/// A project plus its undo/redo history. The only way to change a project.
#[derive(Debug, Clone)]
pub struct Document {
    project: Project,
    undo: Vec<Project>,
    redo: Vec<Project>,
}

impl Document {
    pub fn new(project: Project) -> Self {
        Self { project, undo: Vec::new(), redo: Vec::new() }
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn apply(&mut self, _command: Command) -> Result<Outcome, Error> {
        todo!()
    }

    /// Returns false (and changes nothing) when there is nothing to undo.
    pub fn undo(&mut self) -> bool {
        todo!()
    }

    pub fn redo(&mut self) -> bool {
        todo!()
    }

    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_depth(&self) -> usize {
        self.redo.len()
    }
}

impl Project {
    /// Empty project, `next_id` = 1.
    pub fn new(_name: &str) -> Self {
        todo!()
    }

    /// Checks invariants I1-I6. `Err` carries a human-readable reason.
    pub fn validate(&self) -> Result<(), String> {
        todo!()
    }

    pub fn to_json(&self) -> String {
        todo!()
    }

    pub fn from_json(_json: &str) -> Result<Self, String> {
        todo!()
    }

    pub fn sequence(&self, _id: Id) -> Option<&Sequence> {
        todo!()
    }

    pub fn track(&self, _id: Id) -> Option<(&Sequence, &Track)> {
        todo!()
    }

    pub fn clip(&self, _id: Id) -> Option<(&Sequence, &Track, &Clip)> {
        todo!()
    }
}

impl Sequence {
    /// Largest clip end over all tracks; 0 when empty.
    pub fn duration(&self) -> Time {
        todo!()
    }
}

impl Clip {
    pub fn end(&self) -> Time {
        self.start + self.duration
    }

    /// Value of a built-in property at `local` (time relative to clip start). Falls back to the
    /// default from `CLIP_PARAMS`; `None` for an unknown name.
    pub fn param(&self, _name: &str, _local: Time) -> Option<f64> {
        todo!()
    }

    /// Source time shown at sequence time `at` (SPEC Q3). `at` may lie outside the clip.
    pub fn source_time(&self, _at: Time) -> Time {
        todo!()
    }
}

impl Param {
    /// SPEC Q2.
    pub fn eval(&self, _local: Time) -> f64 {
        todo!()
    }
}

/// Start time of frame `frame`: floor(frame * FLICKS_PER_SECOND * den / num).
pub fn frame_to_time(_frame: i64, _fps: Rational) -> Time {
    todo!()
}

/// Index of the frame that contains `time` (floor).
pub fn time_to_frame(_time: Time, _fps: Rational) -> i64 {
    todo!()
}
