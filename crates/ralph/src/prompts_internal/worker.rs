//! Worker prompt loading and rendering.

use super::util::{
    ensure_no_unresolved_placeholders, load_prompt_with_fallback, project_type_guidance,
};
use crate::contracts::{Config, ProjectType};
use anyhow::Result;

const WORKER_PROMPT_REL_PATH: &str = ".ralph/prompts/worker.md";

const DEFAULT_WORKER_PROMPT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/prompts/worker.md"
));

pub fn load_worker_prompt(repo_root: &std::path::Path) -> Result<String> {
    load_prompt_with_fallback(
        repo_root,
        WORKER_PROMPT_REL_PATH,
        DEFAULT_WORKER_PROMPT,
        "worker",
    )
}

pub fn render_worker_prompt(
    template: &str,
    task_id: &str,
    project_type: ProjectType,
    config: &Config,
) -> Result<String> {
    let id = task_id.trim();
    if id.is_empty() {
        anyhow::bail!("Missing task id: worker prompt requires a non-empty task id.");
    }

    let expanded = super::expand_variables(template, config)?;
    let guidance = project_type_guidance(project_type);
    let mut rendered = if expanded.contains("{{PROJECT_TYPE_GUIDANCE}}") {
        expanded.replace("{{PROJECT_TYPE_GUIDANCE}}", guidance)
    } else {
        format!("{}\n{}", expanded, guidance)
    };
    rendered = rendered.replace("{{INTERACTIVE_INSTRUCTIONS}}", "");
    rendered = rendered.replace("{{TASK_ID}}", id);
    ensure_no_unresolved_placeholders(&rendered, "worker")?;
    Ok(rendered)
}
