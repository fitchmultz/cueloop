//! Agent-ledger command router.
//!
//! Purpose:
//! - Route `cueloop agent ...`, a no-runner task ledger surface for already-running agents.
//!
//! Responsibilities:
//! - Resolve repository configuration once per command.
//! - Dispatch read, mutation, handoff, lifecycle, and validation subcommands.
//! - Keep command routing thin while delegating documents, rendering, and mutations to focused modules.
//!
//! Non-scope:
//! - Dispatching runner CLIs, supervising phases, or replacing the human `task` / `queue` commands.
//! - Owning queue persistence or lifecycle policy; shared helpers live under `queue::operations`.
//! - Formatting output beyond choosing the document/rendering path for each command.
//!
//! Invariants/assumptions:
//! - Mutating commands only target active queue tasks and create undo snapshots through queue locks.
//! - Completion requires explicit evidence from the caller.
//! - Serialized JSON documents are compact convenience output, not the hidden machine contract.

mod documents;
mod mutations;
mod read_model;
mod render;

use anyhow::Result;

use crate::cli::agent_ledger::{AgentArgs, AgentCommand, AgentOutputFormat};
use crate::config;
use crate::contracts::Task;
use crate::queue;

use self::documents::{
    AgentAppendField, DOCUMENT_VERSION, build_handoff_document, build_mutation_document,
};

pub fn handle(args: AgentArgs, force: bool) -> Result<()> {
    let resolved = config::resolve_from_cwd()?;
    match args.command {
        AgentCommand::Overview(args) => {
            let document = read_model::overview_document(&resolved, &args)?;
            render::print_overview(&document, args.format)
        }
        AgentCommand::Next(args) => {
            let task = read_model::next_task(&resolved, &args)?;
            if args.format == AgentOutputFormat::Json {
                return render::print_json(&serde_json::json!({
                    "version": DOCUMENT_VERSION,
                    "task": task,
                }));
            }
            render::print_next_task(task.as_ref(), args.with_title);
            Ok(())
        }
        AgentCommand::Show(args) => {
            let document = read_model::task_document(&resolved, &args.task_id)?;
            render::print_task_document(&document, args.format)
        }
        AgentCommand::Claim(args) => {
            let task = mutations::claim(&resolved, &args, force)?;
            print_mutation("claimed", &task, false, args.format)
        }
        AgentCommand::Release(args) => {
            let task = mutations::release(&resolved, &args, force)?;
            print_mutation("released", &task, false, args.format)
        }
        AgentCommand::Start(args) => {
            let task = mutations::start(&resolved, &args, force)?;
            print_mutation("started", &task, false, args.format)
        }
        AgentCommand::Note(args) => {
            let task = mutations::append_text(&resolved, &args, AgentAppendField::Notes, force)?;
            print_mutation("noted", &task, false, args.format)
        }
        AgentCommand::Evidence(args) => {
            let task = mutations::append_text(&resolved, &args, AgentAppendField::Evidence, force)?;
            print_mutation("evidence_added", &task, false, args.format)
        }
        AgentCommand::PlanAppend(args) => {
            let task = mutations::append_text(&resolved, &args, AgentAppendField::Plan, force)?;
            print_mutation("plan_appended", &task, false, args.format)
        }
        AgentCommand::Handoff(args) => {
            mutations::handoff(&resolved, &args, force)?;
            let located = queue::operations::load_task_with_location(
                &resolved.queue_path,
                &resolved.done_path,
                &args.task_id,
            )?;
            let document = build_handoff_document(located.location, located.task);
            render::print_handoff(&document, args.format)
        }
        AgentCommand::Complete(args) => {
            let task = mutations::complete(&resolved, &args, force)?;
            print_mutation("completed", &task, true, args.format)
        }
        AgentCommand::Reject(args) => {
            let task = mutations::reject(&resolved, &args, force)?;
            print_mutation("rejected", &task, true, args.format)
        }
        AgentCommand::Validate(args) => {
            let document = read_model::validate_document(&resolved)?;
            render::print_validate(document, args.format)
        }
    }
}

fn print_mutation(
    action: &str,
    task: &Task,
    archived: bool,
    format: AgentOutputFormat,
) -> Result<()> {
    let document = build_mutation_document(action, task, archived);
    render::print_mutation(&document, format)
}
