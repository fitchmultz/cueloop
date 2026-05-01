//! Prompt management tests.
//!
//! Purpose:
//! - Prompt management tests.
//!
//! Responsibilities:
//! - Cover stable digest computation, schema cutover behavior, and prompt export/sync helpers.
//!
//! Not handled here:
//! - Prompt rendering tests defined elsewhere in `prompts_internal/tests/`.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Tests use isolated temp directories.

use super::*;
use crate::prompts_internal::registry::PromptTemplateId;
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

#[test]
fn compute_hash_is_stable_sha256() {
    let hash1 = compute_hash("Hello, World!");
    let hash2 = compute_hash("Hello, World!");
    assert_eq!(hash1, hash2);
    assert!(hash1.starts_with("sha256:"));
}

#[test]
fn compute_hash_trims_trailing_whitespace() {
    assert_eq!(compute_hash("Hello"), compute_hash("Hello\n\n  \n"));
}

#[test]
fn compute_hash_changes_with_content() {
    assert_ne!(compute_hash("Hello"), compute_hash("World"));
}

#[test]
fn export_template_writes_digest_header_and_version_info() {
    let temp = TempDir::new().unwrap();
    let written = export_template(temp.path(), PromptTemplateId::Worker, false, "0.5.0").unwrap();
    assert!(written);

    let content = fs::read_to_string(temp.path().join(".cueloop/prompts/worker.md")).unwrap();
    assert!(content.contains("Exported from CueLoop embedded defaults"));
    assert!(content.contains("Digest: sha256:"));

    let info = load_version_info(temp.path()).unwrap().unwrap();
    assert_eq!(info.schema_version, PROMPT_VERSION_SCHEMA);
    assert_eq!(
        info.templates.get("worker").unwrap().digest,
        compute_hash(get_embedded_content(PromptTemplateId::Worker))
    );
}

#[test]
fn check_sync_status_reports_missing_and_up_to_date() {
    let temp = TempDir::new().unwrap();
    assert_eq!(
        check_sync_status(temp.path(), PromptTemplateId::Worker).unwrap(),
        SyncStatus::Missing
    );

    export_template(temp.path(), PromptTemplateId::Worker, false, "0.5.0").unwrap();
    assert_eq!(
        check_sync_status(temp.path(), PromptTemplateId::Worker).unwrap(),
        SyncStatus::UpToDate
    );
}

#[test]
fn current_override_precedes_legacy_override_in_inventory_and_content() {
    let temp = TempDir::new().unwrap();
    let current = temp.path().join(".cueloop/prompts");
    let legacy = temp.path().join(".ralph/prompts");
    fs::create_dir_all(&current).unwrap();
    fs::create_dir_all(&legacy).unwrap();
    fs::write(current.join("worker.md"), "current").unwrap();
    fs::write(legacy.join("worker.md"), "legacy").unwrap();

    let templates = list_templates(temp.path());
    let worker = templates
        .iter()
        .find(|template| template.name == "worker")
        .unwrap();
    assert!(worker.has_override);
    assert_eq!(
        get_effective_content(temp.path(), PromptTemplateId::Worker).unwrap(),
        "current"
    );
}

#[test]
fn legacy_override_counts_as_fallback_override() {
    let temp = TempDir::new().unwrap();
    let legacy = temp.path().join(".ralph/prompts");
    fs::create_dir_all(&legacy).unwrap();
    fs::write(legacy.join("worker.md"), "legacy").unwrap();

    let templates = list_templates(temp.path());
    let worker = templates
        .iter()
        .find(|template| template.name == "worker")
        .unwrap();
    assert!(worker.has_override);
    assert_eq!(
        get_effective_content(temp.path(), PromptTemplateId::Worker).unwrap(),
        "legacy"
    );
}

#[test]
fn export_template_preserves_legacy_override_without_force() {
    let temp = TempDir::new().unwrap();
    let legacy = temp.path().join(".ralph/prompts");
    fs::create_dir_all(&legacy).unwrap();
    fs::write(legacy.join("worker.md"), "legacy").unwrap();

    let written = export_template(temp.path(), PromptTemplateId::Worker, false, "0.5.0").unwrap();

    assert!(!written);
    assert!(!temp.path().join(".cueloop/prompts/worker.md").exists());
    assert_eq!(
        get_effective_content(temp.path(), PromptTemplateId::Worker).unwrap(),
        "legacy"
    );
}

#[test]
fn load_version_info_ignores_legacy_schema_during_cutover() {
    let temp = TempDir::new().unwrap();
    let cache_dir = temp.path().join(".cueloop/cache");
    fs::create_dir_all(&cache_dir).unwrap();
    fs::write(
        cache_dir.join("prompt_versions.json"),
        r#"{
  "ralph_version": "0.5.0",
  "exported_at": "2026-01-28T22:30:00Z",
  "templates": {
    "worker": {
      "hash": "hash:legacy",
      "exported_at": "2026-01-28T22:30:00Z"
    }
  }
}"#,
    )
    .unwrap();

    assert!(load_version_info(temp.path()).unwrap().is_none());
}

#[test]
fn version_info_round_trip_uses_digest_field() {
    let temp = TempDir::new().unwrap();
    let info = PromptVersionInfo {
        schema_version: PROMPT_VERSION_SCHEMA,
        ralph_version: "0.5.0".to_string(),
        exported_at: "2026-01-28T22:30:00Z".to_string(),
        templates: {
            let mut templates = HashMap::new();
            templates.insert(
                "worker".to_string(),
                TemplateVersion {
                    digest: "sha256:abc123".to_string(),
                    exported_at: "2026-01-28T22:30:00Z".to_string(),
                },
            );
            templates
        },
    };

    save_version_info(temp.path(), &info).unwrap();
    let loaded = load_version_info(temp.path()).unwrap().unwrap();
    assert_eq!(loaded.schema_version, PROMPT_VERSION_SCHEMA);
    assert_eq!(
        loaded.templates.get("worker").unwrap().digest,
        "sha256:abc123"
    );
}

#[test]
fn parse_template_name_and_inventory_still_work() {
    assert_eq!(
        parse_template_name("worker-phase1"),
        Some(PromptTemplateId::WorkerPhase1)
    );
    assert_eq!(all_template_ids().len(), 19);

    let temp = TempDir::new().unwrap();
    let templates = list_templates(temp.path());
    assert!(templates.iter().any(|template| template.name == "worker"));
}
