//! CI supervision tests extracted from the production module.
//!
//! Responsibilities:
//! - Cover CI pattern detection, compliance messaging, and continue-session behavior.
//! - Keep large scenario suites out of `ci.rs`.
//!
//! Does not handle:
//! - Production CI execution logic.

use super::*;
use crate::contracts::{
    AgentConfig, CiGateConfig, Config, NotificationConfig, QueueConfig, Runner, RunnerRetryConfig,
};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

fn write_repo_trust(repo_root: &std::path::Path) {
    let ralph_dir = repo_root.join(".ralph");
    fs::create_dir_all(&ralph_dir).unwrap();
    fs::write(
        ralph_dir.join("trust.jsonc"),
        r#"{
  "allow_project_commands": true,
  "trusted_at": "2026-03-07T00:00:00Z"
}"#,
    )
    .unwrap();
}

fn resolved_with_ci_command(
    repo_root: &std::path::Path,
    command: Option<String>,
    enabled: bool,
) -> crate::config::Resolved {
    let argv = command.map(|command| {
        let script_name = if cfg!(windows) {
            "ci-gate-test.cmd"
        } else {
            "ci-gate-test.sh"
        };
        let script_path = repo_root.join(script_name);
        let script = if cfg!(windows) {
            format!("@echo off\r\n{command}\r\n")
        } else {
            format!("#!/bin/sh\nset -e\n{command}\n")
        };
        fs::write(&script_path, script).expect("write CI gate test script");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script_path)
                .expect("script metadata")
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script_path, perms).expect("set script permissions");
        }
        vec![script_path.to_string_lossy().to_string()]
    });

    let cfg = Config {
        agent: AgentConfig {
            runner: Some(Runner::Codex),
            model: Some(crate::contracts::Model::Gpt52Codex),
            reasoning_effort: Some(crate::contracts::ReasoningEffort::Medium),
            iterations: Some(1),
            followup_reasoning_effort: None,
            codex_bin: Some("codex".to_string()),
            opencode_bin: Some("opencode".to_string()),
            gemini_bin: Some("gemini".to_string()),
            claude_bin: Some("claude".to_string()),
            cursor_bin: Some("agent".to_string()),
            kimi_bin: Some("kimi".to_string()),
            pi_bin: Some("pi".to_string()),
            claude_permission_mode: Some(crate::contracts::ClaudePermissionMode::BypassPermissions),
            runner_cli: None,
            phase_overrides: None,
            instruction_files: None,
            repoprompt_plan_required: Some(false),
            repoprompt_tool_injection: Some(false),
            ci_gate: Some(CiGateConfig {
                enabled: Some(enabled),
                argv: argv.or_else(|| Some(vec!["make".to_string(), "ci".to_string()])),
            }),
            git_revert_mode: Some(crate::contracts::GitRevertMode::Disabled),
            git_commit_push_enabled: Some(true),
            phases: Some(2),
            notification: NotificationConfig {
                enabled: Some(false),
                ..NotificationConfig::default()
            },
            webhook: crate::contracts::WebhookConfig::default(),
            runner_retry: RunnerRetryConfig::default(),
            session_timeout_hours: None,
            scan_prompt_version: None,
        },
        queue: QueueConfig {
            file: Some(PathBuf::from(".ralph/queue.json")),
            done_file: Some(PathBuf::from(".ralph/done.json")),
            id_prefix: Some("RQ".to_string()),
            id_width: Some(4),
            size_warning_threshold_kb: Some(500),
            task_count_warning_threshold: Some(500),
            max_dependency_depth: Some(10),
            auto_archive_terminal_after_days: None,
            aging_thresholds: None,
        },
        ..Config::default()
    };

    crate::config::Resolved {
        config: cfg,
        repo_root: repo_root.to_path_buf(),
        queue_path: repo_root.join(".ralph/queue.json"),
        done_path: repo_root.join(".ralph/done.json"),
        id_prefix: "RQ".to_string(),
        id_width: 4,
        global_config_path: None,
        project_config_path: Some(repo_root.join(".ralph/config.json")),
    }
}

#[test]
fn ci_gate_command_label_returns_default() {
    let temp = TempDir::new().unwrap();
    let resolved = resolved_with_ci_command(temp.path(), None, true);
    assert_eq!(ci_gate_command_label(&resolved), "make ci");
}

#[test]
fn ci_gate_command_label_returns_custom() {
    let temp = TempDir::new().unwrap();
    let mut resolved = resolved_with_ci_command(temp.path(), None, true);
    resolved.config.agent.ci_gate = Some(CiGateConfig {
        enabled: Some(true),
        argv: Some(vec!["cargo".to_string(), "test".to_string()]),
    });
    assert_eq!(ci_gate_command_label(&resolved), "cargo test");
}

#[test]
fn run_ci_gate_skips_when_disabled() -> Result<()> {
    let temp = TempDir::new()?;
    let resolved = resolved_with_ci_command(temp.path(), Some("make ci".to_string()), false);
    // Should succeed without running anything, returning success
    let result = run_ci_gate(&resolved)?;
    assert!(result.success);
    Ok(())
}

#[test]
fn run_ci_gate_errors_on_empty_command() {
    let temp = TempDir::new().unwrap();
    write_repo_trust(temp.path());
    let mut resolved = resolved_with_ci_command(temp.path(), None, true);
    resolved.config.agent.ci_gate = Some(CiGateConfig {
        enabled: Some(true),
        argv: Some(vec!["".to_string()]),
    });
    let err = run_ci_gate(&resolved).unwrap_err();
    assert!(format!("{err:#}").contains("CI gate argv entries must be non-empty"));
}

#[test]
fn run_ci_gate_captures_output() -> Result<()> {
    let temp = TempDir::new()?;
    let command = "python3 -c \"import sys; print('stdout text'); print('stderr text', file=sys.stderr); raise SystemExit(1)\"";
    write_repo_trust(temp.path());
    let resolved = resolved_with_ci_command(temp.path(), Some(command.to_string()), true);
    let err = run_ci_gate(&resolved).unwrap_err();

    // CI failure now returns Err(CiFailure)
    let ci_failure = err.downcast::<CiFailure>().unwrap();
    assert_eq!(ci_failure.exit_code, Some(1));
    assert!(ci_failure.stdout.contains("stdout text"));
    assert!(ci_failure.stderr.contains("stderr text"));
    Ok(())
}

#[test]
fn format_ci_output_includes_stderr_first() {
    let stdout = "line1\nline2\nline3";
    let stderr = "error1\nerror2";
    let result = format_ci_output_for_message(stdout, stderr, 50, 50);

    // stderr should appear in output
    assert!(result.contains("error1"));
    assert!(result.contains("error2"));
}

#[test]
fn format_ci_output_shows_head_and_tail() {
    let stdout = (1..=200)
        .map(|i| format!("line{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let stderr = "";

    // Request 50 head + 50 tail
    let result = format_ci_output_for_message(&stdout, stderr, 50, 50);

    // Should show total line count
    assert!(result.contains("200 lines total"));

    // Should show explicit line ranges
    assert!(result.contains("showing lines 1-50 and 151-200"));

    // Should include early lines (format/lint errors appear here)
    assert!(result.contains("line1"));
    assert!(result.contains("line50"));

    // Should include late lines (test failures appear here)
    assert!(result.contains("line151"));
    assert!(result.contains("line200"));

    // Should NOT include middle lines
    assert!(!result.contains("line51"));
    assert!(!result.contains("line100"));
    assert!(!result.contains("line150"));

    // Should indicate truncation
    assert!(result.contains("100 lines omitted"));
}

#[test]
fn format_ci_output_shows_all_when_small() {
    let stdout = "line1\nline2\nline3";
    let stderr = "";

    let result = format_ci_output_for_message(stdout, stderr, 50, 50);

    // Should show all without truncation
    assert!(result.contains("3 lines)"));
    assert!(result.contains("line1"));
    assert!(result.contains("line3"));
    assert!(!result.contains("omitted"));
}

#[test]
fn format_ci_output_handles_empty() {
    let result = format_ci_output_for_message("", "", 50, 50);
    assert!(result.contains("No output captured"));
}

#[test]
fn compliance_message_includes_exit_code_and_output() {
    let temp = TempDir::new().unwrap();
    let resolved = resolved_with_ci_command(temp.path(), None, true);
    let result = CiGateResult {
        success: false,
        exit_code: Some(2),
        stdout: "test output".to_string(),
        stderr: "error: ruff failed".to_string(),
    };

    let msg = strict_ci_gate_compliance_message(&resolved, &result);
    // Should show numeric exit code, not Debug format like "Some(2)"
    assert!(
        msg.contains("exit code 2"),
        "Expected 'exit code 2', got: {msg}"
    );
    assert!(msg.contains("ruff failed"));
}

#[test]
fn compliance_message_includes_formatted_ci_output_with_ranges() {
    let temp = TempDir::new().unwrap();
    let resolved = resolved_with_ci_command(temp.path(), None, true);

    // Create large output that will be truncated
    let stdout = (1..=200)
        .map(|i| format!("out-{i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let stderr = (1..=10)
        .map(|i| format!("err-{i}"))
        .collect::<Vec<_>>()
        .join("\n");

    let result = CiGateResult {
        success: false,
        exit_code: Some(1),
        stdout,
        stderr,
    };

    let msg = strict_ci_gate_compliance_message(&resolved, &result);

    // Should include formatted output with line ranges
    assert!(
        msg.contains("lines total"),
        "Should show total lines in message"
    );
    assert!(
        msg.contains("showing lines"),
        "Should show explicit line ranges"
    );
    assert!(
        msg.contains("err-1"),
        "Should include early stderr in output"
    );
    assert!(
        msg.contains("out-200"),
        "Should include late stdout in output"
    );
    assert!(
        msg.contains("lines omitted"),
        "Should indicate truncation when output is large"
    );
    assert!(
        msg.contains("Fix the errors above before continuing."),
        "Should include enforcement guidance"
    );
}

#[test]
fn format_ci_output_handles_zero_head_budget() {
    let stdout = (1..=8)
        .map(|i| format!("line{i}"))
        .collect::<Vec<_>>()
        .join("\n");

    let result = format_ci_output_for_message(&stdout, "", 0, 3);

    assert!(result.contains("8 lines total"));
    assert!(result.contains("showing lines 6-8"));
    assert!(result.contains("line6"));
    assert!(result.contains("line8"));
    assert!(result.contains("5 lines omitted"));
    assert!(!result.contains("1-0"));
}

#[test]
fn format_ci_output_handles_zero_tail_budget() {
    let stdout = (1..=8)
        .map(|i| format!("line{i}"))
        .collect::<Vec<_>>()
        .join("\n");

    let result = format_ci_output_for_message(&stdout, "", 3, 0);

    assert!(result.contains("8 lines total"));
    assert!(result.contains("showing lines 1-3"));
    assert!(result.contains("line1"));
    assert!(result.contains("line3"));
    assert!(result.contains("5 lines omitted"));
    assert!(!result.contains("9-8"));
}

#[test]
fn format_ci_output_handles_zero_total_budget() {
    let stdout = "line1\nline2\nline3";

    let result = format_ci_output_for_message(stdout, "", 0, 0);

    assert!(result.contains("3 lines total; snippet budget is 0 lines"));
    assert!(result.contains("3 lines omitted"));
    assert!(!result.contains("```"));
}

#[test]
fn compliance_message_orders_output_before_enforcement_text() {
    let temp = TempDir::new().unwrap();
    let resolved = resolved_with_ci_command(temp.path(), None, true);
    let result = CiGateResult {
        success: false,
        exit_code: Some(2),
        stdout: "out-1\nout-2".to_string(),
        stderr: "err-1".to_string(),
    };

    let msg = strict_ci_gate_compliance_message(&resolved, &result);

    let output_idx = msg
        .find("CI output (")
        .expect("message should include CI output snippet");
    let fix_idx = msg
        .find("Fix the errors above before continuing.")
        .expect("message should include enforcement text");

    assert!(
        output_idx < fix_idx,
        "output snippet should appear before enforcement guidance"
    );
}

#[test]
fn build_ci_failure_message_with_user_input_includes_ci_output() {
    let temp = TempDir::new().unwrap();
    let resolved = resolved_with_ci_command(temp.path(), None, true);
    let result = CiGateResult {
        success: false,
        exit_code: Some(1),
        stdout: "test stdout output".to_string(),
        stderr: "ruff failed: TOML parse error".to_string(),
    };
    let user_message = "Please check the pyproject.toml file";

    let combined = build_ci_failure_message_with_user_input(&resolved, &result, user_message);

    // Should include CI output context
    assert!(
        combined.contains("CI output ("),
        "should include CI output header"
    );
    assert!(
        combined.contains("ruff failed: TOML parse error"),
        "should include stderr from CI"
    );
    assert!(combined.contains("exit code 1"), "should include exit code");

    // Should include user message
    assert!(
        combined.contains(user_message),
        "should include user message"
    );

    // Should include enforcement guidance
    assert!(
        combined.contains("Fix the errors above before continuing."),
        "should include enforcement guidance"
    );

    // CI output should come before user message
    let ci_output_idx = combined.find("CI output (").unwrap();
    let user_msg_idx = combined.find(user_message).unwrap();
    assert!(
        ci_output_idx < user_msg_idx,
        "CI output should appear before user message"
    );
}

#[test]
fn build_ci_failure_message_with_empty_user_input_returns_strict_message_only() {
    let temp = TempDir::new().unwrap();
    let resolved = resolved_with_ci_command(temp.path(), None, true);
    let result = CiGateResult {
        success: false,
        exit_code: Some(1),
        stdout: "test stdout output".to_string(),
        stderr: "ruff failed: TOML parse error".to_string(),
    };

    let combined = build_ci_failure_message_with_user_input(&resolved, &result, " \n\t ");
    let strict = strict_ci_gate_compliance_message(&resolved, &result);

    assert_eq!(combined, strict);
    assert!(!combined.contains("Agent message from user intervention:"));
}

#[test]
fn compliance_message_includes_troubleshooting_patterns() {
    let temp = TempDir::new().unwrap();
    let resolved = resolved_with_ci_command(temp.path(), None, true);
    let result = CiGateResult {
        success: false,
        exit_code: Some(2),
        stdout: String::new(),
        stderr: String::new(),
    };

    let msg = strict_ci_gate_compliance_message(&resolved, &result);
    assert!(msg.contains("TOML parse error"));
    assert!(msg.contains("unknown variant"));
    assert!(msg.contains("format-check failed"));
    assert!(msg.contains("lint-check failed"));
}

#[test]
fn compliance_message_contains_required_enforcement_language() {
    let temp = TempDir::new().unwrap();
    let resolved = resolved_with_ci_command(temp.path(), None, true);
    let result = CiGateResult {
        success: false,
        exit_code: Some(2),
        stdout: "fmt-check passed".to_string(),
        stderr: "ruff failed: TOML parse error".to_string(),
    };

    let msg = strict_ci_gate_compliance_message(&resolved, &result);
    assert!(
        msg.contains("CI gate (make ci): CI failed with exit code 2"),
        "Expected CI gate prefix with exit code, got: {msg}"
    );
    assert!(
        msg.contains("Fix the errors above before continuing."),
        "Expected remediation instruction, got: {msg}"
    );
    assert!(
        msg.contains("COMMON PATTERNS:"),
        "Expected COMMON PATTERNS section, got: {msg}"
    );
    assert!(
        msg.contains("ruff failed: TOML parse error"),
        "Expected CI output context in message, got: {msg}"
    );
}

#[test]
fn truncate_for_log_shows_end_of_string() {
    let long = "a".repeat(3000);
    let truncated = truncate_for_log(&long, 100);
    assert!(truncated.starts_with("..."));
    // Should have exactly 103 characters: "..." + 100 'a's
    assert_eq!(truncated.len(), 103);
}

#[test]
fn truncate_for_log_returns_full_if_short() {
    let short = "hello world";
    let truncated = truncate_for_log(short, 100);
    assert_eq!(truncated, short);
}

#[test]
fn truncate_for_log_handles_multibyte_utf8() {
    // Test with multi-byte UTF-8 characters (emoji = 4 bytes each)
    let long = "😀".repeat(100); // 100 emoji = 400 bytes
    let truncated = truncate_for_log(&long, 10); // Keep last 10 chars

    // Should not panic and should produce valid UTF-8
    assert!(truncated.starts_with("..."));
    // After "...", should have exactly 10 emoji characters
    let emoji_part = &truncated[3..]; // Skip "..."
    assert_eq!(emoji_part.chars().count(), 10);
}

#[test]
fn truncate_for_log_handles_empty_string() {
    let truncated = truncate_for_log("", 100);
    assert_eq!(truncated, "");
}

// ========================================================================
// CiFailure Tests
// ========================================================================

#[test]
fn ci_failure_display_includes_exit_code() {
    let failure = CiFailure {
        exit_code: Some(1),
        stdout: String::new(),
        stderr: String::new(),
        error_pattern: None,
    };

    let msg = failure.to_string();
    assert!(
        msg.contains("exit code 1"),
        "Expected 'exit code 1', got: {msg}"
    );
}

#[test]
fn ci_failure_display_includes_error_pattern() {
    let failure = CiFailure {
        exit_code: Some(1),
        stdout: String::new(),
        stderr: String::new(),
        error_pattern: Some("TOML parse error"),
    };

    let msg = failure.to_string();
    assert!(
        msg.contains("[TOML parse error]"),
        "Expected pattern, got: {msg}"
    );
}

#[test]
fn ci_failure_display_includes_truncated_output() {
    let failure = CiFailure {
        exit_code: Some(1),
        stdout: "test output".to_string(),
        stderr: "TOML parse error at line 44".to_string(),
        error_pattern: Some("TOML parse error"),
    };

    let msg = failure.to_string();
    assert!(
        msg.contains(">>> stderr:"),
        "Expected stderr section, got: {msg}"
    );
    assert!(
        msg.contains(">>> stdout:"),
        "Expected stdout section, got: {msg}"
    );
    assert!(
        msg.contains("TOML parse error at line 44"),
        "Expected error message, got: {msg}"
    );
}

#[test]
fn ci_failure_truncates_long_output() {
    let long_output = "x".repeat(1000);
    let failure = CiFailure {
        exit_code: Some(1),
        stdout: long_output.clone(),
        stderr: String::new(),
        error_pattern: None,
    };

    let msg = failure.to_string();
    // Should be truncated, not full 1000 chars
    assert!(
        msg.len() < 800,
        "Message should be truncated, got length {}",
        msg.len()
    );
    assert!(
        msg.contains("..."),
        "Expected truncation marker, got: {msg}"
    );
}

#[test]
fn ci_failure_handles_missing_exit_code() {
    let failure = CiFailure {
        exit_code: None,
        stdout: String::new(),
        stderr: String::new(),
        error_pattern: None,
    };

    let msg = failure.to_string();
    assert!(
        msg.contains("exit code -1"),
        "Expected -1 for missing exit code, got: {msg}"
    );
}

// ========================================================================
// Pattern Detection Tests
// ========================================================================

#[test]
fn detect_toml_parse_error_extracts_line_number() {
    let output = "ruff failed: TOML parse error at line 44, column 18: unknown variant `py314`";
    let pattern = detect_toml_parse_error(output).unwrap();
    assert_eq!(pattern.line_number, Some(44));
    assert_eq!(pattern.pattern_type, "TOML parse error");
}

#[test]
fn detect_toml_parse_error_returns_none_for_non_toml() {
    let output = "Some random error message";
    assert!(detect_toml_parse_error(output).is_none());
}

#[test]
fn detect_unknown_variant_extracts_values() {
    let output =
        "unknown variant `py314`, expected one of py37, py38, py39, py310, py311, py312, py313";
    let pattern = detect_unknown_variant_error(output).unwrap();
    assert_eq!(pattern.invalid_value, Some("py314".to_string()));
    assert!(pattern.valid_values.unwrap().contains("py313"));
    assert_eq!(pattern.pattern_type, "Unknown variant error");
}

#[test]
fn detect_unknown_variant_returns_none_for_non_variant() {
    let output = "Some error without variant";
    assert!(detect_unknown_variant_error(output).is_none());
}

#[test]
fn detect_ruff_error_returns_pattern() {
    let output = "ruff failed with some error";
    let pattern = detect_ruff_error(output).unwrap();
    assert_eq!(pattern.pattern_type, "Ruff error");
    assert_eq!(pattern.file_path, Some("pyproject.toml".to_string()));
}

#[test]
fn detect_format_check_error_returns_pattern() {
    let output = "format-check failed";
    let pattern = detect_format_check_error(output).unwrap();
    assert_eq!(pattern.pattern_type, "Format check failure");
}

#[test]
fn detect_lint_check_error_returns_pattern() {
    let output = "lint check failed";
    let pattern = detect_lint_check_error(output).unwrap();
    assert_eq!(pattern.pattern_type, "Lint check failure");
}

#[test]
fn detect_lock_contention_error_returns_pattern() {
    let output = "Blocking waiting for file lock on build directory";
    let pattern = detect_lock_contention_error(output).unwrap();
    assert_eq!(pattern.pattern_type, "Lock contention");
}

#[test]
fn detect_ci_error_pattern_combines_stdout_stderr() {
    let stdout = "Some output";
    let stderr = "TOML parse error at line 10";
    let pattern = detect_ci_error_pattern(stdout, stderr).unwrap();
    assert_eq!(pattern.line_number, Some(10));
}

#[test]
fn detect_ci_error_pattern_returns_none_on_clean_output() {
    let output = "All tests passed!";
    assert!(detect_ci_error_pattern(output, "").is_none());
}

#[test]
fn compliance_message_includes_lock_contention_guidance() {
    let temp = TempDir::new().unwrap();
    let resolved = resolved_with_ci_command(temp.path(), None, true);
    let result = CiGateResult {
        success: false,
        exit_code: Some(1),
        stdout: String::new(),
        stderr: "Blocking waiting for file lock on build directory".to_string(),
    };

    let msg = strict_ci_gate_compliance_message(&resolved, &result);
    assert!(msg.contains("Lock contention"));
    assert!(msg.contains("waiting on a file lock"));
}

#[test]
fn extract_line_number_from_at_line_pattern() {
    let output = "Error at line 42";
    assert_eq!(extract_line_number(output), Some(42));
}

#[test]
fn extract_line_number_from_colon_pattern() {
    let output = "pyproject.toml:44:18: error";
    assert_eq!(extract_line_number(output), Some(44));
}

#[test]
fn extract_line_number_returns_none_when_not_present() {
    let output = "No line number here";
    assert!(extract_line_number(output).is_none());
}

#[test]
fn extract_invalid_value_finds_backtick_value() {
    let output = "unknown variant `py314`, expected...";
    assert_eq!(extract_invalid_value(output), Some("py314".to_string()));
}

#[test]
fn extract_invalid_value_handles_unicode_prefix() {
    let output = "İstanbul: unknown variant `py314`, expected one of py37, py313";
    assert_eq!(extract_invalid_value(output), Some("py314".to_string()));
}

#[test]
fn extract_valid_values_finds_expected_list() {
    let output = "expected one of py37, py38, py313";
    assert_eq!(
        extract_valid_values(output),
        Some("py37, py38, py313".to_string())
    );
}

#[test]
fn extract_valid_values_handles_unicode_prefix() {
    let output = "İstanbul: expected one of py37, py313.";
    assert_eq!(
        extract_valid_values(output),
        Some("py37, py313".to_string())
    );
}

#[test]
fn infer_file_path_extracts_explicit_toml_file() {
    let output = "ruff failed parsing pyproject.toml, unknown variant `py314`";
    assert_eq!(infer_file_path(output), Some("pyproject.toml".to_string()));
}

#[test]
fn infer_file_path_infers_pyproject_from_ruff_parse_context() {
    let output = "ruff failed: parse error at line 5";
    assert_eq!(infer_file_path(output), Some("pyproject.toml".to_string()));
}

#[test]
fn compliance_message_includes_detected_toml_error() {
    let temp = TempDir::new().unwrap();
    let resolved = resolved_with_ci_command(temp.path(), None, true);
    let result = CiGateResult {
        success: false,
        exit_code: Some(1),
        stdout: String::new(),
        stderr: "TOML parse error at line 44: unknown variant `py314`, expected one of py37, py313"
            .to_string(),
    };

    let msg = strict_ci_gate_compliance_message(&resolved, &result);
    assert!(
        msg.contains("DETECTED ERROR"),
        "Should contain DETECTED ERROR section"
    );
    assert!(
        msg.contains("TOML parse error"),
        "Should identify error type"
    );
    assert!(msg.contains("**Line**"), "Should show Line label");
    assert!(msg.contains("44"), "Should show line 44");
    assert!(msg.contains("py314"), "Should show invalid value");
}

#[test]
fn compliance_message_includes_detected_unknown_variant() {
    let temp = TempDir::new().unwrap();
    let resolved = resolved_with_ci_command(temp.path(), None, true);
    let result = CiGateResult {
        success: false,
        exit_code: Some(1),
        stdout: String::new(),
        stderr: "unknown variant `foo`, expected one of bar, baz".to_string(),
    };

    let msg = strict_ci_gate_compliance_message(&resolved, &result);
    assert!(
        msg.contains("DETECTED ERROR"),
        "Should contain DETECTED ERROR section"
    );
    assert!(
        msg.contains("Unknown variant error"),
        "Should identify error type"
    );
    assert!(msg.contains("`foo`"), "Should show invalid value");
    assert!(msg.contains("bar, baz"), "Should show valid options");
}

#[test]
fn compliance_message_no_detected_section_on_clean_output() {
    let temp = TempDir::new().unwrap();
    let resolved = resolved_with_ci_command(temp.path(), None, true);
    let result = CiGateResult {
        success: false,
        exit_code: Some(1),
        stdout: "build failed".to_string(),
        stderr: String::new(),
    };

    let msg = strict_ci_gate_compliance_message(&resolved, &result);
    assert!(
        !msg.contains("DETECTED ERROR"),
        "Should NOT contain DETECTED ERROR section for unrecognized errors"
    );
    // Should still have common patterns
    assert!(msg.contains("COMMON PATTERNS"));
}

#[test]
fn format_detected_pattern_includes_all_fields() {
    let pattern = DetectedErrorPattern {
        pattern_type: "Test error",
        file_path: Some("test.toml".to_string()),
        line_number: Some(10),
        invalid_value: Some("bad_value".to_string()),
        valid_values: Some("good1, good2".to_string()),
        guidance: "Fix the error",
    };

    let formatted = format_detected_pattern(&pattern);
    assert!(formatted.contains("Test error"));
    assert!(formatted.contains("test.toml"));
    assert!(formatted.contains("10"));
    assert!(formatted.contains("bad_value"));
    assert!(formatted.contains("good1, good2"));
    assert!(formatted.contains("Fix the error"));
}

// ========================================================================
// Error Pattern Key Tests
// ========================================================================

#[test]
fn get_error_pattern_key_returns_pattern_type() {
    let result = CiGateResult {
        success: false,
        exit_code: Some(1),
        stdout: String::new(),
        stderr: "TOML parse error at line 44".to_string(),
    };
    assert_eq!(
        get_error_pattern_key(&result),
        Some("TOML parse error".to_string())
    );
}

#[test]
fn get_error_pattern_key_returns_none_for_unrecognized() {
    let result = CiGateResult {
        success: false,
        exit_code: Some(1),
        stdout: "some random output".to_string(),
        stderr: String::new(),
    };
    assert_eq!(get_error_pattern_key(&result), None);
}

#[test]
fn get_error_pattern_key_detects_unknown_variant() {
    let result = CiGateResult {
        success: false,
        exit_code: Some(1),
        stdout: String::new(),
        stderr: "unknown variant `py314`, expected one of py37, py38".to_string(),
    };
    assert_eq!(
        get_error_pattern_key(&result),
        Some("Unknown variant error".to_string())
    );
}

#[test]
fn get_error_pattern_key_detects_format_check() {
    let result = CiGateResult {
        success: false,
        exit_code: Some(1),
        stdout: String::new(),
        stderr: "format-check failed".to_string(),
    };
    assert_eq!(
        get_error_pattern_key(&result),
        Some("Format check failure".to_string())
    );
}

#[test]
fn get_error_pattern_key_detects_lint_check() {
    let result = CiGateResult {
        success: false,
        exit_code: Some(1),
        stdout: String::new(),
        stderr: "lint check failed".to_string(),
    };
    assert_eq!(
        get_error_pattern_key(&result),
        Some("Lint check failure".to_string())
    );
}

#[test]
fn get_error_pattern_key_combines_stdout_stderr() {
    let result = CiGateResult {
        success: false,
        exit_code: Some(1),
        stdout: "some output".to_string(),
        stderr: "TOML parse error at line 10".to_string(),
    };
    assert_eq!(
        get_error_pattern_key(&result),
        Some("TOML parse error".to_string())
    );
}

// ========================================================================
// Table-Driven Pattern Detection Tests
// ========================================================================

#[test]
fn detect_ci_error_pattern_cases() {
    struct Case {
        stdout: &'static str,
        stderr: &'static str,
        want: Option<&'static str>,
        want_line: Option<u32>,
        want_invalid: Option<&'static str>,
    }

    let cases = [
        Case {
            stdout: "",
            stderr: "ruff failed: TOML parse error at line 44, column 18",
            want: Some("TOML parse error"),
            want_line: Some(44),
            want_invalid: None,
        },
        Case {
            stdout: "",
            stderr: "unknown variant `py314`, expected one of py37, py313",
            want: Some("Unknown variant error"),
            want_line: None,
            want_invalid: Some("py314"),
        },
        Case {
            stdout: "",
            stderr: "TOML parse error at line 10: unknown variant `foo`",
            want: Some("TOML parse error"),
            want_line: Some(10),
            want_invalid: Some("foo"),
        },
        Case {
            stdout: "TOML parse error",
            stderr: "",
            want: Some("TOML parse error"),
            want_line: None,
            want_invalid: None,
        },
        Case {
            stdout: "",
            stderr: "ruff: error checking configuration",
            want: Some("Ruff error"),
            want_line: None,
            want_invalid: None,
        },
        Case {
            stdout: "",
            stderr: "format-check failed: 3 files need formatting",
            want: Some("Format check failure"),
            want_line: None,
            want_invalid: None,
        },
        Case {
            stdout: "",
            stderr: "lint check failed with 5 errors",
            want: Some("Lint check failure"),
            want_line: None,
            want_invalid: None,
        },
        Case {
            stdout: "all good",
            stderr: "",
            want: None,
            want_line: None,
            want_invalid: None,
        },
        Case {
            stdout: "build succeeded",
            stderr: "test passed",
            want: None,
            want_line: None,
            want_invalid: None,
        },
        Case {
            stdout: "",
            stderr: "error: something went wrong",
            want: None,
            want_line: None,
            want_invalid: None,
        },
        Case {
            stdout: "",
            stderr: "pyproject.toml:100:5: error",
            want: None,
            want_line: None,
            want_invalid: None,
        },
    ];

    for case in cases {
        let got = detect_ci_error_pattern(case.stdout, case.stderr);
        assert_eq!(
            got.as_ref().map(|p| p.pattern_type),
            case.want,
            "stderr={} stdout={}",
            case.stderr,
            case.stdout
        );
        if let Some(pattern) = got {
            assert_eq!(
                pattern.line_number, case.want_line,
                "line_number mismatch for stderr={} stdout={}",
                case.stderr, case.stdout
            );
            assert_eq!(
                pattern.invalid_value.as_deref(),
                case.want_invalid,
                "invalid_value mismatch for stderr={} stdout={}",
                case.stderr,
                case.stdout
            );
        }
    }
}

#[test]
fn detect_toml_takes_precedence_over_unknown_variant() {
    let output = "TOML parse error at line 44: unknown variant `py314`";
    let pattern = detect_ci_error_pattern("", output).unwrap();
    assert_eq!(pattern.pattern_type, "TOML parse error");
    assert_eq!(pattern.line_number, Some(44));
}

#[test]
fn detect_toml_takes_precedence_over_ruff() {
    let output = "ruff failed: TOML parse error at line 50";
    let pattern = detect_ci_error_pattern("", output).unwrap();
    assert_eq!(pattern.pattern_type, "TOML parse error");
    assert_eq!(pattern.line_number, Some(50));
}

#[test]
fn detect_unknown_variant_takes_precedence_over_ruff() {
    let output = "ruff: unknown variant `bad`";
    let pattern = detect_ci_error_pattern("", output).unwrap();
    assert_eq!(pattern.pattern_type, "Unknown variant error");
}

#[test]
fn detect_format_takes_precedence_over_lint_when_both_present() {
    let pattern = detect_ci_error_pattern(
        "format-check failed: 1 file needs formatting",
        "lint check failed with 2 errors",
    )
    .unwrap();
    assert_eq!(pattern.pattern_type, "Format check failure");
}

#[test]
fn extract_valid_values_handles_period_terminator() {
    let output = "expected one of foo, bar, baz.";
    assert_eq!(
        extract_valid_values(output),
        Some("foo, bar, baz".to_string())
    );
}

#[test]
fn extract_valid_values_handles_newline_terminator() {
    let output = "expected one of a, b\nc";
    assert_eq!(extract_valid_values(output), Some("a, b".to_string()));
}

#[test]
fn extract_line_number_handles_comma_suffix() {
    let output = "at line 42, column 10";
    assert_eq!(extract_line_number(output), Some(42));
}

#[test]
fn detect_format_case_insensitive() {
    let output = "FORMAT-CHECK FAILED";
    let pattern = detect_format_check_error(output).unwrap();
    assert_eq!(pattern.pattern_type, "Format check failure");
}

#[test]
fn detect_lint_case_insensitive() {
    let output = "LINT CHECK FAILED";
    let pattern = detect_lint_check_error(output).unwrap();
    assert_eq!(pattern.pattern_type, "Lint check failure");
}

#[test]
fn detect_ruff_yields_to_toml_parse() {
    let output = "ruff failed: TOML parse error";
    let pattern = detect_ruff_error(output);
    assert!(
        pattern.is_none(),
        "ruff detector should yield to TOML parse"
    );
}

// ========================================================================
// CI Escalation Threshold Tests
// ========================================================================

fn continue_session_for_ci_tests() -> crate::commands::run::supervision::ContinueSession {
    crate::commands::run::supervision::ContinueSession {
        runner: crate::contracts::Runner::Codex,
        model: crate::contracts::Model::Gpt52Codex,
        reasoning_effort: None,
        runner_cli: crate::runner::ResolvedRunnerCliOptions::default(),
        phase_type: crate::commands::run::PhaseType::Implementation,
        session_id: Some("sess-123".to_string()),
        output_handler: None,
        output_stream: crate::runner::OutputStream::Terminal,
        ci_failure_retry_count: CI_GATE_AUTO_RETRY_LIMIT,
        task_id: "RQ-0947".to_string(),
        last_ci_error_pattern: None,
        consecutive_same_error_count: 0,
    }
}

#[test]
fn run_ci_gate_with_continue_session_escalates_on_threshold_same_pattern() -> Result<()> {
    let temp = TempDir::new()?;
    let command = "python3 -c \"import sys; print('ruff failed: TOML parse error at line 44', file=sys.stderr); raise SystemExit(1)\"";

    write_repo_trust(temp.path());
    let resolved = resolved_with_ci_command(temp.path(), Some(command.to_string()), true);
    let mut session = continue_session_for_ci_tests();
    session.ci_failure_retry_count = 0;
    session.last_ci_error_pattern = Some("TOML parse error".to_string());
    session.consecutive_same_error_count = CI_FAILURE_ESCALATION_THRESHOLD - 1;

    let err = run_ci_gate_with_continue_session(
        &resolved,
        crate::contracts::GitRevertMode::Disabled,
        None,
        &mut session,
        |_output, _elapsed| -> Result<()> { panic!("on_resume should not be called") },
        None,
    )
    .expect_err("expected escalation on repeated identical CI error");

    let msg = err.to_string();
    assert!(msg.contains("MANUAL INTERVENTION REQUIRED"));
    assert!(msg.contains("same error"));
    assert!(msg.contains("TOML parse error"));
    assert_eq!(
        session.consecutive_same_error_count,
        CI_FAILURE_ESCALATION_THRESHOLD
    );
    Ok(())
}

#[test]
fn run_ci_gate_with_continue_session_escalation_honors_continue_choice() -> Result<()> {
    let temp = TempDir::new()?;
    let command = "python3 -c \"import sys; print('format-check failed', file=sys.stderr); raise SystemExit(1)\"";

    write_repo_trust(temp.path());
    let resolved = resolved_with_ci_command(temp.path(), Some(command.to_string()), true);
    let mut resolved = resolved;
    resolved.config.agent.codex_bin = Some(
        temp.path()
            .join("missing-codex")
            .to_string_lossy()
            .to_string(),
    );
    let mut session = continue_session_for_ci_tests();
    session.session_id = None;
    session.ci_failure_retry_count = 0;
    session.last_ci_error_pattern = Some("Format check failure".to_string());
    session.consecutive_same_error_count = CI_FAILURE_ESCALATION_THRESHOLD - 1;

    let prompt_handler: crate::runutil::RevertPromptHandler = Arc::new(|context| {
        assert_eq!(context.label, "CI failure escalation");
        Ok(crate::runutil::RevertDecision::Continue {
            message: "Run the formatter and fix the test failure.".to_string(),
        })
    });

    let err = run_ci_gate_with_continue_session(
        &resolved,
        crate::contracts::GitRevertMode::Ask,
        Some(&prompt_handler),
        &mut session,
        |_output, _elapsed| -> Result<()> { panic!("on_resume should not be called") },
        None,
    )
    .expect_err("expected continue path to attempt fresh invocation and fail on missing runner");

    let msg = err.to_string();
    assert!(msg.contains("runner binary not found"));
    assert!(
        !msg.contains("MANUAL INTERVENTION REQUIRED"),
        "escalation continue path should attempt resume instead of immediate manual bailout"
    );
    Ok(())
}

#[test]
fn run_ci_gate_with_continue_session_resets_counter_when_pattern_changes() -> Result<()> {
    let temp = TempDir::new()?;
    let command = "python3 -c \"import sys; print('format-check failed', file=sys.stderr); raise SystemExit(1)\"";

    write_repo_trust(temp.path());
    let resolved = resolved_with_ci_command(temp.path(), Some(command.to_string()), true);
    let mut session = continue_session_for_ci_tests();
    session.ci_failure_retry_count = CI_GATE_AUTO_RETRY_LIMIT;
    session.last_ci_error_pattern = Some("TOML parse error".to_string());
    session.consecutive_same_error_count = CI_FAILURE_ESCALATION_THRESHOLD - 1;

    let _ = run_ci_gate_with_continue_session(
        &resolved,
        crate::contracts::GitRevertMode::Disabled,
        None,
        &mut session,
        |_output, _elapsed| -> Result<()> { panic!("on_resume should not be called") },
        None,
    )
    .expect_err("expected CI failure after counter reset path");

    assert_eq!(session.consecutive_same_error_count, 1);
    assert_eq!(
        session.last_ci_error_pattern.as_deref(),
        Some("Format check failure")
    );
    Ok(())
}

#[test]
fn run_ci_gate_with_continue_session_clears_pattern_tracking_after_success() -> Result<()> {
    let temp = TempDir::new()?;
    let command = "python3 -c \"raise SystemExit(0)\"";

    write_repo_trust(temp.path());
    let resolved = resolved_with_ci_command(temp.path(), Some(command.to_string()), true);
    let mut session = continue_session_for_ci_tests();
    session.last_ci_error_pattern = Some("TOML parse error".to_string());
    session.consecutive_same_error_count = 2;

    run_ci_gate_with_continue_session(
        &resolved,
        crate::contracts::GitRevertMode::Disabled,
        None,
        &mut session,
        |_output, _elapsed| -> Result<()> { Ok(()) },
        None,
    )?;

    assert_eq!(session.last_ci_error_pattern, None);
    assert_eq!(session.consecutive_same_error_count, 0);
    Ok(())
}
