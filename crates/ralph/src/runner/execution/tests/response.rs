//! Response extraction tests for runner outputs.

use super::super::extract_final_assistant_response;

#[test]
fn extract_final_assistant_response_codex_agent_message() {
    let stdout = concat!(
        r#"{"type":"item.completed","item":{"type":"agent_message","text":"Draft"}}"#,
        "\n",
        r#"{"type":"item.completed","item":{"type":"agent_message","text":"Final answer"}}"#,
        "\n"
    );
    assert_eq!(
        extract_final_assistant_response(stdout),
        Some("Final answer".to_string())
    );
}

#[test]
fn extract_final_assistant_response_claude_assistant_message() {
    let stdout = concat!(
        r#"{"type":"assistant","message":{"content":[{"type":"text","text":"First line"},{"type":"tool_use","name":"Read"}]}}"#,
        "\n"
    );
    assert_eq!(
        extract_final_assistant_response(stdout),
        Some("First line".to_string())
    );
}

#[test]
fn extract_final_assistant_response_gemini_message_assistant() {
    let stdout = concat!(
        r#"{"type":"message","role":"assistant","content":[{"text":"Hello"},{"text":"World"}]}"#,
        "\n"
    );
    assert_eq!(
        extract_final_assistant_response(stdout),
        Some("Hello\nWorld".to_string())
    );
}

#[test]
fn extract_final_assistant_response_opencode_text_stream() {
    let stdout = concat!(
        r#"{"type":"text","part":{"text":"Hello "}}"#,
        "\n",
        r#"{"type":"text","part":{"text":"world"}}"#,
        "\n"
    );
    assert_eq!(
        extract_final_assistant_response(stdout),
        Some("Hello world".to_string())
    );
}

#[test]
fn extract_final_assistant_response_none_when_missing() {
    let stdout = concat!(r#"{"type":"tool_use","tool_name":"read"}"#, "\n");
    assert_eq!(extract_final_assistant_response(stdout), None);
}
