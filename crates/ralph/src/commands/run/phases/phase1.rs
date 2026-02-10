//! Phase 1 (planning) execution and task-refresh helpers.
//!
//! Responsibilities:
//! - Execute Phase 1 planning runner pass and enforce plan-only output constraints.
//! - Refresh task metadata once before Phase 1 begins using task updater logic.
//!
//! Not handled here:
//! - Phase 2/3 execution behavior.
//! - Queue/task selection and task status transitions.
//!
//! Invariants/assumptions:
//! - Phase 1 task refresh is skipped in parallel-worker mode to avoid queue writes.
//! - Phase 1 task refresh never converts non-abort updater failures into hard failures.

use super::shared::execute_runner_pass;
use super::{PhaseInvocation, PhaseType, phase_session_id_for_runner};
use crate::commands::run::{logging, supervision};
use crate::commands::task as task_cmd;
use crate::contracts::{RunnerCliOptionsPatch, Task};
use crate::git::GitError;
use crate::{config, git, promptflow, prompts, queue, runutil};
use anyhow::{Context, Result, bail};

/// Refresh task fields from current repository state before Phase 1 planning.
///
/// Returns the refreshed task when queue writes are allowed and task reload succeeds.
/// Returns `Ok(None)` when refresh is intentionally skipped (parallel worker mode).
pub fn refresh_task_before_phase1(
    resolved: &config::Resolved,
    task_id: &str,
    settings: &crate::runner::AgentSettings,
    policy: &promptflow::PromptPolicy,
    post_run_mode: super::PostRunMode,
    force: bool,
) -> Result<Option<Task>> {
    if matches!(post_run_mode, super::PostRunMode::ParallelWorker) {
        log::info!(
            "Task {task_id}: parallel worker mode skips Phase 1 task refresh to avoid queue writes"
        );
        return Ok(None);
    }

    let runner_cli_overrides = RunnerCliOptionsPatch {
        output_format: Some(settings.runner_cli.output_format),
        verbosity: Some(settings.runner_cli.verbosity),
        approval_mode: Some(settings.runner_cli.approval_mode),
        sandbox: Some(settings.runner_cli.sandbox),
        plan_mode: Some(settings.runner_cli.plan_mode),
        unsupported_option_policy: Some(settings.runner_cli.unsupported_option_policy),
    };
    let update_settings = task_cmd::TaskUpdateSettings {
        fields: "scope,evidence,plan,notes,tags,depends_on".to_string(),
        runner_override: Some(settings.runner.clone()),
        model_override: Some(settings.model.clone()),
        reasoning_effort_override: settings.reasoning_effort,
        runner_cli_overrides,
        force,
        repoprompt_tool_injection: policy.repoprompt_tool_injection,
        dry_run: false,
    };

    log::info!("Task {task_id}: refreshing task metadata before Phase 1 planning");
    match task_cmd::update_task_without_lock(resolved, task_id, &update_settings) {
        Ok(()) => {
            log::info!("Task {task_id}: Phase 1 task refresh completed");
        }
        Err(err) => {
            if runutil::abort_reason(&err).is_some() {
                return Err(err);
            }
            log::warn!(
                "Task {task_id}: Phase 1 task refresh failed (continuing with current task data): {:#}",
                err
            );
            log::debug!("Phase 1 task refresh error details: {:?}", err);
        }
    }

    let done = queue::load_queue_or_default(&resolved.done_path)?;
    let done_ref = if done.tasks.is_empty() && !resolved.done_path.exists() {
        None
    } else {
        Some(&done)
    };
    let max_depth = resolved.config.queue.max_dependency_depth.unwrap_or(10);
    let (updated_queue_file, validation_warnings) = queue::load_queue_with_repair_and_validate(
        &resolved.queue_path,
        done_ref,
        &resolved.id_prefix,
        resolved.id_width,
        max_depth,
    )
    .context("validate repaired queue after Phase 1 task refresh")?;
    queue::log_warnings(&validation_warnings);

    let task_id_trimmed = task_id.trim();
    let task = updated_queue_file
        .tasks
        .into_iter()
        .find(|candidate| candidate.id.trim() == task_id_trimmed)
        .context("reload selected task after Phase 1 task refresh")?;

    Ok(Some(task))
}

pub fn execute_phase1_planning(ctx: &PhaseInvocation<'_>, total_phases: u8) -> Result<String> {
    let label = logging::phase_label(1, total_phases, "Planning", ctx.task_id);

    logging::with_scope(&label, || {
        let baseline_paths = if ctx.allow_dirty_repo {
            git::status_paths(&ctx.resolved.repo_root)?
        } else {
            Vec::new()
        };
        let baseline_snapshots = if ctx.allow_dirty_repo {
            git::snapshot_paths(&ctx.resolved.repo_root, &baseline_paths)?
        } else {
            Vec::new()
        };
        let p1_template = prompts::load_worker_phase1_prompt(&ctx.resolved.repo_root)?;
        let p1_prompt = promptflow::build_phase1_prompt(
            &p1_template,
            ctx.base_prompt,
            ctx.iteration_context,
            ctx.task_id,
            total_phases,
            ctx.policy,
            &ctx.resolved.config,
        )?;
        let phase_session_id =
            phase_session_id_for_runner(ctx.settings.runner.clone(), ctx.task_id, 1);
        let output = execute_runner_pass(
            ctx.resolved,
            ctx.settings,
            ctx.bins,
            &p1_prompt,
            ctx.output_handler.clone(),
            ctx.output_stream,
            true,
            ctx.git_revert_mode,
            ctx.revert_prompt.clone(),
            "Planning",
            PhaseType::Planning,
            phase_session_id,
            ctx.execution_timings,
            ctx.task_id,
            ctx.plugins,
        )?;

        let mut continue_session = supervision::ContinueSession {
            runner: ctx.settings.runner.clone(),
            model: ctx.settings.model.clone(),
            reasoning_effort: ctx.settings.reasoning_effort,
            runner_cli: ctx.settings.runner_cli,
            phase_type: super::PhaseType::Planning,
            session_id: output.session_id.clone(),
            output_handler: ctx.output_handler.clone(),
            output_stream: ctx.output_stream,
            ci_failure_retry_count: 0,
            task_id: ctx.task_id.to_string(),
        };

        // ENFORCEMENT: Phase 1 must not implement.
        // It may only edit `.ralph/queue.json` / `.ralph/done.json` (status bookkeeping)
        // plus the plan cache file for the current task.
        let plan_cache_rel = format!(".ralph/cache/plans/{}.md", ctx.task_id);
        let plan_cache_dir = ".ralph/cache/plans/";
        let allowed_paths = [
            ".ralph/queue.json",
            ".ralph/done.json",
            plan_cache_rel.as_str(),
            plan_cache_dir,
        ];
        loop {
            let mut allowed: Vec<String> = allowed_paths
                .iter()
                .map(|value| value.to_string())
                .collect();
            allowed.extend(baseline_paths.iter().cloned());
            let allowed_refs: Vec<&str> = allowed.iter().map(String::as_str).collect();

            let status = git::require_clean_repo_ignoring_paths(
                &ctx.resolved.repo_root,
                false,
                &allowed_refs,
            );
            let snapshot_check = match status {
                Ok(()) => git::ensure_paths_unchanged(&ctx.resolved.repo_root, &baseline_snapshots)
                    .map_err(|err| GitError::Other(err.context("baseline dirty path changed"))),
                Err(err) => Err(err),
            };

            match snapshot_check {
                Ok(()) => break,
                Err(err) => {
                    let outcome = runutil::apply_git_revert_mode_with_context(
                        &ctx.resolved.repo_root,
                        ctx.git_revert_mode,
                        runutil::RevertPromptContext::new("Phase 1 plan-only violation", true),
                        ctx.revert_prompt.as_ref(),
                    )?;
                    match outcome {
                        runutil::RevertOutcome::Continue { message } => {
                            let (_output, elapsed) = supervision::resume_continue_session(
                                ctx.resolved,
                                &mut continue_session,
                                &message,
                                ctx.plugins,
                            )?;
                            // Record resume duration for Phase 1
                            if let Some(timings) = ctx.execution_timings {
                                timings.borrow_mut().record_runner_duration(
                                    PhaseType::Planning,
                                    &continue_session.runner,
                                    &continue_session.model,
                                    elapsed,
                                );
                            }
                            continue;
                        }
                        runutil::RevertOutcome::Proceed { reason } => {
                            log::warn!(
                                "Phase 1 plan-only violation override: proceeding without reverting ({reason})."
                            );
                            break;
                        }
                        _ => {
                            bail!(
                                "{} Error: {:#}",
                                runutil::format_revert_failure_message(
                                    "Phase 1 violated plan-only contract: it modified files outside allowed queue bookkeeping, including baseline dirty paths.",
                                    outcome,
                                ),
                                err
                            );
                        }
                    }
                }
            }
        }

        // Read plan from cache (Phase 1 writes it directly).
        let plan_text = promptflow::read_plan_cache(&ctx.resolved.repo_root, ctx.task_id)?;
        log::info!(
            "Plan cached for {} at {}",
            ctx.task_id,
            promptflow::plan_cache_path(&ctx.resolved.repo_root, ctx.task_id).display()
        );

        Ok(plan_text)
    })
}
