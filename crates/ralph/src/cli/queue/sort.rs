//! Queue sort subcommand.
//!
//! Responsibilities:
//! - Reorder tasks in .ralph/queue.json by priority.
//!
//! Not handled here:
//! - Time-based or complex sorting (use `ralph queue list --sort-by` for that).
//! - Sorting by arbitrary fields (intentionally limited to prevent footguns).
//!
//! Invariants/assumptions:
//! - Only supports priority sorting to avoid dangerous queue reordering.
//! - Mutates queue.json; use with care in collaborative environments.

use anyhow::Result;
use clap::Args;

use crate::config::Resolved;
use crate::queue;

use super::{QueueSortBy, QueueSortOrder};

/// Arguments for `ralph queue sort`.
#[derive(Args)]
#[command(
    after_long_help = "Examples:\n  ralph queue sort\n  ralph queue sort --order descending\n  ralph queue sort --order ascending\n  ralph queue list --scheduled --sort-by scheduled_start --order ascending\n\nNote:\n  - `ralph queue sort` reorders .ralph/queue.json and intentionally supports priority only.\n  - For triage/time-based sorting without mutating files, use `ralph queue list --sort-by ...`."
)]
pub struct QueueSortArgs {
    /// Sort by field (supported: priority only; reorders queue file).
    #[arg(long, value_enum, default_value_t = QueueSortBy::Priority)]
    pub sort_by: QueueSortBy,

    /// Sort order (default: descending, highest priority first).
    #[arg(long, value_enum, default_value_t = QueueSortOrder::Descending)]
    pub order: QueueSortOrder,
}

pub(crate) fn handle(resolved: &Resolved, force: bool, args: QueueSortArgs) -> Result<()> {
    let _queue_lock = queue::acquire_queue_lock(&resolved.repo_root, "queue sort", force)?;

    // Create undo snapshot before mutation
    crate::undo::create_undo_snapshot(resolved, &format!("queue sort by {}", args.sort_by))?;

    let mut queue_file = queue::load_queue(&resolved.queue_path)?;

    match args.sort_by {
        QueueSortBy::Priority => {
            queue::sort_tasks_by_priority(&mut queue_file, args.order.is_descending());
        }
    }

    queue::save_queue(&resolved.queue_path, &queue_file)?;
    log::info!("Queue sorted by {} (order: {}).", args.sort_by, args.order);
    Ok(())
}
