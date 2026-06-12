//! Machine task mutation operation.
//!
//! Purpose:
//! - Apply atomic machine task mutation requests.
//!
//! Responsibilities:
//! - Parse the versioned task mutation JSON request.
//! - Run mutation validation against active and done queue state.
//! - Persist non-dry-run mutations with queue locking and undo snapshots.
//!
//! Not handled here:
//! - Mutation document envelope construction.
//! - Lifecycle-specific status/archive behavior.
//! - Generic task insertion.
//!
//! Usage:
//! - Called by the machine task router for `machine task mutate`.
//!
//! Invariants/assumptions:
//! - Mutations are applied against a cloned queue first and only saved after validation.
//! - Non-dry-run mutation writes create an undo snapshot before saving.

use anyhow::{Context, Result};

use crate::cli::machine::args::MachineTaskMutateArgs;
use crate::cli::machine::common::{done_queue_ref, queue_max_dependency_depth};
use crate::cli::machine::io::read_json_input;
use crate::config;
use crate::contracts::MachineTaskMutationDocument;
use crate::queue;
use crate::timeutil;

use super::documents::build_task_mutation_document;

pub(super) fn handle_mutate(
    resolved: &config::Resolved,
    args: MachineTaskMutateArgs,
    force: bool,
) -> Result<MachineTaskMutationDocument> {
    let raw = read_json_input(args.input.as_deref())?;
    let request = serde_json::from_str::<queue::operations::TaskMutationRequest>(&raw)
        .context("parse machine task mutation request")?;

    let _queue_lock = queue::acquire_queue_lock(&resolved.repo_root, "machine task mutate", force)?;
    let queue_file = queue::load_queue(&resolved.queue_path)?;
    let done_file = queue::load_queue_or_default(&resolved.done_path)?;
    let done_ref = done_queue_ref(&done_file, &resolved.done_path);
    let now = timeutil::now_utc_rfc3339()?;
    let mut working = queue_file.clone();
    let report = queue::operations::apply_task_mutation_request(
        &mut working,
        done_ref,
        &request,
        &now,
        &resolved.id_prefix,
        resolved.id_width,
        queue_max_dependency_depth(resolved),
    )?;
    if !args.dry_run {
        crate::undo::create_undo_snapshot(
            resolved,
            &format!("machine task mutate [{} task(s)]", report.tasks.len()),
        )?;
        queue::save_queue(&resolved.queue_path, &working)?;
    }
    build_task_mutation_document(&report, args.dry_run)
}
