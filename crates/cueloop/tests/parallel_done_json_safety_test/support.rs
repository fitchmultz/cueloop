//! Purpose: suite-local fixtures and assertions for `parallel_done_json_safety_test`.
//!
//! Responsibilities:
//! - Create disposable git+`.ralph/` repos from the cached integration-test scaffold.
//! - Keep a bare `origin` remote alive for the full test so push/fetch assertions are real.
//! - Centralize repeated parallel-run task fixtures, runner configuration, and git plumbing.
//!
//! Scope:
//! - Helpers used only by `crates/cueloop/tests/parallel_done_json_safety_test.rs`.
//!
//! Usage:
//! - Call `ParallelDoneJsonRepo::new()` to create a seeded repo with a live remote.
//! - Call `seed_parallel_fixture()` before `run_parallel()` to write the shared queue/config setup.
//! - Use the read helpers to inspect queue, done, and parallel-state artifacts after the run.
//!
//! Invariants/Assumptions:
//! - The repo scaffold comes from cached test templates; these helpers must not call real `cueloop init`.
//! - The fake runner uses an explicit configured binary path, so PATH mutation is unnecessary.
//! - The bare `origin` tempdir must stay owned by the fixture for the entire test lifetime.

use anyhow::{Context, Result, ensure};
use cueloop::contracts::{Task, TaskStatus};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use tempfile::TempDir;

const DEFAULT_RUNNER: &str = "opencode";
const DEFAULT_MODEL: &str = "test-model";
const FIXTURE_COMMIT_MESSAGE: &str = "Add tasks for parallel test";
const PARALLEL_ARGS: &[&str] = &[
    "run",
    "loop",
    "--parallel",
    "2",
    "--max-tasks",
    "2",
    "--force",
];

pub(super) struct ParallelDoneJsonRepo {
    dir: TempDir,
    _origin: TempDir,
}

impl ParallelDoneJsonRepo {
    pub(super) fn new() -> Result<Self> {
        let dir = super::test_support::temp_dir_outside_repo();
        super::test_support::seed_git_repo_with_ralph(dir.path())?;

        let origin = super::test_support::temp_dir_outside_repo();
        init_bare_remote(origin.path())?;
        add_origin_remote(dir.path(), origin.path())?;
        push_origin_head(dir.path())?;

        Ok(Self {
            dir,
            _origin: origin,
        })
    }

    pub(super) fn path(&self) -> &Path {
        self.dir.path()
    }

    pub(super) fn seed_parallel_fixture(&self) -> Result<()> {
        super::test_support::write_queue(self.path(), &parallel_tasks())?;
        let runner_path = super::test_support::create_noop_runner(self.path(), DEFAULT_RUNNER)?;
        super::test_support::configure_runner(
            self.path(),
            DEFAULT_RUNNER,
            DEFAULT_MODEL,
            Some(&runner_path),
        )?;
        super::test_support::configure_parallel_for_direct_push(self.path())?;
        super::test_support::git_add_all_commit(self.path(), FIXTURE_COMMIT_MESSAGE)?;
        push_origin_head(self.path())?;
        Ok(())
    }

    pub(super) fn run_parallel(&self) -> (ExitStatus, String, String) {
        let run_lock = super::test_support::parallel_run_lock().lock();
        let result = super::test_support::run_in_dir(self.path(), PARALLEL_ARGS);
        drop(run_lock);
        result
    }

    pub(super) fn read_queue_text(&self) -> Result<Option<String>> {
        read_text_if_exists(&self.path().join(".ralph/queue.jsonc"))
    }

    pub(super) fn read_done_text(&self) -> Result<Option<String>> {
        read_text_if_exists(&self.path().join(".ralph/done.jsonc"))
    }

    pub(super) fn read_parallel_state(&self) -> Result<Option<serde_json::Value>> {
        super::test_support::read_parallel_state(self.path())
    }

    pub(super) fn merge_tree_against_origin_main(&self) -> Result<String> {
        run_git(self.path(), &["fetch", "origin"], "git fetch origin")?;
        let merge_base = git_output(
            self.path(),
            &["merge-base", "HEAD", "origin/main"],
            "git merge-base HEAD origin/main",
        )?;
        git_output(
            self.path(),
            &["merge-tree", &merge_base, "HEAD", "origin/main"],
            "git merge-tree",
        )
    }
}

pub(super) fn assert_no_conflict_markers(label: &str, content: &str) {
    assert!(
        !content.contains("<<<<<<<"),
        "{label} should not contain conflict start marker"
    );
    assert!(
        !content.contains("======="),
        "{label} should not contain conflict separator"
    );
    assert!(
        !content.contains(">>>>>>>"),
        "{label} should not contain conflict end marker"
    );
}

fn parallel_tasks() -> Vec<Task> {
    vec![
        super::test_support::make_test_task("RQ-0958-A", "First parallel task", TaskStatus::Todo),
        super::test_support::make_test_task("RQ-0958-B", "Second parallel task", TaskStatus::Todo),
    ]
}

fn init_bare_remote(remote_path: &Path) -> Result<()> {
    run_git(
        remote_path,
        &["init", "--bare", "--quiet"],
        "git init --bare --quiet",
    )
}

fn add_origin_remote(repo_path: &Path, remote_path: &Path) -> Result<()> {
    let remote = remote_path.to_string_lossy();
    run_git(
        repo_path,
        &["remote", "add", "origin", remote.as_ref()],
        "git remote add origin",
    )
}

fn push_origin_head(repo_path: &Path) -> Result<()> {
    run_git(
        repo_path,
        &["push", "-u", "origin", "HEAD"],
        "git push -u origin HEAD",
    )
}

fn read_text_if_exists(path: &PathBuf) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    Ok(Some(text))
}

fn run_git(dir: &Path, args: &[&str], context: &str) -> Result<()> {
    let status = Command::new("git")
        .current_dir(dir)
        .args(args)
        .status()
        .with_context(|| context.to_string())?;
    ensure!(status.success(), "{context} failed");
    Ok(())
}

fn git_output(dir: &Path, args: &[&str], context: &str) -> Result<String> {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .with_context(|| context.to_string())?;
    ensure!(output.status.success(), "{context} failed");
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
