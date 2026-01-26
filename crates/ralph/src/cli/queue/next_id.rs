//! Queue next-id subcommand.

use anyhow::Result;

use crate::cli::load_and_validate_queues;
use crate::config::Resolved;
use crate::queue;

pub(crate) fn handle(resolved: &Resolved) -> Result<()> {
    let (queue_file, done_file) = load_and_validate_queues(resolved, true)?;
    let done_ref = done_file
        .as_ref()
        .filter(|d| !d.tasks.is_empty() || resolved.done_path.exists());
    let next = queue::next_id_across(
        &queue_file,
        done_ref,
        &resolved.id_prefix,
        resolved.id_width,
    )?;
    println!("{next}");
    Ok(())
}
