//! adit-mcp: the tool layer. `Session` holds the open projects, the request queue and export
//! jobs; the MCP servers (stdio, HTTP) and later the GUI all call into it.
//! Contract: docs/MCP_TOOLS.md. Signatures here are fixed; bodies and private items are free.

mod proto;
mod time;
mod tools;

use adit_media::MediaBackend;
use serde_json::Value;
use std::sync::Arc;

pub use proto::{serve_http, serve_stdio};

/// Cheap to clone; all clones share the same state.
#[derive(Clone)]
pub struct Session {
    inner: Arc<tools::Inner>,
}

pub struct ToolOutput {
    pub json: Value,
    pub images: Vec<Image>,
}

pub struct Image {
    pub mime: String,
    pub bytes: Vec<u8>,
}

pub struct ToolInfo {
    pub name: &'static str,
    pub description: String,
    pub input_schema: Value,
}

impl Session {
    pub fn new(media: Arc<dyn MediaBackend>) -> Self {
        Session {
            inner: Arc::new(tools::Inner::new(media)),
        }
    }

    /// Runs one tool by name with JSON arguments; exactly what the MCP server does per call.
    /// `Err` = the message of an `isError` tool result.
    pub fn call(&self, tool: &str, args: Value) -> Result<ToolOutput, String> {
        tools::call(&self.inner, tool, args)
    }

    /// Tool names, descriptions and input schemas, as served by `tools/list`.
    pub fn tools() -> Vec<ToolInfo> {
        tools::list()
    }
}

// --- typed access for an in-process client (the GUI). Contract: docs/GUI.md S-*. ---

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectInfo {
    pub id: String,
    pub name: String,
    pub path: Option<String>,
    pub dirty: bool,
    /// Grows by at least 1 on every change to the project, including undo and redo.
    pub revision: u64,
}

/// The window, as seen by MCP tools (`ui_screenshot`, `ui_get`, `ui_set`).
pub trait UiBridge: Send + Sync {
    /// PNG of the whole window.
    fn screenshot(&self) -> Result<Image, String>;
    /// `{ project_id, sequence_id, playhead, playhead_s, selection, mark_in, mark_out, tool, playing }`.
    fn get(&self) -> Result<Value, String>;
    /// Accepts any of `project_id`, `playhead`, `selection`, `mark_in`, `mark_out`, `tool`.
    fn set(&self, state: &Value) -> Result<(), String>;
}

impl Session {
    /// Open projects in the order they were opened.
    pub fn projects(&self) -> Vec<ProjectInfo> {
        todo!()
    }

    /// Current revision, cheap enough to call every frame. `None` = no such project.
    pub fn revision(&self, _project_id: &str) -> Option<u64> {
        todo!()
    }

    /// A copy of the project and the revision it belongs to.
    pub fn snapshot(&self, _project_id: &str) -> Option<(adit_core::Project, u64)> {
        todo!()
    }

    /// One composited frame of a snapshot, RGBA at sequence size (the `render_frame` path without
    /// scaling or encoding).
    pub fn render(
        &self,
        _project: &adit_core::Project,
        _sequence_id: adit_core::Id,
        _time: adit_core::Time,
    ) -> Result<adit_media::Frame, String> {
        todo!()
    }

    /// Sequence audio of a snapshot: interleaved stereo f32 at the sequence sample rate.
    pub fn mix(
        &self,
        _project: &adit_core::Project,
        _sequence_id: adit_core::Id,
        _start: adit_core::Time,
        _duration: adit_core::Time,
    ) -> Result<Vec<f32>, String> {
        todo!()
    }

    /// A frame of a media file, for the Source monitor.
    pub fn source_frame(
        &self,
        _path: &str,
        _time: adit_core::Time,
    ) -> Result<Arc<adit_media::Frame>, String> {
        todo!()
    }

    /// Attaches (or with `None` detaches) the window the `ui_*` tools talk to.
    pub fn set_ui(&self, _ui: Option<Arc<dyn UiBridge>>) {
        todo!()
    }
}
