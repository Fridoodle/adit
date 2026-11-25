//! Black-box helpers for the `adit` binary: a minimal MCP stdio client, a streamable-HTTP client,
//! ffmpeg fixtures and ffprobe checks. Contract: docs/MCP_TOOLS.md "Binary".
#![allow(dead_code)]

use base64::Engine;
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const S: i64 = 705_600_000;

pub fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_adit")
}

/// Kills the child on drop so a failing test never leaves a server behind.
pub struct Proc {
    pub child: Child,
}

impl Drop for Proc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Collects a child's stderr in the background (also keeps the pipe drained).
pub fn drain_stderr(child: &mut Child) -> (Arc<Mutex<Vec<String>>>, Receiver<String>) {
    let err = child.stderr.take().expect("stderr piped");
    let lines = Arc::new(Mutex::new(Vec::new()));
    let (tx, rx) = channel();
    let l2 = lines.clone();
    std::thread::spawn(move || {
        for line in BufReader::new(err).lines() {
            let Ok(line) = line else { break };
            l2.lock().unwrap().push(line.clone());
            let _ = tx.send(line);
        }
    });
    (lines, rx)
}

// ------------------------------------------------------------------ stdio client

pub struct ToolResult {
    pub json: Value,
    pub text: String,
    pub images: Vec<(String, Vec<u8>)>,
    pub raw: Value,
}

pub struct Mcp {
    proc: Proc,
    stdin: Option<ChildStdin>,
    rx: Receiver<Result<Value, String>>,
    next: u64,
    pub stderr: Arc<Mutex<Vec<String>>>,
}

pub enum Media {
    Ffmpeg,
    Pattern,
}

impl Mcp {
    pub fn spawn(media: Media, cwd: Option<&Path>) -> Mcp {
        let mut cmd = Command::new(bin());
        cmd.arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        match media {
            Media::Pattern => cmd.env("ADIT_MEDIA", "pattern"),
            Media::Ffmpeg => cmd.env_remove("ADIT_MEDIA"),
        };
        if let Some(d) = cwd {
            cmd.current_dir(d);
        }
        let mut child = cmd.spawn().expect("spawn adit mcp");
        let stdin = child.stdin.take();
        let out = child.stdout.take().unwrap();
        let (stderr, _rx) = drain_stderr(&mut child);
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            // Every stdout line must be one JSON-RPC 2.0 message (Binary: stdout carries only protocol).
            for line in BufReader::new(out).lines() {
                let Ok(line) = line else { break };
                let msg = match serde_json::from_str::<Value>(&line) {
                    Ok(v) if v.is_object() && v["jsonrpc"] == "2.0" => Ok(v),
                    _ => Err(line),
                };
                if tx.send(msg).is_err() {
                    break;
                }
            }
        });
        Mcp {
            proc: Proc { child },
            stdin,
            rx,
            next: 1,
            stderr,
        }
    }

    /// Spawns and performs the `initialize` handshake; returns the initialize result.
    pub fn start(media: Media, cwd: Option<&Path>) -> (Mcp, Value) {
        let mut m = Mcp::spawn(media, cwd);
        let r = m.initialize();
        (m, r)
    }

    pub fn initialize(&mut self) -> Value {
        let r = self.request(
            "initialize",
            json!({ "protocolVersion": "2025-06-18", "capabilities": {},
                    "clientInfo": { "name": "adit-tests", "version": "0" } }),
        );
        let res = r
            .get("result")
            .unwrap_or_else(|| panic!("initialize failed: {r}"))
            .clone();
        self.notify("notifications/initialized", json!({}));
        res
    }

    pub fn send_line(&mut self, line: &str) {
        let s = self.stdin.as_mut().expect("stdin open");
        s.write_all(line.as_bytes()).unwrap();
        s.write_all(b"\n").unwrap();
        s.flush().unwrap();
    }

    pub fn close_stdin(&mut self) {
        self.stdin = None;
    }

    pub fn notify(&mut self, method: &str, params: Value) {
        let line = json!({ "jsonrpc": "2.0", "method": method, "params": params }).to_string();
        self.send_line(&line);
    }

    pub fn request(&mut self, method: &str, params: Value) -> Value {
        self.request_timeout(method, params, Duration::from_secs(60))
    }

    pub fn request_timeout(&mut self, method: &str, params: Value, timeout: Duration) -> Value {
        let id = self.next;
        self.next += 1;
        let line =
            json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }).to_string();
        self.send_line(&line);
        self.wait_id(json!(id), timeout)
    }

    /// Reads messages until the response with `id`; panics on a non-protocol stdout line.
    pub fn wait_id(&mut self, id: Value, timeout: Duration) -> Value {
        let deadline = Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.rx.recv_timeout(left) {
                Ok(Ok(v)) => {
                    if v.get("id") == Some(&id)
                        && (v.get("result").is_some() || v.get("error").is_some())
                    {
                        return v;
                    }
                }
                Ok(Err(line)) => panic!("stdout carried a non-protocol line: {line:?}"),
                Err(RecvTimeoutError::Timeout) => {
                    panic!("no response to id {id} within {timeout:?}")
                }
                Err(RecvTimeoutError::Disconnected) => {
                    panic!(
                        "server closed stdout; stderr: {:?}",
                        self.stderr.lock().unwrap()
                    )
                }
            }
        }
    }

    /// `tools/call`; `Err(text)` for an `isError` result; panics on a JSON-RPC error.
    pub fn tool(&mut self, name: &str, args: Value) -> Result<ToolResult, String> {
        let r = self.request_timeout(
            "tools/call",
            json!({ "name": name, "arguments": args }),
            Duration::from_secs(180),
        );
        let res = r
            .get("result")
            .unwrap_or_else(|| panic!("tools/call {name}: JSON-RPC error {r}"))
            .clone();
        parse_tool_result(res)
    }

    pub fn ok(&mut self, name: &str, args: Value) -> ToolResult {
        self.tool(name, args.clone())
            .unwrap_or_else(|e| panic!("{name} {args} failed: {e}"))
    }

    pub fn err(&mut self, name: &str, args: Value) -> String {
        match self.tool(name, args.clone()) {
            Ok(r) => panic!("{name} {args} should fail, got {}", r.text),
            Err(e) => e,
        }
    }

    pub fn wait_exit(&mut self, timeout: Duration) -> Option<std::process::ExitStatus> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if let Ok(Some(st)) = self.proc.child.try_wait() {
                return Some(st);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        None
    }
}

pub fn parse_tool_result(res: Value) -> Result<ToolResult, String> {
    let content = res["content"]
        .as_array()
        .unwrap_or_else(|| panic!("result.content missing: {res}"));
    let texts: Vec<String> = content
        .iter()
        .filter(|c| c["type"] == "text")
        .map(|c| c["text"].as_str().expect("text block has text").to_string())
        .collect();
    let images: Vec<(String, Vec<u8>)> = content
        .iter()
        .filter(|c| c["type"] == "image")
        .map(|c| {
            let mime = c["mimeType"].as_str().expect("image mimeType").to_string();
            let data = base64::engine::general_purpose::STANDARD
                .decode(c["data"].as_str().expect("image data"))
                .expect("image data is base64");
            (mime, data)
        })
        .collect();
    if res["isError"] == true {
        assert!(
            !texts.is_empty() && !texts[0].trim().is_empty(),
            "T-2: an error result carries a message: {res}"
        );
        return Err(texts.join("\n"));
    }
    assert_eq!(texts.len(), 1, "T-5: exactly one text block: {res}");
    let json: Value = serde_json::from_str(&texts[0])
        .unwrap_or_else(|e| panic!("T-5: text is JSON ({e}): {}", texts[0]));
    Ok(ToolResult {
        json,
        text: texts[0].clone(),
        images,
        raw: res,
    })
}

// ------------------------------------------------------------------ HTTP client

pub fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

pub struct Served {
    pub proc: Proc,
    pub line: String,
    pub port: u16,
    pub token: String,
    pub stderr: Arc<Mutex<Vec<String>>>,
}

/// Starts `adit serve` (with `--port` when given) and waits for its banner line on stderr.
pub fn serve(port: Option<u16>, token_env: Option<&str>) -> Served {
    let mut cmd = Command::new(bin());
    cmd.arg("serve");
    if let Some(p) = port {
        cmd.args(["--port", &p.to_string()]);
    }
    match token_env {
        Some(t) => cmd.env("ADIT_TOKEN", t),
        None => cmd.env_remove("ADIT_TOKEN"),
    };
    cmd.env("ADIT_MEDIA", "pattern")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn adit serve");
    // stdout is unused for serve; keep it drained.
    let mut out = child.stdout.take().unwrap();
    std::thread::spawn(move || {
        let mut sink = Vec::new();
        let _ = out.read_to_end(&mut sink);
    });
    let (stderr, rx) = drain_stderr(&mut child);
    let proc = Proc { child };
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left) {
            Ok(line) if line.starts_with("adit serving on ") => {
                let rest = &line["adit serving on http://127.0.0.1:".len()..];
                let (p, tail) = rest
                    .split_once("/mcp token=")
                    .unwrap_or_else(|| panic!("banner format: {line}"));
                let port: u16 = p.parse().unwrap_or_else(|_| panic!("banner port: {line}"));
                let token = tail.to_string();
                return Served {
                    proc,
                    line,
                    port,
                    token,
                    stderr,
                };
            }
            Ok(_) => continue,
            Err(_) => panic!(
                "adit serve printed no banner; stderr: {:?}",
                stderr.lock().unwrap()
            ),
        }
    }
}

pub struct Http {
    pub url: String,
    pub session: Option<String>,
    agent: ureq::Agent,
    next: u64,
}

pub struct HttpReply {
    pub status: u16,
    pub message: Option<Value>,
}

impl Http {
    pub fn new(port: u16) -> Http {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(5))
            .timeout_read(Duration::from_secs(30))
            .build();
        Http {
            url: format!("http://127.0.0.1:{port}/mcp"),
            session: None,
            agent,
            next: 1,
        }
    }

    /// POSTs one JSON-RPC message with the given `Authorization` header value (or none).
    pub fn post(&mut self, body: &Value, auth: Option<&str>) -> HttpReply {
        let mut req = self
            .agent
            .post(&self.url)
            .set("Content-Type", "application/json")
            .set("Accept", "application/json, text/event-stream");
        if let Some(a) = auth {
            req = req.set("Authorization", a);
        }
        if let Some(s) = &self.session {
            req = req.set("Mcp-Session-Id", s);
        }
        let resp = match req.send_string(&body.to_string()) {
            Ok(r) => r,
            Err(ureq::Error::Status(code, _)) => {
                return HttpReply {
                    status: code,
                    message: None,
                };
            }
            Err(e) => panic!("HTTP transport error: {e}"),
        };
        let status = resp.status();
        if let Some(sid) = resp.header("Mcp-Session-Id") {
            self.session = Some(sid.to_string());
        }
        let ctype = resp
            .header("Content-Type")
            .unwrap_or("")
            .to_ascii_lowercase();
        let want_id = body.get("id").cloned();
        let mut reader = BufReader::new(resp.into_reader());
        let message = if ctype.starts_with("text/event-stream") {
            // SSE: events separated by blank lines; data lines carry JSON-RPC messages.
            let mut data = String::new();
            let mut found = None;
            let mut line = String::new();
            loop {
                line.clear();
                let n = reader.read_line(&mut line).unwrap_or(0);
                let l = line.trim_end_matches(['\r', '\n']);
                if n == 0 || l.is_empty() {
                    if !data.is_empty() {
                        let v: Value = serde_json::from_str(&data).expect("SSE data is JSON");
                        data.clear();
                        if want_id.is_some() && v.get("id") == want_id.as_ref() {
                            found = Some(v);
                            break;
                        }
                    }
                    if n == 0 {
                        break;
                    }
                } else if let Some(d) = l.strip_prefix("data:") {
                    data.push_str(d.trim_start());
                }
            }
            found
        } else {
            let mut s = String::new();
            reader.read_to_string(&mut s).unwrap();
            if s.trim().is_empty() {
                None
            } else {
                Some(serde_json::from_str(&s).expect("JSON body"))
            }
        };
        HttpReply { status, message }
    }

    pub fn request(&mut self, method: &str, params: Value, auth: Option<&str>) -> HttpReply {
        let id = self.next;
        self.next += 1;
        self.post(
            &json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }),
            auth,
        )
    }

    pub fn notify(&mut self, method: &str, auth: Option<&str>) -> u16 {
        self.post(&json!({ "jsonrpc": "2.0", "method": method }), auth)
            .status
    }
}

// ------------------------------------------------------------------ ffmpeg fixtures

pub fn run_ok(cmd: &mut Command) -> std::process::Output {
    let out = cmd.output().expect("command runs");
    assert!(
        out.status.success(),
        "{cmd:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

/// 64x64, 30 fps, `secs` long, flat colour, with a 440 Hz sine track (48 kHz).
pub fn color_clip(dir: &Path, name: &str, color: &str, secs: u32) -> PathBuf {
    let p = dir.join(name);
    run_ok(
        Command::new("ffmpeg")
            .args(["-v", "error", "-y", "-f", "lavfi", "-i"])
            .arg(format!("color=c={color}:s=64x64:r=30:d={secs}"))
            .args(["-f", "lavfi", "-i"])
            .arg(format!(
                "sine=frequency=440:sample_rate=48000:duration={secs}"
            ))
            .args([
                "-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", "-t",
            ])
            .arg(secs.to_string())
            .arg(&p),
    );
    p
}

/// 3 s: red for 1 s then blue; tone with silence in [1 s, 2 s).
pub fn cut_and_silence_clip(dir: &Path) -> PathBuf {
    let p = dir.join("cut.mp4");
    run_ok(
        Command::new("ffmpeg")
            .args(["-v", "error", "-y"])
            .args(["-f", "lavfi", "-i", "color=c=red:s=64x64:r=30:d=1"])
            .args(["-f", "lavfi", "-i", "color=c=blue:s=64x64:r=30:d=2"])
            .args([
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:sample_rate=48000:duration=3",
            ])
            .args([
                "-filter_complex",
                "[0:v][1:v]concat=n=2:v=1:a=0[v];[2:a]volume=enable='between(t,1,2)':volume=0[a]",
                "-map",
                "[v]",
                "-map",
                "[a]",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                "-t",
                "3",
            ])
            .arg(&p),
    );
    p
}

pub struct Probe {
    pub frames: u64,
    pub width: u32,
    pub height: u32,
    pub fps: String,
    pub has_audio: bool,
}

pub fn ffprobe(path: &Path) -> Probe {
    let out = run_ok(
        Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-count_frames",
                "-show_entries",
                "stream=codec_type,nb_read_frames,width,height,r_frame_rate",
                "-of",
                "json",
            ])
            .arg(path),
    );
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    let streams = v["streams"].as_array().unwrap();
    let vs = streams
        .iter()
        .find(|s| s["codec_type"] == "video")
        .expect("a video stream");
    Probe {
        frames: vs["nb_read_frames"].as_str().unwrap().parse().unwrap(),
        width: vs["width"].as_u64().unwrap() as u32,
        height: vs["height"].as_u64().unwrap() as u32,
        fps: vs["r_frame_rate"].as_str().unwrap().to_string(),
        has_audio: streams.iter().any(|s| s["codec_type"] == "audio"),
    }
}

/// Video frame `n` decoded by ffmpeg to RGB24.
pub fn ffmpeg_frame(path: &Path, n: u32, w: u32, h: u32) -> Vec<u8> {
    let out = run_ok(
        Command::new("ffmpeg")
            .args(["-v", "error", "-i"])
            .arg(path)
            .args([
                "-vf",
                &format!("select=eq(n\\,{n})"),
                "-vframes",
                "1",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgb24",
                "-",
            ]),
    );
    assert_eq!(out.stdout.len() as u32, w * h * 3, "decoded frame size");
    out.stdout
}

pub fn rgb_at(buf: &[u8], w: u32, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * w + x) * 3) as usize;
    [buf[i], buf[i + 1], buf[i + 2]]
}

pub fn assert_rgb(got: [u8; 3], want: [u8; 3], tol: i32, what: &str) {
    for c in 0..3 {
        assert!(
            (got[c] as i32 - want[c] as i32).abs() <= tol,
            "{what}: got {got:?}, expected {want:?} ±{tol}"
        );
    }
}

pub fn px(img: &image::RgbaImage, x: u32, y: u32) -> [u8; 3] {
    let p = img.get_pixel(x, y);
    [p[0], p[1], p[2]]
}

pub fn approx(a: f64, b: f64, tol: f64, what: &str) {
    assert!((a - b).abs() <= tol, "{what}: got {a}, expected {b} ±{tol}");
}

pub fn f(v: &Value) -> f64 {
    v.as_f64().unwrap_or_else(|| panic!("not a number: {v}"))
}

pub fn same_path(a: &str, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => Path::new(a) == b,
    }
}

pub fn solid(rgba: [f64; 4]) -> Value {
    json!({ "type": "solid", "color": rgba })
}

pub const SWAP_WGSL: &str = "fn effect(uv: vec2f) -> vec4f { let c = src(uv); \
    return vec4f(c.b, c.g, c.r, c.a) * vec4f(p.gain, p.gain, p.gain, 1.0); }";

/// Tool names of docs/MCP_TOOLS.md.
pub const TOOL_NAMES: &[&str] = &[
    "project_new",
    "project_open",
    "project_save",
    "project_close",
    "project_list",
    "project_get",
    "media_import",
    "detect_silence",
    "detect_scenes",
    "audio_levels",
    "edit",
    "undo",
    "redo",
    "effect_list",
    "effect_check",
    "render_frame",
    "render_contact_sheet",
    "export",
    "job_status",
    "job_cancel",
    "job_wait",
    "request_create",
    "request_wait",
    "request_list",
    "request_complete",
];

/// `project_new` over MCP; returns (project_id, sequence_id, track_ids).
pub fn new_project(m: &mut Mcp, args: Value) -> (String, u64, Vec<u64>) {
    let r = m.ok("project_new", args).json;
    (
        r["project_id"].as_str().unwrap().to_string(),
        r["sequence_id"].as_u64().unwrap(),
        r["track_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap())
            .collect(),
    )
}

/// Runs the binary to completion with a timeout; returns (exit code, stdout, stderr).
pub fn run_adit(args: &[&str], timeout: Duration) -> (Option<i32>, String, String) {
    let mut child = Command::new(bin())
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn adit");
    let mut out = child.stdout.take().unwrap();
    let mut err = child.stderr.take().unwrap();
    let to = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = out.read_to_string(&mut s);
        s
    });
    let te = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = err.read_to_string(&mut s);
        s
    });
    let mut p = Proc { child };
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(st) = p.child.try_wait().unwrap() {
            break st;
        }
        if Instant::now() > deadline {
            panic!("adit {args:?} did not exit within {timeout:?}");
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let (o, e) = (to.join().unwrap(), te.join().unwrap());
    drop(p);
    (status.code(), o, e)
}
