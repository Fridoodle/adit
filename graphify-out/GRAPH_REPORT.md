# Graph Report - adit  (2026-11-25)

## Corpus Check
- 118 files · ~197,465 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 3 file(s) not represented in the graph (top: (none) 2, .proptest-regressions 1)

## Summary
- 3246 nodes · 10803 edges · 128 communities (101 shown, 27 thin omitted)
- Extraction: 72% EXTRACTED · 28% INFERRED · 0% AMBIGUOUS · INFERRED: 3005 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `3cc53371`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Command
- Action
- adit-media/src/lib.rs
- Project
- Part 1 — adit-core
- ui
- base
- MediaError
- R
- adit-render/tests/audio.rs
- adit-media/tests/hardening.rs
- trim
- validate.rs
- adit-core
- frames.rs
- adit-core/tests/common/mod.rs
- encode.rs
- timed
- decode.rs
- session_api.rs
- Tools
- assert_px
- effects_runtime.rs
- Session
- adit-render/tests/common/mod.rs
- clips.rs
- effects_builtin.rs
- kf
- Frame
- adit-media/tests/common/mod.rs
- src/probe.rs
- ed_fix.rs
- text.rs
- adit-media/tests/audio.rs
- proto.rs
- session
- adit-render/tests/export.rs
- adit-render/tests/hardening.rs
- timeline.rs
- render_cli.rs
- ed_app.rs
- RenderError
- edits.rs
- fix_app.rs
- paste_label.rs
- playback.rs
- .render_seq
- adit-gui/tests/common/mod.rs
- u_ed_16_drop_over_a_track_places_the_media_end_to_end
- INDEX.md
- Time
- EffectDef
- effects.rs
- MediaBackend
- adit-core/tests/hardening.rs
- params.rs
- B
- src/tools.rs
- main.rs
- session_ed.rs
- adit-mcp/tests/common/mod.rs
- adit-mcp/tests/hardening.rs
- App
- Mcp
- transport.rs
- transition_at
- geometry.rs
- panels.rs
- adit
- UiState
- CLAUDE.md
- adit GUI contract (SPEC Part 6)
- harness
- Rational
- history.rs
- mk_clip
- RenderError
- R
- Id
- Error
- Tool
- adit-render/tests/hardening_crash.rs
- e2e_story_real_ffmpeg
- stdio.rs
- red
- pattern.rs
- TimelineLayout
- App
- library.rs
- serve
- harness
- ui
- adit-mcp/tests/hardening_crash.rs
- adit/tests/common/mod.rs
- TrackKind
- Inner
- requests.rs
- mix
- src/app.rs
- src/encode.rs
- Http
- state_keys.rs
- ExportSettings
- Dialog

## God Nodes (most connected - your core abstractions)
1. `base()` - 221 edges
2. `Id` - 207 edges
3. `Time` - 172 edges
4. `ui()` - 132 edges
5. `Command` - 111 edges
6. `Project` - 111 edges
7. `App` - 103 edges
8. `harness()` - 94 edges
9. `MediaError` - 91 edges
10. `Action` - 90 edges

## Surprising Connections (you probably didn't know these)
- `ALL_ACTIONS` --references--> `Action`  [INFERRED]
  crates/adit-gui/tests/common/mod.rs → crates/adit-gui/src/lib.rs
- `S` --references--> `Time`  [INFERRED]
  crates/adit-core/tests/common/mod.rs → crates/adit-core/src/model.rs
- `MT` --references--> `Time`  [INFERRED]
  crates/adit-core/tests/hardening.rs → crates/adit-core/src/model.rs
- `filter()` --references--> `B`  [INFERRED]
  crates/adit-render/tests/effects_runtime.rs → crates/adit-render/tests/common/mod.rs
- `c_slide_1_needs_previous_clip()` --calls--> `base()`  [INFERRED]
  crates/adit-core/tests/trims.rs → crates/adit-core/tests/common/mod.rs

## Import Cycles
- 1-file cycle: `crates/adit-media/src/proc.rs -> crates/adit-media/src/proc.rs`
- 1-file cycle: `crates/adit-render/src/effects.rs -> crates/adit-render/src/effects.rs`
- 2-file cycle: `crates/adit-gui/src/app.rs -> crates/adit-gui/src/lib.rs -> crates/adit-gui/src/app.rs`
- 2-file cycle: `crates/adit-mcp/src/lib.rs -> crates/adit-mcp/src/proto.rs -> crates/adit-mcp/src/lib.rs`
- 2-file cycle: `crates/adit-media/src/lib.rs -> crates/adit-media/src/proc.rs -> crates/adit-media/src/lib.rs`

## Communities (128 total, 27 thin omitted)

### Community 0 - "Command"
Cohesion: 0.04
Nodes (51): Command, AddClip, AddEffect, AddMarker, AddMedia, AddSequence, AddTrack, AddTransition (+43 more)

### Community 1 - "Action"
Cohesion: 0.03
Nodes (61): Action, AddEdit, AddEditAll, AddMarker, ApplyAudioTransition, ApplyDefaultTransitions, ApplyVideoTransition, ClearIn (+53 more)

### Community 2 - "adit-media/src/lib.rs"
Cohesion: 0.18
Nodes (12): AUDIO_FILTERS, Ffmpeg, lock(), Unsupported, number_after(), parse_scenes(), parse_silence(), seconds() (+4 more)

### Community 3 - "Project"
Cohesion: 0.17
Nodes (47): alloc(), bad(), bounds_ok(), check_locked(), clear(), clip_ix(), clip_mut(), clip_ref() (+39 more)

### Community 4 - "Part 1 — adit-core"
Cohesion: 0.11
Nodes (18): adit behavioural spec, Audio, Batch, Clips, Commands, Effects, Export, General rules (+10 more)

### Community 5 - "ui"
Cohesion: 0.11
Nodes (50): preview(), snap_delta(), acts(), clip_hit(), close_gap(), drag_of(), gap_project(), menu() (+42 more)

### Community 6 - "base"
Cohesion: 0.08
Nodes (48): c_add_6_r1_insert_splits_and_shifts_all_unlocked_tracks(), c_ext_1_all_unlocked_tracks(), c_ext_1_duration_when_range_passes_the_end(), all_clip_ids(), base(), sq(), h_3_add_media_duration_bound(), c_paste_1_g5_locked_track_is_track_locked() (+40 more)

### Community 7 - "MediaError"
Cohesion: 0.10
Nodes (10): MissingMedia, Offline, Steps, Encoder, MediaBackend, MediaError, Ffmpeg, Io (+2 more)

### Community 8 - "R"
Cohesion: 0.23
Nodes (31): ToolOutput, A, absolutize(), bridge(), effect_check(), effect_list(), entry(), find_job() (+23 more)

### Community 9 - "adit-render/tests/audio.rs"
Cohesion: 0.20
Nodes (33): a1_count_uses_sequence_sample_rate(), a1_empty_space_is_silence(), a1_exact_sample_counts(), a1_mix_window_offset(), a1_mute_solo_disabled(), a1_only_audio_tracks_contribute(), a2_atempo_chained_within_ffmpeg_range(), a2_filter_chain_order_and_atempo() (+25 more)

### Community 10 - "adit-media/tests/hardening.rs"
Cohesion: 0.07
Nodes (46): check_audio_filter(), gray_frame_number(), TempDir, ALLOWED, ametadata_to(), ff(), FORBIDDEN, frame_to_time() (+38 more)

### Community 11 - "trim"
Cohesion: 0.08
Nodes (60): Edge, End, Start, TrimMode, Normal, Ripple, Roll, trim() (+52 more)

### Community 12 - "validate.rs"
Cohesion: 0.13
Nodes (34): i1_negative_start_rejected(), i1_overlap_rejected(), i1_touching_clips_are_fine_half_open(), i1_unsorted_rejected(), i1_zero_and_negative_duration_rejected(), i2_duplicate_clip_ids_rejected(), i2_id_not_below_next_id_rejected(), i2_ids_unique_across_entity_kinds() (+26 more)

### Community 13 - "adit-core"
Cohesion: 0.93
Nodes (6): adit, adit-core, adit-gui, adit-mcp, adit-media, adit-render

### Community 14 - "frames.rs"
Cohesion: 0.11
Nodes (42): assert_id_frame(), id_video(), t2997(), t30(), assert_still(), backwards(), ff(), Fixture (+34 more)

### Community 15 - "adit-core/tests/common/mod.rs"
Cohesion: 0.10
Nodes (36): c_add_1_default_names(), c_add_1_defaults(), c_add_1_i4_kind_mismatch(), c_add_1_nest_cycle_direct_and_indirect(), c_add_1_r7_source_bounds_and_slack(), c_add_1_r7_stills_and_sequences_unbounded(), c_add_1_r7_text_source_in_stays_as_is(), c_add_1_sequence_on_audio_track_ok() (+28 more)

### Community 16 - "encode.rs"
Cohesion: 0.22
Nodes (26): ffmpeg_decode_frame(), flat_rgba(), probe_audio_streams(), stereo_sine(), assert_video(), check_roundtrip(), COLOURS, encode() (+18 more)

### Community 17 - "timed"
Cohesion: 0.19
Nodes (23): timed(), ff(), m_det_1_missing_file_is_err(), m_det_1_no_cut_is_empty(), m_det_1_no_silence_is_empty(), m_det_1_scene_cut_red_to_blue(), m_det_1_silence_between_tones(), m_det_1_silence_shorter_than_min_duration_is_ignored() (+15 more)

### Community 18 - "decode.rs"
Cohesion: 0.16
Nodes (13): CACHE_BYTES, decode_still(), FileDec, frame_len(), Frames, Key, Lru, lru_evicts_least_recent_by_bytes() (+5 more)

### Community 19 - "session_api.rs"
Cohesion: 0.07
Nodes (30): import_pattern(), h_12_export_onto_a_media_file_is_error(), assert_same_pixels(), bridge_setup(), clip_count(), FakeBridge, HOLD, pattern() (+22 more)

### Community 20 - "Tools"
Cohesion: 0.14
Nodes (13): adit MCP contract (SPEC Part 4), Binary, Editing, Effect library, Effects, Export, General rules, Library surface (`adit_mcp`) (+5 more)

### Community 21 - "assert_px"
Cohesion: 0.09
Nodes (62): assert_px(), assert_size(), render(), busy_project(), centroid(), green(), lit(), small() (+54 more)

### Community 22 - "effects_runtime.rs"
Cohesion: 0.10
Nodes (27): param(), try_render(), e1_ctx_size_is_image_size(), e1_helpers_and_constants_allowed(), e1_output_is_straight_alpha(), e1_params_declared_in_order_and_overridden_by_instance(), e1_src_is_transparent_outside_0_1(), e1_uv_origin_top_left() (+19 more)

### Community 23 - "Session"
Cohesion: 0.19
Nodes (24): Session, Proj, project(), summary(), h_9_remove_unused_effect_def_ok(), with_swap(), clip_count(), define() (+16 more)

### Community 24 - "adit-render/tests/common/mod.rs"
Cohesion: 0.08
Nodes (21): AudioCall, AudioFn, BLUE, const_audio(), def(), F30, Fake, FPS30 (+13 more)

### Community 25 - "clips.rs"
Cohesion: 0.06
Nodes (71): c_add_1_g5_g6_g7(), c_add_2_c_add_4_linked_overlap_on_audio_side(), c_add_2_linked_track_locked(), c_add_5_overwrite_clears_range_on_target_only(), c_add_5_overwrite_exact_fit_and_boundaries(), c_add_5_overwrite_locked_target(), c_add_6_insert_on_locked_target(), c_ext_1_errors() (+63 more)

### Community 26 - "effects_builtin.rs"
Cohesion: 0.11
Nodes (32): assert_px_tol(), frame_fn(), px(), C1, check(), eb_brightness_contrast(), eb_chroma_key(), eb_drop_shadow() (+24 more)

### Community 27 - "kf"
Cohesion: 0.11
Nodes (31): frame_to_time(), c_lift_1_r2_head_trim_moves_keyframes(), c_split_1_r4_right_part_copies_and_new_effect_ids(), assert_approx(), kf(), param(), text_spec(), rich_project() (+23 more)

### Community 28 - "Frame"
Cohesion: 0.22
Nodes (23): Frame, quadrants(), solid_frame(), check(), G, green(), K, place() (+15 more)

### Community 29 - "adit-media/tests/common/mod.rs"
Cohesion: 0.22
Nodes (21): av_mp4(), cut_mp4(), ffmpeg(), ffprobe(), gray_h264(), nocut_mp4(), probe_video_stream(), ramp_mono_wav() (+13 more)

### Community 30 - "src/probe.rs"
Cohesion: 0.47
Nodes (8): check_file(), flicks(), parse(), parses_streams_and_skips_cover_art(), probe(), rational(), secs(), to_flicks()

### Community 31 - "ed_fix.rs"
Cohesion: 0.22
Nodes (10): copy_in_a_paste_in_b(), event_with_modifiers(), spans(), two_adjacent(), two_projects(), u_ed_23a_cut_event_without_shift_still_cuts(), u_ed_23a_shift_cut_event_ripple_deletes_and_leaves_clipboard_empty(), u_ed_23g_pasting_a_media_clip_into_another_project_adds_nothing() (+2 more)

### Community 32 - "text.rs"
Cohesion: 0.24
Nodes (3): candidates(), font_dirs(), Fonts

### Community 33 - "adit-media/tests/audio.rs"
Cohesion: 0.35
Nodes (16): assert_len(), assert_ramp(), audio_missing_file_is_not_found(), ff(), m_audio_1_from_aac_in_mp4(), m_audio_1_length_is_rounded_duration(), m_audio_1_mono_duplicated_to_both_channels(), m_audio_1_range_past_end_yields_fewer_samples() (+8 more)

### Community 34 - "proto.rs"
Cohesion: 0.15
Nodes (13): call_tool(), constant_time_eq(), err(), handle(), header(), http_request(), INSTRUCTIONS, is_blocking() (+5 more)

### Community 35 - "session"
Cohesion: 0.09
Nodes (71): add_solid(), assert_rgb(), call(), call_err(), edit(), edit_err(), ffmpeg_frame(), find_clip() (+63 more)

### Community 36 - "adit-render/tests/export.rs"
Cohesion: 0.26
Nodes (19): assert_rgb(), audio_fake(), decode(), out_path(), probe_audio(), probe_video(), red_blue(), run_export() (+11 more)

### Community 37 - "adit-render/tests/hardening.rs"
Cohesion: 0.12
Nodes (23): COUNTER, validate_effect(), eb_every_builtin_validates(), e4_valid_definitions_pass(), BODY, brackets(), deep_parens(), DIRS (+15 more)

### Community 38 - "timeline.rs"
Cohesion: 0.09
Nodes (23): App, EDGE, HEADER, hit_time(), hit_track(), LABELS, layout(), menu_text() (+15 more)

### Community 39 - "render_cli.rs"
Cohesion: 0.18
Nodes (17): assert_rgb(), ffmpeg_frame(), rgb_at(), run_adit(), binary_bare_adit_still_usage_error(), binary_gui_bad_arguments_exit_2_with_usage(), u_fix_13_gui_port_0_exits_2_with_usage(), binary_bad_usage_exit_2() (+9 more)

### Community 40 - "ed_app.rs"
Cohesion: 0.13
Nodes (46): has_label(), undo_depth(), button(), click_at(), click_label(), click_named(), drag(), gap_setup() (+38 more)

### Community 41 - "RenderError"
Cohesion: 0.13
Nodes (6): CtxVals, FORMAT, Gpu, gpu_err(), new_texture(), Scope

### Community 42 - "edits.rs"
Cohesion: 0.06
Nodes (123): Drag, apply(), assert_drag_noop(), clips_on(), edit(), gesture(), body(), check() (+115 more)

### Community 43 - "fix_app.rs"
Cohesion: 0.09
Nodes (42): add_text(), any_text(), assert_fresh(), click_at(), click_named(), content(), ctrl(), field_value() (+34 more)

### Community 44 - "paste_label.rs"
Cohesion: 0.10
Nodes (60): PasteItem, bad(), c_label_1_g3_undo_restores_label(), c_label_1_g5_locked_track_is_track_locked(), c_label_1_g7_unknown_clip_is_not_found(), c_label_1_g9_clip_with_label_round_trips(), c_label_1_label_8_ok_9_invalid_argument(), c_label_1_label_absent_in_project_json_is_0() (+52 more)

### Community 45 - "playback.rs"
Cohesion: 0.05
Nodes (18): guarded(), Monitors, note(), Picture, picture(), show(), Audio, Buffer (+10 more)

### Community 46 - ".render_seq"
Cohesion: 0.26
Nodes (5): blend_index(), param(), Renderer, secs(), assemble()

### Community 47 - "adit-gui/tests/common/mod.rs"
Cohesion: 0.09
Nodes (20): A1, A2, ALL_ACTIONS, B, clip(), F, FPS30, MEDIA (+12 more)

### Community 48 - "u_ed_16_drop_over_a_track_places_the_media_end_to_end"
Cohesion: 0.28
Nodes (5): drop_files(), Dropped, library(), u_ed_16_drop_over_a_track_places_the_media_end_to_end(), u_ed_16_dropped_files_are_imported_in_one_undo_step()

### Community 49 - "INDEX.md"
Cohesion: 0.17
Nodes (6): egui 0.37 / kittest / cpal notes for the GUI, ffmpeg subprocess gotchas, Knowledge index, MCP layer design notes, Mutation testing adit-core, wgpu 30 / naga headless notes

### Community 50 - "Time"
Cohesion: 0.06
Nodes (39): PlaceMode, Insert, Overwrite, Place, Clip, Param, Sequence, time_to_frame() (+31 more)

### Community 51 - "EffectDef"
Cohesion: 0.25
Nodes (10): BLEND, builtin_effects(), BUILTINS, internal(), MIX, SCALE, Spec, to_def() (+2 more)

### Community 52 - "effects.rs"
Cohesion: 0.10
Nodes (20): assemble_parts(), check_param_names(), COMPILER_STACK, entry(), FILTER_PRELUDE, is_comment_end(), is_type(), KEYWORDS (+12 more)

### Community 53 - "MediaBackend"
Cohesion: 0.22
Nodes (12): atempo_steps_multiply_back(), fetch(), filter_chain(), frame_at(), frame_count(), mix_audio(), mix_seq(), resample() (+4 more)

### Community 54 - "adit-core/tests/hardening.rs"
Cohesion: 0.07
Nodes (44): solid_src(), chain(), edge_fixture(), h_10_batch_prunes_after_each_command(), h_15_remove_sequence_with_locked_track_rejected(), h_15_remove_sequence_without_locked_track_ok(), h_3_add_clip_duration_above_max_time_rejected(), h_3_add_clip_duration_max_time_ok() (+36 more)

### Community 55 - "params.rs"
Cohesion: 0.14
Nodes (31): ParamTarget, c_rem_1_g8_prunes_transitions_of_removed_clip(), c_split_1_r4_transitions_follow_their_edge(), c_trans_3_moving_q_prunes_transition_and_undo_restores_it(), pt(), pte(), trk(), add_fx() (+23 more)

### Community 57 - "B"
Cohesion: 0.15
Nodes (3): B, keyed(), e2_keyframed_param_evaluated_at_local_time()

### Community 58 - "src/tools.rs"
Cohesion: 0.06
Nodes (49): EffectDef, EffectKind, Filter, Transition, ParamDef, builtins(), call(), check_effect() (+41 more)

### Community 59 - "main.rs"
Cohesion: 0.41
Nodes (12): backend(), export(), fail(), gui(), library_dir(), main(), render(), run_server() (+4 more)

### Community 60 - "session_ed.rs"
Cohesion: 0.11
Nodes (18): AudioInfo, MediaSpec, VideoInfo, Counting, Failing, pattern(), pattern_project(), s_4_computed_once_per_path_and_cached() (+10 more)

### Community 61 - "adit-mcp/tests/common/mod.rs"
Cohesion: 0.15
Nodes (24): approx(), assert_px(), decode(), f(), F30, find_track(), frame_png(), media_src() (+16 more)

### Community 62 - "adit-mcp/tests/hardening.rs"
Cohesion: 0.27
Nodes (12): add_seq(), cut_mp4(), ffmpeg(), ffmpeg_session(), gap_wav(), h_14_detect_with_ffmpeg_session_works(), h_14_detect_with_pattern_session_is_error(), h_4_edit_nesting_depth_16_ok_17_rejected() (+4 more)

### Community 63 - "App"
Cohesion: 0.21
Nodes (3): App, title(), timecode()

### Community 64 - "Mcp"
Cohesion: 0.27
Nodes (4): Mcp, new_project(), parse_tool_result(), ToolResult

### Community 65 - "transport.rs"
Cohesion: 0.15
Nodes (31): mark_project(), match_project(), nav(), nav_project(), only(), select_project(), step(), two_sec() (+23 more)

### Community 66 - "transition_at"
Cohesion: 0.30
Nodes (13): transition_at(), trans(), trans_project(), u_ed_6_duration_is_limited_by_short_clips(), u_ed_6_effect_name_is_passed_through(), u_ed_6_end_edge_with_next_clip(), u_ed_6_end_edge_without_next_clip(), u_ed_6_gap_duration_is_limited_by_short_clips() (+5 more)

### Community 67 - "geometry.rs"
Cohesion: 0.17
Nodes (25): at_clip(), G, hit_project(), hl(), layout(), PX, snap_project(), snap_ui() (+17 more)

### Community 68 - "panels.rs"
Cohesion: 0.10
Nodes (16): BlendMode, Add, Darken, Difference, Lighten, Multiply, Normal, Overlay (+8 more)

### Community 70 - "UiState"
Cohesion: 0.09
Nodes (54): Sequence, moving(), context_menu(), copy(), drag(), MenuItem, Do, Labels (+46 more)

### Community 71 - "CLAUDE.md"
Cohesion: 0.22
Nodes (7): Contracts and test-first rule, Default working style: caveman + ponytail, graphify (code knowledge graph), Knowledge base (always maintain), One task per session, Project, Token budget: query before reading

### Community 72 - "adit GUI contract (SPEC Part 6)"
Cohesion: 0.17
Nodes (11): Actions (`perform`), adit GUI contract (SPEC Part 6), Binary, Editing comfort (U-ED-*), Keys, Rules added after the GUI review (U-FIX-*), Session additions (`adit_mcp::Session`), State (+3 more)

### Community 73 - "harness"
Cohesion: 0.13
Nodes (42): active(), add_named(), call(), call_stepping(), clip_ids(), edit(), F30, H (+34 more)

### Community 74 - "Rational"
Cohesion: 0.14
Nodes (21): Rational, FPS24, ELEVEN, NTSC, frame(), convert_times(), fps_strings(), frames_to_time() (+13 more)

### Community 76 - "history.rs"
Cohesion: 0.21
Nodes (16): add_marker(), c_batch_1_empty_batch_is_ok_noop(), c_batch_1_first_error_fails_all(), c_batch_1_runs_in_order_one_undo_entry(), doc_with_clip(), g1_error_keeps_project_and_both_stacks(), g1_failing_ripple_after_partial_shift(), g2_change_pushes_one_entry_and_clears_redo() (+8 more)

### Community 77 - "mk_clip"
Cohesion: 0.19
Nodes (23): ClipboardItem, Clip, paste(), copy(), paste(), assert_paste_noop(), item(), item_of() (+15 more)

### Community 78 - "RenderError"
Cohesion: 0.12
Nodes (11): export(), MAX_EXPORT, temp_path(), Renderer, RenderError, Cancelled, Gpu, Media (+3 more)

### Community 79 - "R"
Cohesion: 0.19
Nodes (18): A<'a>, audio_levels(), contact_sheet(), detect_scenes(), detect_silence(), encode(), export(), media_path() (+10 more)

### Community 80 - "Id"
Cohesion: 0.11
Nodes (17): Id, Fx, mk_clip(), clip_ids(), effect_ids(), locked_clips(), Op, op() (+9 more)

### Community 81 - "Error"
Cohesion: 0.18
Nodes (11): Error, InUse, InvalidArgument, KindMismatch, NestCycle, NoAdjacentClip, NotFound, Overlap (+3 more)

### Community 82 - "Tool"
Cohesion: 0.07
Nodes (28): Bridge, lock(), Mirror, png(), Shared, state_json(), switch_sequence(), time() (+20 more)

### Community 83 - "adit-render/tests/hardening_crash.rs"
Cohesion: 0.19
Nodes (8): audio_project(), backend(), braces(), filter(), h_1_brace_depth_64_ok(), h_1_brace_depth_65_rejected(), h_5_mix_audio_duration_above_max_time_rejected(), h_5_mix_audio_duration_i64_max_rejected()

### Community 84 - "e2e_story_real_ffmpeg"
Cohesion: 0.21
Nodes (10): approx(), color_clip(), f(), px(), same_path(), clip(), e2e_detect_silence_and_scenes(), e2e_import_failure_adds_nothing() (+2 more)

### Community 85 - "stdio.rs"
Cohesion: 0.24
Nodes (3): has_timecode(), num_with_suffix(), t1_initialize_reports_adit_tools_and_instructions()

### Community 86 - "red"
Cohesion: 0.18
Nodes (12): renderer(), audio_project(), Dir, h_12_cancelled_export_keeps_existing_file(), h_12_failing_export_keeps_existing_file(), h_12_successful_export_replaces_existing_file(), h_5_export_longer_than_4_hours_rejected(), h_5_mix_audio_negative_duration_rejected() (+4 more)

### Community 87 - "pattern.rs"
Cohesion: 0.26
Nodes (8): assert_pattern(), m_pat_audio_values_and_length(), m_pat_probe(), m_pat_video_frame_30000_1001(), m_pat_video_frame_30fps(), pat(), R2997, R30

### Community 89 - "TimelineLayout"
Cohesion: 0.33
Nodes (5): TimelineLayout, hit(), rows(), time_to_x(), x_to_time()

### Community 95 - "library.rs"
Cohesion: 0.16
Nodes (46): depths(), swap_def(), as_def(), define(), effects(), fade_def(), FADE_WGSL, files() (+38 more)

### Community 96 - "serve"
Cohesion: 0.23
Nodes (11): bin(), free_port(), Media, Ffmpeg, Pattern, serve(), binary_serve_banner_and_bearer_auth(), binary_serve_default_port_7821() (+3 more)

### Community 98 - "harness"
Cohesion: 0.22
Nodes (19): harness(), has_text(), step_until(), u_app_1_one_tab_per_open_project(), u_app_2_missing_file_falls_back_to_untitled(), u_app_2_opens_options_open(), u_app_3_projects_opened_and_closed_over_mcp(), u_app_6_status_bar_shows_mcp_url() (+11 more)

### Community 99 - "ui"
Cohesion: 0.12
Nodes (40): act(), assert_noop(), ui(), copy_project(), label(), u_ed_1_cut_is_copy_then_delete(), u_ed_1_cut_leaves_out_locked_clips_like_delete(), u_ed_1_cut_with_empty_selection_is_no_op() (+32 more)

### Community 100 - "adit-mcp/tests/hardening_crash.rs"
Cohesion: 0.18
Nodes (8): alive(), h_1_define_effect_deep_braces_is_error(), h_1_effect_check_deep_braces_is_not_ok(), h_5_audio_levels_huge_end_is_clamped(), h_5_audio_levels_start_past_clamped_end_is_error(), one_second_of_pattern_audio(), timed(), TOOLS

### Community 101 - "adit/tests/common/mod.rs"
Cohesion: 0.16
Nodes (11): cut_and_silence_clip(), drain_stderr(), ffprobe(), Probe, Proc, run_ok(), S, Served (+3 more)

### Community 102 - "TrackKind"
Cohesion: 0.16
Nodes (10): check_param(), finite(), Project, edges_unique(), new_track(), Track, TrackKind, Audio (+2 more)

### Community 103 - "Inner"
Cohesion: 0.21
Nodes (6): Image, ProjectInfo, ToolInfo, UiBridge, projects(), set_ui()

### Community 105 - "requests.rs"
Cohesion: 0.22
Nodes (16): request_list(), t_req_4_defaults_are_edit_null_panel_and_no_paths(), t_req_4_every_request_in_the_list_has_the_fields(), t_req_4_explicit_scope_edit_with_panel(), t_req_4_scope_app_has_source_dir_and_window_rect(), t_req_4_scope_import_carries_paths(), t_req_4_unknown_scope_is_error_and_files_nothing(), wait_request() (+8 more)

### Community 106 - "mix"
Cohesion: 0.24
Nodes (4): mix(), render_raw(), snapshot(), source_frame()

### Community 107 - "src/app.rs"
Cohesion: 0.15
Nodes (13): existing_selection(), new_tab(), run(), S, Tab, TextBuf, TextField, AudioFilter (+5 more)

### Community 109 - "src/encode.rs"
Cohesion: 0.13
Nodes (14): Inner, start(), TempFile, unsupported(), write_temp(), FfmpegMissing, bin(), command() (+6 more)

### Community 118 - "state_keys.rs"
Cohesion: 0.12
Nodes (21): default_keymap(), KeyChord, default_keymap(), u_ed_1_keymap_has_copy_cut_paste_and_no_duplicate_chord(), busy_project(), busy_ui(), empty_sequence_every_action(), k() (+13 more)

### Community 119 - "ExportSettings"
Cohesion: 0.20
Nodes (10): AudioCodec, Aac, None, Pcm, ExportSettings, VideoCodec, Gif, H264 (+2 more)

### Community 120 - "Dialog"
Cohesion: 0.25
Nodes (7): Dialog, AiImport, CloseProject, ConfirmClose, Gain, Speed, Wand

## Knowledge Gaps
- **220 isolated node(s):** `Speed`, `Gain`, `Wand`, `AiImport`, `ConfirmClose` (+215 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 543 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **27 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Id` connect `Id` to `Command`, `Action`, `Project`, `ui`, `base`, `trim`, `validate.rs`, `adit-core/tests/common/mod.rs`, `clips.rs`, `ed_fix.rs`, `timeline.rs`, `ed_app.rs`, `edits.rs`, `fix_app.rs`, `paste_label.rs`, `playback.rs`, `Time`, `adit-core/tests/hardening.rs`, `params.rs`, `src/tools.rs`, `App`, `transition_at`, `panels.rs`, `UiState`, `Rational`, `history.rs`, `mk_clip`, `RenderError`, `R`, `Error`, `Tool`, `TimelineLayout`, `App`, `TrackKind`, `mix`, `src/app.rs`, `Dialog`?**
  _High betweenness centrality (0.143) - this node is a cross-community bridge._
- **Are the 215 inferred relationships involving `base()` (e.g. with `c_add_1_default_names()` and `c_add_1_defaults()`) actually correct?**
  _`base()` has 215 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Speed`, `Gain`, `Wand` to the rest of the system?**
  _220 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Command` be split into smaller, more focused modules?**
  _Cohesion score 0.03559322033898305 - nodes in this community are weakly interconnected._
- **Why does `Time` connect `Time` to `Command`, `Action`, `adit-media/src/lib.rs`, `Project`, `ui`, `base`, `MediaError`, `trim`, `validate.rs`, `adit-core/tests/common/mod.rs`, `decode.rs`, `session_api.rs`, `clips.rs`, `kf`, `src/probe.rs`, `ed_fix.rs`, `timeline.rs`, `ed_app.rs`, `edits.rs`, `fix_app.rs`, `paste_label.rs`, `playback.rs`, `adit-core/tests/hardening.rs`, `params.rs`, `src/tools.rs`, `session_ed.rs`, `App`, `transition_at`, `panels.rs`, `UiState`, `harness`, `Rational`, `history.rs`, `RenderError`, `R`, `Id`, `Tool`, `TimelineLayout`, `App`, `TrackKind`, `mix`, `src/app.rs`, `Dialog`?**
  _High betweenness centrality (0.103) - this node is a cross-community bridge._
- **Are the 100 inferred relationships involving `Id` (e.g. with `alloc()` and `clip_ix()`) actually correct?**
  _`Id` has 100 INFERRED edges - model-reasoned connections that need verification._
- **Should `Action` be split into smaller, more focused modules?**
  _Cohesion score 0.03278688524590164 - nodes in this community are weakly interconnected._