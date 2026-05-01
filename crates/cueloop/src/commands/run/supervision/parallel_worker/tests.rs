//! Parallel-worker supervision unit tests.
//!
//! Purpose:
//! - Verify focused bookkeeping and marker helpers used by parallel-worker supervision.
//!
//! Responsibilities:
//! - Cover CI marker persistence behavior.
//! - Cover bookkeeping restore and bookkeeping-path filtering helpers.
//!
//! Scope:
//! - Unit tests for helper modules only; end-to-end worker regressions live in runtime tests.
//!
//! Usage:
//! - Compiled when supervision tests include the parallel-worker module.
//!
//! Invariants/assumptions:
//! - Tests use temporary git repositories and synthetic `.ralph` fixtures.
//! - Helper behavior must remain deterministic across repeated restore attempts.

use crate::contracts::Config;
use crate::testsupport::git as git_test;

use super::bookkeeping::{collect_bookkeeping_status_lines, restore_parallel_worker_bookkeeping};
use super::ci_marker::write_ci_failure_marker;

fn resolved_for_bookkeeping(
    repo_root: std::path::PathBuf,
    queue_path: std::path::PathBuf,
    done_path: std::path::PathBuf,
) -> crate::config::Resolved {
    crate::config::Resolved {
        config: Config::default(),
        repo_root,
        queue_path,
        done_path,
        id_prefix: "RQ".to_string(),
        id_width: 4,
        global_config_path: None,
        project_config_path: None,
    }
}

#[test]
fn write_ci_failure_marker_creates_expected_json_payload() {
    let temp = tempfile::TempDir::new().unwrap();

    write_ci_failure_marker(temp.path(), "RQ-1234", "CI gate failed");

    let marker_path = temp
        .path()
        .join(crate::commands::run::parallel::CI_FAILURE_MARKER_FILE);
    assert!(marker_path.exists(), "marker file should exist");

    let raw = std::fs::read_to_string(marker_path).unwrap();
    let payload: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(payload["task_id"], "RQ-1234");
    assert_eq!(payload["error"], "CI gate failed");
    assert!(payload["timestamp"].as_str().is_some());
}

#[test]
fn write_ci_failure_marker_overwrites_existing_marker_contents() {
    let temp = tempfile::TempDir::new().unwrap();
    let marker_path = temp
        .path()
        .join(crate::commands::run::parallel::CI_FAILURE_MARKER_FILE);
    std::fs::create_dir_all(marker_path.parent().unwrap()).unwrap();
    std::fs::write(&marker_path, r#"{"task_id":"RQ-0001","error":"old"}"#).unwrap();

    write_ci_failure_marker(temp.path(), "RQ-9999", "new failure");

    let raw = std::fs::read_to_string(marker_path).unwrap();
    let payload: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(payload["task_id"], "RQ-9999");
    assert_eq!(payload["error"], "new failure");
}

#[test]
fn write_ci_failure_marker_uses_fallback_when_primary_path_is_unusable() {
    let temp = tempfile::TempDir::new().unwrap();
    let primary_parent = temp.path().join(".ralph");
    std::fs::write(&primary_parent, "not-a-directory").unwrap();

    write_ci_failure_marker(temp.path(), "RQ-8888", "ci fallback");

    let fallback = temp
        .path()
        .join(crate::commands::run::parallel::CI_FAILURE_MARKER_FALLBACK_FILE);
    assert!(fallback.exists(), "fallback marker should exist");

    let raw = std::fs::read_to_string(fallback).unwrap();
    let payload: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(payload["task_id"], "RQ-8888");
    assert_eq!(payload["error"], "ci fallback");
}

#[test]
fn restore_bookkeeping_restores_custom_resolved_queue_done_paths() {
    let temp = tempfile::TempDir::new().unwrap();
    let repo_root = temp.path().join("workspace");
    std::fs::create_dir_all(repo_root.join("queue")).unwrap();
    std::fs::create_dir_all(repo_root.join("archive")).unwrap();
    std::fs::create_dir_all(repo_root.join(".ralph/cache")).unwrap();
    git_test::init_repo(&repo_root).unwrap();

    let custom_queue = repo_root.join("queue/active.jsonc");
    let custom_done = repo_root.join("archive/done.jsonc");
    let default_queue = repo_root.join(".ralph/queue.jsonc");
    let default_done = repo_root.join(".ralph/done.jsonc");
    let productivity = repo_root.join(".ralph/cache/productivity.json");
    std::fs::write(&custom_queue, "{\"version\":1,\"tasks\":[]}").unwrap();
    std::fs::write(&custom_done, "{\"version\":1,\"tasks\":[]}").unwrap();
    std::fs::write(&default_queue, "{\"default\":true}").unwrap();
    std::fs::write(&default_done, "{\"default\":true}").unwrap();
    std::fs::write(&productivity, "{\"stats\":[]}").unwrap();
    git_test::git_run(
        &repo_root,
        &[
            "add",
            "-f",
            "queue/active.jsonc",
            "archive/done.jsonc",
            ".ralph/queue.jsonc",
            ".ralph/done.jsonc",
            ".ralph/cache/productivity.json",
        ],
    )
    .unwrap();
    git_test::commit_all(&repo_root, "init custom bookkeeping").unwrap();

    std::fs::write(&custom_queue, "{\"version\":1,\"tasks\":[{\"id\":\"WQ\"}]}").unwrap();
    std::fs::write(&custom_done, "{\"version\":1,\"tasks\":[{\"id\":\"WD\"}]}").unwrap();
    std::fs::write(&default_queue, "{\"default\":false}").unwrap();
    std::fs::write(&default_done, "{\"default\":false}").unwrap();
    std::fs::write(&productivity, "{\"stats\":[\"dirty\"]}").unwrap();

    let resolved =
        resolved_for_bookkeeping(repo_root.clone(), custom_queue.clone(), custom_done.clone());

    restore_parallel_worker_bookkeeping(&resolved, "RQ-0001").unwrap();

    assert_eq!(
        std::fs::read_to_string(&custom_queue).unwrap(),
        "{\"version\":1,\"tasks\":[]}"
    );
    assert_eq!(
        std::fs::read_to_string(&custom_done).unwrap(),
        "{\"version\":1,\"tasks\":[]}"
    );
    assert_eq!(
        std::fs::read_to_string(&productivity).unwrap(),
        "{\"stats\":[]}"
    );
    assert_eq!(
        std::fs::read_to_string(&default_queue).unwrap(),
        "{\"default\":false}",
        "default queue path must not be restored when resolved queue path is custom"
    );
    assert_eq!(
        std::fs::read_to_string(&default_done).unwrap(),
        "{\"default\":false}",
        "default done path must not be restored when resolved done path is custom"
    );
}

#[test]
fn collect_bookkeeping_status_lines_matches_tracked_paths() {
    let repo_root = std::path::PathBuf::from("/repo");
    let resolved = resolved_for_bookkeeping(
        repo_root.clone(),
        repo_root.join(".ralph/queue.jsonc"),
        repo_root.join(".ralph/done.jsonc"),
    );
    let status = "\
 M .ralph/queue.jsonc
M  src/lib.rs
 R .ralph/done.jsonc -> .ralph/done-old.jsonc
?? scratch.txt
";

    let matches = collect_bookkeeping_status_lines(&resolved, status);
    assert_eq!(matches.len(), 2);
    assert!(matches[0].contains(".ralph/queue.jsonc"));
    assert!(matches[1].contains(".ralph/done.jsonc"));
}

#[test]
fn collect_bookkeeping_status_lines_matches_custom_resolved_queue_done_paths() {
    let repo_root = std::path::PathBuf::from("/repo");
    let resolved = resolved_for_bookkeeping(
        repo_root.clone(),
        repo_root.join("queue/active.jsonc"),
        repo_root.join("archive/done.jsonc"),
    );
    let status = "\
 M queue/active.jsonc\0M  archive/done.jsonc\0 M .ralph/queue.jsonc\0M  src/lib.rs\0";

    let matches = collect_bookkeeping_status_lines(&resolved, status);
    assert_eq!(matches.len(), 2);
    assert!(matches[0].contains("queue/active.jsonc"));
    assert!(matches[1].contains("archive/done.jsonc"));
}

#[test]
fn collect_bookkeeping_status_lines_ignores_non_bookkeeping_changes() {
    let repo_root = std::path::PathBuf::from("/repo");
    let resolved = resolved_for_bookkeeping(
        repo_root.clone(),
        repo_root.join(".ralph/queue.jsonc"),
        repo_root.join(".ralph/done.jsonc"),
    );
    let status = "\
M  src/lib.rs
A  docs/notes.md
?? temp.log
";

    let matches = collect_bookkeeping_status_lines(&resolved, status);
    assert!(matches.is_empty());
}

#[test]
fn collect_bookkeeping_status_lines_matches_generated_cache_paths() {
    let repo_root = std::path::PathBuf::from("/repo");
    let resolved = resolved_for_bookkeeping(
        repo_root.clone(),
        repo_root.join(".ralph/queue.jsonc"),
        repo_root.join(".ralph/done.jsonc"),
    );
    let status = "\
?? .ralph/cache/plans/RQ-0001.md
?? .ralph/cache/phase2_final/RQ-0001.md
?? .ralph/logs/parallel-debug.log
M  src/lib.rs
";

    let matches = collect_bookkeeping_status_lines(&resolved, status);
    assert_eq!(matches.len(), 3);
    assert!(matches[0].contains(".ralph/cache/plans/RQ-0001.md"));
    assert!(matches[1].contains(".ralph/cache/phase2_final/RQ-0001.md"));
    assert!(matches[2].contains(".ralph/logs/parallel-debug.log"));
}

#[test]
fn restore_bookkeeping_removes_generated_worker_cache_artifacts() {
    let temp = tempfile::TempDir::new().unwrap();
    let repo_root = temp.path().join("workspace");
    std::fs::create_dir_all(repo_root.join(".ralph/cache")).unwrap();
    git_test::init_repo(&repo_root).unwrap();

    let workspace_queue = repo_root.join(".ralph/queue.jsonc");
    let workspace_done = repo_root.join(".ralph/done.jsonc");
    let productivity = repo_root.join(".ralph/cache/productivity.json");
    std::fs::write(&workspace_queue, "{\"version\":1,\"tasks\":[]}").unwrap();
    std::fs::write(&workspace_done, "{\"version\":1,\"tasks\":[]}").unwrap();
    std::fs::write(&productivity, "{\"stats\":[]}").unwrap();
    git_test::git_run(
        &repo_root,
        &[
            "add",
            "-f",
            ".ralph/queue.jsonc",
            ".ralph/done.jsonc",
            ".ralph/cache/productivity.json",
        ],
    )
    .unwrap();
    git_test::commit_all(&repo_root, "init bookkeeping").unwrap();

    let generated_plan = repo_root.join(".ralph/cache/plans/RQ-0001.md");
    let generated_phase2 = repo_root.join(".ralph/cache/phase2_final/RQ-0001.md");
    let generated_session = repo_root.join(".ralph/cache/session.jsonc");
    let generated_logs = repo_root.join(".ralph/logs/parallel.log");
    std::fs::create_dir_all(generated_plan.parent().unwrap()).unwrap();
    std::fs::create_dir_all(generated_phase2.parent().unwrap()).unwrap();
    std::fs::create_dir_all(generated_logs.parent().unwrap()).unwrap();
    std::fs::write(&generated_plan, "plan").unwrap();
    std::fs::write(&generated_phase2, "phase2").unwrap();
    std::fs::write(&generated_session, "{\"task\":\"RQ-0001\"}").unwrap();
    std::fs::write(&generated_logs, "debug").unwrap();

    let resolved = crate::config::Resolved {
        config: Config::default(),
        repo_root: repo_root.clone(),
        queue_path: workspace_queue.clone(),
        done_path: workspace_done.clone(),
        id_prefix: "RQ".to_string(),
        id_width: 4,
        global_config_path: None,
        project_config_path: None,
    };

    restore_parallel_worker_bookkeeping(&resolved, "RQ-0001").unwrap();

    assert!(!generated_plan.exists());
    assert!(!generated_phase2.exists());
    assert!(!generated_session.exists());
    assert!(!generated_logs.exists());
}

#[test]
fn restore_bookkeeping_restores_tracked_plan_cache() {
    let temp = tempfile::TempDir::new().unwrap();
    let repo_root = temp.path().join("workspace");
    std::fs::create_dir_all(repo_root.join(".ralph/cache")).unwrap();
    git_test::init_repo(&repo_root).unwrap();

    let workspace_queue = repo_root.join(".ralph/queue.jsonc");
    let workspace_done = repo_root.join(".ralph/done.jsonc");
    let productivity = repo_root.join(".ralph/cache/productivity.json");
    std::fs::write(&workspace_queue, "{\"version\":1,\"tasks\":[]}").unwrap();
    std::fs::write(&workspace_done, "{\"version\":1,\"tasks\":[]}").unwrap();
    std::fs::write(&productivity, "{\"stats\":[]}").unwrap();
    git_test::git_run(
        &repo_root,
        &[
            "add",
            "-f",
            ".ralph/queue.jsonc",
            ".ralph/done.jsonc",
            ".ralph/cache/productivity.json",
        ],
    )
    .unwrap();
    git_test::commit_all(&repo_root, "init bookkeeping").unwrap();

    let plan_path = repo_root.join(".ralph/cache/plans/RQ-0001.md");
    std::fs::create_dir_all(plan_path.parent().unwrap()).unwrap();
    std::fs::write(&plan_path, "initial plan").unwrap();
    git_test::git_run(&repo_root, &["add", "-f", ".ralph/cache/plans/RQ-0001.md"]).unwrap();
    git_test::commit_all(&repo_root, "track plan cache").unwrap();

    std::fs::write(&plan_path, "generated plan").unwrap();

    let resolved = crate::config::Resolved {
        config: Config::default(),
        repo_root: repo_root.clone(),
        queue_path: workspace_queue.clone(),
        done_path: workspace_done.clone(),
        id_prefix: "RQ".to_string(),
        id_width: 4,
        global_config_path: None,
        project_config_path: None,
    };

    restore_parallel_worker_bookkeeping(&resolved, "RQ-0001").unwrap();

    assert_eq!(std::fs::read_to_string(&plan_path).unwrap(), "initial plan");
}
