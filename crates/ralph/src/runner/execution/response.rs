//! Response extraction helpers for runner output streams.
//!
//! Responsibilities: parse streaming runner JSON output and extract final assistant responses.
//! Not handled: executing runners, managing processes, or validating runner configurations.
//! Invariants/assumptions: stdout lines are JSON fragments emitted by supported runners.

use std::collections::HashMap;

use serde_json::Value as JsonValue;

use crate::contracts::Runner;

use super::builtin_plugins::{
    ClaudeResponseParser, CodexResponseParser, CursorResponseParser, GeminiResponseParser,
    KimiResponseParser, OpencodeResponseParser, PiResponseParser,
};
use super::json::parse_json_line;
use super::plugin_trait::ResponseParser;

/// Registry of response parsers by runner.
pub struct ResponseParserRegistry {
    parsers: HashMap<String, Box<dyn ResponseParser>>,
}

impl Default for ResponseParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ResponseParserRegistry {
    /// Create a new registry with all built-in parsers registered.
    pub fn new() -> Self {
        let mut parsers: HashMap<String, Box<dyn ResponseParser>> = HashMap::new();

        // Register all built-in parsers
        parsers.insert("codex".to_string(), Box::new(CodexResponseParser));
        parsers.insert("claude".to_string(), Box::new(ClaudeResponseParser));
        parsers.insert("kimi".to_string(), Box::new(KimiResponseParser));
        parsers.insert("gemini".to_string(), Box::new(GeminiResponseParser));
        parsers.insert("opencode".to_string(), Box::new(OpencodeResponseParser));
        parsers.insert("pi".to_string(), Box::new(PiResponseParser));
        parsers.insert("cursor".to_string(), Box::new(CursorResponseParser));

        Self { parsers }
    }

    /// Extract the final assistant response from runner output.
    pub fn extract_final_response(&self, runner: &Runner, stdout: &str) -> Option<String> {
        let runner_id = runner.id();
        let parser = self.parsers.get(runner_id)?;

        let mut final_message: Option<String> = None;
        let mut streaming_buffer = String::new();

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Some(json) = parse_json_line(line) else {
                continue;
            };

            if let Some(text) = parser.parse(&json, &mut streaming_buffer) {
                final_message = Some(text);
            }
        }

        final_message
    }
}

// All response parsers are now imported from builtin_plugins module.

// =============================================================================
// Legacy Compatibility
// =============================================================================

/// Extract the final assistant response from stdout using the parser registry.
///
/// This is the legacy function that maintains backward compatibility.
/// New code should use ResponseParserRegistry directly.
pub(crate) fn extract_final_assistant_response(stdout: &str) -> Option<String> {
    let registry = ResponseParserRegistry::new();

    // Try each parser until we find a match
    // This maintains backward compatibility with the old behavior
    let mut final_message: Option<String> = None;
    let mut streaming_buffer = String::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some(json) = parse_json_line(line) else {
            continue;
        };

        // Try all parsers in order
        if let Some(text) = try_all_parsers(&json, &mut streaming_buffer, &registry) {
            final_message = Some(text);
        }
    }

    final_message
}

/// Try all registered parsers on a JSON value.
fn try_all_parsers(
    json: &JsonValue,
    buffer: &mut String,
    _registry: &ResponseParserRegistry,
) -> Option<String> {
    // Try each built-in parser directly
    // This is more efficient than going through the registry for the legacy path

    // Codex: item.completed with agent_message
    if let Some(text) = CodexResponseParser.parse(json, buffer) {
        return Some(text);
    }

    // Claude: assistant type with message.content
    if let Some(text) = ClaudeResponseParser.parse(json, buffer) {
        return Some(text);
    }

    // Gemini: message type with role=assistant
    if let Some(text) = GeminiResponseParser.parse(json, buffer) {
        return Some(text);
    }

    // Kimi: top-level role=assistant
    if let Some(text) = KimiResponseParser.parse(json, buffer) {
        return Some(text);
    }

    // Opencode: text type with streaming
    if let Some(text) = OpencodeResponseParser.parse(json, buffer) {
        return Some(text);
    }

    // Pi: result type
    if let Some(text) = PiResponseParser.parse(json, buffer) {
        return Some(text);
    }

    // Cursor: message_end type
    if let Some(text) = CursorResponseParser.parse(json, buffer) {
        return Some(text);
    }

    None
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_registry_extracts_codex_response() {
        let registry = ResponseParserRegistry::new();
        let runner = Runner::Codex;

        let stdout = r#"{"type":"item.completed","item":{"type":"agent_message","text":"Hello from Codex"}}"#;

        let result = registry.extract_final_response(&runner, stdout);
        assert_eq!(result, Some("Hello from Codex".to_string()));
    }

    #[test]
    fn response_registry_extracts_kimi_response() {
        let registry = ResponseParserRegistry::new();
        let runner = Runner::Kimi;

        let stdout = r#"{"role":"assistant","content":[{"type":"text","text":"Hello from Kimi"}]}"#;

        let result = registry.extract_final_response(&runner, stdout);
        assert_eq!(result, Some("Hello from Kimi".to_string()));
    }

    #[test]
    fn response_registry_extracts_claude_response() {
        let registry = ResponseParserRegistry::new();
        let runner = Runner::Claude;

        let stdout = r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Hello from Claude"}]}}"#;

        let result = registry.extract_final_response(&runner, stdout);
        assert_eq!(result, Some("Hello from Claude".to_string()));
    }

    #[test]
    fn response_registry_extracts_cursor_result_event() {
        let registry = ResponseParserRegistry::new();
        let runner = Runner::Cursor;

        let stdout = r#"{"type":"result","result":"Hello from Cursor"}"#;

        let result = registry.extract_final_response(&runner, stdout);
        assert_eq!(result, Some("Hello from Cursor".to_string()));
    }

    #[test]
    fn response_registry_extracts_pi_message_end_assistant() {
        let registry = ResponseParserRegistry::new();
        let runner = Runner::Pi;

        let stdout = r#"{"type":"message_end","message":{"role":"assistant","content":[{"type":"text","text":"Hello from Pi"}]}}"#;

        let result = registry.extract_final_response(&runner, stdout);
        assert_eq!(result, Some("Hello from Pi".to_string()));
    }

    #[test]
    fn legacy_extract_final_response_works() {
        let stdout =
            r#"{"type":"item.completed","item":{"type":"agent_message","text":"Legacy response"}}"#;

        let result = extract_final_assistant_response(stdout);
        assert_eq!(result, Some("Legacy response".to_string()));
    }

    #[test]
    fn opencode_response_parser_accumulates_streaming_text() {
        let parser = OpencodeResponseParser;
        let mut buffer = String::new();

        let line1 = r#"{"type":"text","part":{"text":"Hello "}}"#;
        let line2 = r#"{"type":"text","part":{"text":"World"}}"#;

        let result1 = parser.parse(&serde_json::from_str(line1).unwrap(), &mut buffer);
        assert_eq!(result1, Some("Hello ".to_string()));

        let result2 = parser.parse(&serde_json::from_str(line2).unwrap(), &mut buffer);
        assert_eq!(result2, Some("Hello World".to_string()));
    }
}
