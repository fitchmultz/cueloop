//! Purpose: Define shared data types for template loading, listing, and error
//! reporting.
//!
//! Responsibilities:
//! - Represent template sources and listing metadata.
//! - Represent loaded templates plus validation/context warnings.
//! - Define structured template-operation errors.
//!
//! Scope:
//! - Data modeling only; no filesystem access, parsing, or substitution.
//!
//! Usage:
//! - Used by the `load` and `list` companions and re-exported through
//!   `template::loader`.
//!
//! Invariants/Assumptions:
//! - Error messages remain user-facing and actionable.
//! - `TemplateSource::Custom` always stores the resolved template path.

use std::path::PathBuf;

use crate::contracts::Task;
use crate::template::variables::TemplateWarning;

/// Source of a loaded template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateSource {
    /// Custom template from `.ralph/templates/`.
    Custom(PathBuf),
    /// Built-in embedded template (stores the name, not the content).
    Builtin(String),
}

/// Metadata for a template (used for listing).
#[derive(Debug, Clone)]
pub struct TemplateInfo {
    pub name: String,
    pub source: TemplateSource,
    pub description: String,
}

/// Result of loading a template with context.
#[derive(Debug, Clone)]
pub struct LoadedTemplate {
    /// The task with variables substituted.
    pub task: Task,
    /// The source of the template.
    pub source: TemplateSource,
    /// Warnings collected during validation and context detection.
    pub warnings: Vec<TemplateWarning>,
}

/// Error type for template operations.
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("Template not found: {0}")]
    NotFound(String),
    #[error("Failed to read template file: {0}")]
    ReadError(String),
    #[error("Invalid template JSON: {0}")]
    InvalidJson(String),
    #[error("Template validation failed: {0}")]
    ValidationError(String),
}
