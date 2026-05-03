//! Core run-one orchestration.
//!
//! Purpose:
//! - Core run-one orchestration.
//!
//! Responsibilities:
//! - Implement `run_one_impl`: coordinate context preparation, task selection,
//!   execution setup, explicit resume-state narration, phase execution, and completion handling.
//! - Define shared types used by run-one submodules.
//!
//! Not handled here:
//! - Individual phase implementation details (see `phases` module).
//! - Parallel run loop orchestration (see `parallel` module).
//! - Context preparation (see `context` submodule).
//! - Task setup (see `execution_setup` submodule).
//! - Phase execution (see `phase_execution` submodule).
//! - Webhook notifications (see `webhooks` submodule).
//! - Completion handling (see `completion` submodule).
//! - Task selection (see `selection` submodule).
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Callers pass the correct `QueueLockMode` for their context.
//! - Resume decisions are emitted before task selection so operators can see why CueLoop chose a path.

use crate::agent::AgentOverrides;
use crate::commands::run::run_one::{ResumeExecutionProgress, RunOneResumeOptions};
use crate::commands::run::{
    RunOutcome, context::task_context_for_prompt, emit_blocked_state_changed, emit_resume_decision,
    phases::PostRunMode,
};
use crate::config;
use crate::contracts::Task;
use crate::prompts;
use crate::runner::OutputHandler;
use crate::runutil::RevertPromptHandler;
use crate::session::{ResumeBehavior, ResumeDecisionMode, ResumeStatus};
use crate::{commands::run::RunEvent, commands::run::RunEventHandler};
use anyhow::{Result, bail};

// Import from sibling modules
use super::{
    completion::handle_run_completion, context::prepare_run_one_context,
    execution_setup::setup_task_execution, phase_execution::execute_iteration_phases,
    selection::select_task_for_run,
};

/// Context prepared before task execution.
pub(crate) struct RunOneContext {
    /// Owns the queue lock for direct run-one paths that acquire it here.
    pub _queue_lock: Option<crate::lock::DirLock>,
    pub queue_file: crate::contracts::QueueFile,
    pub done: crate::contracts::QueueFile,
    pub git_revert_mode: crate::contracts::GitRevertMode,
    pub git_publish_mode: crate::contracts::GitPublishMode,
    pub push_policy: crate::commands::run::supervision::PushPolicy,
    pub post_run_mode: PostRunMode,
    /// Coordinator-selected base branch for parallel worker integration.
    pub parallel_target_branch: Option<String>,
    pub policy: crate::promptflow::PromptPolicy,
}

/// Setup for task execution after selection.
pub(crate) struct TaskExecutionSetup<'a> {
    pub phases: u8,
    pub iteration_settings: crate::commands::run::iteration::IterationSettings,
    pub phase_matrix: crate::runner::PhaseSettingsMatrix,
    pub preexisting_dirty_allowed: bool,
    pub plugin_registry: crate::plugins::registry::PluginRegistry,
    pub bins: crate::runner::RunnerBinaries<'a>,
    pub execution_timings:
        Option<std::cell::RefCell<crate::commands::run::execution_timings::RunExecutionTimings>>,
}

/// Result of task selection.
pub(crate) enum SelectTaskResult {
    /// A task was selected for execution.
    Selected {
        /// The selected task (boxed to avoid large enum variant).
        task: Box<Task>,
    },
    /// No candidates available (no todo/draft tasks in queue).
    NoCandidates,
    /// Tasks exist but all are blocked by dependencies or schedule.
    Blocked {
        /// Summary of why tasks are blocked.
        summary: Box<crate::queue::operations::QueueRunnabilitySummary>,
        /// Operator-facing blocking state.
        state: Box<crate::contracts::BlockingState>,
    },
}

fn nonempty_file_exists(path: &std::path::Path) -> bool {
    std::fs::metadata(path).is_ok_and(|metadata| metadata.is_file() && metadata.len() > 0)
}

fn infer_resume_phase_from_caches(repo_root: &std::path::Path, task_id: &str) -> u8 {
    if nonempty_file_exists(&crate::promptflow::phase2_final_response_cache_path(
        repo_root, task_id,
    )) {
        return 3;
    }
    if nonempty_file_exists(&crate::promptflow::plan_cache_path(repo_root, task_id)) {
        return 2;
    }
    1
}

fn load_resume_progress(
    resolved: &config::Resolved,
    task_id: &str,
) -> Result<Option<ResumeExecutionProgress>> {
    let cache_dir = crate::config::project_runtime_dir(&resolved.repo_root).join("cache");
    let Some(session) = crate::session::load_session(&cache_dir)? else {
        return Ok(None);
    };

    if session.task_id.trim() != task_id {
        return Ok(None);
    }

    let inferred_phase = infer_resume_phase_from_caches(&resolved.repo_root, task_id);
    let current_phase = session.current_phase.max(inferred_phase);
    if current_phase != session.current_phase {
        log::info!(
            "Task {}: inferred resume phase {} from cached phase artifacts (session recorded phase {})",
            task_id,
            current_phase,
            session.current_phase
        );
    }

    Ok(Some(ResumeExecutionProgress {
        iterations_completed: session.iterations_completed,
        current_phase,
    }))
}

/// Build the base prompt for task execution.
fn build_base_prompt(resolved: &config::Resolved, task: &Task, task_id: &str) -> Result<String> {
    let template = prompts::load_worker_prompt(&resolved.repo_root)?;
    let project_type = resolved
        .config
        .project_type
        .unwrap_or(crate::contracts::ProjectType::Code);
    let mut base_prompt =
        prompts::render_worker_prompt(&template, task_id, project_type, &resolved.config)?;
    base_prompt =
        prompts::wrap_with_instruction_files(&resolved.repo_root, &base_prompt, &resolved.config)?;

    let task_context = task_context_for_prompt(task)?;
    base_prompt = format!("{task_context}\n\n---\n\n{base_prompt}");

    Ok(base_prompt)
}

/// Main run-one implementation.
#[allow(clippy::too_many_arguments)]
pub fn run_one_impl(
    resolved: &config::Resolved,
    agent_overrides: &AgentOverrides,
    force: bool,
    lock_mode: super::QueueLockMode,
    target_task_id: Option<&str>,
    resume_options: RunOneResumeOptions,
    output_handler: Option<OutputHandler>,
    run_event_handler: Option<RunEventHandler>,
    revert_prompt: Option<RevertPromptHandler>,
    parallel_target_branch: Option<&str>,
) -> Result<RunOutcome> {
    // 1. Prepare context (lock, queue, config)
    let ctx = prepare_run_one_context(
        resolved,
        agent_overrides,
        force,
        lock_mode,
        parallel_target_branch,
    )?;

    let (resolved_resume_task_id, allow_resume_dirty_baseline) = if resume_options.detect_session {
        let resolution = crate::session::resolve_run_session_decision(
            &crate::config::project_runtime_dir(&resolved.repo_root).join("cache"),
            &ctx.queue_file,
            crate::session::RunSessionDecisionOptions {
                timeout_hours: resolved.config.agent.session_timeout_hours,
                behavior: if resume_options.auto_resume {
                    ResumeBehavior::AutoResume
                } else {
                    ResumeBehavior::Prompt
                },
                non_interactive: resume_options.non_interactive,
                explicit_task_id: target_task_id,
                announce_missing_session: resume_options.auto_resume,
                mode: ResumeDecisionMode::Execute,
            },
        )?;

        if let Some(decision) = resolution.decision.as_ref() {
            emit_resume_decision(decision, run_event_handler.as_ref());
            if let Some(blocking_state) = decision.blocking_state() {
                emit_blocked_state_changed(&blocking_state, run_event_handler.as_ref());
            }
            if matches!(decision.status, ResumeStatus::RefusingToResume) {
                bail!("{}", decision.message);
            }
        }

        let allow_resume_dirty_baseline = resolution.resume_task_id.is_some();
        (resolution.resume_task_id, allow_resume_dirty_baseline)
    } else {
        (
            resume_options.resume_task_id.clone(),
            resume_options.allow_resume_dirty_baseline,
        )
    };

    // 2. Select task
    let include_draft = agent_overrides.include_draft.unwrap_or(false);
    let selection = select_task_for_run(
        &ctx.queue_file,
        Some(&ctx.done),
        target_task_id,
        resolved_resume_task_id.as_deref(),
        &resolved.repo_root,
        include_draft,
        run_event_handler.as_ref(),
    )?;

    let task = match selection {
        SelectTaskResult::NoCandidates => return Ok(RunOutcome::NoCandidates),
        SelectTaskResult::Blocked { summary, state } => {
            emit_blocked_state_changed(&state, run_event_handler.as_ref());
            return Ok(RunOutcome::Blocked { summary, state });
        }
        SelectTaskResult::Selected { task } => *task,
    };
    let task_id = task.id.trim().to_string();
    let is_selected_resume_task = resolved_resume_task_id.as_deref() == Some(task_id.as_str());
    let allow_selected_resume_dirty_baseline =
        allow_resume_dirty_baseline && is_selected_resume_task;
    let resume_progress = if is_selected_resume_task {
        load_resume_progress(resolved, &task_id)?
    } else {
        None
    };
    if let Some(handler) = &run_event_handler {
        handler(RunEvent::TaskSelected {
            task_id: task_id.clone(),
            title: task.title.clone(),
        });
    }

    // 3. Setup execution
    let setup = setup_task_execution(
        resolved,
        agent_overrides,
        &task,
        ctx.post_run_mode,
        force,
        allow_selected_resume_dirty_baseline,
        resume_progress.is_some(),
    )?;

    // 4. Build prompt
    let base_prompt = build_base_prompt(resolved, &task, &task_id)?;

    // 5. Execute phases
    let output_stream = if output_handler.is_some() {
        crate::runner::OutputStream::HandlerOnly
    } else {
        crate::runner::OutputStream::Terminal
    };

    let exec_result = execute_iteration_phases(
        resolved,
        ctx._queue_lock.as_ref(),
        agent_overrides,
        &task,
        &task_id,
        &setup,
        resume_progress,
        &base_prompt,
        &ctx.policy,
        output_handler.clone(),
        run_event_handler,
        output_stream,
        resolved
            .config
            .project_type
            .unwrap_or(crate::contracts::ProjectType::Code),
        ctx.git_revert_mode,
        ctx.git_publish_mode,
        ctx.push_policy,
        revert_prompt,
        ctx.post_run_mode,
        ctx.parallel_target_branch.as_deref(),
        &setup.plugin_registry,
    );

    // 6. Handle completion
    handle_run_completion(
        exec_result,
        resolved,
        &task,
        &task_id,
        setup.phases,
        ctx.post_run_mode,
        setup.execution_timings,
        agent_overrides,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_resume_phase_prefers_phase2_final_response_cache() -> Result<()> {
        let temp = tempfile::TempDir::new()?;
        crate::promptflow::write_plan_cache(temp.path(), "RQ-0001", "plan")?;
        crate::promptflow::write_phase2_final_response_cache(temp.path(), "RQ-0001", "done")?;

        assert_eq!(infer_resume_phase_from_caches(temp.path(), "RQ-0001"), 3);
        Ok(())
    }

    #[test]
    fn infer_resume_phase_uses_plan_cache_when_phase2_cache_missing() -> Result<()> {
        let temp = tempfile::TempDir::new()?;
        crate::promptflow::write_plan_cache(temp.path(), "RQ-0001", "plan")?;

        assert_eq!(infer_resume_phase_from_caches(temp.path(), "RQ-0001"), 2);
        Ok(())
    }

    #[test]
    fn infer_resume_phase_defaults_to_phase1_without_caches() -> Result<()> {
        let temp = tempfile::TempDir::new()?;

        assert_eq!(infer_resume_phase_from_caches(temp.path(), "RQ-0001"), 1);
        Ok(())
    }
}
