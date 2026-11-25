//! The tools. One static table (`TOOLS`) drives both `tools/list` and dispatch, so they cannot drift.
//! Every argument is untrusted: bad input is an `Err` message, never a panic.

use crate::time::{convert_times, parse_fps, parse_time};
use crate::{Image, ProjectInfo, ToolInfo, ToolOutput, UiBridge};
use adit_core::{
    CLIP_PARAMS, ClipSource, Command, Document, EffectDef, EffectKind, FLICKS_PER_SECOND, Id,
    Media, MediaSpec, ParamDef, Project, Rational, Sequence, Time, time_to_frame,
};
use adit_media::{AudioCodec, ExportSettings, Frame, MediaBackend, VideoCodec};
use adit_render::{RenderError, Renderer};
use image::RgbaImage;
use serde_json::{Value, json};
use std::any::Any;
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::{Duration, Instant};

type R<T> = Result<T, String>;

/// A poisoned lock only means a tool panicked (we report that as an error); the data is usable.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

// ------------------------------------------------------------------------------------ state

pub struct Inner {
    media: Arc<dyn MediaBackend>,
    state: Mutex<State>,
    /// Signalled when a request is created or a job changes state.
    cv: Condvar,
    /// Created on first use (GPU setup is slow); preview rendering shares it.
    renderer: Mutex<Option<Renderer>>,
    /// The window the `ui_*` tools talk to.
    ui: Mutex<Option<Arc<dyn UiBridge>>>,
    /// Effect library directory (T-LIB-1); `None` = nothing saved, nothing offered.
    library: Option<PathBuf>,
    /// S-4 peaks per media path.
    waveforms: Mutex<HashMap<String, Arc<Vec<f32>>>>,
}

#[derive(Default)]
struct State {
    projects: Vec<Entry>,
    next_project: u64,
    requests: Vec<Request>,
    jobs: Vec<(String, Job)>,
}

struct Entry {
    id: String,
    doc: Document,
    path: Option<String>,
    dirty: bool,
    /// Bumped by `Entry::changed` on every mutation of `doc`.
    revision: u64,
}

impl Entry {
    fn changed(&mut self) {
        self.dirty = true;
        self.revision += 1;
    }
}

struct Request {
    id: String,
    project_id: String,
    sequence_id: Id,
    text: String,
    start: Option<Time>,
    end: Option<Time>,
    clip_ids: Vec<Id>,
    rect: Option<[f64; 4]>,
    scope: &'static str,
    panel: Option<String>,
    paths: Vec<String>,
    status: &'static str,
    message: String,
}

#[derive(Clone, Copy, PartialEq)]
enum JobState {
    Running,
    Done,
    Failed,
    Cancelled,
}

struct Job {
    state: JobState,
    error: Option<String>,
    path: String,
    done: Arc<AtomicU64>,
    total: Arc<AtomicU64>,
    cancel: Arc<AtomicBool>,
}

impl Inner {
    pub fn new(media: Arc<dyn MediaBackend>, library: Option<PathBuf>) -> Self {
        Inner {
            media,
            state: Mutex::new(State::default()),
            cv: Condvar::new(),
            renderer: Mutex::new(None),
            ui: Mutex::new(None),
            library,
            waveforms: Mutex::new(HashMap::new()),
        }
    }
}

fn entry<'a>(st: &'a mut State, id: &str) -> R<&'a mut Entry> {
    st.projects
        .iter_mut()
        .find(|e| e.id == id)
        .ok_or_else(|| format!("unknown project_id {id:?}"))
}

// ---------------------------------------------------------------------------------- argument access

#[derive(Clone, Copy)]
struct A<'a>(&'a Value);

impl<'a> A<'a> {
    /// `null` counts as absent.
    fn get(self, k: &str) -> Option<&'a Value> {
        self.0.get(k).filter(|v| !v.is_null())
    }
    fn opt_str(self, k: &str) -> R<Option<&'a str>> {
        match self.get(k) {
            None => Ok(None),
            Some(Value::String(s)) => Ok(Some(s)),
            Some(_) => Err(format!("`{k}` must be a string")),
        }
    }
    fn str(self, k: &str) -> R<&'a str> {
        self.opt_str(k)?
            .ok_or_else(|| format!("missing required argument `{k}`"))
    }
    fn opt_u64(self, k: &str) -> R<Option<u64>> {
        match self.get(k) {
            None => Ok(None),
            Some(v) => v
                .as_u64()
                .map(Some)
                .ok_or_else(|| format!("`{k}` must be a non-negative integer")),
        }
    }
    fn opt_u32(self, k: &str) -> R<Option<u32>> {
        self.opt_u64(k)?
            .map(|v| u32::try_from(v).map_err(|_| format!("`{k}` is out of range")))
            .transpose()
    }
    fn opt_f64(self, k: &str) -> R<Option<f64>> {
        match self.get(k) {
            None => Ok(None),
            Some(v) => v
                .as_f64()
                .filter(|x| x.is_finite())
                .map(Some)
                .ok_or_else(|| format!("`{k}` must be a number")),
        }
    }
    fn opt_bool(self, k: &str) -> R<Option<bool>> {
        match self.get(k) {
            None => Ok(None),
            Some(Value::Bool(b)) => Ok(Some(*b)),
            Some(_) => Err(format!("`{k}` must be a boolean")),
        }
    }
    fn time(self, k: &str, fps: Option<Rational>) -> R<Option<Time>> {
        self.get(k).map(|v| parse_time(k, v, fps)).transpose()
    }
    fn opt_fps(self, k: &str) -> R<Option<Rational>> {
        match self.get(k) {
            None => Ok(None),
            Some(Value::String(s)) => parse_fps(s).map(Some),
            Some(Value::Number(n)) => parse_fps(&n.to_string()).map(Some),
            Some(_) => Err(format!("`{k}` must be a string like \"30\" or \"29.97\"")),
        }
    }
    fn array(self, k: &str) -> R<Option<&'a Vec<Value>>> {
        match self.get(k) {
            None => Ok(None),
            Some(Value::Array(v)) => Ok(Some(v)),
            Some(_) => Err(format!("`{k}` must be an array")),
        }
    }
    fn ids(self, k: &str) -> R<Vec<Id>> {
        self.array(k)?
            .map(|v| {
                v.iter()
                    .map(|x| {
                        x.as_u64()
                            .ok_or_else(|| format!("`{k}` must hold integer ids"))
                    })
                    .collect::<R<Vec<Id>>>()
            })
            .unwrap_or(Ok(Vec::new()))
    }
}

fn ok(json: Value) -> R<ToolOutput> {
    Ok(ToolOutput {
        json,
        images: Vec::new(),
    })
}

fn secs(t: Time) -> f64 {
    t as f64 / FLICKS_PER_SECOND as f64
}

fn fps_str(f: Rational) -> String {
    format!("{}/{}", f.num, f.den)
}

/// Relative paths resolve against the server's working directory.
fn absolutize(p: &str) -> R<String> {
    let path = Path::new(p);
    if path.is_absolute() {
        return Ok(p.to_string());
    }
    let cwd = std::env::current_dir().map_err(|e| format!("no working directory: {e}"))?;
    Ok(cwd.join(path).to_string_lossy().into_owned())
}

fn resolve_seq<'p>(p: &'p Project, a: A) -> R<&'p Sequence> {
    match a.opt_u64("sequence_id")? {
        Some(id) => p
            .sequence(id)
            .ok_or_else(|| format!("sequence {id} not found")),
        None => p
            .sequences
            .first()
            .ok_or_else(|| "the project has no sequence".to_string()),
    }
}

/// A copy of a project plus the sequence the call targets, so slow work runs without the lock.
struct Snap {
    project: Project,
    seq: Id,
    fps: Rational,
}

fn snap(i: &Inner, a: A) -> R<Snap> {
    let mut st = lock(&i.state);
    let p = entry(&mut st, a.str("project_id")?)?.doc.project();
    let s = resolve_seq(p, a)?;
    Ok(Snap {
        seq: s.id,
        fps: s.fps,
        project: p.clone(),
    })
}

// -------------------------------------------------------------------------------------- projects

fn project_new(i: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let name = a.str("name")?;
    let (width, height) = (
        a.opt_u32("width")?.unwrap_or(1920),
        a.opt_u32("height")?.unwrap_or(1080),
    );
    let sample_rate = a.opt_u32("sample_rate")?.unwrap_or(48000);
    let fps = a.opt_fps("fps")?.unwrap_or(Rational { num: 30, den: 1 });
    // shortcut: 256 tracks per kind is plenty; raise if someone needs more.
    let tracks = |k: &str| -> R<u32> {
        match a.opt_u32(k)?.unwrap_or(2) {
            n if n <= 256 => Ok(n),
            _ => Err(format!("`{k}` is at most 256")),
        }
    };
    let (video_tracks, audio_tracks) = (tracks("video_tracks")?, tracks("audio_tracks")?);
    let mut doc = Document::new(Project::new(name));
    let out = doc
        .apply(Command::AddSequence {
            name: "Main".into(),
            width,
            height,
            fps,
            sample_rate,
            video_tracks,
            audio_tracks,
        })
        .map_err(|e| e.to_string())?;
    // Creation is not undoable: start the history afresh.
    let doc = Document::new(doc.project().clone());
    let mut st = lock(&i.state);
    st.next_project += 1;
    let id = format!("p{}", st.next_project);
    st.projects.push(Entry {
        id: id.clone(),
        doc,
        path: None,
        dirty: true,
        revision: 0,
    });
    ok(json!({ "project_id": id, "sequence_id": out.created[0], "track_ids": &out.created[1..] }))
}

fn project_open(i: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let path = absolutize(a.str("path")?)?;
    let text = std::fs::read_to_string(&path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let project = Project::from_json(&text).map_err(|e| format!("{path}: {e}"))?;
    project
        .validate()
        .map_err(|e| format!("{path}: invalid project: {e}"))?;
    let mut st = lock(&i.state);
    st.next_project += 1;
    let id = format!("p{}", st.next_project);
    st.projects.push(Entry {
        id: id.clone(),
        doc: Document::new(project),
        path: Some(path),
        dirty: false,
        revision: 0,
    });
    ok(json!({ "project_id": id }))
}

/// Temp file next to the target, then rename: readers never see a partial file.
fn write_atomic(path: &str, text: &str) -> R<()> {
    static N: AtomicU64 = AtomicU64::new(0);
    let p = Path::new(path);
    let name = p.file_name().ok_or("the path has no file name")?;
    let tmp = p.with_file_name(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let cleanup = |e: std::io::Error| {
        let _ = std::fs::remove_file(&tmp);
        format!("cannot write {path}: {e}")
    };
    // H-13: flushed before the rename, so a crash cannot leave the new name on unwritten data.
    std::fs::File::create(&tmp)
        .and_then(|mut f| {
            std::io::Write::write_all(&mut f, text.as_bytes())?;
            f.sync_all()
        })
        .map_err(cleanup)?;
    std::fs::rename(&tmp, p).map_err(cleanup)
}

fn project_save(i: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let given = a.opt_str("path")?.map(absolutize).transpose()?;
    // shortcut: the write happens under the state lock so `dirty` cannot miss a concurrent edit;
    // project files are small. Release it first if saves ever get slow.
    let mut st = lock(&i.state);
    let e = entry(&mut st, a.str("project_id")?)?;
    let path = given
        .or_else(|| e.path.clone())
        .ok_or("this project has never been saved: pass `path`")?;
    let text = serde_json::to_string_pretty(e.doc.project()).map_err(|e| e.to_string())?;
    write_atomic(&path, &text)?;
    e.path = Some(path.clone());
    e.dirty = false;
    ok(json!({ "project_id": e.id, "path": path }))
}

fn project_close(i: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let id = a.str("project_id")?;
    let mut st = lock(&i.state);
    entry(&mut st, id)?;
    st.projects.retain(|e| e.id != id);
    ok(json!({}))
}

fn project_list(i: &Arc<Inner>, _: A) -> R<ToolOutput> {
    let st = lock(&i.state);
    ok(Value::Array(
        st.projects
            .iter()
            .map(|e| json!({ "project_id": e.id, "name": e.doc.project().name, "path": e.path, "dirty": e.dirty }))
            .collect(),
    ))
}

fn media_json(m: &Media) -> Value {
    let s = &m.spec;
    json!({
        "id": m.id, "name": s.name, "path": s.path,
        "duration_s": s.duration.map(secs),
        "video": s.video.map(|v| json!({ "width": v.width, "height": v.height, "fps": fps_str(v.fps) })),
        "audio": s.audio.map(|x| json!({ "sample_rate": x.sample_rate, "channels": x.channels })),
    })
}

fn source_str(s: &ClipSource) -> String {
    match s {
        ClipSource::Media { media_id } => format!("media:{media_id}"),
        ClipSource::Sequence { sequence_id } => format!("sequence:{sequence_id}"),
        ClipSource::Text { text } => {
            format!("text:{}", text.content.chars().take(40).collect::<String>())
        }
        ClipSource::Solid { .. } => "solid".into(),
        ClipSource::Adjustment => "adjustment".into(),
    }
}

fn summary(e: &Entry) -> Value {
    let p = e.doc.project();
    let sequences: Vec<Value> = p
        .sequences
        .iter()
        .map(|sq| {
            let frame = |t: Time| time_to_frame(t, sq.fps);
            let tracks: Vec<Value> = sq
                .tracks
                .iter()
                .map(|t| {
                    let clips: Vec<Value> = t
                        .clips
                        .iter()
                        .map(|c| {
                            json!({
                                "id": c.id, "name": c.name, "source": source_str(&c.source),
                                "start_s": secs(c.start), "end_s": secs(c.end()),
                                "start_frame": frame(c.start), "end_frame": frame(c.end()),
                                "source_in_s": secs(c.source_in), "speed": c.speed,
                                "enabled": c.enabled, "link": c.link,
                                "effects": c.effects.iter().map(|x| x.effect.as_str()).collect::<Vec<_>>(),
                            })
                        })
                        .collect();
                    let transitions: Vec<Value> = t
                        .transitions
                        .iter()
                        .map(|x| json!({ "id": x.id, "left": x.left, "right": x.right, "effect": x.effect, "duration_s": secs(x.duration) }))
                        .collect();
                    json!({
                        "id": t.id, "name": t.name, "kind": t.kind, "locked": t.locked,
                        "muted": t.muted, "solo": t.solo, "hidden": t.hidden,
                        "clips": clips, "transitions": transitions,
                    })
                })
                .collect();
            let d = sq.duration();
            json!({
                "id": sq.id, "name": sq.name, "width": sq.width, "height": sq.height,
                "fps": fps_str(sq.fps), "duration_s": secs(d), "duration_frames": frame(d),
                "tracks": tracks,
                "markers": sq.markers.iter().map(|m| json!({ "id": m.id, "time_s": secs(m.time), "name": m.name })).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({
        "project_id": e.id, "name": p.name, "path": e.path,
        "undo_depth": e.doc.undo_depth(), "redo_depth": e.doc.redo_depth(),
        "media": p.media.iter().map(media_json).collect::<Vec<_>>(),
        "sequences": sequences,
        "effects": p.effects.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(),
    })
}

fn project_get(i: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let detail = a.opt_str("detail")?.unwrap_or("summary");
    let mut st = lock(&i.state);
    let e = entry(&mut st, a.str("project_id")?)?;
    match detail {
        "summary" => ok(summary(e)),
        "full" => ok(json!({
            "project": e.doc.project(),
            "undo_depth": e.doc.undo_depth(),
            "redo_depth": e.doc.redo_depth(),
        })),
        o => Err(format!(
            "`detail` must be \"summary\" or \"full\", got {o:?}"
        )),
    }
}

// ----------------------------------------------------------------------------------------- media

fn media_import(i: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let id = a.str("project_id")?;
    let paths = a
        .array("paths")?
        .ok_or("missing required argument `paths`")?;
    entry(&mut lock(&i.state), id)?;
    // Probing spawns ffprobe: do it without holding the lock.
    let mut specs: Vec<MediaSpec> = Vec::new();
    for p in paths {
        let p = p.as_str().ok_or("`paths` must hold strings")?;
        let abs = absolutize(p)?;
        let mut spec = i.media.probe(&abs).map_err(|e| format!("{p}: {e}"))?;
        spec.path = abs;
        specs.push(spec);
    }
    let cmds = specs
        .iter()
        .cloned()
        .map(|spec| Command::AddMedia { spec })
        .collect();
    let mut st = lock(&i.state);
    let e = entry(&mut st, id)?;
    let out = e
        .doc
        .apply(Command::Batch { commands: cmds })
        .map_err(|e| e.to_string())?;
    if !specs.is_empty() {
        e.changed();
    }
    let media: Vec<Value> = out
        .created
        .iter()
        .zip(specs)
        .map(|(id, spec)| media_json(&Media { id: *id, spec }))
        .collect();
    ok(json!({ "media": media }))
}

fn media_path(i: &Inner, a: A) -> R<(String, Rational)> {
    let id = a
        .opt_u64("media_id")?
        .ok_or("missing required argument `media_id`")?;
    let mut st = lock(&i.state);
    let p = entry(&mut st, a.str("project_id")?)?.doc.project();
    let m = p
        .media
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| format!("media {id} not found"))?;
    // Frame strings (min_duration) use the first sequence's rate, as everywhere (T-4).
    let fps = p
        .sequences
        .first()
        .map_or(Rational { num: 30, den: 1 }, |s| s.fps);
    Ok((m.spec.path.clone(), fps))
}

fn detect_silence(i: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let (path, fps) = media_path(i, a)?;
    let noise = a.opt_f64("noise_db")?.unwrap_or(-35.0);
    let min = a
        .time("min_duration", Some(fps))?
        .unwrap_or(FLICKS_PER_SECOND * 2 / 5);
    let r = i
        .media
        .detect_silence(&path, noise, min)
        .map_err(|e| e.to_string())?;
    ok(
        json!({ "ranges": r.iter().map(|(s, e)| json!({ "start_s": secs(*s), "end_s": secs(*e) })).collect::<Vec<_>>() }),
    )
}

fn detect_scenes(i: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let (path, _) = media_path(i, a)?;
    let threshold = a.opt_f64("threshold")?.unwrap_or(0.3);
    let r = i
        .media
        .detect_scenes(&path, threshold)
        .map_err(|e| e.to_string())?;
    ok(json!({ "cuts_s": r.into_iter().map(secs).collect::<Vec<_>>() }))
}

fn db(x: f64) -> f64 {
    if x <= 1e-6 {
        -120.0
    } else {
        (20.0 * x.log10()).max(-120.0)
    }
}

fn audio_levels(i: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let s = snap(i, a)?;
    let seq = s.project.sequence(s.seq).ok_or("sequence vanished")?;
    let start = a.time("start", Some(s.fps))?.unwrap_or(0);
    // H-5: past the sequence end is silence; clamping keeps the mix buffer bounded.
    let end = a
        .time("end", Some(s.fps))?
        .unwrap_or(Time::MAX)
        .min(seq.duration());
    let buckets = a.opt_u64("buckets")?.unwrap_or(50);
    if !(1..=10_000).contains(&buckets) {
        return Err("`buckets` must be 1..10000".into());
    }
    if start < 0 || end <= start {
        return Err("need 0 <= start < end (end is clamped to the sequence end)".into());
    }
    let mix = adit_render::mix_audio(&s.project, s.seq, start, end - start, &*i.media)
        .map_err(|e| e.to_string())?;
    let frames = mix.len() / 2;
    let b = buckets as usize;
    let (mut rms, mut peak) = (Vec::with_capacity(b), Vec::with_capacity(b));
    for k in 0..b {
        let part = &mix[frames * k / b * 2..frames * (k + 1) / b * 2];
        let sum: f64 = part.iter().map(|x| f64::from(*x).powi(2)).sum();
        let max = part.iter().fold(0f64, |m, x| m.max(f64::from(*x).abs()));
        rms.push(db((sum / part.len().max(1) as f64).sqrt()));
        peak.push(db(max));
    }
    ok(json!({ "bucket_s": secs(end - start) / buckets as f64, "rms_db": rms, "peak_db": peak }))
}

// ------------------------------------------------------------------------------------- editing

/// `op: fields` for the `edit` description. Ids are integers; times take T-4 strings; `?` = optional.
const OPS: &[(&str, &str)] = &[
    (
        "add_media",
        "spec{name,path,duration?,video?{width,height,fps{num,den}},audio?{sample_rate,channels}}",
    ),
    ("remove_media", "media_id"),
    ("set_media", "media_id, name?, path?"),
    (
        "add_sequence",
        "name, width, height, fps{num,den}, sample_rate, video_tracks?=0, audio_tracks?=0 (created = [sequence_id, track ids])",
    ),
    ("remove_sequence", "sequence_id"),
    (
        "set_sequence",
        "sequence_id, name?, width?, height?, fps?{num,den}, sample_rate?",
    ),
    ("add_track", "sequence_id, kind(video|audio), name?, index?"),
    ("remove_track", "track_id"),
    ("move_track", "track_id, index"),
    (
        "set_track",
        "track_id, name?, locked?, muted?, solo?, hidden?",
    ),
    ("add_marker", "sequence_id, time, duration?=0, name?, note?"),
    ("remove_marker", "marker_id"),
    (
        "define_effect",
        "def{name, kind(filter|transition), wgsl, params[{name,default,min,max}]} (validated; see server instructions for the shader contract)",
    ),
    ("remove_effect_def", "name"),
    (
        "add_clip",
        "track_id, source, start, duration, source_in?=0, linked_track_id?(audio track; also adds the linked audio clip), mode?(place|overwrite|insert)=place, name?",
    ),
    ("remove_clips", "clip_ids[], include_linked?=true"),
    ("ripple_delete_clips", "clip_ids[], include_linked?=true"),
    ("lift_range", "sequence_id, start, end, track_ids?[]"),
    ("extract_range", "sequence_id, start, end, track_ids?[]"),
    ("close_gap", "track_id, time"),
    ("split_at", "sequence_id, time, track_ids?[]"),
    ("split_clip", "clip_id, time, include_linked?=true"),
    (
        "move_clip",
        "clip_id, start, track_id?, mode?(place|overwrite)=place, include_linked?=true",
    ),
    (
        "trim_clip",
        "clip_id, edge(start|end), to, mode?(normal|ripple|roll)=normal, include_linked?=true",
    ),
    ("slip_clip", "clip_id, delta, include_linked?=true"),
    ("slide_clip", "clip_id, delta"),
    (
        "set_clip_speed",
        "clip_id, speed(0.01..100), ripple?=false, include_linked?=true",
    ),
    (
        "set_clip",
        "clip_id, name?, enabled?, reverse?, blend?(normal|add|multiply|screen|overlay|darken|lighten|difference), audio_filter?(ffmpeg -af chain), label?(0 = none, 1..8 = a colour)",
    ),
    (
        "paste_clips",
        "items[{track_id, clip(a full clip object as project_get gives it; its start is where the copy goes, its id is ignored)}], mode?(place|overwrite)=place",
    ),
    (
        "set_text",
        "clip_id, text{content,font,size,color[r,g,b,a],align(left|center|right),bold,italic}",
    ),
    ("link_clips", "clip_ids[]"),
    ("unlink_clips", "clip_ids[]"),
    ("group_clips", "clip_ids[]"),
    ("ungroup_clips", "clip_ids[]"),
    ("set_param", "target, value"),
    (
        "set_keyframe",
        "target, time(relative to clip start), value, easing(linear|hold|ease_in|ease_out|ease_in_out)",
    ),
    ("remove_keyframe", "target, time"),
    ("clear_keyframes", "target"),
    (
        "add_effect",
        "clip_id, effect(name; see effect_list), index?, params?{name:number}",
    ),
    ("remove_effect", "clip_id, effect_id"),
    ("move_effect", "clip_id, effect_id, index"),
    ("set_effect_enabled", "clip_id, effect_id, enabled"),
    (
        "add_transition",
        "track_id, left?(clip id), right?(clip id), effect(a transition), duration, params?{name:number}",
    ),
    ("remove_transition", "transition_id"),
    (
        "set_transition",
        "transition_id, effect?, duration?, params?{name:number}",
    ),
    ("batch", "commands[] (nested commands, applied as one)"),
];

fn edit_description() -> String {
    let mut s = String::from(
        "Apply edit commands to a project as ONE atomic batch: one undo step, and if any command \
         fails nothing changes. `commands` is an array of objects {\"op\": <op>, ...fields}. Time \
         fields (start, end, duration, time, to, delta, source_in) take integer flicks or \"1.5s\" / \
         \"45f\" / \"00:00:01:15\". Ids are integers; `?` marks optional fields. Returns \
         {created:[[ids created by command 0],...], undo_depth, library:[effect names saved to the \
         effect library]}. Ops:\n",
    );
    for (op, fields) in OPS {
        s += &format!("- {op}: {fields}\n");
    }
    s += "source = {\"type\":\"media\",\"media_id\":N} | {\"type\":\"sequence\",\"sequence_id\":N} | \
          {\"type\":\"text\",\"text\":{content,font,size,color,align,bold,italic}} | \
          {\"type\":\"solid\",\"color\":[r,g,b,a]} | {\"type\":\"adjustment\"} (colours are 0..1, straight alpha).\n\
          target = {clip_id, effect_id?, name}; without effect_id `name` is a clip property: ";
    s += &CLIP_PARAMS
        .iter()
        .map(|(n, d)| format!("{n}({d})"))
        .collect::<Vec<_>>()
        .join(", ");
    s += " (volume in dB). Effects used by add_effect / add_transition must exist (effect_list); a \
          define_effect earlier in the same batch counts, and so does an effect of the library.";
    s
}

fn builtins() -> &'static [EffectDef] {
    static B: OnceLock<Vec<EffectDef>> = OnceLock::new();
    B.get_or_init(adit_render::builtin_effects)
}

fn check_effect(p: &Project, name: &str, kind: EffectKind) -> R<()> {
    // E-3: project definitions first, a wrong-kind one does not hide a built-in of the right kind.
    let all = || p.effects.iter().chain(builtins());
    if all().any(|d| d.name == name && d.kind == kind) {
        return Ok(());
    }
    match all().find(|d| d.name == name) {
        Some(d) => Err(format!(
            "effect `{name}` is a {:?} effect, not a {kind:?} effect",
            d.kind
        )),
        None => Err(format!("unknown effect `{name}` (see effect_list)")),
    }
}

/// H-9: every effect a clip or transition names still resolves with its kind (catches
/// `remove_effect_def` of a used effect and a `define_effect` that changes the kind).
fn check_effect_refs(p: &Project) -> R<()> {
    for t in p.sequences.iter().flat_map(|s| &s.tracks) {
        for c in &t.clips {
            for e in &c.effects {
                check_effect(p, &e.effect, EffectKind::Filter)
                    .map_err(|m| format!("clip {} effect {}: {m}", c.id, e.id))?;
            }
        }
        for tr in &t.transitions {
            check_effect(p, &tr.effect, EffectKind::Transition)
                .map_err(|m| format!("transition {}: {m}", tr.id))?;
        }
    }
    Ok(())
}

/// Applies `c` to the scratch document after the checks the core does not do (T-EDIT), collecting
/// the ids it creates. Earlier commands are visible to later checks, so define-then-use works.
/// A failed use of an effect no definition names is left in `need` (T-LIB-4 looks it up).
fn run_checked(
    d: &mut Document,
    c: &Command,
    out: &mut Vec<Id>,
    need: &mut Option<(String, EffectKind)>,
) -> R<()> {
    if let Command::Batch { commands } = c {
        for c in commands {
            run_checked(d, c, out, need)?;
        }
        return Ok(());
    }
    let mut used = |p: &Project, name: &str, kind| {
        check_effect(p, name, kind).inspect_err(|_| {
            if !p.effects.iter().chain(builtins()).any(|d| d.name == name) {
                *need = Some((name.to_string(), kind));
            }
        })
    };
    match c {
        Command::DefineEffect { def } => adit_render::validate_effect(def).map_err(|e| {
            format!(
                "invalid effect `{}`: {}",
                def.name,
                if e.is_empty() { "rejected" } else { &e }
            )
        })?,
        Command::AddEffect { effect, .. } => used(d.project(), effect, EffectKind::Filter)?,
        Command::AddTransition { effect, .. }
        | Command::SetTransition {
            effect: Some(effect),
            ..
        } => used(d.project(), effect, EffectKind::Transition)?,
        Command::SetClip {
            audio_filter: Some(f),
            ..
        } => adit_media::check_audio_filter(f).map_err(|e| format!("audio_filter: {e}"))?,
        _ => {}
    }
    out.extend(d.apply(c.clone()).map_err(|e| e.to_string())?.created);
    Ok(())
}

fn edit(i: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let cmds = a
        .array("commands")?
        .ok_or("missing required argument `commands`")?;
    let mut st = lock(&i.state);
    let e = entry(&mut st, a.str("project_id")?)?;
    let fps = match a.opt_u64("sequence_id")? {
        Some(_) => Some(resolve_seq(e.doc.project(), a)?.fps),
        None => e.doc.project().sequences.first().map(|s| s.fps),
    };
    let mut parsed = Vec::with_capacity(cmds.len());
    for (n, c) in cmds.iter().enumerate() {
        let mut c = c.clone();
        let at = |m: String| format!("commands[{n}]: {m}");
        convert_times(&mut c, fps).map_err(at)?;
        parsed.push(serde_json::from_value::<Command>(c).map_err(|m| at(m.to_string()))?);
    }
    // T-LIB-4: a used effect that only the library has gets a `define_effect` in front of the batch;
    // the dry run then starts over, so it always checks exactly what is applied.
    let mut front: Vec<Command> = Vec::new();
    let created = loop {
        let mut need = None;
        match dry_run(e.doc.project(), &front, &parsed, &mut need) {
            Ok(c) => break c,
            Err(m) => {
                let Some((name, kind)) = need else {
                    return Err(m);
                };
                let tried = front
                    .iter()
                    .any(|c| matches!(c, Command::DefineEffect { def } if def.name == name));
                let Some(def) = i
                    .library
                    .as_deref()
                    .filter(|_| !tried)
                    .and_then(|dir| lib_load(dir, &name))
                else {
                    return Err(m);
                };
                if def.kind != kind {
                    return Err(format!(
                        "library effect `{name}` is a {:?} effect, not a {kind:?} effect",
                        def.kind
                    ));
                }
                front.push(Command::DefineEffect { def });
            }
        }
    };
    let mut defs = Vec::new();
    collect_defs(&parsed, &mut defs);
    let before = e.doc.undo_depth();
    front.extend(parsed);
    e.doc
        .apply(Command::Batch { commands: front })
        .map_err(|e| e.to_string())?;
    if e.doc.undo_depth() != before {
        e.changed();
    }
    let undo_depth = e.doc.undo_depth();
    drop(st);
    // T-LIB-2: only now, so a failed edit saves nothing; a write failure only leaves the name out.
    let mut library: Vec<&str> = Vec::new();
    if let Some(dir) = &i.library {
        for d in defs.iter().filter(|d| lib_name_ok(&d.name)) {
            let saved = std::fs::create_dir_all(dir).is_ok()
                && dir
                    .join(format!("{}.json", d.name))
                    .to_str()
                    .zip(serde_json::to_string(d).ok())
                    .is_some_and(|(p, text)| write_atomic(p, &text).is_ok());
            if saved && !library.contains(&d.name.as_str()) {
                library.push(&d.name);
            }
        }
    }
    ok(json!({ "created": created, "undo_depth": undo_depth, "library": library }))
}

/// Dry run on a copy: gives `created` per command, runs the extra checks in order, and means the
/// real document is only touched by a batch that is known to work. `front` runs first.
// shortcut: costs one extra apply per command; fine for batches of hundreds.
fn dry_run(
    p: &Project,
    front: &[Command],
    cmds: &[Command],
    need: &mut Option<(String, EffectKind)>,
) -> R<Vec<Vec<Id>>> {
    let mut scratch = Document::new(p.clone());
    for c in front {
        run_checked(&mut scratch, c, &mut Vec::new(), need)?;
    }
    let mut created = Vec::with_capacity(cmds.len());
    for (n, c) in cmds.iter().enumerate() {
        let mut ids = Vec::new();
        run_checked(&mut scratch, c, &mut ids, need).map_err(|m| format!("commands[{n}]: {m}"))?;
        created.push(ids);
        scratch = Document::new(scratch.project().clone()); // drop the scratch undo history
    }
    check_effect_refs(scratch.project())?;
    Ok(created)
}

/// Every `define_effect` of `cmds`, at any batch depth, in command order.
fn collect_defs(cmds: &[Command], out: &mut Vec<EffectDef>) {
    for c in cmds {
        match c {
            Command::DefineEffect { def } => out.push(def.clone()),
            Command::Batch { commands } => collect_defs(commands, out),
            _ => {}
        }
    }
}

// ------------------------------------------------------------------------------ effect library

/// T-LIB-2: the name becomes a file name, so nothing outside `[A-Za-z0-9_-]{1,64}` gets near a
/// path; built-in names are never library effects.
fn lib_name_ok(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        && !builtins().iter().any(|d| d.name == name)
}

/// `<dir>/<name>.json` when it is an `EffectDef` with that same name (T-LIB-6).
// shortcut: reads the whole file whatever its size; cap it if the directory is ever shared.
fn lib_load(dir: &Path, name: &str) -> Option<EffectDef> {
    if !lib_name_ok(name) {
        return None;
    }
    let text = std::fs::read_to_string(dir.join(format!("{name}.json"))).ok()?;
    let def: EffectDef = serde_json::from_str(&text).ok()?;
    (def.name == name).then_some(def)
}

/// The valid library effects in name order (T-LIB-3); read from disk on every call.
fn lib_all(dir: &Path) -> Vec<EffectDef> {
    let mut v: Vec<EffectDef> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|f| lib_load(dir, f.file_name().to_str()?.strip_suffix(".json")?))
        .collect();
    v.sort_by(|a, b| a.name.cmp(&b.name));
    v
}

fn step(i: &Arc<Inner>, a: A, undo: bool) -> R<ToolOutput> {
    let steps = a.opt_u64("steps")?.unwrap_or(1);
    let mut st = lock(&i.state);
    let e = entry(&mut st, a.str("project_id")?)?;
    let mut done = 0u64;
    while done < steps && (if undo { e.doc.undo() } else { e.doc.redo() }) {
        done += 1;
    }
    if done > 0 {
        e.changed();
    }
    ok(json!({ "done": done, "undo_depth": e.doc.undo_depth(), "redo_depth": e.doc.redo_depth() }))
}

// ------------------------------------------------------------------------------------- effects

fn def_json(d: &EffectDef, origin: &str, source: bool) -> Value {
    let mut v = json!({ "name": d.name, "kind": d.kind, "builtin": origin == "builtin",
        "origin": origin, "params": d.params });
    if source {
        v["wgsl"] = Value::from(d.wgsl.clone());
    }
    v
}

fn effect_list(i: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let source = a.opt_bool("include_source")?.unwrap_or(false);
    let mut effects: Vec<Value> = builtins()
        .iter()
        .map(|d| def_json(d, "builtin", source))
        .collect();
    let mut in_project = Vec::new();
    if a.get("project_id").is_some() {
        let mut st = lock(&i.state);
        let e = entry(&mut st, a.str("project_id")?)?;
        for d in &e.doc.project().effects {
            effects.push(def_json(d, "project", source));
            in_project.push(d.name.clone());
        }
    }
    if let Some(dir) = &i.library {
        effects.extend(
            lib_all(dir)
                .iter()
                .filter(|d| !in_project.contains(&d.name))
                .map(|d| def_json(d, "library", source)),
        );
    }
    ok(json!({ "effects": effects }))
}

fn effect_check(_: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let kind = match a.str("kind")? {
        "filter" => EffectKind::Filter,
        "transition" => EffectKind::Transition,
        o => {
            return Err(format!(
                "`kind` must be \"filter\" or \"transition\", got {o:?}"
            ));
        }
    };
    let params: Vec<ParamDef> = match a.get("params") {
        None => Vec::new(),
        Some(v) => serde_json::from_value(v.clone()).map_err(|e| format!("`params`: {e}"))?,
    };
    let def = EffectDef {
        name: "check".into(),
        kind,
        wgsl: a.str("wgsl")?.to_string(),
        params,
    };
    match adit_render::validate_effect(&def) {
        Ok(()) => ok(json!({ "ok": true })),
        Err(e) => ok(
            json!({ "ok": false, "error": if e.is_empty() { "invalid effect".to_string() } else { e } }),
        ),
    }
}

// -------------------------------------------------------------------------------------- seeing

/// The frame at 1 / `divisor` of the sequence size, rounded up (S-5); 1 = sequence size.
pub fn render_raw(i: &Inner, p: &Project, seq: Id, t: Time, divisor: u32) -> R<Frame> {
    let s = p
        .sequence(seq)
        .ok_or_else(|| RenderError::SequenceNotFound(seq).to_string())?;
    let out = (s.width.div_ceil(divisor), s.height.div_ceil(divisor));
    let mut g = lock(&i.renderer);
    if g.is_none() {
        *g = Some(Renderer::new().map_err(|e| e.to_string())?);
    }
    g.as_mut()
        .expect("just set")
        .render_frame_scaled(p, seq, t, &*i.media, out)
        .map_err(|e| e.to_string())
}

fn render(i: &Inner, p: &Project, seq: Id, t: Time) -> R<RgbaImage> {
    let f = render_raw(i, p, seq, t, 1)?;
    RgbaImage::from_raw(f.width, f.height, f.rgba)
        .ok_or_else(|| "the renderer returned a malformed frame".into())
}

/// Size with the longer side equal to `target` (up or down), at least 1 px per side.
fn scale_to(w: u32, h: u32, target: u32) -> (u32, u32) {
    let l = u64::from(w.max(h)).max(1);
    let s = |d: u32| ((u64::from(d) * u64::from(target) * 2 + l) / (2 * l)).max(1) as u32;
    (s(w), s(h))
}

fn resize(img: RgbaImage, w: u32, h: u32) -> RgbaImage {
    if img.dimensions() == (w, h) {
        img
    } else {
        image::imageops::resize(&img, w, h, image::imageops::FilterType::Triangle)
    }
}

fn encode(img: &RgbaImage, jpeg: bool) -> R<Image> {
    let mut bytes = Vec::new();
    let res = if jpeg {
        let rgb = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 90).encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )
    } else {
        img.write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
    };
    res.map_err(|e| format!("image encoding failed: {e}"))?;
    Ok(Image {
        mime: if jpeg { "image/jpeg" } else { "image/png" }.into(),
        bytes,
    })
}

fn render_frame(i: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let s = snap(i, a)?;
    let t = a.time("time", Some(s.fps))?.unwrap_or(0);
    if t < 0 {
        return Err("`time` must be >= 0".into());
    }
    let max = a.opt_u32("max_size")?.unwrap_or(640);
    if !(1..=16384).contains(&max) {
        return Err("`max_size` must be 1..16384".into());
    }
    let jpeg = match a.opt_str("format")?.unwrap_or("jpeg") {
        "jpeg" => true,
        "png" => false,
        o => return Err(format!("`format` must be \"jpeg\" or \"png\", got {o:?}")),
    };
    let img = render(i, &s.project, s.seq, t)?;
    let (w, h) = if img.width().max(img.height()) <= max {
        img.dimensions()
    } else {
        scale_to(img.width(), img.height(), max)
    };
    let img = resize(img, w, h);
    Ok(ToolOutput {
        json: json!({ "time_s": secs(t), "frame": time_to_frame(t, s.fps), "width": w, "height": h }),
        images: vec![encode(&img, jpeg)?],
    })
}

fn contact_sheet(i: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let s = snap(i, a)?;
    let seq = s.project.sequence(s.seq).ok_or("sequence vanished")?;
    let start = a.time("start", Some(s.fps))?.unwrap_or(0);
    let end = a
        .time("end", Some(s.fps))?
        .unwrap_or_else(|| seq.duration());
    let count = a.opt_u64("count")?.unwrap_or(9);
    let cell = a.opt_u32("cell_size")?.unwrap_or(320);
    let columns = a.opt_u64("columns")?.unwrap_or(3);
    if !(1..=100).contains(&count) || columns == 0 || !(1..=2048).contains(&cell) {
        return Err("need count 1..100, columns >= 1, cell_size 1..2048".into());
    }
    if start < 0 || end <= start {
        return Err("need 0 <= start < end (is the sequence empty?)".into());
    }
    let columns = columns.min(count) as u32;
    let rows = (count as u32).div_ceil(columns);
    let (cw, ch) = scale_to(seq.width, seq.height, cell);
    // H-11: checked before any rendering. cw, ch <= 2048 and columns, rows <= 100: no overflow.
    let pixels = u64::from(cw * columns) * u64::from(ch * rows);
    if pixels > 64_000_000 {
        return Err(format!(
            "the sheet would be {}x{} = {pixels} pixels, at most 64000000; lower count or cell_size",
            cw * columns,
            ch * rows
        ));
    }
    let mut sheet = RgbaImage::new(cw * columns, ch * rows);
    let mut times = Vec::new();
    for k in 0..count {
        // start + (k + 0.5) * (end - start) / count, exact in i128.
        let t =
            start + ((2 * k as i128 + 1) * i128::from(end - start) / (2 * count as i128)) as Time;
        let img = resize(render(i, &s.project, s.seq, t)?, cw, ch);
        let (col, row) = (k as u32 % columns, k as u32 / columns);
        image::imageops::replace(&mut sheet, &img, i64::from(col * cw), i64::from(row * ch));
        times.push(secs(t));
    }
    Ok(ToolOutput {
        json: json!({ "times_s": times, "columns": columns, "cell_width": cw, "cell_height": ch }),
        images: vec![encode(&sheet, false)?],
    })
}

// ---------------------------------------------------------------------------------------- export

const MAX_RUNNING_JOBS: usize = 4;

/// `p` for comparing file identity: canonical as far as it exists (the file, else its directory),
/// case-folded on Windows.
fn file_key(p: &str) -> std::path::PathBuf {
    let p = Path::new(p);
    let full = std::fs::canonicalize(p)
        .or_else(|_| {
            let dir = p.parent().filter(|d| !d.as_os_str().is_empty());
            std::fs::canonicalize(dir.unwrap_or(Path::new(".")))
                .map(|d| d.join(p.file_name().unwrap_or_default()))
        })
        .unwrap_or_else(|_| p.to_path_buf());
    if cfg!(windows) {
        full.to_string_lossy().to_lowercase().into()
    } else {
        full
    }
}

fn export(i: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let s = snap(i, a)?;
    let seq = s.project.sequence(s.seq).ok_or("sequence vanished")?;
    let path = a.str("path")?;
    if path.is_empty() {
        return Err("`path` must not be empty".into());
    }
    if seq.duration() <= 0 {
        return Err("the sequence is empty: nothing to export".into());
    }
    // H-12: a successful export replaces the target, so it must not be a source file.
    let key = file_key(path);
    if let Some(m) = s
        .project
        .media
        .iter()
        .find(|m| file_key(&m.spec.path) == key)
    {
        return Err(format!(
            "cannot export to {path}: it is the file of media {} ({})",
            m.id, m.spec.name
        ));
    }
    if let Some(dir) = Path::new(path)
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        && !dir.is_dir()
    {
        return Err(format!(
            "cannot write {path}: {} is not a directory",
            dir.display()
        ));
    }
    let video = match a.opt_str("codec")?.unwrap_or("h264") {
        "h264" => VideoCodec::H264,
        "h265" => VideoCodec::H265,
        "prores" => VideoCodec::Prores,
        "gif" => VideoCodec::Gif,
        o => {
            return Err(format!(
                "`codec` must be h264, h265, prores or gif, got {o:?}"
            ));
        }
    };
    let default_audio = if video == VideoCodec::Prores {
        "pcm"
    } else {
        "aac"
    };
    let audio = match a.opt_str("audio")?.unwrap_or(default_audio) {
        _ if video == VideoCodec::Gif => AudioCodec::None,
        "aac" => AudioCodec::Aac,
        "pcm" => AudioCodec::Pcm,
        "none" => AudioCodec::None,
        o => return Err(format!("`audio` must be aac, pcm or none, got {o:?}")),
    };
    let crf = match a.opt_u64("crf")? {
        Some(c) if c <= 51 => Some(c as u8),
        Some(_) => return Err("`crf` must be 0..51".into()),
        None => None,
    };
    let settings = ExportSettings {
        path: path.to_string(),
        width: a.opt_u32("width")?.unwrap_or(seq.width),
        height: a.opt_u32("height")?.unwrap_or(seq.height),
        fps: a.opt_fps("fps")?.unwrap_or(seq.fps),
        sample_rate: seq.sample_rate,
        video,
        audio,
        crf,
    };
    if settings.width == 0 || settings.height == 0 {
        return Err("`width` and `height` must be > 0".into());
    }
    let (done, total, cancel) = (
        Arc::new(AtomicU64::new(0)),
        Arc::new(AtomicU64::new(0)),
        Arc::new(AtomicBool::new(false)),
    );
    let job_id = {
        let mut st = lock(&i.state);
        // H-13: each running export holds an encoder and a renderer.
        let running = st
            .jobs
            .iter()
            .filter(|(_, j)| j.state == JobState::Running)
            .count();
        if running >= MAX_RUNNING_JOBS {
            return Err(format!(
                "{running} exports are already running, at most {MAX_RUNNING_JOBS}; wait for one or cancel it"
            ));
        }
        let id = format!("j{}", st.jobs.len() + 1);
        st.jobs.push((
            id.clone(),
            Job {
                state: JobState::Running,
                error: None,
                path: path.to_string(),
                done: done.clone(),
                total: total.clone(),
                cancel: cancel.clone(),
            },
        ));
        id
    };
    let (inner, id, project, seq_id) = (i.clone(), job_id.clone(), s.project, s.seq);
    std::thread::spawn(move || {
        let run = catch_unwind(AssertUnwindSafe(|| -> Result<(), RenderError> {
            let mut r = Renderer::new()?;
            adit_render::export(
                &mut r,
                &project,
                seq_id,
                &settings,
                &*inner.media,
                &mut |d, t| {
                    done.store(d, Ordering::Relaxed);
                    total.store(t, Ordering::Relaxed);
                },
                &cancel,
            )
        }));
        let (state, error) = match run {
            Ok(Ok(())) => (JobState::Done, None),
            Ok(Err(RenderError::Cancelled)) => (JobState::Cancelled, None),
            Ok(Err(e)) => (JobState::Failed, Some(e.to_string())),
            Err(p) => (
                JobState::Failed,
                Some(format!("internal error: {}", panic_msg(&*p))),
            ),
        };
        if let Some((_, j)) = lock(&inner.state).jobs.iter_mut().find(|(k, _)| *k == id) {
            j.state = state;
            j.error = error;
        }
        inner.cv.notify_all();
    });
    ok(json!({ "job_id": job_id }))
}

fn job_json(id: &str, j: &Job) -> Value {
    let state = match j.state {
        JobState::Running => "running",
        JobState::Done => "done",
        JobState::Failed => "failed",
        JobState::Cancelled => "cancelled",
    };
    let mut v = json!({
        "job_id": id, "state": state, "path": j.path,
        "frames_done": j.done.load(Ordering::Relaxed),
        "frames_total": j.total.load(Ordering::Relaxed),
    });
    if let Some(e) = &j.error {
        v["error"] = Value::from(e.clone());
    }
    v
}

fn find_job<'a>(st: &'a State, a: A) -> R<(&'a str, &'a Job)> {
    let id = a.str("job_id")?;
    st.jobs
        .iter()
        .find(|(k, _)| k == id)
        .map(|(k, j)| (k.as_str(), j))
        .ok_or_else(|| format!("unknown job_id {id:?}"))
}

fn job_status(i: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let st = lock(&i.state);
    let (id, j) = find_job(&st, a)?;
    ok(job_json(id, j))
}

fn job_cancel(i: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let st = lock(&i.state);
    let (id, j) = find_job(&st, a)?;
    j.cancel.store(true, Ordering::Relaxed);
    ok(job_json(id, j))
}

fn job_wait(i: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let t = a.opt_f64("timeout_s")?.unwrap_or(60.0);
    if t < 0.0 {
        return Err("`timeout_s` must be >= 0".into());
    }
    let deadline = Instant::now() + Duration::from_secs_f64(t.min(3600.0));
    let mut st = lock(&i.state);
    loop {
        let (id, j) = find_job(&st, a)?;
        let left = deadline.saturating_duration_since(Instant::now());
        if j.state != JobState::Running || left.is_zero() {
            return ok(job_json(id, j));
        }
        st =
            i.cv.wait_timeout(st, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
    }
}

// -------------------------------------------------------------------------------------- requests

fn request_json(r: &Request) -> Value {
    let mut v = json!({
        "request_id": r.id, "project_id": r.project_id, "sequence_id": r.sequence_id,
        "text": r.text, "status": r.status, "message": r.message,
        "start_s": r.start.map(secs), "end_s": r.end.map(secs), "start": r.start, "end": r.end,
        "clip_ids": r.clip_ids, "rect": r.rect,
        "scope": r.scope, "panel": r.panel, "paths": r.paths,
    });
    if r.scope == "app" {
        // T-REQ-4: the source tree this binary was built from, if it is still there.
        v["source_dir"] = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .filter(|p| p.is_dir())
            .map(|p| p.to_string_lossy().into_owned())
            .into();
    }
    v
}

fn request_create(i: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let text = a.str("text")?.to_string();
    let clip_ids = a.ids("clip_ids")?;
    let rect = match a.array("rect")? {
        None => None,
        Some(v) => {
            let n: Vec<f64> = v.iter().filter_map(Value::as_f64).collect();
            match n[..] {
                [x, y, w, h] if v.len() == 4 && n.iter().all(|c| (0.0..=1.0).contains(c)) => {
                    Some([x, y, w, h])
                }
                _ => return Err("`rect` must be [x, y, w, h], four numbers in 0..1".into()),
            }
        }
    };
    let scope = match a.opt_str("scope")?.unwrap_or("edit") {
        "edit" => "edit",
        "app" => "app",
        "import" => "import",
        o => {
            return Err(format!(
                "`scope` must be \"edit\", \"app\" or \"import\", got {o:?}"
            ));
        }
    };
    let panel = a.opt_str("panel")?.map(str::to_string);
    let paths = a
        .array("paths")?
        .into_iter()
        .flatten()
        .map(|v| v.as_str().map(str::to_string))
        .collect::<Option<Vec<_>>>()
        .ok_or("`paths` must be an array of strings")?;
    let mut st = lock(&i.state);
    let e = entry(&mut st, a.str("project_id")?)?;
    let sq = resolve_seq(e.doc.project(), a)?;
    let (sequence_id, fps) = (sq.id, Some(sq.fps));
    let (start, end) = (a.time("start", fps)?, a.time("end", fps)?);
    let project_id = e.id.clone();
    let id = format!("r{}", st.requests.len() + 1);
    st.requests.push(Request {
        id: id.clone(),
        project_id,
        sequence_id,
        text,
        start,
        end,
        clip_ids,
        rect,
        scope,
        panel,
        paths,
        status: "pending",
        message: String::new(),
    });
    i.cv.notify_all();
    ok(json!({ "request_id": id }))
}

fn request_wait(i: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let t = a.opt_f64("timeout_s")?.unwrap_or(60.0);
    if t < 0.0 {
        return Err("`timeout_s` must be >= 0".into());
    }
    let deadline = Instant::now() + Duration::from_secs_f64(t.min(300.0));
    let mut st = lock(&i.state);
    loop {
        if let Some(r) = st.requests.iter_mut().find(|r| r.status == "pending") {
            r.status = "claimed";
            return ok(json!({ "request": request_json(r) }));
        }
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return ok(json!({ "request": null }));
        }
        st =
            i.cv.wait_timeout(st, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
    }
}

fn request_list(i: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let status = a.opt_str("status")?;
    if let Some(s) = status
        && !["pending", "claimed", "done"].contains(&s)
    {
        return Err(format!(
            "`status` must be pending, claimed or done, got {s:?}"
        ));
    }
    let st = lock(&i.state);
    ok(json!({ "requests": st.requests.iter()
        .filter(|r| status.is_none_or(|s| r.status == s))
        .map(request_json).collect::<Vec<_>>() }))
}

fn request_complete(i: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let id = a.str("request_id")?;
    let message = a.opt_str("message")?.unwrap_or("").to_string();
    let mut st = lock(&i.state);
    let r = st
        .requests
        .iter_mut()
        .find(|r| r.id == id)
        .ok_or_else(|| format!("unknown request_id {id:?}"))?;
    r.status = "done";
    r.message = message;
    ok(json!({ "request_id": id }))
}

// ------------------------------------------------------------------------- in-process access

pub fn projects(i: &Inner) -> Vec<ProjectInfo> {
    lock(&i.state)
        .projects
        .iter()
        .map(|e| ProjectInfo {
            id: e.id.clone(),
            name: e.doc.project().name.clone(),
            path: e.path.clone(),
            dirty: e.dirty,
            revision: e.revision,
        })
        .collect()
}

pub fn revision(i: &Inner, id: &str) -> Option<u64> {
    lock(&i.state)
        .projects
        .iter()
        .find(|e| e.id == id)
        .map(|e| e.revision)
}

pub fn snapshot(i: &Inner, id: &str) -> Option<(Project, u64)> {
    let st = lock(&i.state);
    let e = st.projects.iter().find(|e| e.id == id)?;
    Some((e.doc.project().clone(), e.revision))
}

pub fn mix(i: &Inner, p: &Project, seq: Id, start: Time, duration: Time) -> R<Vec<f32>> {
    adit_render::mix_audio(p, seq, start, duration, &*i.media).map_err(|e| e.to_string())
}

pub fn source_frame(i: &Inner, path: &str, t: Time) -> R<Arc<Frame>> {
    i.media.video_frame(path, t).map_err(|e| e.to_string())
}

/// S-4: 50 peaks per second, the largest |sample| of either channel per bucket.
pub fn waveform(i: &Inner, path: &str) -> R<Arc<Vec<f32>>> {
    if let Some(w) = lock(&i.waveforms).get(path) {
        return Ok(w.clone());
    }
    // 8 kHz is plenty for a picture of the sound and keeps a long file small.
    const RATE: u32 = 8000;
    const CHUNK: Time = 60 * FLICKS_PER_SECOND;
    let spec = i.media.probe(path).map_err(|e| e.to_string())?;
    if spec.audio.is_none() {
        return Err(format!("no audio in {path}"));
    }
    let total = spec
        .duration
        .filter(|d| *d > 0)
        .ok_or_else(|| format!("no duration for {path}"))?;
    let mut peaks = Vec::new();
    let mut start = 0;
    // shortcut: buckets restart at every 60 s chunk, so a backend that returns a few samples more
    // or less than asked shifts later peaks slightly; carry the remainder over if that ever shows.
    while start < total {
        let len = CHUNK.min(total - start);
        let pcm = i
            .media
            .audio(path, start, len, RATE, "")
            .map_err(|e| e.to_string())?;
        // Interleaved stereo: one bucket = RATE / 50 frames = 2 * RATE / 50 samples.
        peaks.extend(
            pcm.chunks(2 * RATE as usize / 50)
                .map(|b| b.iter().fold(0f32, |m, s| m.max(s.abs()))),
        );
        start += len;
    }
    if peaks.is_empty() {
        return Err(format!("no audio in {path}"));
    }
    // An `Err` above is not cached (T-LIB-6).
    Ok(lock(&i.waveforms)
        .entry(path.to_string())
        .or_insert_with(|| Arc::new(peaks))
        .clone())
}

pub fn set_ui(i: &Inner, ui: Option<Arc<dyn UiBridge>>) {
    *lock(&i.ui) = ui;
}

// ------------------------------------------------------------------------------------------ ui

/// The attached window. The lock is released before the caller uses it: the window calls back into
/// the session.
fn bridge(i: &Inner) -> R<Arc<dyn UiBridge>> {
    lock(&i.ui)
        .clone()
        .ok_or_else(|| "no window is attached to this session".to_string())
}

fn ui_screenshot(i: &Arc<Inner>, _: A) -> R<ToolOutput> {
    Ok(ToolOutput {
        json: json!({}),
        images: vec![bridge(i)?.screenshot()?],
    })
}

fn ui_get(i: &Arc<Inner>, _: A) -> R<ToolOutput> {
    ok(bridge(i)?.get()?)
}

/// Frame rate for frame strings: the sequence the window shows (of `project_id` if the call names
/// one, else the window's project); `None` when it cannot be found.
fn window_fps(i: &Inner, ui: &dyn UiBridge, a: A) -> Option<Rational> {
    let at = ui.get().ok()?;
    let pid = match a.opt_str("project_id") {
        Ok(Some(p)) => p.to_string(),
        _ => at.get("project_id")?.as_str()?.to_string(),
    };
    let mut st = lock(&i.state);
    let p = entry(&mut st, &pid).ok()?.doc.project();
    let window_seq = (at.get("project_id")?.as_str() == Some(&pid))
        .then(|| at.get("sequence_id")?.as_u64())
        .flatten();
    window_seq
        .and_then(|id| p.sequence(id))
        .or(p.sequences.first())
        .map(|s| s.fps)
}

fn ui_set(i: &Arc<Inner>, a: A) -> R<ToolOutput> {
    let ui = bridge(i)?;
    let mut state = a.0.clone();
    let times = ["playhead", "mark_in", "mark_out"];
    // Only frame strings need the frame rate; asking the window for it is not free.
    let fps = if times.iter().any(|k| a.get(k).is_some_and(Value::is_string)) {
        window_fps(i, &*ui, a)
    } else {
        None
    };
    for k in times {
        if let Some(v) = a.get(k) {
            state[k] = Value::from(parse_time(k, v, fps)?);
        }
    }
    ui.set(&state)?;
    ok(json!({}))
}

// ------------------------------------------------------------------------------------------ table

type Run = fn(&Arc<Inner>, A) -> R<ToolOutput>;

/// `(name, type, description)`; a leading `*` on the name marks it required.
type Prop = (&'static str, &'static str, &'static str);

struct Tool {
    name: &'static str,
    description: &'static str,
    props: &'static [Prop],
    run: Run,
}

const SEQ: Prop = (
    "sequence_id",
    "integer",
    "Sequence id; default: the project's first sequence.",
);
const PID: Prop = (
    "*project_id",
    "string",
    "Project id from project_new / project_open.",
);

static TOOLS: &[Tool] = &[
    Tool {
        name: "project_new",
        description: "Create a project with one sequence and open it. Not undoable. Returns {project_id, sequence_id, track_ids} (tracks in order V1.., A1..).",
        props: &[
            ("*name", "string", "Project name."),
            ("width", "integer", "Default 1920."),
            ("height", "integer", "Default 1080."),
            (
                "fps",
                "string",
                "\"30\" (default), \"29.97\", \"23.976\", \"59.94\" or \"num/den\".",
            ),
            ("sample_rate", "integer", "Default 48000."),
            ("video_tracks", "integer", "Default 2."),
            ("audio_tracks", "integer", "Default 2."),
        ],
        run: project_new,
    },
    Tool {
        name: "project_open",
        description: "Open an .adit project file as a new project. Returns {project_id}.",
        props: &[("*path", "string", "Path of the .adit file.")],
        run: project_open,
    },
    Tool {
        name: "project_save",
        description: "Save a project as pretty-printed JSON, atomically. `path` is remembered; required the first time.",
        props: &[
            PID,
            (
                "path",
                "string",
                "Target file; default: the remembered path.",
            ),
        ],
        run: project_save,
    },
    Tool {
        name: "project_close",
        description: "Close a project and drop its undo history.",
        props: &[PID],
        run: project_close,
    },
    Tool {
        name: "project_list",
        description: "List open projects: [{project_id, name, path, dirty}].",
        props: &[],
        run: project_list,
    },
    Tool {
        name: "project_get",
        description: "Inspect a project. detail \"summary\" (default): media, sequences, tracks, clips, transitions, markers with times in seconds and frames. detail \"full\": the complete project JSON plus undo/redo depth.",
        props: &[
            PID,
            ("detail", "string", "\"summary\" (default) or \"full\"."),
        ],
        run: project_get,
    },
    Tool {
        name: "media_import",
        description: "Probe media files and add them to the project (one undo step; any failure adds nothing). Returns {media: [{id, name, duration_s, video, audio}]}. Relative paths resolve against the server's working directory.",
        props: &[PID, ("*paths", "strings", "Files to import.")],
        run: media_import,
    },
    Tool {
        name: "detect_silence",
        description: "Find silent stretches of a media file. Returns {ranges: [{start_s, end_s}]}.",
        props: &[
            PID,
            ("*media_id", "integer", "Media id."),
            (
                "noise_db",
                "number",
                "Silence threshold in dB; default -35.",
            ),
            (
                "min_duration",
                "time",
                "Shortest silence to report; default \"0.4s\".",
            ),
        ],
        run: detect_silence,
    },
    Tool {
        name: "detect_scenes",
        description: "Find scene cuts in a video file. Returns {cuts_s: [..]}.",
        props: &[
            PID,
            ("*media_id", "integer", "Media id."),
            (
                "threshold",
                "number",
                "Scene-change score 0..1; default 0.3.",
            ),
        ],
        run: detect_scenes,
    },
    Tool {
        name: "audio_levels",
        description: "Loudness of the sequence's audio mix per bucket. Returns {bucket_s, rms_db, peak_db} (dBFS, -120 = silence).",
        props: &[
            PID,
            SEQ,
            ("start", "time", "Default 0."),
            ("end", "time", "Default: end of the sequence."),
            ("buckets", "integer", "Default 50."),
        ],
        run: audio_levels,
    },
    Tool {
        name: "edit",
        description: "",
        props: &[
            PID,
            (
                "sequence_id",
                "integer",
                "Sequence whose frame rate frame strings (\"45f\") use; default: the first sequence.",
            ),
            ("*commands", "objects", "Commands, see the description."),
        ],
        run: edit,
    },
    Tool {
        name: "undo",
        description: "Undo edits. Returns {done, undo_depth, redo_depth}; done = steps actually undone.",
        props: &[PID, ("steps", "integer", "Default 1.")],
        run: |i, a| step(i, a, true),
    },
    Tool {
        name: "redo",
        description: "Redo undone edits. Returns {done, undo_depth, redo_depth}.",
        props: &[PID, ("steps", "integer", "Default 1.")],
        run: |i, a| step(i, a, false),
    },
    Tool {
        name: "effect_list",
        description: "List built-in effects and (with project_id) the project's own: [{name, kind filter|transition, builtin, params[{name,default,min,max}], wgsl?}].",
        props: &[
            (
                "project_id",
                "string",
                "Also list this project's defined effects.",
            ),
            (
                "include_source",
                "boolean",
                "Include each effect's WGSL; default false.",
            ),
        ],
        run: effect_list,
    },
    Tool {
        name: "effect_check",
        description: "Compile an effect shader without touching any project. Returns {ok: true} or {ok: false, error}.",
        props: &[
            ("*kind", "string", "\"filter\" or \"transition\"."),
            (
                "*wgsl",
                "string",
                "Shader source following the contract in the server instructions.",
            ),
            ("params", "objects", "[{name, default, min, max}]"),
        ],
        run: effect_check,
    },
    Tool {
        name: "render_frame",
        description: "Render one composited frame and return it as an image, plus {time_s, frame, width, height}. Scaled down (never up) so the longer side is at most max_size.",
        props: &[
            PID,
            SEQ,
            ("time", "time", "Default 0."),
            ("max_size", "integer", "Default 640."),
            ("format", "string", "\"jpeg\" (default) or \"png\"."),
        ],
        run: render_frame,
    },
    Tool {
        name: "render_contact_sheet",
        description: "One PNG with `count` frames at evenly spaced times in [start, end), row by row. Returns the image plus {times_s, columns, cell_width, cell_height}.",
        props: &[
            PID,
            SEQ,
            ("start", "time", "Default 0."),
            ("end", "time", "Default: end of the sequence."),
            ("count", "integer", "Default 9."),
            ("columns", "integer", "Default 3."),
            (
                "cell_size",
                "integer",
                "Longer side of each cell in pixels; default 320.",
            ),
        ],
        run: contact_sheet,
    },
    Tool {
        name: "export",
        description: "Start a background export of a snapshot of the sequence; later edits do not affect it. Returns {job_id}; follow with job_wait / job_status.",
        props: &[
            PID,
            SEQ,
            (
                "*path",
                "string",
                "Output file; the container follows the extension.",
            ),
            ("codec", "string", "h264 (default), h265, prores or gif."),
            ("audio", "string", "aac (default), pcm or none."),
            ("width", "integer", "Default: sequence width."),
            ("height", "integer", "Default: sequence height."),
            (
                "fps",
                "string",
                "Same forms as project_new; default: sequence rate.",
            ),
            (
                "crf",
                "integer",
                "Quality 0..51 for h264/h265, lower is better; default 18.",
            ),
        ],
        run: export,
    },
    Tool {
        name: "job_status",
        description: "Status of an export job: {state running|done|failed|cancelled, frames_done, frames_total, error?, path}.",
        props: &[("*job_id", "string", "From export.")],
        run: job_status,
    },
    Tool {
        name: "job_cancel",
        description: "Ask an export job to stop; it ends `cancelled` and its partial file is removed.",
        props: &[("*job_id", "string", "From export.")],
        run: job_cancel,
    },
    Tool {
        name: "job_wait",
        description: "Block until the job leaves `running` or the timeout passes; returns the job status.",
        props: &[
            ("*job_id", "string", "From export."),
            ("timeout_s", "number", "Default 60."),
        ],
        run: job_wait,
    },
    Tool {
        name: "request_create",
        description: "File a magic-wand request: the user pointing at something and saying what they want. Returns {request_id}. Status starts `pending`.",
        props: &[
            PID,
            SEQ,
            ("*text", "string", "What the user wants."),
            ("start", "time", "Start of the pointed-at range."),
            ("end", "time", "End of the pointed-at range."),
            ("clip_ids", "integers", "Clips pointed at."),
            (
                "rect",
                "numbers",
                "[x, y, w, h] in 0..1 of the frame; of the window for scope app.",
            ),
            (
                "scope",
                "string",
                "edit (default): change the project; app: change adit itself; import: bring the files in `paths` into the project as `text` says.",
            ),
            ("panel", "string", "Name of the window panel pointed at."),
            ("paths", "strings", "Files for scope import."),
        ],
        run: request_create,
    },
    Tool {
        name: "request_wait",
        description: "Block until a pending request exists (or the timeout passes), claim the oldest one and return {request: {request_id, project_id, sequence_id, text, start_s, end_s, start, end, clip_ids, rect, scope, panel, paths, source_dir (scope app only)}} or {request: null}. The server instructions say what each scope asks for. Answer with request_complete.",
        props: &[("timeout_s", "number", "Default 60, at most 300.")],
        run: request_wait,
    },
    Tool {
        name: "request_list",
        description: "List requests with their status: {requests: [...]}.",
        props: &[(
            "status",
            "string",
            "Only this status: pending, claimed or done.",
        )],
        run: request_list,
    },
    Tool {
        name: "request_complete",
        description: "Mark a request done with a message shown to the user.",
        props: &[
            ("*request_id", "string", "From request_wait / request_list."),
            ("message", "string", "What you did; default empty."),
        ],
        run: request_complete,
    },
    Tool {
        name: "ui_screenshot",
        description: "Screenshot of the editor window as a PNG image. Errors when no window is attached (headless server).",
        props: &[],
        run: ui_screenshot,
    },
    Tool {
        name: "ui_get",
        description: "State of the editor window: {project_id, sequence_id, playhead, playhead_s, selection, mark_in, mark_out, tool, playing}; times in flicks. Errors when no window is attached.",
        props: &[],
        run: ui_get,
    },
    Tool {
        name: "ui_set",
        description: "Change the editor window's state; give any of the fields. Frame strings (\"45f\") use the frame rate of the window's current sequence (or of project_id's). Errors when no window is attached.",
        props: &[
            ("project_id", "string", "Switch the window to this project."),
            ("playhead", "time", "Playhead position."),
            ("selection", "integers", "Selected clip ids."),
            ("mark_in", "time", "In point."),
            ("mark_out", "time", "Out point."),
            (
                "tool",
                "string",
                "Tool name in lower snake case: select, razor, ripple, ...",
            ),
        ],
        run: ui_set,
    },
];

fn schema(props: &[Prop]) -> Value {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();
    for (name, ty, desc) in props {
        let name: &str = match name.strip_prefix('*') {
            Some(n) => {
                required.push(n);
                n
            }
            None => name,
        };
        let mut p = match *ty {
            "time" => json!({ "type": ["integer", "string"] }),
            "objects" => json!({ "type": "array", "items": { "type": "object" } }),
            "strings" => json!({ "type": "array", "items": { "type": "string" } }),
            "integers" => json!({ "type": "array", "items": { "type": "integer" } }),
            "numbers" => json!({ "type": "array", "items": { "type": "number" } }),
            t => json!({ "type": t }),
        };
        p["description"] = Value::from(*desc);
        properties.insert(name.to_string(), p);
    }
    json!({ "type": "object", "properties": properties, "required": required })
}

pub fn list() -> Vec<ToolInfo> {
    TOOLS
        .iter()
        .map(|t| ToolInfo {
            name: t.name,
            description: if t.name == "edit" {
                edit_description()
            } else {
                t.description.to_string()
            },
            input_schema: schema(t.props),
        })
        .collect()
}

pub fn exists(name: &str) -> bool {
    TOOLS.iter().any(|t| t.name == name)
}

pub fn panic_msg(p: &(dyn Any + Send)) -> String {
    p.downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| p.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".into())
}

pub fn call(i: &Arc<Inner>, tool: &str, args: Value) -> R<ToolOutput> {
    let t = TOOLS
        .iter()
        .find(|t| t.name == tool)
        .ok_or_else(|| format!("unknown tool `{tool}`"))?;
    let args = match args {
        Value::Null => json!({}),
        v @ Value::Object(_) => v,
        _ => return Err("tool arguments must be a JSON object".into()),
    };
    // A panic in a lower crate must not take the server down.
    catch_unwind(AssertUnwindSafe(|| (t.run)(i, A(&args))))
        .unwrap_or_else(|p| Err(format!("internal error in `{tool}`: {}", panic_msg(&*p))))
}
