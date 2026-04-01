//! Plugin command-building regression coverage.
//!
//! Responsibilities:
//! - Verify runner-specific run/resume argv construction for built-in plugins.
//! - Lock down approval, sandbox, session, and phase-aware command flags.
//!
//! Does not handle:
//! - Response parsing or executor metadata behavior.
//! - Subprocess integration beyond command assembly.
//!
//! Assumptions/invariants:
//! - Tests reuse parent helper contexts to model CLI defaults.
//! - Command assertions focus on stable argv semantics instead of arg ordering beyond required flags.

use super::*;

// =============================================================================
// Command Building Tests - Codex
// =============================================================================

#[test]
fn codex_build_run_command_basic() {
    let plugin = BuiltInRunnerPlugin::Codex;
    let ctx = create_run_context("test prompt", None);

    let (cmd, payload, _guards) = plugin.build_run_command(ctx).unwrap();

    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert!(
        args.contains(&"exec".to_string()),
        "Codex should use exec subcommand"
    );
    assert!(
        args.contains(&"--json".to_string()),
        "Codex should use --json flag"
    );
    assert!(
        args.contains(&"-".to_string()),
        "Codex should read from stdin"
    );
    assert!(payload.is_some(), "Codex should have stdin payload");
}

#[test]
fn codex_build_resume_command_includes_thread_id() {
    let plugin = BuiltInRunnerPlugin::Codex;
    let ctx = create_resume_context("thread-123", "continue please");

    let (cmd, _payload, _guards) = plugin.build_resume_command(ctx).unwrap();

    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert!(args.contains(&"exec".to_string()));
    assert!(args.contains(&"resume".to_string()));
    assert!(args.contains(&"thread-123".to_string()));
    assert!(args.contains(&"continue please".to_string()));
}

#[test]
fn codex_build_run_command_with_sandbox_disabled() {
    let plugin = BuiltInRunnerPlugin::Codex;
    let mut ctx = create_run_context("test prompt", None);
    ctx.runner_cli.sandbox = RunnerSandboxMode::Disabled;

    let (cmd, _payload, _guards) = plugin.build_run_command(ctx).unwrap();

    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert!(
        args.contains(&"--dangerously-bypass-approvals-and-sandbox".to_string()),
        "Codex should bypass sandbox when disabled"
    );
}

#[test]
fn codex_build_run_command_with_sandbox_enabled() {
    let plugin = BuiltInRunnerPlugin::Codex;
    let mut ctx = create_run_context("test prompt", None);
    ctx.runner_cli.sandbox = RunnerSandboxMode::Enabled;

    let (cmd, _payload, _guards) = plugin.build_run_command(ctx).unwrap();

    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert!(args.contains(&"--sandbox".to_string()));
    assert!(args.contains(&"workspace-write".to_string()));
}

// =============================================================================
// Command Building Tests - Kimi
// =============================================================================

#[test]
fn kimi_build_run_command_includes_session_id() {
    let plugin = BuiltInRunnerPlugin::Kimi;
    let ctx = create_run_context("test prompt", Some("sess-123"));

    let (cmd, _payload, _guards) = plugin.build_run_command(ctx).unwrap();

    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert!(args.contains(&"--session".to_string()));
    assert!(args.contains(&"sess-123".to_string()));
    assert!(args.contains(&"--print".to_string()));
    assert!(args.contains(&"--prompt".to_string()));
    assert!(args.contains(&"test prompt".to_string()));
}

#[test]
fn kimi_build_run_command_without_session() {
    let plugin = BuiltInRunnerPlugin::Kimi;
    let ctx = create_run_context("test prompt", None);

    let (cmd, _payload, _guards) = plugin.build_run_command(ctx).unwrap();

    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert!(
        !args.contains(&"--session".to_string()),
        "Kimi should not include --session when no session_id provided"
    );
}

#[test]
fn kimi_build_run_command_with_yolo_mode() {
    let plugin = BuiltInRunnerPlugin::Kimi;
    let mut ctx = create_run_context("test prompt", None);
    ctx.runner_cli.approval_mode = RunnerApprovalMode::Yolo;

    let (cmd, _payload, _guards) = plugin.build_run_command(ctx).unwrap();

    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert!(
        args.contains(&"--yolo".to_string()),
        "Kimi should use --yolo flag for yolo mode"
    );
}

// =============================================================================
// Command Building Tests - Claude
// =============================================================================

#[test]
fn claude_build_run_command_basic() {
    let plugin = BuiltInRunnerPlugin::Claude;
    let ctx = create_run_context("test prompt", None);

    let (cmd, payload, _guards) = plugin.build_run_command(ctx).unwrap();

    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert!(args.contains(&"--verbose".to_string()));
    assert!(args.contains(&"-p".to_string()));
    assert!(payload.is_some());
}

#[test]
fn claude_build_resume_command_includes_session() {
    let plugin = BuiltInRunnerPlugin::Claude;
    let ctx = create_resume_context("sess-abc", "continue");

    let (cmd, _payload, _guards) = plugin.build_resume_command(ctx).unwrap();

    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert!(args.contains(&"--resume".to_string()));
    assert!(args.contains(&"sess-abc".to_string()));
    assert!(args.contains(&"continue".to_string()));
}

// =============================================================================
// Command Building Tests - Gemini
// =============================================================================

#[test]
fn gemini_build_run_command_with_approval_mode() {
    let plugin = BuiltInRunnerPlugin::Gemini;
    let mut ctx = create_run_context("test prompt", None);
    ctx.runner_cli.approval_mode = RunnerApprovalMode::Yolo;

    let (cmd, _payload, _guards) = plugin.build_run_command(ctx).unwrap();

    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert!(args.contains(&"--approval-mode".to_string()));
    assert!(args.contains(&"yolo".to_string()));
}

#[test]
fn gemini_build_resume_command_includes_resume_flag() {
    let plugin = BuiltInRunnerPlugin::Gemini;
    let ctx = create_resume_context("sess-gem", "continue");

    let (cmd, _payload, _guards) = plugin.build_resume_command(ctx).unwrap();

    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert!(args.contains(&"--resume".to_string()));
    assert!(args.contains(&"sess-gem".to_string()));
}

// =============================================================================
// Command Building Tests - Cursor
// =============================================================================

#[test]
fn cursor_build_run_command_phase_aware_defaults() {
    let plugin = BuiltInRunnerPlugin::Cursor;
    let mut ctx = create_run_context("test prompt", None);
    ctx.phase_type = Some(PhaseType::Planning);

    let (cmd, _payload, _guards) = plugin.build_run_command(ctx).unwrap();

    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert!(args.contains(&"--sandbox".to_string()));
    // In planning phase, sandbox defaults to "enabled"
    assert!(args.contains(&"enabled".to_string()));
    assert!(args.contains(&"--plan".to_string()));
}

#[test]
fn cursor_build_run_command_implementation_phase() {
    let plugin = BuiltInRunnerPlugin::Cursor;
    let mut ctx = create_run_context("test prompt", None);
    ctx.phase_type = Some(PhaseType::Implementation);
    ctx.runner_cli.approval_mode = RunnerApprovalMode::Yolo;

    let (cmd, _payload, _guards) = plugin.build_run_command(ctx).unwrap();

    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert!(
        args.contains(&"--force".to_string()),
        "Yolo mode should add --force"
    );
    assert!(args.contains(&"--sandbox".to_string()));
    // In implementation phase, sandbox defaults to "disabled"
    assert!(args.contains(&"disabled".to_string()));
}

// =============================================================================
// Command Building Tests - Opencode
// =============================================================================

#[test]
fn opencode_build_run_command_uses_temp_file() {
    let plugin = BuiltInRunnerPlugin::Opencode;
    let ctx = create_run_context("test prompt content", None);

    let (cmd, _payload, guards) = plugin.build_run_command(ctx).unwrap();

    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert!(args.contains(&"run".to_string()));
    assert!(args.contains(&"--format".to_string()));
    assert!(args.contains(&"json".to_string()));
    // Opencode should have temp file guards
    assert!(!guards.is_empty(), "Opencode should have temp file guards");
}

#[test]
fn opencode_build_resume_command_includes_session_flag() {
    let plugin = BuiltInRunnerPlugin::Opencode;
    let ctx = create_resume_context("sess-open", "continue");

    let (cmd, _payload, _guards) = plugin.build_resume_command(ctx).unwrap();

    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert!(args.contains(&"-s".to_string()));
    assert!(args.contains(&"sess-open".to_string()));
}

// =============================================================================
// Command Building Tests - Pi
// =============================================================================

#[test]
fn pi_build_run_command_basic() {
    let plugin = BuiltInRunnerPlugin::Pi;
    let ctx = create_run_context("test prompt", None);

    let (cmd, _payload, _guards) = plugin.build_run_command(ctx).unwrap();

    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert!(args.contains(&"--mode".to_string()));
    assert!(args.contains(&"json".to_string()));
    assert!(args.contains(&"test prompt".to_string()));
}

#[test]
fn pi_build_run_command_with_yolo_mode() {
    let plugin = BuiltInRunnerPlugin::Pi;
    let mut ctx = create_run_context("test prompt", None);
    ctx.runner_cli.approval_mode = RunnerApprovalMode::Yolo;

    let (cmd, _payload, _guards) = plugin.build_run_command(ctx).unwrap();

    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert!(
        args.contains(&"--print".to_string()),
        "Pi should use --print for yolo mode"
    );
}

#[test]
fn pi_build_run_command_with_sandbox() {
    let plugin = BuiltInRunnerPlugin::Pi;
    let mut ctx = create_run_context("test prompt", None);
    ctx.runner_cli.sandbox = RunnerSandboxMode::Enabled;

    let (cmd, _payload, _guards) = plugin.build_run_command(ctx).unwrap();

    let args: Vec<String> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    assert!(args.contains(&"--sandbox".to_string()));
}

// =============================================================================
