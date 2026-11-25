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
