//! Regression tests for parallel worker-event handling.
//!
//! Purpose:
//! - Keep worker-event runtime logic tests out of the production events module.
//!
//! Responsibilities:
//! - Cover blocked-task announcements, worker completion bookkeeping, and failure cleanup.
//! - Provide local fixtures for event-handling tests.
//!
//! Non-scope:
//! - Production event handling logic.
//! - Broader parallel orchestration integration tests outside finished-worker event handling.
//!
//! Usage:
//! - Included by `events.rs` under `#[cfg(test)]`.
//!
//! Invariants/Assumptions:
//! - Tests may access private items from the parent events module through `super::*`.

use super::*;
use crate::commands::run::parallel::state::{
    self, ParallelStateFile, WorkerLifecycle, WorkerRecord,
};
use crate::commands::run::parallel::worker::start_worker_monitor;
use crate::contracts::{Config, QueueFile, Task, TaskStatus};
use crate::git::WorkspaceSpec;
use crate::testsupport::git as git_test;
use anyhow::Result;
use std::process::{Child, Command, ExitStatus, Stdio};
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;

fn create_guard(temp: &TempDir, state_path: std::path::PathBuf) -> ParallelCleanupGuard {
    let workspace_root = temp.path().join("workspaces");
    std::fs::create_dir_all(&workspace_root).expect("create workspace root");
    let state_file = ParallelStateFile::new("2026-04-25T00:00:00Z".to_string(), "main".to_string());
    ParallelCleanupGuard::new_simple(state_path, state_file, workspace_root)
}

macro_rules! handle_finished_workers_for_test {
    ($finished:expr, $guard:expr, $state_path:expr, $workspace_root:expr, $resolved:expr, $target_branch:expr, $stats:expr, $queue_lock:expr $(,)?) => {{
        handle_finished_workers(
            $finished,
            $guard,
            FinishedWorkerHandlingContext {
                state_path: $state_path,
                workspace_root: $workspace_root,
                resolved: $resolved,
                target_branch: $target_branch,
                queue_lock: $queue_lock,
            },
            $stats,
        )
    }};
}

fn register_finished_worker_monitor(
    guard: &mut ParallelCleanupGuard,
    workspace: &WorkspaceSpec,
    task_id: &str,
) -> Result<()> {
    let child: Child = Command::new(std::env::current_exe()?)
        .arg("--help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let worker = start_worker_monitor(
        task_id,
        "Test task".to_string(),
        workspace.clone(),
        child,
        guard.worker_event_sender(),
    );
    guard.register_worker(task_id.to_string(), worker);
    Ok(())
}

fn worker_workspace(temp: &TempDir, task_id: &str) -> Result<WorkspaceSpec> {
    let path = temp.path().join("workspaces").join(task_id);
    std::fs::create_dir_all(&path)?;
    Ok(WorkspaceSpec {
        path,
        branch: "main".to_string(),
    })
}

fn synthetic_status(code: i32) -> ExitStatus {
    #[cfg(unix)]
    {
        ExitStatus::from_raw(code << 8)
    }
    #[cfg(windows)]
    {
        ExitStatus::from_raw(code as u32)
    }
}

fn test_resolved(repo_root: &Path) -> config::Resolved {
    config::Resolved {
        config: Config::default(),
        repo_root: repo_root.to_path_buf(),
        queue_path: repo_root.join(".cueloop/queue.jsonc"),
        done_path: repo_root.join(".cueloop/done.jsonc"),
        id_prefix: "RQ".to_string(),
        id_width: 4,
        global_config_path: None,
        project_config_path: None,
    }
}

fn task(id: &str, status: TaskStatus) -> Task {
    Task {
        id: id.to_string(),
        status,
        kind: Default::default(),
        title: format!("Task {id}"),
        created_at: Some("2026-04-27T00:00:00Z".to_string()),
        updated_at: Some("2026-04-27T00:00:00Z".to_string()),
        completed_at: (status == TaskStatus::Done).then(|| "2026-04-27T00:01:00Z".to_string()),
        ..Task::default()
    }
}

fn initialized_remote_backed_repo(temp: &TempDir) -> Result<(std::path::PathBuf, String)> {
    let remote = temp.path().join("origin.git");
    std::fs::create_dir_all(&remote)?;
    git_test::init_bare_repo(&remote)?;

    let repo_root = temp.path().join("repo");
    std::fs::create_dir_all(&repo_root)?;
    git_test::init_repo(&repo_root)?;
    std::fs::create_dir_all(repo_root.join(".cueloop"))?;
    std::fs::write(repo_root.join("README.md"), "test repo")?;
    crate::queue::save_queue(
        &repo_root.join(".cueloop/queue.jsonc"),
        &QueueFile::default(),
    )?;
    crate::queue::save_queue(
        &repo_root.join(".cueloop/done.jsonc"),
        &QueueFile::default(),
    )?;
    git_test::commit_all(&repo_root, "initial commit")?;
    let branch = git_test::git_output(&repo_root, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    git_test::add_remote(&repo_root, "origin", &remote)?;
    git_test::push_branch(&repo_root, &branch)?;
    Ok((repo_root, branch))
}

#[test]
fn finished_success_marks_completed_persists_state_and_removes_worker() -> Result<()> {
    let temp = TempDir::new()?;
    let state_path = temp.path().join("state.json");
    let (repo_root, branch) = initialized_remote_backed_repo(&temp)?;
    let mut guard = create_guard(&temp, state_path.clone());
    let workspace = worker_workspace(&temp, "RQ-0006")?;
    register_finished_worker_monitor(&mut guard, &workspace, "RQ-0006")?;
    guard.state_file_mut().upsert_worker(WorkerRecord::new(
        "RQ-0006",
        workspace.path.clone(),
        "2026-04-25T00:00:00Z".to_string(),
    ));

    let queue_lock = crate::queue::acquire_queue_lock(&repo_root, "test", false)?;
    let mut stats = ParallelRunStats::default();
    handle_finished_workers_for_test!(
        vec![crate::commands::run::parallel::worker::FinishedWorker {
            task_id: "RQ-0006".to_string(),
            task_title: "Task".to_string(),
            workspace: workspace.clone(),
            status: synthetic_status(0),
        }],
        &mut guard,
        &state_path,
        &temp.path().join("workspaces"),
        &test_resolved(&repo_root),
        &branch,
        &mut stats,
        &queue_lock,
    )?;

    let saved = state::load_state(&state_path)?.expect("saved state");
    let worker = saved.get_worker("RQ-0006").expect("worker record");
    assert_eq!(worker.lifecycle, WorkerLifecycle::Completed);
    assert_eq!(stats.succeeded(), 1);
    assert!(guard.in_flight().is_empty());
    Ok(())
}

#[test]
fn finished_blocked_push_retains_workspace_and_records_attempts() -> Result<()> {
    let temp = TempDir::new()?;
    let state_path = temp.path().join("state.json");
    let mut guard = create_guard(&temp, state_path.clone());
    let workspace = worker_workspace(&temp, "RQ-0006")?;
    register_finished_worker_monitor(&mut guard, &workspace, "RQ-0006")?;
    guard.state_file_mut().upsert_worker(WorkerRecord::new(
        "RQ-0006",
        workspace.path.clone(),
        "2026-04-25T00:00:00Z".to_string(),
    ));

    let marker_path = workspace
        .path
        .join(".cueloop/cache/parallel/blocked_push.json");
    std::fs::create_dir_all(marker_path.parent().expect("marker parent"))?;
    std::fs::write(
        &marker_path,
        serde_json::json!({
            "task_id": "RQ-0006",
            "reason": "blocked by integration",
            "attempt": 3,
            "max_attempts": 5,
            "generated_at": "2026-04-25T00:01:00Z"
        })
        .to_string(),
    )?;

    let queue_lock = crate::queue::acquire_queue_lock(temp.path(), "test", false)?;
    let mut stats = ParallelRunStats::default();
    handle_finished_workers_for_test!(
        vec![crate::commands::run::parallel::worker::FinishedWorker {
            task_id: "RQ-0006".to_string(),
            task_title: "Task".to_string(),
            workspace: workspace.clone(),
            status: synthetic_status(1),
        }],
        &mut guard,
        &state_path,
        &temp.path().join("workspaces"),
        &test_resolved(temp.path()),
        "main",
        &mut stats,
        &queue_lock,
    )?;

    let saved = state::load_state(&state_path)?.expect("saved state");
    let worker = saved.get_worker("RQ-0006").expect("worker record");
    assert_eq!(worker.lifecycle, WorkerLifecycle::BlockedPush);
    assert_eq!(worker.push_attempts, 3);
    assert_eq!(worker.last_error.as_deref(), Some("blocked by integration"));
    assert!(workspace.path.exists());
    assert_eq!(stats.failed(), 1);
    Ok(())
}

#[test]
fn finished_failure_without_block_marker_cleans_workspace() -> Result<()> {
    let temp = TempDir::new()?;
    let state_path = temp.path().join("state.json");
    let mut guard = create_guard(&temp, state_path.clone());
    let workspace = worker_workspace(&temp, "RQ-0006")?;
    register_finished_worker_monitor(&mut guard, &workspace, "RQ-0006")?;
    guard.state_file_mut().upsert_worker(WorkerRecord::new(
        "RQ-0006",
        workspace.path.clone(),
        "2026-04-25T00:00:00Z".to_string(),
    ));

    let queue_lock = crate::queue::acquire_queue_lock(temp.path(), "test", false)?;
    let mut stats = ParallelRunStats::default();
    handle_finished_workers_for_test!(
        vec![crate::commands::run::parallel::worker::FinishedWorker {
            task_id: "RQ-0006".to_string(),
            task_title: "Task".to_string(),
            workspace: workspace.clone(),
            status: synthetic_status(9),
        }],
        &mut guard,
        &state_path,
        &temp.path().join("workspaces"),
        &test_resolved(temp.path()),
        "main",
        &mut stats,
        &queue_lock,
    )?;

    let saved = state::load_state(&state_path)?.expect("saved state");
    let worker = saved.get_worker("RQ-0006").expect("worker record");
    assert_eq!(worker.lifecycle, WorkerLifecycle::Failed);
    assert!(!workspace.path.exists());
    assert_eq!(stats.failed(), 1);
    Ok(())
}

#[test]
fn finished_worker_state_save_failure_keeps_guard_tracking() -> Result<()> {
    let temp = TempDir::new()?;
    let bad_state_path = temp.path().join("state-dir");
    let (repo_root, branch) = initialized_remote_backed_repo(&temp)?;
    std::fs::create_dir_all(&bad_state_path)?;
    let mut guard = create_guard(&temp, bad_state_path.clone());
    let workspace = worker_workspace(&temp, "RQ-0006")?;
    register_finished_worker_monitor(&mut guard, &workspace, "RQ-0006")?;
    guard.state_file_mut().upsert_worker(WorkerRecord::new(
        "RQ-0006",
        workspace.path.clone(),
        "2026-04-25T00:00:00Z".to_string(),
    ));

    let queue_lock = crate::queue::acquire_queue_lock(&repo_root, "test", false)?;
    let mut stats = ParallelRunStats::default();
    let err = handle_finished_workers_for_test!(
        vec![crate::commands::run::parallel::worker::FinishedWorker {
            task_id: "RQ-0006".to_string(),
            task_title: "Task".to_string(),
            workspace: workspace.clone(),
            status: synthetic_status(0),
        }],
        &mut guard,
        &bad_state_path,
        &temp.path().join("workspaces"),
        &test_resolved(&repo_root),
        &branch,
        &mut stats,
        &queue_lock,
    )
    .expect_err("state save should fail");

    assert!(err.to_string().contains("write parallel state"));
    assert!(guard.in_flight().contains_key("RQ-0006"));
    Ok(())
}

#[test]
fn finished_success_surfaces_branch_refresh_failure() -> Result<()> {
    let temp = TempDir::new()?;
    let state_path = temp.path().join("state.json");
    let mut guard = create_guard(&temp, state_path.clone());
    let workspace = worker_workspace(&temp, "RQ-0006")?;
    register_finished_worker_monitor(&mut guard, &workspace, "RQ-0006")?;
    guard.state_file_mut().upsert_worker(WorkerRecord::new(
        "RQ-0006",
        workspace.path.clone(),
        "2026-04-25T00:00:00Z".to_string(),
    ));

    let queue_lock = crate::queue::acquire_queue_lock(temp.path(), "test", false)?;
    let mut stats = ParallelRunStats::default();
    let err = handle_finished_workers_for_test!(
        vec![crate::commands::run::parallel::worker::FinishedWorker {
            task_id: "RQ-0006".to_string(),
            task_title: "Task".to_string(),
            workspace,
            status: synthetic_status(0),
        }],
        &mut guard,
        &state_path,
        &temp.path().join("workspaces"),
        &test_resolved(&temp.path().join("not-a-git-repo")),
        "main",
        &mut stats,
        &queue_lock,
    )
    .expect_err("coordinator refresh failure should block successful worker completion");

    assert!(
        err.to_string().contains("local branch refresh"),
        "unexpected error: {err:#}"
    );
    assert_eq!(stats.succeeded(), 0);
    assert!(guard.in_flight().contains_key("RQ-0006"));
    Ok(())
}

#[test]
fn finished_success_errors_when_tracked_bookkeeping_refresh_cannot_fast_forward() -> Result<()> {
    let temp = TempDir::new()?;
    let remote = temp.path().join("origin.git");
    std::fs::create_dir_all(&remote)?;
    git_test::init_bare_repo(&remote)?;

    let repo_root = temp.path().join("repo");
    std::fs::create_dir_all(repo_root.join(".cueloop"))?;
    git_test::init_repo(&repo_root)?;
    std::fs::write(repo_root.join(".cueloop/queue.jsonc"), "{stale_queue}")?;
    std::fs::write(repo_root.join(".cueloop/done.jsonc"), "{stale_done}")?;
    git_test::commit_all(&repo_root, "tracked stale bookkeeping")?;
    let branch = git_test::git_output(&repo_root, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    git_test::add_remote(&repo_root, "origin", &remote)?;
    git_test::push_branch(&repo_root, &branch)?;

    std::fs::write(
        repo_root.join(".cueloop/queue.jsonc"),
        "{remote_fresh_queue}",
    )?;
    std::fs::write(repo_root.join(".cueloop/done.jsonc"), "{remote_fresh_done}")?;
    git_test::commit_all(&repo_root, "remote fresh bookkeeping")?;
    git_test::push_branch(&repo_root, &branch)?;
    git_test::git_run(&repo_root, &["reset", "--hard", "HEAD~1"])?;
    std::fs::write(repo_root.join("local-only.txt"), "local divergence")?;
    git_test::commit_all(&repo_root, "local divergent commit")?;

    let state_path = temp.path().join("state.json");
    let mut guard = create_guard(&temp, state_path.clone());
    let workspace = worker_workspace(&temp, "RQ-0006")?;
    std::fs::create_dir_all(workspace.path.join(".cueloop"))?;
    std::fs::write(
        workspace.path.join(".cueloop/queue.jsonc"),
        "{workspace_queue}",
    )?;
    std::fs::write(
        workspace.path.join(".cueloop/done.jsonc"),
        "{workspace_done}",
    )?;
    register_finished_worker_monitor(&mut guard, &workspace, "RQ-0006")?;
    guard.state_file_mut().upsert_worker(WorkerRecord::new(
        "RQ-0006",
        workspace.path.clone(),
        "2026-04-25T00:00:00Z".to_string(),
    ));

    let queue_lock = crate::queue::acquire_queue_lock(&repo_root, "test", false)?;
    let mut stats = ParallelRunStats::default();
    let err = handle_finished_workers_for_test!(
        vec![crate::commands::run::parallel::worker::FinishedWorker {
            task_id: "RQ-0006".to_string(),
            task_title: "Task".to_string(),
            workspace,
            status: synthetic_status(0),
        }],
        &mut guard,
        &state_path,
        &temp.path().join("workspaces"),
        &test_resolved(&repo_root),
        &branch,
        &mut stats,
        &queue_lock,
    )
    .expect_err("non-fast-forward tracked bookkeeping refresh must surface");

    assert!(
        err.to_string().contains("local branch refresh"),
        "unexpected error: {err:#}"
    );
    assert_eq!(
        std::fs::read_to_string(repo_root.join(".cueloop/queue.jsonc"))?,
        "{stale_queue}"
    );
    assert_eq!(
        std::fs::read_to_string(repo_root.join(".cueloop/done.jsonc"))?,
        "{stale_done}"
    );
    assert_eq!(stats.succeeded(), 0);
    assert!(guard.in_flight().contains_key("RQ-0006"));
    Ok(())
}

#[test]
fn finished_success_reconciles_ignored_source_bookkeeping_from_worker_done_task() -> Result<()> {
    let temp = TempDir::new()?;
    let repo_root = temp.path().join("repo");
    std::fs::create_dir_all(repo_root.join(".cueloop"))?;
    git_test::init_repo(&repo_root)?;
    std::fs::write(
        repo_root.join(".gitignore"),
        ".cueloop/queue.jsonc\n.cueloop/done.jsonc\n.cueloop/cache/\n",
    )?;
    git_test::git_run(&repo_root, &["add", ".gitignore"])?;
    git_test::git_run(&repo_root, &["commit", "-m", "ignore bookkeeping"])?;
    let branch = git_test::git_output(&repo_root, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let remote = temp.path().join("origin.git");
    std::fs::create_dir_all(&remote)?;
    git_test::init_bare_repo(&remote)?;
    git_test::add_remote(&repo_root, "origin", &remote)?;
    git_test::push_branch(&repo_root, &branch)?;

    let resolved = test_resolved(&repo_root);
    crate::queue::save_queue(
        &resolved.queue_path,
        &QueueFile {
            version: 1,
            tasks: vec![task("RQ-0006", TaskStatus::Todo)],
        },
    )?;
    crate::queue::save_queue(&resolved.done_path, &QueueFile::default())?;

    let state_path = temp.path().join("state.json");
    let mut guard = create_guard(&temp, state_path.clone());
    let workspace = worker_workspace(&temp, "RQ-0006")?;
    std::fs::create_dir_all(workspace.path.join(".cueloop"))?;
    crate::queue::save_queue(
        &workspace.path.join(".cueloop/done.jsonc"),
        &QueueFile {
            version: 1,
            tasks: vec![task("RQ-0006", TaskStatus::Done)],
        },
    )?;
    register_finished_worker_monitor(&mut guard, &workspace, "RQ-0006")?;
    guard.state_file_mut().upsert_worker(WorkerRecord::new(
        "RQ-0006",
        workspace.path.clone(),
        "2026-04-25T00:00:00Z".to_string(),
    ));

    let queue_lock = crate::queue::acquire_queue_lock(&repo_root, "test", false)?;
    let mut stats = ParallelRunStats::default();
    handle_finished_workers_for_test!(
        vec![crate::commands::run::parallel::worker::FinishedWorker {
            task_id: "RQ-0006".to_string(),
            task_title: "Task".to_string(),
            workspace,
            status: synthetic_status(0),
        }],
        &mut guard,
        &state_path,
        &temp.path().join("workspaces"),
        &resolved,
        &branch,
        &mut stats,
        &queue_lock,
    )?;

    assert!(
        crate::queue::load_queue(&resolved.queue_path)?
            .tasks
            .is_empty()
    );
    let done = crate::queue::load_queue(&resolved.done_path)?;
    assert_eq!(done.tasks.len(), 1);
    assert_eq!(done.tasks[0].id, "RQ-0006");
    assert_eq!(stats.succeeded(), 1);
    Ok(())
}
