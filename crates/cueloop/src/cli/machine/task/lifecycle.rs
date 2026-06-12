//! Machine task show and lifecycle operations.
//!
//! Purpose:
//! - Own machine task lookup and lifecycle status transitions.
//!
//! Responsibilities:
//! - Show a task from the active queue or done archive.
//! - Preview or apply lifecycle status changes.
//! - Preserve lifecycle note/evidence redaction and archive behavior.
//!
//! Not handled here:
//! - Generic task mutation requests.
//! - Task build/create/insert operations.
//! - Decomposition planning or write replay.
//!
//! Usage:
//! - Called by the machine task router for show/start/done/reject/status.
//!
//! Invariants/assumptions:
//! - Terminal statuses move tasks to the done archive when not dry-run.
//! - Non-terminal statuses keep tasks in the active queue.
//! - Notes and evidence are redacted and trimmed before being returned or written.

use anyhow::Result;

use crate::config;
use crate::contracts::{
    MACHINE_TASK_LIFECYCLE_VERSION, MACHINE_TASK_SHOW_VERSION, MachineContinuationAction,
    MachineContinuationSummary, MachineTaskLifecycleDocument, MachineTaskLocation,
    MachineTaskShowDocument, Task, TaskStatus,
};
use crate::queue;
use crate::queue::operations::{
    ActiveTaskMutationValidation, LifecycleNoteMode, QueueTaskLocation, redacted_non_empty_items,
};
use crate::timeutil;

pub(super) fn show_task(
    resolved: &config::Resolved,
    task_id: &str,
) -> Result<MachineTaskShowDocument> {
    let located = queue::operations::load_task_with_location(
        &resolved.queue_path,
        &resolved.done_path,
        task_id,
    )?;
    Ok(MachineTaskShowDocument {
        version: MACHINE_TASK_SHOW_VERSION,
        task_id: task_id.to_string(),
        location: machine_task_location(located.location),
        task: located.task,
    })
}

pub(super) fn update_task_lifecycle(
    resolved: &config::Resolved,
    task_id: &str,
    status: TaskStatus,
    notes: &[String],
    evidence: &[String],
    dry_run: bool,
    force: bool,
) -> Result<MachineTaskLifecycleDocument> {
    let terminal = matches!(status, TaskStatus::Done | TaskStatus::Rejected);
    if dry_run {
        let mut task = queue::operations::load_active_task(&resolved.queue_path, task_id)?;
        let now = timeutil::now_utc_rfc3339()?;
        queue::operations::apply_lifecycle_preview(&mut task, status, &now, notes, evidence);
        return Ok(build_task_lifecycle_document(
            task_id,
            status,
            notes,
            evidence,
            Some(task),
            terminal,
            true,
        ));
    }

    if terminal {
        let task = queue::with_locked_queue_mutation(
            resolved,
            "machine task lifecycle",
            format!("machine task {} {}", status, task_id),
            force,
            || {
                queue::operations::complete_active_task_to_archive(
                    &resolved.queue_path,
                    &resolved.done_path,
                    task_id,
                    status,
                    notes,
                    evidence,
                    &resolved.id_prefix,
                    resolved.id_width,
                    resolved.queue_max_dependency_depth(),
                )
            },
        )?;
        return Ok(build_task_lifecycle_document(
            task_id,
            status,
            notes,
            evidence,
            Some(task),
            true,
            false,
        ));
    }

    let updated_task = queue::with_locked_queue_mutation(
        resolved,
        "machine task lifecycle",
        format!("machine task {} {}", status, task_id),
        force,
        || {
            queue::operations::mutate_active_task_on_disk_with_validation(
                &resolved.queue_path,
                &resolved.done_path,
                task_id,
                &resolved.id_prefix,
                resolved.id_width,
                resolved.queue_max_dependency_depth(),
                ActiveTaskMutationValidation::SaveOnly,
                |task, now| {
                    queue::operations::apply_lifecycle_update(
                        task,
                        status,
                        now,
                        notes,
                        evidence,
                        LifecycleNoteMode::Joined,
                    )
                },
            )
        },
    )?;

    Ok(build_task_lifecycle_document(
        task_id,
        status,
        notes,
        evidence,
        Some(updated_task),
        false,
        false,
    ))
}

fn machine_task_location(location: QueueTaskLocation) -> MachineTaskLocation {
    match location {
        QueueTaskLocation::Active => MachineTaskLocation::Active,
        QueueTaskLocation::Done => MachineTaskLocation::Done,
    }
}

fn build_task_lifecycle_document(
    task_id: &str,
    status: TaskStatus,
    notes: &[String],
    evidence: &[String],
    task: Option<Task>,
    archived: bool,
    dry_run: bool,
) -> MachineTaskLifecycleDocument {
    let verb = if dry_run { "would be" } else { "has been" };
    MachineTaskLifecycleDocument {
        version: MACHINE_TASK_LIFECYCLE_VERSION,
        dry_run,
        task_id: task_id.to_string(),
        status: status.as_str().to_string(),
        task,
        notes: redacted_non_empty_items(notes),
        evidence: redacted_non_empty_items(evidence),
        archived,
        continuation: MachineContinuationSummary {
            headline: format!("Task {task_id} {verb} marked {status}."),
            detail: if archived {
                "Terminal task lifecycle updates move the task into the done archive.".to_string()
            } else {
                "Non-terminal task lifecycle updates keep the task in the active queue.".to_string()
            },
            blocking: None,
            next_steps: vec![MachineContinuationAction {
                title: "Validate queue".to_string(),
                command: "cueloop machine queue validate".to_string(),
                detail: "Confirm queue and archive state after the lifecycle update.".to_string(),
            }],
        },
    }
}
