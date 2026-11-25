# Editing a real video through the adit MCP tools

Summary: behaviours of the `edit` tool found while cutting a 9:16 beat-synced edit; saves trial and error next time.

- `set_clip_speed` keeps the clip's source span and changes its timeline duration (1 s clip at 0.5x becomes 2 s), even with `ripple: false`. To fill a timeline slot of length L at speed s, add the clip with `duration = L * s`, then set the speed.
- A media or nested-sequence clip at scale 1 is fitted to the sequence width. 1280x720 in 1080x1920 is 1080x608; scale 3.16 fills the height.
- `position_x` / `position_y` are pixels from the frame centre; negative y is up.
- A track with a higher index draws on top. An adjustment clip (`source: {"type": "adjustment"}`) filters everything below it, so put text on a track above it to keep the text out of the grade and shake.
- Ids come from one counter shared by clips, effects and transitions. `define_effect`, `set_param`, `set_keyframe` and removals use no id. You can predict ids inside one batch and check them in `created`.
- Times written as seconds strings snap differently from neighbours. Back-to-back clips then fail with "clips would overlap on track N"; pass `mode: "overwrite"` on `add_clip`.
- `define_effect` with an existing name replaces that shader; clips that use it pick up the new code.
- Many timed hits (kills, beat pulses): bake the times into one WGSL filter as `const` arrays and read `ctx.time` on a full-length adjustment clip. This replaces hundreds of `set_keyframe` commands. WGSL needs `var ks = KT;` before indexing a const array with a loop variable.
- Text `font` is a font file name without extension (crates/adit-render/src/text.rs `candidates`), searched in Windows/user font dirs; unknown names fall back to Arial. Good on Windows: `seguibl` (Segoe UI Black) for big tags, `bahnschrift` for small captions (fake letter spacing by putting spaces between letters). `bold`/`italic` only switch files for the Arial fallback.
- In a 1.7x 16:9-in-9:16 layout, Valorant's kill banner sits about y +240. Put kill tags above the crosshair (y about -250) so they don't sit on it.
- adit has no frame interpolation. For optical-flow slow-mo, render the source range first with ffmpeg `minterpolate=fps=120:mi_mode=mci:mc_mode=aobmc:me_mode=bidir:vsbmc=1`, starting `-ss` on a frame boundary, plus `setpts=PTS-STARTPTS`. Import it and set `source_in` = original time minus the range start, then set the speed. A 30 fps sequence at 0.5x then pulls a different interpolated frame for every output frame (checked). About 1 min per 3 s of 720p.
- Valorant shows its own ACE banner around y -250 to -470 in this layout. Don't put a custom tag on top of it.
- A nested sequence (`source: {"type":"sequence"}`) used twice (blurred, scaled background plus foreground) gives the 16:9-in-9:16 look with one set of cuts.

Concerns: crates/adit-mcp (edit tool), crates/adit-core (Command). Verified: 2026-11-25
