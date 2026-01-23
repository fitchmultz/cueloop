//! Scan prompt loading and rendering.

use super::util::{
    ensure_no_unresolved_placeholders, load_prompt_with_fallback, project_type_guidance,
};
use crate::contracts::{Config, ProjectType};
use anyhow::{bail, Result};

const SCAN_PROMPT_REL_PATH: &str = ".ralph/prompts/scan.md";

const DEFAULT_SCAN_PROMPT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/prompts/scan.md"
));

pub fn load_scan_prompt(repo_root: &std::path::Path) -> Result<String> {
    load_prompt_with_fallback(repo_root, SCAN_PROMPT_REL_PATH, DEFAULT_SCAN_PROMPT, "scan")
}

pub fn render_scan_prompt(
    template: &str,
    user_focus: &str,
    project_type: ProjectType,
    config: &Config,
) -> Result<String> {
    if !template.contains("{{USER_FOCUS}}") {
        bail!("Template error: scan prompt template is missing the required '{{USER_FOCUS}}' placeholder. Ensure the template in .ralph/prompts/scan.md includes this placeholder.");
    }
    let focus = user_focus.trim();
    let focus = if focus.is_empty() { "(none)" } else { focus };

    let expanded = super::expand_variables(template, config)?;
    let guidance = project_type_guidance(project_type);
    let rendered = if expanded.contains("{{PROJECT_TYPE_GUIDANCE}}") {
        expanded.replace("{{PROJECT_TYPE_GUIDANCE}}", guidance)
    } else {
        format!("{}\n{}", expanded, guidance)
    };
    let rendered = rendered.replace("{{USER_FOCUS}}", focus);
    ensure_no_unresolved_placeholders(&rendered, "scan")?;
    Ok(rendered)
}
