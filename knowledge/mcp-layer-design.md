# MCP layer design notes

Summary: why the MCP server is hand-written, and non-obvious behaviour of the `edit` tool and tests.

Concerns: `crates/adit-mcp/src/{tools,proto,time}.rs`, `crates/adit/src/main.rs`.

- Transport is hand-written JSON-RPC (stdio loop + `tiny_http` for streamable HTTP), not the `rmcp`
  crate: the protocol subset is five methods and this avoids an async runtime. Revisit if resources,
  prompts or server-initiated notifications are needed.
- `Document::apply(Batch)` returns one flat `created` list. The `edit` tool needs per-command lists and
  ordered effect checks, so it dry-runs each command on a scratch `Document` first, then applies one
  real `Batch`.
- `project_new` must not be undoable: it applies `AddSequence`, then wraps the resulting project in a
  fresh `Document`.
- Tool handlers run inside `catch_unwind` with poison-tolerant locks, so a panic in a lower crate is an
  error result. This does not cover stack overflows or allocation aborts; those need input limits
  (SPEC Part 5).
- .NET clients (PowerShell `Process.StandardInput`) send a UTF-8 BOM before their first line; the stdio
  loop strips it.
- Driving the server by hand: see the JSON-RPC sequence in `crates/adit/tests/common/mod.rs`.
- Reading contact sheets: check a cell by cropping it, not by eye on the downscaled sheet. A "stale
  text layer" bug was chased on 2026-11-25 that was only a misread cell.

Verified: 2026-11-25
