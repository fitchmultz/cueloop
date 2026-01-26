//! Parsing tests for runner execution helpers.

use super::super::json::{
    extract_session_id_from_json, extract_session_id_from_text, parse_json_line,
};
use super::super::runners::{effort_as_str, permission_mode_to_arg};
use crate::contracts::{ClaudePermissionMode, ReasoningEffort};
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
fn extract_session_id_from_json_codex_thread_id() {
    let payload = json!({
        "thread_id": "thread-123"
    });
    assert_eq!(
        extract_session_id_from_json(&payload),
        Some("thread-123".to_string())
    );
}

#[test]
fn extract_session_id_from_json_claude_session_id() {
    let payload = json!({
        "session_id": "session-abc"
    });
    assert_eq!(
        extract_session_id_from_json(&payload),
        Some("session-abc".to_string())
    );
}

#[test]
fn extract_session_id_from_json_gemini_session_id() {
    let payload = json!({
        "session_id": "gemini-xyz"
    });
    assert_eq!(
        extract_session_id_from_json(&payload),
        Some("gemini-xyz".to_string())
    );
}

#[test]
fn extract_session_id_from_json_opencode_session_id() {
    let payload = json!({
        "sessionID": "open-789"
    });
    assert_eq!(
        extract_session_id_from_json(&payload),
        Some("open-789".to_string())
    );
}

#[test]
fn extract_session_id_from_text_reads_json_lines() {
    let stdout = "{\"session_id\":\"sess-001\"}\n{\"result\":\"ok\"}\n";
    assert_eq!(
        extract_session_id_from_text(stdout),
        Some("sess-001".to_string())
    );
}
