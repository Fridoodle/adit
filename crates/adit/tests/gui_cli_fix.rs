//! U-FIX-13 (docs/GUI.md): `adit gui --port 0` is bad usage. It must be rejected before a window
//! would open; until then this test sees the window start and fails at the 10 s deadline (the
//! process is killed).
mod common;
use common::*;
use std::time::Duration;

#[test]
fn u_fix_13_gui_port_0_exits_2_with_usage() {
    let args = ["gui", "--port", "0"];
    let (code, stdout, err) = run_adit(&args, Duration::from_secs(10));
    assert_eq!(code, Some(2), "adit {args:?}: exit 2; stderr: {err}");
    assert!(
        err.to_lowercase().contains("usage"),
        "adit {args:?}: usage on stderr, got {err:?}"
    );
    assert!(stdout.is_empty(), "adit {args:?}: nothing on stdout");
}
