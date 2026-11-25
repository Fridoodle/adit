//! `adit serve`: streamable HTTP on 127.0.0.1, bearer token, banner on stderr (Binary section).
mod common;
use common::*;
use serde_json::json;
use std::time::Duration;

fn init_params() -> serde_json::Value {
    json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "adit-tests", "version": "0" } })
}

#[test]
fn binary_serve_banner_and_bearer_auth() {
    let port = free_port();
    let token = format!("tok-{}-{}", std::process::id(), port);
    let sv = serve(Some(port), Some(&token));
    assert_eq!(
        sv.line,
        format!("adit serving on http://127.0.0.1:{port}/mcp token={token}"),
        "banner line"
    );
    assert_eq!(sv.port, port);
    assert_eq!(sv.token, token, "ADIT_TOKEN is used as the token");

    let mut h = Http::new(port);
    // No header / wrong token / wrong scheme -> 401.
    assert_eq!(h.request("initialize", init_params(), None).status, 401);
    assert_eq!(
        h.request("initialize", init_params(), Some("Bearer wrong-token"))
            .status,
        401
    );
    assert_eq!(
        h.request("initialize", init_params(), Some(&format!("Basic {token}")))
            .status,
        401
    );
    assert_eq!(
        h.request(
            "initialize",
            init_params(),
            Some(&format!("Bearer {token}x"))
        )
        .status,
        401
    );

    let auth = format!("Bearer {token}");
    let r = h.request("initialize", init_params(), Some(&auth));
    assert_eq!(r.status, 200);
    let msg = r.message.expect("initialize response");
    assert_eq!(msg["result"]["serverInfo"]["name"], "adit");
    let st = h.notify("notifications/initialized", Some(&auth));
    assert!(
        [200, 202, 204].contains(&st),
        "initialized notification accepted, got {st}"
    );

    let r = h.request("tools/list", json!({}), Some(&auth));
    assert_eq!(r.status, 200);
    let tools = r.message.unwrap()["result"]["tools"]
        .as_array()
        .unwrap()
        .clone();
    for name in TOOL_NAMES {
        assert!(
            tools.iter().any(|t| t["name"] == *name),
            "tools/list over HTTP lacks {name}"
        );
    }
    let r = h.request(
        "tools/call",
        json!({ "name": "project_new", "arguments": { "name": "web" } }),
        Some(&auth),
    );
    assert_eq!(r.status, 200);
    let res =
        parse_tool_result(r.message.unwrap()["result"].clone()).expect("project_new over HTTP");
    let pid = res.json["project_id"].as_str().unwrap().to_string();
    // Every request needs the token, also inside an established session.
    let r = h.request(
        "tools/call",
        json!({ "name": "project_get", "arguments": { "project_id": pid } }),
        None,
    );
    assert_eq!(r.status, 401);
    let r = h.request(
        "tools/call",
        json!({ "name": "project_get", "arguments": { "project_id": pid } }),
        Some("Bearer nope"),
    );
    assert_eq!(r.status, 401);
    let r = h.request(
        "tools/call",
        json!({ "name": "project_get", "arguments": { "project_id": pid } }),
        Some(&auth),
    );
    assert_eq!(r.status, 200);
    let res = parse_tool_result(r.message.unwrap()["result"].clone()).unwrap();
    assert_eq!(res.json["name"], "web");
    // Tool errors over HTTP are isError results too (T-2).
    let r = h.request(
        "tools/call",
        json!({ "name": "project_get", "arguments": { "project_id": "nope" } }),
        Some(&auth),
    );
    assert!(parse_tool_result(r.message.unwrap()["result"].clone()).is_err());
}

#[test]
fn binary_serve_random_token_per_launch() {
    let p1 = free_port();
    let a = serve(Some(p1), None);
    let p2 = free_port();
    let b = serve(Some(p2), None);
    assert!(
        a.token.len() >= 16,
        "random token of reasonable length, got {:?}",
        a.token
    );
    assert!(!a.token.contains(char::is_whitespace));
    assert_ne!(a.token, b.token, "token is random per launch");
    let mut h = Http::new(p1);
    assert_eq!(
        h.request(
            "initialize",
            init_params(),
            Some(&format!("Bearer {}", b.token))
        )
        .status,
        401
    );
    assert_eq!(
        h.request(
            "initialize",
            init_params(),
            Some(&format!("Bearer {}", a.token))
        )
        .status,
        200
    );
}

#[test]
fn binary_serve_default_port_7821() {
    // Only test that uses the default port.
    let sv = serve(None, Some("default-port-token"));
    assert_eq!(sv.port, 7821);
    assert!(
        sv.line
            .starts_with("adit serving on http://127.0.0.1:7821/mcp token=")
    );
}

#[test]
fn binary_serve_loopback_only() {
    let port = free_port();
    let sv = serve(Some(port), Some("loopback-token"));
    assert!(sv.line.contains("http://127.0.0.1:"));
    // Find this machine's outward-facing address (UDP connect sends nothing).
    let sock = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
    let ip = sock
        .connect("192.0.2.1:80")
        .ok()
        .and_then(|_| sock.local_addr().ok())
        .map(|a| a.ip());
    match ip {
        Some(ip) if !ip.is_loopback() && !ip.is_unspecified() => {
            let r = std::net::TcpStream::connect_timeout(
                &std::net::SocketAddr::new(ip, port),
                Duration::from_secs(2),
            );
            assert!(
                r.is_err(),
                "adit serve must not accept connections on {ip}:{port}"
            );
        }
        // shortcut: no non-loopback interface on this machine, so only the banner is checked.
        _ => {}
    }
    // Loopback works.
    assert!(std::net::TcpStream::connect(("127.0.0.1", port)).is_ok());
}
