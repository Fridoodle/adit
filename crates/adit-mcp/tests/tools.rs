//! T-1 (tool list + input schemas, library side) and T-5 (result shape, library side).
mod common;
use common::*;
use serde_json::json;

/// Every tool of docs/MCP_TOOLS.md with every argument it names.
pub const TOOLS: &[(&str, &[&str])] = &[
    (
        "project_new",
        &[
            "name",
            "width",
            "height",
            "fps",
            "sample_rate",
            "video_tracks",
            "audio_tracks",
        ],
    ),
    ("project_open", &["path"]),
    ("project_save", &["project_id", "path"]),
    ("project_close", &["project_id"]),
    ("project_list", &[]),
    ("project_get", &["project_id", "detail"]),
    ("media_import", &["project_id", "paths"]),
    (
        "detect_silence",
        &["project_id", "media_id", "noise_db", "min_duration"],
    ),
    ("detect_scenes", &["project_id", "media_id", "threshold"]),
    (
        "audio_levels",
        &["project_id", "sequence_id", "start", "end", "buckets"],
    ),
    ("edit", &["project_id", "sequence_id", "commands"]),
    ("undo", &["project_id", "steps"]),
    ("redo", &["project_id", "steps"]),
    ("effect_list", &["project_id", "include_source"]),
    ("effect_check", &["kind", "wgsl", "params"]),
    (
        "render_frame",
        &["project_id", "sequence_id", "time", "max_size", "format"],
    ),
    (
        "render_contact_sheet",
        &[
            "project_id",
            "sequence_id",
            "start",
            "end",
            "count",
            "columns",
            "cell_size",
        ],
    ),
    (
        "export",
        &[
            "project_id",
            "sequence_id",
            "path",
            "codec",
            "audio",
            "width",
            "height",
            "fps",
            "crf",
        ],
    ),
    ("job_status", &["job_id"]),
    ("job_cancel", &["job_id"]),
    ("job_wait", &["job_id", "timeout_s"]),
    (
        "request_create",
        &[
            "project_id",
            "sequence_id",
            "text",
            "start",
            "end",
            "clip_ids",
            "rect",
        ],
    ),
    ("request_wait", &["timeout_s"]),
    ("request_list", &["status"]),
    ("request_complete", &["request_id", "message"]),
];

#[test]
fn t1_tools_lists_every_tool_with_schema() {
    let tools = adit_mcp::Session::tools();
    for (name, args) in TOOLS {
        let t = tools
            .iter()
            .find(|t| t.name == *name)
            .unwrap_or_else(|| panic!("tool {name} missing"));
        assert!(!t.description.trim().is_empty(), "{name}: description");
        assert_eq!(
            t.input_schema["type"], "object",
            "{name}: input schema is an object schema"
        );
        for a in *args {
            assert!(
                t.input_schema["properties"].get(*a).is_some(),
                "{name}: schema property {a} missing in {}",
                t.input_schema
            );
        }
    }
    let mut names: Vec<&str> = tools.iter().map(|t| t.name).collect();
    let n = names.len();
    names.sort();
    names.dedup();
    assert_eq!(n, names.len(), "tool names unique");
    // Required arguments are marked required.
    let req = |name: &str| -> Vec<String> {
        let t = tools.iter().find(|t| t.name == name).unwrap();
        t.input_schema["required"]
            .as_array()
            .map(|a| a.iter().map(|v| v.as_str().unwrap().to_string()).collect())
            .unwrap_or_default()
    };
    assert!(req("project_new").contains(&"name".to_string()));
    assert!(req("edit").contains(&"commands".to_string()));
    assert!(req("edit").contains(&"project_id".to_string()));
    assert!(
        !req("render_frame").contains(&"time".to_string()),
        "time is optional"
    );
}

#[test]
fn t5_results_are_json_with_images_only_where_stated() {
    let s = session();
    let p = new_project(&s, json!({}));
    let o = call(&s, "project_get", json!({ "project_id": p.id }));
    assert!(o.json.is_object());
    assert!(o.images.is_empty(), "project_get has no image");
    add_solid(
        &s,
        &p.id,
        p.v1(),
        json!(0),
        json!("1s"),
        [1.0, 0.0, 0.0, 1.0],
    );
    let o = call(&s, "render_frame", json!({ "project_id": p.id }));
    assert_eq!(o.images.len(), 1);
    // Times both ways where useful: time_s plus frame.
    assert!(o.json["time_s"].is_f64() || o.json["time_s"].is_u64());
    let o = call(
        &s,
        "render_contact_sheet",
        json!({ "project_id": p.id, "count": 2, "cell_size": 16 }),
    );
    assert_eq!(o.images.len(), 1, "contact sheet: one image block");
    let o = call(
        &s,
        "audio_levels",
        json!({ "project_id": p.id, "buckets": 2 }),
    );
    assert!(o.images.is_empty());
}
