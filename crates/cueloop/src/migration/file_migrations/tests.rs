//! Purpose: Regression coverage for the file migration facade and companions.
//!
//! Responsibilities:
//! - Preserve the original file migration unit coverage after the module split.
//! - Exercise facade re-exports and internal config-reference helpers.
//!
//! Scope:
//! - Test-only module; production behavior lives in sibling companion modules.
//!
//! Usage:
//! - Compiled only under `#[cfg(test)]` from `file_migrations/mod.rs`.
//!
//! Invariants/Assumptions:
//! - Test names, assertions, fixtures, and behavior remain unchanged from the
//!   former inline file migration test block.
//! - Only import paths should change as needed for the new module layout.

use super::super::{MigrationContext, history};
use super::config_refs::update_config_file_references;
use super::*;
use std::fs;
use tempfile::TempDir;

fn create_test_context(dir: &TempDir) -> MigrationContext {
    let repo_root = dir.path().to_path_buf();
    let project_config_path = repo_root.join(".cueloop/config.json");

    MigrationContext {
        repo_root,
        project_config_path,
        global_config_path: None,
        resolved_config: crate::contracts::Config::default(),
        migration_history: history::MigrationHistory::default(),
    }
}

#[test]
fn apply_file_rename_copies_file() {
    let dir = TempDir::new().unwrap();
    let ctx = create_test_context(&dir);

    fs::create_dir_all(dir.path().join(".cueloop")).unwrap();
    let source = dir.path().join(".cueloop/queue.json");
    fs::write(&source, "{\"version\": 1}").unwrap();

    apply_file_rename(&ctx, ".cueloop/queue.json", ".cueloop/queue.jsonc").unwrap();

    assert!(source.exists());
    assert!(dir.path().join(".cueloop/queue.jsonc").exists());

    let original_content = fs::read_to_string(&source).unwrap();
    let new_content = fs::read_to_string(dir.path().join(".cueloop/queue.jsonc")).unwrap();
    assert_eq!(original_content, new_content);
}

#[test]
fn apply_file_rename_without_backup_removes_original() {
    let dir = TempDir::new().unwrap();
    let ctx = create_test_context(&dir);

    fs::create_dir_all(dir.path().join(".cueloop")).unwrap();
    let source = dir.path().join(".cueloop/queue.json");
    fs::write(&source, "{\"version\": 1}").unwrap();

    let opts = FileMigrationOptions {
        keep_backup: false,
        update_config: false,
    };
    apply_file_rename_with_options(&ctx, ".cueloop/queue.json", ".cueloop/queue.jsonc", &opts)
        .unwrap();

    assert!(!source.exists());
    assert!(dir.path().join(".cueloop/queue.jsonc").exists());
}

#[test]
fn apply_file_rename_fails_if_source_missing() {
    let dir = TempDir::new().unwrap();
    let ctx = create_test_context(&dir);

    let result = apply_file_rename(&ctx, ".cueloop/queue.json", ".cueloop/queue.jsonc");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));
}

#[test]
fn apply_file_rename_fails_if_destination_exists() {
    let dir = TempDir::new().unwrap();
    let ctx = create_test_context(&dir);

    fs::create_dir_all(dir.path().join(".cueloop")).unwrap();
    fs::write(dir.path().join(".cueloop/queue.json"), "{}").unwrap();
    fs::write(dir.path().join(".cueloop/queue.jsonc"), "{}").unwrap();

    let result = apply_file_rename(&ctx, ".cueloop/queue.json", ".cueloop/queue.jsonc");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}

#[test]
fn update_config_file_references_updates_queue_file() {
    let dir = TempDir::new().unwrap();
    let ctx = create_test_context(&dir);

    fs::create_dir_all(dir.path().join(".cueloop")).unwrap();
    fs::write(
        &ctx.project_config_path,
        r#"{
                "version": 1,
                "queue": {
                    "file": ".cueloop/queue.json"
                }
            }"#,
    )
    .unwrap();

    update_config_file_references(&ctx, ".cueloop/queue.json", ".cueloop/queue.jsonc").unwrap();

    let content = fs::read_to_string(&ctx.project_config_path).unwrap();
    assert!(content.contains("\"file\": \".cueloop/queue.jsonc\""));
    assert!(!content.contains("\"file\": \".cueloop/queue.json\""));
}

#[test]
fn update_config_file_references_updates_done_file() {
    let dir = TempDir::new().unwrap();
    let ctx = create_test_context(&dir);

    fs::create_dir_all(dir.path().join(".cueloop")).unwrap();
    fs::write(
        &ctx.project_config_path,
        r#"{
                "version": 1,
                "queue": {
                    "done_file": ".cueloop/done.json"
                }
            }"#,
    )
    .unwrap();

    update_config_file_references(&ctx, ".cueloop/done.json", ".cueloop/done.jsonc").unwrap();

    let content = fs::read_to_string(&ctx.project_config_path).unwrap();
    assert!(content.contains("\"done_file\": \".cueloop/done.jsonc\""));
}

#[test]
fn is_queue_json_to_jsonc_applicable_detects_correct_state() {
    let dir = TempDir::new().unwrap();
    let ctx = create_test_context(&dir);

    assert!(!is_queue_json_to_jsonc_applicable(&ctx));

    fs::create_dir_all(dir.path().join(".cueloop")).unwrap();
    fs::write(dir.path().join(".cueloop/queue.json"), "{}").unwrap();
    assert!(is_queue_json_to_jsonc_applicable(&ctx));

    fs::write(dir.path().join(".cueloop/queue.jsonc"), "{}").unwrap();
    assert!(is_queue_json_to_jsonc_applicable(&ctx));
}

#[test]
fn migrate_queue_json_to_jsonc_removes_legacy_file_when_jsonc_absent() {
    let dir = TempDir::new().unwrap();
    let ctx = create_test_context(&dir);

    fs::create_dir_all(dir.path().join(".cueloop")).unwrap();
    fs::write(dir.path().join(".cueloop/queue.json"), "{\"version\": 1}").unwrap();

    migrate_queue_json_to_jsonc(&ctx).unwrap();

    assert!(!dir.path().join(".cueloop/queue.json").exists());
    assert!(dir.path().join(".cueloop/queue.jsonc").exists());
}

#[test]
fn migrate_queue_json_to_jsonc_removes_legacy_file_when_jsonc_already_exists() {
    let dir = TempDir::new().unwrap();
    let ctx = create_test_context(&dir);

    fs::create_dir_all(dir.path().join(".cueloop")).unwrap();
    fs::write(dir.path().join(".cueloop/queue.json"), "{\"legacy\": true}").unwrap();
    fs::write(dir.path().join(".cueloop/queue.jsonc"), "{\"version\": 1}").unwrap();

    migrate_queue_json_to_jsonc(&ctx).unwrap();

    assert!(!dir.path().join(".cueloop/queue.json").exists());
    assert!(dir.path().join(".cueloop/queue.jsonc").exists());
}

#[test]
fn rollback_file_migration_restores_original() {
    let dir = TempDir::new().unwrap();
    let ctx = create_test_context(&dir);

    fs::create_dir_all(dir.path().join(".cueloop")).unwrap();
    fs::write(dir.path().join(".cueloop/queue.json"), "{\"version\": 1}").unwrap();
    apply_file_rename(&ctx, ".cueloop/queue.json", ".cueloop/queue.jsonc").unwrap();

    assert!(dir.path().join(".cueloop/queue.json").exists());
    assert!(dir.path().join(".cueloop/queue.jsonc").exists());

    rollback_file_migration(&ctx, ".cueloop/queue.json", ".cueloop/queue.jsonc").unwrap();

    assert!(dir.path().join(".cueloop/queue.json").exists());
    assert!(!dir.path().join(".cueloop/queue.jsonc").exists());
}

#[test]
fn rollback_fails_if_original_missing() {
    let dir = TempDir::new().unwrap();
    let ctx = create_test_context(&dir);

    let result = rollback_file_migration(&ctx, ".cueloop/queue.json", ".cueloop/queue.jsonc");
    assert!(result.is_err());
}
