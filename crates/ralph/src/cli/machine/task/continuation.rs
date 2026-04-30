//! Machine task continuation document builders.
//!
//! Purpose:
//! - Machine task continuation document builders.
//!
//! Responsibilities:
//! - Build stable continuation summaries for task mutation and decomposition flows.
//! - Keep operator-facing next steps and blocking narration consistent for machine task commands.
//!
//! Not handled here:
//! - Queue/task write orchestration.
//! - Machine JSON envelope assembly.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Continuation text stays deterministic for machine consumers.
//! - Blocking summaries use the shared machine/blocking contract types.

use crate::cli::machine::common::{
    machine_queue_graph_command, machine_queue_undo_dry_run_command,
    machine_queue_validate_command, machine_run_one_resume_command, machine_task_build_command,
    machine_task_decompose_write_preview_command, machine_task_mutate_command,
};
use crate::commands::task as task_cmd;
use crate::contracts::{
    BlockingState, BlockingStatus, MachineContinuationAction, MachineContinuationSummary,
};

fn step(title: &str, command: &str, detail: &str) -> MachineContinuationAction {
    MachineContinuationAction {
        title: title.to_string(),
        command: command.to_string(),
        detail: detail.to_string(),
    }
}

pub(super) fn build_continuation(created_count: usize) -> MachineContinuationSummary {
    MachineContinuationSummary {
        headline: "AI Build has created task drafts.".to_string(),
        detail: format!(
            "Ralph ran the task-builder prompt and added {created_count} task(s) to the queue."
        ),
        blocking: None,
        next_steps: vec![
            step(
                "Review created tasks",
                machine_queue_graph_command(),
                "Inspect the new task placement, relationships, estimates, and scope.",
            ),
            step(
                "Run the next task",
                machine_run_one_resume_command(),
                "Continue from the updated queue when the task drafts look right.",
            ),
            step(
                "Build another task",
                machine_task_build_command(),
                "Use the same machine contract for another guided AI task creation.",
            ),
        ],
    }
}

pub(super) fn mutation_continuation(
    task_count: usize,
    dry_run: bool,
) -> MachineContinuationSummary {
    if dry_run {
        return MachineContinuationSummary {
            headline: "Mutation continuation is ready.".to_string(),
            detail: format!(
                "Ralph validated an atomic mutation affecting {task_count} task(s) without writing queue changes."
            ),
            blocking: None,
            next_steps: vec![
                step(
                    "Apply the mutation",
                    machine_task_mutate_command(false),
                    "Repeat without --dry-run to write the validated transaction.",
                ),
                step(
                    "Refresh if the queue moved",
                    machine_queue_validate_command(),
                    "Validate and retry from the latest queue state if another write landed first.",
                ),
            ],
        };
    }

    MachineContinuationSummary {
        headline: "Task mutation has been applied.".to_string(),
        detail: format!(
            "Ralph wrote {task_count} task mutation(s) atomically and created an undo checkpoint first."
        ),
        blocking: None,
        next_steps: vec![
            step(
                "Continue work",
                machine_run_one_resume_command(),
                "Proceed from the updated task state.",
            ),
            step(
                "Restore if needed",
                machine_queue_undo_dry_run_command(),
                "Preview the rollback path for this mutation.",
            ),
        ],
    }
}

pub(super) fn decompose_continuation(
    preview: &task_cmd::DecompositionPreview,
    write: Option<&task_cmd::TaskDecomposeWriteResult>,
    checkpoint: Option<&task_cmd::DecompositionPreviewCheckpointRef>,
) -> MachineContinuationSummary {
    if let Some(write) = write {
        let first_leaf_id = write.first_actionable_leaf_task_id.as_deref();
        let all_draft = preview.all_generated_tasks_draft();
        let has_runnable_leaf = preview.has_runnable_generated_leaf();
        let detail = match (first_leaf_id, all_draft, has_runnable_leaf) {
            (Some(first_leaf_id), true, _) => format!(
                "Ralph wrote the planned task tree in draft and created an undo checkpoint before mutating the queue. Promote first actionable leaf {first_leaf_id} before normal run selection."
            ),
            (Some(first_leaf_id), _, true) => format!(
                "Ralph wrote the planned task tree and created an undo checkpoint before mutating the queue. Leaf work is already runnable; start review with first actionable leaf {first_leaf_id}."
            ),
            (Some(first_leaf_id), _, _) => format!(
                "Ralph wrote the planned task tree and created an undo checkpoint before mutating the queue. Start review with first actionable leaf {first_leaf_id}."
            ),
            (None, _, _) => "Ralph wrote the planned task tree and created an undo checkpoint before mutating the queue."
                .to_string(),
        };
        let headline = if all_draft {
            "Decomposition has been written as draft."
        } else if has_runnable_leaf {
            "Decomposition has been written with runnable leaves."
        } else {
            "Decomposition has been written."
        };
        let mut next_steps = Vec::new();
        if all_draft && let Some(first_leaf_id) = first_leaf_id {
            next_steps.push(step(
                "Promote the first actionable leaf",
                &format!("ralph task ready {first_leaf_id}"),
                "Mark the first actionable leaf todo so Ralph can run it.",
            ));
        }
        next_steps.push(step(
            "Inspect the tree",
            machine_queue_graph_command(),
            "Review the written parent/child structure.",
        ));
        if has_runnable_leaf {
            next_steps.push(step(
                "Run the next task",
                machine_run_one_resume_command(),
                "Continue from the updated queue; generated leaf work is already todo.",
            ));
        }
        next_steps.push(step(
            "Restore if needed",
            machine_queue_undo_dry_run_command(),
            "Preview the rollback path for this decomposition.",
        ));
        return MachineContinuationSummary {
            headline: headline.to_string(),
            detail,
            blocking: None,
            next_steps,
        };
    }

    if preview.write_blockers.is_empty() {
        let actionability = preview.plan.actionability();
        let detail = actionability
            .first_actionable_leaf
            .as_ref()
            .map(|leaf| {
                format!(
                    "Ralph planned a task tree that can be written when you are ready. First actionable leaf: {}.",
                    leaf.title
                )
            })
            .unwrap_or_else(|| {
                "Ralph planned a task tree that can be written when you are ready.".to_string()
            });
        let write_command = checkpoint
            .map(|checkpoint| machine_task_decompose_write_preview_command(&checkpoint.id))
            .unwrap_or_else(|| "ralph machine task decompose --write <SOURCE>".to_string());
        let replay_detail = checkpoint
            .map(|checkpoint| {
                format!(
                    "Persist this exact saved preview from checkpoint {} into the queue.",
                    checkpoint.id
                )
            })
            .unwrap_or_else(|| {
                "Run write mode with an explicit source; this invokes the planner again and may differ from this preview."
                    .to_string()
            });
        return MachineContinuationSummary {
            headline: "Decomposition preview is ready.".to_string(),
            detail,
            blocking: None,
            next_steps: vec![step("Write the preview", &write_command, &replay_detail)],
        };
    }

    MachineContinuationSummary {
        headline: "Decomposition preview is blocked from being written.".to_string(),
        detail: "Ralph preserved the proposed tree, but a queue invariant must be resolved before write mode can continue.".to_string(),
        blocking: Some(
            BlockingState::operator_recovery(
                BlockingStatus::Blocked,
                "task_decompose",
                "write_blocked",
                preview
                    .attach_target
                    .as_ref()
                    .map(|target| target.task.id.clone()),
                "Ralph is blocked from continuing this decomposition write.",
                preview.write_blockers.join(" "),
                checkpoint.map(|checkpoint| machine_task_decompose_write_preview_command(&checkpoint.id)),
            )
            .with_observed_at(crate::timeutil::now_utc_rfc3339_or_fallback()),
        ),
        next_steps: blocked_decompose_steps(checkpoint),
    }
}

fn blocked_decompose_steps(
    checkpoint: Option<&task_cmd::DecompositionPreviewCheckpointRef>,
) -> Vec<MachineContinuationAction> {
    let mut steps = vec![
        step(
            "Inspect the queue graph",
            machine_queue_graph_command(),
            "Review the existing child subtree and decide whether to mutate it before retrying.",
        ),
        step(
            "Validate the queue",
            machine_queue_validate_command(),
            "Confirm current queue invariants before attempting another decomposition write.",
        ),
    ];
    if let Some(checkpoint) = checkpoint {
        let command = machine_task_decompose_write_preview_command(&checkpoint.id);
        steps.insert(
            0,
            step(
                "Retry exact preview write",
                &command,
                "Retry writing the same saved preview after resolving the blocker; this does not invoke the planner again.",
            ),
        );
    }
    steps
}
