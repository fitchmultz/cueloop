//! Gitignore management for Ralph initialization.
//!
//! Purpose:
//! - Gitignore management for Ralph initialization.
//!
//! Responsibilities:
//! - Ensure `.ralph/workspaces/` is in `.gitignore` to prevent dirty repo issues.
//! - Ensure `.ralph/logs/` is in `.gitignore` to prevent committing unredacted debug logs.
//! - Ensure `.ralph/trust.jsonc` is in `.gitignore` to keep local trust decisions untracked.
//! - Optionally ensure queue/done files are ignored for local-private queue mode.
//! - Provide idempotent updates to `.gitignore`.
//!
//! Not handled here:
//! - Reading or parsing existing `.gitignore` patterns (only simple line-based checks).
//! - Global gitignore configuration (only repo-local `.gitignore`).
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Updates are additive only (never removes entries).
//! - Safe to run multiple times (idempotent).

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Ensures Ralph-specific entries exist in `.gitignore`.
///
/// Currently ensures:
/// - `.ralph/workspaces/` is ignored (prevents dirty repo when using repo-local workspaces)
/// - `.ralph/logs/` is ignored (prevents committing unredacted debug logs that may contain secrets)
/// - `.ralph/trust.jsonc` is ignored (keeps repo trust machine-local)
///
/// This function is idempotent - calling it multiple times is safe.
pub fn ensure_ralph_gitignore_entries(repo_root: &Path) -> Result<()> {
    let gitignore_path = repo_root.join(".gitignore");

    // Read existing content or start fresh
    let existing_content = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path)
            .with_context(|| format!("read {}", gitignore_path.display()))?
    } else {
        String::new()
    };

    // Check if entries already exist (handle various formats)
    let needs_workspaces_entry = !existing_content.lines().any(is_workspaces_ignore_entry);
    let needs_logs_entry = !existing_content.lines().any(is_logs_ignore_entry);
    let needs_trust_entry = !existing_content.lines().any(is_trust_ignore_entry);

    if !needs_workspaces_entry && !needs_logs_entry && !needs_trust_entry {
        log::debug!(
            ".ralph/workspaces/, .ralph/logs/, and .ralph/trust.jsonc already in .gitignore"
        );
        return Ok(());
    }

    // Append the entries
    let mut new_content = existing_content;
    let will_add_logs = needs_logs_entry;
    let will_add_workspaces = needs_workspaces_entry;
    let will_add_trust = needs_trust_entry;

    // Add newline if file doesn't end with one (and isn't empty)
    if !new_content.is_empty() && !new_content.ends_with('\n') {
        new_content.push('\n');
    }

    // Add logs entry if missing
    if needs_logs_entry {
        if !new_content.is_empty() {
            new_content.push('\n');
        }
        new_content.push_str("# Ralph debug logs (raw/unredacted; do not commit)\n");
        new_content.push_str(".ralph/logs/\n");
    }

    // Add workspaces entry if missing
    if needs_workspaces_entry {
        if !new_content.is_empty() {
            new_content.push('\n');
        }
        new_content.push_str("# Ralph parallel mode workspace directories\n");
        new_content.push_str(".ralph/workspaces/\n");
    }

    // Add trust entry if missing
    if needs_trust_entry {
        if !new_content.is_empty() {
            new_content.push('\n');
        }
        new_content.push_str("# Ralph local trust decision (machine-local; do not commit)\n");
        new_content.push_str(".ralph/trust.jsonc\n");
    }

    fs::write(&gitignore_path, new_content)
        .with_context(|| format!("write {}", gitignore_path.display()))?;

    if will_add_logs {
        log::info!("Added '.ralph/logs/' to .gitignore");
    }
    if will_add_workspaces {
        log::info!("Added '.ralph/workspaces/' to .gitignore");
    }
    if will_add_trust {
        log::info!("Added '.ralph/trust.jsonc' to .gitignore");
    }

    Ok(())
}

/// Check if a line is a workspaces ignore entry.
///
/// Matches:
/// - `.ralph/workspaces/`
/// - `.ralph/workspaces`
fn is_workspaces_ignore_entry(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed == ".ralph/workspaces/" || trimmed == ".ralph/workspaces"
}

/// Check if a line is a logs ignore entry.
///
/// Matches:
/// - `.ralph/logs/`
/// - `.ralph/logs`
fn is_logs_ignore_entry(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed == ".ralph/logs/" || trimmed == ".ralph/logs"
}

/// Check if a line is the local trust-file ignore entry.
fn is_trust_ignore_entry(line: &str) -> bool {
    line.trim() == ".ralph/trust.jsonc"
}

/// Ensure queue/done files are ignored for local-private queue mode.
pub fn ensure_local_queue_gitignore_entries(repo_root: &Path) -> Result<()> {
    let entries = [".ralph/queue.jsonc", ".ralph/done.jsonc"];
    ensure_exact_gitignore_entries(
        repo_root,
        "# Ralph local queue state",
        &entries,
        "local queue/done files",
    )
}

fn ensure_exact_gitignore_entries(
    repo_root: &Path,
    header: &str,
    entries: &[&str],
    label: &str,
) -> Result<()> {
    let gitignore_path = repo_root.join(".gitignore");
    let existing_content = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path)
            .with_context(|| format!("read {}", gitignore_path.display()))?
    } else {
        String::new()
    };
    let missing = entries
        .iter()
        .copied()
        .filter(|entry| !existing_content.lines().any(|line| line.trim() == *entry))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return Ok(());
    }

    let mut new_content = existing_content;
    if !new_content.is_empty() && !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    if !new_content.is_empty() {
        new_content.push('\n');
    }
    new_content.push_str(header);
    new_content.push('\n');
    for entry in missing {
        new_content.push_str(entry);
        new_content.push('\n');
    }
    fs::write(&gitignore_path, new_content)
        .with_context(|| format!("write {}", gitignore_path.display()))?;
    log::info!("Added {} to .gitignore", label);
    Ok(())
}

/// Migrate .json ignore patterns to .jsonc in .gitignore.
///
/// This updates Ralph-managed ignore patterns from .json to .jsonc variants.
/// Patterns like `.ralph/queue.json` become `.ralph/queue.jsonc`.
///
/// Returns true if any changes were made.
pub fn migrate_json_to_jsonc_gitignore(repo_root: &std::path::Path) -> anyhow::Result<bool> {
    let gitignore_path = repo_root.join(".gitignore");
    if !gitignore_path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&gitignore_path)
        .with_context(|| format!("read {}", gitignore_path.display()))?;

    // Define patterns to migrate: (old_pattern, new_pattern)
    let patterns_to_migrate: &[(&str, &str)] = &[
        (".ralph/queue.json", ".ralph/queue.jsonc"),
        (".ralph/done.json", ".ralph/done.jsonc"),
        (".ralph/config.json", ".ralph/config.jsonc"),
        (".ralph/*.json", ".ralph/*.jsonc"),
    ];

    let mut updated = content.clone();
    let mut made_changes = false;

    for (old_pattern, new_pattern) in patterns_to_migrate {
        // Check if old pattern exists and new pattern doesn't
        let has_old = updated.lines().any(|line| {
            let trimmed = line.trim();
            trimmed == *old_pattern || trimmed == old_pattern.trim_end_matches('/')
        });
        let has_new = updated.lines().any(|line| {
            let trimmed = line.trim();
            trimmed == *new_pattern || trimmed == new_pattern.trim_end_matches('/')
        });

        if has_old && !has_new {
            updated = updated.replace(old_pattern, new_pattern);
            log::info!(
                "Migrated .gitignore pattern: {} -> {}",
                old_pattern,
                new_pattern
            );
            made_changes = true;
        }
    }

    if made_changes {
        fs::write(&gitignore_path, updated)
            .with_context(|| format!("write {}", gitignore_path.display()))?;
    }

    Ok(made_changes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn ensure_ralph_gitignore_entries_creates_new_file() -> Result<()> {
        let temp = TempDir::new()?;
        let repo_root = temp.path();

        ensure_ralph_gitignore_entries(repo_root)?;

        let gitignore_path = repo_root.join(".gitignore");
        assert!(gitignore_path.exists());
        let content = fs::read_to_string(&gitignore_path)?;
        assert!(content.contains(".ralph/workspaces/"));
        assert!(content.contains(".ralph/logs/"));
        assert!(content.contains(".ralph/trust.jsonc"));
        assert!(content.contains("# Ralph parallel mode"));
        assert!(content.contains("# Ralph debug logs"));
        assert!(content.contains("# Ralph local trust decision"));
        Ok(())
    }

    #[test]
    fn ensure_ralph_gitignore_entries_appends_to_existing() -> Result<()> {
        let temp = TempDir::new()?;
        let repo_root = temp.path();
        let gitignore_path = repo_root.join(".gitignore");
        fs::write(&gitignore_path, ".env\ntarget/\n")?;

        ensure_ralph_gitignore_entries(repo_root)?;

        let content = fs::read_to_string(&gitignore_path)?;
        assert!(content.contains(".env"));
        assert!(content.contains("target/"));
        assert!(content.contains(".ralph/workspaces/"));
        assert!(content.contains(".ralph/logs/"));
        assert!(content.contains(".ralph/trust.jsonc"));
        Ok(())
    }

    #[test]
    fn ensure_ralph_gitignore_entries_is_idempotent() -> Result<()> {
        let temp = TempDir::new()?;
        let repo_root = temp.path();

        // Run twice
        ensure_ralph_gitignore_entries(repo_root)?;
        ensure_ralph_gitignore_entries(repo_root)?;

        let gitignore_path = repo_root.join(".gitignore");
        let content = fs::read_to_string(&gitignore_path)?;

        // Should only have one entry for each
        let workspaces_count = content.matches(".ralph/workspaces/").count();
        let logs_count = content.matches(".ralph/logs/").count();
        let trust_count = content.matches(".ralph/trust.jsonc").count();
        assert_eq!(
            workspaces_count, 1,
            "Should only have one .ralph/workspaces/ entry"
        );
        assert_eq!(logs_count, 1, "Should only have one .ralph/logs/ entry");
        assert_eq!(
            trust_count, 1,
            "Should only have one .ralph/trust.jsonc entry"
        );
        Ok(())
    }

    #[test]
    fn ensure_ralph_gitignore_entries_detects_existing_workspaces_entry() -> Result<()> {
        let temp = TempDir::new()?;
        let repo_root = temp.path();
        let gitignore_path = repo_root.join(".gitignore");
        fs::write(&gitignore_path, ".ralph/workspaces/\n")?;

        ensure_ralph_gitignore_entries(repo_root)?;

        let content = fs::read_to_string(&gitignore_path)?;
        // Should add logs and trust but not duplicate workspaces
        assert!(content.contains(".ralph/logs/"));
        assert!(content.contains(".ralph/trust.jsonc"));
        let workspaces_count = content.matches(".ralph/workspaces/").count();
        assert_eq!(
            workspaces_count, 1,
            "Should not add duplicate workspaces entry"
        );
        Ok(())
    }

    #[test]
    fn ensure_ralph_gitignore_entries_detects_existing_logs_entry() -> Result<()> {
        let temp = TempDir::new()?;
        let repo_root = temp.path();
        let gitignore_path = repo_root.join(".gitignore");
        fs::write(&gitignore_path, ".ralph/logs/\n")?;

        ensure_ralph_gitignore_entries(repo_root)?;

        let content = fs::read_to_string(&gitignore_path)?;
        // Should add workspaces and trust but not duplicate logs
        assert!(content.contains(".ralph/workspaces/"));
        assert!(content.contains(".ralph/trust.jsonc"));
        let logs_count = content.matches(".ralph/logs/").count();
        assert_eq!(logs_count, 1, "Should not add duplicate logs entry");
        Ok(())
    }

    #[test]
    fn ensure_local_queue_gitignore_entries_adds_queue_and_done_once() -> Result<()> {
        let temp = TempDir::new()?;
        let repo_root = temp.path();

        ensure_local_queue_gitignore_entries(repo_root)?;
        ensure_local_queue_gitignore_entries(repo_root)?;

        let content = fs::read_to_string(repo_root.join(".gitignore"))?;
        assert_eq!(content.matches(".ralph/queue.jsonc").count(), 1);
        assert_eq!(content.matches(".ralph/done.jsonc").count(), 1);
        Ok(())
    }

    #[test]
    fn ensure_ralph_gitignore_entries_detects_existing_entry_without_trailing_slash() -> Result<()>
    {
        let temp = TempDir::new()?;
        let repo_root = temp.path();
        let gitignore_path = repo_root.join(".gitignore");
        fs::write(&gitignore_path, ".ralph/workspaces\n.ralph/logs\n")?;

        ensure_ralph_gitignore_entries(repo_root)?;

        let content = fs::read_to_string(&gitignore_path)?;
        // Should not add the trailing-slash version if non-trailing exists
        let workspaces_count = content
            .lines()
            .filter(|l| l.contains(".ralph/workspaces"))
            .count();
        let logs_count = content
            .lines()
            .filter(|l| l.contains(".ralph/logs"))
            .count();
        assert_eq!(
            workspaces_count, 1,
            "Should not add duplicate workspaces entry"
        );
        assert_eq!(logs_count, 1, "Should not add duplicate logs entry");
        Ok(())
    }

    #[test]
    fn is_logs_ignore_entry_matches_variations() {
        assert!(is_logs_ignore_entry(".ralph/logs/"));
        assert!(is_logs_ignore_entry(".ralph/logs"));
        assert!(is_logs_ignore_entry("  .ralph/logs/  ")); // with whitespace
        assert!(is_logs_ignore_entry("  .ralph/logs  ")); // with whitespace
        assert!(!is_logs_ignore_entry(".ralph/logs/debug.log"));
        assert!(!is_logs_ignore_entry("# .ralph/logs/"));
        assert!(!is_logs_ignore_entry("something else"));
    }
}
