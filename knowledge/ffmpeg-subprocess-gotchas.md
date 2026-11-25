# ffmpeg subprocess gotchas

Summary: things that silently give wrong frames or samples when driving ffmpeg/ffprobe as a subprocess.

Concerns: `crates/adit-media/src/{proc,probe,decode,encode}.rs`, fixtures in `crates/adit-media/tests/common/mod.rs`.

- `-ac 2` upmixes mono at −3 dB (x0.707). For a true copy to both channels use
  `pan=stereo|c0=c0|c1=c0`.
- ffmpeg auto-rotates on decode, so raw frames can differ in size from ffprobe's coded size. Pass
  `-noautorotate` and force `scale=W:H`, or raw-frame reads lose alignment.
- Exact frame N: plain `-ss t_N` can be one frame off (mkv rounds pts to 1 ms; at 30000/1001 fps frame 1
  is stored at 0.033 s, before its true start). Working recipe: `-copyts -noaccurate_seek -ss <t - 1s>`
  plus `select='gte(t, start+(N-0.5)/fps)'` and `-fps_mode passthrough`.
- Matroska has no per-stream duration; container duration (often the audio) overstates the video
  length. The decoder must cope with end-of-stream before the estimated last frame.
- Prefix input/output paths with `file:` so names starting with `-` or containing `:` are not parsed
  as options or protocols. Non-ASCII paths work on Windows (UTF-16 argv).
- `atempo=2.0` output is ~1.6% longer than half (padding). Do not assert exact lengths.
- `geq` needs `format=rgba` before it or alpha comes out 255.
- Always drain stderr on a thread; a full pipe deadlocks ffmpeg. Kill children on drop.
- clippy 0.1.100 flags `chunks_exact(4)` with a constant size; use `as_chunks::<4>()`.

Verified: 2026-11-25
