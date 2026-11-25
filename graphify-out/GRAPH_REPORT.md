# Graph Report - adit  (2026-11-25)

## Corpus Check
- 111 files · ~159,603 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 3 file(s) not represented in the graph (top: (none) 2, .proptest-regressions 1)

## Summary
- 2806 nodes · 8896 edges · 106 communities (89 shown, 17 thin omitted)
- Extraction: 74% EXTRACTED · 26% INFERRED · 0% AMBIGUOUS · INFERRED: 2317 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `8ffbb662`
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
- src/encode.rs
- adit-render/tests/common/mod.rs
- clips.rs
- effects_builtin.rs
- apply
- Frame
- adit-media/tests/common/mod.rs
- pattern.rs
- App
- text.rs
- adit-media/tests/audio.rs
- proto.rs
- new_project
- adit-render/tests/export.rs
- adit-render/tests/hardening.rs
- timeline.rs
- render_cli.rs
- Mcp
- RenderError
- edits.rs
- App
- duration
- playback.rs
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
- monitors.rs
- Session
- ui
- add_solid
- geometry.rs
- requests.rs
- adit
- logic.rs
- CLAUDE.md
- adit GUI contract (SPEC Part 6)
- fix_app.rs
- B
- history.rs
- src/app.rs
- RenderError
- act
- Id
- panels.rs
- Tool
- adit-render/tests/hardening_crash.rs
- trk
- stdio.rs
- red
- Project
- duration
- .call
- adit/tests/common/mod.rs
- adit-mcp/tests/common/mod.rs
- e2e_story_real_ffmpeg
- ExportSettings
- src/probe.rs
- Track
- KeyChord
- Dialog
- PlaceMode
- adit-render/src/lib.rs
- def

## God Nodes (most connected - your core abstractions)
1. `Id` - 181 edges
2. `base()` - 169 edges
3. `Time` - 147 edges
4. `Command` - 104 edges
5. `Project` - 96 edges
6. `session()` - 89 edges
7. `MediaError` - 78 edges
8. `Action` - 77 edges
9. `assert_px()` - 70 edges
10. `render()` - 68 edges

## Surprising Connections (you probably didn't know these)
- `ALL_ACTIONS` --references--> `Action`  [INFERRED]
  crates/adit-gui/tests/common/mod.rs → crates/adit-gui/src/lib.rs
- `ELEVEN` --references--> `Rational`  [INFERRED]
  crates/adit-core/tests/queries.rs → crates/adit-core/src/model.rs
- `NTSC` --references--> `Rational`  [INFERRED]
  crates/adit-core/tests/queries.rs → crates/adit-core/src/model.rs
- `filter()` --references--> `B`  [INFERRED]
  crates/adit-render/tests/effects_runtime.rs → crates/adit-render/tests/common/mod.rs
- `MT` --references--> `Time`  [INFERRED]
  crates/adit-core/tests/hardening.rs → crates/adit-core/src/model.rs

## Import Cycles
- 1-file cycle: `crates/adit-media/src/proc.rs -> crates/adit-media/src/proc.rs`
- 1-file cycle: `crates/adit-render/src/effects.rs -> crates/adit-render/src/effects.rs`
- 2-file cycle: `crates/adit-gui/src/app.rs -> crates/adit-gui/src/lib.rs -> crates/adit-gui/src/app.rs`
- 2-file cycle: `crates/adit-mcp/src/lib.rs -> crates/adit-mcp/src/proto.rs -> crates/adit-mcp/src/lib.rs`
- 2-file cycle: `crates/adit-media/src/lib.rs -> crates/adit-media/src/proc.rs -> crates/adit-media/src/lib.rs`

## Communities (106 total, 17 thin omitted)

### Community 0 - "Command"
Cohesion: 0.04
Nodes (50): Command, AddClip, AddEffect, AddMarker, AddMedia, AddSequence, AddTrack, AddTransition (+42 more)

### Community 1 - "Action"
Cohesion: 0.04
Nodes (55): Action, AddEdit, AddEditAll, AddMarker, ApplyAudioTransition, ApplyDefaultTransitions, ApplyVideoTransition, ClearIn (+47 more)

### Community 2 - "adit-media/src/lib.rs"
Cohesion: 0.26
Nodes (10): AUDIO_FILTERS, lock(), number_after(), parse_scenes(), parse_silence(), seconds(), seconds_and_log_parsing(), secs_to_flicks() (+2 more)

### Community 3 - "Project"
Cohesion: 0.17
Nodes (44): alloc(), bad(), bounds_ok(), check_locked(), clear(), clip_ix(), clip_mut(), clip_ref() (+36 more)

### Community 4 - "Part 1 — adit-core"
Cohesion: 0.11
Nodes (18): adit behavioural spec, Audio, Batch, Clips, Commands, Effects, Export, General rules (+10 more)

### Community 5 - "time.rs"
Cohesion: 0.15
Nodes (19): Rational, FPS24, convert_times(), fps_strings(), frames_to_time(), gcd(), is_decimal(), parse_fps() (+11 more)

### Community 6 - "base"
Cohesion: 0.07
Nodes (41): c_add_6_r1_insert_splits_and_shifts_all_unlocked_tracks(), sq(), h_3_add_media_duration_bound(), q1_duration_is_max_clip_end_over_all_tracks(), q1_duration_of_empty_sequence_is_zero(), add_seq(), add_track(), c_fx_1_core_does_not_check_effect_names() (+33 more)

### Community 7 - "MediaError"
Cohesion: 0.11
Nodes (13): AudioInfo, MediaSpec, VideoInfo, MissingMedia, Offline, Encoder, Ffmpeg, MediaError (+5 more)

### Community 8 - "src/tools.rs"
Cohesion: 0.06
Nodes (93): Image, ToolInfo, ToolOutput, UiBridge, A, A<'a>, absolutize(), audio_levels() (+85 more)

### Community 9 - "adit-render/tests/audio.rs"
Cohesion: 0.20
Nodes (33): a1_count_uses_sequence_sample_rate(), a1_empty_space_is_silence(), a1_exact_sample_counts(), a1_mix_window_offset(), a1_mute_solo_disabled(), a1_only_audio_tracks_contribute(), a2_atempo_chained_within_ffmpeg_range(), a2_filter_chain_order_and_atempo() (+25 more)

### Community 10 - "adit-media/tests/hardening.rs"
Cohesion: 0.07
Nodes (46): ffmpeg(), check_audio_filter(), TempDir, ALLOWED, ametadata_to(), ff(), FORBIDDEN, frame_to_time() (+38 more)

### Community 11 - "trims.rs"
Cohesion: 0.09
Nodes (57): Edge, End, Start, TrimMode, Normal, Ripple, Roll, trim() (+49 more)

### Community 12 - "validate.rs"
Cohesion: 0.13
Nodes (35): i1_negative_start_rejected(), i1_overlap_rejected(), i1_touching_clips_are_fine_half_open(), i1_unsorted_rejected(), i1_zero_and_negative_duration_rejected(), i2_duplicate_clip_ids_rejected(), i2_id_not_below_next_id_rejected(), i2_ids_unique_across_entity_kinds() (+27 more)

### Community 13 - "adit-core"
Cohesion: 0.93
Nodes (6): adit, adit-core, adit-gui, adit-mcp, adit-media, adit-render

### Community 14 - "frames.rs"
Cohesion: 0.10
Nodes (43): assert_id_frame(), gray_frame_number(), id_video(), t2997(), t30(), assert_still(), backwards(), ff() (+35 more)

### Community 15 - "drags.rs"
Cohesion: 0.21
Nodes (44): assert_drag_noop(), body(), check(), click(), dp(), drag_header_and_nothing_do_nothing(), drag_missing_sequence_is_noop(), empty() (+36 more)

### Community 16 - "encode.rs"
Cohesion: 0.22
Nodes (26): ffmpeg_decode_frame(), flat_rgba(), probe_audio_streams(), stereo_sine(), assert_video(), check_roundtrip(), COLOURS, encode() (+18 more)

### Community 17 - "timed"
Cohesion: 0.19
Nodes (23): timed(), ff(), m_det_1_missing_file_is_err(), m_det_1_no_cut_is_empty(), m_det_1_no_silence_is_empty(), m_det_1_scene_cut_red_to_blue(), m_det_1_silence_between_tones(), m_det_1_silence_shorter_than_min_duration_is_ignored() (+15 more)

### Community 18 - "decode.rs"
Cohesion: 0.17
Nodes (13): CACHE_BYTES, decode_still(), FileDec, frame_len(), Frames, Key, Lru, lru_evicts_least_recent_by_bytes() (+5 more)

### Community 19 - "session_api.rs"
Cohesion: 0.08
Nodes (27): call_err(), h_5_audio_levels_start_not_before_end_is_error(), assert_same_pixels(), bridge_setup(), FakeBridge, HOLD, pattern(), pattern_over_red() (+19 more)

### Community 20 - "Tools"
Cohesion: 0.15
Nodes (12): adit MCP contract (SPEC Part 4), Binary, Editing, Effects, Export, General rules, Library surface (`adit_mcp`), Magic-wand requests (+4 more)

### Community 21 - "assert_px"
Cohesion: 0.09
Nodes (62): assert_px(), assert_size(), render(), busy_project(), centroid(), green(), lit(), small() (+54 more)

### Community 22 - "effects_runtime.rs"
Cohesion: 0.09
Nodes (29): keyed(), param(), try_render(), e1_ctx_size_is_image_size(), e1_helpers_and_constants_allowed(), e1_output_is_straight_alpha(), e1_params_declared_in_order_and_overridden_by_instance(), e1_src_is_transparent_outside_0_1() (+21 more)

### Community 23 - "src/encode.rs"
Cohesion: 0.13
Nodes (14): Inner, start(), TempFile, unsupported(), write_temp(), FfmpegMissing, bin(), command() (+6 more)

### Community 24 - "adit-render/tests/common/mod.rs"
Cohesion: 0.10
Nodes (22): Renderer, AudioCall, AudioFn, BLUE, const_audio(), F30, Fake, FPS30 (+14 more)

### Community 25 - "clips.rs"
Cohesion: 0.05
Nodes (86): c_add_1_defaults(), c_add_1_g5_g6_g7(), c_add_1_r7_source_bounds_and_slack(), c_add_1_r7_text_source_in_stays_as_is(), c_add_2_c_add_4_linked_overlap_on_audio_side(), c_add_2_linked_pair_id_order(), c_add_2_linked_track_locked(), c_add_5_linked_overwrite_clears_both_targets() (+78 more)

### Community 26 - "effects_builtin.rs"
Cohesion: 0.11
Nodes (32): assert_px_tol(), frame_fn(), px(), C1, check(), eb_brightness_contrast(), eb_chroma_key(), eb_drop_shadow() (+24 more)

### Community 27 - "apply"
Cohesion: 0.13
Nodes (35): ClipPart, Body, EndEdge, StartEdge, Drag, apply(), gesture(), body_drag() (+27 more)

### Community 28 - "Frame"
Cohesion: 0.22
Nodes (23): Frame, quadrants(), solid_frame(), check(), G, green(), K, place() (+15 more)

### Community 29 - "adit-media/tests/common/mod.rs"
Cohesion: 0.22
Nodes (21): av_mp4(), cut_mp4(), ffmpeg(), ffprobe(), gray_h264(), nocut_mp4(), probe_video_stream(), ramp_mono_wav() (+13 more)

### Community 30 - "pattern.rs"
Cohesion: 0.29
Nodes (6): assert_pattern(), m_pat_audio_values_and_length(), m_pat_probe(), m_pat_video_frame_30000_1001(), m_pat_video_frame_30fps(), pat()

### Community 32 - "text.rs"
Cohesion: 0.20
Nodes (3): candidates(), font_dirs(), Fonts

### Community 33 - "adit-media/tests/audio.rs"
Cohesion: 0.35
Nodes (16): assert_len(), assert_ramp(), audio_missing_file_is_not_found(), ff(), m_audio_1_from_aac_in_mp4(), m_audio_1_length_is_rounded_duration(), m_audio_1_mono_duplicated_to_both_channels(), m_audio_1_range_past_end_yields_fewer_samples() (+8 more)

### Community 34 - "proto.rs"
Cohesion: 0.15
Nodes (12): call_tool(), constant_time_eq(), err(), handle(), header(), http_request(), INSTRUCTIONS, is_blocking() (+4 more)

### Community 35 - "new_project"
Cohesion: 0.14
Nodes (26): edit_err(), full(), new_project(), Proj, BUILTINS, effects(), t2_core_error_text_is_passed_through(), t2_unknown_tool_and_bad_params_are_errors() (+18 more)

### Community 36 - "adit-render/tests/export.rs"
Cohesion: 0.26
Nodes (19): assert_rgb(), audio_fake(), decode(), out_path(), probe_audio(), probe_video(), red_blue(), run_export() (+11 more)

### Community 37 - "adit-render/tests/hardening.rs"
Cohesion: 0.12
Nodes (23): COUNTER, validate_effect(), eb_every_builtin_validates(), e4_valid_definitions_pass(), BODY, brackets(), deep_parens(), DIRS (+15 more)

### Community 38 - "timeline.rs"
Cohesion: 0.12
Nodes (15): timecode(), App, EDGE, HEADER, hit_time(), hit_track(), RULER, tick_step() (+7 more)

### Community 39 - "render_cli.rs"
Cohesion: 0.21
Nodes (15): assert_rgb(), ffmpeg_frame(), ffprobe(), Probe, rgb_at(), run_ok(), binary_bad_usage_exit_2(), binary_render_default_sequence_h264() (+7 more)

### Community 40 - "Mcp"
Cohesion: 0.29
Nodes (4): Mcp, new_project(), parse_tool_result(), ToolResult

### Community 41 - "RenderError"
Cohesion: 0.11
Nodes (13): CtxVals, FORMAT, Gpu, gpu_err(), new_texture(), Scope, RenderError, Cancelled (+5 more)

### Community 42 - "edits.rs"
Cohesion: 0.14
Nodes (50): clips_on(), edit(), add(), after_insert(), AUD, audio_cmds(), AV10, check() (+42 more)

### Community 44 - "duration"
Cohesion: 0.42
Nodes (7): free_port(), serve(), binary_serve_banner_and_bearer_auth(), binary_serve_default_port_7821(), binary_serve_loopback_only(), binary_serve_random_token_per_launch(), init_params()

### Community 45 - "playback.rs"
Cohesion: 0.12
Nodes (11): Audio, Buffer, build(), lock(), mixer(), Msg, Play, Stop (+3 more)

### Community 46 - ".render_seq"
Cohesion: 0.26
Nodes (5): blend_index(), param(), Renderer, secs(), window()

### Community 47 - "adit-gui/tests/common/mod.rs"
Cohesion: 0.09
Nodes (22): Clip, A1, A2, ALL_ACTIONS, B, clip(), F, FPS30 (+14 more)

### Community 49 - "INDEX.md"
Cohesion: 0.17
Nodes (6): egui 0.37 / kittest / cpal notes for the GUI, ffmpeg subprocess gotchas, Knowledge index, MCP layer design notes, Mutation testing adit-core, wgpu 30 / naga headless notes

### Community 50 - "document.rs"
Cohesion: 0.05
Nodes (50): time_to_frame(), new_clip(), Error, InUse, InvalidArgument, KindMismatch, NestCycle, NoAdjacentClip (+42 more)

### Community 51 - "EffectDef"
Cohesion: 0.22
Nodes (12): EffectDef, ParamDef, BLEND, builtin_effects(), BUILTINS, internal(), MIX, SCALE (+4 more)

### Community 52 - "effects.rs"
Cohesion: 0.09
Nodes (24): EffectKind, Filter, Transition, assemble(), assemble_parts(), check_param_names(), COMPILER_STACK, entry() (+16 more)

### Community 53 - "MediaBackend"
Cohesion: 0.16
Nodes (13): MediaBackend, atempo_steps_multiply_back(), fetch(), filter_chain(), frame_at(), frame_count(), mix_audio(), mix_seq() (+5 more)

### Community 54 - "adit-core/tests/hardening.rs"
Cohesion: 0.07
Nodes (61): c_add_1_default_names(), c_add_1_i4_kind_mismatch(), c_add_1_nest_cycle_direct_and_indirect(), c_add_1_r7_stills_and_sequences_unbounded(), c_add_1_sequence_on_audio_track_ok(), c_add_2_kind_mismatch_cases(), c_add_2_linked_sequence_source(), add() (+53 more)

### Community 55 - "adit-core/tests/common/mod.rs"
Cohesion: 0.11
Nodes (34): frame_to_time(), Keyframe, approx(), assert_approx(), kf(), assert_err!, media_src(), param() (+26 more)

### Community 59 - "main.rs"
Cohesion: 0.42
Nodes (10): backend(), export(), fail(), gui(), main(), render(), run_server(), token() (+2 more)

### Community 60 - "session"
Cohesion: 0.16
Nodes (38): add_solid(), call(), depths(), edit(), import_pattern(), next_id(), project(), same_path() (+30 more)

### Community 61 - "adit-mcp/tests/common/mod.rs"
Cohesion: 0.32
Nodes (12): approx(), assert_px(), decode(), frame_png(), t_edit_define_then_use_in_same_batch_renders_e5(), red_then_blue(), t_frame_formats_magic_and_colour(), t_frame_media_pattern_pixels() (+4 more)

### Community 62 - "adit-mcp/tests/hardening.rs"
Cohesion: 0.24
Nodes (12): add_seq(), cut_mp4(), ffmpeg_session(), gap_wav(), h_13_fifth_concurrent_export_is_error(), h_14_detect_with_ffmpeg_session_works(), h_14_detect_with_pattern_session_is_error(), h_4_edit_nesting_depth_16_ok_17_rejected() (+4 more)

### Community 63 - "monitors.rs"
Cohesion: 0.08
Nodes (9): guarded(), Monitors, note(), Picture, picture(), show(), Checker, Worker (+1 more)

### Community 64 - "Session"
Cohesion: 0.27
Nodes (9): Session, alive(), h_11_contact_sheet_over_64_megapixels_is_error(), h_1_define_effect_deep_braces_is_error(), h_1_effect_check_deep_braces_is_not_ok(), h_5_audio_levels_huge_end_is_clamped(), h_5_audio_levels_start_past_clamped_end_is_error(), one_second_of_pattern_audio() (+1 more)

### Community 65 - "ui"
Cohesion: 0.18
Nodes (27): mark_project(), match_project(), only(), select_project(), step(), two_sec(), u_jkl_shuttle_left_doubles_and_caps(), u_jkl_shuttle_right_doubles_and_caps() (+19 more)

### Community 66 - "add_solid"
Cohesion: 0.28
Nodes (13): assert_rgb(), ffmpeg_frame(), probe_video(), rgb_at(), long_project(), t_export_done_and_file_matches(), t_export_errors(), t_export_options_width_height_fps() (+5 more)

### Community 67 - "geometry.rs"
Cohesion: 0.14
Nodes (27): at_clip(), G, hit_project(), hl(), layout(), PX, snap_project(), snap_ui() (+19 more)

### Community 68 - "requests.rs"
Cohesion: 0.22
Nodes (11): request_list(), create(), t_req_1_2_create_and_wait_returns_immediately(), t_req_2_each_request_handed_out_once(), t_req_2_oldest_first(), t_req_2_timeout_returns_null(), t_req_2_wakes_promptly_when_created_from_another_thread(), t_req_3_list_and_complete() (+3 more)

### Community 70 - "logic.rs"
Cohesion: 0.09
Nodes (51): Sequence, Marker, Sequence, Time, S, layout(), preview(), drag() (+43 more)

### Community 71 - "CLAUDE.md"
Cohesion: 0.22
Nodes (7): Contracts and test-first rule, Default working style: caveman + ponytail, graphify (code knowledge graph), Knowledge base (always maintain), One task per session, Project, Token budget: query before reading

### Community 72 - "adit GUI contract (SPEC Part 6)"
Cohesion: 0.18
Nodes (10): Actions (`perform`), adit GUI contract (SPEC Part 6), Binary, Keys, Rules added after the GUI review (U-FIX-*), Session additions (`adit_mcp::Session`), State, Timeline geometry (+2 more)

### Community 73 - "fix_app.rs"
Cohesion: 0.06
Nodes (100): Options, active(), add_named(), call(), call_stepping(), clip_ids(), edit(), F30 (+92 more)

### Community 76 - "history.rs"
Cohesion: 0.16
Nodes (21): c_add_2_unlinked_has_no_link(), c_add_4_place_overlap_and_touching(), add_m(), add_marker(), c_batch_1_empty_batch_is_ok_noop(), c_batch_1_first_error_fails_all(), c_batch_1_runs_in_order_one_undo_entry(), doc_with_clip() (+13 more)

### Community 77 - "src/app.rs"
Cohesion: 0.29
Nodes (7): existing_selection(), frame(), new_tab(), refresh(), S, Tab, ProjectInfo

### Community 78 - "RenderError"
Cohesion: 0.20
Nodes (3): export(), MAX_EXPORT, temp_path()

### Community 79 - "act"
Cohesion: 0.13
Nodes (28): act(), assert_noop(), ui(), link_project(), u_addedit_no_selection_nothing_to_split(), u_addedit_selection_boundary_is_not_strict(), u_del_empty_selection_no_op(), u_enable_empty_no_op() (+20 more)

### Community 80 - "Id"
Cohesion: 0.11
Nodes (17): Id, all_clip_ids(), clip(), find_clip(), Fx, mk_clip(), clip_ids(), effect_ids() (+9 more)

### Community 81 - "panels.rs"
Cohesion: 0.11
Nodes (10): frame(), ParamRow, set_clip(), TOOLS, TextBuf, TextField, AudioFilter, Content (+2 more)

### Community 82 - "Tool"
Cohesion: 0.08
Nodes (26): Bridge, lock(), Mirror, png(), Shared, state_json(), switch_sequence(), time() (+18 more)

### Community 83 - "adit-render/tests/hardening_crash.rs"
Cohesion: 0.21
Nodes (8): audio_project(), backend(), braces(), filter(), h_1_brace_depth_64_ok(), h_1_brace_depth_65_rejected(), h_5_mix_audio_duration_above_max_time_rejected(), h_5_mix_audio_duration_i64_max_rejected()

### Community 84 - "trk"
Cohesion: 0.15
Nodes (29): ParamTarget, c_rem_1_g8_prunes_transitions_of_removed_clip(), pt(), pte(), trk(), add_fx(), add_tr(), c_eff_1_add_effect() (+21 more)

### Community 85 - "stdio.rs"
Cohesion: 0.24
Nodes (3): has_timecode(), num_with_suffix(), t1_initialize_reports_adit_tools_and_instructions()

### Community 86 - "red"
Cohesion: 0.18
Nodes (11): audio_project(), Dir, h_12_cancelled_export_keeps_existing_file(), h_12_failing_export_keeps_existing_file(), h_12_successful_export_replaces_existing_file(), h_5_export_longer_than_4_hours_rejected(), h_5_mix_audio_negative_duration_rejected(), h_5_mix_audio_ordinary_duration_still_works() (+3 more)

### Community 89 - "Project"
Cohesion: 0.47
Nodes (4): nav(), nav_project(), u_editnav_next_and_prev(), u_editnav_none_leaves_playhead_but_stops()

### Community 90 - "duration"
Cohesion: 0.19
Nodes (9): bin(), Media, Ffmpeg, Pattern, run_adit(), binary_bare_adit_still_usage_error(), binary_gui_bad_arguments_exit_2_with_usage(), u_fix_13_gui_port_0_exits_2_with_usage() (+1 more)

### Community 93 - "adit/tests/common/mod.rs"
Cohesion: 0.23
Nodes (7): drain_stderr(), Proc, S, Served, solid(), SWAP_WGSL, TOOL_NAMES

### Community 94 - "adit-mcp/tests/common/mod.rs"
Cohesion: 0.26
Nodes (11): f(), F30, find_clip(), find_track(), media_src(), S, solid(), summary() (+3 more)

### Community 95 - "e2e_story_real_ffmpeg"
Cohesion: 0.19
Nodes (11): approx(), color_clip(), cut_and_silence_clip(), f(), px(), same_path(), clip(), e2e_detect_silence_and_scenes() (+3 more)

### Community 96 - "ExportSettings"
Cohesion: 0.20
Nodes (10): AudioCodec, Aac, None, Pcm, ExportSettings, VideoCodec, Gif, H264 (+2 more)

### Community 97 - "src/probe.rs"
Cohesion: 0.49
Nodes (8): check_file(), flicks(), parse(), parses_streams_and_skips_cover_art(), probe(), rational(), secs(), to_flicks()

### Community 98 - "Track"
Cohesion: 0.11
Nodes (13): check_param(), Clip, finite(), Param, Project, edges_unique(), end_ok(), new_track() (+5 more)

### Community 99 - "KeyChord"
Cohesion: 0.31
Nodes (7): default_keymap(), KeyChord, default_keymap(), k(), spec_keymap(), u_keys_keymap_is_exactly_the_table(), u_keys_spec_table_is_well_formed()

### Community 100 - "Dialog"
Cohesion: 0.25
Nodes (6): Dialog, CloseProject, ConfirmClose, Gain, Speed, Wand

### Community 101 - "PlaceMode"
Cohesion: 0.33
Nodes (5): PlaceMode, Insert, Overwrite, Place, AddB

## Knowledge Gaps
- **208 isolated node(s):** `Speed`, `Gain`, `Wand`, `ConfirmClose`, `CloseProject` (+203 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 493 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **17 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Id` connect `Id` to `Command`, `Project`, `base`, `src/tools.rs`, `trims.rs`, `validate.rs`, `clips.rs`, `apply`, `timeline.rs`, `RenderError`, `App`, `playback.rs`, `document.rs`, `adit-core/tests/hardening.rs`, `adit-core/tests/common/mod.rs`, `monitors.rs`, `logic.rs`, `fix_app.rs`, `history.rs`, `src/app.rs`, `panels.rs`, `Tool`, `trk`, `Track`, `Dialog`, `PlaceMode`, `adit-render/src/lib.rs`?**
  _High betweenness centrality (0.139) - this node is a cross-community bridge._
- **Are the 97 inferred relationships involving `Id` (e.g. with `alloc()` and `clip_ix()`) actually correct?**
  _`Id` has 97 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Speed`, `Gain`, `Wand` to the rest of the system?**
  _208 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Command` be split into smaller, more focused modules?**
  _Cohesion score 0.03696741854636591 - nodes in this community are weakly interconnected._
- **Why does `Time` connect `logic.rs` to `Command`, `adit-media/src/lib.rs`, `Project`, `time.rs`, `base`, `MediaError`, `src/tools.rs`, `trims.rs`, `validate.rs`, `decode.rs`, `session_api.rs`, `clips.rs`, `apply`, `timeline.rs`, `RenderError`, `App`, `playback.rs`, `document.rs`, `MediaBackend`, `adit-core/tests/hardening.rs`, `adit-core/tests/common/mod.rs`, `monitors.rs`, `fix_app.rs`, `history.rs`, `src/app.rs`, `Id`, `panels.rs`, `Tool`, `trk`, `src/probe.rs`, `Track`, `Dialog`, `PlaceMode`, `adit-render/src/lib.rs`?**
  _High betweenness centrality (0.105) - this node is a cross-community bridge._
- **Are the 163 inferred relationships involving `base()` (e.g. with `c_add_1_default_names()` and `c_add_1_defaults()`) actually correct?**
  _`base()` has 163 INFERRED edges - model-reasoned connections that need verification._
- **Should `Action` be split into smaller, more focused modules?**
  _Cohesion score 0.03636363636363636 - nodes in this community are weakly interconnected._