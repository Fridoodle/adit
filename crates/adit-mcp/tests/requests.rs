//! T-REQ-1, T-REQ-2, T-REQ-3 (magic-wand requests).
mod common;
use common::*;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn create(s: &adit_mcp::Session, pid: &str, text: &str) -> Value {
    call(
        s,
        "request_create",
        json!({ "project_id": pid, "text": text }),
    )
    .json["request_id"]
        .clone()
}

fn wait(s: &adit_mcp::Session, timeout_s: u64) -> Value {
    call(s, "request_wait", json!({ "timeout_s": timeout_s })).json["request"].clone()
}

#[test]
fn t_req_1_2_create_and_wait_returns_immediately() {
    let s = session();
    let p = new_project(&s, json!({}));
    let c = add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("3s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    let id = call(
        &s,
        "request_create",
        json!({ "project_id": p.id, "text": "make this pop", "start": "1s", "end": "60f",
                "clip_ids": [c], "rect": [0.1, 0.2, 0.3, 0.4] }),
    )
    .json["request_id"]
        .clone();
    assert!(!id.is_null());
    let l = request_list(&s, json!({}));
    assert_eq!(l.len(), 1);
    assert_eq!(l[0]["status"], "pending", "T-REQ-1: starts pending");

    let t0 = Instant::now();
    let r = wait(&s, 10);
    assert!(
        t0.elapsed() < Duration::from_millis(500),
        "pending request -> immediate, took {:?}",
        t0.elapsed()
    );
    assert_eq!(r["request_id"], id);
    assert_eq!(r["project_id"], p.id.as_str());
    assert_eq!(
        r["sequence_id"].as_u64(),
        Some(p.seq),
        "default: first sequence"
    );
    assert_eq!(r["text"], "make this pop");
    assert_eq!(r["start"], 705_600_000);
    assert_eq!(r["end"], 1_411_200_000, "60f at 30 fps = 2 s");
    approx(f(&r["start_s"]), 1.0, 1e-9, "start_s");
    approx(f(&r["end_s"]), 2.0, 1e-9, "end_s");
    assert_eq!(r["clip_ids"], json!([c]));
    let rect: Vec<f64> = r["rect"].as_array().unwrap().iter().map(f).collect();
    assert_eq!(rect, vec![0.1, 0.2, 0.3, 0.4]);
    let l = request_list(&s, json!({}));
    assert_eq!(l[0]["status"], "claimed", "T-REQ-2: wait marks it claimed");
}

#[test]
fn t_req_2_timeout_returns_null() {
    let s = session();
    let t0 = Instant::now();
    let r = wait(&s, 1);
    let el = t0.elapsed();
    assert!(r.is_null(), "no request -> null, got {r}");
    assert!(
        el >= Duration::from_millis(900) && el < Duration::from_secs(4),
        "timeout 1 s took {el:?}"
    );
}

#[test]
fn t_req_2_wakes_promptly_when_created_from_another_thread() {
    let s = session();
    let p = new_project(&s, json!({}));
    let created_at: Arc<Mutex<Option<Instant>>> = Arc::new(Mutex::new(None));
    let waiter = {
        let s = s.clone();
        let created_at = created_at.clone();
        std::thread::spawn(move || {
            let r = wait(&s, 20);
            let woke = Instant::now();
            (
                r,
                woke,
                created_at.lock().unwrap().expect("created before wake"),
            )
        })
    };
    std::thread::sleep(Duration::from_millis(400));
    let id = {
        let mut g = created_at.lock().unwrap();
        *g = Some(Instant::now());
        create(&s, &p.id, "from the GUI")
    };
    let (r, woke, at) = waiter.join().unwrap();
    assert_eq!(r["request_id"], id);
    assert!(
        woke.duration_since(at) < Duration::from_secs(1),
        "T-REQ-2: woke after {:?}",
        woke.duration_since(at)
    );
}

#[test]
fn t_req_2_each_request_handed_out_once() {
    let s = session();
    let p = new_project(&s, json!({}));
    let spawn = |s: &adit_mcp::Session| {
        let s = s.clone();
        std::thread::spawn(move || wait(&s, 3))
    };
    let w1 = spawn(&s);
    let w2 = spawn(&s);
    std::thread::sleep(Duration::from_millis(300));
    let id = create(&s, &p.id, "only one");
    let (a, b) = (w1.join().unwrap(), w2.join().unwrap());
    let got: Vec<&Value> = [&a, &b].into_iter().filter(|r| !r.is_null()).collect();
    assert_eq!(got.len(), 1, "exactly one waiter gets it: {a} / {b}");
    assert_eq!(got[0]["request_id"], id);
    // Claimed -> not handed out again.
    assert!(wait(&s, 1).is_null());

    // Two requests, two concurrent waiters: each gets a different one.
    let r1 = create(&s, &p.id, "one");
    let r2 = create(&s, &p.id, "two");
    let w1 = spawn(&s);
    let w2 = spawn(&s);
    let (a, b) = (w1.join().unwrap(), w2.join().unwrap());
    let mut ids = vec![a["request_id"].clone(), b["request_id"].clone()];
    ids.sort_by_key(|v| v.to_string());
    let mut want = vec![r1, r2];
    want.sort_by_key(|v| v.to_string());
    assert_eq!(ids, want);
}

#[test]
fn t_req_2_oldest_first() {
    let s = session();
    let p = new_project(&s, json!({}));
    let a = create(&s, &p.id, "first");
    let b = create(&s, &p.id, "second");
    assert_eq!(wait(&s, 5)["request_id"], a);
    assert_eq!(wait(&s, 5)["request_id"], b);
}

#[test]
fn t_req_3_list_and_complete() {
    let s = session();
    let p = new_project(&s, json!({}));
    let a = create(&s, &p.id, "a");
    let b = create(&s, &p.id, "b");
    let c = create(&s, &p.id, "c");
    assert_eq!(wait(&s, 5)["request_id"], a); // a claimed
    call(
        &s,
        "request_complete",
        json!({ "request_id": a, "message": "done it" }),
    );
    assert_eq!(wait(&s, 5)["request_id"], b); // b claimed
    let all = request_list(&s, json!({}));
    assert_eq!(all.len(), 3);
    let status =
        |id: &Value| all.iter().find(|r| r["request_id"] == *id).unwrap()["status"].clone();
    assert_eq!(status(&a), "done");
    assert_eq!(status(&b), "claimed");
    assert_eq!(status(&c), "pending");
    let ids = |st: &str| -> Vec<Value> {
        request_list(&s, json!({ "status": st }))
            .iter()
            .map(|r| r["request_id"].clone())
            .collect()
    };
    assert_eq!(ids("pending"), vec![c.clone()]);
    assert_eq!(ids("claimed"), vec![b.clone()]);
    assert_eq!(ids("done"), vec![a.clone()]);
    // Completing a pending request is allowed; message defaults to "".
    call(&s, "request_complete", json!({ "request_id": c }));
    assert_eq!(ids("done").len(), 2);
    assert!(wait(&s, 1).is_null(), "a done request is not handed out");
    call_err(
        &s,
        "request_complete",
        json!({ "request_id": "no_such_request" }),
    );
    call_err(
        &s,
        "request_create",
        json!({ "project_id": "nope", "text": "x" }),
    );
}
