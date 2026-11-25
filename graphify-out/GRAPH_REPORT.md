# Graph Report - adit  (2026-11-25)

## Corpus Check
- 101 files · ~141,832 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 3 file(s) not represented in the graph (top: (none) 2, .proptest-regressions 1)

## Summary
- 2509 nodes · 7862 edges · 105 communities (94 shown, 11 thin omitted)
- Extraction: 73% EXTRACTED · 27% INFERRED · 0% AMBIGUOUS · INFERRED: 2152 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `a638ad4b`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Command
- Action
- adit-media/src/lib.rs
- Project
- Part 1 — adit-core
- time.rs
- base
- MediaError
- src/tools.rs
- adit-render/tests/audio.rs
- adit-media/tests/hardening.rs
- trims.rs
- validate.rs
- adit-core
- frames.rs
- drags.rs
- encode.rs
- timed
- decode.rs
- session_api.rs
- Tools
- assert_px
- effects_runtime.rs
- proc.rs
- adit-render/tests/common/mod.rs
- clips.rs
- effects_builtin.rs
- session
- Frame
- adit-media/tests/common/mod.rs
- pattern.rs
- Time
- text.rs
- adit-media/tests/audio.rs
- proto.rs
- adit-mcp/tests/common/mod.rs
- Error
- adit-render/tests/hardening.rs
- BlendMode
- render_cli.rs
- Mcp
- RenderError
- edits.rs
- adit-render/tests/export.rs
- duration
- e2e_story_real_ffmpeg
- .render_seq
- adit-gui/tests/common/mod.rs
- Http
- INDEX.md
- document.rs
- EffectDef
- effects.rs
- MediaBackend
- adit-core/tests/hardening.rs
- adit-core/tests/common/mod.rs
- main.rs
- session
- adit-mcp/tests/common/mod.rs
- adit-mcp/tests/hardening.rs
- MediaError
- Session
- ui
- add_solid
- geometry.rs
- requests.rs
- adit
- logic.rs
- CLAUDE.md
- adit GUI contract (SPEC Part 6)
- session_api.rs
- B
- history.rs
- RenderError
- act
- Id
- renderer
- slide_fx
- adit-render/tests/hardening_crash.rs
- trk
- stdio.rs
- Inner
- R
- adit-core/tests/common/mod.rs
- UiBridge
- props.rs
- adit/tests/common/mod.rs
- Document
- render_raw
- speed
- src/probe.rs
- Hit
- Easing
- JobState
- TrimMode
- def
- Edge

## God Nodes (most connected - your core abstractions)
1. `base()` - 169 edges
2. `Id` - 150 edges
3. `Time` - 125 edges
4. `Command` - 99 edges
5. `session()` - 89 edges
6. `Project` - 87 edges
7. `MediaError` - 74 edges
8. `Action` - 73 edges
9. `assert_px()` - 70 edges
10. `render()` - 68 edges

## Surprising Connections (you probably didn't know these)
- `u_state_defaults()` --calls--> `ui()`  [INFERRED]
  crates/adit-gui/tests/state_keys.rs → crates/adit-gui/tests/common/mod.rs
- `ALL_ACTIONS` --references--> `Action`  [INFERRED]
  crates/adit-gui/tests/common/mod.rs → crates/adit-gui/src/lib.rs
- `S` --references--> `Time`  [INFERRED]
  crates/adit-core/tests/common/mod.rs → crates/adit-core/src/model.rs
- `MT` --references--> `Time`  [INFERRED]
  crates/adit-core/tests/hardening.rs → crates/adit-core/src/model.rs
- `H` --references--> `Time`  [INFERRED]
  crates/adit-core/tests/props.rs → crates/adit-core/src/model.rs

## Import Cycles
- 1-file cycle: `crates/adit-media/src/proc.rs -> crates/adit-media/src/proc.rs`
- 1-file cycle: `crates/adit-render/src/effects.rs -> crates/adit-render/src/effects.rs`
- 2-file cycle: `crates/adit-gui/src/app.rs -> crates/adit-gui/src/lib.rs -> crates/adit-gui/src/app.rs`
- 2-file cycle: `crates/adit-media/src/lib.rs -> crates/adit-media/src/proc.rs -> crates/adit-media/src/lib.rs`
- 2-file cycle: `crates/adit-mcp/src/lib.rs -> crates/adit-mcp/src/proto.rs -> crates/adit-mcp/src/lib.rs`

## Communities (105 total, 11 thin omitted)

### Community 0 - "Command"
Cohesion: 0.04
Nodes (46): Command, AddClip, AddEffect, AddMarker, AddMedia, AddSequence, AddTrack, AddTransition (+38 more)

### Community 1 - "Action"
Cohesion: 0.04
Nodes (55): Action, AddEdit, AddEditAll, AddMarker, ApplyAudioTransition, ApplyDefaultTransitions, ApplyVideoTransition, ClearIn (+47 more)

### Community 2 - "adit-media/src/lib.rs"
Cohesion: 0.26
Nodes (10): AUDIO_FILTERS, lock(), number_after(), parse_scenes(), parse_silence(), seconds(), seconds_and_log_parsing(), secs_to_flicks() (+2 more)

### Community 3 - "Project"
Cohesion: 0.18
Nodes (43): alloc(), bad(), bounds_ok(), check_locked(), clear(), clip_ix(), clip_mut(), clip_ref() (+35 more)

### Community 4 - "Part 1 — adit-core"
Cohesion: 0.11
Nodes (18): adit behavioural spec, Audio, Batch, Clips, Commands, Effects, Export, General rules (+10 more)

### Community 5 - "time.rs"
Cohesion: 0.19
Nodes (16): Rational, convert_times(), fps_strings(), frames_to_time(), gcd(), is_decimal(), parse_fps(), parse_int() (+8 more)

### Community 6 - "base"
Cohesion: 0.07
Nodes (40): c_add_6_r1_insert_splits_and_shifts_all_unlocked_tracks(), sq(), h_3_add_media_duration_bound(), q1_duration_is_max_clip_end_over_all_tracks(), q1_duration_of_empty_sequence_is_zero(), add_seq(), add_track(), c_fx_1_core_does_not_check_effect_names() (+32 more)

### Community 7 - "MediaError"
Cohesion: 0.14
Nodes (11): AudioInfo, MediaSpec, VideoInfo, MissingMedia, Encoder, Ffmpeg, MediaError, Ffmpeg (+3 more)

### Community 8 - "src/tools.rs"
Cohesion: 0.09
Nodes (28): absolutize(), builtins(), db(), def_json(), edit_description(), effect_list(), Entry, entry() (+20 more)

### Community 9 - "adit-render/tests/audio.rs"
Cohesion: 0.20
Nodes (33): a1_count_uses_sequence_sample_rate(), a1_empty_space_is_silence(), a1_exact_sample_counts(), a1_mix_window_offset(), a1_mute_solo_disabled(), a1_only_audio_tracks_contribute(), a2_atempo_chained_within_ffmpeg_range(), a2_filter_chain_order_and_atempo() (+25 more)

### Community 10 - "adit-media/tests/hardening.rs"
Cohesion: 0.07
Nodes (46): check_audio_filter(), gray_frame_number(), TempDir, ALLOWED, ametadata_to(), ff(), FORBIDDEN, frame_to_time() (+38 more)

### Community 11 - "trims.rs"
Cohesion: 0.14
Nodes (38): trim(), c_slide_1_needs_previous_clip(), c_slip_1_moves_source_only_with_linked(), c_slip_1_r7_bounds(), c_speed_1_r7_source_bounds(), c_trim_1_linked_same_edge_only(), c_trim_1_r7_applies_to_linked_clips(), c_trim_2_overlap_and_duration_errors() (+30 more)

### Community 12 - "validate.rs"
Cohesion: 0.12
Nodes (39): text_spec(), text_src(), i1_negative_start_rejected(), i1_overlap_rejected(), i1_touching_clips_are_fine_half_open(), i1_unsorted_rejected(), i1_zero_and_negative_duration_rejected(), i2_duplicate_clip_ids_rejected() (+31 more)

### Community 13 - "adit-core"
Cohesion: 0.87
Nodes (6): adit, adit-core, adit-gui, adit-mcp, adit-media, adit-render

### Community 14 - "frames.rs"
Cohesion: 0.11
Nodes (42): assert_id_frame(), id_video(), t2997(), t30(), assert_still(), backwards(), ff(), Fixture (+34 more)

### Community 15 - "drags.rs"
Cohesion: 0.13
Nodes (61): Drag, Tool, Hand, Pen, RateStretch, Razor, Ripple, Roll (+53 more)

### Community 16 - "encode.rs"
Cohesion: 0.22
Nodes (26): ffmpeg_decode_frame(), flat_rgba(), probe_audio_streams(), stereo_sine(), assert_video(), check_roundtrip(), COLOURS, encode() (+18 more)

### Community 17 - "timed"
Cohesion: 0.18
Nodes (23): timed(), ff(), m_det_1_missing_file_is_err(), m_det_1_no_cut_is_empty(), m_det_1_no_silence_is_empty(), m_det_1_scene_cut_red_to_blue(), m_det_1_silence_between_tones(), m_det_1_silence_shorter_than_min_duration_is_ignored() (+15 more)

### Community 18 - "decode.rs"
Cohesion: 0.16
Nodes (14): CACHE_BYTES, decode_still(), f(), FileDec, frame_len(), Frames, Key, Lru (+6 more)

### Community 19 - "session_api.rs"
Cohesion: 0.07
Nodes (30): import_pattern(), h_12_export_onto_a_media_file_is_error(), assert_same_pixels(), bridge_setup(), clip_count(), FakeBridge, HOLD, pattern() (+22 more)

### Community 20 - "Tools"
Cohesion: 0.15
Nodes (12): adit MCP contract (SPEC Part 4), Binary, Editing, Effects, Export, General rules, Library surface (`adit_mcp`), Magic-wand requests (+4 more)

### Community 21 - "assert_px"
Cohesion: 0.09
Nodes (63): assert_px(), assert_size(), render(), busy_project(), centroid(), green(), lit(), small() (+55 more)

### Community 22 - "effects_runtime.rs"
Cohesion: 0.14
Nodes (17): e1_ctx_size_is_image_size(), e1_helpers_and_constants_allowed(), e1_output_is_straight_alpha(), e1_src_is_transparent_outside_0_1(), e1_uv_origin_top_left(), e2_missing_param_takes_paramdef_default(), e4_never_panics_on_garbage(), e4_rejects_bad_param_names() (+9 more)

### Community 23 - "proc.rs"
Cohesion: 0.22
Nodes (6): failed(), Proc, run(), spawn_err(), TAIL, tail()

### Community 24 - "adit-render/tests/common/mod.rs"
Cohesion: 0.13
Nodes (18): AudioCall, AudioFn, BLUE, const_audio(), F30, Fake, FPS30, GREEN (+10 more)

### Community 25 - "clips.rs"
Cohesion: 0.05
Nodes (86): c_add_1_defaults(), c_add_1_g5_g6_g7(), c_add_1_r7_source_bounds_and_slack(), c_add_1_r7_text_source_in_stays_as_is(), c_add_2_c_add_4_linked_overlap_on_audio_side(), c_add_2_linked_pair_id_order(), c_add_2_linked_track_locked(), c_add_5_linked_overwrite_clears_both_targets() (+78 more)

### Community 26 - "effects_builtin.rs"
Cohesion: 0.12
Nodes (34): Frame, assert_px_tol(), frame_fn(), px(), time_frame(), C1, check(), eb_brightness_contrast() (+26 more)

### Community 27 - "session"
Cohesion: 0.20
Nodes (18): add_solid(), edit_err(), full(), BUILTINS, effects(), t2_core_error_text_is_passed_through(), t2_unknown_tool_and_bad_params_are_errors(), t_edit_batch_is_atomic_when_third_command_fails() (+10 more)

### Community 28 - "Frame"
Cohesion: 0.21
Nodes (22): quadrants(), solid_frame(), check(), G, green(), K, place(), v5_crop_each_side_of_solid() (+14 more)

### Community 29 - "adit-media/tests/common/mod.rs"
Cohesion: 0.23
Nodes (21): av_mp4(), cut_mp4(), ffmpeg(), ffprobe(), gray_h264(), nocut_mp4(), probe_video_stream(), ramp_mono_wav() (+13 more)

### Community 30 - "pattern.rs"
Cohesion: 0.19
Nodes (9): Pattern, assert_pattern(), m_pat_audio_values_and_length(), m_pat_probe(), m_pat_video_frame_30000_1001(), m_pat_video_frame_30fps(), pat(), R2997 (+1 more)

### Community 31 - "Time"
Cohesion: 0.12
Nodes (14): Clip, Param, Sequence, time_to_frame(), Time, q4_time_to_frame_is_largest_frame_not_after_t(), TimelineLayout, hit() (+6 more)

### Community 32 - "text.rs"
Cohesion: 0.22
Nodes (3): candidates(), font_dirs(), Fonts

### Community 33 - "adit-media/tests/audio.rs"
Cohesion: 0.35
Nodes (16): assert_len(), assert_ramp(), audio_missing_file_is_not_found(), ff(), m_audio_1_from_aac_in_mp4(), m_audio_1_length_is_rounded_duration(), m_audio_1_mono_duplicated_to_both_channels(), m_audio_1_range_past_end_yields_fewer_samples() (+8 more)

### Community 34 - "proto.rs"
Cohesion: 0.15
Nodes (12): call_tool(), constant_time_eq(), err(), handle(), header(), http_request(), INSTRUCTIONS, is_blocking() (+4 more)

### Community 35 - "adit-mcp/tests/common/mod.rs"
Cohesion: 0.13
Nodes (32): depths(), edit(), f(), F30, find_clip(), find_track(), media_src(), new_project() (+24 more)

### Community 36 - "Error"
Cohesion: 0.18
Nodes (11): Error, InUse, InvalidArgument, KindMismatch, NestCycle, NoAdjacentClip, NotFound, Overlap (+3 more)

### Community 37 - "adit-render/tests/hardening.rs"
Cohesion: 0.12
Nodes (23): COUNTER, validate_effect(), eb_every_builtin_validates(), e4_valid_definitions_pass(), BODY, brackets(), deep_parens(), DIRS (+15 more)

### Community 38 - "BlendMode"
Cohesion: 0.22
Nodes (9): BlendMode, Add, Darken, Difference, Lighten, Multiply, Normal, Overlay (+1 more)

### Community 39 - "render_cli.rs"
Cohesion: 0.14
Nodes (19): assert_rgb(), ffmpeg_frame(), ffprobe(), Probe, rgb_at(), run_adit(), run_ok(), binary_bare_adit_still_usage_error() (+11 more)

### Community 40 - "Mcp"
Cohesion: 0.27
Nodes (4): Mcp, new_project(), parse_tool_result(), ToolResult

### Community 41 - "RenderError"
Cohesion: 0.12
Nodes (12): CtxVals, FORMAT, Gpu, gpu_err(), new_texture(), Scope, RenderError, Cancelled (+4 more)

### Community 42 - "edits.rs"
Cohesion: 0.13
Nodes (54): apply(), clips_on(), edit(), add(), after_insert(), AUD, audio_cmds(), AV10 (+46 more)

### Community 43 - "adit-render/tests/export.rs"
Cohesion: 0.26
Nodes (19): assert_rgb(), audio_fake(), decode(), out_path(), probe_audio(), probe_video(), red_blue(), run_export() (+11 more)

### Community 44 - "duration"
Cohesion: 0.23
Nodes (11): bin(), free_port(), Media, Ffmpeg, Pattern, serve(), binary_serve_banner_and_bearer_auth(), binary_serve_default_port_7821() (+3 more)

### Community 45 - "e2e_story_real_ffmpeg"
Cohesion: 0.19
Nodes (11): approx(), color_clip(), cut_and_silence_clip(), f(), px(), same_path(), clip(), e2e_detect_silence_and_scenes() (+3 more)

### Community 46 - ".render_seq"
Cohesion: 0.28
Nodes (4): blend_index(), param(), Renderer, secs()

### Community 47 - "adit-gui/tests/common/mod.rs"
Cohesion: 0.09
Nodes (21): Clip, A1, A2, ALL_ACTIONS, B, clip(), F, FPS30 (+13 more)

### Community 49 - "INDEX.md"
Cohesion: 0.20
Nodes (5): ffmpeg subprocess gotchas, Knowledge index, MCP layer design notes, Mutation testing adit-core, wgpu 30 / naga headless notes

### Community 50 - "document.rs"
Cohesion: 0.08
Nodes (28): check_param(), finite(), Project, edges_unique(), end_ok(), new_track(), transition_ok(), Clip (+20 more)

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
Cohesion: 0.10
Nodes (43): c_add_1_default_names(), c_add_1_i4_kind_mismatch(), c_add_2_kind_mismatch_cases(), base(), solid_src(), edge_fixture(), h_10_batch_prunes_after_each_command(), h_15_remove_sequence_with_locked_track_rejected() (+35 more)

### Community 55 - "adit-core/tests/common/mod.rs"
Cohesion: 0.13
Nodes (26): frame_to_time(), Keyframe, c_split_1_r4_right_part_copies_and_new_effect_ids(), assert_approx(), kf(), param(), ELEVEN, lookups_sequence_track_clip() (+18 more)

### Community 59 - "main.rs"
Cohesion: 0.17
Nodes (19): AudioCodec, Aac, None, Pcm, ExportSettings, VideoCodec, Gif, H264 (+11 more)

### Community 60 - "session"
Cohesion: 0.20
Nodes (24): call(), call_err(), session(), summary(), t_fxcheck(), h_11_small_contact_sheet_still_works(), h_1_effect_check_too_deep_is_not_ok_and_session_answers(), h_5_audio_levels_start_not_before_end_is_error() (+16 more)

### Community 61 - "adit-mcp/tests/common/mod.rs"
Cohesion: 0.30
Nodes (13): approx(), assert_px(), decode(), frame_png(), t_edit_define_then_use_in_same_batch_renders_e5(), red_then_blue(), t_frame_formats_magic_and_colour(), t_frame_media_pattern_pixels() (+5 more)

### Community 62 - "adit-mcp/tests/hardening.rs"
Cohesion: 0.25
Nodes (13): add_seq(), cut_mp4(), ffmpeg(), ffmpeg_session(), gap_wav(), h_13_fifth_concurrent_export_is_error(), h_14_detect_with_ffmpeg_session_works(), h_14_detect_with_pattern_session_is_error() (+5 more)

### Community 63 - "MediaError"
Cohesion: 0.25
Nodes (8): Inner, start(), TempFile, unsupported(), write_temp(), FfmpegMissing, bin(), command()

### Community 64 - "Session"
Cohesion: 0.27
Nodes (9): Session, alive(), h_11_contact_sheet_over_64_megapixels_is_error(), h_1_define_effect_deep_braces_is_error(), h_1_effect_check_deep_braces_is_not_ok(), h_5_audio_levels_huge_end_is_clamped(), h_5_audio_levels_start_past_clamped_end_is_error(), one_second_of_pattern_audio() (+1 more)

### Community 65 - "ui"
Cohesion: 0.17
Nodes (32): ui(), mark_project(), match_project(), nav(), nav_project(), only(), select_project(), step() (+24 more)

### Community 66 - "add_solid"
Cohesion: 0.25
Nodes (13): assert_rgb(), ffmpeg_frame(), probe_video(), rgb_at(), long_project(), t_export_done_and_file_matches(), t_export_errors(), t_export_options_width_height_fps() (+5 more)

### Community 67 - "geometry.rs"
Cohesion: 0.17
Nodes (25): at_clip(), G, hit_project(), hl(), layout(), PX, snap_project(), snap_ui() (+17 more)

### Community 68 - "requests.rs"
Cohesion: 0.24
Nodes (10): request_list(), create(), t_req_1_2_create_and_wait_returns_immediately(), t_req_2_each_request_handed_out_once(), t_req_2_oldest_first(), t_req_2_timeout_returns_null(), t_req_2_wakes_promptly_when_created_from_another_thread(), t_req_3_list_and_complete() (+2 more)

### Community 70 - "logic.rs"
Cohesion: 0.11
Nodes (39): Sequence, drag(), Op, Edit, Export, GainDialog, Import, Redo (+31 more)

### Community 71 - "CLAUDE.md"
Cohesion: 0.22
Nodes (7): Contracts and test-first rule, Default working style: caveman + ponytail, graphify (code knowledge graph), Knowledge base (always maintain), One task per session, Project, Token budget: query before reading

### Community 72 - "adit GUI contract (SPEC Part 6)"
Cohesion: 0.20
Nodes (9): Actions (`perform`), adit GUI contract (SPEC Part 6), Binary, Keys, Session additions (`adit_mcp::Session`), State, Timeline geometry, Timeline gestures (`drag`) (+1 more)

### Community 73 - "session_api.rs"
Cohesion: 0.09
Nodes (51): App, run(), Options, active(), add_named(), call(), call_stepping(), clip_ids() (+43 more)

### Community 74 - "B"
Cohesion: 0.10
Nodes (13): B, keyed(), param(), try_render(), e1_params_declared_in_order_and_overridden_by_instance(), e2_keyframed_param_evaluated_at_local_time(), e3_transition_kind_on_clip_is_unknown_effect(), e4_invalid_project_def_fails_render() (+5 more)

### Community 76 - "history.rs"
Cohesion: 0.15
Nodes (22): c_add_2_unlinked_has_no_link(), c_add_4_place_overlap_and_touching(), add_m(), add_marker(), c_batch_1_empty_batch_is_ok_noop(), c_batch_1_first_error_fails_all(), c_batch_1_runs_in_order_one_undo_entry(), doc_with_clip() (+14 more)

### Community 78 - "RenderError"
Cohesion: 0.12
Nodes (6): MediaBackend, export(), MAX_EXPORT, temp_path(), Renderer, SequenceNotFound

### Community 79 - "act"
Cohesion: 0.09
Nodes (31): default_keymap(), KeyChord, default_keymap(), act(), assert_noop(), link_project(), u_del_empty_selection_no_op(), u_enable_empty_no_op() (+23 more)

### Community 80 - "Id"
Cohesion: 0.12
Nodes (17): new_clip(), ClipSource, Adjustment, Media, Sequence, Solid, Text, Id (+9 more)

### Community 81 - "renderer"
Cohesion: 0.18
Nodes (12): renderer(), audio_project(), Dir, h_12_cancelled_export_keeps_existing_file(), h_12_failing_export_keeps_existing_file(), h_12_successful_export_replaces_existing_file(), h_5_export_longer_than_4_hours_rejected(), h_5_mix_audio_negative_duration_rejected() (+4 more)

### Community 82 - "slide_fx"
Cohesion: 0.67
Nodes (3): c_slide_1_errors(), c_slide_1_forward_and_back(), slide_fx()

### Community 83 - "adit-render/tests/hardening_crash.rs"
Cohesion: 0.21
Nodes (8): audio_project(), backend(), braces(), filter(), h_1_brace_depth_64_ok(), h_1_brace_depth_65_rejected(), h_5_mix_audio_duration_above_max_time_rejected(), h_5_mix_audio_duration_i64_max_rejected()

### Community 84 - "trk"
Cohesion: 0.14
Nodes (30): ParamTarget, c_rem_1_g8_prunes_transitions_of_removed_clip(), pt(), pte(), trk(), add_fx(), add_tr(), c_eff_1_add_effect() (+22 more)

### Community 85 - "stdio.rs"
Cohesion: 0.22
Nodes (3): has_timecode(), num_with_suffix(), t1_initialize_reports_adit_tools_and_instructions()

### Community 86 - "Inner"
Cohesion: 0.21
Nodes (30): ToolOutput, A, bridge(), detect_scenes(), detect_silence(), effect_check(), find_job(), Inner (+22 more)

### Community 89 - "R"
Cohesion: 0.22
Nodes (17): A<'a>, audio_levels(), check_effect(), check_effect_refs(), contact_sheet(), edit(), encode(), export() (+9 more)

### Community 90 - "adit-core/tests/common/mod.rs"
Cohesion: 0.10
Nodes (23): PlaceMode, Insert, Overwrite, Place, c_add_1_nest_cycle_direct_and_indirect(), c_add_1_r7_stills_and_sequences_unbounded(), c_add_1_sequence_on_audio_track_ok(), c_add_2_linked_sequence_source() (+15 more)

### Community 91 - "UiBridge"
Cohesion: 0.15
Nodes (8): Image, ProjectInfo, ToolInfo, UiBridge, call(), panic_msg(), projects(), set_ui()

### Community 92 - "props.rs"
Cohesion: 0.22
Nodes (12): all_clip_ids(), clip_ids(), effect_ids(), H, Op, op(), pick(), to_cmd() (+4 more)

### Community 93 - "adit/tests/common/mod.rs"
Cohesion: 0.19
Nodes (7): drain_stderr(), Proc, S, Served, solid(), SWAP_WGSL, TOOL_NAMES

### Community 94 - "Document"
Cohesion: 0.18
Nodes (4): Outcome, Document, err(), ok()

### Community 95 - "render_raw"
Cohesion: 0.21
Nodes (4): mix(), render_raw(), snapshot(), source_frame()

### Community 96 - "speed"
Cohesion: 0.29
Nodes (10): c_speed_1_linked(), c_speed_1_overlap_without_ripple(), c_speed_1_r1_ripple(), c_speed_1_range_and_kinds(), c_speed_1_rounding_and_min_duration(), c_speed_1_sequence_clip_ok(), c_speed_1_spec_example_no_ripple(), c_speed_1_uses_old_speed() (+2 more)

### Community 97 - "src/probe.rs"
Cohesion: 0.49
Nodes (8): check_file(), flicks(), parse(), parses_streams_and_skips_cover_art(), probe(), rational(), secs(), to_flicks()

### Community 98 - "Hit"
Cohesion: 0.22
Nodes (9): ClipPart, Body, EndEdge, StartEdge, Hit, Empty, Header, Nothing (+1 more)

### Community 99 - "Easing"
Cohesion: 0.33
Nodes (6): Easing, EaseIn, EaseInOut, EaseOut, Hold, Linear

### Community 100 - "JobState"
Cohesion: 0.40
Nodes (5): JobState, Cancelled, Done, Failed, Running

### Community 101 - "TrimMode"
Cohesion: 0.50
Nodes (4): TrimMode, Normal, Ripple, Roll

### Community 103 - "Edge"
Cohesion: 0.67
Nodes (3): Edge, End, Start

## Knowledge Gaps
- **199 isolated node(s):** `AddClip`, `AddEffect`, `AddMarker`, `AddMedia`, `AddSequence` (+194 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 451 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **11 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Time` connect `Time` to `Command`, `adit-media/src/lib.rs`, `Project`, `time.rs`, `base`, `MediaError`, `src/tools.rs`, `trims.rs`, `validate.rs`, `drags.rs`, `decode.rs`, `session_api.rs`, `clips.rs`, `pattern.rs`, `document.rs`, `adit-core/tests/hardening.rs`, `adit-core/tests/common/mod.rs`, `logic.rs`, `session_api.rs`, `history.rs`, `RenderError`, `Id`, `trk`, `R`, `adit-core/tests/common/mod.rs`, `props.rs`, `render_raw`, `src/probe.rs`, `Hit`?**
  _High betweenness centrality (0.096) - this node is a cross-community bridge._
- **Are the 163 inferred relationships involving `base()` (e.g. with `c_add_1_default_names()` and `c_add_1_defaults()`) actually correct?**
  _`base()` has 163 INFERRED edges - model-reasoned connections that need verification._
- **What connects `AddClip`, `AddEffect`, `AddMarker` to the rest of the system?**
  _199 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Command` be split into smaller, more focused modules?**
  _Cohesion score 0.043478260869565216 - nodes in this community are weakly interconnected._
- **Why does `Id` connect `Id` to `Command`, `Project`, `base`, `src/tools.rs`, `trims.rs`, `validate.rs`, `drags.rs`, `clips.rs`, `Time`, `Error`, `RenderError`, `document.rs`, `adit-core/tests/hardening.rs`, `logic.rs`, `history.rs`, `RenderError`, `slide_fx`, `trk`, `R`, `adit-core/tests/common/mod.rs`, `props.rs`, `Document`, `render_raw`, `speed`, `Hit`?**
  _High betweenness centrality (0.089) - this node is a cross-community bridge._
- **Are the 97 inferred relationships involving `Id` (e.g. with `alloc()` and `clip_ix()`) actually correct?**
  _`Id` has 97 INFERRED edges - model-reasoned connections that need verification._
- **Should `Action` be split into smaller, more focused modules?**
  _Cohesion score 0.03636363636363636 - nodes in this community are weakly interconnected._