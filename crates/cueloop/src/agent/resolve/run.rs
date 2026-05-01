//! Purpose: Resolve CLI agent override arguments into validated runtime
//! override data.
//!
//! Responsibilities:
//! - Parse and validate run-command override flags, including notifications and
//!   phase settings.
//! - Parse and validate scan/task override flags.
//! - Apply runner/model compatibility checks before emitting `AgentOverrides`.
//!
//! Scope:
//! - Top-level CLI override resolution only; phase-only helpers and RepoPrompt
//!   config fallback live in sibling modules.
//!
//! Usage:
//! - Called by CLI command handlers before task/run orchestration begins.
//!
//! Invariants/Assumptions:
//! - Override resolution validates runner/model compatibility.
//! - The `--quick` flag overrides `--phases` to force single-pass execution.
//! - Notification and RepoPrompt flags preserve their existing precedence.

use anyhow::Result;

use crate::runner;

use super::super::args::{AgentArgs, RunAgentArgs};
use super::super::parse::{
    parse_git_publish_mode, parse_git_revert_mode, parse_runner, parse_runner_cli_patch,
};
use super::super::repoprompt::repoprompt_flags_from_mode;
use super::phase_overrides::resolve_phase_overrides;
use super::types::AgentOverrides;

/// Helper macro to resolve a boolean CLI flag with enable/disable variants.
///
/// Takes the enable flag expression and disable flag expression, returns
/// `Some(true)` if enabled, `Some(false)` if disabled, or `None` if neither.
macro_rules! resolve_bool_flag {
    ($enable:expr, $disable:expr) => {
        if $enable {
            Some(true)
        } else if $disable {
            Some(false)
        } else {
            None
        }
    };
}

/// Helper macro to resolve a simple optional boolean flag.
///
/// Returns `Some(true)` if the flag is set, `None` otherwise.
macro_rules! resolve_simple_flag {
    ($flag:expr) => {
        if $flag { Some(true) } else { None }
    };
}

/// Resolve agent overrides from CLI arguments for run commands.
///
/// This parses the CLI arguments and validates runner/model compatibility.
pub fn resolve_run_agent_overrides(args: &RunAgentArgs) -> Result<AgentOverrides> {
    let profile = args.profile.clone();

    let runner = match args.runner.as_deref() {
        Some(value) => Some(parse_runner(value)?),
        None => None,
    };

    let model = match args.model.as_deref() {
        Some(value) => Some(runner::parse_model(value)?),
        None => None,
    };

    let reasoning_effort = match args.effort.as_deref() {
        Some(value) => Some(runner::parse_reasoning_effort(value)?),
        None => None,
    };
    let runner_cli = parse_runner_cli_patch(&args.runner_cli)?;

    if let (Some(runner_kind), Some(model)) = (runner.as_ref(), model.as_ref()) {
        runner::validate_model_for_runner(runner_kind, model)?;
    }

    let repoprompt_override = args.repo_prompt.map(repoprompt_flags_from_mode);

    let git_revert_mode = match args.git_revert_mode.as_deref() {
        Some(value) => Some(parse_git_revert_mode(value)?),
        None => None,
    };

    let git_publish_mode = match args.git_publish_mode.as_deref() {
        Some(value) => Some(parse_git_publish_mode(value)?),
        None => None,
    };
    let include_draft = resolve_simple_flag!(args.include_draft);

    let phases = if args.quick { Some(1) } else { args.phases };

    let notify_on_complete = resolve_bool_flag!(args.notify, args.no_notify);
    let notify_on_fail = resolve_bool_flag!(args.notify_fail, args.no_notify_fail);
    let notify_sound = resolve_simple_flag!(args.notify_sound);
    let lfs_check = resolve_simple_flag!(args.lfs_check);
    let no_progress = resolve_simple_flag!(args.no_progress);

    let phase_overrides = resolve_phase_overrides(args)?;

    Ok(AgentOverrides {
        profile,
        runner,
        model,
        reasoning_effort,
        runner_cli,
        phases,
        repoprompt_plan_required: repoprompt_override.map(|flags| flags.plan_required),
        repoprompt_tool_injection: repoprompt_override.map(|flags| flags.tool_injection),
        git_revert_mode,
        git_publish_mode,
        include_draft,
        notify_on_complete,
        notify_on_fail,
        notify_on_loop_complete: None,
        notify_sound,
        lfs_check,
        no_progress,
        phase_overrides,
    })
}

/// Resolve agent overrides from CLI arguments for scan/task commands.
///
/// This is a simpler version that doesn't include phases.
pub fn resolve_agent_overrides(args: &AgentArgs) -> Result<AgentOverrides> {
    let runner = match args.runner.as_deref() {
        Some(value) => Some(parse_runner(value)?),
        None => None,
    };

    let model = match args.model.as_deref() {
        Some(value) => Some(runner::parse_model(value)?),
        None => None,
    };

    let reasoning_effort = match args.effort.as_deref() {
        Some(value) => Some(runner::parse_reasoning_effort(value)?),
        None => None,
    };

    if let (Some(runner_kind), Some(model)) = (runner.as_ref(), model.as_ref()) {
        runner::validate_model_for_runner(runner_kind, model)?;
    }

    let repoprompt_override = args.repo_prompt.map(repoprompt_flags_from_mode);
    let runner_cli = parse_runner_cli_patch(&args.runner_cli)?;

    Ok(AgentOverrides {
        profile: None,
        runner,
        model,
        reasoning_effort,
        runner_cli,
        phases: None,
        repoprompt_plan_required: repoprompt_override.map(|flags| flags.plan_required),
        repoprompt_tool_injection: repoprompt_override.map(|flags| flags.tool_injection),
        git_revert_mode: None,
        git_publish_mode: None,
        include_draft: None,
        notify_on_complete: None,
        notify_on_fail: None,
        notify_on_loop_complete: None,
        notify_sound: None,
        lfs_check: None,
        no_progress: None,
        phase_overrides: None,
    })
}
