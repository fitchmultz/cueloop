//! Queue archive subcommand.

use anyhow::Result;

use crate::config::Resolved;
use crate::queue;

pub(crate) fn handle(resolved: &Resolved, force: bool) -> Result<()> {
    let _queue_lock = queue::acquire_queue_lock(&resolved.repo_root, "queue archive", force)?;
    let report = queue::archive_done_tasks(
        &resolved.queue_path,
        &resolved.done_path,
        &resolved.id_prefix,
        resolved.id_width,
    )?;
    if report.moved_ids.is_empty() {
        log::info!("No completed tasks to move.");
    } else {
        log::info!("Moved {} completed task(s).", report.moved_ids.len());
    }
    Ok(())
}
