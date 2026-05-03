//! Shared data models for the AGENTS.md wizard.
//!
//! Purpose:
//! - Shared data models for the AGENTS.md wizard.
//!
//! Responsibilities:
//! - Define init-wizard configuration hints and result payloads.
//! - Define the update-wizard return shape consumed by the workflow layer.
//!
//! Not handled here:
//! - Prompting.
//! - Wizard step orchestration.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Suggested command hints should stay aligned with `render.rs` detection defaults.
//! - Result types stay aligned with `workflow.rs` and `render.rs` consumers.

use crate::cli::context::ProjectTypeHint;
use std::path::PathBuf;

/// Configuration hints collected during init wizard.
#[derive(Debug, Clone)]
pub(crate) struct ConfigHints {
    /// Project description to replace placeholder.
    pub(crate) project_description: Option<String>,
    /// CI command suggestion or user-provided override.
    pub(crate) ci_command: String,
    /// Build command suggestion or user-provided override.
    pub(crate) build_command: String,
    /// Test command suggestion or user-provided override.
    pub(crate) test_command: String,
    /// Lint command suggestion or user-provided override.
    pub(crate) lint_command: String,
    /// Format command.
    pub(crate) format_command: String,
    /// Whether the user explicitly customized command fields in interactive mode.
    pub(crate) customized_commands: bool,
}

impl Default for ConfigHints {
    fn default() -> Self {
        Self {
            project_description: None,
            ci_command: "TODO: record this repo's CI command.".to_string(),
            build_command: "TODO: record this repo's build command.".to_string(),
            test_command: "TODO: record this repo's test command.".to_string(),
            lint_command: "TODO: record this repo's lint command.".to_string(),
            format_command: "TODO: record this repo's format command.".to_string(),
            customized_commands: false,
        }
    }
}

/// Result of the init wizard.
#[derive(Debug, Clone)]
pub(crate) struct InitWizardResult {
    /// Selected project type.
    pub(crate) project_type: ProjectTypeHint,
    /// Optional output path override.
    pub(crate) output_path: Option<PathBuf>,
    /// Config hints for customizing the generated content.
    pub(crate) config_hints: ConfigHints,
    /// Whether to confirm before writing.
    pub(crate) confirm_write: bool,
}

/// Result of the update wizard: section name -> new content.
pub(crate) type UpdateWizardResult = Vec<(String, String)>;
