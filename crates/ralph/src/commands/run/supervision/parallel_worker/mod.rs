//! Parallel worker supervision facade.
//!
//! Purpose:
//! - Expose post-run supervision for parallel workers from a thin module surface.
//!
//! Responsibilities:
//! - Coordinate CI enforcement, bookkeeping restore, and optional git finalization.
//! - Delegate bookkeeping cleanup and CI marker persistence to focused helpers.
//!
//! Scope:
//! - Parallel-worker post-run handling only.
//! - Standard post-run supervision remains in `super::mod` and sibling helpers.
//!
//! Usage:
//! - Compiled for supervision regression tests that exercise worker bookkeeping contracts.
//!
//! Invariants/assumptions:
//! - Worker bookkeeping must be restored before any publish step.
//! - Queue/done lookups remain read-only in this module.

mod bookkeeping;
mod ci_marker;
#[cfg(test)]
mod tests;

use crate::contracts::{GitPublishMode, GitRevertMode};
use crate::git;
use crate::queue;
use crate::runutil;
use anyhow::{Context, Result};

use super::CiContinueContext;
use super::PushPolicy;
use super::enforce_post_run_ci_gate;
use super::git_ops::{finalize_git_state, warn_if_modified_lfs};
use bookkeeping::restore_parallel_worker_bookkeeping_and_check_clean;
use ci_marker::write_ci_failure_marker;

/// Post-run supervision for parallel workers.
///
/// Restores shared bookkeeping files and commits/pushes only the worker's
/// task changes without mutating workspace-local queue/done clones.
#[allow(clippy::too_many_arguments)]
pub(crate) fn post_run_supervise_parallel_worker(
    resolved: &crate::config::Resolved,
    task_id: &str,
    git_revert_mode: GitRevertMode,
    git_publish_mode: GitPublishMode,
    push_policy: PushPolicy,
    revert_prompt: Option<runutil::RevertPromptHandler>,
    ci_continue: Option<CiContinueContext<'_>>,
    lfs_check: bool,
    plugins: Option<&crate::plugins::registry::PluginRegistry>,
) -> Result<()> {
    let label = format!("PostRunSuperviseParallelWorker for {}", task_id.trim());
    super::logging::with_scope(&label, || {
        let status = git::status_porcelain(&resolved.repo_root)?;
        let is_dirty = !status.trim().is_empty();

        if is_dirty {
            if let Err(err) = warn_if_modified_lfs(&resolved.repo_root, lfs_check) {
                return Err(anyhow::anyhow!(
                    "LFS validation failed: {}. Use --lfs-check to enable strict validation or fix the LFS issues.",
                    err
                ));
            }
            enforce_post_run_ci_gate(
                resolved,
                git_revert_mode,
                revert_prompt.as_ref(),
                ci_continue,
                plugins,
                |err| {
                    write_ci_failure_marker(
                        &resolved.repo_root,
                        task_id,
                        &format!("CI gate failed: {:#}", err),
                    );
                },
            )?;
        }

        let status = restore_parallel_worker_bookkeeping_and_check_clean(resolved, task_id)?;

        if status.trim().is_empty() {
            return Ok(());
        }

        if git_publish_mode != GitPublishMode::Off {
            let task_title = task_title_from_queue_or_done(resolved, task_id)?.unwrap_or_default();
            finalize_git_state(
                resolved,
                task_id,
                &task_title,
                git_publish_mode,
                push_policy,
            )
            .context("Git finalization failed")?;
        } else {
            log::info!("Git publish mode is off; leaving repo dirty after worker run.");
        }

        Ok(())
    })
}

fn task_title_from_queue_or_done(
    resolved: &crate::config::Resolved,
    task_id: &str,
) -> Result<Option<String>> {
    let queue_file = queue::load_queue(&resolved.queue_path)?;
    if let Some(title) = find_task_title(&queue_file, task_id) {
        return Ok(Some(title));
    }
    let done_file = queue::load_queue_or_default(&resolved.done_path)?;
    Ok(find_task_title(&done_file, task_id))
}

fn find_task_title(queue_file: &crate::contracts::QueueFile, task_id: &str) -> Option<String> {
    queue_file
        .tasks
        .iter()
        .find(|task| task.id.trim() == task_id)
        .map(|task| task.title.clone())
}
