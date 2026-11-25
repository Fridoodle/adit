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

## Rules added after the GUI review (U-FIX-*)

Logic (`perform`, `drag`):

- **U-FIX-1 Locked tracks.** The effective selection never contains clips on locked tracks: they are
  left out before any command is built (AddEdit, Delete, RippleDelete, Nudge, ToggleEnable, moves).
  Linked partners on locked tracks are left out the same way. If nothing remains → no op.
- **U-FIX-2 Edge clicks.** A click (`dt == 0`, no track change) on a `StartEdge` / `EndEdge` never
  trims: with `Select` it selects (U-CLAR), with `Ripple`, `Roll` or `RateStretch` it is a no-op.
- **U-FIX-3 Hand.** The window passes the unsnapped pointer time to `drag` for tool `Hand`.

Window:

- **U-FIX-4 Click times.** For clicks and drags alike the window replaces `from.time` and `to_time`
  with `snap(..)` of the pointer time (tool `Hand` excepted), so every edit made with the mouse lands
  on a snap candidate or a frame boundary. A Razor click with snapping off splits exactly on a frame.
- **U-FIX-5 Effect Controls hold edits.** Text typed into the text-clip content field, the font
  field and the audio-filter field stays in the field while it has focus and is committed as one
  command when focus leaves the field or the selected clip changes; numeric fields commit when the
  drag or edit ends. An edit made over MCP to a field that is not being edited shows up at once.
  Speed uses `include_linked: ui.linked_selection`.
- **U-FIX-6 Dialogs remember their target.** Speed, Gain and Wand dialogs act on the project and
  clips captured when they opened, whatever tab or selection is active when they are confirmed. If
  the project is gone the dialog closes with a status message.
- **U-FIX-7 Layout.** Track rows scroll vertically inside the Timeline panel; the number of tracks
  never changes the height of the panels. With 18 tracks at a 1600x1000 window the Program monitor
  image is at least 200 px high, and it is the same height as with 4 tracks. Adjacent clips are
  visually separated (1 px outline).
- **U-FIX-8 Sequences.** The Timeline header has a sequence selector listing the project's
  sequences by name; choosing one sets `ui.sequence_id` (playhead 0, selection and marks cleared).
  `ui_get` reports it; `ui_set { sequence_id }` switches (unknown id → `Err`). If the current
  sequence is removed the tab falls back to the first one.
- **U-FIX-9 Bridge.** `UiBridge::set` validates before accepting: no open project → `Err`; times
  must be within `0..=MAX_TIME`; the resulting `mark_in` must be `<` the resulting `mark_out` when
  both are set; unknown tool name → `Err`. An accepted `set` is visible to `get` immediately (same
  call order as issued), also when the window is minimized or not repainting; it reaches the drawn
  state on the next frame. A `ui_set { project_id }` naming a project opened over MCP a moment ago
  works (the window syncs before applying).
- **U-FIX-10 Unsaved work.** File > Close Project on a project with unsaved changes asks first.
  A project closed by an MCP client leaves a status message naming it (and saying so if it had
  unsaved changes). The quit prompt appears only if some open project has `dirty` and
  `revision > 0` (a fresh untouched `Untitled` does not count) and goes away when that stops being
  true.
- **U-FIX-11 Offline media.** After opening or syncing a project, media whose file cannot be read
  (`Session::source_frame` fails) is reported once in the status bar as `media offline: <name>` and
  its clips are labelled `[offline] <name>` in the timeline.
- **U-FIX-12 Type tool.** Adding a track and the text clip is one batch (one undo step); the new
  track id is `project.next_id` before the batch.
- **U-FIX-13 Misc.** `adit gui --port 0` → exit 2. Audio stream errors reach the status bar. An idle
  window repaints no more often than every 500 ms. Effects-browser filters go only to selected clips
  on unlocked video tracks.

- **U-FIX-14 Labels and readings the tests rely on.** The text-clip content field is the only
  multi-line text input while no dialog is open. The Speed dialog has one numeric field and an `OK`
  button. The sequence selector shows the current sequence name and, when opened, one entry per other
  sequence by name. The Close Project confirmation has a `Cancel` button; the quit prompt has the
  heading `Unsaved changes`. With linked partners on locked tracks, Delete / RippleDelete name the
  remaining clips explicitly with `include_linked: false`. In `media offline: <name>` and
  `[offline] <name>`, the first is the media name, the second the clip name.

- **U-FIX-15 Settled conflicts.** A project closed over MCP is named in the status message by its
  name only when unsaved changes were lost, otherwise by its id (U-APP-3's tab text must disappear).
  An edit action that does nothing because every selected clip is on a locked track puts
  `track N is locked` in the status bar. In `ui_set`, frame strings resolve at the fps of the
  sequence that was current before the call, also when the same call switches `sequence_id`.
  The offline check covers media with video only.

## Editing comfort (U-ED-*)

New public items in `crates/adit-gui/src/lib.rs`:

```rust
pub struct ClipboardItem { pub track_id: Id, pub clip: adit_core::Clip }   // Debug, Clone, PartialEq
// Action gains: Copy, Cut, Paste, Unlink, SetLabel(u8), CloseGap { track_id: Id, time: Time }
// Op gains: Copy, Paste
pub fn copy(project: &Project, ui: &UiState) -> Vec<ClipboardItem>;
pub fn paste(items: &[ClipboardItem], project: &Project, ui: &mut UiState) -> Vec<Op>;
pub fn transition_at(project: &Project, sequence_id: Id, track_id: Id, time: Time, effect: &str) -> Vec<Op>;
pub fn snap_delta(delta: Time, clips: &[Id], project: &Project, ui: &UiState, tolerance: Time) -> Time;
pub fn preview(drag: Drag, project: &Project, ui: &UiState) -> Option<Project>;
pub enum MenuItem { Do(Action), Labels, Transitions }                      // Debug, Clone, PartialEq
pub fn context_menu(hit: Hit, has_clipboard: bool, project: &Project, ui: &mut UiState) -> Vec<MenuItem>;
// impl App: pub fn ai_import(&mut self, paths: Vec<String>);
```

Logic (pure, same guarantees as `perform`: never panics, missing sequence → nothing):

- **U-ED-1 Copy, Cut.** `copy` = the effective selection as `(track_id, clip)`, in track order (index in
  `tracks` ascending) then by start. `perform(Copy)` → `[Op::Copy]`, or no op when the effective
  selection is empty. `perform(Cut)` → `[Op::Copy, Op::Edit(..)]` where the edit and the `ui` change
  are those of `Delete` (U-DEL); empty → no op. `perform(Paste)` → `[Op::Paste]` always.
  Keys (added to U-KEYS): `^C` Copy, `^X` Cut, `^V` Paste.
- **U-ED-2 Paste.** `paste`: `base` = smallest `clip.start` of `items`, `span` = largest clip end −
  `base`. Items whose track is not in the current sequence or is locked are left out; none left (or
  `items` empty) → no op, `ui` unchanged. Else one `Op::Edit([PasteClips { items, mode: overwrite }])`
  with each kept clip's `start` = `playhead + (clip.start - base)`, in the order given. Afterwards
  playhead = `at(frame(playhead + span))`, selection emptied.
- **U-ED-3 Unlink.** `Unlink`: `UnlinkClips { clip_ids }` naming the effective-selection clips that have
  `link: Some(_)` (track order, then start); none → no op. Selection unchanged.
- **U-ED-4 Label.** `SetLabel(n)`: one `SetClip { clip_id, label: Some(n) }` (every other field `None`)
  per effective-selection clip (track order, then start), one batch. `n > 8` or empty → no op.
- **U-ED-5 Close gap.** `CloseGap { track_id, time }` → `Op::Edit([Command::CloseGap { track_id, time
  }])` when the track is in the sequence and unlocked, `time` is inside no clip of it
  (`start <= time < end`), and a clip of it starts after `time`. Otherwise no op.
- **U-ED-6 Transition at a point.** `transition_at`: track not in the sequence, or locked → no op.
  If a clip `C` of the track has `start <= time < end`: the edge is the start when
  `time - C.start < C.end - time`, else the end. End edge: with a clip `N` on the track starting at
  `C.end` → `AddTransition { track_id, left: C, right: N, effect, duration: min(1 s, 2 * C.duration,
  2 * N.duration), params: {} }`; without → `left: C` only, `duration: min(1 s, C.duration)`.
  Start edge: mirrored with a clip `P` ending at `C.start` (`left: P, right: C`), else `right: C`
  only. If `time` is in a gap: `L` = last clip ending at or before `time`, `R` = first clip starting
  after it; both must exist, else no op; → one `Op::Edit([CloseGap { track_id, time }, AddTransition
  { track_id, left: L, right: R, effect, duration: min(1 s, 2 * L.duration, 2 * R.duration) }])`.
- **U-ED-7 Snap for moves.** `snap_delta`: `clips` are the moving clips (unknown ids ignored). With
  `ui.snap` off, or no candidate in range → `delta` rounded to the nearest multiple of `F`
  (`at(round)` of `|delta|`, sign kept). With it on: candidates as U-SNAP excluding `clips`; for
  every moving clip both `start + delta` and `end + delta` are compared with every candidate; the
  pair with the smallest distance `<= tolerance` wins (ties → the smaller resulting delta) and the
  result is `delta + (candidate - edge)`. Example: clip `[2s, 4s)` moving, other clip `[5s, 8s)`,
  tolerance 0.1 s, `delta` 0.95 s → 1 s (its end lands on 5 s).
- **U-ED-8 Preview.** `preview` runs `drag` on copies and applies its `Op::Edit` batches to a copy of
  the project: `Some(result)` when there is at least one edit and the core accepts all of them, else
  `None`. Neither `project` nor `ui` changes.
- **U-ED-9 Context menu.** `context_menu` on a `Clip` hit: if the clip is not in the effective
  selection, selection = `[clip]` first. Items in this order: `Do(Cut)`, `Do(Copy)`, `Do(Paste)` (only
  with `has_clipboard`), `Do(Delete)`, `Do(RippleDelete)`, `Do(Unlink)` (only if an
  effective-selection clip has a link), `Do(Group)` (only with at least 2 clips in `ui.selection`),
  `Do(Ungroup)` (only if an effective-selection clip has a group), `Do(ToggleEnable)`,
  `Do(SpeedDialog)`, `Do(GainDialog)`, `Labels`, `Transitions`, `Do(WandPrompt)`.
  On an `Empty` hit (selection unchanged): `Do(Paste)` (only with `has_clipboard`),
  `Do(CloseGap { track_id, time })` (only if U-ED-5 gives an op), `Transitions` (only if U-ED-6 gives
  an op for that track and time), `Do(WandPrompt)`. Any other hit → empty, `ui` unchanged.

Window:

- **U-ED-10 Right click.** A right click on the timeline opens a menu built from `context_menu` (hit
  time rounded to a frame, not snapped). Button texts: `Cut`, `Copy`, `Paste`, `Delete`,
  `Ripple Delete` (also for `CloseGap`), `Ungroup Audio/Video` (Unlink), `Group`, `Ungroup Clips`,
  `Enable/Disable`, `Speed...`, `Audio Gain...`, `Ask AI...`; submenu `Label` with `None`, `Violet`,
  `Iris`, `Caribbean`, `Lavender`, `Cerulean`, `Forest`, `Rose`, `Mango` (→ `SetLabel(0..=8)`);
  submenu `Add Transition` listing the transition effects of `effect_list` on a video track and
  `crossfade` on an audio track (→ `transition_at` with the hit track and time). `Op::Copy` stores
  `copy(..)` in a clipboard shared by all tabs; `Op::Paste` runs `paste` with it. The egui
  copy / cut / paste events act as `^C` / `^X` / `^V` when no text field has focus.
- **U-ED-11 Tool bar.** Each tool button shows an icon, not text. Its accessible label is the tool
  name: `Selection`, `Track Select`, `Ripple Edit`, `Rolling Edit`, `Rate Stretch`, `Razor`, `Slip`,
  `Slide`, `Pen`, `Hand`, `Zoom`, `Type`, `Magic Wand`. Hovering shows the name with its key and one
  sentence saying what the tool does.
- **U-ED-12 Snapping.** The Timeline header has a toggle button `Snap` (selected when on) that runs
  `ToggleSnap`. A `Select` body drag uses `snap_delta` over the moving clips (not the pointer snap of
  U-FIX-4), so a clip's start or end lands on the candidate; while a drag is snapped a vertical line
  is drawn at the candidate. `ui_get` reports `snap`; `ui_set { snap: bool }` sets it.
- **U-ED-13 Live result.** While a gesture is dragged the timeline draws the clips of `preview(..)`
  instead of the current ones, so clips that would be cut, overwritten or left with a gap show it
  before release; ranges that held a clip before and hold none or another one afterwards are tinted
  red. With `None` the outline of U-APP-4 is drawn.
- **U-ED-14 Timeline looks.** A transition is a box over its time range at the top of the row with a
  diagonal and, when at least 60 px wide, its effect name. A clip with `label != 0` is filled with
  that label's colour. Audio clips of media draw the waveform of S-4 for their source range
  (following `source_in`, `speed`, `reverse`), computed off the UI thread.
- **U-ED-15 Uniform scale.** Effect Controls has a checkbox `Uniform Scale`, checked by default. While
  checked, an edit of Scale X or Scale Y sets both params to the new value in one batch (one undo
  step), and their keyframe toggle acts on both.
- **U-ED-16 File drop.** Files dropped on the window are imported with one `media_import`. If the
  pointer is over a timeline track, the imported media are also placed there as a bin drop
  (U-APP-4), the first at the pointer time and each next one where the previous ends. Errors go to
  the status bar.
- **U-ED-17 Preview resolution.** The Program monitor has a selector showing `Full`, `1/2` or `1/4`
  (default `1/2`); the picture is rendered with `Session::render_scaled` (S-5) at that divisor.
  `ui_get` reports `preview_scale` (1, 2 or 4); `ui_set { preview_scale }` sets it (other → `Err`).
- **U-ED-18 Wand anywhere.** With tool `Wand`, a click or dragged rectangle on any panel other than
  the Timeline, the Program monitor, the tool bar, the menu bar and the project tabs opens the wand
  prompt for the app: it names the panel, and `Send` calls `request_create { scope: "app", panel,
  rect (0..1 of the window, omitted for a click), text, project_id, sequence_id }`. `Edit > Ask AI to
  Change adit...` opens the same prompt without a panel. The Requests panel shows the scope.
- **U-ED-19 AI import.** `File > AI Import...` and `AI Import...` in the Project panel's right-click
  menu ask for files and then call `App::ai_import(paths)`, which opens a dialog listing the files
  with a text field labelled `Instructions` and a button `Send`; `Send` calls `request_create
  { scope: "import", paths, text, project_id, sequence_id }`. Nothing is imported by the window.
- **U-ED-20 Library.** The Effects browser lists library effects (T-LIB-3) marked as such; applying
  one works like any other effect.

- **U-ED-21 Clarifications.** `Op::Copy` also puts the text `adit clips` on the system clipboard, so
  the OS delivers a paste event for `^V`. A paste into another project works only for items whose
  track id exists there (`shortcut`). A menu closes after a choice; `Label` and `Add Transition` are
  submenu buttons whose entries appear when clicked. The tolerance of `snap_delta` in the window is
  8 px in time, as for `snap`. Effect Controls numeric fields are labelled by their names, among
  them `Scale X` and `Scale Y`. A file drop snaps the pointer time and sizes the clips as a bin drop
  does; the import is one undo step and the placement one more. `panel` in a wand request is the
  U-APP-1 title of the panel (`Project`, `Effects`, `Source`, `Effect Controls`, `Requests`), or
  `Meters` or `Status`; a click on the panel's title counts. The Requests panel puts `[app]` or
  `[import]` in front of the text of such requests. The AI import dialog closes after `Send`.
  A library effect is listed in the Effects browser as `<name> (library)`.
- **U-ED-22 Logic clarifications.** U-CLAR's "missing sequence → no ops" also holds for `Paste`;
  `snap_delta` then returns `delta` as given. `Copy`, `SetLabel` and `CloseGap` leave `ui` unchanged.
  In `paste`, `base` and `span` are taken over all items, before any is left out; an item whose
  clip does not fit its track's kind is sent as is and the core rejects the batch. In `snap_delta`
  "smaller" means the smaller signed value; the rounding of an exact half frame is not specified.
  `context_menu` on a clip of a locked track or with an unknown clip id → empty, `ui` unchanged;
  whether `Transitions` is offered never depends on the effect. `Unlink` leaves locked partners
  alone (U-FIX-1). `transition_at` on a gap returns the batch even if the core will then refuse
  the `CloseGap` (`RippleBlocked`); the status bar shows it.

- **U-ED-23 Rules added after the review.**
  (a) An egui cut event that arrives with Shift held and neither Ctrl nor Command (Windows sends
  Shift+Delete that way) runs `RippleDelete`, not `Cut`; the clipboard is untouched.
  (b) The drag preview is computed once per change of the gesture (press hit, pointer time, track,
  modifiers) or project revision, not every frame.
  (c) Dropped files with no known pointer position are imported at once and placed (U-ED-16) at the
  first pointer position reported afterwards, if that is over a track.
  (d) With tool `Wand`, a press on a panel of U-ED-18 reaches no widget of that panel: a double
  click on an effect in the Effects browser applies nothing and only opens the prompt.
  (e) The Effects browser re-reads `effect_list` at least every 2 s while it is shown, so library
  effects saved elsewhere appear.
  (f) A failed waveform is tried again after the project's revision changes.
  (g) A paste into a project other than the one the clips were copied from leaves out items whose
  source is `Media` or `Sequence`; if none is left, nothing is pasted and the status bar says so.
  (h) `snap_delta` returns without panicking for every `i64` `delta` and `tolerance` (values are
  clamped to `±MAX_TIME` first).

## Timeline navigation (U-NAV-*)

`S` = 705,600,000 flicks per second. `visible_span(ui) = round(view_width * S / px_per_sec)` flicks:
the time the timeline area shows. All math is f64, rounded to the nearest flick at the end; where
`px_per_sec` is not a whole number, times in the examples hold within 1 flick and `px_per_sec`
within 1e-9.

Logic (public in `adit_gui`):

- **U-NAV-1 Zoom at a point.** `zoom_at(ui, factor, anchor: Time)`: `new = clamp(px_per_sec *
  factor, 1, 2000)`; `scroll = max(0, anchor - round((anchor - scroll) * px_per_sec / new))` with the
  old `px_per_sec`; then `px_per_sec = new`. The anchor stays at the same screen x (unless the scroll
  hits 0). `factor` not finite or `<= 0` → `ui` unchanged. Examples (view 1000 px): pps 100, scroll 0,
  anchor 4 s, factor 2 → pps 200, scroll 2 s. Pps 100, scroll 0, anchor 1 s, factor 0.5 → pps 50,
  scroll 0. Pps 1500, scroll 10 s, anchor 10 s, factor 2 → pps 2000, scroll 10 s.
- **U-NAV-2 Zoom bar.** `zoom_bar_total(ui, duration) = max(duration, scroll + visible_span(ui), S)`.
  `zoom_bar(ui, duration) -> (f64, f64)` = `(scroll / total, (scroll + visible_span) / total)`, the
  thumb as fractions of the bar. Example: pps 100, view 1000 px (visible 10 s), duration 40 s, scroll
  5 s → `(0.125, 0.375)`; duration 4 s, scroll 0 → total 10 s → `(0, 1)`.
  `bar_grab(ui, duration, part: BarPart) -> BarGrab` records `part` (`Thumb`, `Start`, `End`) and, at
  that moment, `scroll`, `visible = visible_span(ui)` and `total = zoom_bar_total(ui, duration)`
  (public fields of the same names). `bar_drag(ui, &grab, d)`: `d` is the pointer's movement since the
  press as a fraction of the bar's width (any sign); `dt = round(d * total)`, `end0 = grab.scroll +
  grab.visible`. The result depends only on `grab`, `d` and `ui.view_width`, so calling it every frame
  with the growing `d` is the same as calling it once.
  - `Thumb`: `scroll = clamp(grab.scroll + dt, 0, max(0, total - visible))`; `px_per_sec` unchanged.
  - `Start` (right end fixed): `start = clamp(grab.scroll + dt, 0, end0 - 1)`; `px_per_sec =
    clamp(view_width * S / (end0 - start), 1, 2000)`; `scroll = max(0, end0 - visible_span(ui))` with
    the new `px_per_sec`.
  - `End` (left end fixed): `end = max(end0 + dt, grab.scroll + 1)`; `px_per_sec = clamp(view_width *
    S / (end - grab.scroll), 1, 2000)`; `scroll = grab.scroll`.
  Examples, grab at pps 100, view 1000 px, scroll 5 s, duration 40 s (visible 10 s, total 40 s, end0
  15 s): `Thumb` d 0.25 → scroll 15 s; d 1 → 30 s; d -0.5 → 0. `Start` d -0.125 → pps 1000/15,
  scroll 0; d 0.125 → pps 200, scroll 10 s; d 1 → pps 2000, scroll 14.5 s. `End` d 0.25 → pps 50,
  scroll 5 s; d -0.25 → pps 2000, scroll 5 s; d 100 → pps 1, scroll 5 s.

Window:

- **U-NAV-3 Zoom bar strip.** Below the track rows the Timeline panel has a strip 14 px high from the
  header's right edge to the panel's right edge, with accessible label `Zoom bar`. The rows end above
  it; its height is fixed, so U-FIX-7 still holds. The thumb is drawn from `left + a * W` to `left +
  b * W` (`(a, b) = zoom_bar(..)` with the sequence duration, `W` the strip width), at least 16 px
  wide. A primary press inside the drawn thumb less than 6 px from its left end grabs `Start`, less
  than 6 px from its right end `End`, elsewhere `Thumb` (`bar_grab` at press); a press on the strip
  outside the thumb does nothing. While dragged, each frame applies `bar_drag(ui, grab, dx / W)`,
  `dx` = pointer x − press x. Nothing in the project changes.
- **U-NAV-4 Wheel.** Over the timeline area (right of the headers), wheel with Ctrl or Alt held
  zooms: each frame `zoom_at(ui, f, t)` with `f = exp(dy / 200)`, `dy` = that frame's smoothed
  vertical wheel delta in points (for Ctrl, egui's `zoom_delta()`, which is the same formula), so
  wheel up zooms in; `t` = the unsnapped `x_to_time` of the pointer, so the time under the pointer
  stays under it (within one pixel). Plain wheel scrolls the tracks vertically when they
  overflow, else the time; Shift + wheel scrolls the time (as before).
- **U-NAV-5 Drag to pan.** These drags pan the view live instead of editing: a primary drag with
  tool `Select` that starts on `Empty` space of a track row, a primary drag with tool `Hand` that
  starts anywhere in the track rows, and a middle-button drag with any tool that starts anywhere in
  the track rows (also on a clip). With press point `p0`, and `scroll0`, `sy0` (vertical track
  scroll) at the press, each frame sets `scroll = max(0, scroll0 - round((x - p0.x) * S /
  px_per_sec))` and the vertical track scroll to `clamp(sy0 - (y - p0.y), 0, max)`. Release does
  nothing more: no command, no undo step, selection and playhead unchanged. A middle-button press
  never edits. A `Select` click (no drag) on empty space still empties the selection (U-DRAG-SEL).
  `drag` keeps U-DRAG-VIEW for its other callers; the window does not pass these gestures to it.
- **U-NAV-6 Trim by default.** The window's `edge_px` is 8. With tool `Select` (the default), a
  primary drag that starts within 8 px inside a clip's start or end (U-GEO-3) trims it (U-DRAG-TRIM,
  `mode` normal); it never moves the clip. While the pointer hovers an edge zone where a drag would
  trim (`Select`, `Ripple`, `Roll` on either edge, `RateStretch` on `EndEdge`) the cursor is the
  horizontal-resize cursor. Example: clip `[1 s, 3 s)` on V1 at 100 px/s, scroll 0, 30 fps, snapping
  on and nothing else nearby: press 7 px left of its end (2.93 s), release 50 px right (3.43 s) →
  end at frame 103 (3.4333 s), start unchanged, one undo step.

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

- **S-4** `waveform(path)` → `Result<Arc<Vec<f32>>, String>`: peaks of the file's audio, 50 per second.
  Peak `i` is the largest `|sample|` of either channel in `[i / 50 s, (i + 1) / 50 s)`; a last
  partial bucket counts. Computed once per path from `MediaBackend::audio` (whole file, no filter)
  and cached; media without audio or a failing backend → `Err`. `Pattern` (10 s) gives 500 peaks of
  0.5.
- **S-5** `render_scaled(project, sequence_id, time, divisor: u32)` → `Result<Frame, String>`: the
  frame of `render` scaled (bilinear) to `ceil(width / divisor)` x `ceil(height / divisor)`;
  `divisor` 1 equals `render`; a divisor other than 1, 2, 4 → `Err`.
- **S-6** `prefetch(project, sequence_id, time)`: `MediaBackend::prefetch(path, t)` for every entry
  of `adit_render::media_frames(project, sequence_id, time)`, in order. Returns nothing, never panics,
  does not lock the session state.

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
  | G | GainDialog | ^C | Copy |
  | ^X | Cut | ^V | Paste |

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
  While playing forward, a background thread calls `Session::prefetch` for every sequence frame from
  the playhead up to `0.6 s * shuttle` of sequence time ahead (0.6 s of real time), so a cut to a new source
  position is decoded before the playhead gets there. It never blocks the window.
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
