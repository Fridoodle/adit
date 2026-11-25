# adit

**adit** ("Agentic Editor") is a video editor written in Rust. Every edit goes through one
command layer. A person can drive it from a Premiere-style window. An AI client can drive it
over MCP. Both see each other's edits live.

## Status

- Headless engine, MCP server and the window work.
- Never exercised by a person yet: real keyboard and mouse in the window, native file dialogs, audio output.

## Requirements

- Rust (edition 2024 toolchain).
- `ffmpeg` and `ffprobe` on `PATH`, or set `ADIT_FFMPEG` / `ADIT_FFPROBE`.
- A GPU supported by wgpu for rendering.

## Build and run

```sh
cargo build --release
cargo run -p adit -- gui [project.adit] [--port N]   # window + MCP over HTTP in one process
cargo run -p adit -- mcp                             # MCP server on stdio, headless
cargo run -p adit -- serve --port 7821               # MCP over HTTP on 127.0.0.1, bearer token on stderr
cargo run -p adit -- render project.adit -o out.mp4  # export without MCP
```

`serve` and `gui` bind loopback only. Every HTTP request needs the bearer token printed on stderr
(or set `ADIT_TOKEN`).

## Connect an MCP client

stdio, for example in Claude Code:

```sh
claude mcp add adit -- adit mcp
```

Typical flow: `project_new` → `media_import` → `edit` → `render_frame` / `render_contact_sheet`
to look → `audio_levels` to listen → `export`. Every edit is undoable. Effects are WGSL shaders
you can define at runtime with `define_effect`. Full tool list and rules: [docs/MCP_TOOLS.md](docs/MCP_TOOLS.md).

## Layout

Crates in `crates/`, each depending on the one before:

| Crate | Role |
|---|---|
| `adit-core` | Project model, `Command` enum, undo. No IO. |
| `adit-media` | ffmpeg subprocesses (probe, decode, encode). |
| `adit-render` | wgpu compositor, WGSL effects, audio mix, export. |
| `adit-mcp` | `Session` tool layer, MCP over stdio and HTTP. |
| `adit-gui` | egui window. |
| `adit` | The binary. |

## Develop

```sh
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo test --workspace
cargo mutants -p adit-core --jobs 2 --timeout 120   # ~25 min
```

Behaviour is specified before it is built. Contracts, with rule ids that tests cite:

- [docs/SPEC.md](docs/SPEC.md) — engine behaviour
- [docs/MCP_TOOLS.md](docs/MCP_TOOLS.md) — MCP tools
- [docs/GUI.md](docs/GUI.md) — window

Order of work: change the spec, write tests from the spec without seeing the implementation,
commit tests, then implement against them. See [CLAUDE.md](CLAUDE.md) for the full workflow.
Durable project notes live in [knowledge/](knowledge/INDEX.md).

## License

None set yet.
