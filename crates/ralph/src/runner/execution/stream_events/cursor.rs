//! Cursor-style tool call extraction.
//!
//! Responsibilities:
//! - Render nested `tool_call` envelopes used by Cursor-style runner output.
//! - Preserve tool argument summaries through shared detail formatting.
//!
//! Does not handle:
//! - Codex command items.
//! - Gemini/Kimi role-based assistant payloads.

use crate::outpututil;
use serde_json::Value as JsonValue;

use super::super::stream_tool_details::format_tool_details;

pub(super) fn collect_lines(json: &JsonValue, lines: &mut Vec<String>) {
    if json.get("type").and_then(|t| t.as_str()) != Some("tool_call") {
        return;
    }

    let Some(tool_call) = json.get("tool_call") else {
        return;
    };

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
