# Graph Report - adit  (2026-11-25)

## Corpus Check
- 129 files · ~210,424 words
- Verdict: corpus is large enough that graph structure adds value.
- Unclassified: 3 file(s) not represented in the graph (top: (none) 2, .proptest-regressions 1)

## Summary
- 3442 nodes · 11491 edges · 136 communities (104 shown, 32 thin omitted)
- Extraction: 73% EXTRACTED · 27% INFERRED · 0% AMBIGUOUS · INFERRED: 3130 edges (avg confidence: 0.81)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `c0c89e8b`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Command
- Action
- MediaError
- Project
- Part 1 — adit-core
- transition_at
- base
- App
- App
- adit-render/tests/audio.rs
- adit-media/tests/hardening.rs
- trim
- validate.rs
- adit-core
- frames.rs
- common
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
- adit-core/tests/common/mod.rs
- Frame
- Document
- Harness
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
- drags.rs
- fix_app.rs
- paste_label.rs
- playback.rs
- .render_seq
- adit-gui/tests/common/mod.rs
- contact_sheet
- INDEX.md
- model.rs
- EffectDef
- effects.rs
- MediaBackend
- ui
- session_prefetch.rs
- .timeline_panel
- B
- VideoCodec
- main.rs
- session_ed.rs
- adit-mcp/tests/common/mod.rs
- common
- nav_logic.rs
- Mcp
- transport.rs
- ExportSettings
- edits.rs
- adit
- Time
- CLAUDE.md
- adit GUI contract (SPEC Part 6)
- harness
- edit
- history.rs
- Op
- Inner
- Id
- Tool
- src/encode.rs
- adit/tests/common/mod.rs
- preview-performance.md
- Rational
- .get
- src/app.rs
- library.rs
- ffmpeg
- act
- Mutex
- UiBridge
- src/tools.rs
- SPEC.md
- requests.rs
- README.md
- t30
- adit
- Http
- src/probe.rs
- panels.rs
- renderer
- props.rs
- e2e_story_real_ffmpeg
- arc
- adit-render/tests/hardening_crash.rs
- monitors.rs
- Error
- adit-media/src/lib.rs
- stdio.rs
- KeyChord
- RenderError

## God Nodes (most connected - your core abstractions)
1. `base()` - 221 edges
2. `Id` - 214 edges
3. `Time` - 192 edges
4. `ui()` - 132 edges
5. `App` - 124 edges
6. `Project` - 113 edges
7. `Command` - 111 edges
8. `harness()` - 110 edges
9. `MediaError` - 100 edges
10. `Action` - 90 edges

## Surprising Connections (you probably didn't know these)
- `ALL_ACTIONS` --references--> `Action`  [INFERRED]
  crates/adit-gui/tests/common/mod.rs → crates/adit-gui/src/lib.rs
- `filter()` --references--> `B`  [INFERRED]
  crates/adit-render/tests/effects_runtime.rs → crates/adit-render/tests/common/mod.rs
- `S` --references--> `Time`  [INFERRED]
  crates/adit-core/tests/common/mod.rs → crates/adit-core/src/model.rs
- `MT` --references--> `Time`  [INFERRED]
  crates/adit-core/tests/hardening.rs → crates/adit-core/src/model.rs
- `H` --references--> `Time`  [INFERRED]
  crates/adit-core/tests/props.rs → crates/adit-core/src/model.rs

## Import Cycles
- 1-file cycle: `crates/adit-media/src/proc.rs -> crates/adit-media/src/proc.rs`
- 1-file cycle: `crates/adit-render/src/effects.rs -> crates/adit-render/src/effects.rs`
- 2-file cycle: `crates/adit-gui/src/lib.rs -> crates/adit-gui/src/logic.rs -> crates/adit-gui/src/lib.rs`
- 2-file cycle: `crates/adit-mcp/src/lib.rs -> crates/adit-mcp/src/proto.rs -> crates/adit-mcp/src/lib.rs`
- 2-file cycle: `crates/adit-media/src/lib.rs -> crates/adit-media/src/proc.rs -> crates/adit-media/src/lib.rs`
- 2-file cycle: `crates/adit-gui/src/app.rs -> crates/adit-gui/src/lib.rs -> crates/adit-gui/src/app.rs`

## Communities (136 total, 32 thin omitted)

### Community 0 - "Command"
Cohesion: 0.03
Nodes (63): Command, AddClip, AddEffect, AddMarker, AddMedia, AddSequence, AddTrack, AddTransition (+55 more)

### Community 1 - "Action"
Cohesion: 0.03
Nodes (61): Action, AddEdit, AddEditAll, AddMarker, ApplyAudioTransition, ApplyDefaultTransitions, ApplyVideoTransition, ClearIn (+53 more)

### Community 2 - "MediaError"
Cohesion: 0.08
Nodes (13): MissingMedia, Offline, Rec, Steps, Ffmpeg, MediaBackend, MediaError, Ffmpeg (+5 more)

### Community 3 - "Project"
Cohesion: 0.19
Nodes (43): alloc(), bad(), bounds_ok(), check_locked(), clear(), clip_ix(), clip_mut(), clip_ref() (+35 more)

### Community 4 - "Part 1 — adit-core"
Cohesion: 0.20
Nodes (10): Batch, Clips, Commands, General rules, Invariants (`Project::validate`), Media, sequences, tracks, markers, effect definitions, Parameters, keyframes, effects, transitions, Part 1 — adit-core (+2 more)

### Community 5 - "transition_at"
Cohesion: 0.30
Nodes (13): transition_at(), trans(), trans_project(), u_ed_6_duration_is_limited_by_short_clips(), u_ed_6_effect_name_is_passed_through(), u_ed_6_end_edge_with_next_clip(), u_ed_6_end_edge_without_next_clip(), u_ed_6_gap_duration_is_limited_by_short_clips() (+5 more)

### Community 6 - "base"
Cohesion: 0.04
Nodes (99): base(), solid_src(), sq(), chain(), edge_fixture(), h_10_batch_prunes_after_each_command(), h_15_remove_sequence_with_locked_track_rejected(), h_15_remove_sequence_without_locked_track_ok() (+91 more)

### Community 9 - "adit-render/tests/audio.rs"
Cohesion: 0.20
Nodes (33): a1_count_uses_sequence_sample_rate(), a1_empty_space_is_silence(), a1_exact_sample_counts(), a1_mix_window_offset(), a1_mute_solo_disabled(), a1_only_audio_tracks_contribute(), a2_atempo_chained_within_ffmpeg_range(), a2_filter_chain_order_and_atempo() (+25 more)

### Community 10 - "adit-media/tests/hardening.rs"
Cohesion: 0.07
Nodes (45): check_audio_filter(), TempDir, ALLOWED, ametadata_to(), ff(), FORBIDDEN, frame_to_time(), gray_offset() (+37 more)

### Community 11 - "trim"
Cohesion: 0.08
Nodes (60): Edge, End, Start, TrimMode, Normal, Ripple, Roll, trim() (+52 more)

### Community 12 - "validate.rs"
Cohesion: 0.11
Nodes (40): c_add_2_linked_sequence_source(), seq_src(), c_paste_1_nest_cycle_direct_and_indirect(), c_slip_1_kinds_and_errors(), i1_negative_start_rejected(), i1_overlap_rejected(), i1_touching_clips_are_fine_half_open(), i1_unsorted_rejected() (+32 more)

### Community 13 - "adit-core"
Cohesion: 0.93
Nodes (6): adit, adit-core, adit-gui, adit-mcp, adit-media, adit-render

### Community 14 - "frames.rs"
Cohesion: 0.12
Nodes (31): t2997(), assert_still(), backwards(), Fixture, Gray2997Mkv, Gray30Mp4, Id2997, Id30 (+23 more)

### Community 15 - "common"
Cohesion: 0.30
Nodes (12): assert_rgb(), ffmpeg_frame(), probe_video(), rgb_at(), long_project(), t_export_done_and_file_matches(), t_export_options_width_height_fps(), t_export_snapshot_ignores_later_edits() (+4 more)

### Community 16 - "encode.rs"
Cohesion: 0.22
Nodes (26): ffmpeg_decode_frame(), flat_rgba(), probe_audio_streams(), stereo_sine(), assert_video(), check_roundtrip(), COLOURS, encode() (+18 more)

### Community 17 - "timed"
Cohesion: 0.21
Nodes (23): timed(), ff(), m_det_1_missing_file_is_err(), m_det_1_no_cut_is_empty(), m_det_1_no_silence_is_empty(), m_det_1_scene_cut_red_to_blue(), m_det_1_silence_between_tones(), m_det_1_silence_shorter_than_min_duration_is_ignored() (+15 more)

### Community 18 - "decode.rs"
Cohesion: 0.13
Nodes (17): CACHE_BYTES, close(), decode_still(), FileDec, frame_len(), Frames, Key, Lru (+9 more)

### Community 19 - "session_api.rs"
Cohesion: 0.06
Nodes (46): at_clip(), G, hit_project(), hl(), layout(), PX, snap_project(), snap_ui() (+38 more)

### Community 20 - "Tools"
Cohesion: 0.22
Nodes (9): Editing, Effect library, Effects, Export, Magic-wand requests, Media, Projects, Seeing (+1 more)

### Community 21 - "assert_px"
Cohesion: 0.09
Nodes (60): assert_px(), render(), busy_project(), centroid(), green(), lit(), small(), text_seq() (+52 more)

### Community 22 - "effects_runtime.rs"
Cohesion: 0.10
Nodes (29): keyed(), param(), try_render(), e1_ctx_size_is_image_size(), e1_helpers_and_constants_allowed(), e1_output_is_straight_alpha(), e1_params_declared_in_order_and_overridden_by_instance(), e1_src_is_transparent_outside_0_1() (+21 more)

### Community 23 - "Session"
Cohesion: 0.11
Nodes (25): Session, Proj, project(), summary(), clip_count(), define(), effects(), files() (+17 more)

### Community 24 - "adit-render/tests/common/mod.rs"
Cohesion: 0.10
Nodes (18): AudioCall, AudioFn, BLUE, const_audio(), def(), F30, Fake, FPS30 (+10 more)

### Community 25 - "clips.rs"
Cohesion: 0.05
Nodes (91): c_add_1_default_names(), c_add_1_defaults(), c_add_1_g5_g6_g7(), c_add_1_r7_source_bounds_and_slack(), c_add_1_r7_stills_and_sequences_unbounded(), c_add_2_c_add_4_linked_overlap_on_audio_side(), c_add_2_kind_mismatch_cases(), c_add_2_linked_pair_id_order() (+83 more)

### Community 26 - "effects_builtin.rs"
Cohesion: 0.12
Nodes (31): assert_px_tol(), px(), C1, check(), eb_brightness_contrast(), eb_chroma_key(), eb_drop_shadow(), eb_ellipse() (+23 more)

### Community 27 - "adit-core/tests/common/mod.rs"
Cohesion: 0.06
Nodes (69): ParamTarget, frame_to_time(), Easing, EaseIn, EaseInOut, EaseOut, Hold, Linear (+61 more)

### Community 28 - "Frame"
Cohesion: 0.12
Nodes (34): Frame, assert_size(), frame_fn(), quadrants(), solid_frame(), time_frame(), detailed(), e_3_project_gaussian_blur_overrides_builtin() (+26 more)

### Community 29 - "Document"
Cohesion: 0.20
Nodes (4): Outcome, Document, err(), ok()

### Community 30 - "Harness"
Cohesion: 0.17
Nodes (39): button(), click_at(), alt(), button(), click_at(), ctrl(), drag(), long_seq() (+31 more)

### Community 31 - "ed_fix.rs"
Cohesion: 0.31
Nodes (10): copy_in_a_paste_in_b(), event_with_modifiers(), spans(), two_adjacent(), two_projects(), u_ed_23a_cut_event_without_shift_still_cuts(), u_ed_23a_shift_cut_event_ripple_deletes_and_leaves_clipboard_empty(), u_ed_23g_pasting_a_media_clip_into_another_project_adds_nothing() (+2 more)

### Community 32 - "text.rs"
Cohesion: 0.24
Nodes (3): candidates(), font_dirs(), Fonts

### Community 33 - "adit-media/tests/audio.rs"
Cohesion: 0.13
Nodes (39): assert_len(), assert_ramp(), audio_missing_file_is_not_found(), ff(), m_audio_1_from_aac_in_mp4(), m_audio_1_length_is_rounded_duration(), m_audio_1_mono_duplicated_to_both_channels(), m_audio_1_range_past_end_yields_fewer_samples() (+31 more)

### Community 34 - "proto.rs"
Cohesion: 0.19
Nodes (13): call_tool(), constant_time_eq(), err(), handle(), header(), http_request(), INSTRUCTIONS, is_blocking() (+5 more)

### Community 35 - "session"
Cohesion: 0.10
Nodes (65): add_solid(), call(), call_err(), depths(), edit(), edit_err(), full(), import_pattern() (+57 more)

### Community 36 - "adit-render/tests/export.rs"
Cohesion: 0.25
Nodes (20): ExportSettings, assert_rgb(), audio_fake(), decode(), out_path(), probe_audio(), probe_video(), red_blue() (+12 more)

### Community 37 - "adit-render/tests/hardening.rs"
Cohesion: 0.12
Nodes (23): COUNTER, validate_effect(), eb_every_builtin_validates(), e4_valid_definitions_pass(), BODY, brackets(), deep_parens(), DIRS (+15 more)

### Community 38 - "timeline.rs"
Cohesion: 0.08
Nodes (24): BAR, EDGE, HEADER, hit_time(), hit_track(), LABELS, preview(), RULER (+16 more)

### Community 39 - "render_cli.rs"
Cohesion: 0.14
Nodes (21): assert_rgb(), cut_and_silence_clip(), ffmpeg_frame(), ffprobe(), Probe, rgb_at(), run_adit(), run_ok() (+13 more)

### Community 40 - "ed_app.rs"
Cohesion: 0.10
Nodes (44): undo_depth(), click_label(), click_named(), drag(), drop_files(), Dropped, gap_setup(), label_of() (+36 more)

### Community 41 - "RenderError"
Cohesion: 0.12
Nodes (6): CtxVals, FORMAT, Gpu, gpu_err(), new_texture(), Scope

### Community 42 - "drags.rs"
Cohesion: 0.19
Nodes (46): Drag, assert_drag_noop(), gesture(), body(), check(), click(), dp(), drag_header_and_nothing_do_nothing() (+38 more)

### Community 43 - "fix_app.rs"
Cohesion: 0.09
Nodes (44): call_stepping(), add_text(), any_text(), assert_fresh(), click_at(), click_named(), content(), ctrl() (+36 more)

### Community 44 - "paste_label.rs"
Cohesion: 0.14
Nodes (49): PasteItem, bad(), c_label_1_g3_undo_restores_label(), c_label_1_label_8_ok_9_invalid_argument(), c_label_1_set_clip_label_omitted_in_json_is_none(), c_label_1_set_clip_sets_label_and_nothing_else(), c_label_1_split_by_overwrite_paste_copies_label(), c_paste_1_bad_params_and_keyframes_invalid_argument() (+41 more)

### Community 45 - "playback.rs"
Cohesion: 0.07
Nodes (16): Audio, Buffer, build(), Job, lock(), LOOKAHEAD_S, MAX_AHEAD_S, mixer() (+8 more)

### Community 46 - ".render_seq"
Cohesion: 0.26
Nodes (6): blend_index(), media_frames(), param(), Renderer, secs(), window()

### Community 47 - "adit-gui/tests/common/mod.rs"
Cohesion: 0.09
Nodes (20): A1, A2, ALL_ACTIONS, B, clip(), F, FPS30, MEDIA (+12 more)

### Community 48 - "contact_sheet"
Cohesion: 0.31
Nodes (8): contact_sheet(), encode(), render(), render_frame(), render_raw(), resize(), scale_to(), secs()

### Community 49 - "INDEX.md"
Cohesion: 0.17
Nodes (6): egui 0.37 / kittest / cpal notes for the GUI, ffmpeg subprocess gotchas, Knowledge index, MCP layer design notes, Mutation testing adit-core, wgpu 30 / naga headless notes

### Community 50 - "model.rs"
Cohesion: 0.07
Nodes (33): check_param(), finite(), Project, time_to_frame(), edges(), edges_unique(), nests_itself(), new_track() (+25 more)

### Community 51 - "EffectDef"
Cohesion: 0.23
Nodes (11): BLEND, BLUR_1D, builtin_effects(), BUILTINS, internal(), MIX, SCALE, Spec (+3 more)

### Community 52 - "effects.rs"
Cohesion: 0.07
Nodes (36): EffectDef, EffectKind, Filter, Transition, ParamDef, builtins(), check_effect(), check_effect_refs() (+28 more)

### Community 53 - "MediaBackend"
Cohesion: 0.22
Nodes (12): atempo_steps_multiply_back(), fetch(), filter_chain(), frame_at(), frame_count(), mix_audio(), mix_seq(), resample() (+4 more)

### Community 54 - "ui"
Cohesion: 0.09
Nodes (68): copy(), preview(), snap_delta(), ui(), acts(), clip_hit(), close_gap(), drag_of() (+60 more)

### Community 55 - "session_prefetch.rs"
Cohesion: 0.13
Nodes (28): clip(), media(), ms(), project(), Rec, rec(), S, s6_prefetch_calls_backend_for_each_media_frame_in_order() (+20 more)

### Community 56 - ".timeline_panel"
Cohesion: 0.38
Nodes (4): App, menu_text(), tick_step(), tolerance()

### Community 58 - "VideoCodec"
Cohesion: 0.40
Nodes (5): VideoCodec, Gif, H264, H265, Prores

### Community 59 - "main.rs"
Cohesion: 0.27
Nodes (16): AudioCodec, Aac, None, Pcm, backend(), export(), fail(), gui() (+8 more)

### Community 60 - "session_ed.rs"
Cohesion: 0.12
Nodes (15): Counting, Failing, pattern(), pattern_project(), s_4_computed_once_per_path_and_cached(), s_4_last_partial_bucket_counts(), s_5_divisor_1_equals_render(), s_5_other_divisors_are_err() (+7 more)

### Community 61 - "adit-mcp/tests/common/mod.rs"
Cohesion: 0.14
Nodes (25): approx(), assert_px(), decode(), f(), F30, find_clip(), find_track(), frame_png() (+17 more)

### Community 62 - "common"
Cohesion: 0.24
Nodes (4): alive(), h_1_define_effect_deep_braces_is_error(), h_1_effect_check_deep_braces_is_not_ok(), TOOLS

### Community 63 - "nav_logic.rs"
Cohesion: 0.18
Nodes (24): grab(), near(), near_t(), u_nav_1_anchor_left_of_scroll(), u_nav_1_example_max_zoom_clamps(), u_nav_1_example_zoom_in_keeps_anchor(), u_nav_1_example_zoom_out_scroll_clamps_at_zero(), u_nav_1_factor_one_keeps_view() (+16 more)

### Community 64 - "Mcp"
Cohesion: 0.27
Nodes (4): Mcp, new_project(), parse_tool_result(), ToolResult

### Community 65 - "transport.rs"
Cohesion: 0.15
Nodes (31): mark_project(), match_project(), nav(), nav_project(), only(), select_project(), step(), two_sec() (+23 more)

### Community 66 - "ExportSettings"
Cohesion: 0.18
Nodes (3): export(), MAX_EXPORT, temp_path()

### Community 68 - "edits.rs"
Cohesion: 0.10
Nodes (68): apply(), clips_on(), edit(), add(), after_insert(), AUD, audio_cmds(), AV10 (+60 more)

### Community 70 - "Time"
Cohesion: 0.09
Nodes (58): Clip, Param, Sequence, Sequence, Time, layout(), moving(), copy() (+50 more)

### Community 71 - "CLAUDE.md"
Cohesion: 0.25
Nodes (7): Contracts and test-first rule, Default working style: caveman + ponytail, graphify (code knowledge graph), Knowledge base (always maintain), One task per session, Project, Token budget: query before reading

### Community 72 - "adit GUI contract (SPEC Part 6)"
Cohesion: 0.17
Nodes (12): Actions (`perform`), adit GUI contract (SPEC Part 6), Binary, Editing comfort (U-ED-*), Keys, Rules added after the GUI review (U-FIX-*), Session additions (`adit_mcp::Session`), State (+4 more)

### Community 73 - "harness"
Cohesion: 0.11
Nodes (64): active(), add_named(), call(), clip_ids(), edit(), F30, H, harness() (+56 more)

### Community 74 - "edit"
Cohesion: 0.19
Nodes (18): body_drag(), edge(), edge_click(), enable(), END_T, locked_partner(), one_clip(), ops_text() (+10 more)

### Community 76 - "history.rs"
Cohesion: 0.13
Nodes (25): c_add_1_i4_kind_mismatch(), c_add_1_r7_text_source_in_stays_as_is(), c_add_2_unlinked_has_no_link(), c_add_4_place_overlap_and_touching(), add_m(), add_marker(), text_src(), c_batch_1_empty_batch_is_ok_noop() (+17 more)

### Community 77 - "Op"
Cohesion: 0.13
Nodes (29): ClipboardItem, Clip, Op, Copy, Edit, Export, GainDialog, Import (+21 more)

### Community 79 - "Inner"
Cohesion: 0.24
Nodes (37): ToolOutput, A, audio_levels(), bridge(), detect_scenes(), detect_silence(), edit(), effect_check() (+29 more)

### Community 80 - "Id"
Cohesion: 0.13
Nodes (10): Id, clip(), Fx, Msg, Stop, prefetch(), Snap, builtin_effects() (+2 more)

### Community 82 - "Tool"
Cohesion: 0.08
Nodes (27): Bridge, lock(), Mirror, png(), Shared, state_json(), switch_sequence(), time() (+19 more)

### Community 83 - "src/encode.rs"
Cohesion: 0.14
Nodes (13): Inner, start(), TempFile, unsupported(), write_temp(), bin(), command(), failed() (+5 more)

### Community 84 - "adit/tests/common/mod.rs"
Cohesion: 0.15
Nodes (8): RENDERER, drain_stderr(), Proc, S, Served, solid(), SWAP_WGSL, TOOL_NAMES

### Community 87 - "Rational"
Cohesion: 0.10
Nodes (26): Rational, FPS24, frame(), convert_times(), fps_strings(), frames_to_time(), gcd(), is_decimal() (+18 more)

### Community 89 - ".get"
Cohesion: 0.36
Nodes (3): A<'a>, effect_list(), waveform()

### Community 93 - "src/app.rs"
Cohesion: 0.10
Nodes (17): Dialog, AiImport, CloseProject, ConfirmClose, Gain, Speed, Wand, existing_selection() (+9 more)

### Community 95 - "library.rs"
Cohesion: 0.16
Nodes (45): swap_def(), as_def(), define(), effects(), fade_def(), FADE_WGSL, files(), instructions() (+37 more)

### Community 96 - "ffmpeg"
Cohesion: 0.18
Nodes (18): add_seq(), cut_mp4(), ffmpeg(), ffmpeg_session(), gap_wav(), h_13_fifth_concurrent_export_is_error(), h_14_detect_with_ffmpeg_session_works(), h_14_detect_with_pattern_session_is_error() (+10 more)

### Community 99 - "act"
Cohesion: 0.10
Nodes (33): act(), assert_noop(), copy_project(), label(), u_ed_1_cut_is_copy_then_delete(), u_ed_1_cut_leaves_out_locked_clips_like_delete(), u_ed_1_cut_with_empty_selection_is_no_op(), u_ed_1_cut_without_linked_selection() (+25 more)

### Community 100 - "Mutex"
Cohesion: 0.23
Nodes (11): bin(), free_port(), Media, Ffmpeg, Pattern, serve(), binary_serve_banner_and_bearer_auth(), binary_serve_default_port_7821() (+3 more)

### Community 101 - "UiBridge"
Cohesion: 0.24
Nodes (5): Image, ProjectInfo, UiBridge, projects(), set_ui()

### Community 102 - "src/tools.rs"
Cohesion: 0.06
Nodes (38): ToolInfo, absolutize(), call(), db(), def_json(), edit_description(), Entry, file_key() (+30 more)

### Community 103 - "SPEC.md"
Cohesion: 0.22
Nodes (8): adit behavioural spec, Audio, Effects, Export, Part 2 — adit-media, Part 3 — adit-render, Part 5 — hardening (added after independent review), Video

### Community 105 - "requests.rs"
Cohesion: 0.22
Nodes (16): request_list(), t_req_4_defaults_are_edit_null_panel_and_no_paths(), t_req_4_every_request_in_the_list_has_the_fields(), t_req_4_explicit_scope_edit_with_panel(), t_req_4_scope_app_has_source_dir_and_window_rect(), t_req_4_scope_import_carries_paths(), t_req_4_unknown_scope_is_error_and_files_nothing(), wait_request() (+8 more)

### Community 106 - "README.md"
Cohesion: 0.25
Nodes (4): adit MCP contract (SPEC Part 4), Binary, General rules, Library surface (`adit_mcp`)

### Community 107 - "t30"
Cohesion: 0.25
Nodes (22): assert_id_frame(), id_video(), t30(), ff(), m_frame_1_decodes_at_stream_size(), m_frame_2_clamps_before_start_and_after_end(), m_frame_2_still_returns_image_for_every_time(), m_frame_3_same_frame_returns_same_arc() (+14 more)

### Community 109 - "adit"
Cohesion: 0.25
Nodes (8): adit, Build and run, Connect an MCP client, Develop, Layout, License, Requirements, Status

### Community 119 - "src/probe.rs"
Cohesion: 0.40
Nodes (9): FLICKS_PER_SECOND, check_file(), flicks(), parse(), parses_streams_and_skips_cover_art(), probe(), rational(), secs() (+1 more)

### Community 120 - "panels.rs"
Cohesion: 0.08
Nodes (22): BlendMode, Add, Darken, Difference, Lighten, Multiply, Normal, Overlay (+14 more)

### Community 129 - "renderer"
Cohesion: 0.16
Nodes (12): renderer(), audio_project(), Dir, h_12_cancelled_export_keeps_existing_file(), h_12_failing_export_keeps_existing_file(), h_12_successful_export_replaces_existing_file(), h_5_export_longer_than_4_hours_rejected(), h_5_mix_audio_negative_duration_rejected() (+4 more)

### Community 130 - "props.rs"
Cohesion: 0.20
Nodes (12): all_clip_ids(), clip_ids(), effect_ids(), H, Op, op(), pick(), to_cmd() (+4 more)

### Community 133 - "e2e_story_real_ffmpeg"
Cohesion: 0.21
Nodes (10): approx(), color_clip(), f(), px(), same_path(), clip(), e2e_detect_silence_and_scenes(), e2e_import_failure_adds_nothing() (+2 more)

### Community 134 - "arc"
Cohesion: 0.24
Nodes (6): pump(), Setup, setup(), u_app_7_forward_playback_prefetches_the_cut_ahead_before_the_playhead_reaches_it(), u_app_7_no_prefetch_while_not_playing(), u_app_7_prefetch_stops_after_pause()

### Community 135 - "adit-render/tests/hardening_crash.rs"
Cohesion: 0.21
Nodes (8): audio_project(), backend(), braces(), filter(), h_1_brace_depth_64_ok(), h_1_brace_depth_65_rejected(), h_5_mix_audio_duration_above_max_time_rejected(), h_5_mix_audio_duration_i64_max_rejected()

### Community 136 - "monitors.rs"
Cohesion: 0.17
Nodes (6): guarded(), Monitors, note(), Picture, picture(), show()

### Community 137 - "Error"
Cohesion: 0.14
Nodes (11): Error, InUse, InvalidArgument, KindMismatch, NestCycle, NoAdjacentClip, NotFound, Overlap (+3 more)

### Community 138 - "adit-media/src/lib.rs"
Cohesion: 0.17
Nodes (11): AUDIO_FILTERS, Encoder, lock(), number_after(), parse_scenes(), parse_silence(), seconds(), seconds_and_log_parsing() (+3 more)

### Community 139 - "stdio.rs"
Cohesion: 0.24
Nodes (3): has_timecode(), num_with_suffix(), t1_initialize_reports_adit_tools_and_instructions()

### Community 140 - "KeyChord"
Cohesion: 0.36
Nodes (8): default_keymap(), KeyChord, default_keymap(), u_ed_1_keymap_has_copy_cut_paste_and_no_duplicate_chord(), k(), spec_keymap(), u_keys_keymap_is_exactly_the_table(), u_keys_spec_table_is_well_formed()

### Community 144 - "RenderError"
Cohesion: 0.24
Nodes (8): Renderer, RenderError, Cancelled, Gpu, Media, SequenceNotFound, Shader, UnknownEffect

## Knowledge Gaps
- **223 isolated node(s):** `Stop`, `Running`, `Done`, `Failed`, `Cancelled` (+218 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 568 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **32 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Id` connect `Id` to `Command`, `Action`, `props.rs`, `Project`, `transition_at`, `base`, `App`, `monitors.rs`, `Error`, `App`, `trim`, `validate.rs`, `RenderError`, `clips.rs`, `adit-core/tests/common/mod.rs`, `Document`, `Harness`, `ed_fix.rs`, `timeline.rs`, `ed_app.rs`, `drags.rs`, `fix_app.rs`, `paste_label.rs`, `playback.rs`, `contact_sheet`, `model.rs`, `effects.rs`, `ui`, `session_prefetch.rs`, `.timeline_panel`, `Time`, `history.rs`, `Op`, `Tool`, `.get`, `src/app.rs`, `src/tools.rs`, `panels.rs`?**
  _High betweenness centrality (0.136) - this node is a cross-community bridge._
- **Are the 215 inferred relationships involving `base()` (e.g. with `c_add_1_default_names()` and `c_add_1_defaults()`) actually correct?**
  _`base()` has 215 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Stop`, `Running`, `Done` to the rest of the system?**
  _223 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Command` be split into smaller, more focused modules?**
  _Cohesion score 0.033234126984126984 - nodes in this community are weakly interconnected._
- **Why does `Time` connect `Time` to `Command`, `Action`, `props.rs`, `Project`, `MediaError`, `transition_at`, `base`, `App`, `monitors.rs`, `App`, `arc`, `trim`, `validate.rs`, `adit-media/src/lib.rs`, `RenderError`, `decode.rs`, `session_api.rs`, `clips.rs`, `adit-core/tests/common/mod.rs`, `Harness`, `ed_fix.rs`, `timeline.rs`, `ed_app.rs`, `drags.rs`, `fix_app.rs`, `paste_label.rs`, `playback.rs`, `contact_sheet`, `model.rs`, `ui`, `session_prefetch.rs`, `.timeline_panel`, `session_ed.rs`, `harness`, `history.rs`, `Id`, `Tool`, `Rational`, `src/app.rs`, `src/tools.rs`, `src/probe.rs`, `panels.rs`?**
  _High betweenness centrality (0.107) - this node is a cross-community bridge._
- **Are the 100 inferred relationships involving `Id` (e.g. with `alloc()` and `clip_ix()`) actually correct?**
  _`Id` has 100 INFERRED edges - model-reasoned connections that need verification._
- **Should `Action` be split into smaller, more focused modules?**
  _Cohesion score 0.03278688524590164 - nodes in this community are weakly interconnected._