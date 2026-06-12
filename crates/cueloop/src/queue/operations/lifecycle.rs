//! Shared task lifecycle and lookup helpers.
//!
//! Purpose:
//! - Centralize active/done task lookup and single-task lifecycle mutations shared by CLI surfaces.
//!
//! Responsibilities:
//! - Locate tasks across the active queue and done archive with a stable location value.
//! - Apply in-memory lifecycle status, note, and evidence updates consistently.
//! - Provide disk-backed helpers that validate, save, and reload lifecycle mutations.
//!
//! Non-scope:
//! - Queue lock acquisition or undo snapshot creation; callers own mutation boundaries.
//! - Batch result formatting and command-specific webhook output.
//! - Runner orchestration or phase supervision.
//!
//! Invariants/assumptions:
//! - Callers pass normalized UTC RFC3339 timestamps to lifecycle mutation helpers.
//! - Disk-backed active mutations target active queue tasks only.
//! - Task IDs follow the canonical queue-operation contract: trim surrounding whitespace and remain case-sensitive.

use std::path::Path;

use anyhow::{Result, anyhow, bail};
use serde::Serialize;

use crate::contracts::{QueueFile, Task, TaskStatus};
use crate::queue::{load_queue, load_queue_or_default, save_queue, validation};

/// Queue file that contains a located task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueTaskLocation {
    Active,
    Done,
}

/// Owned task lookup result across active and done queue files.
#[derive(Debug, Clone)]
pub struct LocatedTask {
    pub location: QueueTaskLocation,
    pub task: Task,
}

/// How lifecycle notes should be appended to a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleNoteMode {
    /// Append each non-empty redacted input string as its own note item.
    Items,
    /// Join all non-empty redacted input strings with newlines into one note item.
    Joined,
}

/// Whether a disk-backed active-task mutation should validate the full active/done queue set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTaskMutationValidation {
    /// Preserve legacy command surfaces that saved active queue mutations without whole-queue validation.
    SaveOnly,
    /// Validate active queue plus done archive before saving the active queue.
    ValidateQueueSet,
}

/// Locate a task in already-loaded active and optional done queues.
pub fn find_task_with_location(
    active: &QueueFile,
    done: Option<&QueueFile>,
    task_id: &str,
) -> Option<LocatedTask> {
    let needle = task_id.trim();
    if needle.is_empty() {
        return None;
    }

    if let Some(task) = super::query::find_task(active, needle) {
        return Some(LocatedTask {
            location: QueueTaskLocation::Active,
            task: task.clone(),
        });
    }

    done.and_then(|done_file| super::query::find_task(done_file, needle))
        .map(|task| LocatedTask {
            location: QueueTaskLocation::Done,
            task: task.clone(),
        })
}

/// Load active and done queues, then locate a task across both files.
pub fn load_task_with_location(
    queue_path: &Path,
    done_path: &Path,
    task_id: &str,
) -> Result<LocatedTask> {
    let active = load_queue(queue_path)?;
    let done = load_queue_or_default(done_path)?;
    find_task_with_location(&active, Some(&done), task_id)
        .ok_or_else(|| anyhow!("Task {task_id} was not found in the active queue or done archive."))
}

/// Load one task from the active queue only.
pub fn load_active_task(queue_path: &Path, task_id: &str) -> Result<Task> {
    let needle = task_id.trim();
    if needle.is_empty() {
        bail!("Task ID cannot be empty when loading an active task.");
    }
    let active = load_queue(queue_path)?;
    super::query::find_task(&active, needle)
        .cloned()
        .ok_or_else(|| anyhow!("Task {needle} was not found in the active queue."))
}

/// Append redacted, trimmed, non-empty items to a string vector.
pub fn append_redacted_items(target: &mut Vec<String>, items: &[String]) {
    for item in redacted_non_empty_items(items) {
        target.push(item);
    }
}

/// Return redacted, trimmed, non-empty copies of input strings.
pub fn redacted_non_empty_items(items: &[String]) -> Vec<String> {
    items
        .iter()
        .map(|item| crate::redaction::redact_text(item))
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect()
}

/// Apply a lifecycle status update plus notes/evidence to an in-memory task.
pub fn apply_lifecycle_update(
    task: &mut Task,
    status: TaskStatus,
    now_rfc3339: &str,
    notes: &[String],
    evidence: &[String],
    note_mode: LifecycleNoteMode,
) -> Result<()> {
    if matches!(status, TaskStatus::Done | TaskStatus::Rejected) {
        bail!(
            "terminal lifecycle updates must use complete_task so the task moves to the done archive"
        );
    }

    super::status::apply_status_policy(task, status, now_rfc3339, None)?;
    append_lifecycle_notes(task, notes, note_mode);
    append_redacted_items(&mut task.evidence, evidence);
    Ok(())
}

/// Apply the preview shape used by machine task lifecycle documents.
pub fn apply_lifecycle_preview(
    task: &mut Task,
    status: TaskStatus,
    now_rfc3339: &str,
    notes: &[String],
    evidence: &[String],
) {
    task.status = status;
    task.updated_at = Some(now_rfc3339.to_string());
    if status == TaskStatus::Doing && task.started_at.is_none() {
        task.started_at = Some(now_rfc3339.to_string());
    }
    if matches!(status, TaskStatus::Done | TaskStatus::Rejected) {
        task.completed_at = Some(now_rfc3339.to_string());
    }
    append_lifecycle_notes(task, notes, LifecycleNoteMode::Items);
    append_redacted_items(&mut task.evidence, evidence);
}

fn append_lifecycle_notes(task: &mut Task, notes: &[String], mode: LifecycleNoteMode) {
    match mode {
        LifecycleNoteMode::Items => append_redacted_items(&mut task.notes, notes),
        LifecycleNoteMode::Joined => {
            let joined = redacted_non_empty_items(notes).join("\n");
            if !joined.is_empty() {
                task.notes.push(joined);
            }
        }
    }
}

/// Complete a single active task, move it to the done archive, reload it, and return it.
#[allow(clippy::too_many_arguments)]
pub fn complete_active_task_to_archive(
    queue_path: &Path,
    done_path: &Path,
    task_id: &str,
    status: TaskStatus,
    notes: &[String],
    evidence: &[String],
    id_prefix: &str,
    id_width: usize,
    max_dependency_depth: u8,
) -> Result<Task> {
    let now = crate::timeutil::now_utc_rfc3339()?;
    super::status::complete_task(
        queue_path,
        done_path,
        task_id,
        status,
        &now,
        notes,
        evidence,
        id_prefix,
        id_width,
        max_dependency_depth,
        None,
    )?;
    Ok(load_task_with_location(queue_path, done_path, task_id)?.task)
}

/// Mutate one active task on disk, validate against the done archive, save, and return the task.
#[allow(clippy::too_many_arguments)]
pub fn mutate_active_task_on_disk(
    queue_path: &Path,
    done_path: &Path,
    task_id: &str,
    id_prefix: &str,
    id_width: usize,
    max_dependency_depth: u8,
    mutate: impl FnOnce(&mut Task, &str) -> Result<()>,
) -> Result<Task> {
    mutate_active_task_on_disk_with_validation(
        queue_path,
        done_path,
        task_id,
        id_prefix,
        id_width,
        max_dependency_depth,
        ActiveTaskMutationValidation::ValidateQueueSet,
        mutate,
    )
}

/// Mutate one active task on disk, optionally validate against the done archive, save, and return the task.
#[allow(clippy::too_many_arguments)]
pub fn mutate_active_task_on_disk_with_validation(
    queue_path: &Path,
    done_path: &Path,
    task_id: &str,
    id_prefix: &str,
    id_width: usize,
    max_dependency_depth: u8,
    validation_mode: ActiveTaskMutationValidation,
    mutate: impl FnOnce(&mut Task, &str) -> Result<()>,
) -> Result<Task> {
    let needle = task_id.trim();
    if needle.is_empty() {
        bail!("Task ID cannot be empty when mutating an active task.");
    }

    let mut active = load_queue(queue_path)?;
    let now = crate::timeutil::now_utc_rfc3339()?;
    let task = active
        .tasks
        .iter_mut()
        .find(|task| task.id.trim() == needle)
        .ok_or_else(|| anyhow!("Task {needle} was not found in the active queue."))?;
    mutate(task, &now)?;
    let updated_task = task.clone();

    match validation_mode {
        ActiveTaskMutationValidation::SaveOnly => {}
        ActiveTaskMutationValidation::ValidateQueueSet => {
            let done = load_queue_or_default(done_path)?;
            let done_ref = crate::queue::optional_done_queue(&done, done_path);
            let warnings = validation::validate_queue_set(
                &active,
                done_ref,
                id_prefix,
                id_width,
                max_dependency_depth,
            )?;
            validation::log_warnings(&warnings);
        }
    }

    save_queue(queue_path, &active)?;
    Ok(updated_task)
}
