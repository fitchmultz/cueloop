//! Transaction-style task mutation helpers.
//!
//! Purpose:
//! - Transaction-style task mutation helpers.
//!
//! Responsibilities:
//! - Define structured task-mutation requests that can apply multiple field edits atomically.
//! - Enforce optimistic-lock checks against `updated_at` when requested by callers.
//! - Reuse existing edit primitives while providing all-or-nothing mutation semantics.
//!
//! Non-scope:
//! - Queue persistence or lock acquisition.
//! - CLI argument parsing or JSON IO.
//! - Terminal archive moves across queue/done files.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Requests target tasks in the active queue only.
//! - Atomic requests leave the caller's queue untouched when any mutation fails.
//! - `expected_updated_at` compares canonical RFC3339 instants, not source formatting.

use crate::contracts::QueueFile;
use crate::queue::TaskEditKey;
use anyhow::{Context, Result, anyhow, bail};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TaskMutationRequest {
    #[serde(default = "task_mutation_request_version")]
    pub version: u8,
    #[serde(default = "task_mutation_request_atomic_default")]
    pub atomic: bool,
    #[serde(default)]
    pub tasks: Vec<TaskMutationSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TaskMutationSpec {
    pub task_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_updated_at: Option<String>,
    #[serde(default)]
    pub edits: Vec<TaskFieldEdit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TaskFieldEdit {
    pub field: String,
    /// Edit value. In `set` mode, list fields use the same comma/list parsing as human task edits.
    /// In `append` mode, the value is one redacted list item appended literally after trimming.
    #[serde(default)]
    pub value: String,
    #[serde(default)]
    pub mode: TaskFieldEditMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskFieldEditMode {
    #[default]
    Set,
    Append,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskMutationReport {
    #[serde(default = "task_mutation_request_version")]
    pub version: u8,
    pub atomic: bool,
    pub tasks: Vec<TaskMutationTaskReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskMutationTaskReport {
    pub task_id: String,
    pub applied_edits: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum TaskMutationError {
    #[error("Unsupported task mutation request version {version}; expected 1.")]
    UnsupportedVersion { version: u8 },
    #[error("Task mutation request must include at least one task.")]
    EmptyRequest,
    #[error("Task mutation for {task_id} must include at least one edit.")]
    EmptyTaskEdits { task_id: String },
    #[error(
        "Task mutation conflict for {task_id}: expected updated_at {expected}, found {actual}."
    )]
    OptimisticConflict {
        task_id: String,
        expected: String,
        actual: String,
    },
    #[error(
        "Task mutation conflict for {task_id}: expected updated_at {expected}, but the task has no updated_at."
    )]
    MissingActualTimestamp { task_id: String, expected: String },
}

const fn task_mutation_request_version() -> u8 {
    1
}

const fn task_mutation_request_atomic_default() -> bool {
    true
}

#[allow(clippy::too_many_arguments)]
pub fn apply_task_mutation_request(
    queue: &mut QueueFile,
    done: Option<&QueueFile>,
    request: &TaskMutationRequest,
    now_rfc3339: &str,
    id_prefix: &str,
    id_width: usize,
    max_dependency_depth: u8,
) -> Result<TaskMutationReport> {
    if request.version != task_mutation_request_version() {
        return Err(TaskMutationError::UnsupportedVersion {
            version: request.version,
        }
        .into());
    }

    if request.tasks.is_empty() {
        return Err(TaskMutationError::EmptyRequest.into());
    }

    if request.atomic {
        let mut working = queue.clone();
        let report = apply_request_into_queue(
            &mut working,
            done,
            request,
            now_rfc3339,
            id_prefix,
            id_width,
            max_dependency_depth,
        )?;
        *queue = working;
        return Ok(report);
    }

    apply_request_into_queue(
        queue,
        done,
        request,
        now_rfc3339,
        id_prefix,
        id_width,
        max_dependency_depth,
    )
}

#[allow(clippy::too_many_arguments)]
fn apply_request_into_queue(
    queue: &mut QueueFile,
    done: Option<&QueueFile>,
    request: &TaskMutationRequest,
    now_rfc3339: &str,
    id_prefix: &str,
    id_width: usize,
    max_dependency_depth: u8,
) -> Result<TaskMutationReport> {
    let mut reports = Vec::with_capacity(request.tasks.len());

    for task in &request.tasks {
        if task.edits.is_empty() {
            return Err(TaskMutationError::EmptyTaskEdits {
                task_id: task.task_id.trim().to_string(),
            }
            .into());
        }

        ensure_expected_updated_at(queue, task)?;

        for edit in &task.edits {
            let key = edit.field.parse::<TaskEditKey>().with_context(|| {
                format!(
                    "Invalid task mutation field '{}' for task {}",
                    edit.field, task.task_id
                )
            })?;
            match edit.mode {
                TaskFieldEditMode::Set => super::edit::apply_task_edit(
                    queue,
                    done,
                    &task.task_id,
                    key,
                    &edit.value,
                    now_rfc3339,
                    id_prefix,
                    id_width,
                    max_dependency_depth,
                )?,
                TaskFieldEditMode::Append => append_task_field(
                    queue,
                    done,
                    &task.task_id,
                    key,
                    &edit.value,
                    now_rfc3339,
                    id_prefix,
                    id_width,
                    max_dependency_depth,
                )?,
            }
        }

        reports.push(TaskMutationTaskReport {
            task_id: task.task_id.trim().to_string(),
            applied_edits: task.edits.len(),
        });
    }

    Ok(TaskMutationReport {
        version: task_mutation_request_version(),
        atomic: request.atomic,
        tasks: reports,
    })
}

#[allow(clippy::too_many_arguments)]
fn append_task_field(
    queue: &mut QueueFile,
    done: Option<&QueueFile>,
    task_id: &str,
    key: TaskEditKey,
    value: &str,
    now_rfc3339: &str,
    id_prefix: &str,
    id_width: usize,
    max_dependency_depth: u8,
) -> Result<()> {
    let needle = task_id.trim();
    if needle.is_empty() {
        bail!("Task mutation append is missing task_id.");
    }
    let index = queue
        .tasks
        .iter()
        .position(|task| task.id.trim() == needle)
        .ok_or_else(|| anyhow!("{}", crate::error_messages::task_not_found_in_queue(needle)))?;
    let previous = queue.tasks[index].clone();
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!(
            "Task mutation append for {needle} field {} requires a non-empty value.",
            key.as_str()
        );
    }

    {
        let task = queue
            .tasks
            .get_mut(index)
            .ok_or_else(|| anyhow!("{}", crate::error_messages::task_not_found_in_queue(needle)))?;
        let appended = crate::redaction::redact_text(trimmed);
        match key {
            TaskEditKey::Tags => task.tags.push(appended),
            TaskEditKey::Scope => task.scope.push(appended),
            TaskEditKey::Evidence => task.evidence.push(appended),
            TaskEditKey::Plan => task.plan.push(appended),
            TaskEditKey::Notes => task.notes.push(appended),
            TaskEditKey::DependsOn => task.depends_on.push(appended),
            TaskEditKey::Blocks => task.blocks.push(appended),
            TaskEditKey::RelatesTo => task.relates_to.push(appended),
            other => bail!(
                "Task mutation append does not support field {}; use mode=set.",
                other.as_str()
            ),
        }
        task.updated_at = Some(now_rfc3339.to_string());
    }

    match crate::queue::validate_queue_set(queue, done, id_prefix, id_width, max_dependency_depth) {
        Ok(warnings) => crate::queue::log_warnings(&warnings),
        Err(err) => {
            queue.tasks[index] = previous;
            return Err(err);
        }
    }

    Ok(())
}

fn ensure_expected_updated_at(queue: &QueueFile, task: &TaskMutationSpec) -> Result<()> {
    let Some(expected) = task.expected_updated_at.as_ref() else {
        return Ok(());
    };

    let task_id = task.task_id.trim();
    if task_id.is_empty() {
        bail!("Task mutation is missing task_id.");
    }

    let current = queue
        .tasks
        .iter()
        .find(|candidate| candidate.id.trim() == task_id)
        .ok_or_else(|| anyhow!("{}", crate::error_messages::task_not_found(task_id)))?;

    let expected_trimmed = expected.trim();
    let expected_dt = crate::timeutil::parse_rfc3339(expected_trimmed)
        .with_context(|| format!("parse expected updated_at for task {}", task_id))?;

    match current.updated_at.as_deref().map(str::trim) {
        Some(actual)
            if crate::timeutil::parse_rfc3339(actual)
                .map(|actual_dt| actual_dt == expected_dt)
                .unwrap_or(false) =>
        {
            Ok(())
        }
        Some(actual) => Err(TaskMutationError::OptimisticConflict {
            task_id: task_id.to_string(),
            expected: expected_trimmed.to_string(),
            actual: actual.to_string(),
        }
        .into()),
        None => Err(TaskMutationError::MissingActualTimestamp {
            task_id: task_id.to_string(),
            expected: expected_trimmed.to_string(),
        }
        .into()),
    }
}
