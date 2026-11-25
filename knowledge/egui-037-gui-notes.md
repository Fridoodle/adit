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
