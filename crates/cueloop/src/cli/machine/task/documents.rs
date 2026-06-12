//! Machine task document builders.
//!
//! Purpose:
//! - Assemble stable machine task JSON envelopes that are reused outside the router.
//!
//! Responsibilities:
//! - Build mutation and decomposition machine documents.
//! - Preserve the app-facing re-exported builder API.
//! - Flatten decomposition preview trees into the compatibility `tasks` list.
//!
//! Not handled here:
//! - Reading CLI input.
//! - Queue mutation or decomposition planning.
//! - Lifecycle/follow-up command execution.
//!
//! Usage:
//! - Re-exported by `cli::machine` as `build_task_mutation_document` and
//!   `build_task_decompose_document`.
//!
//! Invariants/assumptions:
//! - Machine documents remain versioned and JSON-compatible.
//! - Decomposition flattening order stays pre-order and behavior-preserving.

use anyhow::Result;
use serde::Serialize;

use crate::commands::task as task_cmd;
use crate::contracts::{
    MACHINE_DECOMPOSE_VERSION, MACHINE_TASK_MUTATION_VERSION, MachineDecomposeDocument,
    MachineTaskMutationDocument, TaskStatus,
};
use crate::queue;

use super::continuation::{decompose_continuation, mutation_continuation};

pub(crate) fn build_task_mutation_document(
    report: &queue::operations::TaskMutationReport,
    dry_run: bool,
) -> Result<MachineTaskMutationDocument> {
    let continuation = mutation_continuation(report.tasks.len(), dry_run);
    let blocking = continuation.blocking.clone();

    Ok(MachineTaskMutationDocument {
        version: MACHINE_TASK_MUTATION_VERSION,
        blocking,
        report: serde_json::to_value(report)?,
        continuation,
    })
}

pub(crate) fn build_decompose_document(
    preview: &task_cmd::DecompositionPreview,
    write: Option<&task_cmd::TaskDecomposeWriteResult>,
    checkpoint: Option<&task_cmd::DecompositionPreviewCheckpointRef>,
) -> MachineDecomposeDocument {
    let continuation = decompose_continuation(preview, write, checkpoint);
    let blocking = continuation.blocking.clone();
    let tasks = flatten_decompose_tasks(preview);

    MachineDecomposeDocument {
        version: MACHINE_DECOMPOSE_VERSION,
        blocking,
        result: serde_json::json!({
            "version": 2,
            "mode": if write.is_some() { "write" } else { "preview" },
            "preview": preview,
            "tasks": tasks,
            "write": write,
            "checkpoint": checkpoint,
            "replay_exact": checkpoint.is_some(),
        }),
        continuation,
    }
}

#[derive(Serialize)]
struct FlattenedDecomposeTask<'a> {
    id: Option<&'a str>,
    key: &'a str,
    title: &'a str,
    status: TaskStatus,
    depends_on_keys: &'a [String],
}

fn flatten_decompose_tasks(
    preview: &task_cmd::DecompositionPreview,
) -> Vec<FlattenedDecomposeTask<'_>> {
    let mut tasks = Vec::new();
    flatten_decompose_node(&preview.plan.root, preview, &mut tasks);
    tasks
}

fn flatten_decompose_node<'a>(
    node: &'a task_cmd::PlannedNode,
    preview: &task_cmd::DecompositionPreview,
    tasks: &mut Vec<FlattenedDecomposeTask<'a>>,
) {
    let status = if node.children.is_empty() {
        preview.leaf_status
    } else {
        preview.parent_status
    };
    tasks.push(FlattenedDecomposeTask {
        id: None,
        key: &node.planner_key,
        title: &node.title,
        status,
        depends_on_keys: &node.depends_on_keys,
    });
    for child in &node.children {
        flatten_decompose_node(child, preview, tasks);
    }
}
