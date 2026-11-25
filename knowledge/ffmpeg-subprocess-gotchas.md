# ffmpeg subprocess gotchas

Summary: things that silently give wrong frames or samples when driving ffmpeg/ffprobe as a subprocess.

Concerns: `crates/adit-media/src/{proc,probe,decode,encode}.rs`, fixtures in `crates/adit-media/tests/common/mod.rs`.

- `-ac 2` upmixes mono at −3 dB (x0.707). For a true copy to both channels use
  `pan=stereo|c0=c0|c1=c0`.
- ffmpeg auto-rotates on decode, so raw frames can differ in size from ffprobe's coded size. Pass
  `-noautorotate` and force `scale=W:H`, or raw-frame reads lose alignment.
- Exact frame N: plain `-ss t_N` can be one frame off (mkv rounds pts to 1 ms; at 30000/1001 fps frame 1
  is stored at 0.033 s, before its true start). Working recipe:
  `-copyts -seek_timestamp 1 -noaccurate_seek -ss <slot - 1s>` plus
  `-vf fps=fps=N/D:start_time=<absolute slot time>` and `-fps_mode passthrough`. Works for CFR and VFR
  (holds the last frame) in any access order.
- Input `-ss` counts from the file's `start_time` unless `-seek_timestamp 1` is given. With `-copyts`
  and absolute pts, forgetting it double-counts the start offset (frame 100 came back as 190).
- A test fixture with a start offset only catches seek bugs if it has keyframes near the target
  (`-g 10` or ffv1). A one-GOP x264 file always falls back to the start and hides the bug.
- Matroska `format.duration` is the end timestamp (includes `start_time`); mp4 and ts give end minus
  start. Matroska also has no per-stream duration; container duration (often the audio) overstates
  the video length, so the decoder must cope with end-of-stream before the estimated last frame.
- Open: `audio()` counts `start` from the file start time, not from the first audio sample. Differs
  only when audio starts later than video.
- Prefix input/output paths with `file:` so names starting with `-` or containing `:` are not parsed
  as options or protocols. Non-ASCII paths work on Windows (UTF-16 argv).
- `atempo=2.0` output is ~1.6% longer than half (padding). Do not assert exact lengths.
- `geq` needs `format=rgba` before it or alpha comes out 255.
- Always drain stderr on a thread; a full pipe deadlocks ffmpeg. Kill children on drop.
- clippy 0.1.100 flags `chunks_exact(4)` with a constant size; use `as_chunks::<4>()`.

Verified: 2026-11-25
