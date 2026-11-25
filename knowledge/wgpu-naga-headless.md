# wgpu 30 / naga headless notes

Summary: API facts and traps for the offscreen renderer and WGSL validation.

Concerns: `crates/adit-render/src/{gpu,effects,text}.rs`.

- wgpu 30: `InstanceDescriptor::new_without_display_handle_from_env()`; `RequestAdapterOptions` has
  `apply_limit_buckets` (use `..Default::default()`); pipeline layout takes
  `bind_group_layouts: &[Option<&_>]` and `immediate_size`; `push_error_scope` returns a guard whose
  `pop()` is a future, scopes are per thread, guard is not `Send`. Without `on_uncaptured_error` any
  error outside a scope panics.
- No GPU: `force_fallback_adapter: true` gives WARP on Windows. Whole suite passes, effect tests ~10x
  slower.
- naga's WGSL parser recurses on the caller's stack. Deep nesting (brace depth ~64 in a debug build)
  overflows it, and `catch_unwind` cannot catch a stack overflow. Cap input size/depth and parse on a
  thread with a big stack (SPEC H-1).
- naga accepts `override` constants; pipeline creation then fails. Reject modules with overrides
  (SPEC H-2).
- Put user WGSL before the adit prelude: declaration order does not matter in WGSL, and naga's line
  numbers then match the author's. Keep `@vertex`/`@fragment` out of the text you validate or the
  "no entry points" check trips on adit's own.
- Bilinear sampling is done by hand with `textureLoad` on premultiplied texels, then un-premultiplied:
  1:1 pixel centres return stored bytes exactly, transparent neighbours do not darken edges.
- Text: `ab_glyph` `PxScale` is ascent-to-descent height, not em size; use
  `size * height_unscaled() / units_per_em()`. Fonts come from `%WINDIR%\Fonts` by file name.

Verified: 2026-11-25
