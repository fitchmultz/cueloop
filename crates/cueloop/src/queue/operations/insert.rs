//! Purpose: Apply atomic task-insert requests through the shared materializer.
//!
//! Responsibilities:
//! - Validate versioned task-insert requests before queue mutation.
//! - Convert request specs into canonical materialized task specs.
//! - Return assigned durable IDs in request order after validation succeeds.
//!
//! Scope:
//! - In-memory queue mutation only; lock acquisition, undo, and persistence live
//!   in CLI handlers.
//!
//! Usage:
//! - Used by `cueloop task insert` and `cueloop machine task insert`.
//!
//! Invariants/Assumptions:
//! - Requests omit durable IDs and use local `key` values instead.
//! - Active queue mutation stays all-or-nothing.

use anyhow::{Result, bail};

use crate::contracts::{
    QueueFile, TASK_INSERT_VERSION, TaskInsertCreatedTask, TaskInsertDocument, TaskInsertRequest,
    TaskInsertSpec, TaskStatus,
};

use super::{
    MaterializeInsertion, MaterializeTaskGraphOptions, MaterializedTaskSpec,
    apply_materialized_task_graph,
};

#[allow(clippy::too_many_arguments)]
pub fn apply_task_insert_request(
    active: &mut QueueFile,
    done: Option<&QueueFile>,
    request: &TaskInsertRequest,
    now_rfc3339: &str,
    id_prefix: &str,
    id_width: usize,
    max_dependency_depth: u8,
    dry_run: bool,
) -> Result<TaskInsertDocument> {
    validate_request(request)?;

    let specs = request
        .tasks
        .iter()
        .map(materialize_spec_from_request)
        .collect::<Result<Vec<_>>>()?;

    let report = apply_materialized_task_graph(
        active,
        done,
        &specs,
        &MaterializeTaskGraphOptions {
            now_rfc3339,
            id_prefix,
            id_width,
            max_dependency_depth,
            insertion: MaterializeInsertion::QueueDefaultTop,
            dry_run,
        },
    )?;

    let tasks = specs
        .iter()
        .zip(report.created_tasks)
        .map(|(spec, task)| TaskInsertCreatedTask {
            key: spec.local_key.clone(),
            task,
        })
        .collect::<Vec<_>>();

    Ok(TaskInsertDocument {
        version: TASK_INSERT_VERSION,
        dry_run,
        created_count: tasks.len(),
        tasks,
    })
}

fn validate_request(request: &TaskInsertRequest) -> Result<()> {
    if request.version != TASK_INSERT_VERSION {
        bail!(
            "Unsupported task insert request version {}",
            request.version
        );
    }
    if request.tasks.is_empty() {
        bail!("Task insert request must contain at least one task");
    }
    Ok(())
}

fn materialize_spec_from_request(spec: &TaskInsertSpec) -> Result<MaterializedTaskSpec> {
    if matches!(spec.status, TaskStatus::Done | TaskStatus::Rejected) {
        bail!(
            "task key {} cannot use terminal status {} in active-queue insert requests",
            spec.key.trim(),
            spec.status.as_str()
        );
    }

    Ok(MaterializedTaskSpec {
        local_key: spec.key.clone(),
        title: spec.title.clone(),
        description: spec.description.clone(),
        priority: spec.priority,
        status: spec.status,
        kind: spec.kind,
        tags: spec.tags.clone(),
        scope: spec.scope.clone(),
        evidence: spec.evidence.clone(),
        plan: spec.plan.clone(),
        notes: spec.notes.clone(),
        request: spec.request.clone(),
        relates_to: spec.relates_to.clone(),
        blocks: spec.blocks.clone(),
        duplicates: spec.duplicates.clone(),
        custom_fields: spec.custom_fields.clone(),
        agent: spec.agent.clone(),
        parent_local_key: spec.parent_key.clone(),
        parent_task_id: spec.parent_id.clone(),
        depends_on_local_keys: spec.depends_on_keys.clone(),
        depends_on_task_ids: spec.depends_on.clone(),
        estimated_minutes: spec.estimated_minutes,
    })
}
