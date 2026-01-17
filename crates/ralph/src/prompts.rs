use crate::contracts::Task;
use anyhow::{bail, Context, Result};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const WORKER_PROMPT_REL_PATH: &str = ".ralph/prompts/worker.md";
const TASK_BUILDER_PROMPT_REL_PATH: &str = ".ralph/prompts/task_builder.md";
const SCAN_PROMPT_REL_PATH: &str = ".ralph/prompts/scan.md";

pub fn worker_prompt_path(repo_root: &Path) -> PathBuf {
	repo_root.join(WORKER_PROMPT_REL_PATH)
}

pub fn load_worker_prompt(repo_root: &Path) -> Result<String> {
	let path = worker_prompt_path(repo_root);
	match fs::read_to_string(&path) {
		Ok(contents) => Ok(contents),
		Err(err) if err.kind() == io::ErrorKind::NotFound => bail!(
			"worker prompt template not found at {} (expected repo-local prompts).",
			path.display()
		),
		Err(err) => Err(err).with_context(|| format!("read worker prompt {}", path.display())),
	}
}

pub fn render_worker_prompt(template: &str, task: &Task) -> Result<String> {
	if !template.contains("{{TASK_YAML}}") {
		bail!("worker prompt template missing {{TASK_YAML}} placeholder");
	}
	let task_yaml = serde_yaml::to_string(task).context("serialize task YAML")?;

	let mut rendered = template.replace("{{INTERACTIVE_INSTRUCTIONS}}", "");
	rendered = rendered.replace("{{TASK_YAML}}", task_yaml.trim_end());
	Ok(rendered)
}

pub fn task_builder_prompt_path(repo_root: &Path) -> PathBuf {
	repo_root.join(TASK_BUILDER_PROMPT_REL_PATH)
}

pub fn load_task_builder_prompt(repo_root: &Path) -> Result<String> {
	let path = task_builder_prompt_path(repo_root);
	match fs::read_to_string(&path) {
		Ok(contents) => Ok(contents),
		Err(err) if err.kind() == io::ErrorKind::NotFound => bail!(
			"task builder prompt template not found at {} (expected repo-local prompts).",
			path.display()
		),
		Err(err) => Err(err)
			.with_context(|| format!("read task builder prompt {}", path.display())),
	}
}

pub fn render_task_builder_prompt(
	template: &str,
	user_request: &str,
	hint_tags: &str,
	hint_scope: &str,
) -> Result<String> {
	if !template.contains("{{USER_REQUEST}}") {
		bail!("task builder prompt template missing {{USER_REQUEST}} placeholder");
	}
	if !template.contains("{{HINT_TAGS}}") {
		bail!("task builder prompt template missing {{HINT_TAGS}} placeholder");
	}
	if !template.contains("{{HINT_SCOPE}}") {
		bail!("task builder prompt template missing {{HINT_SCOPE}} placeholder");
	}

	let request = user_request.trim();
	if request.is_empty() {
		bail!("user request must be non-empty");
	}

	let mut rendered = template.replace("{{USER_REQUEST}}", request);
	rendered = rendered.replace("{{HINT_TAGS}}", hint_tags.trim());
	rendered = rendered.replace("{{HINT_SCOPE}}", hint_scope.trim());
	rendered = rendered.replace("{{INTERACTIVE_INSTRUCTIONS}}", "");
	Ok(rendered)
}

pub fn scan_prompt_path(repo_root: &Path) -> PathBuf {
	repo_root.join(SCAN_PROMPT_REL_PATH)
}

pub fn load_scan_prompt(repo_root: &Path) -> Result<String> {
	let path = scan_prompt_path(repo_root);
	match fs::read_to_string(&path) {
		Ok(contents) => Ok(contents),
		Err(err) if err.kind() == io::ErrorKind::NotFound => bail!(
			"scan prompt template not found at {} (expected repo-local prompts).",
			path.display()
		),
		Err(err) => Err(err).with_context(|| format!("read scan prompt {}", path.display())),
	}
}

pub fn render_scan_prompt(template: &str, user_focus: &str) -> Result<String> {
	if !template.contains("{{USER_FOCUS}}") {
		bail!("scan prompt template missing {{USER_FOCUS}} placeholder");
	}
	let focus = user_focus.trim();
	let focus = if focus.is_empty() { "(none)" } else { focus };
	Ok(template.replace("{{USER_FOCUS}}", focus))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::contracts::{Task, TaskStatus};

	fn dummy_task() -> Task {
		Task {
			id: "RQ-0001".to_string(),
			status: TaskStatus::Todo,
			title: "Example".to_string(),
			tags: vec!["code".to_string()],
			scope: vec!["crates/ralph/src/prompts.rs".to_string()],
			evidence: vec!["Test".to_string()],
			plan: vec!["Do thing".to_string()],
			notes: vec![],
			request: None,
			agent: None,
			created_at: None,
			updated_at: None,
			completed_at: None,
			blocked_reason: None,
		}
	}

	#[test]
	fn render_worker_prompt_replaces_task_yaml() -> Result<()> {
		let template = "Hello\n{{INTERACTIVE_INSTRUCTIONS}}\n# CURRENT TASK\n{{TASK_YAML}}\n";
		let rendered = render_worker_prompt(template, &dummy_task())?;
		assert!(rendered.contains("RQ-0001"));
		assert!(!rendered.contains("{{TASK_YAML}}"));
		Ok(())
	}

	#[test]
	fn render_scan_prompt_replaces_focus_placeholder() -> Result<()> {
		let template = "FOCUS:\n{{USER_FOCUS}}\n";
		let rendered = render_scan_prompt(template, "hello world")?;
		assert!(rendered.contains("hello world"));
		assert!(!rendered.contains("{{USER_FOCUS}}"));
		Ok(())
	}

	#[test]
	fn render_task_builder_prompt_replaces_placeholders() -> Result<()> {
		let template = "Request:\n{{USER_REQUEST}}\nTags:\n{{HINT_TAGS}}\nScope:\n{{HINT_SCOPE}}\n";
		let rendered = render_task_builder_prompt(template, "do thing", "code", "repo")?;
		assert!(rendered.contains("do thing"));
		assert!(rendered.contains("code"));
		assert!(rendered.contains("repo"));
		assert!(!rendered.contains("{{USER_REQUEST}}"));
		Ok(())
	}
}