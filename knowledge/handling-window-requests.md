# Handling requests from the window

Summary: requests typed in `adit gui` live in the GUI process; the `.mcp.json` stdio server cannot see them.

- `.mcp.json` starts `target/release/adit.exe mcp`: its own headless `Session`, no projects, no requests.
- `adit gui` prints `adit serving on http://127.0.0.1:<port>/mcp token=<hex>` to the terminal.
  Call that server with JSON-RPC `tools/call` over POST, header `Authorization: Bearer <token>`.
  No `initialize` handshake or session header is needed.
- `request_list`, then `request_wait` (claims it), work with `edit`, then `request_complete`.
- Locating game moments (e.g. League "SHUT DOWN" banners): `render_contact_sheet` with a narrow
  range and ~0.2 s spacing; image comes back base64 in `content[].data`.
- Timed overlay on an existing clip: a filter with a `start` param compared against `ctx.time`
  (seconds since clip start) works; the same effect can be added twice to one clip.
  Example: library effect `explosion_burst`.

Verified: 2026-11-25
