//! MCP wire protocol (JSON-RPC 2.0: initialize, ping, tools/list, tools/call) over stdio and
//! streamable HTTP. Hand-written: the protocol subset is tiny and this avoids an async runtime.

use crate::{Session, tools};
use base64::Engine;
use serde_json::{Value, json};
use std::io::{self, BufRead, Read, Write};
use std::sync::{Arc, Mutex};

const LATEST: &str = "2025-06-18";
const SUPPORTED: [&str; 3] = [LATEST, "2025-03-26", "2024-11-05"];

const INSTRUCTIONS: &str = r#"adit is a video editor you drive with tools. Typical flow: project_new (or project_open), media_import, edit (add clips, effects, transitions), render_frame / render_contact_sheet to look at the result, audio_levels to listen, export to write a file. project_get shows what exists. Every edit is undoable (undo / redo). Several projects can be open at once; every project tool takes project_id.

TIMES. Wherever a tool or an edit command takes a time or duration (edit fields start, end, duration, time, to, delta, source_in; tool arguments time, start, end, min_duration) it accepts an integer count of flicks (705,600,000 per second), or a string: "1.5s" (seconds), "45f" (frames at the sequence's frame rate), "00:00:01:15" (hours:minutes:seconds:frames, non-drop). Negative values such as "-0.5s" or "-3f" are allowed where the command allows them (e.g. delta). Frame strings use the frame rate of the tool's sequence_id, else of the project's first sequence. Results give times in seconds (*_s) and often frames.

EFFECT SHADERS (define_effect in edit; test with effect_check). An effect is WGSL (kind "filter" or "transition") plus declared parameters. A filter defines `fn effect(uv: vec2f) -> vec4f`; a transition defines `fn transition(uv: vec2f) -> vec4f`. uv is 0..1 with the origin at the top-left; colours are straight-alpha RGBA, sRGB-encoded 0..1. adit prepends these, so use them without declaring them:
- `fn src(uv: vec2f) -> vec4f`: the input image (filters), bilinear, transparent outside 0..1.
- `fn src_a(uv: vec2f) -> vec4f`, `fn src_b(uv: vec2f) -> vec4f`: the two layers (transitions).
- `ctx.size: vec2f` image size in pixels; `ctx.time: f32` seconds since clip start; `ctx.duration: f32` clip or transition length in seconds; `ctx.progress: f32` time / duration, 0..1; `ctx.frame: f32` sequence frame number.
- `p.<name>: f32` for every declared parameter (names must be WGSL identifiers, unique, at most 64), in declaration order.
You may add helper functions and constants. Do not declare bindings or entry points. Parameters can be set and keyframed per clip (set_param, set_keyframe). Example filter: `fn effect(uv: vec2f) -> vec4f { let c = src(uv); return vec4f(c.b, c.g, c.r, c.a) * vec4f(p.gain, p.gain, p.gain, 1.0); }` with parameter gain (default 1). effect_list shows the built-in effects and their source.

REQUESTS. The user can point at something in the editor and say what they want; call request_wait to receive it, do the work with edit, then request_complete with a short message.

Errors come back as tool results with isError true and a message naming the cause; a failed call changes nothing."#;

fn err(id: &Value, code: i64, msg: impl Into<String>) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": msg.into() } })
}

/// Tools that wait for something: handled on their own thread so they do not stall the transport.
pub fn is_blocking(msg: &Value) -> bool {
    msg["method"] == "tools/call"
        && ["request_wait", "job_wait"].contains(&msg["params"]["name"].as_str().unwrap_or(""))
}

/// One incoming JSON-RPC message -> the response to send, if any.
pub fn handle(session: &Session, msg: &Value) -> Option<Value> {
    let Some(obj) = msg.as_object() else {
        return Some(err(&Value::Null, -32600, "expected a JSON-RPC object"));
    };
    let id = obj.get("id");
    let Some(method) = obj.get("method").and_then(Value::as_str) else {
        // A response from the client, or junk: nothing to answer unless it claims to be a request.
        return id
            .filter(|_| !obj.contains_key("result") && !obj.contains_key("error"))
            .map(|id| err(id, -32600, "missing method"));
    };
    let Some(id) = id else {
        return None; // notification (notifications/initialized, ...)
    };
    let params = obj.get("params").unwrap_or(&Value::Null);
    let result = match method {
        "initialize" => {
            let want = params["protocolVersion"].as_str().unwrap_or("");
            json!({
                "protocolVersion": if SUPPORTED.contains(&want) { want } else { LATEST },
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "adit", "version": env!("CARGO_PKG_VERSION") },
                "instructions": INSTRUCTIONS,
            })
        }
        "ping" => json!({}),
        "tools/list" => json!({ "tools": Session::tools().into_iter()
            .map(|t| json!({ "name": t.name, "description": t.description, "inputSchema": t.input_schema }))
            .collect::<Vec<_>>() }),
        "tools/call" => match call_tool(session, params) {
            Ok(v) => v,
            Err(e) => return Some(err(id, -32602, e)),
        },
        _ => return Some(err(id, -32601, format!("method not found: {method}"))),
    };
    Some(json!({ "jsonrpc": "2.0", "id": id, "result": result }))
}

fn call_tool(session: &Session, params: &Value) -> Result<Value, String> {
    let name = params["name"]
        .as_str()
        .ok_or("tools/call needs a string `name`")?;
    if !tools::exists(name) {
        return Err(format!("unknown tool `{name}`"));
    }
    let args = match &params["arguments"] {
        Value::Null => json!({}),
        a @ Value::Object(_) => a.clone(),
        _ => return Err("`arguments` must be an object".into()),
    };
    Ok(match session.call(name, args) {
        Ok(out) => {
            let mut content = vec![json!({ "type": "text", "text": out.json.to_string() })];
            content.extend(out.images.iter().map(|im| {
                json!({ "type": "image", "mimeType": im.mime,
                        "data": base64::engine::general_purpose::STANDARD.encode(&im.bytes) })
            }));
            json!({ "content": content, "isError": false })
        }
        Err(e) => json!({ "content": [{ "type": "text", "text": e }], "isError": true }),
    })
}

/// Serves MCP on stdin/stdout until stdin closes.
pub fn serve_stdio(session: Session) -> io::Result<()> {
    let out = Arc::new(Mutex::new(io::stdout()));
    let send = |v: &Value| {
        let mut o = out.lock().unwrap_or_else(|e| e.into_inner());
        // A closed stdout means the client is gone; the read loop ends soon after.
        let _ = writeln!(o, "{v}").and_then(|()| o.flush());
    };
    let mut stdin = io::stdin().lock();
    let mut line = Vec::new();
    loop {
        line.clear();
        if stdin.read_until(b'\n', &mut line)? == 0 {
            return Ok(());
        }
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let msg: Value = match serde_json::from_slice(&line) {
            Ok(m) => m,
            Err(e) => {
                send(&err(&Value::Null, -32700, format!("parse error: {e}")));
                continue;
            }
        };
        if is_blocking(&msg) {
            let (session, out) = (session.clone(), out.clone());
            std::thread::spawn(move || {
                if let Some(r) = handle(&session, &msg) {
                    let mut o = out.lock().unwrap_or_else(|e| e.into_inner());
                    let _ = writeln!(o, "{r}").and_then(|()| o.flush());
                }
            });
        } else if let Some(r) = handle(&session, &msg) {
            send(&r);
        }
    }
}

/// Equal without an early exit on the first differing byte.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let mut diff = a.len() ^ b.len();
    for i in 0..a.len().max(b.len()) {
        diff |= usize::from(a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0));
    }
    diff == 0
}

fn header(code: u16, body: String) -> tiny_http::Response<io::Cursor<Vec<u8>>> {
    let r = tiny_http::Response::from_string(body).with_status_code(code);
    match tiny_http::Header::from_bytes("Content-Type", "application/json") {
        Ok(h) => r.with_header(h),
        Err(()) => r,
    }
}

fn http_request(session: &Session, token: &str, mut req: tiny_http::Request) {
    let authorized = req
        .headers()
        .iter()
        .find(|h| h.field.equiv("Authorization"))
        .and_then(|h| h.value.as_str().strip_prefix("Bearer "))
        .is_some_and(|t| constant_time_eq(t.as_bytes(), token.as_bytes()));
    let reply = if !authorized {
        header(
            401,
            err(&Value::Null, -32001, "missing or wrong bearer token").to_string(),
        )
    } else if req.url().split('?').next() != Some("/mcp") {
        header(404, String::new())
    } else if *req.method() != tiny_http::Method::Post {
        // No server-initiated messages, so no SSE stream and no sessions to delete.
        header(405, String::new())
    } else {
        let mut body = Vec::new();
        // shortcut: 64 MiB request cap; raise if clients ever need to send bigger payloads.
        match req.as_reader().take(64 << 20).read_to_end(&mut body) {
            Err(e) => header(400, err(&Value::Null, -32700, e.to_string()).to_string()),
            Ok(_) => match serde_json::from_slice::<Value>(&body) {
                Err(e) => header(
                    400,
                    err(&Value::Null, -32700, format!("parse error: {e}")).to_string(),
                ),
                Ok(msg) if msg.is_array() => header(
                    400,
                    err(&Value::Null, -32600, "batches are not supported").to_string(),
                ),
                Ok(msg) => match handle(session, &msg) {
                    Some(r) => header(200, r.to_string()),
                    None => header(202, String::new()),
                },
            },
        }
    };
    let _ = req.respond(reply);
}

/// Serves MCP streamable HTTP on 127.0.0.1:`port` at `/mcp`, requiring `Authorization: Bearer <token>`.
pub fn serve_http(session: Session, port: u16, token: String) -> io::Result<()> {
    let server = tiny_http::Server::http(("127.0.0.1", port)).map_err(io::Error::other)?;
    let port = server.server_addr().to_ip().map_or(port, |a| a.port());
    eprintln!("adit serving on http://127.0.0.1:{port}/mcp token={token}");
    let token: Arc<str> = token.into();
    for req in server.incoming_requests() {
        let (session, token) = (session.clone(), token.clone());
        // One thread per request so request_wait / job_wait never block other callers.
        std::thread::spawn(move || http_request(&session, &token, req));
    }
    Ok(())
}
