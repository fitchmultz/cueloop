//! Purpose: suite-local fixtures and CLI helpers for `queue_search_test`.
//!
//! Responsibilities:
//! - Create disposable repos from the cached git+`.cueloop/` integration scaffold.
//! - Centralize queue/done fixture writes for search scenarios.
//! - Provide small helpers for invoking `cueloop queue search` and decoding JSON output.
//!
//! Scope:
//! - Helpers used only by `crates/cueloop/tests/queue_search_test.rs`.
//!
//! Usage:
//! - Call `SearchRepo::new()` per test.
//! - Seed queue/done state with `write_queue()` / `write_done()`.
//! - Use `search_json()` for JSON-output assertions and `search()` for raw stdout/stderr checks.
//!
//! Invariants/Assumptions:
//! - Fixtures use cached seeded repo scaffolding instead of repeating `git init` + `.cueloop` bootstrap.
//! - Helpers preserve end-to-end CLI coverage by invoking the real `cueloop` binary.
//! - JSON assertions are only valid when the caller passes `--format json`.

use anyhow::{Context, Result, ensure};
use cueloop::contracts::Task;
use std::path::Path;
use std::process::ExitStatus;
use tempfile::TempDir;

pub(super) struct SearchRepo {
    dir: TempDir,
}

impl SearchRepo {
    pub(super) fn new() -> Result<Self> {
        let dir = super::test_support::temp_dir_outside_repo();
        super::test_support::seed_git_repo_with_cueloop(dir.path())?;
        Ok(Self { dir })
    }

    pub(super) fn path(&self) -> &Path {
        self.dir.path()
    }

    pub(super) fn write_queue(&self, tasks: &[Task]) -> Result<()> {
        super::test_support::write_queue(self.path(), tasks)
    }

    pub(super) fn write_done(&self, tasks: &[Task]) -> Result<()> {
        super::test_support::write_done(self.path(), tasks)
    }

    pub(super) fn search(&self, args: &[&str]) -> (ExitStatus, String, String) {
        super::test_support::run_in_dir(self.path(), args)
    }

    pub(super) fn search_ok(&self, args: &[&str]) -> Result<(String, String)> {
        let (status, stdout, stderr) = self.search(args);
        ensure!(
            status.success(),
            "search failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
        Ok((stdout, stderr))
    }

    pub(super) fn search_json(&self, args: &[&str]) -> Result<serde_json::Value> {
        let (stdout, _stderr) = self.search_ok(args)?;
        serde_json::from_str(&stdout).context("parse queue search JSON output")
    }
}
