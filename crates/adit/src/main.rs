//! `adit` binary: `adit mcp`, `adit serve`, `adit gui`, `adit render`. Contract: docs/MCP_TOOLS.md "Binary", docs/GUI.md "Binary".

use adit_core::{FLICKS_PER_SECOND, Project, Rational};
use adit_mcp::Session;
use adit_media::{AudioCodec, ExportSettings, Ffmpeg, MediaBackend, Pattern, VideoCodec};
use adit_render::Renderer;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

const USAGE: &str = "usage:
  adit mcp                         MCP server on stdio
  adit serve [--port N]            MCP server over HTTP on 127.0.0.1:N (default 7821); token from env ADIT_TOKEN or random
  adit gui [project.adit] [--port N] editor window; also serves MCP over HTTP like `adit serve`
  adit render <project.adit> -o <out> [--sequence ID] [--codec h264|h265|prores|gif]
  adit --help
env: ADIT_MEDIA=pattern uses a procedural 64x64 30 fps 10 s media backend instead of ffmpeg";

/// Prints `msg` (if any) and the usage to stderr; exit code 2.
fn usage(msg: &str) -> ExitCode {
    if !msg.is_empty() {
        eprintln!("adit: {msg}");
    }
    eprintln!("{USAGE}");
    ExitCode::from(2)
}

fn fail(msg: impl std::fmt::Display) -> ExitCode {
    eprintln!("adit: {msg}");
    ExitCode::FAILURE
}

fn backend() -> Result<Arc<dyn MediaBackend>, String> {
    if std::env::var("ADIT_MEDIA").is_ok_and(|v| v == "pattern") {
        return Ok(Arc::new(Pattern {
            width: 64,
            height: 64,
            fps: Rational { num: 30, den: 1 },
            duration: Some(10 * FLICKS_PER_SECOND),
        }));
    }
    Ok(Arc::new(Ffmpeg::new().map_err(|e| e.to_string())?))
}

fn token() -> Result<String, String> {
    if let Ok(t) = std::env::var("ADIT_TOKEN")
        && !t.is_empty()
    {
        return Ok(t);
    }
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).map_err(|e| format!("no system randomness: {e}"))?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args[..] {
        [] => usage("a command is required"),
        ["--help" | "-h" | "help"] => {
            eprintln!("{USAGE}");
            ExitCode::SUCCESS
        }
        ["mcp"] => run_server(|s| adit_mcp::serve_stdio(s).map_err(|e| e.to_string())),
        ["serve", ref rest @ ..] => {
            let port = match rest {
                [] => 7821,
                ["--port", p] => match p.parse::<u16>() {
                    Ok(p) => p,
                    Err(_) => return usage(&format!("invalid port {p:?}")),
                },
                _ => return usage("serve takes only --port N"),
            };
            run_server(|s| {
                let token = token()?;
                adit_mcp::serve_http(s, port, token).map_err(|e| e.to_string())
            })
        }
        ["gui", ref rest @ ..] => gui(rest),
        ["render", ref rest @ ..] => render(rest),
        [other, ..] => usage(&format!("unknown command {other:?}")),
    }
}

fn run_server(f: impl FnOnce(Session) -> Result<(), String>) -> ExitCode {
    match backend().and_then(|b| f(Session::new(b))) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail(e),
    }
}

/// `adit gui [project.adit] [--port N]` (docs/GUI.md "Binary").
fn gui(args: &[&str]) -> ExitCode {
    let (mut open, mut port) = (None, 7821u16);
    let mut it = args.iter();
    while let Some(&a) = it.next() {
        match a {
            "--port" => match it.next().map(|p| p.parse::<u16>()) {
                // Port 0 would bind a random port the user cannot know (U-FIX-13).
                Some(Ok(p)) if p != 0 => port = p,
                Some(_) => return usage("invalid port"),
                None => return usage("--port needs a value"),
            },
            _ if a.starts_with('-') => return usage(&format!("unknown option {a:?}")),
            _ if open.is_none() => open = Some(a.to_string()),
            _ => return usage(&format!("unexpected argument {a:?}")),
        }
    }
    let session = match backend() {
        Ok(b) => Session::new(b),
        Err(e) => return fail(e),
    };
    let token = match token() {
        Ok(t) => t,
        Err(e) => return fail(e),
    };
    // serve_http returns only on a bind failure; no answer within the wait means it is serving.
    let (tx, rx) = std::sync::mpsc::channel();
    let (s2, t2) = (session.clone(), token.clone());
    std::thread::spawn(move || {
        let _ = tx.send(adit_mcp::serve_http(s2, port, t2));
    });
    let mcp_url = match rx.recv_timeout(std::time::Duration::from_millis(500)) {
        Ok(Err(e)) => {
            eprintln!("adit: MCP is off: cannot serve on port {port}: {e}");
            None
        }
        _ => Some(format!("http://127.0.0.1:{port}/mcp")),
    };
    let options = adit_gui::Options {
        open,
        mcp_token: mcp_url.as_ref().map(|_| token),
        mcp_url,
    };
    match adit_gui::run(session, options) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail(e),
    }
}

fn render(args: &[&str]) -> ExitCode {
    let (mut input, mut out, mut seq, mut codec) = (None, None, None, "h264");
    let mut it = args.iter();
    while let Some(&a) = it.next() {
        let mut value = |what: &str| {
            it.next()
                .copied()
                .ok_or_else(|| format!("{what} needs a value"))
        };
        let r = match a {
            "-o" => value("-o").map(|v| out = Some(v)),
            "--codec" => value("--codec").map(|v| codec = v),
            "--sequence" => value("--sequence").and_then(|v| {
                v.parse::<u64>()
                    .map(|n| seq = Some(n))
                    .map_err(|_| format!("invalid sequence id {v:?}"))
            }),
            _ if a.starts_with('-') => Err(format!("unknown option {a:?}")),
            _ if input.is_none() => {
                input = Some(a);
                Ok(())
            }
            _ => Err(format!("unexpected argument {a:?}")),
        };
        if let Err(e) = r {
            return usage(&e);
        }
    }
    let (Some(input), Some(out)) = (input, out) else {
        return usage("render needs a project file and -o <out>");
    };
    let (video, audio) = match codec {
        "h264" => (VideoCodec::H264, AudioCodec::Aac),
        "h265" => (VideoCodec::H265, AudioCodec::Aac),
        "prores" => (VideoCodec::Prores, AudioCodec::Pcm),
        "gif" => (VideoCodec::Gif, AudioCodec::None),
        c => return usage(&format!("unknown codec {c:?}")),
    };
    match export(input, out, seq, video, audio) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => fail(e),
    }
}

fn export(
    input: &str,
    out: &str,
    seq: Option<u64>,
    video: VideoCodec,
    audio: AudioCodec,
) -> Result<(), String> {
    let text = std::fs::read_to_string(input).map_err(|e| format!("cannot read {input}: {e}"))?;
    let project = Project::from_json(&text).map_err(|e| format!("{input}: {e}"))?;
    project
        .validate()
        .map_err(|e| format!("{input}: invalid project: {e}"))?;
    let s = match seq {
        Some(id) => project
            .sequence(id)
            .ok_or_else(|| format!("sequence {id} not found in {input}"))?,
        None => project
            .sequences
            .first()
            .ok_or_else(|| format!("{input} has no sequence"))?,
    };
    if let Some(dir) = std::path::Path::new(out)
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        && !dir.is_dir()
    {
        return Err(format!(
            "cannot write {out}: {} is not a directory",
            dir.display()
        ));
    }
    let settings = ExportSettings {
        path: out.to_string(),
        width: s.width,
        height: s.height,
        fps: s.fps,
        sample_rate: s.sample_rate,
        video,
        audio,
        crf: None,
    };
    let media = backend()?;
    let mut renderer = Renderer::new().map_err(|e| e.to_string())?;
    let mut last = (0, 0);
    adit_render::export(
        &mut renderer,
        &project,
        s.id,
        &settings,
        &*media,
        &mut |d, t| last = (d, t),
        &AtomicBool::new(false),
    )
    .map_err(|e| e.to_string())?;
    eprintln!("adit: wrote {out} ({} frames)", last.0);
    Ok(())
}
