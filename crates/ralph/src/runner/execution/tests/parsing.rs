//! Parsing tests for runner execution helpers.
//!
//! Purpose:
//! - Parsing tests for runner execution helpers.
//!
//! Responsibilities:
//! - Provide focused implementation or regression coverage for this file's owning feature.
//!
//! Scope:
//! - Limited to this file's owning feature boundary.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/Assumptions:
//! - Keep behavior aligned with Ralph's canonical CLI, machine-contract, and queue semantics.

use super::super::command::{effort_as_str, permission_mode_to_arg};
use super::super::json::{
    extract_session_id_from_json, extract_session_id_from_text, parse_json_line,
};
use crate::contracts::{ClaudePermissionMode, ReasoningEffort, Runner};
use serde_json::json;

#[test]
fn permission_mode_to_arg_mapping() {
    assert_eq!(
        permission_mode_to_arg(ClaudePermissionMode::AcceptEdits),
        "acceptEdits"
    );
    assert_eq!(
        permission_mode_to_arg(ClaudePermissionMode::BypassPermissions),
        "bypassPermissions"
    );
}

#[test]
fn effort_as_str_mapping() {
    assert_eq!(effort_as_str(ReasoningEffort::Low), "low");
    assert_eq!(effort_as_str(ReasoningEffort::Medium), "medium");
    assert_eq!(effort_as_str(ReasoningEffort::High), "high");
    assert_eq!(effort_as_str(ReasoningEffort::XHigh), "xhigh");
}

#[test]
fn parse_json_line_handles_invalid_json() {
    assert!(parse_json_line("{").is_none());
}

#[test]
fn parse_json_line_parses_json_with_prefix_noise() {
    let line = "[INFO] {\"type\":\"assistant\",\"session_id\":\"sess-001\"} trailing";
    let json = parse_json_line(line).expect("should parse");
    assert_eq!(
        json.get("session_id").and_then(|v| v.as_str()),
        Some("sess-001")
    );
}

#[test]
fn generic_session_fields_are_not_persisted_without_lifecycle_shape() {
    for (runner, payload) in [
        (Runner::Codex, json!({ "thread_id": "thread-123" })),
        (Runner::Claude, json!({ "session_id": "session-abc" })),
        (
            Runner::Gemini,
            json!({ "type": "message", "session_id": "gemini-xyz" }),
        ),
        (Runner::Opencode, json!({ "sessionID": "ses_123" })),
        (
            Runner::Cursor,
            json!({ "type": "tool_call", "session_id": "cursor-123" }),
        ),
    ] {
        assert_eq!(
            extract_session_id_from_json(&runner, &payload),
            None,
            "{runner:?}"
        );
    }
}

#[test]
fn confirmed_lifecycle_events_are_persisted_by_runner() {
    for (runner, payload, expected) in [
        (
            Runner::Pi,
            json!({ "type": "session", "id": "pi-123" }),
            "pi-123",
        ),
        (
            Runner::Claude,
            json!({ "type": "system", "subtype": "init", "session_id": "claude-123" }),
            "claude-123",
        ),
        (
            Runner::Gemini,
            json!({ "type": "session_started", "session_id": "gemini-123" }),
            "gemini-123",
        ),
        (
            Runner::Codex,
            json!({ "type": "thread.started", "thread_id": "thread-123" }),
            "thread-123",
        ),
        (
            Runner::Opencode,
            json!({ "type": "session", "sessionID": "ses_123" }),
            "ses_123",
        ),
        (
            Runner::Cursor,
            json!({ "type": "system", "subtype": "init", "session_id": "cursor-123" }),
            "cursor-123",
        ),
    ] {
        assert_eq!(
            extract_session_id_from_json(&runner, &payload),
            Some(expected),
            "{runner:?}"
        );
    }
}

#[test]
fn malformed_session_ids_are_rejected() {
    for id in ["", " pi-123", "pi 123", "pi-123\n"] {
        let payload = json!({ "type": "session", "id": id });
        assert_eq!(extract_session_id_from_json(&Runner::Pi, &payload), None);
    }

    let oversized = "x".repeat(513);
    let payload = json!({ "type": "session", "id": oversized });
    assert_eq!(extract_session_id_from_json(&Runner::Pi, &payload), None);
}

#[test]
fn opencode_session_ids_must_use_runner_prefix() {
    let payload = json!({ "type": "session", "sessionID": "open-789" });
    assert_eq!(
        extract_session_id_from_json(&Runner::Opencode, &payload),
        None
    );
}

#[test]
fn extract_session_id_from_text_ignores_chatter_and_accepts_lifecycle_event() {
    let stdout = concat!(
        r#"{"type":"assistant","session_id":"wrong"}"#,
        "\n",
        r#"{"type":"session","id":"pi-good"}"#,
        "\n",
    );
    assert_eq!(
        extract_session_id_from_text(&Runner::Pi, stdout),
        Some("pi-good".to_string())
    );
}

#[test]
fn extract_session_id_from_text_keeps_prefix_suffix_json_compatibility() {
    let stdout = "[INFO] {\"type\":\"session\",\"id\":\"pi-with-prefix\"} [OK]\n";
    assert_eq!(
        extract_session_id_from_text(&Runner::Pi, stdout),
        Some("pi-with-prefix".to_string())
    );
}

#[test]
fn extract_session_id_non_string_values() {
    let stdout = "{\"type\":\"session\",\"id\":12345}\n";
    assert_eq!(extract_session_id_from_text(&Runner::Pi, stdout), None);
}

#[test]
fn extract_session_id_nested_fields_ignored() {
    let stdout = "{\"type\":\"session\",\"data\": {\"id\":\"nested-id\"}}\n";
    assert_eq!(extract_session_id_from_text(&Runner::Pi, stdout), None);
}

#[test]
fn kimi_outputs_never_persist_session_ids_from_stdout() {
    let payload = json!({
        "type": "session",
        "id": "kimi-123",
        "role": "assistant",
        "content": [{"type": "text", "text": "Hello"}],
        "tool_calls": [
            {"type": "function", "id": "tool_bUJW2GCXzg65VTa72XV9YhNn", "function": {"name": "test"}}
        ]
    });
    assert_eq!(extract_session_id_from_json(&Runner::Kimi, &payload), None);
}

#[test]
fn plugin_finish_event_persists_external_plugin_session_id() {
    let payload = json!({ "type": "finish", "session_id": "plugin-session-123" });
    assert_eq!(
        extract_session_id_from_json(&Runner::Plugin("custom".into()), &payload),
        Some("plugin-session-123")
    );
}

#[test]
fn plugin_generic_session_fields_are_not_persisted_without_finish_event() {
    let payload = json!({ "session_id": "plugin-session-123" });
    assert_eq!(
        extract_session_id_from_json(&Runner::Plugin("custom".into()), &payload),
        None
    );
}

#[test]
fn extract_session_id_from_text_kimi_format() {
    let stdout = r#"{"role":"assistant","content":[{"type":"text","text":"Hello"}],"tool_calls":[{"id":"tool_xyz789","type":"function"}]}"#;
    assert_eq!(extract_session_id_from_text(&Runner::Kimi, stdout), None);
}
