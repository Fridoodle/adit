//! adit-core: project model and the single command layer every client (GUI, MCP) edits through.
//! No IO, no GPU. Behaviour is specified in docs/SPEC.md; rule ids there (G1, C-ADD-3, ...) are
//! what the tests cite.

mod command;
mod document;
mod engine;
mod error;
mod model;

pub use command::*;
pub use document::*;
pub use error::*;
pub use model::*;
