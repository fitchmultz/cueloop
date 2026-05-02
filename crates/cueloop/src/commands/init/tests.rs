//! Initialization workflow regression tests.
//!
//! Purpose:
//! - Verify `crate::commands::init` preserves file-creation and validation behavior.
//!
//! Responsibilities:
//! - Cover creation, force-overwrite, README handling, and invalid queue/done validation.
//! - Exercise wizard-driven initialization outputs.
//! - Verify new CueLoop runtime defaults and legacy CueLoop runtime compatibility.
//!
//! Scope:
//! - Unit tests for init workflow behavior only.
//!
//! Usage:
//! - Compiled when `cargo test` exercises the init command module.
//!
//! Invariants/assumptions:
//! - Tests use temporary repositories and never mutate the real workspace.
//! - The default README contract remains versioned and deterministic.

use std::fs;

use tempfile::TempDir;

use super::*;
use crate::contracts::{Config, ProjectType};

fn resolved_for(dir: &TempDir) -> crate::config::Resolved {
    let repo_root = dir.path().to_path_buf();
    let queue_path = repo_root.join(".cueloop/queue.jsonc");
    let done_path = repo_root.join(".cueloop/done.jsonc");
    let project_config_path = Some(repo_root.join(".cueloop/config.jsonc"));
    fs::create_dir_all(repo_root.join(".cueloop")).expect("create .cueloop test runtime");
    crate::config::Resolved {
        config: Config::default(),
        repo_root,
        queue_path,
        done_path,
        id_prefix: "RQ".to_string(),
        id_width: 4,
        global_config_path: None,
        project_config_path,
    }
}

#[test]
fn init_creates_missing_files() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let resolved = resolved_for(&dir);
    let report = run_init(
        &resolved,
        InitOptions {
            force: false,
            force_lock: false,
            interactive: false,
        },
    )?;
    assert_eq!(report.queue_status, FileInitStatus::Created);
    assert_eq!(report.done_status, FileInitStatus::Created);
    assert_eq!(report.config_status, FileInitStatus::Created);
    assert!(matches!(
        report.readme_status,
        Some((FileInitStatus::Created, Some(README_VERSION)))
    ));
    let queue = crate::queue::load_queue(&resolved.queue_path)?;
    assert_eq!(queue.version, 1);
    let done = crate::queue::load_queue(&resolved.done_path)?;
    assert_eq!(done.version, 1);
    let raw_cfg = std::fs::read_to_string(resolved.project_config_path.as_ref().unwrap())?;
    let cfg: Config = serde_json::from_str(&raw_cfg)?;
    assert_eq!(cfg.version, 2);
    let readme_path = resolved.repo_root.join(".cueloop/README.md");
    assert!(readme_path.exists());
    let readme_raw = std::fs::read_to_string(readme_path)?;
    assert!(readme_raw.contains("# CueLoop runtime files"));
    assert!(readme_raw.contains("cueloop init"));
    Ok(())
}

#[test]
fn init_generates_readme_with_correct_archive_command() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let resolved = resolved_for(&dir);
    run_init(
        &resolved,
        InitOptions {
            force: false,
            force_lock: false,
            interactive: false,
        },
    )?;
    let readme_path = resolved.repo_root.join(".cueloop/README.md");
    let readme_raw = std::fs::read_to_string(readme_path)?;
    assert!(
        readme_raw.contains("cueloop queue archive"),
        "README should contain 'cueloop queue archive' command"
    );
    assert!(
        !readme_raw.contains("cueloop queue done"),
        "README should NOT contain stale 'cueloop queue done' command"
    );
    Ok(())
}

#[test]
fn init_skips_existing_when_not_forced() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let resolved = resolved_for(&dir);
    let queue_json = r#"{
  "version": 1,
  "tasks": [
    {
      "id": "RQ-0001",
      "status": "todo",
      "title": "Keep",
      "tags": ["code"],
      "scope": ["x"],
      "evidence": ["y"],
      "plan": ["z"],
      "request": "test",
      "created_at": "2026-01-18T00:00:00Z",
      "updated_at": "2026-01-18T00:00:00Z"
    }
  ]
}"#;
    std::fs::write(&resolved.queue_path, queue_json)?;
    let done_json = r#"{
  "version": 1,
  "tasks": [
    {
      "id": "RQ-0002",
      "status": "done",
      "title": "Done",
      "tags": ["code"],
      "scope": ["x"],
      "evidence": ["y"],
      "plan": ["z"],
      "request": "test",
      "created_at": "2026-01-18T00:00:00Z",
      "updated_at": "2026-01-18T00:00:00Z",
      "completed_at": "2026-01-18T00:00:00Z"
    }
  ]
}"#;
    std::fs::write(&resolved.done_path, done_json)?;
    let config_json = r#"{
  "version": 2,
  "queue": {
    "file": ".cueloop/queue.jsonc"
  }
}"#;
    std::fs::write(resolved.project_config_path.as_ref().unwrap(), config_json)?;
    let report = run_init(
        &resolved,
        InitOptions {
            force: false,
            force_lock: false,
            interactive: false,
        },
    )?;
    assert_eq!(report.queue_status, FileInitStatus::Valid);
    assert_eq!(report.done_status, FileInitStatus::Valid);
    assert_eq!(report.config_status, FileInitStatus::Valid);
    assert!(matches!(
        report.readme_status,
        Some((FileInitStatus::Created, Some(README_VERSION)))
    ));
    let raw = std::fs::read_to_string(&resolved.queue_path)?;
    assert!(raw.contains("Keep"));
    let done_raw = std::fs::read_to_string(&resolved.done_path)?;
    assert!(done_raw.contains("Done"));
    Ok(())
}

#[test]
fn init_overwrites_when_forced() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let resolved = resolved_for(&dir);
    std::fs::write(&resolved.queue_path, r#"{"version":1,"tasks":[]}"#)?;
    std::fs::write(&resolved.done_path, r#"{"version":1,"tasks":[]}"#)?;
    std::fs::write(
        resolved.project_config_path.as_ref().unwrap(),
        r#"{"version":2,"project_type":"docs"}"#,
    )?;
    let report = run_init(
        &resolved,
        InitOptions {
            force: true,
            force_lock: false,
            interactive: false,
        },
    )?;
    assert_eq!(report.queue_status, FileInitStatus::Created);
    assert_eq!(report.done_status, FileInitStatus::Created);
    assert_eq!(report.config_status, FileInitStatus::Created);
    assert!(matches!(
        report.readme_status,
        Some((FileInitStatus::Created, Some(README_VERSION)))
    ));
    let cfg_raw = std::fs::read_to_string(resolved.project_config_path.as_ref().unwrap())?;
    let cfg: Config = serde_json::from_str(&cfg_raw)?;
    assert_eq!(cfg.project_type, Some(ProjectType::Code));
    assert_eq!(
        cfg.queue.file,
        Some(std::path::PathBuf::from(".cueloop/queue.jsonc"))
    );
    assert_eq!(
        cfg.queue.done_file,
        Some(std::path::PathBuf::from(".cueloop/done.jsonc"))
    );
    assert_eq!(cfg.queue.id_prefix, Some("RQ".to_string()));
    assert_eq!(cfg.queue.id_width, Some(4));
    assert_eq!(cfg.agent.runner, Some(crate::contracts::Runner::Codex));
    assert_eq!(cfg.agent.model, Some(crate::contracts::Model::Gpt54));
    assert_eq!(
        cfg.agent.reasoning_effort,
        Some(crate::contracts::ReasoningEffort::Medium)
    );
    assert_eq!(cfg.agent.iterations, Some(1));
    assert_eq!(cfg.agent.followup_reasoning_effort, None);
    assert_eq!(cfg.agent.gemini_bin, Some("gemini".to_string()));
    Ok(())
}

#[test]
fn init_creates_json_for_new_install() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let resolved = resolved_for(&dir);
    let report = run_init(
        &resolved,
        InitOptions {
            force: false,
            force_lock: false,
            interactive: false,
        },
    )?;
    assert_eq!(report.queue_status, FileInitStatus::Created);
    assert_eq!(report.done_status, FileInitStatus::Created);
    assert_eq!(report.config_status, FileInitStatus::Created);

    let queue_raw = std::fs::read_to_string(&resolved.queue_path)?;
    assert!(queue_raw.contains('{'));
    let done_raw = std::fs::read_to_string(&resolved.done_path)?;
    assert!(done_raw.contains('{'));
    let cfg_raw = std::fs::read_to_string(resolved.project_config_path.as_ref().unwrap())?;
    assert!(cfg_raw.contains('{'));
    Ok(())
}

#[test]
fn init_skips_readme_when_not_referenced() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let resolved = resolved_for(&dir);

    // Legacy prompt overrides remain readable as a fallback during the CueLoop cutover.
    let overrides = resolved.repo_root.join(".cueloop/prompts");
    fs::create_dir_all(&overrides)?;
    let prompt_files = [
        "worker.md",
        "worker_phase1.md",
        "worker_phase2.md",
        "worker_phase2_handoff.md",
        "worker_phase3.md",
        "worker_single_phase.md",
        "task_builder.md",
        "task_updater.md",
        "scan.md",
        "completion_checklist.md",
        "code_review.md",
        "phase2_handoff_checklist.md",
        "iteration_checklist.md",
    ];
    for file in prompt_files {
        fs::write(overrides.join(file), "no reference")?;
    }

    let report = run_init(
        &resolved,
        InitOptions {
            force: false,
            force_lock: false,
            interactive: false,
        },
    )?;
    assert_eq!(report.readme_status, None);
    let readme_path = resolved.repo_root.join(".cueloop/README.md");
    assert!(!readme_path.exists());
    Ok(())
}

#[test]
fn init_fails_on_invalid_existing_queue() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let resolved = resolved_for(&dir);

    let queue_json = r#"{
  "version": 1,
  "tasks": [
    {
      "id": "WRONG-0001",
      "status": "todo",
      "title": "Bad ID",
      "tags": [],
      "scope": [],
      "evidence": [],
      "plan": [],
      "request": "test",
      "created_at": "2026-01-18T00:00:00Z",
      "updated_at": "2026-01-18T00:00:00Z"
    }
  ]
}"#;
    std::fs::write(&resolved.queue_path, queue_json)?;
    std::fs::write(&resolved.done_path, r#"{"version":1,"tasks":[]}"#)?;
    std::fs::write(
        resolved.project_config_path.as_ref().unwrap(),
        r#"{"version":2,"project_type":"code"}"#,
    )?;

    let result = run_init(
        &resolved,
        InitOptions {
            force: false,
            force_lock: false,
            interactive: false,
        },
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("validate existing queue"));
    Ok(())
}

#[test]
fn init_fails_on_invalid_existing_done() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let resolved = resolved_for(&dir);

    std::fs::write(&resolved.queue_path, r#"{"version":1,"tasks":[]}"#)?;

    let done_json = r#"{
  "version": 1,
  "tasks": [
    {
      "id": "WRONG-0002",
      "status": "done",
      "title": "Bad ID",
      "tags": [],
      "scope": [],
      "evidence": [],
      "plan": [],
      "request": "test",
      "created_at": "2026-01-18T00:00:00Z",
      "updated_at": "2026-01-18T00:00:00Z"
    }
  ]
}"#;
    std::fs::write(&resolved.done_path, done_json)?;
    std::fs::write(
        resolved.project_config_path.as_ref().unwrap(),
        r#"{"version":2,"project_type":"code"}"#,
    )?;

    let result = run_init(
        &resolved,
        InitOptions {
            force: false,
            force_lock: false,
            interactive: false,
        },
    );

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("validate existing done"));
    Ok(())
}

#[test]
fn init_with_wizard_answers_creates_configured_files() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let resolved = resolved_for(&dir);

    let wizard_answers = WizardAnswers {
        runner: crate::contracts::Runner::Codex,
        model: "gpt-5.4".to_string(),
        phases: 2,
        queue_tracking_mode: crate::commands::init::QueueTrackingMode::TrackedShared,
        parallel_ignored_file_allowlist: Vec::new(),
        create_first_task: true,
        first_task_title: Some("Test task".to_string()),
        first_task_description: Some("Test description".to_string()),
        first_task_priority: crate::contracts::TaskPriority::High,
    };

    let report = run_init(
        &resolved,
        InitOptions {
            force: false,
            force_lock: false,
            interactive: false,
        },
    )?;

    writers::write_queue(
        &resolved.queue_path,
        true,
        &resolved.id_prefix,
        resolved.id_width,
        Some(&wizard_answers),
    )?;

    writers::write_config(
        resolved.project_config_path.as_ref().unwrap(),
        true,
        Some(&wizard_answers),
    )?;

    assert_eq!(report.done_status, FileInitStatus::Created);

    let cfg_raw = std::fs::read_to_string(resolved.project_config_path.as_ref().unwrap())?;
    let cfg: Config = serde_json::from_str(&cfg_raw)?;
    assert_eq!(cfg.agent.runner, Some(crate::contracts::Runner::Codex));
    assert_eq!(cfg.agent.phases, Some(2));

    let queue = crate::queue::load_queue(&resolved.queue_path)?;
    assert_eq!(queue.tasks.len(), 1);
    assert_eq!(queue.tasks[0].title, "Test task");
    assert_eq!(
        queue.tasks[0].priority,
        crate::contracts::TaskPriority::High
    );

    Ok(())
}

#[test]
fn init_updates_outdated_readme_by_default() -> anyhow::Result<()> {
    let dir = TempDir::new()?;
    let resolved = resolved_for(&dir);

    fs::create_dir_all(resolved.repo_root.join(".cueloop"))?;
    let old_readme = "<!-- CUELOOP_README_VERSION: 1 -->\n# Old content";
    fs::write(resolved.repo_root.join(".cueloop/README.md"), old_readme)?;
    fs::write(&resolved.queue_path, r#"{"version":1,"tasks":[]}"#)?;
    fs::write(&resolved.done_path, r#"{"version":1,"tasks":[]}"#)?;
    fs::write(
        resolved.project_config_path.as_ref().unwrap(),
        r#"{"version":2}"#,
    )?;

    let report = run_init(
        &resolved,
        InitOptions {
            force: false,
            force_lock: false,
            interactive: false,
        },
    )?;

    assert!(matches!(
        report.readme_status,
        Some((FileInitStatus::Updated, Some(README_VERSION)))
    ));

    let content = std::fs::read_to_string(resolved.repo_root.join(".cueloop/README.md"))?;
    assert!(!content.contains("Old content"));
    assert!(content.contains("CueLoop runtime files"));
    Ok(())
}
