//! Explicit repo-local runtime directory migration.
//!
//! Purpose:
//! - Move a legacy `.ralph/` project runtime directory to the current `.cueloop/` path.
//!
//! Responsibilities:
//! - Classify runtime-dir migration state without mutating files.
//! - Apply the explicit directory rename when safe.
//! - Rewrite known config and `.gitignore` path references after the rename.
//! - Refresh the generated runtime README when one moved with the runtime dir.
//! - Record migration history under the active `.cueloop/cache/migrations.jsonc` path.
//!
//! Scope:
//! - Handles only the explicit `.ralph` -> `.cueloop` runtime directory cutover.
//! - Does not run as part of the registry-backed `ralph migrate --apply` flow.
//! - Does not merge two populated runtime directories or rename binaries/packages/apps.
//!
//! Usage:
//! - Called by `ralph migrate runtime-dir --check/--apply`.
//!
//! Invariants/Assumptions:
//! - If both runtime directories exist as directories, no mutation is attempted.
//! - The directory rename is the only required mutation; follow-up reference rewrites are best-effort
//!   and are reported as warnings if they fail.
//! - Migration history is written after the runtime dir has moved so the active history path is
//!   `.cueloop/cache/migrations.jsonc`.

use crate::commands::init::readme;
use crate::constants::identity::{LEGACY_PROJECT_RUNTIME_DIR, PROJECT_RUNTIME_DIR};
use crate::migration::history::{self, AppliedMigration};
use anyhow::{Context, Result, bail};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};

/// Stable migration history ID for the explicit runtime-dir migration.
pub const RUNTIME_DIR_MIGRATION_ID: &str = "runtime_dir_rename_ralph_to_cueloop_2026_05";

const CONFIG_PATH_REWRITES: &[(&str, &str)] = &[
    (".ralph/queue.jsonc", ".cueloop/queue.jsonc"),
    (".ralph/done.jsonc", ".cueloop/done.jsonc"),
    (".ralph/config.jsonc", ".cueloop/config.jsonc"),
    (".ralph/queue.json", ".cueloop/queue.json"),
    (".ralph/done.json", ".cueloop/done.json"),
    (".ralph/config.json", ".cueloop/config.json"),
];

const KNOWN_GITIGNORE_RUNTIME_ENTRIES: &[&str] = &[
    ".ralph/queue.jsonc",
    ".ralph/done.jsonc",
    ".ralph/config.jsonc",
    ".ralph/queue.json",
    ".ralph/done.json",
    ".ralph/config.json",
    ".ralph/*.jsonc",
    ".ralph/*.json",
    ".ralph/logs/",
    ".ralph/logs",
    ".ralph/workspaces/",
    ".ralph/workspaces",
    ".ralph/trust.jsonc",
    ".ralph/cache/",
    ".ralph/cache",
    ".ralph/lock/",
    ".ralph/lock",
];

/// Non-mutating state for the runtime-dir migration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeDirMigrationState {
    /// Neither runtime directory exists yet; there is nothing to migrate.
    Uninitialized {
        legacy_path: PathBuf,
        current_path: PathBuf,
    },
    /// Current `.cueloop/` is already present and legacy `.ralph/` is absent.
    AlreadyCurrent { current_path: PathBuf },
    /// Legacy `.ralph/` exists and can be renamed to `.cueloop/`.
    NeedsMigration {
        legacy_path: PathBuf,
        current_path: PathBuf,
    },
    /// Migration is blocked by a pre-existing destination or ambiguous filesystem state.
    Collision {
        legacy_path: PathBuf,
        current_path: PathBuf,
        reason: String,
    },
}

impl RuntimeDirMigrationState {
    /// Short machine-readable-ish label for CLI output.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Uninitialized { .. } => "no-op/uninitialized",
            Self::AlreadyCurrent { .. } => "already-current",
            Self::NeedsMigration { .. } => "needs-migration",
            Self::Collision { .. } => "collision",
        }
    }

    /// Human guidance for the current state.
    pub fn guidance(&self) -> String {
        match self {
            Self::Uninitialized {
                legacy_path,
                current_path,
            } => format!(
                "No project runtime directory exists at {} or {}; nothing to migrate.",
                legacy_path.display(),
                current_path.display()
            ),
            Self::AlreadyCurrent { current_path } => format!(
                "Project runtime is already current at {}; no migration is needed.",
                current_path.display()
            ),
            Self::NeedsMigration {
                legacy_path,
                current_path,
            } => format!(
                "Legacy runtime directory {} can be moved to {}.",
                legacy_path.display(),
                current_path.display()
            ),
            Self::Collision {
                legacy_path,
                current_path,
                reason,
            } => format!(
                "Runtime-dir migration is blocked: {reason}. No changes were made. Manually inspect {} and {}, merge or remove one path, then rerun `ralph migrate runtime-dir --apply`.",
                legacy_path.display(),
                current_path.display()
            ),
        }
    }

    /// True when `--check` should fail because action or intervention is required.
    pub fn check_should_fail(&self) -> bool {
        matches!(self, Self::NeedsMigration { .. } | Self::Collision { .. })
    }
}

/// Result of applying the runtime-dir migration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDirApplyReport {
    /// State observed before applying.
    pub initial_state: RuntimeDirMigrationState,
    /// Whether the directory rename was performed.
    pub moved: bool,
    /// Whether `.gitignore` changed.
    pub gitignore_updated: bool,
    /// Number of config files whose known runtime path references changed.
    pub config_files_updated: usize,
    /// Whether the moved runtime README was refreshed.
    pub readme_refreshed: bool,
    /// Whether migration history was written.
    pub history_recorded: bool,
    /// Best-effort follow-up failures after the safe directory rename.
    pub warnings: Vec<String>,
}

/// Inspect runtime-dir migration state without mutating the filesystem.
pub fn check_runtime_dir_migration(repo_root: &Path) -> RuntimeDirMigrationState {
    let legacy_path = repo_root.join(LEGACY_PROJECT_RUNTIME_DIR);
    let current_path = repo_root.join(PROJECT_RUNTIME_DIR);
    let legacy_is_dir = legacy_path.is_dir();
    let current_is_dir = current_path.is_dir();

    match (
        legacy_is_dir,
        current_is_dir,
        legacy_path.exists(),
        current_path.exists(),
    ) {
        (true, true, _, _) => RuntimeDirMigrationState::Collision {
            legacy_path,
            current_path,
            reason: "both .ralph and .cueloop exist as directories".to_string(),
        },
        (true, false, _, true) => RuntimeDirMigrationState::Collision {
            legacy_path,
            current_path,
            reason: ".cueloop exists and is not a directory".to_string(),
        },
        (true, false, _, false) => RuntimeDirMigrationState::NeedsMigration {
            legacy_path,
            current_path,
        },
        (false, true, true, _) => RuntimeDirMigrationState::Collision {
            legacy_path,
            current_path,
            reason: ".ralph exists and is not a directory while .cueloop is active".to_string(),
        },
        (false, true, false, _) => RuntimeDirMigrationState::AlreadyCurrent { current_path },
        (false, false, true, _) => RuntimeDirMigrationState::Collision {
            legacy_path,
            current_path,
            reason: ".ralph exists and is not a directory".to_string(),
        },
        (false, false, false, true) => RuntimeDirMigrationState::Collision {
            legacy_path,
            current_path,
            reason: ".cueloop exists and is not a directory".to_string(),
        },
        (false, false, false, false) => RuntimeDirMigrationState::Uninitialized {
            legacy_path,
            current_path,
        },
    }
}

/// Apply the explicit runtime-dir migration when safe.
pub fn apply_runtime_dir_migration(repo_root: &Path) -> Result<RuntimeDirApplyReport> {
    let initial_state = check_runtime_dir_migration(repo_root);
    match &initial_state {
        RuntimeDirMigrationState::Uninitialized { .. }
        | RuntimeDirMigrationState::AlreadyCurrent { .. } => {
            return Ok(RuntimeDirApplyReport {
                initial_state,
                moved: false,
                gitignore_updated: false,
                config_files_updated: 0,
                readme_refreshed: false,
                history_recorded: false,
                warnings: Vec::new(),
            });
        }
        RuntimeDirMigrationState::Collision { .. } => {
            bail!(initial_state.guidance());
        }
        RuntimeDirMigrationState::NeedsMigration {
            legacy_path,
            current_path,
        } => {
            let legacy_json_files = legacy_json_runtime_files(legacy_path);
            if !legacy_json_files.is_empty() {
                let rendered = legacy_json_files
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                bail!(
                    "runtime-dir migration is blocked because legacy JSON runtime files still exist: {rendered}. Run `ralph migrate --apply` first to convert legacy .json files to .jsonc, then rerun `ralph migrate runtime-dir --apply`. No changes were made."
                );
            }

            fs::rename(legacy_path, current_path).with_context(|| {
                format!(
                    "move runtime directory {} to {}",
                    legacy_path.display(),
                    current_path.display()
                )
            })?;
        }
    }

    let mut warnings = Vec::new();

    let gitignore_updated = collect_warning(
        &mut warnings,
        "update .gitignore runtime-dir references",
        || update_gitignore_runtime_dir_references(repo_root),
    )
    .unwrap_or(false);

    let config_files_updated = collect_warning(
        &mut warnings,
        "update config runtime-dir references",
        || update_config_runtime_dir_references(repo_root),
    )
    .unwrap_or(0);

    let readme_refreshed = collect_warning(&mut warnings, "refresh runtime README", || {
        refresh_runtime_readme(repo_root)
    })
    .unwrap_or(false);

    let history_recorded = collect_warning(
        &mut warnings,
        "record runtime-dir migration history",
        || record_runtime_dir_migration_history(repo_root),
    )
    .unwrap_or(false);

    Ok(RuntimeDirApplyReport {
        initial_state,
        moved: true,
        gitignore_updated,
        config_files_updated,
        readme_refreshed,
        history_recorded,
        warnings,
    })
}

fn collect_warning<T, F>(warnings: &mut Vec<String>, label: &str, f: F) -> Option<T>
where
    F: FnOnce() -> Result<T>,
{
    match f() {
        Ok(value) => Some(value),
        Err(err) => {
            warnings.push(format!("{label}: {err:#}"));
            None
        }
    }
}

fn update_gitignore_runtime_dir_references(repo_root: &Path) -> Result<bool> {
    let path = repo_root.join(".gitignore");
    if !path.exists() {
        return Ok(false);
    }

    let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let existing_lines = raw
        .lines()
        .map(|line| line.trim().to_string())
        .collect::<std::collections::HashSet<_>>();

    let mut changed = false;
    let mut updated_lines = Vec::new();
    for line in raw.lines() {
        if let Some(converted) = convert_gitignore_line(line) {
            if converted.trim() != line.trim() {
                changed = true;
            }
            if converted.trim() != line.trim() && existing_lines.contains(converted.trim()) {
                continue;
            }
            updated_lines.push(converted);
        } else {
            updated_lines.push(line.to_string());
        }
    }

    let mut updated = updated_lines.join("\n");
    if raw.ends_with('\n') {
        updated.push('\n');
    }

    if updated != raw {
        crate::fsutil::write_atomic(&path, updated.as_bytes())
            .with_context(|| format!("write {}", path.display()))?;
        changed = true;
    }

    Ok(changed)
}

fn convert_gitignore_line(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let (negated, runtime_entry) = match trimmed.strip_prefix('!') {
        Some(entry) => (true, entry),
        None => (false, trimmed),
    };
    if !KNOWN_GITIGNORE_RUNTIME_ENTRIES.contains(&runtime_entry) {
        return None;
    }

    let leading_len = line.len() - line.trim_start().len();
    let trailing_len = line.len() - line.trim_end().len();
    let leading = &line[..leading_len];
    let trailing = &line[line.len() - trailing_len..];
    let converted = runtime_entry.replacen(LEGACY_PROJECT_RUNTIME_DIR, PROJECT_RUNTIME_DIR, 1);
    let negation = if negated { "!" } else { "" };
    Some(format!("{leading}{negation}{converted}{trailing}"))
}

fn legacy_json_runtime_files(legacy_path: &Path) -> Vec<PathBuf> {
    ["queue.json", "done.json", "config.json"]
        .into_iter()
        .map(|name| legacy_path.join(name))
        .filter(|path| path.exists())
        .collect()
}

fn update_config_runtime_dir_references(repo_root: &Path) -> Result<usize> {
    let mut candidates = vec![
        repo_root.join(PROJECT_RUNTIME_DIR).join("config.jsonc"),
        repo_root.join(PROJECT_RUNTIME_DIR).join("config.json"),
    ];

    let mut updated_count = 0;
    candidates.sort();
    candidates.dedup();

    for path in candidates {
        if !path.exists() {
            continue;
        }
        if update_config_file_runtime_dir_references(&path)? {
            updated_count += 1;
        }
    }

    Ok(updated_count)
}

fn update_config_file_runtime_dir_references(path: &Path) -> Result<bool> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut updated = raw.clone();
    for (old, new) in CONFIG_PATH_REWRITES {
        updated = updated.replace(old, new);
    }

    if updated == raw {
        return Ok(false);
    }

    crate::fsutil::write_atomic(path, updated.as_bytes())
        .with_context(|| format!("write {}", path.display()))?;
    Ok(true)
}

fn refresh_runtime_readme(repo_root: &Path) -> Result<bool> {
    let path = repo_root.join(PROJECT_RUNTIME_DIR).join("README.md");
    if !path.exists() {
        return Ok(false);
    }

    let (status, _) = readme::write_readme(&path, false)
        .with_context(|| format!("refresh {}", path.display()))?;
    Ok(matches!(
        status,
        crate::commands::init::FileInitStatus::Updated
    ))
}

fn record_runtime_dir_migration_history(repo_root: &Path) -> Result<bool> {
    let mut migration_history = history::load_migration_history(repo_root)?;
    let already_recorded = migration_history
        .applied_migrations
        .iter()
        .any(|migration| migration.id == RUNTIME_DIR_MIGRATION_ID);
    if !already_recorded {
        migration_history.applied_migrations.push(AppliedMigration {
            id: RUNTIME_DIR_MIGRATION_ID.to_string(),
            applied_at: Utc::now(),
            migration_type: "RuntimeDirRename".to_string(),
        });
    }
    history::save_migration_history(repo_root, &migration_history)?;
    Ok(!already_recorded)
}

#[cfg(test)]
mod tests;
