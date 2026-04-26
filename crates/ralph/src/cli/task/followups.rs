//! Handler for `ralph task followups ...` commands.
//!
//! Purpose:
//! - Handler for `ralph task followups ...` commands.
//!
//! Responsibilities:
//! - Lock queue state before applying proposal-backed queue growth.
//! - Delegate validation/materialization to queue operations.
//! - Render human-readable or JSON apply reports.
//!
//! Not handled here:
//! - Proposal schema semantics beyond CLI option mapping.
//! - Worker prompt guidance or parallel integration orchestration.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Successful non-dry-run applies create undo before saving queue changes.
//! - Dry runs never save queue changes or remove proposal files.

use anyhow::Result;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::cli::task::args::{TaskFollowupsArgs, TaskFollowupsCommand, TaskFollowupsFormatArg};
use crate::config;
use crate::queue::{self, FollowupApplyOptions, FollowupApplyReport};

pub fn handle(args: &TaskFollowupsArgs, force: bool, resolved: &config::Resolved) -> Result<()> {
    match &args.command {
        TaskFollowupsCommand::Apply(args) => {
            let default_proposal_path =
                queue::default_followups_path(&resolved.repo_root, &args.task);
            // #region agent log
            append_debug_log(
                "H1",
                "crates/ralph/src/cli/task/followups.rs:handle",
                "attempting followups apply lock acquisition",
                serde_json::json!({
                    "taskId": args.task.as_str(),
                    "force": force,
                    "defaultProposalPath": default_proposal_path.display().to_string(),
                    "defaultProposalExists": default_proposal_path.exists(),
                    "lockLabel": "task followups apply",
                }),
            );
            // #endregion
            let _queue_lock =
                match queue::acquire_queue_lock(&resolved.repo_root, "task followups apply", force)
                {
                    Ok(lock) => {
                        // #region agent log
                        append_debug_log(
                            "H1",
                            "crates/ralph/src/cli/task/followups.rs:handle",
                            "followups apply lock acquisition succeeded",
                            serde_json::json!({
                                "taskId": args.task.as_str(),
                                "lockLabel": "task followups apply",
                            }),
                        );
                        // #endregion
                        lock
                    }
                    Err(err) => {
                        // #region agent log
                        append_debug_log(
                            "H1",
                            "crates/ralph/src/cli/task/followups.rs:handle",
                            "followups apply lock acquisition failed",
                            serde_json::json!({
                                "taskId": args.task.as_str(),
                                "lockLabel": "task followups apply",
                                "error": format!("{err:#}"),
                            }),
                        );
                        // #endregion
                        return Err(err);
                    }
                };
            let report = queue::apply_followups_file(
                resolved,
                &FollowupApplyOptions {
                    task_id: args.task.as_str(),
                    input_path: args.input.as_deref(),
                    dry_run: args.dry_run,
                    create_undo: true,
                    remove_proposal: true,
                },
            )?;
            // #region agent log
            append_debug_log(
                "H4",
                "crates/ralph/src/cli/task/followups.rs:handle",
                "followups apply finished",
                serde_json::json!({
                    "taskId": args.task.as_str(),
                    "dryRun": args.dry_run,
                    "createdTasksCount": report.created_tasks.len(),
                    "proposalPath": report.proposal_path.as_str(),
                }),
            );
            // #endregion
            print_report(&report, args.format)
        }
    }
}

fn append_debug_log(hypothesis_id: &str, location: &str, message: &str, data: serde_json::Value) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0);
    let payload = serde_json::json!({
        "sessionId": "f05fb4",
        "runId": "pre-fix",
        "hypothesisId": hypothesis_id,
        "location": location,
        "message": message,
        "data": data,
        "timestamp": timestamp,
    });
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/Users/mitchfultz/Projects/AI/ralph/.cursor/debug-f05fb4.log")
        && let Ok(line) = serde_json::to_string(&payload)
    {
        let _ = writeln!(file, "{line}");
    }
}

fn print_report(report: &FollowupApplyReport, format: TaskFollowupsFormatArg) -> Result<()> {
    match format {
        TaskFollowupsFormatArg::Text => print_text_report(report),
        TaskFollowupsFormatArg::Json => {
            println!("{}", serde_json::to_string_pretty(report)?);
            Ok(())
        }
    }
}

fn print_text_report(report: &FollowupApplyReport) -> Result<()> {
    let verb = if report.dry_run {
        "Would create"
    } else {
        "Applied"
    };
    let count = report.created_tasks.len();
    println!(
        "{verb} {count} follow-up task(s) for {}.",
        report.source_task_id
    );
    if count == 0 {
        println!("No follow-up tasks were proposed.");
        return Ok(());
    }

    for task in &report.created_tasks {
        if task.depends_on.is_empty() {
            println!("  - {} [{}]: {}", task.task_id, task.key, task.title);
        } else {
            println!(
                "  - {} [{}]: {} (depends_on: {})",
                task.task_id,
                task.key,
                task.title,
                task.depends_on.join(", ")
            );
        }
    }
    Ok(())
}
