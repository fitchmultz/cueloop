//! Cleanup command CLI arguments.
//!
//! Purpose:
//! - Cleanup command CLI arguments.
//!
//! Responsibilities:
//! - Define CLI arguments for the `cueloop cleanup` command.
//!
//! Not handled here:
//! - Actual cleanup logic (see `crate::commands::cleanup`).
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Arguments are validated by Clap before being passed to the handler.

use clap::Args;

/// Arguments for `cueloop cleanup`.
#[derive(Args, Debug)]
#[command(after_long_help = "Examples:\n\
  cueloop cleanup              # Clean temp files older than 7 days\n\
  cueloop cleanup --force      # Clean all cueloop temp files\n\
  cueloop cleanup --dry-run    # Show what would be deleted without deleting")]
pub struct CleanupArgs {
    /// Force cleanup of all cueloop temp files regardless of age.
    #[arg(long)]
    pub force: bool,

    /// Dry run - show what would be deleted without deleting.
    #[arg(long)]
    pub dry_run: bool,
}

/// Handle the cleanup command.
pub fn handle_cleanup(args: CleanupArgs) -> anyhow::Result<()> {
    crate::commands::cleanup::run(&args)
}
