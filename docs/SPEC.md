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
  locked track holding material after `start`, `Sequence::duration()` drops by exactly
  `min(old_duration, end) - start` when `old_duration > start`.
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
