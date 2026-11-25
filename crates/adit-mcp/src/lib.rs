//! adit-mcp: the tool layer. `Session` holds the open projects, the request queue and export
//! jobs; the MCP servers (stdio, HTTP) and later the GUI all call into it.
//! Contract: docs/MCP_TOOLS.md. Signatures here are fixed; bodies and private items are free.

use adit_media::MediaBackend;
use serde_json::Value;
use std::sync::Arc;

/// Cheap to clone; all clones share the same state.
#[derive(Clone)]
pub struct Session {
    _private: (),
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
    pub fn new(_media: Arc<dyn MediaBackend>) -> Self {
        todo!()
    }

    /// Runs one tool by name with JSON arguments; exactly what the MCP server does per call.
    /// `Err` = the message of an `isError` tool result.
    pub fn call(&self, _tool: &str, _args: Value) -> Result<ToolOutput, String> {
        todo!()
    }

    /// Tool names, descriptions and input schemas, as served by `tools/list`.
    pub fn tools() -> Vec<ToolInfo> {
        todo!()
    }
}

/// Serves MCP on stdin/stdout until stdin closes.
pub fn serve_stdio(_session: Session) -> std::io::Result<()> {
    todo!()
}

/// Serves MCP streamable HTTP on 127.0.0.1:`port` at `/mcp`, requiring `Authorization: Bearer <token>`.
pub fn serve_http(_session: Session, _port: u16, _token: String) -> std::io::Result<()> {
    todo!()
}
