//! JSON parsing helpers for runner streaming output.

use serde_json::Value as JsonValue;

pub(super) fn parse_json_line(line: &str) -> Option<JsonValue> {
    serde_json::from_str::<JsonValue>(line).ok()
}

pub(super) fn extract_session_id_from_json(json: &JsonValue) -> Option<String> {
    if let Some(id) = json.get("thread_id").and_then(|v| v.as_str()) {
        return Some(id.to_string());
    }
    if let Some(id) = json.get("session_id").and_then(|v| v.as_str()) {
        return Some(id.to_string());
    }
    if let Some(id) = json.get("sessionID").and_then(|v| v.as_str()) {
        return Some(id.to_string());
    }
    None
}

pub(super) fn extract_session_id_from_text(stdout: &str) -> Option<String> {
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(json) = serde_json::from_str::<JsonValue>(line) {
            if let Some(id) = extract_session_id_from_json(&json) {
                return Some(id);
            }
        }
    }
    None
}
