//! Initialization command facade.
//!
//! Purpose:
//! - Expose the Ralph initialization workflow from a thin command module.
//!
//! Responsibilities:
//! - Re-export initialization options, result types, and README helpers.
//! - Keep workflow orchestration, migration checks, and tests in focused companions.
//!
//! Scope:
//! - This module coordinates `ralph init` implementation only.
//! - CLI parsing remains in `crate::cli::init`.
//!
//! Usage:
//! - Called by CLI handlers, tutorial flows, and integration tests.
//!
//! Invariants/assumptions:
//! - Interactive and non-interactive initialization share the same underlying workflow.
//! - README and writer helpers remain delegated to dedicated submodules.

pub mod gitignore;
pub mod readme;
pub mod wizard;
pub mod writers;

mod migration_check;
#[cfg(test)]
mod tests;
mod types;
mod workflow;

pub use crate::constants::versions::README_VERSION;
pub use readme::{
    ReadmeCheckResult, ReadmeVersionError, check_readme_current, check_readme_current_from_root,
    extract_readme_version,
};
pub use types::{FileInitStatus, InitOptions, InitReport};
pub use wizard::{WizardAnswers, print_completion_message, run_wizard};
pub use workflow::run_init;
pub use writers::{write_config, write_done, write_queue};
