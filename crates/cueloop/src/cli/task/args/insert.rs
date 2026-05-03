//! CLI arguments for atomic task insertion.
//!
//! Purpose:
//! - Define args for `cueloop task insert`.
//!
//! Responsibilities:
//! - Expose JSON file/stdin input, dry-run, and text/JSON output selection.
//!
//! Not handled here:
//! - Queue mutation, locking, or JSON parsing.

use clap::Args;
use clap::ValueEnum;

#[derive(ValueEnum, Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskInsertFormatArg {
    Text,
    Json,
}

#[derive(Args, Clone)]
pub struct TaskInsertArgs {
    /// Read the insert request from a JSON file.
    ///
    /// When omitted, CueLoop reads the JSON request from stdin.
    #[arg(long, value_name = "PATH")]
    pub input: Option<String>,

    /// Preview ID allocation and validation without saving queue changes.
    #[arg(long)]
    pub dry_run: bool,

    /// Output format for the insertion result.
    #[arg(long, value_enum, default_value_t = TaskInsertFormatArg::Text)]
    pub format: TaskInsertFormatArg,
}
