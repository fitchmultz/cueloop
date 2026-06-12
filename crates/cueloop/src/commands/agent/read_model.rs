//! Agent ledger read-side document builders.
//!
//! Purpose:
//! - Build read-only agent ledger documents from queue state.
//!
//! Responsibilities:
//! - Load and validate active/done queue files for overview, next, show, and validate commands.
//! - Select runnable tasks using shared queue runnability helpers.
//! - Keep read-side queue inspection out of the command router and mutation module.
//!
//! Non-scope:
//! - Mutating queue files or acquiring queue locks.
//! - Rendering documents to stdout.
//! - Machine-contract envelope construction.
//!
//! Invariants/assumptions:
//! - Overview validation errors are captured into the overview document instead of aborting.
//! - `next` validates the queue set before selecting a runnable task.
//! - Task lookup uses the shared active/done lookup helper under `queue::operations`.

use anyhow::Result;

use crate::cli::agent_ledger::{AgentNextArgs, AgentOverviewArgs};
use crate::config;
use crate::contracts::{QueueFile, Task, TaskStatus};
use crate::queue;
use crate::queue::operations::{
    RunnableSelectionOptions, queue_runnability_report, select_runnable_task_index,
};

use super::documents::{
    AgentBlockedTask, AgentOverviewDocument, AgentTaskDocument, AgentValidateDocument,
    DOCUMENT_VERSION, build_task_document, task_summary,
};

pub(super) fn overview_document(
    resolved: &config::Resolved,
    args: &AgentOverviewArgs,
) -> Result<AgentOverviewDocument> {
    let active = queue::load_queue(&resolved.queue_path)?;
    let done = queue::load_queue_or_default(&resolved.done_path)?;
    let done_ref = queue::optional_done_queue(&done, &resolved.done_path);
    let validation = queue::validate_queue_set(
        &active,
        done_ref,
        &resolved.id_prefix,
        resolved.id_width,
        resolved.queue_max_dependency_depth(),
    );

    let (next_runnable_task_id, blocked, validation_error) = match validation {
        Ok(_) => {
            let selection_options = RunnableSelectionOptions::new(false, true);
            let report = queue_runnability_report(&active, done_ref, selection_options)?;
            let next =
                selected_task(&active, done_ref, selection_options).map(|task| task.id.clone());
            let blocked = report
                .tasks
                .into_iter()
                .filter(|row| !row.runnable && !row.reasons.is_empty())
                .filter_map(|row| {
                    let task = active.tasks.iter().find(|task| task.id == row.id)?;
                    let reasons = serde_json::to_value(row.reasons).ok()?;
                    Some(AgentBlockedTask {
                        id: task.id.clone(),
                        title: task.title.clone(),
                        reasons,
                    })
                })
                .collect();
            (next, blocked, None)
        }
        Err(err) => (None, Vec::new(), Some(err.to_string())),
    };

    let recent_done = if args.include_done {
        done.tasks
            .iter()
            .rev()
            .take(args.done_limit)
            .map(task_summary)
            .collect()
    } else {
        Vec::new()
    };

    Ok(AgentOverviewDocument {
        version: DOCUMENT_VERSION,
        active_count: active.tasks.len(),
        done_count: done.tasks.len(),
        next_runnable_task_id,
        doing: active
            .tasks
            .iter()
            .filter(|task| task.status == TaskStatus::Doing)
            .map(task_summary)
            .collect(),
        blocked,
        active: active.tasks.iter().map(task_summary).collect(),
        recent_done,
        validation_error,
    })
}

pub(super) fn next_task(
    resolved: &config::Resolved,
    _args: &AgentNextArgs,
) -> Result<Option<Task>> {
    let active = queue::load_queue(&resolved.queue_path)?;
    let done = queue::load_queue_or_default(&resolved.done_path)?;
    let done_ref = queue::optional_done_queue(&done, &resolved.done_path);
    queue::validate_queue_set(
        &active,
        done_ref,
        &resolved.id_prefix,
        resolved.id_width,
        resolved.queue_max_dependency_depth(),
    )?;
    Ok(selected_task(
        &active,
        done_ref,
        RunnableSelectionOptions::new(false, true),
    )
    .cloned())
}

fn selected_task<'a>(
    active: &'a QueueFile,
    done: Option<&'a QueueFile>,
    options: RunnableSelectionOptions,
) -> Option<&'a Task> {
    select_runnable_task_index(active, done, options).and_then(|idx| active.tasks.get(idx))
}

pub(super) fn task_document(
    resolved: &config::Resolved,
    task_id: &str,
) -> Result<AgentTaskDocument> {
    let located = queue::operations::load_task_with_location(
        &resolved.queue_path,
        &resolved.done_path,
        task_id,
    )?;
    Ok(build_task_document(located))
}

pub(super) fn validate_document(resolved: &config::Resolved) -> Result<AgentValidateDocument> {
    let active = queue::load_queue(&resolved.queue_path)?;
    let done = queue::load_queue_or_default(&resolved.done_path)?;
    let done_ref = queue::optional_done_queue(&done, &resolved.done_path);
    Ok(
        match queue::validate_queue_set(
            &active,
            done_ref,
            &resolved.id_prefix,
            resolved.id_width,
            resolved.queue_max_dependency_depth(),
        ) {
            Ok(warnings) => AgentValidateDocument {
                version: DOCUMENT_VERSION,
                valid: true,
                warnings: warnings
                    .into_iter()
                    .map(|warning| format!("[{}] {}", warning.task_id, warning.message))
                    .collect(),
                error: None,
            },
            Err(err) => AgentValidateDocument {
                version: DOCUMENT_VERSION,
                valid: false,
                warnings: Vec::new(),
                error: Some(err.to_string()),
            },
        },
    )
}
