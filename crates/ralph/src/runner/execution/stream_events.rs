//! JSON event normalization and display-line extraction for runner streams.
//!
//! Responsibilities:
//! - Correlate tool-use and tool-result events across runner formats.
//! - Convert runner-specific JSON payloads into display lines.
//! - Format compact tool-call details for terminal/handler rendering.
//!
//! Does not handle:
//! - Reading bytes from subprocess streams.
//! - Writing output to sinks or handlers.
//!
//! Assumptions/invariants:
//! - JSON values are best-effort and may be partially populated.

use crate::outpututil;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use super::stream_tool_details::{
    format_codex_command_line, format_codex_tool_line, format_tool_details,
};

#[derive(Default)]
pub(super) struct ToolCallTracker {
    tool_name_by_id: HashMap<String, String>,
}

impl ToolCallTracker {
    pub(super) fn correlate(&mut self, json: &mut JsonValue) {
        if let Some(event_type) = json.get("type").and_then(|t| t.as_str()) {
            if event_type == "tool_use" {
                if let (Some(tool_id), Some(tool_name)) = (
                    json.get("tool_id").and_then(|v| v.as_str()),
                    json.get("tool_name").and_then(|v| v.as_str()),
                ) {
                    self.tool_name_by_id
                        .insert(tool_id.to_string(), tool_name.to_string());
                }
            } else if event_type == "tool_result" {
                let tool_id = json.get("tool_id").and_then(|v| v.as_str());
                if let Some(tool_id) = tool_id
                    && let Some(tool_name) = self.tool_name_by_id.remove(tool_id)
                    && let Some(obj) = json.as_object_mut()
                {
                    obj.insert("tool_name".to_string(), JsonValue::String(tool_name));
                }
            }
        }

        if let Some(role) = json.get("role").and_then(|r| r.as_str())
            && role == "assistant"
            && let Some(tool_calls) = json.get("tool_calls").and_then(|c| c.as_array())
        {
            for tool_call in tool_calls {
                if let (Some(tool_id), Some(function)) = (
                    tool_call.get("id").and_then(|v| v.as_str()),
                    tool_call.get("function"),
                ) && let Some(tool_name) = function.get("name").and_then(|v| v.as_str())
                {
                    self.tool_name_by_id
                        .insert(tool_id.to_string(), tool_name.to_string());
                }
            }
        }

        if let Some(role) = json.get("role").and_then(|r| r.as_str())
            && role == "tool"
            && let Some(tool_call_id) = json.get("tool_call_id").and_then(|v| v.as_str())
            && let Some(tool_name) = self.tool_name_by_id.remove(tool_call_id)
            && let Some(obj) = json.as_object_mut()
        {
            obj.insert("tool_name".to_string(), JsonValue::String(tool_name));
        }
    }
}

pub(crate) fn extract_display_lines(json: &JsonValue) -> Vec<String> {
    let mut lines = Vec::new();

    if let Some(result) = json.get("result").and_then(|r| r.as_str())
        && !result.is_empty()
    {
        lines.push(result.to_string());
    }

    if let Some(event_type) = json.get("type").and_then(|t| t.as_str()) {
        if event_type == "assistant"
            && let Some(message) = json.get("message")
            && let Some(content) = message.get("content").and_then(|c| c.as_array())
        {
            for item in content {
                if let Some(item_type) = item.get("type").and_then(|t| t.as_str()) {
                    match item_type {
                        "text" => {
                            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                                lines.push(text.to_string());
                            }
                        }
                        "thinking" | "analysis" | "reasoning" => {
                            if let Some(text) = item.get("text").and_then(|t| t.as_str())
                                && !text.is_empty()
                            {
                                lines.push(outpututil::format_reasoning(text));
                            }
                        }
                        "tool_use" => {
                            if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                                let details = item.get("input").and_then(format_tool_details);
                                lines.push(outpututil::format_tool_call(name, details.as_deref()));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if (event_type == "item.completed" || event_type == "item.started")
            && let Some(item) = json.get("item")
            && let Some(item_type) = item.get("type").and_then(|t| t.as_str())
        {
            match item_type {
                "agent_message" => {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str())
                        && !text.is_empty()
                    {
                        lines.push(text.to_string());
                        lines.push(String::new());
                    }
                }
                "reasoning" => {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str())
                        && !text.is_empty()
                    {
                        lines.push(outpututil::format_reasoning(text));
                    }
                }
                "mcp_tool_call" => {
                    if let Some(line) = format_codex_tool_line(item) {
                        lines.push(line);
                    }
                }
                "command_execution" => {
                    if let Some(line) = format_codex_command_line(item) {
                        lines.push(line);
                    }
                }
                "web_search" => {
                    let query = item.get("query").and_then(|q| q.as_str()).unwrap_or("");
                    let action = item.get("action").and_then(|a| a.as_str());
                    let details = if query.is_empty() {
                        action.map(|a| format!("action={}", a))
                    } else {
                        Some(match action {
                            Some(a) => format!("query={} action={}", query, a),
                            None => format!("query={}", query),
                        })
                    };
                    lines.push(outpututil::format_tool_call(
                        "web_search",
                        details.as_deref(),
                    ));
                }
                "collab_tool_call" => {
                    if let Some(tool) = item.get("tool").and_then(|t| t.as_str()) {
                        let status = item.get("status").and_then(|s| s.as_str());
                        let details = status.map(|s| format!("({})", s));
                        lines.push(outpututil::format_tool_call(
                            &format!("collab.{}", tool),
                            details.as_deref(),
                        ));
                    }
                }
                "error" => {
                    if let Some(message) = item.get("message").and_then(|m| m.as_str())
                        && !message.trim().is_empty()
                    {
                        lines.push(format!("[Error] {}", message));
                    }
                }
                _ => {}
            }
        }

        if event_type == "text"
            && let Some(text) = json
                .get("part")
                .and_then(|p| p.get("text"))
                .and_then(|t| t.as_str())
        {
            lines.push(text.to_string());
        }

        if event_type == "reasoning"
            && let Some(text) = json
                .get("part")
                .and_then(|p| p.get("text"))
                .and_then(|t| t.as_str())
            && !text.is_empty()
        {
            lines.push(outpututil::format_reasoning(text));
        }

        if event_type == "error"
            && let Some(message) = json.get("message").and_then(|m| m.as_str())
            && !message.trim().is_empty()
        {
            lines.push(format!("[Error] {}", message));
        }

        if event_type == "tool_use"
            && let Some(tool) = json
                .get("part")
                .and_then(|p| p.get("tool"))
                .and_then(|t| t.as_str())
        {
            let status = json
                .get("part")
                .and_then(|p| p.get("state"))
                .and_then(|s| s.get("status"))
                .and_then(|s| s.as_str());
            let status_suffix = status.map(|value| format!("({value})"));
            let details = json
                .get("part")
                .and_then(|p| {
                    p.get("state")
                        .and_then(|s| s.get("input"))
                        .or_else(|| p.get("input"))
                })
                .and_then(format_tool_details);
            let full_details = match (status_suffix.as_deref(), details.as_deref()) {
                (None, None) => None,
                (None, Some(d)) => Some(d.to_string()),
                (Some(s), None) => Some(s.to_string()),
                (Some(s), Some(d)) => Some(format!("{} {}", s, d)),
            };
            lines.push(outpututil::format_tool_call(tool, full_details.as_deref()));
        }

        if event_type == "message" {
            let role = json.get("role").and_then(|r| r.as_str());
            if role == Some("assistant")
                && let Some(content) = json.get("content")
            {
                match content {
                    JsonValue::String(text) => {
                        if !text.is_empty() {
                            lines.push(text.clone());
                        }
                    }
                    JsonValue::Array(items) => {
                        for item in items {
                            if let Some(text) = item.get("text").and_then(|t| t.as_str())
                                && !text.is_empty()
                            {
                                lines.push(text.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if event_type == "message_end"
            && let Some(message) = json.get("message")
        {
            let role = message.get("role").and_then(|r| r.as_str());
            match role {
                Some("assistant") => {
                    if let Some(content) = message.get("content") {
                        match content {
                            JsonValue::String(text) => {
                                if !text.is_empty() {
                                    lines.push(text.clone());
                                }
                            }
                            JsonValue::Array(items) => {
                                for item in items {
                                    if let Some(text) = item.get("text").and_then(|t| t.as_str())
                                        && !text.is_empty()
                                    {
                                        lines.push(text.to_string());
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Some("toolResult") => {
                    let tool = message
                        .get("toolName")
                        .and_then(|t| t.as_str())
                        .unwrap_or("tool");
                    let is_error = message
                        .get("isError")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let status = if is_error { "error" } else { "completed" };
                    lines.push(outpututil::format_tool_call(
                        tool,
                        Some(&format!("({status})")),
                    ));
                }
                _ => {}
            }
        }

        if event_type == "result"
            && json
                .get("is_error")
                .and_then(|flag| flag.as_bool())
                .unwrap_or(false)
        {
            if let Some(errors) = json.get("errors").and_then(|e| e.as_array()) {
                for error in errors {
                    if let Some(message) = error.as_str()
                        && !message.trim().is_empty()
                    {
                        lines.push(format!("[Error] {}", message));
                    }
                }
            } else if let Some(message) = json
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                && !message.trim().is_empty()
            {
                lines.push(format!("[Error] {}", message));
            }
        }

        if event_type == "tool_call"
            && let Some(tool_call) = json.get("tool_call")
        {
            if let Some(mcp) = tool_call.get("mcpToolCall") {
                if let Some(args) = mcp.get("args") {
                    let tool_name = args
                        .get("providerIdentifier")
                        .and_then(|v| v.as_str())
                        .and_then(|provider| {
                            args.get("toolName")
                                .and_then(|v| v.as_str())
                                .map(|name| format!("{provider}.{name}"))
                        })
                        .or_else(|| {
                            args.get("name")
                                .and_then(|v| v.as_str())
                                .map(|name| name.to_string())
                        });
                    if let Some(tool_name) = tool_name {
                        let details = args.get("args").and_then(format_tool_details);
                        lines.push(outpututil::format_tool_call(&tool_name, details.as_deref()));
                    }
                }
            } else if let Some(shell) = tool_call.get("shellToolCall")
                && let Some(args) = shell.get("args")
            {
                let details = format_tool_details(args);
                lines.push(outpututil::format_tool_call("shell", details.as_deref()));
            }
        }

        if event_type == "tool_use"
            && let Some(tool) = json.get("tool_name").and_then(|t| t.as_str())
        {
            let details = json.get("parameters").and_then(format_tool_details);
            lines.push(outpututil::format_tool_call(tool, details.as_deref()));
        }

        if event_type == "tool_result"
            && let Some(tool) = json.get("tool_name").and_then(|t| t.as_str())
        {
            let status = json
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("completed");
            lines.push(outpututil::format_tool_call(
                tool,
                Some(&format!("({status})")),
            ));
        }
    }

    if let Some(denials) = json.get("permission_denials").and_then(|d| d.as_array()) {
        for denial in denials {
            if let Some(tool_name) = denial.get("tool_name").and_then(|t| t.as_str()) {
                lines.push(outpututil::format_permission_denied(tool_name));
            }
        }
    }

    if let Some(role) = json.get("role").and_then(|r| r.as_str())
        && role == "assistant"
        && let Some(content) = json.get("content").and_then(|c| c.as_array())
    {
        for item in content {
            if let Some(item_type) = item.get("type").and_then(|t| t.as_str()) {
                match item_type {
                    "text" => {
                        if let Some(text) = item.get("text").and_then(|t| t.as_str())
                            && !text.is_empty()
                        {
                            lines.push(text.to_string());
                        }
                    }
                    "think" => {
                        if let Some(think) = item.get("think").and_then(|t| t.as_str())
                            && !think.is_empty()
                        {
                            lines.push(outpututil::format_reasoning(think));
                        }
                    }
                    _ => {}
                }
            }
        }
        if let Some(tool_calls) = json.get("tool_calls").and_then(|c| c.as_array()) {
            for tool_call in tool_calls {
                if let Some(function) = tool_call.get("function")
                    && let Some(name) = function.get("name").and_then(|n| n.as_str())
                {
                    let details = function
                        .get("arguments")
                        .and_then(|a| a.as_str())
                        .and_then(|args_str| {
                            serde_json::from_str::<JsonValue>(args_str)
                                .inspect_err(|e| {
                                    log::trace!("Failed to parse tool arguments JSON: {}", e)
                                })
                                .ok()
                        })
                        .and_then(|args_json| format_tool_details(&args_json));
                    lines.push(outpututil::format_tool_call(name, details.as_deref()));
                }
            }
        }
    }

    if let Some(role) = json.get("role").and_then(|r| r.as_str())
        && role == "tool"
    {
        let tool_name = json
            .get("tool_name")
            .and_then(|t| t.as_str())
            .unwrap_or("Tool");
        lines.push(outpututil::format_tool_call(tool_name, Some("(completed)")));
    }

    lines
}
