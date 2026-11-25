# Preview playback performance

Summary: where Program-monitor frame time goes on an integrated GPU, what fixed it, and how to measure.

Concerns: `crates/adit-render/src/{gpu,compose,effects,builtins,lib}.rs`, `crates/adit-media/src/decode.rs`.

Test project: a 1080x1920 sequence nesting a 1920x1080 sequence twice (blurred backdrop + sharp copy),
two text tracks with `drop_shadow`, one adjustment clip; 720p30 h264 sources, Radeon 890M (Vulkan).
Release build, 1/2 preview, ms per frame:

- Before: ~260 (3.8 fps). Decode only ~12 of it.
- Built-in `gaussian_blur` radius 30 (capped 24) as one 2-D pass = 49x49 taps at full sequence size:
  ~190 ms. Now two 1-D passes `BLUR_1D` (E-BLUR-1) when no project effect of that name exists.
- Fresh render-target texture per pass (~25 x 8 MB a frame): driver allocated and zeroed them every
  frame, ~30 ms. Symptom: submitting + waiting after every pass was *faster* than one submission.
  Now `Gpu::recycle` hands last frame's `pass`/`fill` targets out again. Never keep a `pass`/`fill`
  texture across frames; caches that live longer must hold `upload` textures (text cache does).
- `adit_sample` texel-centre fast path: 1 load instead of 4 for 1:1 layers and integer-offset taps.
- Same nested sequence on two tracks: composited once per frame (`Renderer::nested`).
- Text layers cached by (TextSpec, size) — CPU side was small; mostly saves the upload.
- ffmpeg frames read ahead (2) on a thread so decode overlaps rendering.
- After: steady 13-27 ms (GPU-only ~24). A clip cut to a far source position costs ~180-210 ms
  cold: new ffmpeg + seek + decode from the keyframe (OBS files: keyframe every 1 s).
- Cut stalls: `Frames` keeps up to 4 readers per file (`Slot`, atomic `next` so a reader can be
  picked without waiting on the thread reading it); `prefetch` only opens a reader when no open one
  reaches the frame, so it never steals the playing reader. The window's playback thread calls
  `Session::prefetch` (via `adit_render::media_frames`, which must mirror `render_seq`) for every
  frame 0.6 s ahead. Worst frame at cuts 204-286 ms -> ~50-60 ms (bench paced at 30 fps).

Measuring: absolute numbers drift ±30% with iGPU clocks/thermals; repeat 3x. A scratch harness that
loads a `.adit` with `Project::from_json`, wraps `Ffmpeg` in a timing `MediaBackend` and calls
`Renderer::render_frame_scaled` 60 times splits decode from render; same time repeated = GPU only.
Per-pass GPU time: temporarily submit the encoder and `device.poll(wait)` inside `Gpu::pass`.

Verified: 2026-11-25
