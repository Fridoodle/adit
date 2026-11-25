# adit GUI contract (SPEC Part 6)

The window is a second client of `adit_mcp::Session`. It never changes a project except by sending
`adit_core::Command`s through the session, so everything it does is undoable, validated by
`docs/SPEC.md`, and visible to a connected MCP client; and everything Claude does shows up in the
window. API: `crates/adit-gui/src/lib.rs`, session additions in `crates/adit-mcp/src/lib.rs`.

Notation as in SPEC.md. `frame(t)` = `time_to_frame(t, sequence.fps)`, `at(n)` =
`frame_to_time(n, sequence.fps)`, `F` = `at(1)`. "Sequence" = the one named by `ui.sequence_id`.
**Targets** = `ui.targets` restricted to unlocked tracks of the sequence, or all unlocked tracks when
`ui.targets` is empty. **Edit points** = every clip start and end on every track of the sequence, plus
0. **Effective selection** = `ui.selection`, plus the linked sets (SPEC R6) of its clips when
`ui.linked_selection`. Ids in `ui.selection` that no longer exist are ignored.

`perform` and `drag` are pure: they read the project, change only `ui`, and return `Op`s. They never
panic, whatever the state (missing sequence → no ops, `ui` unchanged). An action that has nothing to
do returns no `Op` (never an empty `Edit`). The commands in an `Op::Edit` are applied by the window as
one atomic batch; a batch the core rejects changes nothing and is shown in the status bar.

- **U-CLAR** Targets can be empty (every listed target locked); there is no fall-back to all tracks.
  "No ops, `ui` unchanged" for a missing sequence holds for every action and every gesture. Stale
  selection ids are left out of commands and not counted (`Group`, `ToggleLink`). `Insert` /
  `Overwrite` send `name: None`. Geometry is computed in f64 and converted at the end. A `Select`
  click on an edge zone selects the clip. `RateStretch` on a start edge or body, a `Wand` click on
  empty space, and `Header` / `Nothing` hits with any tool are no-ops. A ruler drag changes only the
  playhead, with every tool. `rows()` of a missing sequence is empty.

## Session additions (`adit_mcp::Session`)

- **S-1** `projects()` lists open projects in opening order with `revision`. `revision(id)` grows by
  at least 1 on every change to that project (edit, undo, redo, import) and does not change on reads,
  renders or failed edits. `snapshot(id)` returns the project and its revision, taken together.
- **S-2** `render(project, sequence_id, time)` returns the same pixels as the `render_frame` tool with
  `format: png` at full size. `mix` returns what `adit_render::mix_audio` returns. `source_frame` is
  `MediaBackend::video_frame`. All three work on the snapshot passed in and do not lock the session
  state while rendering (edits from another thread proceed meanwhile).
- **S-3** `ui_screenshot {}` → image block (PNG) from `UiBridge::screenshot`; `ui_get {}` → the JSON of
  `UiBridge::get`; `ui_set { ...fields }` → `UiBridge::set`. With no window attached all three return
  an error result saying no window is attached. They are listed by `tools/list` always. Time fields of
  `ui_set` (`playhead`, `mark_in`, `mark_out`) accept T-4 strings, with frame strings using the fps of
  the window's current sequence.

## Binary

- `adit gui [project.adit] [--port N]` opens the window and also serves MCP over HTTP exactly like
  `adit serve` (same banner line on stderr, same token rules; default port 7821; if the port is taken
  the window still opens and the status bar says MCP is off). Bad arguments → exit 2 with usage.
  Bare `adit` stays a usage error.

## State

- **U-STATE** `UiState::new(seq)`: playhead 0, empty selection, no marks, tool `Select`, `snap` true,
  `linked_selection` true, not playing, shuttle 0, no targets, no source, `px_per_sec` 100, scroll 0,
  `view_width` 1000.

## Keys

- **U-KEYS** `default_keymap()` is exactly this table (`^` = Ctrl, `⇧` = Shift, `⌥` = Alt; key names
  are `egui::Key` variants). No chord appears twice.

  | Key | Action | Key | Action |
  |---|---|---|---|
  | V | SetTool(Select) | A | SetTool(TrackSelect) |
  | B | SetTool(Ripple) | N | SetTool(Roll) |
  | R | SetTool(RateStretch) | C | SetTool(Razor) |
  | Y | SetTool(Slip) | U | SetTool(Slide) |
  | P | SetTool(Pen) | H | SetTool(Hand) |
  | Z | SetTool(Zoom) | T | SetTool(Type) |
  | ⇧W | SetTool(Wand) | ^⇧W | WandPrompt |
  | Space | PlayPause | K | ShuttleStop |
  | J | ShuttleLeft | L | ShuttleRight |
  | ArrowLeft | Step(-1) | ArrowRight | Step(1) |
  | ⇧ArrowLeft | Step(-5) | ⇧ArrowRight | Step(5) |
  | Home | GoToStart | End | GoToEnd |
  | ArrowUp | PrevEdit | ArrowDown | NextEdit |
  | I | MarkIn | O | MarkOut |
  | X | MarkClip | ^⇧X | ClearInOut |
  | ^⇧I | ClearIn | ^⇧O | ClearOut |
  | ⇧I | GoToIn | ⇧O | GoToOut |
  | ^K | AddEdit | ^⇧K | AddEditAll |
  | Q | RippleTrimPrev | W | RippleTrimNext |
  | Delete | Delete | Backspace | Delete |
  | ⇧Delete | RippleDelete | ⇧Backspace | RippleDelete |
  | Semicolon | Lift | Quote | Extract |
  | Comma | Insert | Period | Overwrite |
  | D | SelectAtPlayhead | ^A | SelectAll |
  | ^⇧A | DeselectAll | ⇧E | ToggleEnable |
  | ⌥ArrowLeft | Nudge(-1) | ⌥ArrowRight | Nudge(1) |
  | ⌥⇧ArrowLeft | Nudge(-5) | ⌥⇧ArrowRight | Nudge(5) |
  | ^L | ToggleLink | ^G | Group |
  | ^⇧G | Ungroup | ^D | ApplyVideoTransition |
  | ^⇧D | ApplyAudioTransition | ⇧D | ApplyDefaultTransitions |
  | M | AddMarker | S | ToggleSnap |
  | F | MatchFrame | Equals | ZoomIn |
  | Minus | ZoomOut | Backslash | ZoomFit |
  | ^Z | Undo | ^⇧Z | Redo |
  | ^S | Save | ^I | Import |
  | ^M | Export | ^R | SpeedDialog |
  | G | GainDialog | | |

  Keys act only when no text field has focus.

## Actions (`perform`)

Transport and marks change `ui` only.

- **U-TOOL** `SetTool(t)`: `ui.tool = t`.
- **U-PLAY** `PlayPause`: if playing → `playing = false`, `shuttle = 0`. Else `playing = true`,
  `shuttle = 1`, and if `playhead >= sequence.duration()` the playhead goes to 0 first.
- **U-JKL** `ShuttleRight`: `shuttle = 1` if `shuttle <= 0`, else `min(shuttle * 2, 8)`; playing.
  `ShuttleLeft`: `shuttle = -1` if `shuttle >= 0`, else `max(shuttle * 2, -8)`; playing.
  `ShuttleStop`: `shuttle = 0`, not playing.
- **U-STEP** `Step(n)`: stops playback (`playing false`, `shuttle 0`); playhead =
  `at(clamp(frame(playhead) + n, 0, frame(sequence.duration())))`. `GoToStart` → 0. `GoToEnd` →
  `at(frame(sequence.duration()))`. Both stop playback.
- **U-EDITNAV** `NextEdit`: playhead = the smallest edit point `> playhead`; none → unchanged.
  `PrevEdit`: the largest edit point `< playhead`; none → unchanged. Both stop playback.
- **U-MARK** `MarkIn`: `mark_in = playhead`; if `mark_out <= mark_in`, `mark_out = None`.
  `MarkOut`: `mark_out = playhead` (exclusive end); if `mark_in >= mark_out`, `mark_in = None`.
  `MarkClip`: over the effective selection if non-empty, else over the clips under the playhead
  (`start <= playhead < end`) on target tracks: `mark_in` = smallest start, `mark_out` = largest end;
  no clips → unchanged. `ClearIn`, `ClearOut`, `ClearInOut` clear. `GoToIn` / `GoToOut` move the
  playhead to the mark if set.
- **U-SELECT** `SelectAtPlayhead`: selection = ids of clips under the playhead on target tracks, in
  track order. `SelectAll`: every clip of the sequence, in track then time order. `DeselectAll`:
  empty.
- **U-TOGGLE** `ToggleSnap`, `ToggleLinkedSelection` flip their flag.
- **U-MATCH** `MatchFrame`: the clip under the playhead on the highest-index target video track that
  has one with a `Media` source → `ui.source = { media_id, playhead: clip.source_time(playhead),
  marks None }`. None → unchanged.
- **U-ZOOM** `ZoomIn`: `px_per_sec = min(px_per_sec * 1.5, 2000)`. `ZoomOut`:
  `max(px_per_sec / 1.5, 1)`. `ZoomFit`: `px_per_sec = clamp(view_width / max(duration_seconds, 1),
  1, 2000)`, `scroll = 0`.

Edits return one `Op::Edit` (unless stated) and adjust `ui` as stated; `ui` changes happen even
though the batch is applied later.

- **U-ADDEDIT** `AddEdit`: if the effective selection is non-empty → one `SplitClip { clip_id, time:
  playhead, include_linked: false }` per effective-selection clip that strictly contains the playhead
  (in track then time order); none → no op. Otherwise `SplitAt { time: playhead, track_ids:
  Some(targets) }` if any clip on a target track strictly contains the playhead, else no op.
  `AddEditAll`: `SplitAt { time: playhead, track_ids: None }` under the same "any clip on an unlocked
  track contains it" condition.
- **U-QW** `RippleTrimPrev`: `a` = largest edit point `< playhead`; `ExtractRange { start: a, end:
  playhead, track_ids: None }`; playhead = `a`. No such point (playhead 0) → no op.
  `RippleTrimNext`: `b` = smallest edit point `> playhead`; `ExtractRange { start: playhead, end: b,
  track_ids: None }`; playhead unchanged. None → no op. Example: one clip `[0, 10s)`, playhead 4 s:
  Q leaves `[0, 6s)` showing source from 4 s with the playhead at 0; W instead leaves `[0, 4s)`.
- **U-DEL** `Delete`: `RemoveClips { clip_ids: selection, include_linked: linked_selection }`;
  selection emptied. `RippleDelete`: the same with `RippleDeleteClips`. Empty selection → no op.
- **U-LIFT** `Lift` / `Extract`: need both marks, else no op. `LiftRange` / `ExtractRange { start:
  mark_in, end: mark_out, track_ids: Some(ui.targets) if non-empty else None }`; playhead = `mark_in`;
  both marks cleared.
- **U-INSERT** `Insert` / `Overwrite`: need `ui.source` and its media in the project, else no op.
  `src_in = source.mark_in or 0`; `src_out = source.mark_out`, else the media duration, else
  `src_in + 5 s` for stills; `duration = src_out - src_in` (`<= 0` → no op). `start = ui.mark_in or
  playhead`. Video track = lowest-index target video track, used if the media has video; audio track =
  lowest-index target audio track, used if the media has audio. Both → `AddClip { track_id: video,
  linked_track_id: Some(audio), .. }`; one → `AddClip` on it; none → no op. `source_in = src_in`,
  `mode` = `insert` / `overwrite`. Afterwards playhead = `at(frame(start + duration))`, selection
  emptied, `ui.mark_in` and `ui.mark_out` cleared.
- **U-NUDGE** `Nudge(n)`: moves the effective selection by `d = n * F` as U-DRAG-MOVE with no track
  change. Empty selection, or a clip that would start before 0 → no op.
- **U-ENABLE** `ToggleEnable`: `SetClip { enabled }` for each effective-selection clip, where the new
  value is `false` if all of them are enabled, else `true`.
- **U-LINK** `ToggleLink` on `ui.selection` (not the effective one): if all selected clips share one
  `Some` link id → `UnlinkClips`; else if at least 2 are selected → `LinkClips`; else no op.
  `Group` (at least 2 selected) → `GroupClips`; `Ungroup` (non-empty) → `UngroupClips`.
- **U-TRANS** `ApplyVideoTransition`: for each target video track, in track order: if a clip ends at
  the playhead and another starts there → `AddTransition { left, right, effect: "cross_dissolve",
  duration: min(1 s, 2 * left.duration, 2 * right.duration) }`; if only a clip ends there → `left`
  only, `duration: min(1 s, clip.duration)`; if only a clip starts there → `right` only, likewise.
  `ApplyAudioTransition`: the same on target audio tracks with effect `"crossfade"`.
  `ApplyDefaultTransitions`: both, video commands first. No cut at the playhead → no op.
- **U-MARKER** `AddMarker`: `AddMarker { time: playhead, duration: 0, name: "", note: "" }`.
- **U-OPS** `Undo`, `Redo`, `Save`, `Import`, `Export`, `SpeedDialog`, `GainDialog`, `WandPrompt`
  return the `Op` of the same name and change nothing. `SpeedDialog` and `GainDialog` return no op
  when the selection is empty.

## Timeline geometry

- **U-GEO-1** `time_to_x(t) = header_width + (t - scroll) * px_per_sec / s` (as f32).
  `x_to_time(x) = scroll + round((x - header_width) * s / px_per_sec)`. They invert each other within
  one pixel.
- **U-GEO-2** `rows()`: video tracks from the highest `tracks` index down to the lowest, then audio
  tracks from the lowest index up (V3, V2, V1, A1, A2). Row `i` covers `y` in
  `[ruler_height + i * track_height, ruler_height + (i + 1) * track_height)`.
- **U-GEO-3** `hit(x, y)`: `y < ruler_height` → `Ruler { time }` (for any `x >= header_width`;
  left of it → `Nothing`). Below the last row or `y < 0` → `Nothing`. In a row with
  `x < header_width` → `Header`. Otherwise with `t = x_to_time(x)`: the clip of that track with
  `start <= t < end` → `Clip`; its `part` is `StartEdge` if `x - time_to_x(start) < edge_px`,
  `EndEdge` if `time_to_x(end) - x <= edge_px`, checked in that order, but only when the clip is at
  least `3 * edge_px` wide on screen; otherwise `Body`. No clip → `Empty`. `time` in every variant is
  `x_to_time(x)`.
- **U-SNAP** `snap(time, .., exclude, tolerance)`: with `ui.snap` off → `at(round-to-nearest frame)`.
  With it on, candidates are 0, the playhead, `mark_in`, `mark_out`, marker times, and the start and
  end of every clip of the sequence not in `exclude`; the nearest candidate within `tolerance`
  (inclusive; ties → the smaller) is returned as is; no candidate in range → nearest frame as above.

## Timeline gestures (`drag`)

`dt = to_time - from.time` for clip/empty hits. A gesture with `dt == 0` and no track change is a
click. Gestures that cannot apply return no op.

- **U-DRAG-SEL** Tool `Select`, click on a clip: selection = `[clip]`; with `shift`, the clip is
  toggled in the selection instead. Click on `Empty` or `Ruler`: selection emptied; a `Ruler` click
  (any tool) also sets playhead = `at(frame(time))` and stops playback. `Ruler` drag (any tool):
  playhead = `at(frame(to_time))`.
- **U-DRAG-MOVE** Tool `Select`, drag of a clip `Body`: if the clip is not in the selection, the
  selection becomes `[clip]` first. Every effective-selection clip moves by `dt`: one `MoveClip
  { clip_id, start: clip.start + dt, track_id, mode: overwrite, include_linked: false }` per clip,
  ordered so no moving clip overwrites another (by start descending when `dt > 0`, ascending when
  `dt < 0`). `track_id` is `Some(to_track)` only when the effective selection is exactly the dragged
  clip, and `to_track` differs from its track and has the same kind; otherwise `None`. Any clip that would start before 0 → no op.
- **U-DRAG-TRIM** Drag of `StartEdge` / `EndEdge`: `TrimClip { clip_id, edge, to: to_time, mode,
  include_linked: ui.linked_selection }` with `mode` = `normal` for `Select`, `ripple` for `Ripple`,
  `roll` for `Roll`. Tool `Ripple` or `Roll` on a `Body` → no op.
- **U-DRAG-RATE** Tool `RateStretch`, drag of `EndEdge`: `new = to_time - clip.start` (`<= 0` → no
  op); `SetClipSpeed { speed: clip.speed * clip.duration / new, ripple: false, include_linked:
  ui.linked_selection }`; a speed outside `[0.01, 100]` → no op.
- **U-DRAG-RAZOR** Tool `Razor`, click on a clip: `SplitClip { clip_id, time, include_linked:
  ui.linked_selection }`; with `shift`: `SplitAt { time, track_ids: None }`. `time` not strictly
  inside the clip → no op.
- **U-DRAG-SLIP** Tool `Slip`, body drag: `SlipClip { delta: -dt, include_linked: linked_selection }`.
  Tool `Slide`, body drag: `SlideClip { delta: dt }`.
- **U-DRAG-TRACKSEL** Tool `TrackSelect`, click on a clip or empty space: selection = every clip on
  that track with `start >= T`, where `T` is the hit clip's start, or the hit time on empty space;
  with `shift`, the clips with `start >= T` on all tracks of the sequence.
- **U-DRAG-VIEW** Tool `Hand`, drag: `scroll = max(0, scroll - dt)`. Tool `Zoom`, click: as `ZoomIn`
  (`alt`: `ZoomOut`).
- **U-DRAG-PEN** Tool `Pen`, click on a clip: `SetKeyframe { target: opacity of the clip, time:
  time - clip.start, value: clip.param("opacity", time - clip.start), easing: linear }`.
- **U-DRAG-WAND** Tool `Wand`, drag starting on a clip or empty space: `mark_in = min(from.time,
  to_time)`, `mark_out = max(..)` (a click marks the clip's range, or nothing on empty space),
  selection = clips of the pressed track intersecting the range; then `Op::WandPrompt`.
- **U-DRAG-TYPE** Tool `Type` does nothing on the timeline (it acts in the Program monitor,
  U-APP-4).

## Window (`App`)

Layout as in Premiere Pro's editing workspace. Checked by headless UI tests (egui_kittest) and by
`ui_screenshot`.

- **U-APP-1 Panels.** Menu bar (File, Edit, Clip, Sequence, Markers, Window, Help); top left: Source
  monitor tabbed with Effect Controls; top right: Program monitor with timecode and transport
  buttons; bottom left: Project bin tabbed with Effects browser; a vertical tool bar; bottom right:
  Timeline with ruler, track headers (lock, mute, solo, hide toggles), clips labelled with their name,
  transitions, markers, in/out range, playhead; audio meters; a Requests panel; a status bar. One tab
  per open project above the panels. Each panel has a visible title; these exact labels exist as
  text: `Project`, `Effects`, `Source`, `Effect Controls`, `Program`, `Timeline`, `Requests`.
- **U-APP-2 Start.** With no project open the window creates one (`project_new`, name `Untitled`).
  With `Options.open` it opens that file; a failure is shown in the status bar and an `Untitled`
  project is used. `project_id()` and `ui_state()` describe the active tab.
- **U-APP-3 Live.** Each frame the window compares `Session::revision` with what it drew; edits made
  by an MCP client appear without user input, and projects opened or closed over MCP add or remove
  tabs. Selection ids that vanish are dropped. The Program monitor shows `Session::render` of the
  sequence at the playhead, rendered off the UI thread (latest request wins); the Source monitor
  shows `Session::source_frame`.
- **U-APP-4 Input.** `run_action(a)` = `perform(a, ..)` then each `Op` in order: `Edit` → the `edit`
  tool with that batch; `Undo` / `Redo` → those tools; `Save` → `project_save` (asks for a path the
  first time); `Import` → file dialog then `media_import`; `Export` → dialog then `export`;
  `SpeedDialog` / `GainDialog` → small modal that issues `SetClipSpeed` / `SetParam volume`;
  `WandPrompt` → U-APP-5. Keys go through `default_keymap()` to `run_action` unless a text field has
  focus. Timeline mouse input goes through `TimelineLayout::hit`, `snap` (tolerance = 8 px in time)
  and `drag`; while dragging, the gesture is previewed without being applied. Double-click in the
  bin loads the Source monitor; dragging from the bin to a track adds the clip there (`AddClip`,
  overwrite, linked audio on the first unlocked audio track when the media has both). The Effects
  browser applies a filter to the selection or a transition at the playhead cut on double-click.
  Effect Controls edits the selected clip: transform, opacity, blend, speed, volume, pan, audio
  filter, text, every effect parameter, each with a keyframe toggle at the playhead. Tool `Type`:
  a click in the Program monitor adds a 5 s `Text` clip at the playhead on the highest video track
  that is free there (adding a track if none is) and selects it.
- **U-APP-5 Magic wand.** `WandPrompt` opens a text box describing the marked range and selected
  clips. A rectangle dragged in the Program monitor with tool `Wand` is included as `rect`.
  Submitting calls `request_create { project_id, sequence_id, text, start: mark_in, end: mark_out,
  clip_ids: selection, rect }`. The Requests panel lists requests with status and, once done, the
  message from `request_complete`.
- **U-APP-6 MCP.** The status bar shows `Options.mcp_url` and a button that copies a ready-to-paste
  `claude mcp add` command including the token; when no client has called a tool yet it says so.
- **U-APP-7 Playback.** While `ui.playing`, the playhead follows a clock at `shuttle` times real
  time and stops at the sequence end (forward) or at 0 (backward). Audio plays at `shuttle == 1`
  through the default output device, mixed ahead in chunks by `Session::mix`; without an audio
  device playback still runs, silent. Meters show the peak of what is playing.
- **U-APP-8 Bridge.** The window registers a `UiBridge`: `screenshot` returns the next painted frame
  as PNG; `get`/`set` read and write the active tab's `UiState` fields named in the trait docs
  (`set` with an unknown `project_id` or a non-existent clip id → `Err`; times are rounded to
  frames).
- **U-APP-10 Labels and clarifications.** Clip names in the timeline, project tab names and
  status-bar messages are real widgets with accessible labels (not painter-only text), so tests and
  screen readers can find them. The wand prompt has a text field labelled `Request` and a button
  `Send`. `ui_set` with `project_id` switches the active tab; frame strings without one use the
  sequence `UiBridge::get` reports. Tool names in `ui_get`/`ui_set` are lower snake case
  (`select`, `track_select`, `rate_stretch`, ...). The usage text lists `adit gui`.
- **U-APP-9 Robustness.** No tool error, missing media file, failed render or absent audio device
  closes or freezes the window; errors go to the status bar. Closing the window with unsaved
  projects asks first.
