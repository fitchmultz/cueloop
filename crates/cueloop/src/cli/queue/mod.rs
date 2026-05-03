//! `cueloop queue ...` command group: Clap types and handler facade.
//!
//! Purpose:
//! - `cueloop queue ...` command group: Clap types and handler facade.
//!
//! Responsibilities:
//! - Define clap structures for queue-related subcommands.
//! - Route queue subcommands to their specific handlers.
//! - Re-export argument types used by queue commands.
//!
//! Not handled here:
//! - Queue persistence and locking semantics (see `crate::queue` and `crate::lock`).
//! - Task execution or runner behavior.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Configuration is resolved from the current working directory.
//! - Queue state changes occur within the subcommand handlers.

mod aging;
mod archive;
mod burndown;
mod dashboard;
mod explain;
mod export;
mod graph;
mod history;
mod import;
mod issue;
mod list;
mod next;
mod next_id;
mod prune;
mod repair;
mod schema;
mod search;
mod shared;
mod show;
mod sort;
mod stats;
mod stop;
mod tree;
mod unlock;
mod validate;

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::config;

pub use aging::QueueAgingArgs;
pub use archive::QueueArchiveArgs;
pub use burndown::QueueBurndownArgs;
pub use dashboard::QueueDashboardArgs;
pub use explain::QueueExplainArgs;
pub use export::QueueExportArgs;
pub use graph::QueueGraphArgs;
pub use history::QueueHistoryArgs;
pub use import::QueueImportArgs;
pub use issue::{
    QueueIssueArgs, QueueIssueCommand, QueueIssuePublishArgs, QueueIssuePublishManyArgs,
};
pub use list::QueueListArgs;
pub use next::QueueNextArgs;
pub use next_id::QueueNextIdArgs;
pub use prune::QueuePruneArgs;
pub use repair::RepairArgs;
pub use search::QueueSearchArgs;
pub use shared::{
    QueueExportFormat, QueueImportFormat, QueueListFormat, QueueListSortBy, QueueReportFormat,
    QueueShowFormat, QueueSortBy, QueueSortOrder, StatusArg,
};
pub use show::QueueShowArgs;
pub(crate) use show::show_task;
pub use sort::QueueSortArgs;
pub use stats::QueueStatsArgs;
pub use tree::QueueTreeArgs;
pub use unlock::QueueUnlockArgs;

pub fn handle_queue(cmd: QueueCommand, force: bool) -> Result<()> {
    let resolved = config::resolve_from_cwd()?;
    match cmd {
        QueueCommand::Validate => validate::handle(&resolved),
        QueueCommand::Next(args) => next::handle(&resolved, args),
        QueueCommand::NextId(args) => next_id::handle(&resolved, args),
        QueueCommand::Show(args) => show::handle(&resolved, args),
        QueueCommand::List(args) => list::handle(&resolved, args),
        QueueCommand::Search(args) => search::handle(&resolved, args),
        QueueCommand::Archive(args) => archive::handle(&resolved, force, args),
        QueueCommand::Repair(args) => repair::handle(&resolved, force, args),
        QueueCommand::Unlock(args) => unlock::handle(&resolved, args),
        QueueCommand::Sort(args) => sort::handle(&resolved, force, args),
        QueueCommand::Stats(args) => stats::handle(&resolved, args),
        QueueCommand::History(args) => history::handle(&resolved, args),
        QueueCommand::Burndown(args) => burndown::handle(&resolved, args),
        QueueCommand::Aging(args) => aging::handle(&resolved, args),
        QueueCommand::Schema => schema::handle(),
        QueueCommand::Prune(args) => prune::handle(&resolved, force, args),
        QueueCommand::Graph(args) => graph::handle(&resolved, args),
        QueueCommand::Export(args) => export::handle(&resolved, args),
        QueueCommand::Import(args) => import::handle(&resolved, force, args),
        QueueCommand::Stop => stop::handle(&resolved),
        QueueCommand::Explain(args) => explain::handle(&resolved, args),
        QueueCommand::Tree(args) => tree::handle(&resolved, args),
        QueueCommand::Dashboard(args) => dashboard::handle(&resolved, args),
        QueueCommand::Issue(args) => issue::handle(&resolved, force, args),
    }
}

#[derive(Args)]
#[command(
    about = "Inspect and manage the task queue",
    after_long_help = "Examples:\n  cueloop queue list\n  cueloop queue list --status todo --tag rust\n  cueloop queue show RQ-0008\n  cueloop queue next --with-title\n  cueloop queue next-id\n  cueloop queue archive"
)]
pub struct QueueArgs {
    #[command(subcommand)]
    pub command: QueueCommand,
}

#[derive(Subcommand)]
pub enum QueueCommand {
    /// Inspect whether CueLoop can safely continue from the current queue state.
    #[command(
        after_long_help = "Continuation workflow:\n - This command is read-only.\n - If the queue is valid, CueLoop tells you whether continuation is ready, waiting, or blocked.\n - If recoverable issues are present, CueLoop explains the blocking state and the next repair/undo steps.\n - Warnings are preserved as partial value; they do not force manual queue surgery.\n\nExamples:\n cueloop queue validate\n cueloop --verbose queue validate\n cueloop queue repair --dry-run\n cueloop queue repair\n cueloop undo --dry-run"
    )]
    Validate,

    /// Prune tasks from the done archive based on age, status, or keep-last rules.
    #[command(
        after_long_help = "Prune removes old tasks from .cueloop/done.jsonc while preserving recent history.\n\nSafety:\n --keep-last always protects the N most recently completed tasks (by completed_at).\n If no filters are provided, all tasks are pruned except those protected by --keep-last.\n Missing or invalid completed_at timestamps are treated as oldest for keep-last ordering\n but do NOT match the age filter (safety-first).\n\nExamples:\n cueloop queue prune --dry-run --age 30 --status rejected\n cueloop queue prune --keep-last 100\n cueloop queue prune --age 90\n cueloop queue prune --age 30 --status done --keep-last 50"
    )]
    Prune(QueuePruneArgs),

    /// Print the next todo task (ID by default).
    #[command(
        after_long_help = "Examples:\n cueloop queue next\n cueloop queue next --with-title\n cueloop queue next --with-eta\n cueloop queue next --with-title --with-eta\n cueloop queue next --explain\n cueloop queue next --explain --with-title"
    )]
    Next(QueueNextArgs),

    /// Preview the next available task ID (across queue + done archive).
    #[command(
        after_long_help = "Preview workflow:\n - This command is read-only and does not reserve IDs.\n - For agents or scripts that create tasks, prefer `cueloop task insert` so IDs are assigned under the queue lock.\n - Keep `next-id` for manual recovery or one-off queue surgery only.\n\nExamples:\n cueloop queue next-id\n cueloop queue next-id --count 5\n cueloop queue next-id -n 3\n cueloop task insert --input /tmp/task-insert.json\n cueloop --verbose queue next-id"
    )]
    NextId(QueueNextIdArgs),

    /// Show a task by ID.
    Show(QueueShowArgs),

    /// List tasks in queue order.
    List(QueueListArgs),

    /// Search tasks by content (title, evidence, plan, notes, request, tags, scope, custom fields).
    #[command(
        after_long_help = "Examples:\n cueloop queue search \"authentication\"\n cueloop queue search \"RQ-\\d{4}\" --regex\n cueloop queue search \"TODO\" --match-case\n cueloop queue search \"fix\" --status todo --tag rust\n cueloop queue search \"refactor\" --scope crates/cueloop --tag rust\n cueloop queue search \"auth bug\" --fuzzy\n cueloop queue search \"fuzzy search\" --fuzzy --match-case"
    )]
    Search(QueueSearchArgs),

    /// Move completed tasks from queue.jsonc to done.jsonc.
    #[command(
        after_long_help = "Examples:\n  cueloop queue archive\n  cueloop queue archive --dry-run"
    )]
    Archive(QueueArchiveArgs),

    /// Normalize recoverable queue issues so CueLoop can continue safely.
    #[command(
        after_long_help = "Continuation workflow:\n - Use --dry-run first to preview recoverable fixes.\n - Applying the repair creates an undo checkpoint before queue files are rewritten.\n - This command is for normal continuation, not manual emergency surgery.\n\nExamples:\n cueloop queue repair --dry-run\n cueloop queue repair\n cueloop undo --dry-run"
    )]
    Repair(RepairArgs),

    /// Safely remove the queue lock file with process detection.
    #[command(after_long_help = "Safely remove the queue lock directory.\n\n\
Safety:\n  - Checks if the lock holder process is still running\n  - Blocks if process is active (override with --force)\n  - Requires confirmation in interactive mode (bypass with --yes)\n\n\
Examples:\n  cueloop queue unlock --dry-run\n  cueloop queue unlock --yes\n  cueloop queue unlock --force --yes")]
    Unlock(QueueUnlockArgs),

    /// Sort tasks by priority (reorders the queue file).
    #[command(
        after_long_help = "Examples:\n cueloop queue sort\n cueloop queue sort --order descending\n cueloop queue sort --order ascending"
    )]
    Sort(QueueSortArgs),

    /// Show task statistics (completion rate, avg duration, tag breakdown).
    #[command(
        after_long_help = "Examples:\n cueloop queue stats\n cueloop queue stats --tag rust --tag cli\n cueloop queue stats --format json"
    )]
    Stats(QueueStatsArgs),

    /// Show task history timeline (creation/completion events by day).
    #[command(
        after_long_help = "Examples:\n cueloop queue history\n cueloop queue history --days 14"
    )]
    History(QueueHistoryArgs),

    /// Show burndown chart of remaining tasks over time.
    #[command(
        after_long_help = "Examples:\n cueloop queue burndown\n cueloop queue burndown --days 30"
    )]
    Burndown(QueueBurndownArgs),

    /// Show task aging buckets to identify stale work.
    #[command(
        after_long_help = "Examples:\n  cueloop queue aging\n  cueloop queue aging --format json\n  cueloop queue aging --status todo --status doing"
    )]
    Aging(QueueAgingArgs),

    /// Print the JSON schema for the queue file.
    #[command(after_long_help = "Example:\n cueloop queue schema")]
    Schema,

    /// Visualize task dependencies as a graph.
    #[command(
        after_long_help = "Examples:\n cueloop queue graph\n cueloop queue graph --task RQ-0001\n cueloop queue graph --format dot\n cueloop queue graph --critical\n cueloop queue graph --reverse --task RQ-0001"
    )]
    Graph(QueueGraphArgs),

    /// Export task data to CSV, TSV, JSON, Markdown, or GitHub issue format.
    #[command(
        after_long_help = "Examples:\n cueloop queue export\n cueloop queue export --format csv --output tasks.csv\n cueloop queue export --format json --status done\n cueloop queue export --format tsv --tag rust --tag cli\n cueloop queue export --format md --status todo\n cueloop queue export --format gh --id-pattern RQ-0001\n cueloop queue export --include-archive --format csv\n cueloop queue export --format csv --created-after 2026-01-01"
    )]
    Export(QueueExportArgs),

    /// Import tasks from CSV, TSV, or JSON.
    #[command(
        after_long_help = "Examples:\n cueloop queue import --format json < tasks.json\n cueloop queue import --format csv tasks.csv\n cueloop queue import --format tsv --on-duplicate rename tasks.tsv\n cueloop queue import --format json --dry-run < tasks.json"
    )]
    Import(QueueImportArgs),

    /// Request graceful stop of a running loop after current task completes.
    #[command(
        after_long_help = "Examples:\n cueloop queue stop\n\nNotes:\n - This creates a stop signal file that the run loop checks between tasks.\n - Sequential mode: exits between tasks (current task completes, then exits).\n - Parallel mode: stops scheduling new tasks; waits for in-flight tasks to complete.\n - The stop signal is automatically cleared when the loop honors the request.\n - To force immediate termination, use Ctrl+C in the running loop."
    )]
    Stop,

    /// Explain why tasks are (not) runnable.
    #[command(
        after_long_help = "Examples:\n  cueloop queue explain\n  cueloop queue explain --format json\n  cueloop queue explain --include-draft\n  cueloop queue explain --format json --include-draft"
    )]
    Explain(QueueExplainArgs),

    /// Render a parent/child hierarchy tree (based on parent_id).
    #[command(
        after_long_help = "Examples:\n  cueloop queue tree\n  cueloop queue tree --include-done\n  cueloop queue tree --root RQ-0001\n  cueloop queue tree --max-depth 25"
    )]
    Tree(QueueTreeArgs),

    /// Aggregated dashboard for analytics UI (combines stats, burndown, history, productivity).
    #[command(
        after_long_help = "Examples:\n  cueloop queue dashboard\n  cueloop queue dashboard --days 30\n  cueloop queue dashboard --days 7\n\n\
The dashboard command returns all analytics data in a single JSON payload for GUI clients.\n\
Each section includes a 'status' field ('ok' or 'unavailable') for graceful partial failure handling."
    )]
    Dashboard(QueueDashboardArgs),

    /// Publish tasks to GitHub Issues.
    #[command(
        after_long_help = "Examples:\n  cueloop queue issue publish RQ-0655\n  cueloop queue issue publish RQ-0655 --dry-run\n  cueloop queue issue publish RQ-0655 --label bug --assignee @me\n  cueloop queue issue publish RQ-0655 --repo owner/repo\n  cueloop queue issue publish-many --status todo --tag bug --dry-run\n  cueloop queue issue publish-many --status todo --execute --force"
    )]
    Issue(QueueIssueArgs),
}

#[cfg(test)]
mod tests;
