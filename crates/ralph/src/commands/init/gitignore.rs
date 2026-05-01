//! Gitignore management for CueLoop initialization.
//!
//! Purpose:
//! - Gitignore management for CueLoop initialization.
//!
//! Responsibilities:
//! - Ensure active runtime `workspaces/` entries are in `.gitignore` to prevent dirty repo issues.
//! - Ensure active runtime `logs/` entries are in `.gitignore` to prevent committing unredacted debug logs.
//! - Ensure active runtime `trust.jsonc` entries are in `.gitignore` to keep local trust decisions untracked.
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

/// Ensures CueLoop-specific entries exist in `.gitignore`.
///
/// The active runtime dir comes from config path authority: new repos get `.cueloop/*`,
/// while legacy repos with `.ralph` markers keep `.ralph/*` entries.
///
/// This function is idempotent - calling it multiple times is safe.
pub fn ensure_ralph_gitignore_entries(repo_root: &Path) -> Result<()> {
    let gitignore_path = repo_root.join(".gitignore");

    let existing_content = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path)
            .with_context(|| format!("read {}", gitignore_path.display()))?
    } else {
        String::new()
    };

    let runtime_name = active_runtime_name(repo_root);
    let logs_entry = format!("{runtime_name}/logs/");
    let workspaces_entry = format!("{runtime_name}/workspaces/");
    let trust_entry = format!("{runtime_name}/trust.jsonc");

    let needs_workspaces_entry = !existing_content
        .lines()
        .any(|line| is_runtime_ignore_entry(line, &runtime_name, "workspaces"));
    let needs_logs_entry = !existing_content
        .lines()
        .any(|line| is_runtime_ignore_entry(line, &runtime_name, "logs"));
    let needs_trust_entry = !existing_content
        .lines()
        .any(|line| line.trim() == trust_entry);

    if !needs_workspaces_entry && !needs_logs_entry && !needs_trust_entry {
        log::debug!("{workspaces_entry}, {logs_entry}, and {trust_entry} already in .gitignore");
        return Ok(());
    }

    let mut new_content = existing_content;
    if !new_content.is_empty() && !new_content.ends_with('\n') {
        new_content.push('\n');
    }

    if needs_logs_entry {
        if !new_content.is_empty() {
            new_content.push('\n');
        }
        new_content.push_str("# CueLoop debug logs (raw/unredacted; do not commit)\n");
        new_content.push_str(&logs_entry);
        new_content.push('\n');
    }

    if needs_workspaces_entry {
        if !new_content.is_empty() {
            new_content.push('\n');
        }
        new_content.push_str("# CueLoop parallel mode workspace directories\n");
        new_content.push_str(&workspaces_entry);
        new_content.push('\n');
    }

    if needs_trust_entry {
        if !new_content.is_empty() {
            new_content.push('\n');
        }
        new_content.push_str("# CueLoop local trust decision (machine-local; do not commit)\n");
        new_content.push_str(&trust_entry);
        new_content.push('\n');
    }

    fs::write(&gitignore_path, new_content)
        .with_context(|| format!("write {}", gitignore_path.display()))?;

    if needs_logs_entry {
        log::info!("Added '{}' to .gitignore", logs_entry);
    }
    if needs_workspaces_entry {
        log::info!("Added '{}' to .gitignore", workspaces_entry);
    }
    if needs_trust_entry {
        log::info!("Added '{}' to .gitignore", trust_entry);
    }

    Ok(())
}

fn is_runtime_ignore_entry(line: &str, runtime_name: &str, child: &str) -> bool {
    let trimmed = line.trim();
    trimmed == format!("{runtime_name}/{child}/") || trimmed == format!("{runtime_name}/{child}")
}

fn active_runtime_name(repo_root: &Path) -> String {
    crate::config::project_runtime_dir(repo_root)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(crate::constants::identity::PROJECT_RUNTIME_DIR)
        .to_string()
}

/// Ensure queue/done files are ignored for local-private queue mode.
pub fn ensure_local_queue_gitignore_entries(repo_root: &Path) -> Result<()> {
    let runtime_name = active_runtime_name(repo_root);
    let entries = vec![
        format!("{runtime_name}/queue.jsonc"),
        format!("{runtime_name}/done.jsonc"),
    ];
    ensure_exact_gitignore_entries(
        repo_root,
        "# CueLoop local queue state",
        &entries,
        "local queue/done files",
    )
}

fn ensure_exact_gitignore_entries(
    repo_root: &Path,
    header: &str,
    entries: &[String],
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
        .filter(|entry| {
            !existing_content
                .lines()
                .any(|line| line.trim() == entry.as_str())
        })
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
    fn ensure_ralph_gitignore_entries_creates_current_runtime_file() -> Result<()> {
        let temp = TempDir::new()?;
        let repo_root = temp.path();

        ensure_ralph_gitignore_entries(repo_root)?;

        let gitignore_path = repo_root.join(".gitignore");
        assert!(gitignore_path.exists());
        let content = fs::read_to_string(&gitignore_path)?;
        assert!(content.contains(".cueloop/workspaces/"));
        assert!(content.contains(".cueloop/logs/"));
        assert!(content.contains(".cueloop/trust.jsonc"));
        assert!(content.contains("# CueLoop parallel mode"));
        assert!(content.contains("# CueLoop debug logs"));
        assert!(content.contains("# CueLoop local trust decision"));
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
        assert!(content.contains(".cueloop/workspaces/"));
        assert!(content.contains(".cueloop/logs/"));
        assert!(content.contains(".cueloop/trust.jsonc"));
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
        let workspaces_count = content.matches(".cueloop/workspaces/").count();
        let logs_count = content.matches(".cueloop/logs/").count();
        let trust_count = content.matches(".cueloop/trust.jsonc").count();
        assert_eq!(
            workspaces_count, 1,
            "Should only have one .cueloop/workspaces/ entry"
        );
        assert_eq!(logs_count, 1, "Should only have one .cueloop/logs/ entry");
        assert_eq!(
            trust_count, 1,
            "Should only have one .cueloop/trust.jsonc entry"
        );
        Ok(())
    }

    #[test]
    fn ensure_ralph_gitignore_entries_detects_existing_workspaces_entry() -> Result<()> {
        let temp = TempDir::new()?;
        let repo_root = temp.path();
        let gitignore_path = repo_root.join(".gitignore");
        fs::write(&gitignore_path, ".cueloop/workspaces/\n")?;

        ensure_ralph_gitignore_entries(repo_root)?;

        let content = fs::read_to_string(&gitignore_path)?;
        // Should add logs and trust but not duplicate workspaces
        assert!(content.contains(".cueloop/logs/"));
        assert!(content.contains(".cueloop/trust.jsonc"));
        let workspaces_count = content.matches(".cueloop/workspaces/").count();
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
        fs::write(&gitignore_path, ".cueloop/logs/\n")?;

        ensure_ralph_gitignore_entries(repo_root)?;

        let content = fs::read_to_string(&gitignore_path)?;
        // Should add workspaces and trust but not duplicate logs
        assert!(content.contains(".cueloop/workspaces/"));
        assert!(content.contains(".cueloop/trust.jsonc"));
        let logs_count = content.matches(".cueloop/logs/").count();
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
        assert_eq!(content.matches(".cueloop/queue.jsonc").count(), 1);
        assert_eq!(content.matches(".cueloop/done.jsonc").count(), 1);
        Ok(())
    }

    #[test]
    fn ensure_ralph_gitignore_entries_detects_existing_entry_without_trailing_slash() -> Result<()>
    {
        let temp = TempDir::new()?;
        let repo_root = temp.path();
        let gitignore_path = repo_root.join(".gitignore");
        fs::write(&gitignore_path, ".cueloop/workspaces\n.cueloop/logs\n")?;

        ensure_ralph_gitignore_entries(repo_root)?;

        let content = fs::read_to_string(&gitignore_path)?;
        // Should not add the trailing-slash version if non-trailing exists
        let workspaces_count = content
            .lines()
            .filter(|l| l.contains(".cueloop/workspaces"))
            .count();
        let logs_count = content
            .lines()
            .filter(|l| l.contains(".cueloop/logs"))
            .count();
        assert_eq!(
            workspaces_count, 1,
            "Should not add duplicate workspaces entry"
        );
        assert_eq!(logs_count, 1, "Should not add duplicate logs entry");
        Ok(())
    }

    #[test]
    fn ensure_ralph_gitignore_entries_uses_legacy_runtime_when_marked() -> Result<()> {
        let temp = TempDir::new()?;
        let repo_root = temp.path();
        fs::create_dir_all(repo_root.join(".ralph"))?;
        fs::write(repo_root.join(".ralph/config.jsonc"), "{}")?;

        ensure_ralph_gitignore_entries(repo_root)?;
        ensure_local_queue_gitignore_entries(repo_root)?;

        let content = fs::read_to_string(repo_root.join(".gitignore"))?;
        assert!(content.contains(".ralph/workspaces/"));
        assert!(content.contains(".ralph/logs/"));
        assert!(content.contains(".ralph/trust.jsonc"));
        assert!(content.contains(".ralph/queue.jsonc"));
        assert!(content.contains(".ralph/done.jsonc"));
        assert!(!content.contains(".cueloop/workspaces/"));
        assert!(!content.contains(".cueloop/queue.jsonc"));
        Ok(())
    }

    #[test]
    fn is_runtime_ignore_entry_matches_variations() {
        assert!(is_runtime_ignore_entry(
            ".cueloop/logs/",
            ".cueloop",
            "logs"
        ));
        assert!(is_runtime_ignore_entry(".cueloop/logs", ".cueloop", "logs"));
        assert!(is_runtime_ignore_entry(
            "  .cueloop/logs/  ",
            ".cueloop",
            "logs"
        ));
        assert!(is_runtime_ignore_entry(
            "  .cueloop/logs  ",
            ".cueloop",
            "logs"
        ));
        assert!(!is_runtime_ignore_entry(
            ".cueloop/logs/debug.log",
            ".cueloop",
            "logs"
        ));
        assert!(!is_runtime_ignore_entry(
            "# .cueloop/logs/",
            ".cueloop",
            "logs"
        ));
        assert!(!is_runtime_ignore_entry(
            "something else",
            ".cueloop",
            "logs"
        ));
    }
}
