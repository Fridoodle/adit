# egui 0.37 / kittest / cpal notes for the GUI

Summary: API changes and test-harness behaviour that differ from older egui code and examples.

Concerns: `crates/adit-gui/src/app.rs`, `crates/adit-gui/src/app/*.rs`, `crates/adit-gui/tests/app.rs`.

- `eframe::App` has `fn ui(&mut self, ui: &mut egui::Ui, frame)` (no `update(ctx, ..)`), plus an
  optional `logic`.
- Panels: `egui::Panel::{top,bottom,left,right}(salt).show(ui, ..)`; `show_inside` is deprecated;
  `CentralPanel::show(ui, ..)` works nested. `Modal::new(id).show(&ctx, ..)` with `should_close()`.
  `MenuBar::new().ui(ui, ..)`. `Id::new` is deprecated, use `Id::unique`. `ctx.text_edit_focused()`.
- Screenshots: `ctx.request_screenshot(|Arc<ColorImage>| ..)`; there is no `Event::Screenshot` to
  forward any more. egui_kittest's `wgpu` feature fulfils it after the pass at 1 px per point
  (software adapter is fine).
- With `default-features = false`, eframe needs feature `accesskit` (labels for kittest and screen
  readers) and `clipboard` (`ctx.copy_text`).
- kittest finds widgets by accessible label. Painter-only text is invisible to it, so clip names, tab
  names and status messages are real `Label`/`Button` widgets. `get_by_label` on a `Label` +
  `TextEdit.labelled_by` pair resolves to the text field. A `Label` reports its whole galley text.
- cpal 0.18: `StreamConfig.sample_rate` is a plain `u32`;
  `build_output_stream(config, data_cb, err_cb, None)`; the stream is not `Send`, keep it on its own
  thread.
- `adit gui` starts the MCP HTTP server before the window exists: for about the first second `ui_*`
  tools answer "no window is attached".
- `ui_screenshot` waits for an in-flight Program-monitor render (up to 3 s) so the monitor is not
  black in the shot.

Verified: 2026-11-25

- Boot freeze (2026-11-25): release build of `adit gui` went Not Responding at start on the Vulkan backend (wgpu's default pick here); DX12 and GL were fine, debug builds were fine. The window's device and the renderer's device were being created at the same time on two threads. Fix: `warm_renderer` in `app.rs` creates the session renderer before `eframe::run_native`. The exact lock inside the driver was not identified (no debugger on this machine); evidence is backend-specific + fixed by ordering. Quick check: `WGPU_BACKEND=vulkan|dx12 adit gui` and `(Get-Process adit).Responding`. Always test the release binary, not only debug.

Editing-comfort window work (U-ED-10..21), verified 2026-11-25:
- `egui::WidgetType` is deprecated: use `egui::Role`. A painter-drawn button (tool bar icons) =
  `allocate_exact_size(.., Sense::click())` + `resp.widget_info(|| WidgetInfo::selected(Role::Button,
  true, on, name))`: kittest finds it by `name` and sees `toggled`. `Button::selectable` and
  `checkbox` also report `toggled`. A `Role::Label` widget's text goes to accesskit *value*, not label.
- kittest `click()` lands at the node's rect even when a `ScrollArea` has it out of view, so it hits
  nothing. Rows a test must click have to be on screen (library effects are listed first for this).
- `Response::context_menu` + `ui.menu_button(..)` inside it gives submenus that open on click in
  kittest (closing after a choice is egui default, not checked by a test).
- `perform(Cut)` empties the selection before its `Op::Copy` runs, so `App::run_action` takes
  `copy(..)` before calling `perform`.
- `worker::Worker` keeps only the newest request: when one worker serves many keys (waveforms),
  every request names all keys still missing.
- Dropped files: `ctx.input(|i| i.raw.dropped_files)` (`DroppedFile::path()`), one frame only.
- Wand on panels: decide on raw pointer press/release against panel rects from `Panel::show(..)
  .response.rect`; skip frames that started with a dialog open, or the click on the dialog's `Send`
  reopens the prompt.

Verified: 2026-11-25
- Blocking a panel's widgets (U-ED-23d, `panels.rs` `wand_cover`): `ui.interact(ui.max_rect(), Id::unique(..), Sense::click_and_drag())` added *after* the panel's widgets wins egui's hit test (last = topmost on a tie), no dimming. Keep it while a dialog is open: in kittest the second click of a double click lands after the first click's dialog opened, and egui's double-click timing is per input, not per widget. Verified: 2026-11-25
- Wheel + modifiers (U-NAV-4, `app/timeline.rs`): egui's `InputOptions::zoom_modifier` is Ctrl/Cmd, so Ctrl + wheel arrives as `zoom_delta()` (`exp(delta / 200)`) and `smooth_scroll_delta()` is zero; checking `smooth_scroll_delta` with Ctrl held never fires. Alt is `vertical_scroll_modifier` (delta forced to y), Shift forces x. A `Sense::drag` response's `drag_started_by(PointerButton::Middle)` tells middle drags apart; plain `dragged()` is true for any button. Verified: 2026-11-25
