//! Worker-event handling helpers for parallel orchestration.
//!
//! Purpose:
//! - Worker-event handling helpers for parallel orchestration.
//!
//! Responsibilities:
//! - Summarize blocked workers at loop start.
//! - Apply worker exit events to persisted parallel state.
//!
//! Non-scope:
//! - Worker spawning or selection.
//! - Loop termination decisions.
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/Assumptions:
//! - Keep behavior aligned with Ralph's canonical CLI, machine-contract, and queue semantics.

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::Path;

use crate::commands::run::parallel::cleanup_guard::ParallelCleanupGuard;
use crate::commands::run::parallel::state::{self, WorkerLifecycle, WorkerRecord};
use crate::commands::run::parallel::sync;
use crate::commands::run::parallel::worker::FinishedWorker;
use crate::commands::run::parallel::workspace_cleanup::remove_workspace_best_effort;
use crate::config;
use crate::contracts::QueueFile;
use crate::timeutil;

use super::stats::ParallelRunStats;

fn summarize_block_reason(reason: &str) -> String {
    let first_line = reason.lines().next().unwrap_or(reason).trim();
    const MAX_REASON_LEN: usize = 180;
    if first_line.len() <= MAX_REASON_LEN {
        return first_line.to_string();
    }
    let mut truncated = first_line
        .chars()
        .take(MAX_REASON_LEN - 3)
        .collect::<String>();
    truncated.push_str("...");
    truncated
}

pub(super) fn announce_blocked_tasks_at_loop_start(
    queue_file: &QueueFile,
    state_file: &state::ParallelStateFile,
) {
    let queued_ids: HashSet<&str> = queue_file
        .tasks
        .iter()
        .map(|task| task.id.trim())
        .filter(|task_id| !task_id.is_empty())
        .collect();

    let blocked_workers: Vec<&WorkerRecord> = state_file
        .workers
        .iter()
        .filter(|worker| worker.lifecycle == WorkerLifecycle::BlockedPush)
        .filter(|worker| queued_ids.contains(worker.task_id.trim()))
        .collect();

    if blocked_workers.is_empty() {
        return;
    }

    log::warn!(
        "Parallel loop start: {} queued task(s) are in blocked_push and will be skipped until retried.",
        blocked_workers.len()
    );
    for worker in blocked_workers {
        let reason = worker
            .last_error
            .as_deref()
            .map(summarize_block_reason)
            .unwrap_or_else(|| "No failure reason recorded".to_string());
        log::warn!(
            "Blocked task {} (attempts: {}) reason: {}",
            worker.task_id,
            worker.push_attempts,
            reason
        );
    }
    log::warn!("Use `cueloop run parallel retry --task <TASK_ID>` to retry a blocked task.");
}

pub(super) struct FinishedWorkerHandlingContext<'a> {
    pub(super) state_path: &'a Path,
    pub(super) workspace_root: &'a Path,
    pub(super) resolved: &'a config::Resolved,
    pub(super) target_branch: &'a str,
    pub(super) queue_lock: &'a crate::lock::DirLock,
}

pub(super) fn handle_finished_workers(
    finished: Vec<FinishedWorker>,
    guard: &mut ParallelCleanupGuard,
    context: FinishedWorkerHandlingContext<'_>,
    stats: &mut ParallelRunStats,
) -> Result<()> {
    for finished_worker in finished {
        let FinishedWorker {
            task_id,
            task_title: _task_title,
            workspace,
            status,
        } = finished_worker;

        if status.success() {
            log::info!("Worker {} completed successfully", task_id);
            refresh_coordinator_branch(&context.resolved.repo_root, context.target_branch)?;
            sync::bookkeeping::reconcile_successful_workers(
                context.resolved,
                context.queue_lock,
                &[sync::bookkeeping::SuccessfulWorkerBookkeeping {
                    task_id: task_id.clone(),
                    workspace_path: workspace.path.clone(),
                }],
                "parallel worker bookkeeping reconciliation",
            )?;

            stats.record_success();

            if let Some(worker) = guard.state_file_mut().get_worker_mut(&task_id) {
                worker.mark_completed(timeutil::now_utc_rfc3339_or_fallback());
            }
        } else {
            stats.record_failure();

            let blocked_marker =
                match super::super::integration::read_blocked_push_marker(&workspace.path) {
                    Ok(marker) => marker,
                    Err(err) => {
                        log::warn!(
                            "Failed reading blocked marker for {} ({}): {}",
                            task_id,
                            workspace.path.display(),
                            err
                        );
                        None
                    }
                };

            if let Some(marker) = blocked_marker {
                if let Some(worker) = guard.state_file_mut().get_worker_mut(&task_id) {
                    worker.push_attempts = marker.attempt;
                    worker.mark_blocked(
                        timeutil::now_utc_rfc3339_or_fallback(),
                        marker.reason.clone(),
                    );
                }

                log::warn!(
                    "Worker {} blocked after {}/{} integration attempts: {}",
                    task_id,
                    marker.attempt,
                    marker.max_attempts,
                    marker.reason
                );
                log::warn!(
                    "Retaining blocked workspace for retry: {}",
                    workspace.path.display()
                );
            } else {
                if let Some(worker) = guard.state_file_mut().get_worker_mut(&task_id) {
                    worker.mark_failed(
                        timeutil::now_utc_rfc3339_or_fallback(),
                        format!("Worker exited with status: {:?}", status.code()),
                    );
                }

                log::warn!(
                    "Worker {} failed with exit status: {:?}",
                    task_id,
                    status.code()
                );

                remove_workspace_best_effort(context.workspace_root, &workspace, "worker failure");
            }
        }

        state::save_state(context.state_path, guard.state_file())?;
        guard.remove_worker(&task_id);
    }

    Ok(())
}

fn refresh_coordinator_branch(repo_root: &Path, target_branch: &str) -> Result<()> {
    crate::git::branch::fast_forward_branch_to_origin(repo_root, target_branch).with_context(|| {
        format!(
            "worker completed, but local branch refresh to origin/{} failed; resolve the coordinator checkout before continuing so tracked queue/done bookkeeping cannot remain stale",
            target_branch
        )
    })
}

#[cfg(test)]
#[path = "events_tests.rs"]
mod events_tests;
