//! Review prompt loading and rendering (code review, completion checklist, phase2 handoff).

use super::util::{
    ensure_no_unresolved_placeholders, load_prompt_with_fallback, project_type_guidance,
};
use crate::contracts::{Config, ProjectType};
use anyhow::{bail, Result};

const COMPLETION_CHECKLIST_REL_PATH: &str = ".ralph/prompts/completion_checklist.md";
const CODE_REVIEW_PROMPT_REL_PATH: &str = ".ralph/prompts/code_review.md";
const PHASE2_HANDOFF_CHECKLIST_REL_PATH: &str = ".ralph/prompts/phase2_handoff_checklist.md";

const DEFAULT_COMPLETION_CHECKLIST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/prompts/completion_checklist.md"
));

const DEFAULT_CODE_REVIEW_PROMPT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/prompts/code_review.md"
));

const DEFAULT_PHASE2_HANDOFF_CHECKLIST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/prompts/phase2_handoff_checklist.md"
));

pub fn load_completion_checklist(repo_root: &std::path::Path) -> Result<String> {
    load_prompt_with_fallback(
        repo_root,
        COMPLETION_CHECKLIST_REL_PATH,
        DEFAULT_COMPLETION_CHECKLIST,
        "completion checklist",
    )
}

pub fn load_code_review_prompt(repo_root: &std::path::Path) -> Result<String> {
    load_prompt_with_fallback(
        repo_root,
        CODE_REVIEW_PROMPT_REL_PATH,
        DEFAULT_CODE_REVIEW_PROMPT,
        "code review",
    )
}

pub fn load_phase2_handoff_checklist(repo_root: &std::path::Path) -> Result<String> {
    load_prompt_with_fallback(
        repo_root,
        PHASE2_HANDOFF_CHECKLIST_REL_PATH,
        DEFAULT_PHASE2_HANDOFF_CHECKLIST,
        "phase2 handoff checklist",
    )
}

pub fn render_completion_checklist(template: &str, config: &Config) -> Result<String> {
    let expanded = super::expand_variables(template, config)?;
    ensure_no_unresolved_placeholders(&expanded, "completion checklist")?;
    Ok(expanded)
}

pub fn render_phase2_handoff_checklist(template: &str, config: &Config) -> Result<String> {
    let expanded = super::expand_variables(template, config)?;
    ensure_no_unresolved_placeholders(&expanded, "phase2 handoff checklist")?;
    Ok(expanded)
}

pub fn render_code_review_prompt(
    template: &str,
    task_id: &str,
    git_status: &str,
    git_diff: &str,
    git_diff_staged: &str,
    project_type: ProjectType,
    config: &Config,
) -> Result<String> {
    if !template.contains("{{TASK_ID}}") {
        bail!("Template error: code review prompt template is missing the required '{{TASK_ID}}' placeholder.");
    }
    if !template.contains("{{GIT_STATUS}}") {
        bail!("Template error: code review prompt template is missing the required '{{GIT_STATUS}}' placeholder.");
    }
    if !template.contains("{{GIT_DIFF}}") {
        bail!("Template error: code review prompt template is missing the required '{{GIT_DIFF}}' placeholder.");
    }
    if !template.contains("{{GIT_DIFF_STAGED}}") {
        bail!("Template error: code review prompt template is missing the required '{{GIT_DIFF_STAGED}}' placeholder.");
    }

    let id = task_id.trim();
    if id.is_empty() {
        bail!("Missing task id: code review prompt requires a non-empty task id.");
    }

    let expanded = super::expand_variables(template, config)?;
    let guidance = project_type_guidance(project_type);
    let mut rendered = if expanded.contains("{{PROJECT_TYPE_GUIDANCE}}") {
        expanded.replace("{{PROJECT_TYPE_GUIDANCE}}", guidance)
    } else {
        format!("{}\n{}", expanded, guidance)
    };

    rendered = rendered.replace("{{TASK_ID}}", id);
    rendered = rendered.replace("{{GIT_STATUS}}", git_status);
    rendered = rendered.replace("{{GIT_DIFF}}", git_diff);
    rendered = rendered.replace("{{GIT_DIFF_STAGED}}", git_diff_staged);

    ensure_no_unresolved_placeholders(&rendered, "code review")?;
    Ok(rendered)
}
