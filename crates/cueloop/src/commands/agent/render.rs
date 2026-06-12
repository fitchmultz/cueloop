//! Agent ledger terminal and JSON rendering.
//!
//! Purpose:
//! - Own stdout rendering for `cueloop agent` command documents.
//!
//! Responsibilities:
//! - Render compact human-readable output for overview, task, handoff, mutation, and validation docs.
//! - Render pretty JSON for `--format json` responses.
//! - Keep presentation logic out of mutation and queue operation modules.
//!
//! Non-scope:
//! - Building document data from queues.
//! - Mutating or validating queue files.
//! - Machine-contract output rendering.
//!
//! Invariants/assumptions:
//! - Human output remains concise and line-oriented for shell use.
//! - JSON output uses the document structs exactly as built by `documents`.

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::cli::agent_ledger::AgentOutputFormat;
use crate::contracts::Task;

use super::documents::{
    AgentHandoffDocument, AgentMutationDocument, AgentOverviewDocument, AgentTaskDocument,
    AgentValidateDocument,
};

pub(super) fn print_overview(
    document: &AgentOverviewDocument,
    format: AgentOutputFormat,
) -> Result<()> {
    if format == AgentOutputFormat::Json {
        return print_json(document);
    }
    println!("Active tasks: {}", document.active_count);
    println!("Done archive tasks: {}", document.done_count);
    match &document.next_runnable_task_id {
        Some(id) => println!("Next runnable: {id}"),
        None => println!("Next runnable: none"),
    }
    if let Some(error) = &document.validation_error {
        println!("Validation: invalid - {error}");
    }
    if !document.doing.is_empty() {
        println!("Doing:");
        for task in &document.doing {
            println!("  {}\t{}", task.id, task.title);
        }
    }
    if !document.active.is_empty() {
        println!("Active queue:");
        for task in &document.active {
            println!("  {}\t{}\t{}", task.id, task.status.as_str(), task.title);
        }
    }
    if !document.recent_done.is_empty() {
        println!("Recent done:");
        for task in &document.recent_done {
            println!("  {}\t{}\t{}", task.id, task.status.as_str(), task.title);
        }
    }
    Ok(())
}

pub(super) fn print_next_task(task: Option<&Task>, with_title: bool) {
    match task {
        Some(task) if with_title => {
            println!("{}\t{}", task.id, task.title);
        }
        Some(task) => println!("{}", task.id),
        None => println!("No runnable task."),
    }
}

pub(super) fn print_task_document(
    document: &AgentTaskDocument,
    format: AgentOutputFormat,
) -> Result<()> {
    if format == AgentOutputFormat::Json {
        return print_json(document);
    }
    println!(
        "{}\t{}\t{}",
        document.task.id, document.task.status, document.task.title
    );
    if let Some(description) = document.task.description.as_deref()
        && !description.trim().is_empty()
    {
        println!("\n{description}");
    }
    print_list("Scope", &document.task.scope);
    print_list("Plan", &document.task.plan);
    print_list("Evidence", &document.task.evidence);
    print_list("Notes", &document.task.notes);
    Ok(())
}

pub(super) fn print_handoff(
    document: &AgentHandoffDocument,
    format: AgentOutputFormat,
) -> Result<()> {
    if format == AgentOutputFormat::Json {
        return print_json(document);
    }
    println!("Handoff for {}: {}", document.task.id, document.task.title);
    println!("Status: {}", document.task.status);
    if !document.claim.is_empty() {
        println!("Claim:");
        for (key, value) in &document.claim {
            println!("  {key}: {value}");
        }
    }
    if let Some(next) = &document.next_step {
        println!("Next: {next}");
    }
    print_list("Remaining plan", &document.remaining_plan);
    print_list("Evidence", &document.evidence);
    print_list("Recent notes", &document.recent_notes);
    Ok(())
}

pub(super) fn print_mutation(
    document: &AgentMutationDocument,
    format: AgentOutputFormat,
) -> Result<()> {
    if format == AgentOutputFormat::Json {
        return print_json(document);
    }
    if let Some(task) = &document.task {
        println!("{}\t{}\t{}", task.id, document.action, task.title);
    }
    Ok(())
}

pub(super) fn print_validate(
    document: AgentValidateDocument,
    format: AgentOutputFormat,
) -> Result<()> {
    if format == AgentOutputFormat::Json {
        return print_json(&document);
    }
    if document.valid {
        println!("Queue is valid.");
        for warning in &document.warnings {
            println!("warning: {warning}");
        }
        Ok(())
    } else {
        bail!(
            document
                .error
                .unwrap_or_else(|| "queue is invalid".to_string())
        )
    }
}

fn print_list(label: &str, values: &[String]) {
    if values.is_empty() {
        return;
    }
    println!("\n{label}:");
    for value in values {
        println!("  - {value}");
    }
}

pub(super) fn print_json(value: &impl Serialize) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).context("serialize agent ledger JSON")?
    );
    Ok(())
}
