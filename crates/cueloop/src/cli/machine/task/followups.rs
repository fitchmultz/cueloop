//! Machine task follow-up application.
//!
//! Purpose:
//! - Apply agent-proposed follow-up tasks through the machine task surface.
//!
//! Responsibilities:
//! - Route follow-up subcommands.
//! - Call the queue follow-up application workflow with machine-safe options.
//! - Wrap follow-up application reports in a versioned machine document.
//!
//! Not handled here:
//! - Follow-up proposal generation.
//! - Generic task insertion request parsing.
//! - Machine task router dispatch beyond follow-up subcommands.
//!
//! Usage:
//! - Called by the machine task router for `machine task followups apply`.
//!
//! Invariants/assumptions:
//! - Non-dry-run follow-up application creates undo state through queue follow-up options.
//! - Follow-up reports are serialized as-is inside the machine document result field.

use anyhow::Result;

use crate::cli::machine::args::{MachineTaskFollowupsArgs, MachineTaskFollowupsCommand};
use crate::config;
use crate::contracts::{
    MACHINE_TASK_FOLLOWUPS_VERSION, MachineContinuationAction, MachineContinuationSummary,
    MachineTaskFollowupsDocument,
};
use crate::queue;

pub(super) fn handle_followups(
    resolved: &config::Resolved,
    args: MachineTaskFollowupsArgs,
    force: bool,
) -> Result<MachineTaskFollowupsDocument> {
    match args.command {
        MachineTaskFollowupsCommand::Apply(args) => {
            let _queue_lock = queue::acquire_queue_lock(
                &resolved.repo_root,
                "machine task followups apply",
                force,
            )?;
            let report = queue::apply_followups_file(
                resolved,
                &queue::FollowupApplyOptions {
                    task_id: args.task.as_str(),
                    input_path: args.input.as_deref(),
                    dry_run: args.dry_run,
                    create_undo: true,
                    remove_proposal: true,
                },
            )?;
            build_task_followups_document(&report, args.dry_run)
        }
    }
}

fn build_task_followups_document(
    report: &queue::FollowupApplyReport,
    dry_run: bool,
) -> Result<MachineTaskFollowupsDocument> {
    Ok(MachineTaskFollowupsDocument {
        version: MACHINE_TASK_FOLLOWUPS_VERSION,
        dry_run,
        report: serde_json::to_value(report)?,
        continuation: MachineContinuationSummary {
            headline: if dry_run {
                "Follow-up application preview is ready.".to_string()
            } else {
                "Follow-up proposal has been applied.".to_string()
            },
            detail: format!(
                "{} follow-up task(s) for {}.",
                report.created_tasks.len(),
                report.source_task_id
            ),
            blocking: None,
            next_steps: vec![MachineContinuationAction {
                title: "Validate queue".to_string(),
                command: "cueloop machine queue validate".to_string(),
                detail: "Confirm inserted follow-up tasks and dependency edges.".to_string(),
            }],
        },
    })
}
