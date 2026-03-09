//! Run loop orchestration.
//!
//! Responsibilities:
//! - Orchestrate the sequential run loop (`run_loop`).
//! - Handle session recovery and graceful stop signals.
//! - Track task completion statistics and send notifications.
//!
//! Not handled here:
//! - Individual task execution (see `run_one`).
//! - Parallel run loop (see `parallel`).
//! - Phase execution details (see `phases`).
//!
//! Invariants/assumptions:
//! - Queue lock contention errors are non-retriable to prevent infinite loops.
//! - Session timeout uses configured hours (defaults to 24 hours).

use crate::agent::AgentOverrides;
use crate::config;
use crate::constants::limits::MAX_CONSECUTIVE_FAILURES;
use crate::contracts::TaskStatus;
use crate::signal;
use crate::{queue, runutil, webhook};
use anyhow::Result;

use super::queue_lock::{clear_stale_queue_lock_for_resume, is_queue_lock_already_held_error};
use super::run_one::{RunOutcome, run_one};
use session::resolve_resume_state;
use wait::{WaitExit, WaitMode, wait_for_work};

mod session;
mod wait;

pub struct RunLoopOptions {
    /// 0 means "no limit"
    pub max_tasks: u32,
    pub agent_overrides: AgentOverrides,
    pub force: bool,
    /// Auto-resume without prompting (for --resume flag)
    pub auto_resume: bool,
    /// Starting completed count (for resumed sessions)
    pub starting_completed: u32,
    /// Skip interactive prompts (for CI/non-interactive runs)
    pub non_interactive: bool,
    /// Number of parallel workers to use when parallel mode is enabled.
    pub parallel_workers: Option<u8>,
    /// Wait when blocked by dependencies/schedule instead of exiting.
    pub wait_when_blocked: bool,
    /// Poll interval in milliseconds while waiting (default: 1000).
    pub wait_poll_ms: u64,
    /// Timeout in seconds for waiting (0 = no timeout).
    pub wait_timeout_seconds: u64,
    /// Notify when queue becomes unblocked.
    pub notify_when_unblocked: bool,
    /// Wait when queue is empty instead of exiting (continuous mode).
    pub wait_when_empty: bool,
    /// Poll interval in milliseconds while waiting on an empty queue (default: 30000).
    pub empty_poll_ms: u64,
}

pub fn run_loop(resolved: &config::Resolved, opts: RunLoopOptions) -> Result<()> {
    let parallel_workers = opts.parallel_workers.or(resolved.config.parallel.workers);
    if let Some(workers) = parallel_workers
        && workers >= 2
    {
        if opts.auto_resume {
            log::warn!("Parallel run ignores --resume; starting a fresh parallel loop.");
        }
        if opts.starting_completed != 0 {
            log::warn!("Parallel run ignores starting_completed; counters will start at zero.");
        }
        return super::parallel::run_loop_parallel(
            resolved,
            super::parallel::ParallelRunOptions {
                max_tasks: opts.max_tasks,
                workers,
                agent_overrides: opts.agent_overrides,
                force: opts.force,
            },
        );
    }

    let cache_dir = resolved.repo_root.join(".ralph/cache");
    let queue_file = queue::load_queue(&resolved.queue_path)?;
    let resume_state = resolve_resume_state(resolved, &opts)?;
    let resume_task_id = resume_state.resume_task_id;
    let completed_count = resume_state.completed_count;

    // Preemptively clear stale queue lock when resuming a session.
    // This handles the case where a previous ralph process crashed/killed
    // and left behind a stale lock file.
    if resume_task_id.is_some()
        && let Err(err) = clear_stale_queue_lock_for_resume(&resolved.repo_root)
    {
        log::warn!("Failed to clear stale queue lock for resume: {}", err);
        // Continue anyway - the lock acquisition in run_one will fail
        // with a more specific error if the lock is still held.
    }

    let include_draft = opts.agent_overrides.include_draft.unwrap_or(false);
    let initial_todo_count = queue_file
        .tasks
        .iter()
        .filter(|t| {
            t.status == TaskStatus::Todo || (include_draft && t.status == TaskStatus::Draft)
        })
        .count() as u32;

    if initial_todo_count == 0 && resume_task_id.is_none() {
        // Keep this phrase stable; some tests look for it.
        if include_draft {
            log::info!("No todo or draft tasks found.");
        } else {
            log::info!("No todo tasks found.");
        }
        if !opts.wait_when_empty {
            return Ok(());
        }
        // In continuous mode, continue into the loop to wait for work
    }

    let label = format!(
        "RunLoop (todo={initial_todo_count}, max_tasks={})",
        opts.max_tasks
    );

    // Track loop completion stats for notification
    let mut tasks_attempted: usize = 0;
    let mut tasks_succeeded: usize = 0;
    let mut tasks_failed: usize = 0;

    // Track consecutive failures to prevent infinite loops
    let mut consecutive_failures: u32 = 0;

    // Use a mutable reference to allow modification inside the closure
    let mut completed = completed_count;

    // Clear any stale stop signal from previous runs to ensure clean state
    signal::clear_stop_signal_at_loop_start(&cache_dir);

    // Emit loop_started webhook before entering the run loop
    let loop_start_time = std::time::Instant::now();
    let loop_started_at = crate::timeutil::now_utc_rfc3339_or_fallback();
    let loop_webhook_ctx = crate::webhook::WebhookContext {
        repo_root: Some(resolved.repo_root.display().to_string()),
        branch: crate::git::current_branch(&resolved.repo_root).ok(),
        commit: crate::session::get_git_head_commit(&resolved.repo_root),
        ..Default::default()
    };
    webhook::notify_loop_started(
        &resolved.config.agent.webhook,
        &loop_started_at,
        loop_webhook_ctx.clone(),
    );

    let result = super::logging::with_scope(&label, || {
        loop {
            if opts.max_tasks != 0 && completed >= opts.max_tasks {
                log::info!("RunLoop: end (reached max task limit: {completed})");
                return Ok(());
            }

            // Check for graceful stop signal before starting next task
            if signal::stop_signal_exists(&cache_dir) {
                log::info!("Stop signal detected; no new tasks will be started.");
                if let Err(e) = signal::clear_stop_signal(&cache_dir) {
                    log::warn!("Failed to clear stop signal: {}", e);
                }
                return Ok(());
            }

            match run_one(
                resolved,
                &opts.agent_overrides,
                opts.force,
                resume_task_id.as_deref(),
            ) {
                Ok(RunOutcome::NoCandidates) => {
                    if opts.wait_when_empty {
                        // Enter wait loop for new tasks
                        match wait_for_work(
                            resolved,
                            include_draft,
                            WaitMode::EmptyAllowed,
                            opts.wait_poll_ms,
                            opts.empty_poll_ms,
                            0, // No timeout for empty wait
                            opts.notify_when_unblocked,
                            &loop_webhook_ctx,
                        )? {
                            WaitExit::RunnableAvailable { .. } => {
                                log::info!("RunLoop: new runnable tasks detected; continuing");
                                continue;
                            }
                            WaitExit::NoCandidates => {
                                // Should not happen in EmptyAllowed mode, but handle gracefully
                                continue;
                            }
                            WaitExit::TimedOut => {
                                log::info!("RunLoop: end (wait timeout reached)");
                                return Ok(());
                            }
                            WaitExit::StopRequested => {
                                log::info!("RunLoop: end (stop signal received)");
                                return Ok(());
                            }
                        }
                    } else {
                        log::info!("RunLoop: end (no more todo tasks remaining)");
                        return Ok(());
                    }
                }
                Ok(RunOutcome::Blocked { summary }) => {
                    if opts.wait_when_blocked || opts.wait_when_empty {
                        // Determine wait mode based on flags
                        let mode = if opts.wait_when_empty {
                            WaitMode::EmptyAllowed
                        } else {
                            WaitMode::BlockedOnly
                        };
                        // Wait for a runnable task to become available
                        match wait_for_work(
                            resolved,
                            include_draft,
                            mode,
                            opts.wait_poll_ms,
                            opts.empty_poll_ms,
                            opts.wait_timeout_seconds,
                            opts.notify_when_unblocked,
                            &loop_webhook_ctx,
                        )? {
                            WaitExit::RunnableAvailable {
                                summary: new_summary,
                            } => {
                                log::info!(
                                    "RunLoop: unblocked (ready={}, deps={}, sched={}); continuing",
                                    new_summary.runnable_candidates,
                                    new_summary.blocked_by_dependencies,
                                    new_summary.blocked_by_schedule
                                );
                                continue;
                            }
                            WaitExit::NoCandidates => {
                                log::info!("RunLoop: end (queue became empty while waiting)");
                                return Ok(());
                            }
                            WaitExit::TimedOut => {
                                log::info!("RunLoop: end (wait timeout reached)");
                                return Ok(());
                            }
                            WaitExit::StopRequested => {
                                log::info!("RunLoop: end (stop signal received)");
                                return Ok(());
                            }
                        }
                    } else {
                        // Not in wait mode - exit with helpful message
                        log::info!(
                            "RunLoop: end (blocked: ready={} deps={} sched={}). \
                             Use --wait-when-blocked to wait for dependencies/schedules.",
                            summary.runnable_candidates,
                            summary.blocked_by_dependencies,
                            summary.blocked_by_schedule
                        );
                        return Ok(());
                    }
                }
                Ok(RunOutcome::Ran { task_id: _ }) => {
                    completed += 1;
                    tasks_attempted += 1;
                    tasks_succeeded += 1;
                    consecutive_failures = 0; // Reset on success

                    // Persist session progress for accurate resume limits
                    if let Err(e) = crate::session::increment_session_progress(&cache_dir) {
                        log::warn!("Failed to persist session progress: {}", e);
                    }

                    if initial_todo_count == 0 {
                        log::info!("RunLoop: task-complete (completed={completed})");
                    } else {
                        log::info!("RunLoop: task-complete ({completed}/{initial_todo_count})");
                    }
                }
                Err(err) => {
                    if let Some(reason) = runutil::abort_reason(&err) {
                        match reason {
                            runutil::RunAbortReason::Interrupted => {
                                log::info!("RunLoop: aborting after interrupt");
                            }
                            runutil::RunAbortReason::UserRevert => {
                                log::info!("RunLoop: aborting after user-requested revert");
                            }
                        }
                        return Err(err);
                    }

                    // Queue lock errors are non-retriable - return immediately
                    // to prevent the 50-failure abort loop on deterministic lock errors.
                    if is_queue_lock_already_held_error(&err) {
                        log::error!("RunLoop: aborting due to queue lock contention");
                        return Err(err);
                    }

                    // Dirty repository errors are non-retriable - return immediately
                    // to prevent the 50-failure abort loop on deterministic dirty repo errors.
                    // A dirty repo cannot self-resolve; user intervention is required.
                    if runutil::is_dirty_repo_error(&err) {
                        log::error!("RunLoop: aborting due to dirty repository");
                        return Err(err);
                    }

                    // Queue validation errors are non-retriable - return immediately
                    // to prevent the 50-failure abort loop on deterministic validation errors.
                    // Queue validation errors (invalid relationships, duplicate IDs, etc.)
                    // cannot self-resolve; user intervention is required.
                    if runutil::is_queue_validation_error(&err) {
                        log::error!("RunLoop: aborting due to queue validation error");
                        return Err(err);
                    }

                    completed += 1;
                    tasks_attempted += 1;
                    tasks_failed += 1;
                    consecutive_failures += 1;

                    // Persist session progress for accurate resume limits
                    if let Err(e) = crate::session::increment_session_progress(&cache_dir) {
                        log::warn!("Failed to persist session progress: {}", e);
                    }

                    log::error!("RunLoop: task failed: {:#}", err);

                    // Safety check: prevent infinite loops from rapid consecutive failures
                    if consecutive_failures >= MAX_CONSECUTIVE_FAILURES {
                        log::error!(
                            "RunLoop: aborting after {MAX_CONSECUTIVE_FAILURES} consecutive failures"
                        );
                        return Err(anyhow::anyhow!(
                            "Run loop aborted after {} consecutive task failures. \
                             This usually indicates a systemic issue (e.g., repo dirty, \
                             runner misconfiguration, or interrupt flag stuck). \
                             Check logs above for root cause.",
                            MAX_CONSECUTIVE_FAILURES
                        ));
                    }
                    // Continue with next task even if one failed
                }
            }
        }
    });

    // Send loop completion notification
    if tasks_attempted > 0 {
        let notify_config = crate::notification::build_notification_config(
            &resolved.config.agent.notification,
            &crate::notification::NotificationOverrides {
                notify_on_complete: opts.agent_overrides.notify_on_complete,
                notify_on_fail: opts.agent_overrides.notify_on_fail,
                notify_sound: opts.agent_overrides.notify_sound,
            },
        );
        crate::notification::notify_loop_complete(
            tasks_attempted,
            tasks_succeeded,
            tasks_failed,
            &notify_config,
        );
    }

    // Emit loop_stopped webhook after loop completes
    let loop_stopped_at = crate::timeutil::now_utc_rfc3339_or_fallback();
    let loop_duration_ms = loop_start_time.elapsed().as_millis() as u64;
    let loop_note = match &result {
        Ok(()) => Some(format!(
            "Completed: {}/{} succeeded",
            tasks_succeeded, tasks_attempted
        )),
        Err(e) => Some(format!("Error: {}", e)),
    };
    webhook::notify_loop_stopped(
        &resolved.config.agent.webhook,
        &loop_stopped_at,
        webhook::WebhookContext {
            duration_ms: Some(loop_duration_ms),
            ..loop_webhook_ctx
        },
        loop_note.as_deref(),
    );

    // Clear session on successful completion
    if result.is_ok()
        && let Err(e) = crate::session::clear_session(&cache_dir)
    {
        log::warn!("Failed to clear session on loop completion: {}", e);
    }

    result
}
