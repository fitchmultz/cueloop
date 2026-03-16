//! Purpose: suite-local helpers for `task_lifecycle_test` integration coverage.
//!
//! Responsibilities:
//! - Create isolated repos from the cached git+`.ralph/` test scaffold.
//! - Centralize queue IO and `ralph` CLI execution boilerplate.
//! - Centralize repeated fake-runner and CI bootstrap for lifecycle run tests.
//!
//! Scope:
//! - Helper utilities used only by `crates/ralph/tests/task_lifecycle_test.rs`.
//!
//! Usage:
//! - Call `LifecycleRepo::new()` for each test to get a disposable seeded repo.
//! - Use `run_ok()` for success-path commands and `run()` when the test expects failure.
//! - Use `setup_runner_with_passing_ci()` after writing the queue fixture for runner-based tests.
//!
//! Invariants/assumptions callers must respect:
//! - Helpers preserve real end-to-end CLI coverage; they do not bypass `ralph` commands.
//! - `setup_runner_with_passing_ci()` rewrites repo config, writes a minimal `Makefile`, and commits fixture state.
//! - Repos are disposable temp dirs seeded from cached fixtures, so callers may freely mutate them.

use anyhow::{Result, ensure};
use ralph::contracts::{QueueFile, Task, TaskStatus};
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use tempfile::TempDir;

const DEFAULT_RUNNER: &str = "codex";
const DEFAULT_MODEL: &str = "gpt-5.3-codex";
const PASSING_CI_MAKEFILE: &str = "ci:\n\t@echo 'CI passed'\n";
const FIXTURE_TIMESTAMP: &str = "2026-02-19T00:00:00Z";

pub(super) struct LifecycleRepo {
    dir: TempDir,
}

impl LifecycleRepo {
    pub(super) fn new() -> Result<Self> {
        let dir = super::test_support::temp_dir_outside_repo();
        super::test_support::seed_git_repo_with_ralph(dir.path())?;
        Ok(Self { dir })
    }

    pub(super) fn path(&self) -> &Path {
        self.dir.path()
    }

    pub(super) fn write_queue(&self, tasks: &[Task]) -> Result<()> {
        super::test_support::write_queue(self.path(), tasks)
    }

    pub(super) fn read_queue(&self) -> Result<QueueFile> {
        super::test_support::read_queue(self.path())
    }

    pub(super) fn read_done(&self) -> Result<QueueFile> {
        super::test_support::read_done(self.path())
    }

    pub(super) fn run(&self, args: &[&str]) -> (ExitStatus, String, String) {
        super::test_support::run_in_dir(self.path(), args)
    }

    pub(super) fn run_ok(&self, args: &[&str]) -> Result<()> {
        let (status, stdout, stderr) = self.run(args);
        ensure!(
            status.success(),
            "ralph {} failed\nstdout:\n{stdout}\nstderr:\n{stderr}",
            args.join(" ")
        );
        Ok(())
    }

    pub(super) fn setup_runner_with_passing_ci(&self, script: &str) -> Result<PathBuf> {
        let runner_path =
            super::test_support::create_fake_runner(self.path(), DEFAULT_RUNNER, script)?;
        super::test_support::configure_runner(
            self.path(),
            DEFAULT_RUNNER,
            DEFAULT_MODEL,
            Some(&runner_path),
        )?;
        std::fs::write(self.path().join("Makefile"), PASSING_CI_MAKEFILE)?;
        super::test_support::git_add_all_commit(self.path(), "setup")?;
        Ok(runner_path)
    }
}

pub(super) fn draft_task(id: &str, title: &str) -> Task {
    Task {
        id: id.to_string(),
        title: title.to_string(),
        status: TaskStatus::Draft,
        created_at: Some(FIXTURE_TIMESTAMP.to_string()),
        updated_at: Some(FIXTURE_TIMESTAMP.to_string()),
        ..Default::default()
    }
}

pub(super) fn terminal_task(id: &str, title: &str, status: TaskStatus) -> Task {
    let mut task = super::test_support::make_test_task(id, title, status);
    task.completed_at = Some(FIXTURE_TIMESTAMP.to_string());
    task
}
