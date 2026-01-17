use crate::config;
use crate::contracts::{Model, ReasoningEffort, Runner, TaskStatus};
use crate::{gitutil, prompts, queue, runner, timeutil};
use anyhow::{anyhow, bail, Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};

const OUTPUT_TAIL_LINES: usize = 20;
const OUTPUT_TAIL_LINE_MAX_CHARS: usize = 200;

pub enum RunOutcome {
	NoTodo,
	Ran { task_id: String },
}

pub struct RunLoopOptions {
	/// 0 means "no limit"
	pub max_tasks: u32,
}

pub fn run_loop(resolved: &config::Resolved, opts: RunLoopOptions) -> Result<()> {
	let mut completed = 0u32;
	loop {
		if opts.max_tasks != 0 && completed >= opts.max_tasks {
			println!(">> [RALPH] Reached max task limit ({completed}).");
			return Ok(());
		}

		match run_one(resolved)? {
			RunOutcome::NoTodo => return Ok(()),
			RunOutcome::Ran { task_id } => {
				completed += 1;
				println!(">> [RALPH] Completed {task_id}.");
			}
		}
	}
}

pub fn run_one(resolved: &config::Resolved) -> Result<RunOutcome> {
	let queue_file = queue::load_queue(&resolved.queue_path)?;
	queue::validate_queue(&queue_file, &resolved.id_prefix, resolved.id_width)?;

	let idx = match queue_file.tasks.iter().position(|t| t.status == TaskStatus::Todo) {
		Some(idx) => idx,
		None => {
			println!(">> [RALPH] No todo tasks found.");
			return Ok(RunOutcome::NoTodo);
		}
	};

	let task = queue_file.tasks[idx].clone();
	let task_id = task.id.trim().to_string();
	if task_id.is_empty() {
		bail!("selected task has empty id");
	}

	// Require a clean repo before we invoke the runner.
	// This prevents accidental destruction of unrelated user work on failure recovery.
	gitutil::require_clean_repo(&resolved.repo_root)?;

	let task_agent = task.agent.as_ref();

	let runner_kind: Runner = task_agent
		.and_then(|agent| agent.runner)
		.or(resolved.config.agent.runner)
		.unwrap_or_default();

	let model: Model = task_agent
		.and_then(|agent| agent.model)
		.or(resolved.config.agent.model)
		.unwrap_or_default();

	let reasoning_effort: Option<ReasoningEffort> = task_agent
		.and_then(|agent| agent.reasoning_effort)
		.or(resolved.config.agent.reasoning_effort);

	runner::validate_model_for_runner(runner_kind, model)?;

	let codex_bin = resolved.config.agent.codex_bin.as_deref().unwrap_or("codex");
	let opencode_bin = resolved.config.agent.opencode_bin.as_deref().unwrap_or("opencode");

	let template = prompts::load_worker_prompt(&resolved.repo_root)?;
	let prompt = prompts::render_worker_prompt(&template, &task)?;

	let output = match runner::run_prompt(
		runner_kind,
		&resolved.repo_root,
		codex_bin,
		opencode_bin,
		model,
		reasoning_effort,
		&prompt,
	) {
		Ok(output) => output,
		Err(err) => {
			gitutil::revert_uncommitted(&resolved.repo_root)?;
			bail!(
				"runner invocation failed; reverted uncommitted changes; rerun is recommended: {:#}",
				err
			);
		}
	};

	if !output.stdout.is_empty() {
		print!("{}", output.stdout);
	}
	if !output.stderr.is_empty() {
		eprint!("{}", output.stderr);
	}

	if !output.success() {
		let exit_reason = match output.status.code() {
			Some(code) => format!("runner exited non-zero (code={code})"),
			None => "runner terminated by signal".to_string(),
		};

		let combined = output.combined();
		let tail = tail_lines(&combined, OUTPUT_TAIL_LINES, OUTPUT_TAIL_LINE_MAX_CHARS);
		if !tail.is_empty() {
			eprintln!(">> [RALPH] runner output (tail):");
			for line in tail {
				eprintln!(">> [RALPH] runner: {line}");
			}
		}

		gitutil::revert_uncommitted(&resolved.repo_root)?;
		bail!("runner failed ({exit_reason}); reverted uncommitted changes; rerun is recommended");
	}

	println!(">> [RALPH] Runner completed successfully for {task_id}.");

	post_run_supervise(resolved, &task_id)?;
	Ok(RunOutcome::Ran { task_id })
}

fn post_run_supervise(resolved: &config::Resolved, task_id: &str) -> Result<()> {
	let status = gitutil::status_porcelain(&resolved.repo_root)?;
	let is_dirty = !status.trim().is_empty();

	let mut queue_file = queue::load_queue(&resolved.queue_path)?;
	queue::validate_queue(&queue_file, &resolved.id_prefix, resolved.id_width)?;

	let (task_status, task_title) = queue_file
		.tasks
		.iter()
		.find(|t| t.id.trim() == task_id)
		.map(|t| (t.status, t.title.clone()))
		.ok_or_else(|| anyhow!("task {task_id} no longer exists in queue"))?;

	if task_status == TaskStatus::Blocked {
		if is_dirty {
			gitutil::revert_uncommitted(&resolved.repo_root)?;
			bail!("task {task_id} was marked blocked; reverted uncommitted changes");
		}
		bail!("task {task_id} was marked blocked; cannot auto-revert committed changes");
	}

	if is_dirty {
		if let Err(err) = run_make_ci(&resolved.repo_root) {
			gitutil::revert_uncommitted(&resolved.repo_root)?;
			bail!("make ci failed; reverted uncommitted changes: {:#}", err);
		}
		queue_file = queue::load_queue(&resolved.queue_path)?;
		queue::validate_queue(&queue_file, &resolved.id_prefix, resolved.id_width)?;
		let (task_status, task_title) = queue_file
			.tasks
			.iter()
			.find(|t| t.id.trim() == task_id)
			.map(|t| (t.status, t.title.clone()))
			.ok_or_else(|| anyhow!("task {task_id} no longer exists in queue"))?;

		if task_status == TaskStatus::Blocked {
			gitutil::revert_uncommitted(&resolved.repo_root)?;
			bail!("task {task_id} was marked blocked; reverted uncommitted changes");
		}

		if task_status != TaskStatus::Done {
			let now = timeutil::now_utc_rfc3339()?;
			queue::set_status(&mut queue_file, task_id, TaskStatus::Done, &now, None, None)?;
			queue::save_queue(&resolved.queue_path, &queue_file)?;
		}

		let commit_message = format_task_commit_message(task_id, &task_title);
		gitutil::commit_all(&resolved.repo_root, &commit_message)?;
		if gitutil::is_ahead_of_upstream(&resolved.repo_root)? {
			gitutil::push_upstream(&resolved.repo_root)?;
		}

		gitutil::require_clean_repo(&resolved.repo_root)?;
		return Ok(());
	}

	if task_status != TaskStatus::Done {
		let now = timeutil::now_utc_rfc3339()?;
		queue::set_status(&mut queue_file, task_id, TaskStatus::Done, &now, None, None)?;
		queue::save_queue(&resolved.queue_path, &queue_file)?;
		if let Err(err) = run_make_ci(&resolved.repo_root) {
			gitutil::revert_uncommitted(&resolved.repo_root)?;
			bail!("make ci failed; reverted uncommitted changes: {:#}", err);
		}
		let commit_message = format_task_commit_message(task_id, &task_title);
		gitutil::commit_all(&resolved.repo_root, &commit_message)?;
		if gitutil::is_ahead_of_upstream(&resolved.repo_root)? {
			gitutil::push_upstream(&resolved.repo_root)?;
		}
		gitutil::require_clean_repo(&resolved.repo_root)?;
		return Ok(());
	}

	if gitutil::is_ahead_of_upstream(&resolved.repo_root)? {
		gitutil::push_upstream(&resolved.repo_root)?;
	}

	Ok(())
}

fn run_make_ci(repo_root: &Path) -> Result<()> {
	let status = Command::new("make")
		.arg("ci")
		.current_dir(repo_root)
		.stdin(Stdio::inherit())
		.stdout(Stdio::inherit())
		.stderr(Stdio::inherit())
		.status()
		.with_context(|| format!("run make ci in {}", repo_root.display()))?;

	if status.success() {
		return Ok(());
	}

	bail!("make ci failed with exit code {:?}", status.code())
}

fn format_task_commit_message(task_id: &str, title: &str) -> String {
	let mut raw = format!("{task_id}: {title}");
	raw = raw.replace(['\n', '\r', '\t'], " ");
	let squashed = raw.split_whitespace().collect::<Vec<&str>>().join(" ");
	truncate_chars(&squashed, 100)
}

fn tail_lines(text: &str, max_lines: usize, max_chars: usize) -> Vec<String> {
	if max_lines == 0 || text.trim().is_empty() {
		return Vec::new();
	}
	let mut lines: Vec<&str> = text
		.lines()
		.map(|l| l.trim_end())
		.filter(|l| !l.trim().is_empty())
		.collect();

	if lines.len() > max_lines {
		lines = lines[lines.len() - max_lines..].to_vec();
	}

	lines
		.into_iter()
		.map(|line| truncate_chars(line.trim(), max_chars))
		.collect()
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
	if max_chars == 0 {
		return String::new();
	}
	let mut chars = value.chars();
	let mut out = String::new();
	for _ in 0..max_chars {
		match chars.next() {
			Some(ch) => out.push(ch),
			None => return out,
		}
	}
	if chars.next().is_none() {
		return out;
	}
	if max_chars <= 3 {
		return out;
	}
	out.truncate(max_chars - 3);
	out.push_str("...");
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn truncate_chars_adds_ellipsis() {
		let value = "abcdefghijklmnopqrstuvwxyz";
		let truncated = truncate_chars(value, 10);
		assert_eq!(truncated, "abcdefg...");
	}
}
