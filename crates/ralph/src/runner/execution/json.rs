//! JSON parsing helpers for runner streaming output.
//!
//! Purpose:
//! - Parse runner JSON output and extract only confirmed runner lifecycle session IDs.
//!
//! Responsibilities:
//! - Preserve permissive JSON-line parsing needed for stream rendering.
//! - Validate candidate session IDs before they can be persisted for resume.
//! - Keep session extraction runner-aware so arbitrary chatter fields are ignored.
//!
//! Scope:
//! - Handles stream JSON parsing and session-id extraction from already-emitted runner output.
//! - Does not validate whether a persisted ID still exists in the external runner store.
//!
//! Usage:
//! - Streaming process readers call these helpers while consuming stdout.
//! - Continue-session policy uses the lexical validator before preferring a candidate ID.
//!
//! Invariants/Assumptions:
//! - Prefer no session ID over a guessed ID.
//! - Generic `thread_id`, `session_id`, or `sessionID` fields are not authoritative unless
//!   wrapped in a runner-specific lifecycle/session event shape.

use crate::contracts::Runner;
use serde_json::Value as JsonValue;

const MAX_SESSION_ID_LEN: usize = 512;

pub(super) fn parse_json_line(line: &str) -> Option<JsonValue> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(value) = serde_json::from_str::<JsonValue>(trimmed) {
        return Some(value);
    }

    // Some runners interleave logs or ANSI control sequences with JSON. As a best-effort
    // compatibility layer, attempt to parse the first JSON value starting at the first '{'.
    let json_start = trimmed.find('{')?;
    let potential_json = &trimmed[json_start..];
    let mut stream = serde_json::Deserializer::from_str(potential_json).into_iter::<JsonValue>();
    stream.next().and_then(|res| {
        res.inspect_err(|e| log::trace!("JSON stream parse error: {}", e))
            .ok()
    })
}

pub(crate) fn is_valid_runner_session_id(runner: &Runner, id: &str) -> bool {
    validate_runner_session_id(runner, id)
}

pub(super) fn extract_session_id_from_json<'a>(
    runner: &Runner,
    json: &'a JsonValue,
) -> Option<&'a str> {
    let id = match runner {
        Runner::Pi => extract_pi_session_id(json),
        Runner::Claude => extract_claude_session_id(json),
        Runner::Gemini => extract_gemini_session_id(json),
        Runner::Codex => extract_codex_session_id(json),
        Runner::Opencode => extract_opencode_session_id(json),
        Runner::Cursor => extract_cursor_session_id(json),
        Runner::Plugin(_) => extract_plugin_session_id(json),
        Runner::Kimi => None,
    }?;

    validate_runner_session_id(runner, id).then_some(id)
}

pub(super) fn extract_session_id_from_text(runner: &Runner, stdout: &str) -> Option<String> {
    for line in stdout.lines() {
        let Some(json) = parse_json_line(line) else {
            continue;
        };
        if let Some(id) = extract_session_id_from_json(runner, &json) {
            return Some(id.to_owned());
        }
    }
    None
}

fn validate_runner_session_id(runner: &Runner, id: &str) -> bool {
    let trimmed = id.trim();
    if trimmed.is_empty() || trimmed != id || trimmed.len() > MAX_SESSION_ID_LEN {
        return false;
    }

    if trimmed
        .chars()
        .any(|ch| ch.is_control() || ch.is_whitespace())
    {
        return false;
    }

    match runner {
        Runner::Opencode => trimmed.starts_with("ses"),
        _ => true,
    }
}

fn extract_pi_session_id(json: &JsonValue) -> Option<&str> {
    (json.get("type").and_then(|v| v.as_str()) == Some("session"))
        .then(|| json.get("id").and_then(|v| v.as_str()))?
}

fn extract_claude_session_id(json: &JsonValue) -> Option<&str> {
    let event_type = json.get("type").and_then(|v| v.as_str())?;
    let subtype = json.get("subtype").and_then(|v| v.as_str());
    matches!(
        (event_type, subtype),
        ("system", Some("init")) | ("session", _)
    )
    .then(|| json.get("session_id").and_then(|v| v.as_str()))?
}

fn extract_gemini_session_id(json: &JsonValue) -> Option<&str> {
    let event_type = json.get("type").and_then(|v| v.as_str())?;
    matches!(event_type, "session" | "session_started" | "system")
        .then(|| json.get("session_id").and_then(|v| v.as_str()))?
}

fn extract_codex_session_id(json: &JsonValue) -> Option<&str> {
    let event_type = json.get("type").and_then(|v| v.as_str())?;
    matches!(event_type, "thread.started" | "session.started" | "session").then(|| {
        json.get("thread_id")
            .or_else(|| json.get("id"))
            .and_then(|v| v.as_str())
    })?
}

fn extract_opencode_session_id(json: &JsonValue) -> Option<&str> {
    let event_type = json.get("type").and_then(|v| v.as_str())?;
    matches!(
        event_type,
        "session" | "session.started" | "session.updated"
    )
    .then(|| {
        json.get("sessionID")
            .or_else(|| json.get("session_id"))
            .and_then(|v| v.as_str())
    })?
}

fn extract_cursor_session_id(json: &JsonValue) -> Option<&str> {
    let event_type = json.get("type").and_then(|v| v.as_str())?;
    let subtype = json.get("subtype").and_then(|v| v.as_str());
    matches!(
        (event_type, subtype),
        ("system", Some("init")) | ("session", _) | ("session.started", _)
    )
    .then(|| json.get("session_id").and_then(|v| v.as_str()))?
}

fn extract_plugin_session_id(json: &JsonValue) -> Option<&str> {
    (json.get("type").and_then(|v| v.as_str()) == Some("finish"))
        .then(|| json.get("session_id").and_then(|v| v.as_str()))?
}
