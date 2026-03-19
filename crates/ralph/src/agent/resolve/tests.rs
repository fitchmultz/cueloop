//! Purpose: Preserve regression coverage for agent override parsing and
//! resolution after the facade split.
//!
//! Responsibilities:
//! - Verify run-command override parsing, quick-mode handling, and phase
//!   overrides.
//! - Verify scan/task override parsing and RepoPrompt mode mapping.
//! - Keep the prior inline `agent::resolve` behavior coverage intact.
//!
//! Scope:
//! - Agent override resolution only; parser-specific behavior stays covered in
//!   sibling modules.
//!
//! Usage:
//! - Runs as the `agent::resolve` unit test suite.
//!
//! Invariants/Assumptions:
//! - Assertions remain aligned with the former monolithic test block.
//! - Resolution behavior, validation, and error surfaces remain unchanged.

use crate::agent::args::RunnerCliArgs;
use crate::agent::repoprompt::RepoPromptMode;
use crate::contracts::{
    GitPublishMode, GitRevertMode, Model, ReasoningEffort, Runner, RunnerApprovalMode,
    RunnerPlanMode, RunnerSandboxMode,
};

use super::super::args::{AgentArgs, RunAgentArgs};
use super::{resolve_agent_overrides, resolve_run_agent_overrides};

#[test]
fn resolve_agent_overrides_parses_valid_args() {
    let args = AgentArgs {
        runner: Some("opencode".to_string()),
        model: Some("gpt-5.3".to_string()),
        effort: None,
        repo_prompt: None,
        runner_cli: RunnerCliArgs::default(),
    };

    let overrides = resolve_agent_overrides(&args).unwrap();
    assert_eq!(overrides.runner, Some(Runner::Opencode));
    assert_eq!(overrides.model, Some(Model::Gpt53));
    assert_eq!(overrides.reasoning_effort, None);
    assert_eq!(overrides.repoprompt_plan_required, None);
    assert_eq!(overrides.repoprompt_tool_injection, None);
    assert_eq!(overrides.git_revert_mode, None);
    assert_eq!(overrides.git_publish_mode, None);
    assert_eq!(overrides.include_draft, None);
}

#[test]
fn resolve_agent_overrides_sets_rp_flags() {
    let args = AgentArgs {
        runner: None,
        model: None,
        effort: None,
        repo_prompt: Some(RepoPromptMode::Plan),
        runner_cli: RunnerCliArgs::default(),
    };

    let overrides = resolve_agent_overrides(&args).unwrap();
    assert_eq!(overrides.repoprompt_plan_required, Some(true));
    assert_eq!(overrides.repoprompt_tool_injection, Some(true));
    assert_eq!(overrides.git_revert_mode, None);
    assert_eq!(overrides.git_publish_mode, None);
    assert_eq!(overrides.include_draft, None);
}

#[test]
fn resolve_agent_overrides_maps_repo_prompt_modes() {
    let tools_args = AgentArgs {
        runner: None,
        model: None,
        effort: None,
        repo_prompt: Some(RepoPromptMode::Tools),
        runner_cli: RunnerCliArgs::default(),
    };
    let tools_overrides = resolve_agent_overrides(&tools_args).unwrap();
    assert_eq!(tools_overrides.repoprompt_plan_required, Some(false));
    assert_eq!(tools_overrides.repoprompt_tool_injection, Some(true));

    let off_args = AgentArgs {
        runner: None,
        model: None,
        effort: None,
        repo_prompt: Some(RepoPromptMode::Off),
        runner_cli: RunnerCliArgs::default(),
    };
    let off_overrides = resolve_agent_overrides(&off_args).unwrap();
    assert_eq!(off_overrides.repoprompt_plan_required, Some(false));
    assert_eq!(off_overrides.repoprompt_tool_injection, Some(false));
}

#[test]
fn resolve_agent_overrides_parses_runner_cli_args() {
    let args = AgentArgs {
        runner: None,
        model: None,
        effort: None,
        repo_prompt: None,
        runner_cli: RunnerCliArgs {
            approval_mode: Some("auto-edits".to_string()),
            sandbox: Some("disabled".to_string()),
            ..RunnerCliArgs::default()
        },
    };

    let overrides = resolve_agent_overrides(&args).unwrap();
    assert_eq!(
        overrides.runner_cli.approval_mode,
        Some(RunnerApprovalMode::AutoEdits)
    );
    assert_eq!(
        overrides.runner_cli.sandbox,
        Some(RunnerSandboxMode::Disabled)
    );
}

#[test]
fn resolve_run_agent_overrides_includes_phases() {
    let args = RunAgentArgs {
        profile: None,
        runner: Some("codex".to_string()),
        model: Some("gpt-5.3-codex".to_string()),
        effort: Some("high".to_string()),
        runner_cli: RunnerCliArgs::default(),
        phases: Some(2),
        quick: false,
        repo_prompt: None,
        git_revert_mode: Some("enabled".to_string()),
        git_publish_mode: Some("off".to_string()),
        include_draft: true,
        notify: false,
        no_notify: false,
        notify_fail: false,
        no_notify_fail: false,
        notify_sound: false,
        lfs_check: false,
        no_progress: false,
        runner_phase1: None,
        model_phase1: None,
        effort_phase1: None,
        runner_phase2: None,
        model_phase2: None,
        effort_phase2: None,
        runner_phase3: None,
        model_phase3: None,
        effort_phase3: None,
    };

    let overrides = resolve_run_agent_overrides(&args).unwrap();
    assert_eq!(overrides.runner, Some(Runner::Codex));
    assert_eq!(overrides.model, Some(Model::Gpt53Codex));
    assert_eq!(overrides.reasoning_effort, Some(ReasoningEffort::High));
    assert_eq!(overrides.phases, Some(2));
    assert_eq!(overrides.git_revert_mode, Some(GitRevertMode::Enabled));
    assert_eq!(overrides.git_publish_mode, Some(GitPublishMode::Off));
    assert_eq!(overrides.include_draft, Some(true));
}

#[test]
fn resolve_run_agent_overrides_parses_runner_cli_args() {
    let args = RunAgentArgs {
        profile: None,
        runner: None,
        model: None,
        effort: None,
        runner_cli: RunnerCliArgs {
            approval_mode: Some("yolo".to_string()),
            plan_mode: Some("enabled".to_string()),
            ..RunnerCliArgs::default()
        },
        phases: None,
        quick: false,
        repo_prompt: None,
        git_revert_mode: None,
        git_publish_mode: None,
        include_draft: false,
        notify: false,
        no_notify: false,
        notify_fail: false,
        no_notify_fail: false,
        notify_sound: false,
        lfs_check: false,
        no_progress: false,
        runner_phase1: None,
        model_phase1: None,
        effort_phase1: None,
        runner_phase2: None,
        model_phase2: None,
        effort_phase2: None,
        runner_phase3: None,
        model_phase3: None,
        effort_phase3: None,
    };

    let overrides = resolve_run_agent_overrides(&args).unwrap();
    assert_eq!(
        overrides.runner_cli.approval_mode,
        Some(RunnerApprovalMode::Yolo)
    );
    assert_eq!(
        overrides.runner_cli.plan_mode,
        Some(RunnerPlanMode::Enabled)
    );
}

#[test]
fn resolve_run_agent_overrides_quick_flag_sets_phases_to_one() {
    let args = RunAgentArgs {
        profile: None,
        runner: None,
        model: None,
        effort: None,
        runner_cli: RunnerCliArgs::default(),
        phases: None,
        quick: true,
        repo_prompt: None,
        git_revert_mode: None,
        git_publish_mode: None,
        include_draft: false,
        notify: false,
        no_notify: false,
        notify_fail: false,
        no_notify_fail: false,
        notify_sound: false,
        lfs_check: false,
        no_progress: false,
        runner_phase1: None,
        model_phase1: None,
        effort_phase1: None,
        runner_phase2: None,
        model_phase2: None,
        effort_phase2: None,
        runner_phase3: None,
        model_phase3: None,
        effort_phase3: None,
    };

    let overrides = resolve_run_agent_overrides(&args).unwrap();
    assert_eq!(overrides.phases, Some(1));
}

#[test]
fn resolve_run_agent_overrides_phases_override_takes_precedence_when_quick_false() {
    let args = RunAgentArgs {
        profile: None,
        runner: None,
        model: None,
        effort: None,
        runner_cli: RunnerCliArgs::default(),
        phases: Some(3),
        quick: false,
        repo_prompt: None,
        git_revert_mode: None,
        git_publish_mode: None,
        include_draft: false,
        notify: false,
        no_notify: false,
        notify_fail: false,
        no_notify_fail: false,
        notify_sound: false,
        lfs_check: false,
        no_progress: false,
        runner_phase1: None,
        model_phase1: None,
        effort_phase1: None,
        runner_phase2: None,
        model_phase2: None,
        effort_phase2: None,
        runner_phase3: None,
        model_phase3: None,
        effort_phase3: None,
    };

    let overrides = resolve_run_agent_overrides(&args).unwrap();
    assert_eq!(overrides.phases, Some(3));
}

#[test]
fn resolve_run_agent_overrides_phase_flags_parsed_correctly() {
    let args = RunAgentArgs {
        profile: None,
        runner: Some("claude".to_string()),
        model: Some("sonnet".to_string()),
        effort: None,
        runner_cli: RunnerCliArgs::default(),
        phases: Some(3),
        quick: false,
        repo_prompt: None,
        git_revert_mode: None,
        git_publish_mode: None,
        include_draft: false,
        notify: false,
        no_notify: false,
        notify_fail: false,
        no_notify_fail: false,
        notify_sound: false,
        lfs_check: false,
        no_progress: false,
        runner_phase1: Some("codex".to_string()),
        model_phase1: Some("gpt-5.3-codex".to_string()),
        effort_phase1: Some("high".to_string()),
        runner_phase2: Some("claude".to_string()),
        model_phase2: Some("opus".to_string()),
        effort_phase2: None,
        runner_phase3: Some("codex".to_string()),
        model_phase3: Some("gpt-5.3-codex".to_string()),
        effort_phase3: Some("medium".to_string()),
    };

    let overrides = resolve_run_agent_overrides(&args).unwrap();

    assert_eq!(overrides.runner, Some(Runner::Claude));
    assert_eq!(overrides.model, Some(Model::Custom("sonnet".to_string())));

    let phase_overrides = overrides
        .phase_overrides
        .expect("phase_overrides should be set");

    let phase1 = phase_overrides.phase1.expect("phase1 should be set");
    assert_eq!(phase1.runner, Some(Runner::Codex));
    assert_eq!(phase1.model, Some(Model::Gpt53Codex));
    assert_eq!(phase1.reasoning_effort, Some(ReasoningEffort::High));

    let phase2 = phase_overrides.phase2.expect("phase2 should be set");
    assert_eq!(phase2.runner, Some(Runner::Claude));
    assert_eq!(phase2.model, Some(Model::Custom("opus".to_string())));
    assert_eq!(phase2.reasoning_effort, None);

    let phase3 = phase_overrides.phase3.expect("phase3 should be set");
    assert_eq!(phase3.runner, Some(Runner::Codex));
    assert_eq!(phase3.model, Some(Model::Gpt53Codex));
    assert_eq!(phase3.reasoning_effort, Some(ReasoningEffort::Medium));
}

#[test]
fn resolve_run_agent_overrides_phase_flags_partial() {
    let args = RunAgentArgs {
        profile: None,
        runner: None,
        model: None,
        effort: None,
        runner_cli: RunnerCliArgs::default(),
        phases: None,
        quick: false,
        repo_prompt: None,
        git_revert_mode: None,
        git_publish_mode: None,
        include_draft: false,
        notify: false,
        no_notify: false,
        notify_fail: false,
        no_notify_fail: false,
        notify_sound: false,
        lfs_check: false,
        no_progress: false,
        runner_phase1: Some("codex".to_string()),
        model_phase1: None,
        effort_phase1: None,
        runner_phase2: None,
        model_phase2: None,
        effort_phase2: None,
        runner_phase3: None,
        model_phase3: None,
        effort_phase3: None,
    };

    let overrides = resolve_run_agent_overrides(&args).unwrap();

    let phase_overrides = overrides
        .phase_overrides
        .expect("phase_overrides should be set");

    let phase1 = phase_overrides.phase1.expect("phase1 should be set");
    assert_eq!(phase1.runner, Some(Runner::Codex));
    assert_eq!(phase1.model, None);
    assert_eq!(phase1.reasoning_effort, None);

    assert!(phase_overrides.phase2.is_none());
    assert!(phase_overrides.phase3.is_none());
}

#[test]
fn resolve_run_agent_overrides_empty_phase_flags_returns_none() {
    let args = RunAgentArgs {
        profile: None,
        runner: None,
        model: None,
        effort: None,
        runner_cli: RunnerCliArgs::default(),
        phases: None,
        quick: false,
        repo_prompt: None,
        git_revert_mode: None,
        git_publish_mode: None,
        include_draft: false,
        notify: false,
        no_notify: false,
        notify_fail: false,
        no_notify_fail: false,
        notify_sound: false,
        lfs_check: false,
        no_progress: false,
        runner_phase1: None,
        model_phase1: None,
        effort_phase1: None,
        runner_phase2: None,
        model_phase2: None,
        effort_phase2: None,
        runner_phase3: None,
        model_phase3: None,
        effort_phase3: None,
    };

    let overrides = resolve_run_agent_overrides(&args).unwrap();
    assert!(overrides.phase_overrides.is_none());
}

#[test]
fn resolve_run_agent_overrides_invalid_runner_phase_includes_phase_in_error() {
    let args = RunAgentArgs {
        profile: None,
        runner: None,
        model: None,
        effort: None,
        runner_cli: RunnerCliArgs::default(),
        phases: None,
        quick: false,
        repo_prompt: None,
        git_revert_mode: None,
        git_publish_mode: None,
        include_draft: false,
        notify: false,
        no_notify: false,
        notify_fail: false,
        no_notify_fail: false,
        notify_sound: false,
        lfs_check: false,
        no_progress: false,
        runner_phase1: Some("invalid_runner".to_string()),
        model_phase1: None,
        effort_phase1: None,
        runner_phase2: None,
        model_phase2: None,
        effort_phase2: None,
        runner_phase3: None,
        model_phase3: None,
        effort_phase3: None,
    };

    let result = resolve_run_agent_overrides(&args);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Invalid runner"));
}
