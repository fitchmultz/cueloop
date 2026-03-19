//! Purpose: Load built-in and custom templates, then optionally apply template
//! context substitution.
//!
//! Responsibilities:
//! - Resolve template lookup precedence between custom and built-in templates.
//! - Parse template JSON into tasks.
//! - Validate template variables and apply context-aware substitution.
//!
//! Scope:
//! - Loading and substitution only; listing/query helpers live elsewhere.
//!
//! Usage:
//! - Called by CLI task-creation surfaces that need template resolution.
//!
//! Invariants/Assumptions:
//! - Custom templates under `.ralph/templates/{name}.json` override built-ins.
//! - Strict mode fails on unknown template variables.
//! - Non-strict mode preserves unknown placeholders and returns warnings.

use std::path::Path;

use anyhow::{Result, bail};

use crate::contracts::Task;
use crate::template::builtin::get_builtin_template;
use crate::template::variables::{
    TemplateContext, detect_context_with_warnings, substitute_variables_in_task,
    validate_task_template,
};

use super::types::{LoadedTemplate, TemplateError, TemplateSource};

/// Load a template by name.
///
/// Checks `.ralph/templates/{name}.json` first, then falls back to built-in templates.
pub fn load_template(name: &str, project_root: &Path) -> Result<(Task, TemplateSource)> {
    let custom_path = project_root
        .join(".ralph/templates")
        .join(format!("{}.json", name));
    if custom_path.exists() {
        let content = std::fs::read_to_string(&custom_path)
            .map_err(|e| TemplateError::ReadError(e.to_string()))?;
        let task: Task = serde_json::from_str(&content)
            .map_err(|e| TemplateError::InvalidJson(e.to_string()))?;

        let validation = validate_task_template(&task);
        if validation.has_unknown_variables() {
            let unknowns = validation.unknown_variable_names();
            log::warn!(
                "Template '{}' contains unknown variables: {}",
                name,
                unknowns.join(", ")
            );
        }

        return Ok((task, TemplateSource::Custom(custom_path)));
    }

    if let Some(template_json) = get_builtin_template(name) {
        let task: Task = serde_json::from_str(template_json)
            .map_err(|e| TemplateError::InvalidJson(e.to_string()))?;
        return Ok((task, TemplateSource::Builtin(name.to_string())));
    }

    Err(TemplateError::NotFound(name.to_string()).into())
}

/// Load a template by name with variable substitution.
///
/// Checks `.ralph/templates/{name}.json` first, then falls back to built-in templates.
/// Substitutes template variables (`{{target}}`, `{{module}}`, `{{file}}`, `{{branch}}`) with
/// context-aware values.
///
/// If `strict` is true and unknown variables are present, returns an error.
pub fn load_template_with_context(
    name: &str,
    project_root: &Path,
    target: Option<&str>,
    strict: bool,
) -> Result<LoadedTemplate> {
    let (mut task, source) = load_template(name, project_root)?;
    let validation = validate_task_template(&task);

    if strict && validation.has_unknown_variables() {
        let unknowns = validation.unknown_variable_names();
        bail!(TemplateError::ValidationError(format!(
            "Template '{}' contains unknown variables: {}",
            name,
            unknowns.join(", ")
        )));
    }

    let (context, mut warnings) =
        detect_context_with_warnings(target, project_root, validation.uses_branch);
    warnings.extend(validation.warnings);
    substitute_variables_in_task(&mut task, &context);

    Ok(LoadedTemplate {
        task,
        source,
        warnings,
    })
}

/// Load a template by name with variable substitution (legacy, non-strict).
///
/// This is a convenience function for backward compatibility.
/// Use `load_template_with_context` for full control.
pub fn load_template_with_context_legacy(
    name: &str,
    project_root: &Path,
    target: Option<&str>,
) -> Result<(Task, TemplateSource)> {
    let loaded = load_template_with_context(name, project_root, target, false)?;
    Ok((loaded.task, loaded.source))
}

/// Get the template context for inspection.
pub fn get_template_context(target: Option<&str>, project_root: &Path) -> TemplateContext {
    let (context, _) = detect_context_with_warnings(target, project_root, true);
    context
}
