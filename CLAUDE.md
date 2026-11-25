# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

**adit** — "Agentic Editor". Rust-based editing software focused on MCP server editing and AI-driven editing.
Repo: https://github.com/Fridoodle/adit (default branch `main`).

Status: greenfield, no crate yet. Rust stable (rustup, MSVC target, clippy + rustfmt) and VS 2022 C++ Build Tools are installed.
When the Cargo workspace is created, replace this line with the real build/lint/test commands, e.g.:

```
cargo build
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test
cargo test -p <crate> <test_name>   # single test
```

## Default working style: caveman + ponytail

Both are on by default for every session in this repo.

- **caveman** (github.com/JuliusBrussee/caveman) — terse output. Answer first, one idea per sentence, no greetings/recaps/chatter between tool calls. Code, commands, paths and error messages stay verbatim. Full sentences only for security warnings, irreversible actions, or a confused user.
- **ponytail** (github.com/DietrichGebert/ponytail) — lazy senior dev. Before writing code, stop at the first rung that holds: does it need to exist → already in this codebase → std lib → platform feature → existing dependency → one line → only then the minimum that works. Never cut validation, error handling, security, or tests for logic with branches/parsing. Mark deliberate shortcuts with `shortcut:` comments. End replies with what was skipped or not checked.

Both plugins are installed at user scope on this machine (slash commands `/caveman`, `/ponytail`, `/ponytail-review`, `/ponytail-audit`). On a new machine there is no `claude` on PATH, so use npx:

```
npx -y @anthropic-ai/claude-code plugin marketplace add JuliusBrussee/caveman
npx -y @anthropic-ai/claude-code plugin install caveman@caveman
npx -y @anthropic-ai/claude-code plugin marketplace add DietrichGebert/ponytail
npx -y @anthropic-ai/claude-code plugin install ponytail@ponytail
```

If the plugins are not loaded, follow the rules above anyway.

## One task per session

When the user brings up a task unrelated to the current one, remind them to start a new session for it (fresh context = fewer tokens). Save anything learned to `knowledge/` first so the new session can pick it up.

## Token budget: query before reading

Order of lookup for any codebase question:

1. `knowledge/INDEX.md` — notes from earlier sessions.
2. Graphify graph (`graphify-out/`), see below.
3. Raw source files — only for lines steps 1–2 already located.

### graphify (code knowledge graph)

Installed as a CLI (`uv tool install graphifyy`) plus project skill (`.claude/skills/graphify/`) and hooks (`.claude/settings.json`, git post-commit/checkout/merge hooks auto-rebuild the graph). Build the first graph with `/graphify .` once code exists.

- When `graphify-out/graph.json` exists, run `graphify query "<question>"` before your first grep, search or source-file read for a codebase question, and put this rule in every subagent prompt that explores code. Read raw files only for lines the graph or an earlier read already located. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. When the answer is under a Markdown heading, `graphify section "<heading>"` prints that section's text, so the .md need not be opened.
- If `graphify-out/wiki/index.md` exists, use it for broad navigation instead of raw source browsing.
- Read `graphify-out/GRAPH_REPORT.md` only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).
- Human visual view (Obsidian-style graph): `graphify export obsidian --dir graphify-out/obsidian`, open that folder as an Obsidian vault. `graphify export html` gives an interactive `graph.html`.
- A committed `graphify-out/graph.json` is rewritten whole on every build; a large diff to it is expected. Do not flag it as churn.

## Knowledge base (always maintain)

`knowledge/` holds durable project knowledge so future sessions don't re-explore the repo. It is Markdown, so graphify indexes it too (`graphify section "<heading>"`).

**When to write:** before ending any task in which you learned something non-obvious that took real reading or debugging to find — an architecture decision and its reason, how subsystems connect, an MCP protocol quirk, a crate gotcha, a build/env fix, a dead end not to retry.

**Do not write:** things the code, git log, or this file already say plainly; task-only scratch details.

**How:**
- One topic per file, kebab-case name, e.g. `knowledge/mcp-transport.md`. Update an existing note instead of creating a duplicate; delete notes that turn out wrong.
- File starts with `# Title`, then a one-line `Summary:`, then the facts. Name the files/modules it concerns so it can be checked against the code. Add `Verified: YYYY-MM-DD`.
- Add or update one line in `knowledge/INDEX.md`: `- [Title](file.md) — hook`.
- Treat notes as hints: if a note names a file or function, confirm it still exists before relying on it.
