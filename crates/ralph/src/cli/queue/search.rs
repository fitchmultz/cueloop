//! Queue search subcommand.

use anyhow::{bail, Result};
use clap::Args;

use crate::cli::{load_and_validate_queues, resolve_list_limit};
use crate::config::Resolved;
use crate::contracts::{Task, TaskStatus};
use crate::{outpututil, queue};

use super::{QueueListFormat, StatusArg};

/// Arguments for `ralph queue search`.
#[derive(Args)]
/// Search tasks by content (title, evidence, plan, notes, request, tags, scope, custom fields).
#[command(
    after_long_help = "Examples:\n  ralph queue search \"authentication\"\n  ralph queue search \"RQ-\\d{4}\" --regex\n  ralph queue search \"TODO\" --match-case\n  ralph queue search \"fix\" --status todo --tag rust\n  ralph queue search \"refactor\" --scope crates/ralph --tag rust"
)]
pub struct QueueSearchArgs {
    /// Search query (substring or regex pattern).
    #[arg(value_name = "QUERY")]
    pub query: String,

    /// Interpret query as a regular expression.
    #[arg(long)]
    pub regex: bool,

    /// Case-sensitive search (default: case-insensitive).
    #[arg(long)]
    pub match_case: bool,

    /// Filter by status (repeatable).
    #[arg(long, value_enum)]
    pub status: Vec<StatusArg>,

    /// Filter by tag (repeatable, case-insensitive).
    #[arg(long)]
    pub tag: Vec<String>,

    /// Filter by scope token (repeatable, case-insensitive; substring match).
    #[arg(long)]
    pub scope: Vec<String>,

    /// Include tasks from .ralph/done.json in search.
    #[arg(long)]
    pub include_done: bool,

    /// Only search tasks in .ralph/done.json (ignores active queue).
    #[arg(long)]
    pub only_done: bool,

    /// Output format.
    #[arg(long, value_enum, default_value_t = QueueListFormat::Compact)]
    pub format: QueueListFormat,

    /// Maximum results to show (0 = no limit).
    #[arg(long, default_value_t = 50)]
    pub limit: u32,

    /// Show all results (ignores --limit).
    #[arg(long)]
    pub all: bool,
}

pub(crate) fn handle(resolved: &Resolved, args: QueueSearchArgs) -> Result<()> {
    if args.include_done && args.only_done {
        bail!("Conflicting flags: --include-done and --only-done are mutually exclusive. Choose either to include done tasks or to only search done tasks.");
    }

    let (queue_file, done_file) =
        load_and_validate_queues(resolved, args.include_done || args.only_done)?;
    let done_ref = done_file
        .as_ref()
        .filter(|d| !d.tasks.is_empty() || resolved.done_path.exists());

    let statuses: Vec<TaskStatus> = args.status.into_iter().map(|s| s.into()).collect();

    // Pre-filter by status/tag/scope using filter_tasks
    let mut prefiltered: Vec<&Task> = Vec::new();
    if !args.only_done {
        prefiltered.extend(queue::filter_tasks(
            &queue_file,
            &statuses,
            &args.tag,
            &args.scope,
            None,
        ));
    }
    if args.include_done || args.only_done {
        if let Some(done_ref) = done_ref {
            prefiltered.extend(queue::filter_tasks(
                done_ref,
                &statuses,
                &args.tag,
                &args.scope,
                None,
            ));
        }
    }

    // Apply content search
    let results = queue::search_tasks(
        prefiltered.into_iter(),
        &args.query,
        args.regex,
        args.match_case,
    )?;

    let limit = resolve_list_limit(args.limit, args.all);
    let max = limit.unwrap_or(usize::MAX);
    for task in results.into_iter().take(max) {
        match args.format {
            QueueListFormat::Compact => println!("{}", outpututil::format_task_compact(task)),
            QueueListFormat::Long => println!("{}", outpututil::format_task_detailed(task)),
        }
    }

    Ok(())
}
