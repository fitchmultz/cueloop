//! Purpose: Preserve regression coverage for template loading, listing,
//! validation, and context-aware substitution after the facade split.
//!
//! Responsibilities:
//! - Verify built-in/custom template precedence and listing behavior.
//! - Verify validation warnings, strict-mode failures, and substitution.
//! - Keep the former inline `template::loader` test coverage intact.
//!
//! Scope:
//! - Loader-specific behavior only; built-in template contents and merge logic
//!   stay covered elsewhere.
//!
//! Usage:
//! - Runs as the `template::loader` unit test suite.
//!
//! Invariants/Assumptions:
//! - Assertions remain aligned with the former monolithic test block.
//! - Template precedence, warning semantics, and substitution behavior remain
//!   unchanged.

use std::io::Write;

use tempfile::TempDir;

use super::list::{list_templates, template_exists};
use super::load::{load_template, load_template_with_context};
use super::types::TemplateSource;
use crate::template::variables::TemplateWarning;

fn create_test_project() -> TempDir {
    TempDir::new().expect("Failed to create temp dir")
}

#[test]
fn test_load_builtin_template() {
    let temp_dir = create_test_project();
    let result = load_template("bug", temp_dir.path());
    assert!(result.is_ok());

    let (task, source) = result.unwrap();
    assert_eq!(task.priority, crate::contracts::TaskPriority::High);
    assert!(matches!(source, TemplateSource::Builtin(s) if s == "bug"));
}

#[test]
fn test_load_custom_template() {
    let temp_dir = create_test_project();
    let templates_dir = temp_dir.path().join(".cueloop/templates");
    std::fs::create_dir_all(&templates_dir).unwrap();

    let custom_template = r#"{
            "id": "",
            "title": "",
            "status": "todo",
            "priority": "critical",
            "tags": ["custom", "test"],
            "plan": ["Step 1", "Step 2"]
        }"#;

    let mut file = std::fs::File::create(templates_dir.join("custom.json")).unwrap();
    file.write_all(custom_template.as_bytes()).unwrap();

    let result = load_template("custom", temp_dir.path());
    assert!(result.is_ok());

    let (task, source) = result.unwrap();
    assert_eq!(task.priority, crate::contracts::TaskPriority::Critical);
    assert!(matches!(source, TemplateSource::Custom(_)));
}

#[test]
fn test_custom_overrides_builtin() {
    let temp_dir = create_test_project();
    let templates_dir = temp_dir.path().join(".cueloop/templates");
    std::fs::create_dir_all(&templates_dir).unwrap();

    let custom_template = r#"{
            "id": "",
            "title": "",
            "status": "todo",
            "priority": "low",
            "tags": ["custom-bug"]
        }"#;

    let mut file = std::fs::File::create(templates_dir.join("bug.json")).unwrap();
    file.write_all(custom_template.as_bytes()).unwrap();

    let result = load_template("bug", temp_dir.path());
    assert!(result.is_ok());

    let (task, source) = result.unwrap();
    assert_eq!(task.priority, crate::contracts::TaskPriority::Low);
    assert!(matches!(source, TemplateSource::Custom(_)));
}

#[test]
fn test_load_nonexistent_template() {
    let temp_dir = create_test_project();
    let result = load_template("nonexistent", temp_dir.path());
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("not found") || err_msg.contains("NotFound"));
}

#[test]
fn test_list_templates() {
    let temp_dir = create_test_project();
    let templates_dir = temp_dir.path().join(".cueloop/templates");
    std::fs::create_dir_all(&templates_dir).unwrap();

    let custom_template = r#"{"title": "", "priority": "low"}"#;
    let mut file = std::fs::File::create(templates_dir.join("custom.json")).unwrap();
    file.write_all(custom_template.as_bytes()).unwrap();

    let templates = list_templates(temp_dir.path());

    assert_eq!(templates.len(), 11);
    assert!(templates.iter().any(|t| t.name == "custom"));
    assert!(templates.iter().any(|t| t.name == "bug"));
    assert!(templates.iter().any(|t| t.name == "feature"));
}

#[test]
fn test_template_exists() {
    let temp_dir = create_test_project();

    assert!(template_exists("bug", temp_dir.path()));
    assert!(template_exists("feature", temp_dir.path()));
    assert!(!template_exists("nonexistent", temp_dir.path()));

    let templates_dir = temp_dir.path().join(".cueloop/templates");
    std::fs::create_dir_all(&templates_dir).unwrap();
    let mut file = std::fs::File::create(templates_dir.join("custom.json")).unwrap();
    file.write_all(b"{}").unwrap();

    assert!(template_exists("custom", temp_dir.path()));
}

#[test]
fn test_load_template_with_context_substitutes_variables() {
    let temp_dir = create_test_project();
    let templates_dir = temp_dir.path().join(".cueloop/templates");
    std::fs::create_dir_all(&templates_dir).unwrap();

    let custom_template = r#"{
            "id": "",
            "title": "Fix {{target}}",
            "status": "todo",
            "priority": "high",
            "tags": ["bug", "{{module}}"],
            "scope": ["{{target}}"],
            "plan": ["Analyze {{file}}"],
            "evidence": ["Issue in {{target}}"]
        }"#;

    let mut file = std::fs::File::create(templates_dir.join("bug.json")).unwrap();
    file.write_all(custom_template.as_bytes()).unwrap();

    let result = load_template_with_context("bug", temp_dir.path(), Some("src/cli/task.rs"), false);
    assert!(result.is_ok());

    let loaded = result.unwrap();
    assert_eq!(loaded.task.title, "Fix src/cli/task.rs");
    assert!(loaded.task.tags.contains(&"bug".to_string()));
    assert!(loaded.task.tags.contains(&"cli::task".to_string()));
    assert!(loaded.task.scope.contains(&"src/cli/task.rs".to_string()));
    assert!(loaded.task.plan.contains(&"Analyze task.rs".to_string()));
    assert!(
        loaded
            .task
            .evidence
            .contains(&"Issue in src/cli/task.rs".to_string())
    );
}

#[test]
fn test_load_template_with_context_no_target() {
    let temp_dir = create_test_project();

    let result = load_template_with_context("bug", temp_dir.path(), None, false);
    assert!(result.is_ok());

    let loaded = result.unwrap();
    assert!(loaded.task.title.contains("{{target}}") || loaded.task.title.is_empty());
}

#[test]
fn test_load_template_with_context_returns_warnings() {
    let temp_dir = create_test_project();
    let templates_dir = temp_dir.path().join(".cueloop/templates");
    std::fs::create_dir_all(&templates_dir).unwrap();

    let custom_template = r#"{
            "id": "",
            "title": "Fix {{target}} with {{unknown_var}}",
            "status": "todo",
            "priority": "high",
            "tags": ["bug"]
        }"#;

    let mut file = std::fs::File::create(templates_dir.join("custom.json")).unwrap();
    file.write_all(custom_template.as_bytes()).unwrap();

    let result = load_template_with_context("custom", temp_dir.path(), Some("src/main.rs"), false);
    assert!(result.is_ok());

    let loaded = result.unwrap();
    assert!(!loaded.warnings.is_empty());
    assert!(loaded.warnings.iter().any(|w| matches!(
        w,
        TemplateWarning::UnknownVariable { name, .. } if name == "unknown_var"
    )));
}

#[test]
fn test_load_template_strict_mode_fails_on_unknown() {
    let temp_dir = create_test_project();
    let templates_dir = temp_dir.path().join(".cueloop/templates");
    std::fs::create_dir_all(&templates_dir).unwrap();

    let custom_template = r#"{
            "id": "",
            "title": "Fix {{unknown_var}}",
            "status": "todo",
            "priority": "high",
            "tags": ["bug"]
        }"#;

    let mut file = std::fs::File::create(templates_dir.join("custom.json")).unwrap();
    file.write_all(custom_template.as_bytes()).unwrap();

    let result = load_template_with_context("custom", temp_dir.path(), Some("src/main.rs"), true);
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("unknown_var"));
}

#[test]
fn test_load_template_strict_mode_succeeds_when_no_unknown() {
    let temp_dir = create_test_project();

    let result = load_template_with_context("bug", temp_dir.path(), Some("src/main.rs"), true);
    assert!(result.is_ok());
}

#[test]
fn test_load_template_with_context_git_warning() {
    let temp_dir = create_test_project();
    let templates_dir = temp_dir.path().join(".cueloop/templates");
    std::fs::create_dir_all(&templates_dir).unwrap();

    let custom_template = r#"{
            "id": "",
            "title": "Fix on branch {{branch}}",
            "status": "todo",
            "priority": "high",
            "tags": ["bug"]
        }"#;

    let mut file = std::fs::File::create(templates_dir.join("custom.json")).unwrap();
    file.write_all(custom_template.as_bytes()).unwrap();

    std::fs::create_dir_all(temp_dir.path().join(".git")).unwrap();
    std::fs::write(
        temp_dir.path().join(".git/HEAD"),
        "invalid: refs/heads/nonexistent",
    )
    .unwrap();

    let result = load_template_with_context("custom", temp_dir.path(), None, false);
    assert!(result.is_ok());

    let loaded = result.unwrap();
    assert!(
        loaded
            .warnings
            .iter()
            .any(|w| matches!(w, TemplateWarning::GitBranchDetectionFailed { .. }))
    );
}

#[test]
fn test_load_template_with_context_no_git_warning_when_no_branch_var() {
    let temp_dir = create_test_project();
    let templates_dir = temp_dir.path().join(".cueloop/templates");
    std::fs::create_dir_all(&templates_dir).unwrap();

    let custom_template = r#"{
            "id": "",
            "title": "Fix {{target}}",
            "status": "todo",
            "priority": "high",
            "tags": ["bug"]
        }"#;

    let mut file = std::fs::File::create(templates_dir.join("custom.json")).unwrap();
    file.write_all(custom_template.as_bytes()).unwrap();

    let result = load_template_with_context("custom", temp_dir.path(), Some("src/main.rs"), false);
    assert!(result.is_ok());

    let loaded = result.unwrap();
    assert!(
        !loaded
            .warnings
            .iter()
            .any(|w| matches!(w, TemplateWarning::GitBranchDetectionFailed { .. }))
    );
}

#[test]
fn test_load_custom_template_with_unknown_variable_logs_warning() {
    let temp_dir = create_test_project();
    let templates_dir = temp_dir.path().join(".cueloop/templates");
    std::fs::create_dir_all(&templates_dir).unwrap();

    let custom_template = r#"{
            "id": "",
            "title": "Fix {{typo_target}}",
            "status": "todo",
            "priority": "high"
        }"#;

    let mut file = std::fs::File::create(templates_dir.join("custom.json")).unwrap();
    file.write_all(custom_template.as_bytes()).unwrap();

    let result = load_template("custom", temp_dir.path());
    assert!(result.is_ok());

    let (task, _) = result.unwrap();
    assert_eq!(task.title, "Fix {{typo_target}}");
}

#[test]
fn test_load_custom_template_with_known_variables_succeeds() {
    let temp_dir = create_test_project();
    let templates_dir = temp_dir.path().join(".cueloop/templates");
    std::fs::create_dir_all(&templates_dir).unwrap();

    let custom_template = r#"{
            "id": "",
            "title": "Fix {{target}} in {{file}}",
            "status": "todo",
            "priority": "high"
        }"#;

    let mut file = std::fs::File::create(templates_dir.join("custom.json")).unwrap();
    file.write_all(custom_template.as_bytes()).unwrap();

    let result = load_template("custom", temp_dir.path());
    assert!(result.is_ok());
}
