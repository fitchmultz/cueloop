//! Shared helpers for machine command handlers.
//!
//! Responsibilities:
//! - Build shared machine documents reused across handlers.
//! - Centralize queue-path and config-safety shaping for machine responses.
//! - Reuse small queue/done helper semantics across machine subcommands.
//! - Convert operator-facing resume decisions into machine contract payloads.
//!
//! Not handled here:
//! - Clap argument definitions.
//! - JSON stdout/stderr emission.
//! - Queue/task/run command routing.
//!
//! Invariants/assumptions:
//! - Machine config documents remain versioned through `crate::contracts` constants.
//! - Done-queue omission semantics match the existing machine/read-only behavior.
//! - Resume previews must be read-only and never mutate persisted session state.

use std::path::Path;

use crate::config;
use crate::contracts::{
    GitPublishMode, GitRevertMode, MACHINE_CONFIG_RESOLVE_VERSION, MachineConfigResolveDocument,
    MachineConfigSafetySummary, MachineQueuePaths, MachineResumeDecision, QueueFile,
};
use crate::session::{ResumeBehavior, ResumeDecisionMode, ResumeReason, ResumeScope, ResumeStatus};

pub(super) fn build_config_resolve_document(
    resolved: &config::Resolved,
    repo_trusted: bool,
    dirty_repo: bool,
    resume_preview: Option<MachineResumeDecision>,
) -> MachineConfigResolveDocument {
    MachineConfigResolveDocument {
        version: MACHINE_CONFIG_RESOLVE_VERSION,
        paths: queue_paths(resolved),
        safety: MachineConfigSafetySummary {
            repo_trusted,
            dirty_repo,
            git_publish_mode: resolved
                .config
                .agent
                .effective_git_publish_mode()
                .unwrap_or(GitPublishMode::Off),
            approval_mode: resolved.config.agent.effective_approval_mode(),
            ci_gate_enabled: resolved.config.agent.ci_gate_enabled(),
            git_revert_mode: resolved
                .config
                .agent
                .git_revert_mode
                .unwrap_or(GitRevertMode::Ask),
            parallel_configured: resolved.config.parallel.workers.is_some(),
            execution_interactivity: "noninteractive_streaming".to_string(),
            interactive_approval_supported: false,
        },
        config: resolved.config.clone(),
        resume_preview,
    }
}

pub(super) fn build_resume_preview(
    resolved: &config::Resolved,
    explicit_task_id: Option<&str>,
    auto_resume: bool,
    non_interactive: bool,
    announce_missing_session: bool,
) -> anyhow::Result<Option<MachineResumeDecision>> {
    let queue_file = crate::queue::load_queue(&resolved.queue_path)?;
    let resolution = crate::session::resolve_run_session_decision(
        &resolved.repo_root.join(".ralph/cache"),
        &queue_file,
        crate::session::RunSessionDecisionOptions {
            timeout_hours: resolved.config.agent.session_timeout_hours,
            behavior: if auto_resume {
                ResumeBehavior::AutoResume
            } else {
                ResumeBehavior::Prompt
            },
            non_interactive,
            explicit_task_id,
            announce_missing_session,
            mode: ResumeDecisionMode::Preview,
        },
    )?;

    Ok(resolution
        .decision
        .as_ref()
        .map(machine_resume_decision_from_runtime))
}

pub(super) fn machine_resume_decision_from_runtime(
    decision: &crate::session::ResumeDecision,
) -> MachineResumeDecision {
    MachineResumeDecision {
        status: machine_resume_status(decision.status).to_string(),
        scope: machine_resume_scope(decision.scope).to_string(),
        reason: machine_resume_reason(decision.reason).to_string(),
        task_id: decision.task_id.clone(),
        message: decision.message.clone(),
        detail: decision.detail.clone(),
    }
}

fn machine_resume_status(status: ResumeStatus) -> &'static str {
    match status {
        ResumeStatus::ResumingSameSession => "resuming_same_session",
        ResumeStatus::FallingBackToFreshInvocation => "falling_back_to_fresh_invocation",
        ResumeStatus::RefusingToResume => "refusing_to_resume",
    }
}

fn machine_resume_scope(scope: ResumeScope) -> &'static str {
    match scope {
        ResumeScope::RunSession => "run_session",
        ResumeScope::ContinueSession => "continue_session",
    }
}

fn machine_resume_reason(reason: ResumeReason) -> &'static str {
    match reason {
        ResumeReason::NoSession => "no_session",
        ResumeReason::SessionValid => "session_valid",
        ResumeReason::SessionTimedOutConfirmed => "session_timed_out_confirmed",
        ResumeReason::SessionStale => "session_stale",
        ResumeReason::SessionDeclined => "session_declined",
        ResumeReason::ResumeConfirmationRequired => "resume_confirmation_required",
        ResumeReason::SessionTimedOutRequiresConfirmation => {
            "session_timed_out_requires_confirmation"
        }
        ResumeReason::ExplicitTaskSelectionOverridesSession => {
            "explicit_task_selection_overrides_session"
        }
        ResumeReason::ResumeTargetMissing => "resume_target_missing",
        ResumeReason::ResumeTargetTerminal => "resume_target_terminal",
        ResumeReason::RunnerSessionInvalid => "runner_session_invalid",
        ResumeReason::MissingRunnerSessionId => "missing_runner_session_id",
    }
}

pub(super) fn done_queue_ref<'a>(done: &'a QueueFile, done_path: &Path) -> Option<&'a QueueFile> {
    if done.tasks.is_empty() && !done_path.exists() {
        None
    } else {
        Some(done)
    }
}

pub(super) fn queue_paths(resolved: &config::Resolved) -> MachineQueuePaths {
    MachineQueuePaths {
        repo_root: resolved.repo_root.display().to_string(),
        queue_path: resolved.queue_path.display().to_string(),
        done_path: resolved.done_path.display().to_string(),
        project_config_path: resolved
            .project_config_path
            .as_ref()
            .map(|path| path.display().to_string()),
        global_config_path: resolved
            .global_config_path
            .as_ref()
            .map(|path| path.display().to_string()),
    }
}

pub(super) fn queue_max_dependency_depth(resolved: &config::Resolved) -> u8 {
    resolved.config.queue.max_dependency_depth.unwrap_or(10)
}
