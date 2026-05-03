//! Task update tests.
//!
//! Purpose:
//! - Task update tests.
//!
//! Responsibilities:
//! - Verify queue-backup restore behavior for parse, validation, and save flows.
//! - Exercise update-state helpers without invoking external runners.
//!
//! Not handled here:
//! - End-to-end runner execution.
//! - CLI formatting assertions.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Test queues use local temp directories and repo-scoped `.cueloop` files.
//! - Restore-on-failure logic must preserve the pre-update queue snapshot.

use super::state::{load_validate_and_save_queue_after_update, restore_queue_from_backup};
use super::update_task;
use crate::config::Resolved;
use crate::contracts::{Config, QueueFile, Task, TaskStatus};
use crate::queue;
use crate::testsupport::git as git_test;
use crate::testsupport::runner::create_fake_runner;
use anyhow::Result;
use std::collections::HashMap;
use tempfile::TempDir;

fn task_with_timestamps(
    id: &str,
    status: TaskStatus,
    created_at: Option<&str>,
    updated_at: Option<&str>,
) -> Task {
    Task {
        id: id.to_string(),
        status,
        kind: Default::default(),
        title: "Test task".to_string(),
        description: None,
        priority: Default::default(),
        tags: vec!["tag".to_string()],
        scope: vec!["file".to_string()],
        evidence: vec!["observed".to_string()],
        plan: vec!["do thing".to_string()],
        notes: vec![],
        request: Some("test request".to_string()),
        agent: None,
        created_at: created_at.map(|s| s.to_string()),
        updated_at: updated_at.map(|s| s.to_string()),
        completed_at: None,
        started_at: None,
        scheduled_start: None,
        depends_on: vec![],
        blocks: vec![],
        relates_to: vec![],
        duplicates: None,
        custom_fields: HashMap::new(),
        estimated_minutes: None,
        actual_minutes: None,
        parent_id: None,
    }
}

fn create_test_resolved(temp: &TempDir) -> Result<Resolved> {
    let repo_root = temp.path().to_path_buf();
    let cueloop_dir = repo_root.join(".cueloop");
    std::fs::create_dir_all(&cueloop_dir)?;

    Ok(Resolved {
        config: Config::default(),
        repo_root,
        queue_path: cueloop_dir.join("queue.json"),
        done_path: cueloop_dir.join("done.jsonc"),
        id_prefix: "RQ".to_string(),
        id_width: 4,
        global_config_path: None,
        project_config_path: None,
    })
}

#[test]
fn restore_queue_from_backup_success() -> Result<()> {
    let temp = TempDir::new()?;
    let queue_path = temp.path().join("queue.json");
    let backup_path = temp.path().join("queue.json.backup");

    let original = QueueFile {
        version: 1,
        tasks: vec![task_with_timestamps(
            "RQ-0001",
            TaskStatus::Todo,
            Some("2026-01-18T00:00:00Z"),
            Some("2026-01-18T00:00:00Z"),
        )],
    };
    queue::save_queue(&queue_path, &original)?;
    queue::save_queue(&backup_path, &original)?;

    std::fs::write(&queue_path, "corrupted json")?;

    restore_queue_from_backup(&queue_path, &backup_path)?;

    let restored = queue::load_queue(&queue_path)?;
    assert_eq!(restored.tasks.len(), 1);
    assert_eq!(restored.tasks[0].id, "RQ-0001");
    Ok(())
}

#[test]
fn load_validate_and_save_queue_restores_on_parse_failure() -> Result<()> {
    let temp = TempDir::new()?;
    let resolved = create_test_resolved(&temp)?;

    let initial = QueueFile {
        version: 1,
        tasks: vec![task_with_timestamps(
            "RQ-0001",
            TaskStatus::Todo,
            Some("2026-01-18T00:00:00Z"),
            Some("2026-01-18T00:00:00Z"),
        )],
    };
    queue::save_queue(&resolved.queue_path, &initial)?;

    let backup_dir = resolved.repo_root.join(".cueloop/cache");
    let backup_path = queue::backup_queue(&resolved.queue_path, &backup_dir)?;

    std::fs::write(&resolved.queue_path, "{ not valid json }")?;

    let result = load_validate_and_save_queue_after_update(&resolved, &backup_path, 10);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("restored queue from backup"),
        "Error should mention backup restoration: {}",
        err_msg
    );

    let restored_content = std::fs::read_to_string(&resolved.queue_path)?;
    let restored: QueueFile = serde_json::from_str(&restored_content)?;
    assert_eq!(restored.tasks.len(), 1);
    assert_eq!(restored.tasks[0].id, "RQ-0001");
    Ok(())
}

#[test]
fn load_validate_and_save_queue_restores_on_validation_failure() -> Result<()> {
    let temp = TempDir::new()?;
    let resolved = create_test_resolved(&temp)?;

    let initial = QueueFile {
        version: 1,
        tasks: vec![task_with_timestamps(
            "RQ-0001",
            TaskStatus::Todo,
            Some("2026-01-18T00:00:00Z"),
            Some("2026-01-18T00:00:00Z"),
        )],
    };
    queue::save_queue(&resolved.queue_path, &initial)?;

    let backup_dir = resolved.repo_root.join(".cueloop/cache");
    let backup_path = queue::backup_queue(&resolved.queue_path, &backup_dir)?;

    std::fs::write(
        &resolved.queue_path,
        r#"{"version":1,"tasks":[{"id":"RQ-0001","title":"Test","status":"todo","tags":[],"scope":[],"evidence":[],"plan":[],"notes":[],"depends_on":[],"blocks":[],"relates_to":[],"custom_fields":{}}]}"#,
    )?;

    let result = load_validate_and_save_queue_after_update(&resolved, &backup_path, 10);

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("restored queue from backup"),
        "Error should mention backup restoration: {}",
        err_msg
    );

    let restored_content = std::fs::read_to_string(&resolved.queue_path)?;
    let restored: QueueFile = serde_json::from_str(&restored_content)?;
    assert_eq!(restored.tasks.len(), 1);
    assert_eq!(restored.tasks[0].id, "RQ-0001");
    Ok(())
}

#[test]
fn update_task_rejects_stray_non_queue_mutations() -> Result<()> {
    let temp = TempDir::new()?;
    let mut resolved = create_test_resolved(&temp)?;
    git_test::init_repo(&resolved.repo_root)?;
    std::fs::write(resolved.repo_root.join("README.md"), "# task update test\n")?;

    let initial = QueueFile {
        version: 1,
        tasks: vec![task_with_timestamps(
            "RQ-0001",
            TaskStatus::Todo,
            Some("2026-01-18T00:00:00Z"),
            Some("2026-01-18T00:00:00Z"),
        )],
    };
    queue::save_queue(&resolved.queue_path, &initial)?;
    queue::save_queue(&resolved.done_path, &QueueFile::default())?;
    git_test::commit_all(&resolved.repo_root, "init")?;

    let queue_after_path = resolved
        .repo_root
        .join(".cueloop/cache/update-queue-after.json");
    std::fs::create_dir_all(
        queue_after_path
            .parent()
            .expect("queue-after parent should exist"),
    )?;
    std::fs::write(
        &queue_after_path,
        serde_json::json!({
            "version": 1,
            "tasks": [{
                "id": "RQ-0001",
                "status": "todo",
                "title": "Updated title",
                "priority": "medium",
                "tags": ["tag"],
                "scope": ["file"],
                "evidence": ["observed"],
                "plan": ["do thing"],
                "notes": [],
                "request": "test request",
                "created_at": "2026-01-18T00:00:00Z",
                "updated_at": "2026-01-19T00:00:00Z",
                "depends_on": [],
                "blocks": [],
                "relates_to": [],
                "custom_fields": {}
            }]
        })
        .to_string(),
    )?;

    let readme_path = resolved.repo_root.join("README.md");
    let runner_script = format!(
        r#"#!/bin/sh
set -e
cat >/dev/null
cp "{queue_after}" "{queue_path}"
printf '\nstray edit\n' >> "{readme_path}"
echo '{{"type":"item.completed","item":{{"type":"agent_message","text":"updated task"}}}}'
"#,
        queue_after = queue_after_path.display(),
        queue_path = resolved.queue_path.display(),
        readme_path = readme_path.display(),
    );
    let runner_dir = TempDir::new()?;
    let runner_path = create_fake_runner(runner_dir.path(), "codex", &runner_script)?;
    resolved.config.agent.codex_bin = Some(runner_path.to_string_lossy().to_string());
    resolved.config.agent.git_revert_mode = Some(crate::contracts::GitRevertMode::Enabled);

    let settings = crate::commands::task::TaskUpdateSettings {
        fields: "scope,evidence,plan,notes,tags,depends_on".to_string(),
        runner_override: Some(crate::contracts::Runner::Codex),
        model_override: Some(crate::contracts::Model::Gpt53Codex),
        reasoning_effort_override: None,
        runner_cli_overrides: crate::contracts::RunnerCliOptionsPatch::default(),
        force: false,
        repoprompt_tool_injection: false,
        dry_run: false,
    };

    let err = update_task(&resolved, "RQ-0001", &settings)
        .expect_err("task update should fail on stray mutation");
    let message = format!("{err:#}");
    assert!(message.contains("Queue-only mutation boundary violated."));
    assert!(message.contains("README.md"));

    let queue_after = queue::load_queue(&resolved.queue_path)?;
    assert_eq!(queue_after.tasks[0].title, "Test task");
    assert_eq!(
        std::fs::read_to_string(&readme_path)?,
        "# task update test\n"
    );
    Ok(())
}

#[test]
fn load_validate_and_save_queue_succeeds_with_valid_queue() -> Result<()> {
    let temp = TempDir::new()?;
    let resolved = create_test_resolved(&temp)?;

    let initial = QueueFile {
        version: 1,
        tasks: vec![task_with_timestamps(
            "RQ-0001",
            TaskStatus::Todo,
            Some("2026-01-18T00:00:00Z"),
            Some("2026-01-18T00:00:00Z"),
        )],
    };
    queue::save_queue(&resolved.queue_path, &initial)?;

    let backup_dir = resolved.repo_root.join(".cueloop/cache");
    let backup_path = queue::backup_queue(&resolved.queue_path, &backup_dir)?;

    let updated = QueueFile {
        version: 1,
        tasks: vec![{
            let mut task = task_with_timestamps(
                "RQ-0001",
                TaskStatus::Todo,
                Some("2026-01-18T00:00:00Z"),
                Some("2026-01-19T00:00:00Z"),
            );
            task.title = "Updated title".to_string();
            task
        }],
    };
    queue::save_queue(&resolved.queue_path, &updated)?;

    let result = load_validate_and_save_queue_after_update(&resolved, &backup_path, 10);
    assert!(result.is_ok());

    let final_content = std::fs::read_to_string(&resolved.queue_path)?;
    let final_queue: QueueFile = serde_json::from_str(&final_content)?;
    assert_eq!(final_queue.tasks.len(), 1);
    assert_eq!(final_queue.tasks[0].title, "Updated title");
    Ok(())
}
