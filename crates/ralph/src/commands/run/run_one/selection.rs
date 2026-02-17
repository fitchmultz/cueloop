//! Task selection for run-one.
//!
//! Responsibilities:
//! - Select a task for execution from the queue.
//! - Handle target task ID and resume task ID.
//! - Build blocked summary when no runnable tasks are found.
//!
//! Not handled here:
//! - Context preparation (see context.rs).
//! - Task setup (see execution_setup.rs).
//!
//! Invariants/assumptions:
//! - Queue files are already loaded and validated before selection.
//! - Resume task validation checks if the task still exists in the queue.

use crate::contracts::{QueueFile, Task, TaskStatus};
use anyhow::Result;
use std::path::Path;

use super::orchestration::SelectTaskResult;
use crate::commands::run::{
    run_session::validate_resumed_task, selection::select_run_one_task_index,
};
use crate::queue::{RunnableSelectionOptions, operations::QueueRunnabilitySummary};

/// Select a task for execution.
pub(crate) fn select_task_for_run(
    queue_file: &QueueFile,
    done_ref: Option<&QueueFile>,
    target_task_id: Option<&str>,
    resume_task_id: Option<&str>,
    repo_root: &Path,
    include_draft: bool,
) -> Result<SelectTaskResult> {
    let effective_target = if target_task_id.is_some() {
        target_task_id
    } else if let Some(resume_id) = resume_task_id {
        match validate_resumed_task(queue_file, resume_id, repo_root) {
            Ok(()) => Some(resume_id),
            Err(e) => {
                log::info!("Session resume failed: {e}");
                None
            }
        }
    } else {
        None
    };

    let task_idx =
        match select_run_one_task_index(queue_file, done_ref, effective_target, include_draft)? {
            Some(idx) => idx,
            None => {
                let candidates: Vec<_> = queue_file
                    .tasks
                    .iter()
                    .filter(|t| {
                        t.status == TaskStatus::Todo
                            || (include_draft && t.status == TaskStatus::Draft)
                    })
                    .cloned()
                    .collect();

                if candidates.is_empty() {
                    if include_draft {
                        log::info!("No todo or draft tasks found.");
                    } else {
                        log::info!("No todo tasks found.");
                    }
                    return Ok(SelectTaskResult::NoCandidates);
                }

                let summary =
                    build_blocked_summary(queue_file, done_ref, &candidates, include_draft);
                return Ok(SelectTaskResult::Blocked { summary });
            }
        };

    let task = queue_file.tasks[task_idx].clone();
    Ok(SelectTaskResult::Selected {
        task: Box::new(task),
    })
}

/// Build a summary for blocked tasks when no runnable tasks are found.
fn build_blocked_summary(
    queue_file: &QueueFile,
    done_ref: Option<&QueueFile>,
    candidates: &[Task],
    include_draft: bool,
) -> QueueRunnabilitySummary {
    let options = RunnableSelectionOptions::new(include_draft, true);
    match crate::queue::operations::queue_runnability_report(queue_file, done_ref, options) {
        Ok(report) => {
            if report.summary.blocked_by_schedule > 0 && report.summary.blocked_by_dependencies > 0
            {
                log::info!(
                    "All runnable tasks are blocked by unmet dependencies and future schedule ({} deps, {} scheduled).",
                    report.summary.blocked_by_dependencies,
                    report.summary.blocked_by_schedule
                );
            } else if report.summary.blocked_by_schedule > 0 {
                log::info!(
                    "All runnable tasks are blocked by future schedule ({} scheduled).",
                    report.summary.blocked_by_schedule
                );
            } else if report.summary.blocked_by_dependencies > 0 {
                log::info!(
                    "All runnable tasks are blocked by unmet dependencies ({} blocked).",
                    report.summary.blocked_by_dependencies
                );
            } else if report.summary.blocked_by_status_or_flags > 0 {
                log::info!(
                    "All tasks are blocked by status or flags ({} blocked).",
                    report.summary.blocked_by_status_or_flags
                );
            } else {
                log::info!("All runnable tasks are blocked.");
            }
            log::info!("Run 'ralph queue explain' for details.");
            report.summary.clone()
        }
        Err(e) => {
            log::info!(
                "No runnable tasks found (failed to analyze blockers: {}).",
                e
            );
            log::info!("Run 'ralph queue explain' for details.");
            QueueRunnabilitySummary {
                total_active: queue_file.tasks.len(),
                candidates_total: candidates.len(),
                runnable_candidates: 0,
                blocked_by_dependencies: candidates.len(),
                blocked_by_schedule: 0,
                blocked_by_status_or_flags: 0,
            }
        }
    }
}
