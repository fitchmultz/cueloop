//! Regression tests for explicit runtime-dir migration.
//!
//! Purpose:
//! - Verify `.ralph` -> `.cueloop` runtime-dir migration behavior.
//!
//! Responsibilities:
//! - Cover state classification, safe apply, collision refusal, generated README refresh,
//!   `.gitignore` rewrites, config reference rewrites, and migration history recording.
//!
//! Scope:
//! - Test-only coverage for `super::runtime_dir`.
//! - Does not test the CLI parser; integration coverage lives in `tests/migration_cli_integration_test.rs`.
//!
//! Usage:
//! - Included by `runtime_dir.rs` under `#[cfg(test)]`.
//!
//! Invariants/Assumptions:
//! - Test fixtures use temporary directories only.
//! - Old runtime files are moved only by the explicit runtime-dir command.

use super::{
    OLD_PROJECT_RUNTIME_DIR, RUNTIME_DIR_MIGRATION_ID, RuntimeDirMigrationState,
    apply_runtime_dir_migration, check_runtime_dir_migration,
    update_gitignore_runtime_dir_references,
};
use crate::constants::identity::PROJECT_RUNTIME_DIR;
use crate::migration::history;
use anyhow::Result;
use std::{fs, path::Path};
use tempfile::TempDir;

fn write_minimal_old_runtime(repo_root: &Path) -> Result<()> {
    let runtime = repo_root.join(OLD_PROJECT_RUNTIME_DIR);
    fs::create_dir_all(&runtime)?;
    fs::write(runtime.join("queue.jsonc"), r#"{"version":1,"tasks":[]}"#)?;
    fs::write(runtime.join("done.jsonc"), r#"{"version":1,"tasks":[]}"#)?;
    fs::write(
        runtime.join("config.jsonc"),
        r#"{"version":2,"queue":{"file":".ralph/queue.jsonc","done_file":".ralph/done.jsonc"}}"#,
    )?;
    Ok(())
}

#[test]
fn check_reports_uninitialized_when_no_runtime_dirs_exist() -> Result<()> {
    let temp = TempDir::new()?;

    let state = check_runtime_dir_migration(temp.path());

    assert!(matches!(
        state,
        RuntimeDirMigrationState::Uninitialized { .. }
    ));
    assert_eq!(state.label(), "no-op/uninitialized");
    assert!(!state.check_should_fail());
    Ok(())
}

#[test]
fn check_reports_already_current_when_only_cueloop_exists() -> Result<()> {
    let temp = TempDir::new()?;
    fs::create_dir_all(temp.path().join(PROJECT_RUNTIME_DIR))?;

    let state = check_runtime_dir_migration(temp.path());

    assert!(matches!(
        state,
        RuntimeDirMigrationState::AlreadyCurrent { .. }
    ));
    assert_eq!(state.label(), "already-current");
    assert!(!state.check_should_fail());
    Ok(())
}

#[test]
fn check_reports_needs_migration_when_only_old_runtime_exists() -> Result<()> {
    let temp = TempDir::new()?;
    write_minimal_old_runtime(temp.path())?;

    let state = check_runtime_dir_migration(temp.path());

    assert!(matches!(
        state,
        RuntimeDirMigrationState::NeedsMigration { .. }
    ));
    assert_eq!(state.label(), "needs-migration");
    assert!(state.check_should_fail());
    Ok(())
}

#[test]
fn check_reports_collision_when_both_dirs_exist() -> Result<()> {
    let temp = TempDir::new()?;
    fs::create_dir_all(temp.path().join(OLD_PROJECT_RUNTIME_DIR))?;
    fs::create_dir_all(temp.path().join(PROJECT_RUNTIME_DIR))?;

    let state = check_runtime_dir_migration(temp.path());

    assert!(matches!(state, RuntimeDirMigrationState::Collision { .. }));
    assert_eq!(state.label(), "collision");
    assert!(state.guidance().contains("No changes were made"));
    assert!(state.check_should_fail());
    Ok(())
}

#[test]
fn check_reports_collision_when_old_runtime_is_file() -> Result<()> {
    let temp = TempDir::new()?;
    fs::write(temp.path().join(OLD_PROJECT_RUNTIME_DIR), "not a directory")?;

    let state = check_runtime_dir_migration(temp.path());

    assert!(matches!(state, RuntimeDirMigrationState::Collision { .. }));
    assert!(state.guidance().contains("is not a directory"));
    assert!(state.check_should_fail());
    Ok(())
}

#[test]
fn apply_moves_runtime_dir_updates_refs_and_records_history() -> Result<()> {
    let temp = TempDir::new()?;
    write_minimal_old_runtime(temp.path())?;
    fs::write(
        temp.path().join(".gitignore"),
        ".ralph/logs/\n.ralph/workspaces/\n.ralph/trust.jsonc\n.ralph/trust.json\n.ralph/cache/\n.ralph/undo/\n.ralph/webhooks/\n",
    )?;

    let report = apply_runtime_dir_migration(temp.path())?;

    assert!(report.moved);
    assert!(report.gitignore_updated);
    assert_eq!(report.config_files_updated, 1);
    assert!(report.history_recorded);
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert!(!temp.path().join(OLD_PROJECT_RUNTIME_DIR).exists());
    assert!(temp.path().join(PROJECT_RUNTIME_DIR).is_dir());

    let config = fs::read_to_string(temp.path().join(".cueloop/config.jsonc"))?;
    assert!(config.contains(".cueloop/queue.jsonc"));
    assert!(config.contains(".cueloop/done.jsonc"));
    assert!(!config.contains(".ralph/queue.jsonc"));

    let gitignore = fs::read_to_string(temp.path().join(".gitignore"))?;
    assert!(gitignore.contains(".cueloop/logs/"));
    assert!(gitignore.contains(".cueloop/workspaces/"));
    assert!(gitignore.contains(".cueloop/trust.jsonc"));
    assert!(gitignore.contains(".cueloop/trust.json"));
    assert!(gitignore.contains(".cueloop/cache/"));
    assert!(gitignore.contains(".cueloop/undo/"));
    assert!(gitignore.contains(".cueloop/webhooks/"));
    assert!(!gitignore.contains(".ralph/"));

    let loaded = history::load_migration_history(temp.path())?;
    assert!(
        loaded
            .applied_migrations
            .iter()
            .any(|migration| migration.id == RUNTIME_DIR_MIGRATION_ID)
    );
    assert!(temp.path().join(".cueloop/cache/migrations.jsonc").exists());
    Ok(())
}

#[test]
fn apply_refuses_collision_before_mutation() -> Result<()> {
    let temp = TempDir::new()?;
    write_minimal_old_runtime(temp.path())?;
    fs::create_dir_all(temp.path().join(PROJECT_RUNTIME_DIR))?;
    fs::write(temp.path().join(".cueloop/config.jsonc"), "{}")?;

    let err = apply_runtime_dir_migration(temp.path()).unwrap_err();

    assert!(err.to_string().contains("Runtime-dir migration is blocked"));
    assert!(temp.path().join(".cueloop/config.jsonc").exists());
    assert!(temp.path().join(OLD_PROJECT_RUNTIME_DIR).exists());
    Ok(())
}

#[test]
fn apply_moves_old_json_files_without_preflight_block() -> Result<()> {
    let temp = TempDir::new()?;
    write_minimal_old_runtime(temp.path())?;
    fs::write(temp.path().join(".ralph/config.json"), r#"{"version":1}"#)?;

    let report = apply_runtime_dir_migration(temp.path())?;

    assert!(report.moved);
    assert!(temp.path().join(".cueloop/config.json").exists());
    assert!(!temp.path().join(OLD_PROJECT_RUNTIME_DIR).exists());
    Ok(())
}

#[test]
fn apply_refreshes_moved_generated_readme() -> Result<()> {
    let temp = TempDir::new()?;
    write_minimal_old_runtime(temp.path())?;
    fs::write(
        temp.path().join(".ralph/README.md"),
        "<!-- CUELOOP_README_VERSION: 1 -->\n# Old CueLoop runtime files\n",
    )?;

    let report = apply_runtime_dir_migration(temp.path())?;

    assert!(report.readme_refreshed);
    let readme = fs::read_to_string(temp.path().join(".cueloop/README.md"))?;
    assert!(readme.contains("CUELOOP_README_VERSION"));
    assert!(readme.contains("CueLoop runtime files"));
    assert!(!readme.contains("Old CueLoop runtime files"));
    Ok(())
}

#[test]
fn gitignore_conversion_avoids_duplicate_current_entries() -> Result<()> {
    let temp = TempDir::new()?;
    fs::write(
        temp.path().join(".gitignore"),
        ".ralph/logs/\n.cueloop/logs/\n  .ralph/workspaces/  \n!.ralph/done.jsonc\n",
    )?;

    assert!(update_gitignore_runtime_dir_references(temp.path())?);

    let gitignore = fs::read_to_string(temp.path().join(".gitignore"))?;
    assert_eq!(gitignore.matches(".cueloop/logs/").count(), 1);
    assert!(gitignore.contains("  .cueloop/workspaces/  "));
    assert!(gitignore.contains("!.cueloop/done.jsonc"));
    assert!(!gitignore.contains("!.ralph/done.jsonc"));
    Ok(())
}
