//! Pure editing logic behind the window (GUI.md U-STATE .. U-DRAG-*). No egui widgets, no IO.

use crate::{Action, Drag, Hit, KeyChord, Op, TimelineLayout, UiState};
use adit_core::{Id, Project, Time};

pub(crate) fn new_state(_sequence_id: Id) -> UiState {
    todo!()
}

pub(crate) fn perform(_action: Action, _project: &Project, _ui: &mut UiState) -> Vec<Op> {
    todo!()
}

pub(crate) fn default_keymap() -> Vec<(KeyChord, Action)> {
    todo!()
}

pub(crate) fn time_to_x(_l: &TimelineLayout, _time: Time) -> f32 {
    todo!()
}

pub(crate) fn x_to_time(_l: &TimelineLayout, _x: f32) -> Time {
    todo!()
}

pub(crate) fn rows(_project: &Project, _sequence_id: Id) -> Vec<Id> {
    todo!()
}

pub(crate) fn hit(
    _l: &TimelineLayout,
    _project: &Project,
    _sequence_id: Id,
    _x: f32,
    _y: f32,
) -> Hit {
    todo!()
}

pub(crate) fn snap(
    _time: Time,
    _project: &Project,
    _ui: &UiState,
    _exclude: &[Id],
    _tolerance: Time,
) -> Time {
    todo!()
}

pub(crate) fn drag(_drag: Drag, _project: &Project, _ui: &mut UiState) -> Vec<Op> {
    todo!()
}
