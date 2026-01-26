//! Queue stats subcommand.

use anyhow::Result;
use clap::Args;

use crate::cli::load_and_validate_queues;
use crate::config::Resolved;
use crate::reports;

use super::QueueReportFormat;

/// Arguments for `ralph queue stats`.
#[derive(Args)]
#[command(
    after_long_help = "Examples:\n  ralph queue stats\n  ralph queue stats --tag rust --tag cli\n  ralph queue stats --format json"
)]
pub struct QueueStatsArgs {
    /// Filter by tag (repeatable, case-insensitive).
    #[arg(long)]
    pub tag: Vec<String>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = QueueReportFormat::Text)]
    pub format: QueueReportFormat,
}

pub(crate) fn handle(resolved: &Resolved, args: QueueStatsArgs) -> Result<()> {
    let (queue_file, done_file) = load_and_validate_queues(resolved, true)?;
    let done_ref = done_file
        .as_ref()
        .filter(|d| !d.tasks.is_empty() || resolved.done_path.exists());
    reports::print_stats(&queue_file, done_ref, &args.tag, args.format.into())?;
    Ok(())
}
