use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct GitPushError {
    pub message: String,
}

fn classify_push_error(stderr: &str) -> GitPushError {
    let raw = stderr.trim();
    let lower = raw.to_lowercase();

    if lower.contains("no upstream")
        || lower.contains("set-upstream")
        || lower.contains("set the remote as upstream")
    {
        return GitPushError {
			message: "git push failed: no upstream configured for current branch. Set it with: git push -u origin <branch> OR git branch --set-upstream-to origin/<branch>.".to_string(),
		};
    }

    if lower.contains("permission denied")
        || lower.contains("authentication failed")
        || lower.contains("access denied")
        || lower.contains("could not read from remote repository")
        || lower.contains("repository not found")
    {
        return GitPushError {
			message: "git push failed: authentication/permission denied. Verify the remote URL, credentials, and that you have push access.".to_string(),
		};
    }

    let detail = if raw.is_empty() {
        "unknown git error".to_string()
    } else {
        raw.to_string()
    };
    GitPushError {
        message: format!("git push failed: {detail}"),
    }
}

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
pub fn require_clean_repo(repo_root: &Path, force: bool) -> Result<()> {
    let status = status_porcelain(repo_root)?;
    if status.trim().is_empty() {
        return Ok(());
    }

    if force {
        return Ok(());
    }

    let mut tracked = Vec::new();
    let mut untracked = Vec::new();

    for line in status.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("??") {
            untracked.push(line);
        } else {
            tracked.push(line);
        }
    }

    let mut msg = String::from("repo is dirty; commit/stash your changes before running Ralph.");

    if !tracked.is_empty() {
        msg.push_str("\n\nTracked changes (suggest 'git stash' or 'git commit'):");
        for line in tracked.iter().take(10) {
            msg.push_str("\n  ");
            msg.push_str(line);
        }
        if tracked.len() > 10 {
            msg.push_str(&format!("\n  ...and {} more", tracked.len() - 10));
        }
    }

    if !untracked.is_empty() {
        msg.push_str("\n\nUntracked files (suggest 'git clean -fd' or 'git add'):");
        for line in untracked.iter().take(10) {
            msg.push_str("\n  ");
            msg.push_str(line);
        }
        if untracked.len() > 10 {
            msg.push_str(&format!("\n  ...and {} more", untracked.len() - 10));
        }
    }

    msg.push_str("\n\nUse --force to bypass this check if you are sure.");
    bail!(msg);
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
    if git_run(repo_root, &["restore", "--staged", "--worktree", "."]).is_err() {
        // Older git fallback.
        git_run(repo_root, &["checkout", "--", "."]).context("fallback git checkout -- .")?;
        // Ensure staged changes are cleared too.
        git_run(repo_root, &["reset", "--quiet", "HEAD"]).context("git reset --quiet HEAD")?;
    }

    // Remove untracked files/directories created during the run.
    git_run(repo_root, &["clean", "-fd", "-e", ".env", "-e", ".env.*"])
        .context("git clean -fd -e .env*")?;
    Ok(())
}

// commit_all stages everything and creates a single commit.
pub fn commit_all(repo_root: &Path, message: &str) -> Result<()> {
    let message = message.trim();
    if message.is_empty() {
        bail!("commit message is empty");
    }

    git_run(repo_root, &["add", "-A"]).context("git add -A")?;
    let status = status_porcelain(repo_root)?;
    if status.trim().is_empty() {
        bail!("no changes to commit");
    }

    git_run(repo_root, &["commit", "-m", message]).context("git commit")?;
    Ok(())
}

// upstream_ref returns the configured upstream for the current branch (e.g. "origin/main").
pub fn upstream_ref(repo_root: &Path) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("--symbolic-full-name")
        .arg("@{u}")
        .output()
        .with_context(|| {
            format!(
                "run git rev-parse --abbrev-ref --symbolic-full-name @{{u}} in {}",
                repo_root.display()
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let classified = classify_push_error(&stderr);
        bail!(
            "git rev-parse @{{u}} failed (code={:?}): {}",
            output.status.code(),
            classified.message
        );
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        bail!("no upstream configured for current branch");
    }
    Ok(value)
}

// is_ahead_of_upstream reports whether HEAD is ahead of the configured upstream.
pub fn is_ahead_of_upstream(repo_root: &Path) -> Result<bool> {
    let upstream = upstream_ref(repo_root)?;
    let range = format!("{upstream}...HEAD");
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("rev-list")
        .arg("--left-right")
        .arg("--count")
        .arg(range)
        .output()
        .with_context(|| {
            format!(
                "run git rev-list --left-right --count in {}",
                repo_root.display()
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "git rev-list --left-right --count failed (code={:?}): {}",
            output.status.code(),
            stderr.trim()
        );
    }

    let counts = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = counts.split_whitespace().collect();
    if parts.len() != 2 {
        bail!("unexpected rev-list output: {}", counts.trim());
    }

    let ahead: u32 = parts[1].parse().context("parse ahead count")?;
    Ok(ahead > 0)
}

// push_upstream pushes HEAD to the configured upstream.
pub fn push_upstream(repo_root: &Path) -> Result<()> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .arg("push")
        .output()
        .with_context(|| format!("run git push in {}", repo_root.display()))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let classified = classify_push_error(&stderr);
    bail!(classified.message)
}
