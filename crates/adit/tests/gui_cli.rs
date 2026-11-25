//! `adit gui` argument handling (docs/GUI.md "Binary"). No test opens a window: every case here
//! must be rejected before one would be created.
mod common;
use common::*;
use std::time::Duration;

#[test]
fn binary_gui_bad_arguments_exit_2_with_usage() {
    for args in [
        vec!["gui", "--port", "notanumber"],
        vec!["gui", "--port", "70000"],
        vec!["gui", "--port"],
        vec!["gui", "a.adit", "b.adit"],
        vec!["gui", "--bogus"],
    ] {
        let (code, stdout, err) = run_adit(&args, Duration::from_secs(20));
        assert_eq!(code, Some(2), "adit {args:?}: exit 2; stderr: {err}");
        assert!(
            err.to_lowercase().contains("usage"),
            "adit {args:?}: usage on stderr, got {err:?}"
        );
        // The usage documents the new command (today's usage has no `adit gui` line).
        assert!(
            err.contains("adit gui"),
            "adit {args:?}: usage lists `adit gui`, got {err:?}"
        );
        assert!(stdout.is_empty(), "adit {args:?}: nothing on stdout");
    }
}

#[test]
fn binary_bare_adit_still_usage_error() {
    let (code, stdout, err) = run_adit(&[], Duration::from_secs(20));
    assert_eq!(code, Some(2), "bare adit: exit 2; stderr: {err}");
    assert!(err.to_lowercase().contains("usage"), "{err:?}");
    assert!(
        err.contains("adit gui"),
        "usage lists `adit gui`, got {err:?}"
    );
    assert!(stdout.is_empty());
}
