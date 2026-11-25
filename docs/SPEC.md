# adit behavioural spec

This is the contract. Tests are written from this file, never from the implementation. Each rule has an
id; tests cite the id they check. Where this file and the code disagree, the code is wrong.

Notation: `s` = `FLICKS_PER_SECOND` (705,600,000). A clip covers the half-open range
`[start, start + duration)`. "End" of a clip = `start + duration`. `round` = `f64::round` (half away from
zero). Types and signatures: `crates/adit-core/src/{model,command,document,error}.rs`.

# Part 1 — adit-core

## Invariants (`Project::validate`)

- **I1** On every track, clips are sorted by `start`, do not overlap, and have `start >= 0`,
  `duration > 0`.
- **I2** Every id (media, sequence, track, marker, clip, effect instance, transition, link, group) is
  `< next_id`; ids of entities are unique across the whole project. (Link and group ids are shared by
  their member clips but never equal an entity id.)
- **I3** A `Media` clip whose media has `duration: Some(D)` satisfies `source_in >= 0` and
  `source_in + round(duration * speed) <= D + SOURCE_SLACK`. Every clip has `source_in >= 0`,
  `speed` finite and `> 0`.
- **I4** Video tracks hold: `Media` with `video`, `Sequence`, `Text`, `Solid`, `Adjustment`.
  Audio tracks hold: `Media` with `audio`, `Sequence`.
- **I5** A transition has `duration > 0` and at least one of `left`/`right`. Each referenced clip is on
  that track. If both are set, `left.end == right.start`. Two-sided: `duration / 2 <= left.duration` and
  `duration - duration / 2 <= right.duration` (integer division). One-sided: `duration <=` that clip's
  duration. No clip edge (a clip's start, a clip's end) carries more than one transition.
- **I6** Every `media_id` and `sequence_id` referenced by a clip exists. No sequence contains itself,
  directly or through nested sequences. Sequence `width`, `height`, `sample_rate`, `fps.num`, `fps.den`
  are all `> 0`. Markers are sorted by time, `time >= 0`, `duration >= 0`. Keyframes in every `Param` are
  sorted by time with unique times. Every number is finite.

## General rules

- **G1 Atomic.** If `apply` returns `Err`, the project is `==` to what it was, and undo/redo depths are
  unchanged.
- **G2** If `apply` returns `Ok`, `validate()` passes. If the project changed, exactly one undo entry is
  pushed and the redo stack is cleared. If the project is `==` to before, nothing is pushed and redo is
  kept.
- **G3** `undo()` restores the project exactly (`==`) to the state before the last undoable command and
  makes it redoable; `redo()` restores the state after. With nothing to undo/redo they return `false` and
  change nothing. History is unlimited.
- **G4 Ids.** A new id is `next_id`, after which `next_id += 1`. `next_id` never decreases through
  `apply` (undo restores the old value).
- **G5 Locked tracks.** A command that would add, remove or change a clip or transition on a locked track
  fails with `TrackLocked`. Exception: ripple shifts (R1) silently skip locked tracks. `SetTrack`,
  `MoveTrack` work on locked tracks; `RemoveTrack` on a locked track fails with `TrackLocked`.
  When `track_ids: None` means "all unlocked tracks", locked tracks are skipped, not an error.
- **G6** Any non-finite `f64`, negative `start`/`time`, or `duration <= 0` (where a duration is given;
  markers allow 0) in a command → `InvalidArgument`. Empty `clip_ids` → `InvalidArgument`.
  Negative values are allowed for keyframe `time` and for `delta`. A negative `to` in `TrimClip` →
  `InvalidArgument`. A negative `source_in` → `SourceOutOfRange` (R7).
- **G10 Clarifications.** G5 covers every command that changes a clip's fields, params, keyframes,
  effects, link or group, and `CloseGap` on a locked target track. `SplitClip` with
  `include_linked: false` gives the right part `link: None`. `Clip::param` returns `None` for any name
  not in `CLIP_PARAMS`, stored or not. `AddTransition` naming a clip that does not exist → `NotFound`.
  `RemoveKeyframe` on a param that does not exist → `InvalidArgument`. `GroupClips` across sequences →
  `InvalidArgument`. In R7, "stays as is" means trims never change `source_in` of `Text`, `Solid`,
  `Adjustment` clips.
- **G7** An id that does not exist → `NotFound`.
- **G8 Transition pruning.** After every successful command, transitions that no longer satisfy I5 are
  removed silently.
- **G9** `Project::from_json(p.to_json()) == Ok(p)` for every valid project. `Command` also round-trips
  through serde JSON. Omitted optional command fields take the defaults in `command.rs`
  (`include_linked` defaults to `true`, `mode` to `place`/`normal`).

## Primitives used by the command rules

- **R1 Ripple shift** `shift(seq, T, d)`: every clip with `start >= T` on every *unlocked* track of the
  sequence gets `start += d`. If afterwards any track has overlapping clips or a clip with `start < 0`,
  the command fails with `RippleBlocked`. Markers do not move.
- **R2 Head trim by `h`** (h may be negative = extend): `start += h`, `duration -= h`, every keyframe
  time (clip params and effect params) `-= h`. If not `reverse`: `source_in += round(h * speed)`.
  If `reverse`: `source_in` unchanged.
- **R3 Tail trim by `t`** (t may be negative = extend): `duration -= t`. If `reverse`:
  `source_in += round(t * speed)`. Otherwise `source_in` unchanged.
- **R4 Split of clip `c` at `T`** (`c.start < T < c.end`): the left part keeps `c`'s id and is `c`
  tail-trimmed by `c.end - T`. The right part is a copy of `c` head-trimmed by `T - c.start`, with a new
  clip id and new ids for each of its effect instances. A transition whose `left` was `c` now has
  `left` = the right part. Consequence: every parameter evaluates to the same value at the same sequence
  time before and after a split, and `source_time(at)` is unchanged for every `at` (within 1 flick).
- **R5 Clear `[a, b)` on a track**: clips entirely inside are removed; a clip crossing `a` only is
  tail-trimmed to end at `a`; a clip crossing `b` only is head-trimmed to start at `b`; a clip containing
  the whole range strictly on both sides is split at `a` (R4) and the right part head-trimmed to `b`.
- **R6 Linked set** of a clip: the clip plus every clip in the same sequence with the same
  `link: Some(_)`.
- **R7 Source bounds**: after any change to `start`/`duration`/`source_in`/`speed` of a `Media` clip, I3
  must hold, else `SourceOutOfRange`. `Sequence` clips need only `source_in >= 0`. `Text`, `Solid`,
  `Adjustment` clips have no source bounds; their `source_in` stays as is.

## Queries

- **Q1** `Sequence::duration()` = max clip end over all tracks, 0 if no clips.
- **Q2** `Param::eval(t)`: no keyframes → `value`. `t <=` first keyframe time → first value. `t >=` last
  keyframe time → last value. Between keyframes `k0`, `k1`: `u = (t - k0.time) / (k1.time - k0.time)`,
  result `k0.value + (k1.value - k0.value) * f(u)` where `f` depends on `k0.easing`: `Linear` u; `Hold` 0;
  `EaseIn` u²; `EaseOut` 1 − (1 − u)²; `EaseInOut` 3u² − 2u³.
- **Q3** `Clip::source_time(at)` with `local = at - start`: not reversed →
  `source_in + round(local * speed)`; reversed → `source_in + round((duration - local) * speed)`.
- **Q4** `frame_to_time(n, fps)` = `floor(n * s * den / num)` computed without overflow (i128).
  `time_to_frame(t, fps)` = the largest `n` with `frame_to_time(n, fps) <= t`. Example: 30000/1001 fps,
  frame 1 → 23,543,520; frame 30 → 706,305,600. 24 fps frame 1 → 29,400,000.
- **Q5** `Clip::param(name, local)`: the stored `Param` evaluated at `local`, else the default from
  `CLIP_PARAMS`, else `None` for a name not in `CLIP_PARAMS`.
- **P1** Clip property names and defaults are exactly `CLIP_PARAMS`.

## Commands

`created` = `Outcome.created`. Unless stated, `created` is empty.

### Media, sequences, tracks, markers, effect definitions

- **C-MEDIA-1** `AddMedia`: appends; `created = [media_id]`. A `Some(duration) <= 0`, or neither `video`
  nor `audio` → `InvalidArgument`.
- **C-MEDIA-2** `RemoveMedia`: `InUse` if any clip references it.
- **C-MEDIA-3** `SetMedia`: sets the given fields (relink = new `path`).
- **C-SEQ-1** `AddSequence`: creates the sequence, then `video_tracks` video tracks named `V1..Vn`, then
  `audio_tracks` audio tracks `A1..An`, in that order in `tracks`. `created = [sequence_id, track ids in
  order]`. Zero width/height/sample_rate/fps parts → `InvalidArgument`.
- **C-SEQ-2** `RemoveSequence`: `InUse` if a clip in another sequence nests it.
- **C-SEQ-3** `SetSequence`: sets the given fields; zero values → `InvalidArgument`.
- **C-TRACK-1** `AddTrack`: inserted at `index` of `tracks` (`None` = append; `index > len` →
  `InvalidArgument`). Default name `V{k}` / `A{k}` with `k` = number of tracks of that kind after adding.
  `created = [track_id]`.
- **C-TRACK-2** `RemoveTrack`: removes the track with its clips and transitions. `MoveTrack`: moves it to
  `index` (`index >= len` → `InvalidArgument`). `SetTrack`: sets the given fields.
- **C-MARK-1** `AddMarker`: inserted keeping markers sorted by time (after existing markers with the same
  time). `created = [marker_id]`. `RemoveMarker` removes it.
- **C-FX-1** `DefineEffect`: empty name → `InvalidArgument`; a def with the same name is replaced in
  place, otherwise appended. `RemoveEffectDef`: unknown name → `InvalidArgument`. Core does not check
  that effect names used by clips or transitions exist (the layer above owns the registry).

### Clips

- **C-ADD-1** `AddClip` creates a clip with `speed 1`, `reverse false`, `enabled true`, `blend Normal`,
  empty `audio_filter`, `params`, `effects`, no group. `name` defaults to the media's name, the nested
  sequence's name, or `"Text"`, `"Solid"`, `"Adjustment"`. Kind must satisfy I4 else `KindMismatch`.
  A `Sequence` source that would nest the track's own sequence (directly or indirectly) → `NestCycle`.
  R7 applies.
- **C-ADD-2** With `linked_track_id`: the source must be `Media` with both video and audio, or
  `Sequence`; `track_id` must be a video track and `linked_track_id` an audio track of the same sequence;
  else `KindMismatch`. Two clips are created with the same `start`, `duration`, `source_in`, and
  `link: Some(L)`. Ids are allocated in the order: video clip, audio clip, `L`.
  `created = [video_clip, audio_clip]`. Without it: `created = [clip]`, `link: None`.
- **C-ADD-3** The new clip ids are allocated before any id needed for splitting other clips, so
  `created[0]` is the new clip's id and equals the `next_id` before the command.
- **C-ADD-4** `mode: place`: any overlap with an existing clip on a target track → `Overlap`.
- **C-ADD-5** `mode: overwrite`: clear (R5) `[start, start + duration)` on each target track, then place.
- **C-ADD-6** `mode: insert`: split (R4) at `start` every clip crossing `start` on every unlocked track
  of the sequence, then `shift(seq, start, duration)`, then place. Example: V1 has clip X `[0, 10s)`;
  insert 2s at 4s → X `[0, 4s)`, new clip `[4s, 6s)`, X's right part `[6s, 12s)` with
  `source_in = X.source_in + 4s`.
- **C-REM-1** `RemoveClips`: removes the listed clips, plus their linked sets (R6) when `include_linked`.
  Other clips do not move.
- **C-RIP-1** `RippleDeleteClips`: all clips must be in one sequence (else `InvalidArgument`). Remove
  them (with linked sets when `include_linked`). Merge the removed clips' ranges into maximal intervals
  (touching intervals merge). For each interval `[a, b)` from the latest to the earliest: if any clip on
  any unlocked track of the sequence intersects `[a, b)` → `RippleBlocked`; else `shift(seq, b, -(b-a))`.
- **C-RIP-2** Example: V1 = P `[0,2s)`, Q `[2s,5s)`, R `[5s,9s)`; A1 (unlocked) = M `[5s,9s)`; A2 locked =
  L `[5s,9s)`. Ripple-delete Q → P `[0,2s)`, R `[2s,6s)`, M `[2s,6s)`, L unchanged `[5s,9s)`. If A1 instead
  held M `[1s,9s)` the command fails with `RippleBlocked` and nothing changes.
- **C-LIFT-1** `LiftRange`: `end <= start` → `InvalidArgument`. Clear (R5) `[start, end)` on the target
  tracks: the listed ones (a listed locked track → `TrackLocked`; a track of another sequence →
  `InvalidArgument`) or, for `None`, all unlocked tracks.
- **C-EXT-1** `ExtractRange`: as `LiftRange`, then on each *target* track every clip with
  `start >= end` gets `start -= (end - start)`. Non-target tracks do not change. With `None` and no
  locked tracks: if `old_duration > end`, `Sequence::duration()` drops by exactly `end - start`; if
  `old_duration <= start` it is unchanged; otherwise the new duration is `<= start`.
- **C-GAP-1** `CloseGap`: the gap is `[a, b)` where `a` = end of the last clip on the track ending at or
  before `time` (0 if none) and `b` = start of the first clip starting after `time`. If `time` is inside a
  clip, or there is no clip after `time` → `InvalidArgument`. If any clip on any unlocked track of the
  sequence intersects `[a, b)` → `RippleBlocked`. Else `shift(seq, b, -(b - a))`.
- **C-SPLIT-1** `SplitAt`: on the target tracks (listed, or all unlocked), every clip with
  `start < time < end` is split (R4). `created` = the right parts' clip ids, in track order (index
  ascending). Right parts that came from clips sharing a link id get one fresh shared link id (so the
  right halves are linked to each other, not to the left halves). No clip crossed → `Ok`, nothing changes.
- **C-SPLIT-2** `SplitClip`: `time` not strictly inside the clip → `InvalidArgument`. Splits the clip
  and, when `include_linked`, every clip of its linked set that strictly contains `time`; link handling
  as C-SPLIT-1. `created[0]` = the right part of `clip_id`.
- **C-MOVE-1** `MoveClip`: `delta = start - clip.start`. The moving set is the clip, plus its linked set
  when `include_linked`. Every moving clip gets `start += delta`; the named clip additionally moves to
  `track_id` if given (must be in the same sequence, else `InvalidArgument`; kind per I4, else
  `KindMismatch`). A resulting negative start → `InvalidArgument`. The moving clips are taken out first,
  so they never collide with their own old position. `place`: overlap with a non-moving clip →
  `Overlap`. `overwrite`: clear (R5) each destination range first. `insert` → `InvalidArgument`.
- **C-TRIM-1** `TrimClip`, `d = to - (current time of that edge)`. Affected clips: the clip, plus (when
  `include_linked`) clips of its linked set whose same edge is at the same sequence time. A resulting
  `duration <= 0` → `InvalidArgument`. R7 applies to every changed clip.
- **C-TRIM-2** `normal`: `Start` → head trim by `d` (R2); `End` → tail trim by `-d` (R3). Overlap with a
  neighbour or `start < 0` → `Overlap`.
- **C-TRIM-3** `ripple`, `End`: tail trim by `-d`, and every other clip on unlocked tracks with
  `start >= old end` shifts by `d` (R1, judged on the final state).
  `ripple`, `Start`: the clip keeps its `start`; `duration -= d`; source and keyframes change as in a head
  trim by `d` (R2); every other clip on unlocked tracks with `start >= old end` shifts by `-d`.
  Example: X `[2s,6s)` source_in 1s, Y `[6s,8s)`; ripple-trim X's start to 3s → X `[2s,5s)` source_in 2s,
  Y `[5s,7s)`.
- **C-TRIM-4** `roll`: `End` needs a clip `N` on the same track with `N.start == clip.end`, `Start`
  needs `P` with `P.end == clip.start`; else `NoAdjacentClip`. `End`: clip tail trim `-d`, `N` head trim
  `d`. `Start`: clip head trim `d`, `P` tail trim `-d`. Nothing else moves; total span is unchanged.
- **C-SLIP-1** `SlipClip`: `source_in += delta` on the clip (and linked set). Only `Media` and `Sequence`
  clips, else `InvalidArgument`. R7. Position and duration do not change.
- **C-SLIDE-1** `SlideClip`: needs adjacent `P` and `N` (as C-TRIM-4) else `NoAdjacentClip`.
  `clip.start += delta`, `P` tail trim `-delta`, `N` head trim `delta`. R7 on `P` and `N`; a neighbour
  duration `<= 0` → `InvalidArgument`. The clip's `source_in` and duration do not change.
- **C-SPEED-1** `SetClipSpeed`: `speed` must be in `[0.01, 100]` else `InvalidArgument`. Only `Media` and
  `Sequence` clips, else `InvalidArgument`. `new_duration = max(1, round(duration * old_speed / speed))`;
  `source_in` unchanged. Applies to the linked set when `include_linked`. `ripple: true`: other clips on
  unlocked tracks with `start >= old end` shift by `new_duration - duration` (R1). `ripple: false`:
  overlap → `Overlap`. Example: 4s clip at speed 1 → speed 2 → 2s.
- **C-SET-1** `SetClip` sets the given fields. `SetText`: clip source must be `Text` else
  `KindMismatch`; `size <= 0` → `InvalidArgument`.
- **C-LINK-1** `LinkClips`: needs at least 2 clips, all in one sequence, else `InvalidArgument`; all get
  one freshly allocated link id. `UnlinkClips`: listed clips get `link: None`. `GroupClips` /
  `UngroupClips`: the same for `group`.

### Parameters, keyframes, effects, transitions

- **C-PARAM-1** Target with `effect_id: None`: `name` must be in `CLIP_PARAMS` else `InvalidArgument`.
  With `effect_id: Some`: the instance must exist on that clip (`NotFound`); `name` must be non-empty.
- **C-PARAM-2** `SetParam`: the param becomes `{ value, keyframes: [] }`.
- **C-PARAM-3** `SetKeyframe`: inserts the keyframe keeping time order; a keyframe at the same `time` is
  replaced. If the param did not exist it is created with `value` = the clip-property default (0 for
  effect params). `time` may be negative or beyond the clip.
- **C-PARAM-4** `RemoveKeyframe`: no keyframe at exactly `time` → `InvalidArgument`. `ClearKeyframes`:
  removes all keyframes, keeps `value`.
- **C-EFF-1** `AddEffect`: inserts at `index` (`None` = end; `index > len` → `InvalidArgument`) an
  enabled instance whose params are `{ value, keyframes: [] }` per entry. Empty effect name →
  `InvalidArgument`. `created = [effect_instance_id]`.
- **C-EFF-2** `RemoveEffect`, `MoveEffect` (`index >= len` → `InvalidArgument`), `SetEffectEnabled`:
  as named; unknown `effect_id` on that clip → `NotFound`.
- **C-TRANS-1** `AddTransition`: must satisfy I5, else `InvalidArgument`. Existing transitions on the
  same clip edge(s) are replaced. `created = [transition_id]`.
- **C-TRANS-2** `RemoveTransition` removes it. `SetTransition` sets the given fields; a result violating
  I5 → `InvalidArgument`.
- **C-TRANS-3** (with G8) Example: P `[0,2s)`, Q `[2s,4s)` with a two-sided 1s transition; moving Q to
  start at 3s removes the transition; undo brings it back.

### Batch

- **C-BATCH-1** Commands run in order against the evolving project. The first `Err` fails the whole batch
  with that error (G1). On success: one undo entry, `created` = concatenation in order. Nested batches
  are allowed. An empty batch is `Ok` and changes nothing.

# Part 5 — hardening (added after independent review)

An MCP client is untrusted input. None of the inputs below may crash, hang or abort the process; each
is an ordinary error.

- **H-3 Time bounds (core).** `MAX_TIME` = 3,600,000 s in flicks. Every `start`, `time`, `to`,
  `source_in`, `duration` and marker time in a command must be `<= MAX_TIME`, keyframe `time` and
  `delta` within `±MAX_TIME`, and every clip must end at or before `MAX_TIME`; otherwise
  `InvalidArgument`. `validate()` rejects a project with a clip where `start + duration > MAX_TIME`
  (computed without overflow), `source_in > MAX_TIME`, a marker beyond `MAX_TIME`, or a keyframe time
  outside `±MAX_TIME`. A ripple that would push a clip past `MAX_TIME` → `RippleBlocked`.
- **H-4 Nesting depth (core).** A sequence's nesting depth is 1 + the largest depth of the sequences
  its clips nest (0 nested → 1). `AddClip` that would make any sequence deeper than `MAX_NEST_DEPTH`
  (16) → `InvalidArgument`; `validate()` rejects deeper projects.
- **H-10 Pruning inside batches (core).** G8 applies after each command of a `Batch`, not once at the
  end. Example: P `[0,2s)`, Q `[2s,4s)` with a two-sided transition; a batch that moves Q to 3 s and
  back to 2 s ends without the transition.
- **H-15 (core).** `RemoveSequence` of a sequence that has a locked track → `TrackLocked`.
- **H-6 Audio filter allow-list (media).** `check_audio_filter(s)`: `""` is ok. Otherwise `s` must be
  filters separated by `,`, each `name` or `name=args`, with `name` in the allow-list and no `;`, `[`,
  `]` anywhere (no filtergraph labels, no second chain). Allow-list: `volume, atempo, areverse,
  highpass, lowpass, bandpass, bandreject, equalizer, bass, treble, acompressor, alimiter, agate,
  aecho, chorus, flanger, aphaser, tremolo, vibrato, afade, adelay, loudnorm, dynaudnorm, compand,
  anequalizer, afftdn, anlmdn, deesser, crystalizer, stereowiden, extrastereo, pan, aresample,
  asetrate, silenceremove, apad, atrim, asetpts, acontrast, asoftclip, speechnorm, channelmap,
  aformat`. Everything else is rejected, in particular `amovie`, `movie`, `ametadata`, `asendcmd`,
  `azmq`, `afir`, `arnndn`, `ladspa`, `lv2`. `Ffmpeg::audio` calls it and returns
  `MediaError::Ffmpeg` without starting ffmpeg when it fails; no file is created or read.
- **H-7 Start offsets (media).** For a file whose first video frame is not at timestamp 0 (e.g. made
  with `-output_ts_offset 10`), `video_frame(path, t)` still returns frame `time_to_frame(t, fps)`
  counted from the first frame, for cold, random and sequential access alike. `probe` reports
  `duration` measured from the file's start time (a 20 s clip offset by 10 s probes as about 20 s,
  not 30 s). `audio(path, start, ..)` likewise counts `start` from the beginning of the audio.
- **H-8 Variable frame rate (media).** Which frame is returned for a time does not depend on access
  order: reading a time cold and reading it after a sequential run from 0 give identical pixels.
- **H-14 (media).** `detect_silence` / `detect_scenes` are `MediaBackend` methods; `Ffmpeg` implements
  them, `Pattern` returns `Unsupported`. The MCP tools use the session's backend, not an env var.
- **H-1 Shader limits (render).** `validate_effect` rejects WGSL longer than 65,536 bytes and WGSL
  whose `(`/`[`/`{` nesting is deeper than 64, before parsing. 500 nested parentheses return `Err`;
  the process survives.
- **H-2 (render).** `validate_effect` rejects WGSL declaring `override` constants (E-4 must hold).
- **H-5 Allocation bounds (render, mcp).** `mix_audio` with a `duration` above `MAX_TIME` or negative
  → `Err`, not an allocation. `audio_levels` clamps `end` to the sequence end and rejects
  `start >= end`. `export` of a sequence longer than 4 hours → `Err` (`shortcut`: the soundtrack is
  mixed in memory).
- **H-11 (mcp).** `render_contact_sheet` whose sheet would exceed 64,000,000 pixels → error.
- **H-9 Effect references stay valid (mcp).** `edit` fails, changing nothing, if after the batch any
  clip effect or transition names an effect that no longer resolves with the right kind (e.g.
  `remove_effect_def` of an effect in use, or a `define_effect` that changes its kind).
- **H-12 Export does not destroy files (render, mcp).** Export writes to a temporary file next to the
  target and renames it over the target only on success. On failure or cancel a file that already
  existed at the target path is untouched. `export` to a path that is one of the project's media
  files → error.
- **H-13 (mcp).** `project_save` flushes the temp file to disk (`sync_all`) before the rename. At
  most 4 export jobs run at once; a fifth `export` → error.

- **H-16 Clarifications.** H-1 measures `EffectDef.wgsl` itself; a definition at depth exactly 64 must
  validate without overflowing the stack (validate on a thread with enough stack if needed). H-3
  covers `AddMedia` duration and marker duration; a non-ripple `SetClipSpeed` ending past `MAX_TIME`
  → `InvalidArgument`. H-6: filter names are lower-case and non-empty (`"volume=1,"` and `"=1"` are
  rejected); `|` is allowed inside args; the error text contains the rejected name. H-12 wins over
  X-2 when a file already existed at the target. H-13 "at once" = jobs in state `running`.

  The bracket limit does not bound recursion from operator chains (`!!!!…x`, `1+1+…`); shader
  parsing and pipeline creation therefore run on a thread with a 1 GiB reserved stack (measured need
  in a debug build for a worst-case 64 KB chain: 256–512 MiB).

Known limits, not fixed in this milestone: a user shader with a very long loop can stall the renderer;
blocking tools and HTTP requests each take a thread without an upper bound; one idle ffmpeg reader per
video file is kept alive.

# Part 2 — adit-media

API: `crates/adit-media/src/lib.rs`. ffmpeg and ffprobe run as subprocesses; nothing links against them.

- **M-PROBE-1** `Ffmpeg::probe` returns `name` = file name without directory, `path` = the path given,
  `duration` in flicks (container duration, within one frame of the true length), `video` (coded size
  after rotation is *not* applied; `fps` = the stream's average frame rate as an exact rational, e.g.
  30000/1001), `audio` (sample rate, channel count). A file with only audio has `video: None` and vice
  versa.
- **M-PROBE-2** A still image (png, jpg, ...) probes as `duration: None`, `video: Some` with
  `fps` 25/1, `audio: None`.
- **M-PROBE-3** A missing file → `MediaError::NotFound`. A file ffprobe cannot read (e.g. a text file)
  → `MediaError::Ffmpeg` or `Unsupported`. Neither panics. `video_frame` and `audio` on a missing
  file also return `NotFound`; the detect functions return some `Err`.
- **M-FRAME-1** `video_frame(path, t)` returns frame `n = time_to_frame(t, fps)` of the file, decoded to
  RGBA8 at the stream's size. It must be the exact frame: for a constant-frame-rate file, asking for the
  start time of frame `n` returns frame `n`, never `n-1` or `n+1`, for sequential access, backwards
  access and random access alike.
- **M-FRAME-2** `t < 0` returns frame 0; `t >=` duration returns the last frame. Stills return the image
  for every `t`.
- **M-FRAME-3** Sequential forward reads (frame n, n+1, n+2, ...) must not start a new ffmpeg process
  per frame: reading 60 consecutive frames of a 64x64 clip takes well under the time of 60 process
  launches (test bound: < 5 s on any machine). Frames are cached (LRU bounded by bytes, default 512 MB);
  asking for the same frame twice returns the same `Arc`.
- **M-FRAME-4** Alpha: sources without alpha decode with `a = 255`. A PNG with transparency keeps its
  alpha (straight, not premultiplied).
- **M-AUDIO-1** `audio(path, start, duration, sr, "")` returns interleaved stereo f32 resampled to `sr`,
  mono sources duplicated to both channels, length `2 * round(duration * sr / s)` samples within ±2
  frames, covering source range `[start, start + duration)`. A range past the end of the file yields
  fewer samples (possibly none), not an error.
- **M-AUDIO-2** `filter` is passed to ffmpeg `-af` after the range is cut. `"volume=0.5"` halves the
  samples; `"atempo=2.0"` halves the length (within 3%: ffmpeg pads). An invalid filter → `MediaError::Ffmpeg`.
- **M-AUDIO-3** A file without audio → `Ok(vec![])`.
- **M-PAT** `Pattern { width, height, fps, duration }`: `probe` returns `name` = `path`, the given
  `duration`, `video` with the given size/fps, `audio` 48000 Hz stereo. `video_frame(_, t)`: with
  `n = time_to_frame(max(t, 0), fps)`, pixel `(x, y)` = `(x % 256, y % 256, n % 256, 255)`.
  `audio(_, _, duration, sr, _)`: `round(duration * sr / s)` frames, every left sample `0.5`, every right
  sample `0.25`; `filter` is ignored.
- **M-ENC-1** `Encoder` writes a file that ffprobe reads back with the requested size, frame rate,
  exactly as many video frames as `write_frame` was called, and (if audio was given and the codec is not
  `None`) one audio stream at `sample_rate`. `H264` output is `yuv420p` so ordinary players open it; odd
  width or height → `MediaError::Unsupported`.
- **M-ENC-2** Decoding frame `k` of an encoded file returns the picture given to the `k`-th
  `write_frame` (lossy: mean absolute error per channel < 8 for flat colours at `crf: Some(10)`).
- **M-ENC-3** A `write_frame` with the wrong buffer length → `MediaError::Io` without writing. If ffmpeg
  dies, `write_frame` / `finish` return `Err` carrying the tail of ffmpeg's stderr; nothing panics or
  hangs. Dropping an `Encoder` without `finish` kills the ffmpeg process.
- **M-ENC-4** With `audio: Some` the soundtrack length is not allowed to cut the video short: the file
  has all written video frames even when the audio is shorter.
- **M-DET-1** `detect_silence` on a file that is 1 s tone, 1 s silence, 1 s tone (noise −30 dB,
  min 0.5 s) returns one range within 0.1 s of `[1s, 2s)`. `detect_scenes` on a file that switches from
  flat red to flat blue at 1 s returns one time within one frame of 1 s. No silence / no cut → empty.
- **M-PROC-1** Paths with spaces and non-ASCII characters work. No shell is involved: arguments are
  passed as an argv array. ffmpeg's stderr is always drained so a full pipe cannot deadlock.

# Part 3 — adit-render

API: `crates/adit-render/src/lib.rs`. All colour math is done on sRGB-encoded values in 0..1 as they
are (no linearisation), with straight alpha. GPU textures are `Rgba8Unorm`-style (no automatic sRGB
conversion). Pixel tests allow ±2 per 8-bit channel.

## Video

- **V-1 Canvas.** `render_frame` returns a frame of the sequence's `width` x `height`. The canvas starts
  opaque black `(0,0,0,1)`. A nested sequence renders onto a transparent canvas `(0,0,0,0)` of its own
  size.
- **V-2 Layers.** Video tracks are drawn in `tracks` order (index 0 first, higher on top). `hidden`
  tracks and clips with `enabled: false` are skipped. Audio tracks draw nothing. On each track the clip
  whose range contains `time` is drawn (plus transitions, V-9).
- **V-3 Source image** of a clip at `time`:
  `Media` → `media.video_frame(path, clip.source_time(time))` at the media's own size (no auto-fit);
  `Solid` → sequence-sized, filled with `color`;
  `Text` → sequence-sized transparent image with the text drawn in `color` at `size` px; the text
  block's bounding box is centred on the image; `align` positions lines within the block; `\n` breaks
  lines;
  `Sequence` → the nested sequence rendered at `clip.source_time(time)` at its own size (V-1);
  `Adjustment` → see V-8.
  A media clip whose file cannot be read renders as transparent rather than failing the frame:
  `render_frame` only returns `Err` for a missing sequence, a GPU failure, or an unknown/invalid effect.
- **V-4 Effects.** The clip's enabled effects run in order, each mapping the image to a new image of the
  same size (E-*). An effect name that is neither built-in nor in `project.effects` →
  `RenderError::UnknownEffect`.
- **V-5 Crop.** `crop_left/top/right/bottom` (fractions of the image, 0..1) make those margins
  transparent.
- **V-6 Transform.** The image (w x h) is placed on a transparent canvas-sized layer: with no transform
  its centre sits on the canvas centre at 1:1 pixel scale. `position_x/y` move it in canvas pixels
  (x right, y down). `scale_x/y` scale it and `rotation` rotates it clockwise in degrees, both about the
  anchor point, which is the image centre offset by `anchor_x/y` image pixels. Sampling is bilinear;
  outside the image is transparent. Parameters are evaluated at `local = time - clip.start` (Q5).
- **V-7 Opacity and blend.** Layer alpha is multiplied by `opacity`. The layer `s` (alpha `sa`) is
  composited over the canvas `d` (alpha `da`) per pixel:
  `B` = blend function; `s' = (1 - da) * s + da * B(d, s)`; `out.a = sa + da * (1 - sa)`;
  `out.rgb = (sa * s' + (1 - sa) * da * d) / out.a` (0 when `out.a = 0`).
  `B`: Normal `s`; Add `min(1, d + s)`; Multiply `d * s`; Screen `1 - (1-d)(1-s)`; Overlay
  `d < 0.5 ? 2ds : 1 - 2(1-d)(1-s)`; Darken `min(d, s)`; Lighten `max(d, s)`; Difference `|d - s|`.
  Examples on the black canvas: white solid at opacity 0.5 → `(128,128,128)`; red solid `(1,0,0,1)` under
  a `(0,0,1,1)` solid with blend Add → `(255,0,255)`; `(0.5,0.5,0.5,1)` under `(0.5,0.5,0.5,1)` Multiply
  → `(64,64,64)`.
- **V-8 Adjustment clip.** Its source image is the canvas as composited so far (tracks below). Its
  effects run on that; the result replaces the canvas mixed by `opacity`
  (`canvas = mix(canvas, result, opacity)`). Transform, crop and blend are ignored.
- **V-9 Transitions.** Window of a transition on a track: two-sided → `[cut - duration/2, cut -
  duration/2 + duration)` with `cut = left.end`; `left` only → `[left.end - duration, left.end)`;
  `right` only → `[right.start, right.start + duration)`. When `time` is in a window,
  `progress = (time - window_start) / duration`, and the track contributes one layer: the transition
  shader applied to `A` = the left clip's layer and `B` = the right clip's layer (each after effects,
  crop, transform, opacity; canvas-sized; a missing side is fully transparent), composited with blend
  Normal. A clip is sampled outside its own range when needed (`source_time` continues past the edit;
  media time clamps to the file).
  Built-in transitions, with `p = progress`, on premultiplied values then un-premultiplied:
  `cross_dissolve` → `A (1-p) + B p`;
  `dip_to_black` → `p < 0.5`: `mix(A, black, 2p)`, else `mix(black, B, 2p - 1)` (black = opaque);
  `wipe` → `B` where `uv.x < p`, else `A`;
  `push` → `uv.x < 1 - p`: `A` at `(uv.x + p, uv.y)`, else `B` at `(uv.x - (1 - p), uv.y)`.
  Example: red solid `[0,2s)`, blue solid `[2s,4s)`, 1 s `cross_dissolve`: at 2 s → `(128,0,128)`;
  at 1.75 s → `(191,0,64)`; at 1.4 s → pure red.
- **V-10 Determinism.** Rendering the same project and time twice gives identical frames. Rendering does
  not modify the project.

## Effects

- **E-1 Shader contract.** An `EffectDef` of kind `Filter` supplies WGSL containing
  `fn effect(uv: vec2f) -> vec4f`; kind `Transition` supplies `fn transition(uv: vec2f) -> vec4f`.
  `uv` is 0..1 with the origin at the top-left. Values are straight-alpha RGBA. adit prepends:
  - `fn src(uv: vec2f) -> vec4f` — the input image (filters), bilinear, transparent outside 0..1;
  - `fn src_a(uv: vec2f) -> vec4f`, `fn src_b(uv: vec2f) -> vec4f` — the two layers (transitions);
  - `ctx.size: vec2f` (image size in pixels), `ctx.time: f32` (seconds since clip start),
    `ctx.duration: f32` (clip or transition length in seconds), `ctx.progress: f32`
    (`time / duration`, 0..1), `ctx.frame: f32` (sequence frame number);
  - `p.<name>: f32` for every declared parameter, in declaration order.
  The user code may define helper functions and constants. It must not declare bindings or entry points.
- **E-2 Parameters.** Instance params are evaluated at `local` time (keyframes animate them); a
  parameter missing on the instance takes the `ParamDef.default`. Parameter names must be WGSL
  identifiers, unique, at most 64 per effect.
- **E-3 Lookup.** `project.effects` is searched first, then built-ins (a project definition can shadow a
  built-in). Kind must match the use (filter on a clip, transition on a cut) else `UnknownEffect`.
- **E-4 Validation.** `validate_effect` builds the full shader exactly as the renderer does and checks
  it with naga (parse + validate). Invalid WGSL, a missing/wrongly-typed `effect`/`transition`
  function, bad parameter names → `Err(message)` naming the problem. It never panics, whatever the
  input. A definition that passes `validate_effect` renders without `RenderError::Shader`.
- **E-5 Runtime effect example.** A filter `fn effect(uv: vec2f) -> vec4f { let c = src(uv);
  return vec4f(c.b, c.g, c.r, c.a) * vec4f(p.gain, p.gain, p.gain, 1.0); }` with param `gain` default 1
  turns a `(1, 0.5, 0, 1)` solid into `(0, 128, 255)`, and with `gain = 0.5` into `(0, 64, 128)`.
- **E-BUILTIN** `builtin_effects()` returns at least these, written against E-1 like any user effect.
  Filters with exact formulas (`c` = rgb, alpha unchanged unless stated; results clamp to 0..1):
  - `exposure {stops: 0}`: `c * 2^stops`
  - `brightness_contrast {brightness: 0, contrast: 1}`: `(c - 0.5) * contrast + 0.5 + brightness`
  - `saturation {amount: 1}`: `mix(vec3(l), c, amount)`, `l = dot(c, (0.2126, 0.7152, 0.0722))`
  - `temperature {temp: 0, tint: 0}`: `r += 0.1 temp`, `b -= 0.1 temp`, `g += 0.1 tint`
  - `levels {in_black: 0, in_white: 1, gamma: 1, out_black: 0, out_white: 1}`:
    `pow(clamp((c - in_black) / (in_white - in_black), 0, 1), 1 / gamma) * (out_white - out_black) + out_black`
  - `lift_gamma_gain {lift_r,g,b: 0, gamma_r,g,b: 1, gain_r,g,b: 1}`:
    `pow(clamp(c * gain + lift, 0, 1), 1 / gamma)` per channel
  - `invert {amount: 1}`: `mix(c, 1 - c, amount)`
  - `tint {r: 1, g: 1, b: 1, amount: 1}`: `mix(c, c * (r, g, b), amount)`
  - `hue_rotate {degrees: 0}`: rotation of `c` about the grey axis (0 = identity; 120 maps pure red to
    pure green within tolerance 6)
  - `flip {horizontal: 0, vertical: 0}`: mirrors when the value is `> 0.5`
  - `rect {x: 0.25, y: 0.25, w: 0.5, h: 0.5, r: 1, g: 1, b: 1, a: 1}`: draws that colour over the input
    where `x <= uv.x < x + w` and `y <= uv.y < y + h` (normal alpha-over)
  - `ellipse {cx: 0.5, cy: 0.5, rx: 0.25, ry: 0.25, r, g, b, a}`: same for the ellipse
  - `linear_gradient {r0,g0,b0,a0, r1,g1,b1,a1, angle: 0}`: replaces the image; angle 0 = left to right
  Filters checked only for plausibility (changes the picture, no error, identity at the neutral value):
  `gaussian_blur {radius: 4}` (pixels; 0 = identity), `sharpen {amount: 0}`, `vignette {amount: 0.5,
  radius: 0.75}`, `pixelate {size: 8}` (1 = identity), `chroma_key {r: 0, g: 1, b: 0, threshold: 0.3,
  softness: 0.1}` (pure key colour → alpha 0), `drop_shadow {dx: 4, dy: 4, blur: 4, opacity: 0.5}`,
  `stroke {width: 2, r: 0, g: 0, b: 0, a: 1}`, `noise {amount: 0.1, seed: 0}`.
  Transitions: `cross_dissolve`, `dip_to_black`, `wipe`, `push` (V-9).
  Every built-in passes `validate_effect`. Names are unique.

- **E-6 Clarifications.** The anchor (V-6) is only the pivot for scale and rotation: changing it alone
  does not move the image. Inside a transition window the clips' own blend modes are ignored (V-9;
  `shortcut`). Text is centred by its layout box. `ParamDef.min/max` are hints for UIs and are not
  enforced. An invalid effect at render time → `RenderError::Shader` or `UnknownEffect`. A project
  definition of the wrong kind does not hide a built-in of the right kind. `drop_shadow` positive
  `dy` is down. `ellipse` and `linear_gradient` colour defaults: white opaque (`r1..a1`), black opaque
  (`r0..a0`). `atempo` factors are written with enough digits to multiply back to the speed, each step
  within 0.5..100. `Renderer` is `Send`.

## Audio

- **A-1** `mix_audio` returns exactly `2 * round(duration * sample_rate / s)` samples (interleaved L, R).
  Empty space is silence. Only audio tracks contribute. Muted tracks are skipped; if any audio track has
  `solo`, only solo tracks play. Disabled clips are skipped.
- **A-2** A `Media` clip plays the source range starting at `source_in`, of length
  `round(duration * speed)`, fetched with `MediaBackend::audio` at the sequence sample rate with the
  filter chain: `areverse` if `reverse`; then `atempo=<speed>` if `speed != 1` (chained in steps within
  ffmpeg's accepted range when needed); then the clip's `audio_filter`; joined with commas. The result
  is cut or zero-padded to the clip's length.
- **A-3** Gain: `volume` is in dB, gain `10^(volume / 20)`; `pan` in −1..1 with left gain
  `min(1, 1 - pan)` and right gain `min(1, 1 + pan)`. Both are evaluated per sample frame at its local
  time, so keyframes fade smoothly. With `Pattern` audio (L 0.5, R 0.25): volume −6.0206 dB → L 0.25,
  R 0.125; pan 1 → L 0, R 0.25.
- **A-4** Tracks sum. Two overlapping `Pattern` clips on two tracks → L 1.0, R 0.5. No limiter.
- **A-5** Transitions on audio tracks: any effect name means an equal-power crossfade over the V-9
  window: left clip gain `cos(p * π/2)`, right clip gain `sin(p * π/2)`; a missing side is silence
  (fade in / fade out). Clips are sampled outside their own range when the window needs it.
- **A-6** A `Sequence` clip on an audio track plays the nested sequence's mix from
  `source_in` (speed and reverse ignored for nested audio; `shortcut`).
- **A-7** A clip whose audio cannot be fetched contributes silence; `mix_audio` fails only for a missing
  sequence.

## Export

- **X-1** `export` writes `N` frames where `N` = number of frames `n >= 0` with
  `frame_to_time(n, settings.fps) < sequence.duration()`; frame `n` is `render_frame` at
  `frame_to_time(n, settings.fps)`, scaled to `settings.width` x `height` if those differ from the
  sequence (bilinear). The soundtrack is `mix_audio(0, sequence.duration())` resampled if
  `settings.sample_rate` differs. `AudioCodec::None` or a sequence without audio tracks → no audio
  stream.
- **X-2** `progress` is called after every frame with `(done, N)`, ending with `(N, N)`. `cancel` set →
  stops within one frame, returns `Cancelled`, and removes the partial file.
- **X-3** An empty sequence (duration 0) → `Err` (nothing to export), no file left behind.
- **X-4** End to end: a 1 s, 30 fps, 64x64 sequence with a red solid `[0, 0.5s)` and a blue solid
  `[0.5s, 1s)` exported as H264 probes as 30 frames, 64x64, 30 fps; decoded frame 5 is red and frame 20
  is blue (±12 per channel).
