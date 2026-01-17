use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

// status_porcelain returns raw `git status --porcelain` output (may be empty).
pub fn status_porcelain(repo_root: &Path) -> Result<String> {
	let output = Command::new("git")
		.arg("-C")
		.arg(repo_root)
		.arg("status")
		.arg("--porcelain")
		.output()
		.with_context(|| format!("run git status --porcelain in {}", repo_root.display()))?;

	if !output.status.success() {
		let stderr = String::from_utf8_lossy(&output.stderr);
		bail!(
			"git status --porcelain failed (code={:?}): {}",
			output.status.code(),
			stderr.trim()
		);
	}

	Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// require_clean_repo fails if the repo has any uncommitted changes.
// This enforces the assumption that the repo is clean before any agent run.
pub fn require_clean_repo(repo_root: &Path) -> Result<()> {
	let status = status_porcelain(repo_root)?;
	if status.trim().is_empty() {
		return Ok(());
	}
	bail!(
		"repo is dirty; commit/stash your changes before running Ralph.\n\ngit status --porcelain:\n{}",
		status.trim_end()
	);
}

fn git_run(repo_root: &Path, args: &[&str]) -> Result<()> {
	let output = Command::new("git")
		.arg("-C")
		.arg(repo_root)
		.args(args)
		.output()
		.with_context(|| format!("run git {} in {}", args.join(" "), repo_root.display()))?;

	if output.status.success() {
		return Ok(());
	}

	let stderr = String::from_utf8_lossy(&output.stderr);
	bail!(
		"git {} failed (code={:?}): {}",
		args.join(" "),
		output.status.code(),
		stderr.trim()
	);
}

// revert_uncommitted discards ONLY uncommitted changes.
// It does NOT reset to a pre-run SHA; it restores the working tree to current HEAD.
pub fn revert_uncommitted(repo_root: &Path) -> Result<()> {
	// Revert tracked changes in both index and working tree.
	// Prefer `git restore` (modern); fall back to older `git checkout` syntax.
	if let Err(_) = git_run(repo_root, &["restore", "--staged", "--worktree", "."]) {
		// Older git fallback.
		git_run(repo_root, &["checkout", "--", "."]).context("fallback git checkout -- .")?;
		// Ensure staged changes are cleared too.
		git_run(repo_root, &["reset", "--quiet", "HEAD"]).context("git reset --quiet HEAD")?;
	}

	// Remove untracked files/directories created during the run.
	git_run(repo_root, &["clean", "-fd"]).context("git clean -fd")?;
	Ok(())
}