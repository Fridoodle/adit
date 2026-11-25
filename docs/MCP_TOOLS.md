# adit MCP contract (SPEC Part 4)

The MCP server is how Claude (or any MCP client) drives adit. It is also the layer the GUI will sit
on: both go through `adit_mcp::Session`, so they see each other's edits. Rules have ids (`T-*`); tests
cite them. Behavioural rules of the edits themselves are in `docs/SPEC.md`.

## Binary

- `adit mcp` — MCP server on stdio (newline-delimited JSON-RPC 2.0, MCP protocol). Headless: no window.
  Nothing but protocol messages is ever written to stdout; logs go to stderr.
- `adit serve [--port N]` — the same server over MCP streamable HTTP on `127.0.0.1:N` (default 7821),
  endpoint `/mcp`. Prints one line `adit serving on http://127.0.0.1:N/mcp token=<token>` to stderr.
  Every request must carry `Authorization: Bearer <token>` (random per launch, or env `ADIT_TOKEN`);
  otherwise HTTP 401. It never binds to a non-loopback address.
- `adit render <project.adit> -o <out> [--sequence ID] [--codec h264|h265|prores|gif]` — export without
  MCP. Exit code 0 on success, non-zero with a message on stderr otherwise.
- `adit --help` / unknown arguments: usage on stderr, exit code 2 for bad usage.
- Env `ADIT_MEDIA=pattern` makes the server use the procedural `Pattern` backend (64x64, 30 fps,
  10 s, for tests and demos) instead of ffmpeg for decoding; export still encodes with ffmpeg.

## General rules

- **T-1** `initialize` reports server name `adit` and tool capability. `tools/list` returns every tool
  below with a JSON schema for its input. `instructions` in the initialize result contains the time
  formats (T-4) and the effect shader contract (SPEC E-1), so a client needs no other documentation.
- **T-2** A tool failure is a normal tool result with `isError: true` and a text message that names the
  cause (the `adit_core::Error` text, ffmpeg's message, ...). The server keeps running; a failed call
  changes nothing (SPEC G1). Unknown tool / malformed params → JSON-RPC error, server keeps running.
- **T-3 Many projects.** The server holds any number of open projects, each with its own undo history.
  `project_id` is a short string chosen by the server (`p1`, `p2`, ...), never reused within a run.
  Every project tool takes `project_id`; an unknown one → error. Edits to one project never affect
  another.
- **T-4 Times.** Wherever a tool or an edit command takes a time or duration it accepts either an
  integer (flicks, 705,600,000 per second) or a string: `"1.5s"` (seconds), `"45f"` (frames at the
  sequence's frame rate, exact), `"00:00:01:15"` (h:m:s:frames). Negative values (`"-0.5s"`, `"-3f"`)
  are accepted where the command allows them (e.g. `delta`). A malformed string → error naming the
  field. Conversion: seconds → `round(x * 705600000)`; frames → SPEC Q4. Inside `edit` commands this
  applies to the fields `start`, `end`, `duration`, `time`, `to`, `delta`, `source_in`. Frame-based
  strings use the fps of the sequence named by the tool's `sequence_id` argument, else the project's
  first sequence; with no sequence → error.
- **T-5** Results are one text content block holding JSON (compact), plus image blocks where stated.
  Times in results are given both ways where useful: `*_s` (seconds, f64) and raw flicks.
- **T-6 Sequence default.** Tools with an optional `sequence_id` use the project's first sequence when
  it is omitted; no sequence → error.

## Tools

### Projects

- **T-NEW** `project_new { name, width?=1920, height?=1080, fps?="30" , sample_rate?=48000,
  video_tracks?=2, audio_tracks?=2 }` → `{ project_id, sequence_id, track_ids }`. `fps` is `"30"`,
  `"29.97"` (→ 30000/1001), `"23.976"` (→ 24000/1001), `"59.94"`, or `"num/den"`. The creation is not
  undoable (undo depth 0 afterwards).
- **T-OPEN** `project_open { path }` → `{ project_id }`. Reads an `.adit` JSON file; an invalid file
  (bad JSON, fails `validate`) → error. **T-SAVE** `project_save { project_id, path? }` writes the
  project JSON (pretty-printed) atomically (temp file + rename); `path` is remembered; saving a never
  saved project without `path` → error. Open(Save(p)) gives an equal project.
- **T-CLOSE** `project_close { project_id }`. **T-LIST** `project_list {}` → `[{ project_id, name,
  path, dirty }]` where `dirty` = changed since last save/open.
- **T-GET** `project_get { project_id, detail?="summary" }`.
  `"full"` → `{ project: <the adit_core::Project JSON> , undo_depth, redo_depth }`.
  `"summary"` → `{ project_id, name, path, undo_depth, redo_depth, media: [{ id, name, path,
  duration_s, video: {width,height,fps}, audio: {sample_rate,channels} }], sequences: [{ id, name,
  width, height, fps: "num/den", duration_s, duration_frames, tracks: [{ id, name, kind, locked, muted,
  solo, hidden, clips: [{ id, name, source, start_s, end_s, start_frame, end_frame, source_in_s, speed,
  enabled, link, effects: [names] }], transitions: [{ id, left, right, effect, duration_s }] }],
  markers: [{ id, time_s, name }] }], effects: [names of project-defined effects] }`.
  `source` is `"media:<id>"`, `"sequence:<id>"`, `"text:<first 40 chars>"`, `"solid"`, `"adjustment"`.

### Media

- **T-IMPORT** `media_import { project_id, paths: [..] }` probes each file and adds it (one undo step
  for the call). → `{ media: [{ id, name, duration_s, video, audio }] }`. Any file failing → error and
  nothing is added. Relative paths are resolved against the server's working directory and stored
  absolute.
- **T-SILENCE** `detect_silence { project_id, media_id, noise_db?=-35, min_duration?="0.4s" }` →
  `{ ranges: [{ start_s, end_s }] }`. **T-SCENES** `detect_scenes { project_id, media_id,
  threshold?=0.3 }` → `{ cuts_s: [..] }`. (Unavailable with `ADIT_MEDIA=pattern` → error.)
- **T-LEVELS** `audio_levels { project_id, sequence_id?, start?=0, end?=sequence end, buckets?=50 }` →
  `{ bucket_s, rms_db: [..], peak_db: [..] }` of the sequence mix (−120 for silence).

### Editing

- **T-EDIT** `edit { project_id, sequence_id?, commands: [Command, ...] }` applies the commands as one
  atomic batch = one undo step (SPEC C-BATCH-1). `Command` is the serde JSON of `adit_core::Command`
  (`{"op": "add_clip", ...}`), with T-4 time strings allowed. → `{ created: [[ids of command 0],
  [ids of command 1], ...], undo_depth }`. The tool description lists every `op` with its fields.
  Extra checks done here, before anything is applied (failure → error, nothing changes):
  `add_effect` and `add_transition` must name an effect that exists (built-in or project-defined) with
  the right kind; `define_effect` must pass `validate_effect`; a `define_effect` earlier in the same
  batch makes the name usable later in the batch.
- **T-UNDO** `undo { project_id, steps?=1 }`, `redo { project_id, steps?=1 }` →
  `{ done: <steps actually performed>, undo_depth, redo_depth }`.

### Effects

- **T-FXLIST** `effect_list { project_id?, include_source?=false }` → `{ effects: [{ name, kind,
  builtin, params: [{name, default, min, max}], wgsl? }] }` (project-defined ones included when
  `project_id` is given).
- **T-FXCHECK** `effect_check { kind, wgsl, params }` → `{ ok: true }` or `{ ok: false, error }`.
  Lets the client test a shader without touching a project.

### Seeing

- **T-FRAME** `render_frame { project_id, sequence_id?, time?=0, max_size?=640, format?="jpeg" }` → an
  image block (`image/jpeg` or `image/png`) of the composited frame, scaled down (never up) so the
  longer side is at most `max_size`, plus text `{ time_s, frame, width, height }`.
- **T-SHEET** `render_contact_sheet { project_id, sequence_id?, start?=0, end?=sequence end, count?=9,
  columns?=3, cell_size?=320 }` → one image block: `count` frames at evenly spaced times
  `start + (i + 0.5) * (end - start) / count`, laid out row by row, each scaled so its longer side is
  `cell_size`; plus text `{ times_s: [..], columns, cell_width, cell_height }`.

### Export

- **T-EXPORT** `export { project_id, sequence_id?, path, codec?="h264", audio?="aac", width?, height?,
  fps?, crf? }` starts a background job on a snapshot of the project (later edits do not affect it) →
  `{ job_id }`. **T-JOB** `job_status { job_id }` → `{ state: "running" | "done" | "failed" |
  "cancelled", frames_done, frames_total, error? , path }`. `job_cancel { job_id }`. `job_wait
  { job_id, timeout_s?=60 }` blocks until the job leaves `running` or the timeout passes, then returns
  the status.

### Magic-wand requests

A request is the user pointing at something and saying what they want. The GUI files them; any client
can too.

- **T-REQ-1** `request_create { project_id, sequence_id?, text, start?, end?, clip_ids?=[], rect? }` →
  `{ request_id }`. `rect` is `[x, y, w, h]` in 0..1 of the frame. Status starts `pending`.
- **T-REQ-2** `request_wait { timeout_s?=60 }` (max 300) returns the oldest `pending` request as
  `{ request: { request_id, project_id, sequence_id, text, start_s, end_s, start, end, clip_ids, rect } }`
  and marks it `claimed`; if none arrives before the timeout → `{ request: null }`. A request created
  while a `request_wait` is blocking wakes it promptly (< 1 s). Each request is handed out once.
- **T-REQ-3** `request_list { status? }` → all requests with their status. `request_complete
  { request_id, message?="" }` marks it `done` with the message (shown to the user in the GUI).
  Unknown id → error.

## Library surface (`adit_mcp`)

```rust
pub struct Session;                       // cheap to clone (Arc inside); Send + Sync
impl Session {
    pub fn new(media: std::sync::Arc<dyn adit_media::MediaBackend>) -> Self;
    /// Runs one tool by name with JSON arguments; exactly what the MCP server does per call.
    pub fn call(&self, tool: &str, args: serde_json::Value) -> Result<ToolOutput, String>;
    /// Tool names, descriptions and input schemas, as served by `tools/list`.
    pub fn tools() -> Vec<ToolInfo>;
}
pub struct ToolOutput { pub json: serde_json::Value, pub images: Vec<Image> }
pub struct Image { pub mime: String, pub bytes: Vec<u8> }
pub struct ToolInfo { pub name: &'static str, pub description: String, pub input_schema: serde_json::Value }
pub fn serve_stdio(session: Session) -> std::io::Result<()>;
pub fn serve_http(session: Session, port: u16, token: String) -> std::io::Result<()>;
```
