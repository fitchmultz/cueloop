//! Task builder prompt loading and rendering.

use super::util::{
    ensure_no_unresolved_placeholders, escape_placeholder_like_text, load_prompt_with_fallback,
    project_type_guidance,
};
use crate::contracts::{Config, ProjectType};
use anyhow::{bail, Result};

const TASK_BUILDER_PROMPT_REL_PATH: &str = ".ralph/prompts/task_builder.md";

const DEFAULT_TASK_BUILDER_PROMPT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/prompts/task_builder.md"
));

pub fn load_task_builder_prompt(repo_root: &std::path::Path) -> Result<String> {
    load_prompt_with_fallback(
        repo_root,
        TASK_BUILDER_PROMPT_REL_PATH,
        DEFAULT_TASK_BUILDER_PROMPT,
        "task builder",
    )
}

pub fn render_task_builder_prompt(
    template: &str,
    user_request: &str,
    hint_tags: &str,
    hint_scope: &str,
    project_type: ProjectType,
    config: &Config,
) -> Result<String> {
    if !template.contains("{{USER_REQUEST}}") {
        bail!("Template error: task builder prompt template is missing the required '{{USER_REQUEST}}' placeholder. Ensure the template in .ralph/prompts/task_builder.md includes this placeholder.");
    }
    if !template.contains("{{HINT_TAGS}}") {
        bail!("Template error: task builder prompt template is missing the required '{{HINT_TAGS}}' placeholder. Ensure the template includes this placeholder.");
    }
    if !template.contains("{{HINT_SCOPE}}") {
        bail!("Template error: task builder prompt template is missing the required '{{HINT_SCOPE}}' placeholder. Ensure the template includes this placeholder.");
    }

    let request = user_request.trim();
    if request.is_empty() {
        bail!("Missing request: user request must be non-empty. Provide a descriptive request for the task builder.");
    }

    let expanded = super::expand_variables(template, config)?;
    let guidance = project_type_guidance(project_type);
    let mut rendered = if expanded.contains("{{PROJECT_TYPE_GUIDANCE}}") {
        expanded.replace("{{PROJECT_TYPE_GUIDANCE}}", guidance)
    } else {
        format!("{}\n{}", expanded, guidance)
    };
    rendered = rendered.replace("{{USER_REQUEST}}", request);
    rendered = rendered.replace("{{HINT_TAGS}}", hint_tags.trim());
    rendered = rendered.replace("{{HINT_SCOPE}}", hint_scope.trim());
    rendered = rendered.replace("{{INTERACTIVE_INSTRUCTIONS}}", "");
    let safe_request = escape_placeholder_like_text(request);
    let safe_hint_tags = escape_placeholder_like_text(hint_tags.trim());
    let safe_hint_scope = escape_placeholder_like_text(hint_scope.trim());
    let mut rendered_for_validation = if expanded.contains("{{PROJECT_TYPE_GUIDANCE}}") {
        expanded.replace("{{PROJECT_TYPE_GUIDANCE}}", guidance)
    } else {
        format!("{}\n{}", expanded, guidance)
    };
    rendered_for_validation =
        rendered_for_validation.replace("{{USER_REQUEST}}", safe_request.trim());
    rendered_for_validation =
        rendered_for_validation.replace("{{HINT_TAGS}}", safe_hint_tags.trim());
    rendered_for_validation =
        rendered_for_validation.replace("{{HINT_SCOPE}}", safe_hint_scope.trim());
    rendered_for_validation = rendered_for_validation.replace("{{INTERACTIVE_INSTRUCTIONS}}", "");
    ensure_no_unresolved_placeholders(&rendered_for_validation, "task builder")?;
    Ok(rendered)
}
