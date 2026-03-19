//! Purpose: Enumerate available templates and answer simple existence queries.
//!
//! Responsibilities:
//! - List custom and built-in templates with custom override precedence.
//! - Derive display descriptions for template listings.
//! - Report whether a template name resolves to any source.
//!
//! Scope:
//! - Listing/query behavior only; parsing and substitution live elsewhere.
//!
//! Usage:
//! - Used by CLI surfaces that present template choices or validate a template
//!   name before loading.
//!
//! Invariants/Assumptions:
//! - Custom templates override built-ins with the same name.
//! - Listing order remains stable via name sorting.
//! - Only `.json` files under `.ralph/templates/` are considered custom
//!   templates.

use std::collections::HashSet;
use std::path::Path;

use crate::contracts::Task;
use crate::template::builtin::{
    get_builtin_template, get_template_description, list_builtin_templates,
};

use super::types::{TemplateInfo, TemplateSource};

/// List all available templates (built-in + custom).
///
/// Custom templates override built-ins with the same name.
pub fn list_templates(project_root: &Path) -> Vec<TemplateInfo> {
    let mut templates = Vec::new();
    let mut seen_names = HashSet::new();

    let custom_dir = project_root.join(".ralph/templates");
    if let Ok(entries) = std::fs::read_dir(&custom_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json")
                && let Some(name) = path.file_stem()
            {
                let name = name.to_string_lossy().to_string();
                seen_names.insert(name.clone());

                let description = if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(task) = serde_json::from_str::<Task>(&content) {
                        task.plan
                            .first()
                            .cloned()
                            .unwrap_or_else(|| "Custom template".to_string())
                    } else {
                        "Custom template".to_string()
                    }
                } else {
                    "Custom template".to_string()
                };

                templates.push(TemplateInfo {
                    name,
                    source: TemplateSource::Custom(path),
                    description,
                });
            }
        }
    }

    for name in list_builtin_templates() {
        if !seen_names.contains(name) {
            templates.push(TemplateInfo {
                name: name.to_string(),
                source: TemplateSource::Builtin(name.to_string()),
                description: get_template_description(name).to_string(),
            });
        }
    }

    templates.sort_by(|a, b| a.name.cmp(&b.name));
    templates
}

/// Check if a template exists (either custom or built-in).
pub fn template_exists(name: &str, project_root: &Path) -> bool {
    let custom_path = project_root
        .join(".ralph/templates")
        .join(format!("{}.json", name));
    custom_path.exists() || get_builtin_template(name).is_some()
}
