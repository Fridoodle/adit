# Graph Report - adit  (2026-11-25)

## Corpus Check
- 85 files · ~113,784 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 3 file(s) not represented in the graph (top: (none) 2, .proptest-regressions 1)

## Summary
- 1951 nodes · 6012 edges · 87 communities (80 shown, 7 thin omitted)
- Extraction: 71% EXTRACTED · 29% INFERRED · 0% AMBIGUOUS · INFERRED: 1715 edges (avg confidence: 0.79)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `fb152664`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Command
- adit-core/tests/common/mod.rs
- Time
- Project
- Part 1 — adit-core
- time.rs
- base
- proto.rs
- src/tools.rs
- adit-render/tests/common/mod.rs
- adit-media/tests/hardening.rs
- trims.rs
- validate.rs
- adit-core
- frames.rs
- history.rs
- encode.rs
- timed
- decode.rs
- seq_src
- Tools
- assert_px
- effects_runtime.rs
- src/encode.rs
- session
- clips.rs
- effects_builtin.rs
- new_project
- Frame
- adit-media/tests/common/mod.rs
- pattern.rs
- Id
- text.rs
- adit-media/tests/audio.rs
- transitions.rs
- adit-mcp/tests/common/mod.rs
- Error
- adit-render/tests/hardening.rs
- model.rs
- render_cli.rs
- Mcp
- RenderError
- Document
- adit-render/tests/export.rs
- duration
- R
- .render_seq
- stdio.rs
- Http
- INDEX.md
- document.rs
- EffectDef
- effects.rs
- MediaBackend
- adit-core/tests/hardening.rs
- adit-core/tests/common/mod.rs
- RenderError
- params.rs
- session
- adit-mcp/tests/common/mod.rs
- adit-mcp/tests/hardening.rs
- MediaError
- common
- adit/tests/common/mod.rs
- add_solid
- detect.rs
- requests.rs
- text_seq
- src/probe.rs
- CLAUDE.md
- edit
- renderer
- Media
- params.rs
- main.rs
- src/export.rs
- proc.rs
- props.rs
- adit-render/tests/hardening_crash.rs
- contact_sheet
- Easing
- adit-render/src/lib.rs
- def

## God Nodes (most connected - your core abstractions)
1. `base()` - 169 edges
2. `Id` - 129 edges
3. `Time` - 94 edges
4. `Command` - 93 edges
5. `session()` - 89 edges
6. `assert_px()` - 70 edges
7. `Project` - 68 edges
8. `render()` - 68 edges
9. `MediaError` - 66 edges
10. `timed()` - 57 edges

## Surprising Connections (you probably didn't know these)
- `S` --references--> `Time`  [INFERRED]
  crates/adit-core/tests/common/mod.rs → crates/adit-core/src/model.rs
- `MT` --references--> `Time`  [INFERRED]
  crates/adit-core/tests/hardening.rs → crates/adit-core/src/model.rs
- `H` --references--> `Time`  [INFERRED]
  crates/adit-core/tests/props.rs → crates/adit-core/src/model.rs
- `ELEVEN` --references--> `Rational`  [INFERRED]
  crates/adit-core/tests/queries.rs → crates/adit-core/src/model.rs
- `NTSC` --references--> `Rational`  [INFERRED]
  crates/adit-core/tests/queries.rs → crates/adit-core/src/model.rs

## Import Cycles
- 1-file cycle: `crates/adit-render/src/effects.rs -> crates/adit-render/src/effects.rs`
- 1-file cycle: `crates/adit-media/src/proc.rs -> crates/adit-media/src/proc.rs`
- 2-file cycle: `crates/adit-mcp/src/lib.rs -> crates/adit-mcp/src/proto.rs -> crates/adit-mcp/src/lib.rs`
- 2-file cycle: `crates/adit-media/src/lib.rs -> crates/adit-media/src/proc.rs -> crates/adit-media/src/lib.rs`

## Communities (87 total, 7 thin omitted)

### Community 0 - "Command"
Cohesion: 0.04
Nodes (52): Command, AddClip, AddEffect, AddMarker, AddMedia, AddSequence, AddTrack, AddTransition (+44 more)

### Community 1 - "adit-core/tests/common/mod.rs"
Cohesion: 0.15
Nodes (14): frame_to_time(), time_to_frame(), ELEVEN, lookups_sequence_track_clip(), NTSC, P1_EXPECTED, q2_no_keyframes_returns_value(), q3_clip() (+6 more)

### Community 2 - "Time"
Cohesion: 0.15
Nodes (15): Clip, Param, Sequence, FLICKS_PER_SECOND, Time, AUDIO_FILTERS, lock(), number_after() (+7 more)

### Community 3 - "Project"
Cohesion: 0.21
Nodes (38): alloc(), bad(), bounds_ok(), check_locked(), clear(), clip_ix(), clip_mut(), clip_ref() (+30 more)

### Community 4 - "Part 1 — adit-core"
Cohesion: 0.11
Nodes (18): adit behavioural spec, Audio, Batch, Clips, Commands, Effects, Export, General rules (+10 more)

### Community 5 - "time.rs"
Cohesion: 0.16
Nodes (18): Rational, VideoInfo, FPS24, convert_times(), fps_strings(), frames_to_time(), gcd(), is_decimal() (+10 more)

### Community 6 - "base"
Cohesion: 0.09
Nodes (48): c_add_6_r1_insert_splits_and_shifts_all_unlocked_tracks(), c_ext_1_all_unlocked_tracks(), c_ext_1_duration_when_range_passes_the_end(), c_ext_1_listed_tracks_only(), c_ext_1_locked_track_does_not_move(), extract(), base(), sq() (+40 more)

### Community 7 - "proto.rs"
Cohesion: 0.13
Nodes (14): call_tool(), constant_time_eq(), err(), handle(), header(), http_request(), INSTRUCTIONS, is_blocking() (+6 more)

### Community 8 - "src/tools.rs"
Cohesion: 0.08
Nodes (27): ToolInfo, call(), edit_description(), Entry, find_job(), Job, job_json(), JobState (+19 more)

### Community 9 - "adit-render/tests/common/mod.rs"
Cohesion: 0.06
Nodes (55): a1_count_uses_sequence_sample_rate(), a1_empty_space_is_silence(), a1_exact_sample_counts(), a1_mix_window_offset(), a1_mute_solo_disabled(), a1_only_audio_tracks_contribute(), a2_atempo_chained_within_ffmpeg_range(), a2_filter_chain_order_and_atempo() (+47 more)

### Community 10 - "adit-media/tests/hardening.rs"
Cohesion: 0.07
Nodes (45): check_audio_filter(), TempDir, ALLOWED, ametadata_to(), ff(), FORBIDDEN, frame_to_time(), gray_offset() (+37 more)

### Community 11 - "trims.rs"
Cohesion: 0.08
Nodes (61): Edge, End, Start, TrimMode, Normal, Ripple, Roll, Base (+53 more)

### Community 12 - "validate.rs"
Cohesion: 0.13
Nodes (35): i1_negative_start_rejected(), i1_overlap_rejected(), i1_touching_clips_are_fine_half_open(), i1_unsorted_rejected(), i1_zero_and_negative_duration_rejected(), i2_duplicate_clip_ids_rejected(), i2_id_not_below_next_id_rejected(), i2_ids_unique_across_entity_kinds() (+27 more)

### Community 13 - "adit-core"
Cohesion: 1.00
Nodes (5): adit, adit-core, adit-mcp, adit-media, adit-render

### Community 14 - "frames.rs"
Cohesion: 0.10
Nodes (42): assert_id_frame(), gray_frame_number(), t2997(), t30(), assert_still(), backwards(), ff(), Fixture (+34 more)

### Community 15 - "history.rs"
Cohesion: 0.19
Nodes (17): add_marker(), c_batch_1_empty_batch_is_ok_noop(), c_batch_1_first_error_fails_all(), c_batch_1_runs_in_order_one_undo_entry(), doc_with_clip(), g1_error_keeps_project_and_both_stacks(), g1_failing_ripple_after_partial_shift(), g2_change_pushes_one_entry_and_clears_redo() (+9 more)

### Community 16 - "encode.rs"
Cohesion: 0.22
Nodes (26): ffmpeg_decode_frame(), flat_rgba(), probe_audio_streams(), stereo_sine(), assert_video(), check_roundtrip(), COLOURS, encode() (+18 more)

### Community 17 - "timed"
Cohesion: 0.32
Nodes (15): timed(), assert_within(), ff(), ffmpeg_new_finds_binaries_on_path(), m_probe_1_audio_only_stereo(), m_probe_1_fps_30000_1001_exact_rational_video_only(), m_probe_1_fps_30_video_only(), m_probe_1_m_proc_1_path_with_space_and_non_ascii() (+7 more)

### Community 18 - "decode.rs"
Cohesion: 0.15
Nodes (13): CACHE_BYTES, decode_still(), f(), FileDec, frame_len(), Frames, Key, Lru (+5 more)

### Community 19 - "seq_src"
Cohesion: 0.19
Nodes (13): c_add_1_default_names(), c_add_1_i4_kind_mismatch(), c_add_1_nest_cycle_direct_and_indirect(), c_add_1_r7_stills_and_sequences_unbounded(), c_add_1_sequence_on_audio_track_ok(), c_add_2_linked_sequence_source(), c_add_2_unlinked_has_no_link(), c_add_4_place_overlap_and_touching() (+5 more)

### Community 20 - "Tools"
Cohesion: 0.15
Nodes (12): adit MCP contract (SPEC Part 4), Binary, Editing, Effects, Export, General rules, Library surface (`adit_mcp`), Magic-wand requests (+4 more)

### Community 21 - "assert_px"
Cohesion: 0.14
Nodes (39): assert_px(), render(), time_frame(), green(), small(), time_media(), v10_rendering_is_deterministic_and_pure(), v1_frame_is_sequence_sized_and_canvas_opaque_black() (+31 more)

### Community 22 - "effects_runtime.rs"
Cohesion: 0.10
Nodes (30): try_render(), e1_ctx_size_is_image_size(), e1_ctx_time_duration_progress_frame(), e1_helpers_and_constants_allowed(), e1_output_is_straight_alpha(), e1_params_declared_in_order_and_overridden_by_instance(), e1_src_is_transparent_outside_0_1(), e1_uv_origin_top_left() (+22 more)

### Community 23 - "src/encode.rs"
Cohesion: 0.25
Nodes (8): Inner, start(), TempFile, unsupported(), write_temp(), FfmpegMissing, bin(), command()

### Community 24 - "session"
Cohesion: 0.26
Nodes (19): edit(), import_pattern(), next_id(), project(), session(), t_edit_created_shape_and_undo_depth(), h_12_export_onto_a_media_file_is_error(), h_9_remove_def_shadowing_builtin_of_same_kind_ok() (+11 more)

### Community 25 - "clips.rs"
Cohesion: 0.05
Nodes (80): c_add_1_defaults(), c_add_1_g5_g6_g7(), c_add_1_r7_source_bounds_and_slack(), c_add_1_r7_text_source_in_stays_as_is(), c_add_2_c_add_4_linked_overlap_on_audio_side(), c_add_2_linked_pair_id_order(), c_add_2_linked_track_locked(), c_add_5_linked_overwrite_clears_both_targets() (+72 more)

### Community 26 - "effects_builtin.rs"
Cohesion: 0.11
Nodes (32): assert_px_tol(), frame_fn(), px(), C1, check(), eb_brightness_contrast(), eb_chroma_key(), eb_drop_shadow() (+24 more)

### Community 27 - "new_project"
Cohesion: 0.20
Nodes (19): depths(), edit_err(), full(), new_project(), BUILTINS, effects(), t2_core_error_text_is_passed_through(), t2_unknown_tool_and_bad_params_are_errors() (+11 more)

### Community 28 - "Frame"
Cohesion: 0.22
Nodes (23): Frame, quadrants(), solid_frame(), check(), G, green(), K, place() (+15 more)

### Community 29 - "adit-media/tests/common/mod.rs"
Cohesion: 0.22
Nodes (22): av_mp4(), cut_mp4(), ffmpeg(), ffprobe(), gray_h264(), id_video(), nocut_mp4(), probe_video_stream() (+14 more)

### Community 30 - "pattern.rs"
Cohesion: 0.19
Nodes (9): Pattern, assert_pattern(), m_pat_audio_values_and_length(), m_pat_probe(), m_pat_video_frame_30000_1001(), m_pat_video_frame_30fps(), pat(), R2997 (+1 more)

### Community 31 - "Id"
Cohesion: 0.17
Nodes (9): Id, clip(), find_clip(), Fx, media_src(), mk_clip(), p1_q5_defaults_for_every_clip_param(), q3_source_time_rounds_half_away_from_zero() (+1 more)

### Community 32 - "text.rs"
Cohesion: 0.15
Nodes (8): TextAlign, Center, Left, Right, TextSpec, candidates(), font_dirs(), Fonts

### Community 33 - "adit-media/tests/audio.rs"
Cohesion: 0.35
Nodes (16): assert_len(), assert_ramp(), audio_missing_file_is_not_found(), ff(), m_audio_1_from_aac_in_mp4(), m_audio_1_length_is_rounded_duration(), m_audio_1_mono_duplicated_to_both_channels(), m_audio_1_range_past_end_yields_fewer_samples() (+8 more)

### Community 34 - "transitions.rs"
Cohesion: 0.24
Nodes (12): e1_transition_ctx_progress_duration_and_params(), e1_transition_src_a_and_src_b(), e3_project_transition_shadows_builtin(), e3_transition_kind_mismatch_and_unknown(), halves(), solids(), T_CTX, v9_cross_dissolve_spec_example() (+4 more)

### Community 35 - "adit-mcp/tests/common/mod.rs"
Cohesion: 0.20
Nodes (11): F30, find_clip(), find_track(), media_src(), Proj, S, solid(), summary() (+3 more)

### Community 36 - "Error"
Cohesion: 0.14
Nodes (11): Error, InUse, InvalidArgument, KindMismatch, NestCycle, NoAdjacentClip, NotFound, Overlap (+3 more)

### Community 37 - "adit-render/tests/hardening.rs"
Cohesion: 0.13
Nodes (21): validate_effect(), eb_every_builtin_validates(), e4_valid_definitions_pass(), BODY, brackets(), deep_parens(), filter(), h_1_100000_nested_parens_rejected() (+13 more)

### Community 38 - "model.rs"
Cohesion: 0.08
Nodes (27): new_clip(), BlendMode, Add, Darken, Difference, Lighten, Multiply, Normal (+19 more)

### Community 39 - "render_cli.rs"
Cohesion: 0.24
Nodes (14): assert_rgb(), bin(), rgb_at(), run_adit(), binary_bad_usage_exit_2(), binary_help_prints_usage_on_stderr(), binary_render_default_sequence_h264(), binary_render_failures_exit_nonzero_with_message() (+6 more)

### Community 40 - "Mcp"
Cohesion: 0.29
Nodes (3): Mcp, parse_tool_result(), ToolResult

### Community 41 - "RenderError"
Cohesion: 0.14
Nodes (6): CtxVals, FORMAT, Gpu, gpu_err(), new_texture(), Scope

### Community 42 - "Document"
Cohesion: 0.16
Nodes (5): Outcome, Document, execute(), err(), ok()

### Community 43 - "adit-render/tests/export.rs"
Cohesion: 0.20
Nodes (24): AudioCodec, Aac, None, Pcm, ExportSettings, assert_rgb(), audio_fake(), decode() (+16 more)

### Community 44 - "duration"
Cohesion: 0.42
Nodes (7): free_port(), serve(), binary_serve_banner_and_bearer_auth(), binary_serve_default_port_7821(), binary_serve_loopback_only(), binary_serve_random_token_per_launch(), init_params()

### Community 45 - "R"
Cohesion: 0.30
Nodes (29): ToolOutput, A, absolutize(), detect_scenes(), detect_silence(), effect_check(), effect_list(), entry() (+21 more)

### Community 46 - ".render_seq"
Cohesion: 0.28
Nodes (4): blend_index(), param(), Renderer, secs()

### Community 47 - "stdio.rs"
Cohesion: 0.24
Nodes (3): has_timecode(), num_with_suffix(), t1_initialize_reports_adit_tools_and_instructions()

### Community 49 - "INDEX.md"
Cohesion: 0.20
Nodes (5): ffmpeg subprocess gotchas, Knowledge index, MCP layer design notes, Mutation testing adit-core, wgpu 30 / naga headless notes

### Community 50 - "document.rs"
Cohesion: 0.14
Nodes (17): check_param(), finite(), Project, edges(), edges_unique(), end_ok(), nests_itself(), new_track() (+9 more)

### Community 51 - "EffectDef"
Cohesion: 0.27
Nodes (9): BLEND, BUILTINS, internal(), MIX, SCALE, Spec, to_def(), TRANSFORM (+1 more)

### Community 52 - "effects.rs"
Cohesion: 0.08
Nodes (28): EffectDef, EffectKind, Filter, Transition, ParamDef, builtin_effects(), assemble(), assemble_parts() (+20 more)

### Community 53 - "MediaBackend"
Cohesion: 0.22
Nodes (12): atempo_steps_multiply_back(), fetch(), filter_chain(), frame_at(), frame_count(), mix_audio(), mix_seq(), resample() (+4 more)

### Community 54 - "adit-core/tests/hardening.rs"
Cohesion: 0.07
Nodes (45): c_add_2_kind_mismatch_cases(), solid_src(), chain(), edge_fixture(), h_10_batch_prunes_after_each_command(), h_15_remove_sequence_with_locked_track_rejected(), h_15_remove_sequence_without_locked_track_ok(), h_3_add_clip_duration_above_max_time_rejected() (+37 more)

### Community 55 - "adit-core/tests/common/mod.rs"
Cohesion: 0.19
Nodes (21): Keyframe, c_split_1_r4_right_part_copies_and_new_effect_ids(), approx(), assert_approx(), kf(), assert_err!, param(), S (+13 more)

### Community 56 - "RenderError"
Cohesion: 0.25
Nodes (8): Renderer, RenderError, Cancelled, Gpu, Media, SequenceNotFound, Shader, UnknownEffect

### Community 59 - "params.rs"
Cohesion: 0.27
Nodes (12): c_rem_1_g8_prunes_transitions_of_removed_clip(), trk(), add_tr(), c_trans_1_add_two_sided_and_one_sided(), c_trans_1_i5_errors(), c_trans_1_replaces_transition_on_same_edge(), c_trans_2_remove_and_set(), c_trans_3_spec_example() (+4 more)

### Community 60 - "session"
Cohesion: 0.23
Nodes (15): call(), call_err(), t_fxcheck(), list(), t3_project_ids_unique_and_never_reused(), t3_two_projects_are_independent(), t3_unknown_project_id_is_error_everywhere(), t6_sequence_default_is_first_sequence() (+7 more)

### Community 61 - "adit-mcp/tests/common/mod.rs"
Cohesion: 0.24
Nodes (16): approx(), assert_px(), decode(), f(), frame_png(), same_path(), t_edit_define_then_use_in_same_batch_renders_e5(), t_import_one_undo_step_and_fields() (+8 more)

### Community 62 - "adit-mcp/tests/hardening.rs"
Cohesion: 0.15
Nodes (19): add_seq(), cut_mp4(), ffmpeg(), ffmpeg_session(), gap_wav(), h_11_small_contact_sheet_still_works(), h_13_fifth_concurrent_export_is_error(), h_14_detect_with_ffmpeg_session_works() (+11 more)

### Community 63 - "MediaError"
Cohesion: 0.14
Nodes (10): AudioInfo, MediaSpec, Encoder, Ffmpeg, MediaBackend, MediaError, Ffmpeg, Io (+2 more)

### Community 64 - "common"
Cohesion: 0.15
Nodes (12): Session, alive(), h_11_contact_sheet_over_64_megapixels_is_error(), h_1_define_effect_deep_braces_is_error(), h_1_effect_check_deep_braces_is_not_ok(), h_5_audio_levels_huge_end_is_clamped(), h_5_audio_levels_start_past_clamped_end_is_error(), one_second_of_pattern_audio() (+4 more)

### Community 65 - "adit/tests/common/mod.rs"
Cohesion: 0.10
Nodes (22): approx(), color_clip(), cut_and_silence_clip(), drain_stderr(), f(), ffmpeg_frame(), ffprobe(), new_project() (+14 more)

### Community 66 - "add_solid"
Cohesion: 0.30
Nodes (14): add_solid(), assert_rgb(), ffmpeg_frame(), probe_video(), rgb_at(), long_project(), t_export_done_and_file_matches(), t_export_errors() (+6 more)

### Community 67 - "detect.rs"
Cohesion: 0.39
Nodes (8): ff(), m_det_1_missing_file_is_err(), m_det_1_no_cut_is_empty(), m_det_1_no_silence_is_empty(), m_det_1_scene_cut_red_to_blue(), m_det_1_silence_between_tones(), m_det_1_silence_shorter_than_min_duration_is_ignored(), TENTH

### Community 68 - "requests.rs"
Cohesion: 0.36
Nodes (9): request_list(), create(), t_req_1_2_create_and_wait_returns_immediately(), t_req_2_each_request_handed_out_once(), t_req_2_oldest_first(), t_req_2_timeout_returns_null(), t_req_2_wakes_promptly_when_created_from_another_thread(), t_req_3_list_and_complete() (+1 more)

### Community 69 - "text_seq"
Cohesion: 0.29
Nodes (9): assert_size(), busy_project(), centroid(), lit(), text_seq(), v3_text_align_positions_lines_within_block(), v3_text_is_drawn_centred_in_its_colour(), v3_text_newline_breaks_lines() (+1 more)

### Community 70 - "src/probe.rs"
Cohesion: 0.32
Nodes (9): check_file(), flicks(), parse(), parses_streams_and_skips_cover_art(), probe(), rational(), secs(), to_flicks() (+1 more)

### Community 71 - "CLAUDE.md"
Cohesion: 0.22
Nodes (7): Contracts and test-first rule, Default working style: caveman + ponytail, graphify (code knowledge graph), Knowledge base (always maintain), One task per session, Project, Token budget: query before reading

### Community 72 - "edit"
Cohesion: 0.17
Nodes (12): A<'a>, audio_levels(), builtins(), check_effect(), check_effect_refs(), db(), edit(), export() (+4 more)

### Community 73 - "renderer"
Cohesion: 0.18
Nodes (12): renderer(), audio_project(), Dir, h_12_cancelled_export_keeps_existing_file(), h_12_failing_export_keeps_existing_file(), h_12_successful_export_replaces_existing_file(), h_5_export_longer_than_4_hours_rejected(), h_5_mix_audio_negative_duration_rejected() (+4 more)

### Community 74 - "Media"
Cohesion: 0.67
Nodes (3): Media, Ffmpeg, Pattern

### Community 76 - "params.rs"
Cohesion: 0.25
Nodes (17): ParamTarget, pt(), pte(), add_fx(), c_eff_1_add_effect(), c_eff_1_errors(), c_eff_2_remove_move_enable(), c_eff_2_unknown_effect_on_clip() (+9 more)

### Community 77 - "main.rs"
Cohesion: 0.24
Nodes (14): VideoCodec, Gif, H264, H265, Prores, backend(), export(), fail() (+6 more)

### Community 78 - "src/export.rs"
Cohesion: 0.14
Nodes (5): COUNTER, export(), MAX_EXPORT, temp_path(), DIRS

### Community 79 - "proc.rs"
Cohesion: 0.21
Nodes (6): failed(), Proc, run(), spawn_err(), TAIL, tail()

### Community 80 - "props.rs"
Cohesion: 0.23
Nodes (11): all_clip_ids(), clip_ids(), effect_ids(), H, Op, op(), pick(), to_cmd() (+3 more)

### Community 81 - "adit-render/tests/hardening_crash.rs"
Cohesion: 0.21
Nodes (8): audio_project(), backend(), braces(), filter(), h_1_brace_depth_64_ok(), h_1_brace_depth_65_rejected(), h_5_mix_audio_duration_above_max_time_rejected(), h_5_mix_audio_duration_i64_max_rejected()

### Community 82 - "contact_sheet"
Cohesion: 0.33
Nodes (8): Image, contact_sheet(), encode(), render(), render_frame(), resize(), scale_to(), secs()

### Community 83 - "Easing"
Cohesion: 0.33
Nodes (6): Easing, EaseIn, EaseInOut, EaseOut, Hold, Linear

## Knowledge Gaps
- **115 isolated node(s):** `Running`, `Done`, `Failed`, `Cancelled`, `Media` (+110 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 338 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **7 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Id` connect `Id` to `Command`, `Project`, `time.rs`, `base`, `src/tools.rs`, `trims.rs`, `validate.rs`, `history.rs`, `seq_src`, `clips.rs`, `Error`, `model.rs`, `Document`, `document.rs`, `adit-core/tests/hardening.rs`, `RenderError`, `params.rs`, `edit`, `params.rs`, `props.rs`, `contact_sheet`, `adit-render/src/lib.rs`?**
  _High betweenness centrality (0.100) - this node is a cross-community bridge._
- **Are the 163 inferred relationships involving `base()` (e.g. with `c_add_1_default_names()` and `c_add_1_defaults()`) actually correct?**
  _`base()` has 163 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Running`, `Done`, `Failed` to the rest of the system?**
  _115 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Command` be split into smaller, more focused modules?**
  _Cohesion score 0.038461538461538464 - nodes in this community are weakly interconnected._
- **Why does `RenderError` connect `RenderError` to `RenderError`, `adit-render/tests/common/mod.rs`, `adit-render/tests/export.rs`, `src/export.rs`, `.render_seq`, `EffectDef`, `adit-render/src/lib.rs`, `MediaBackend`, `effects_runtime.rs`, `MediaError`, `Id`?**
  _High betweenness centrality (0.071) - this node is a cross-community bridge._
- **Are the 97 inferred relationships involving `Id` (e.g. with `alloc()` and `clip_ix()`) actually correct?**
  _`Id` has 97 INFERRED edges - model-reasoned connections that need verification._
- **Should `adit-core/tests/common/mod.rs` be split into smaller, more focused modules?**
  _Cohesion score 0.14705882352941177 - nodes in this community are weakly interconnected._