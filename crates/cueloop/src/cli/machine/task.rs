//! Task-oriented machine command router.
//!
//! Purpose:
//! - Route `cueloop machine task ...` commands to focused task-machine modules.
//!
//! Responsibilities:
//! - Resolve repository configuration once for machine task commands.
//! - Dispatch each task subcommand to the module that owns its behavior.
//! - Re-export stable machine task document builders consumed by app-facing callers.
//!
//! Not handled here:
//! - Queue/task write orchestration.
//! - Machine task JSON envelope assembly.
//! - Status parsing or decompose policy parsing.
//!
//! Usage:
//! - Called from the machine CLI facade through `handle_task`.
//!
//! Invariants/assumptions:
//! - This file stays a thin router/facade.
//! - Public crate-level re-exports from `cli::machine` remain unchanged.

mod build;
mod continuation;
mod create_insert;
mod decompose;
mod documents;
mod followups;
mod lifecycle;
mod mutation;
mod parsing;
#[cfg(test)]
mod tests;

use anyhow::Result;

use crate::cli::machine::args::{MachineTaskArgs, MachineTaskCommand};
use crate::cli::machine::io::print_json;
use crate::config;
use crate::contracts::TaskStatus;

pub(crate) use documents::{build_decompose_document, build_task_mutation_document};
#[cfg(test)]
use parsing::{parse_child_policy, parse_task_status};

pub(super) fn handle_task(args: MachineTaskArgs, force: bool) -> Result<()> {
    let resolved = config::resolve_from_cwd()?;
    match args.command {
        MachineTaskCommand::Build(args) => {
            print_json(&build::handle_build(&resolved, *args, force)?)
        }
        MachineTaskCommand::Create(args) => {
            print_json(&create_insert::handle_create(&resolved, args, force)?)
        }
        MachineTaskCommand::Insert(args) => {
            print_json(&create_insert::handle_insert(&resolved, args, force)?)
        }
        MachineTaskCommand::Mutate(args) => {
            print_json(&mutation::handle_mutate(&resolved, args, force)?)
        }
        MachineTaskCommand::Show(args) => {
            print_json(&lifecycle::show_task(&resolved, &args.task_id)?)
        }
        MachineTaskCommand::Start(args) => print_json(&lifecycle::update_task_lifecycle(
            &resolved,
            &args.task_id,
            TaskStatus::Doing,
            &args.notes,
            &args.evidence,
            args.dry_run,
            force,
        )?),
        MachineTaskCommand::Done(args) => print_json(&lifecycle::update_task_lifecycle(
            &resolved,
            &args.task_id,
            TaskStatus::Done,
            &args.notes,
            &args.evidence,
            args.dry_run,
            force,
        )?),
        MachineTaskCommand::Reject(args) => print_json(&lifecycle::update_task_lifecycle(
            &resolved,
            &args.task_id,
            TaskStatus::Rejected,
            &args.notes,
            &args.evidence,
            args.dry_run,
            force,
        )?),
        MachineTaskCommand::Status(args) => print_json(&lifecycle::update_task_lifecycle(
            &resolved,
            &args.task_id,
            parsing::parse_task_status(&args.status)?,
            &args.notes,
            &args.evidence,
            args.dry_run,
            force,
        )?),
        MachineTaskCommand::Followups(args) => {
            print_json(&followups::handle_followups(&resolved, args, force)?)
        }
        MachineTaskCommand::Decompose(args) => {
            print_json(&decompose::handle_decompose(&resolved, *args, force)?)
        }
    }
}
