//! Stream filtering tests for runner execution output.

use super::super::stream::{display_filtered_json, extract_display_lines, StreamSink};
use crate::runner::{OutputHandler, OutputStream};
use serde_json::json;
use std::sync::{Arc, Mutex};

#[test]
fn extract_display_lines_codex_agent_message() {
    let payload = json!({
        "type": "item.completed",
        "item": {"type": "agent_message", "text": "Hi!"}
    });
    assert_eq!(extract_display_lines(&payload), vec!["Hi!", ""]);
}

#[test]
fn extract_display_lines_codex_reasoning() {
    let payload = json!({
        "type": "item.completed",
        "item": {"type": "reasoning", "text": "Working it out"}
    });
    assert_eq!(
        extract_display_lines(&payload),
        vec!["[Reasoning] Working it out"]
    );
}

#[test]
fn extract_display_lines_codex_tool_call() {
    let payload = json!({
        "type": "item.completed",
        "item": {
            "type": "mcp_tool_call",
            "server": "RepoPrompt",
            "tool": "get_file_tree",
            "status": "completed",
            "arguments": {
                "path": "/tmp/project",
                "pattern": "*.rs"
            }
        }
    });
    assert_eq!(
        extract_display_lines(&payload),
        vec!["[Tool] RepoPrompt.get_file_tree (completed) path=/tmp/project pattern=*.rs"]
    );
}

#[test]
fn extract_display_lines_codex_command_execution() {
    let payload = json!({
        "type": "item.started",
        "item": {
            "type": "command_execution",
            "command": "/bin/zsh -lc ls",
            "status": "in_progress",
            "exit_code": null
        }
    });
    assert_eq!(
        extract_display_lines(&payload),
        vec!["[Command] /bin/zsh -lc ls (in_progress)"]
    );
}

#[test]
fn extract_display_lines_claude_result_and_tool_use() {
    let payload = json!({
        "result": "Final answer",
        "type": "assistant",
        "message": {
            "content": [
                {"type": "text", "text": "Streamed text"},
                {"type": "tool_use", "name": "Read", "input": {"file_path": "/tmp/a.txt"}}
            ]
        }
    });
    assert_eq!(
        extract_display_lines(&payload),
        vec![
            "Final answer",
            "Streamed text",
            "[Tool] Read path=/tmp/a.txt"
        ]
    );
}

#[test]
fn extract_display_lines_permission_denial() {
    let payload = json!({
        "permission_denials": [
            {"tool_name": "write"}
        ]
    });
    assert_eq!(
        extract_display_lines(&payload),
        vec!["[Permission denied: write]"]
    );
}

#[test]
fn display_filtered_json_calls_output_handler() {
    let payload = json!({
        "type": "text",
        "part": { "text": "hello" }
    });
    let captured: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let handler: OutputHandler = Arc::new(Box::new({
        let captured = Arc::clone(&captured);
        move |text: &str| {
            captured
                .lock()
                .expect("capture lock")
                .push(text.to_string());
        }
    }));

    display_filtered_json(
        &payload,
        &StreamSink::Stdout,
        Some(&handler),
        OutputStream::HandlerOnly,
    )
    .expect("display filtered json");

    let guard = captured.lock().expect("capture lock");
    assert_eq!(guard.as_slice(), &["hello\n".to_string()]);
}

#[test]
fn extract_display_lines_opencode_text() {
    let payload = json!({
        "type": "text",
        "part": { "text": "hello" }
    });
    assert_eq!(extract_display_lines(&payload), vec!["hello"]);
}

#[test]
fn extract_display_lines_opencode_tool_use() {
    let payload = json!({
        "type": "tool_use",
        "part": {
            "tool": "read",
            "state": {
                "status": "completed",
                "input": { "filePath": "/tmp/example.txt" }
            }
        }
    });
    assert_eq!(
        extract_display_lines(&payload),
        vec!["[Tool] read (completed) path=/tmp/example.txt"]
    );
}

#[test]
fn extract_display_lines_gemini_tool_use_and_result() {
    let tool_use = json!({
        "type": "tool_use",
        "tool_name": "read_file",
        "parameters": { "file_path": "notes.txt" }
    });
    assert_eq!(
        extract_display_lines(&tool_use),
        vec!["[Tool] read_file path=notes.txt"]
    );

    let tool_result = json!({
        "type": "tool_result",
        "tool_name": "read_file",
        "status": "success"
    });
    assert_eq!(
        extract_display_lines(&tool_result),
        vec!["[Tool] read_file (success)"]
    );
}

#[test]
fn extract_display_lines_gemini_message_assistant() {
    let payload = json!({
        "type": "message",
        "role": "assistant",
        "content": "hi"
    });
    assert_eq!(extract_display_lines(&payload), vec!["hi"]);
}

#[test]
fn extract_display_lines_unknown_event_is_noop() {
    let payload = json!({"type": "unknown"});
    assert!(extract_display_lines(&payload).is_empty());
}
