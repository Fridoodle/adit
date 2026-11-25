//! The egui window (GUI.md U-APP-*). Public items and signatures here are fixed.

use crate::{Action, Options, UiState};
use adit_mcp::Session;
use eframe::egui;

/// The egui application (GUI.md U-APP-*).
pub struct App {
    _private: (),
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>, _session: Session, _options: Options) -> Self {
        todo!()
    }

    /// Session id (`p1`, ...) of the project in the active tab.
    pub fn project_id(&self) -> Option<&str> {
        todo!()
    }

    /// Editing state of the active tab.
    pub fn ui_state(&self) -> Option<&UiState> {
        todo!()
    }

    /// Runs an action exactly as its key would (applies the resulting `Op`s through the session).
    pub fn run_action(&mut self, _action: Action) {
        todo!()
    }
}

impl eframe::App for App {
    fn ui(&mut self, _ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        todo!()
    }
}

/// Opens the window and blocks until it closes.
pub fn run(_session: Session, _options: Options) -> Result<(), String> {
    todo!()
}
