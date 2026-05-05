//! Atomic task insertion handler for `cueloop task insert`.
//!
//! Purpose:
//! - Insert one or more full task specs while allocating IDs under the queue lock.
//!
//! Responsibilities:
//! - Read a JSON task-insert request from stdin or a file.
//! - Materialize tasks atomically through the shared queue operation layer.
//! - Persist queue changes and print text or JSON results.
//!
//! Not handled here:
//! - Manual queue editing flows.
//! - Generic task import/export.

use crate::cli::task::args::{TaskInsertArgs, TaskInsertFormatArg};
use crate::config;
use crate::contracts::{TaskInsertDocument, TaskInsertRequest};
use crate::queue;
use crate::queue::operations::apply_task_insert_request;
use crate::timeutil;
use anyhow::{Context, Result, bail};
use std::fs;
use std::io::Read;

pub fn handle(args: &TaskInsertArgs, force: bool, resolved: &config::Resolved) -> Result<()> {
    let raw = read_request(args).context("read task insert request")?;
    let request = serde_json::from_str::<TaskInsertRequest>(&raw)
        .context("parse task insert request json")?;

    let _queue_lock = queue::acquire_queue_lock(&resolved.repo_root, "task", force)?;
    let mut active = queue::load_queue(&resolved.queue_path)?;
    let done = queue::load_queue_or_default(&resolved.done_path)?;
    let done_ref = queue::optional_done_queue(&done, &resolved.done_path);
    let now = timeutil::now_utc_rfc3339()?;

    let document = apply_task_insert_request(
        &mut active,
        done_ref,
        &request,
        &now,
        &resolved.id_prefix,
        resolved.id_width,
        resolved.queue_max_dependency_depth(),
        args.dry_run,
    )?;

    if !args.dry_run {
        crate::undo::create_undo_snapshot(
            resolved,
            &format!("task insert [{} task(s)]", document.created_count),
        )?;
        queue::save_queue(&resolved.queue_path, &active)?;
    }

    print_report(&document, args.format)
}

fn read_request(args: &TaskInsertArgs) -> Result<String> {
    if let Some(path) = args.input.as_deref() {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            bail!("--input must be a non-empty path");
        }
        return fs::read_to_string(trimmed)
            .with_context(|| format!("read task insert request from {}", trimmed));
    }

    let mut stdin = std::io::stdin().lock();
    let mut raw = String::new();
    stdin
        .read_to_string(&mut raw)
        .context("read task insert request from stdin")?;
    if raw.trim().is_empty() {
        bail!("Task insert request is empty. Pass --input or pipe JSON on stdin.");
    }
    Ok(raw)
}

fn print_report(document: &TaskInsertDocument, format: TaskInsertFormatArg) -> Result<()> {
    match format {
        TaskInsertFormatArg::Text => {
            if document.dry_run {
                println!("Task insertion preview is ready.");
                println!(
                    "CueLoop allocated IDs under the queue lock but did not save queue changes."
                );
            } else {
                println!("Task insertion has been applied.");
                println!(
                    "CueLoop assigned IDs under the queue lock and saved the queue atomically."
                );
            }
            println!();
            println!("Tasks:");
            for created in &document.tasks {
                println!(
                    "  - {} [{}] {}",
                    created.task.id, created.key, created.task.title
                );
            }
        }
        TaskInsertFormatArg::Json => {
            println!("{}", serde_json::to_string_pretty(document)?);
        }
    }
    Ok(())
}
