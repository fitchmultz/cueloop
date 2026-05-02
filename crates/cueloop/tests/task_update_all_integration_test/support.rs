//! Purpose: suite-local helpers for `task_update_all_integration_test` integration coverage.
//!
//! Responsibilities:
//! - Preserve the original CLI/bootstrap helpers used by update-all task tests.
//! - Centralize queue fixture writers for empty, single-task, and two-task scenarios.
//! - Delegate shared runner/config wiring to the common `test_support` helpers.
//!
//! Scope:
//! - Helpers used only by `crates/cueloop/tests/task_update_all_integration_test.rs` companion modules.
//!
//! Usage:
//! - Call `run_in_dir()` for CLI execution.
//! - Call `configure_runner()` and `create_fake_runner()` when a test needs a local fake runner.
//! - Call the queue writer helpers to seed the exact queue/done fixtures expected by this suite.
//!
//! Invariants/assumptions callers must respect:
//! - Queue fixture contents intentionally match the pre-split monolith exactly.
//! - `configure_runner()` is a thin delegate to the shared config helper; do not reintroduce inline JSON config mutation here.
//! - Helpers preserve end-to-end CLI coverage and do not bypass `cueloop` commands.

use super::test_support;
use super::*;
use cueloop::config::project_runtime_dir;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;

const EMPTY_DONE_JSON: &str = r#"{ 
  "version": 1,
  "tasks": []
}"#;

const EMPTY_QUEUE_JSON: &str = r#"{ 
  "version": 1,
  "tasks": []
}"#;

const ONE_TASK_QUEUE_JSON: &str = r#"{
  "version": 1,
  "tasks": [
    {
      "id": "RQ-0001",
      "status": "todo",
      "title": "First task",
      "tags": ["test"],
      "scope": ["crates/cueloop"],
      "evidence": ["integration test"],
      "plan": ["step one"],
      "notes": [],
      "request": "first request",
      "created_at": "2026-01-18T00:00:00Z",
      "updated_at": "2026-01-18T00:00:00Z"
    }
  ]
}"#;

const TWO_TASK_QUEUE_JSON: &str = r#"{ 
  "version": 1,
  "tasks": [
    {
      "id": "RQ-0001",
      "status": "todo",
      "title": "First task",
      "tags": ["test"],
      "scope": ["crates/cueloop"],
      "evidence": ["integration test"],
      "plan": ["step one"],
      "notes": [],
      "request": "first request",
      "created_at": "2026-01-18T00:00:00Z",
      "updated_at": "2026-01-18T00:00:00Z"
    },
    {
      "id": "RQ-0002",
      "status": "todo",
      "title": "Second task",
      "tags": ["test"],
      "scope": ["crates/cueloop"],
      "evidence": ["integration test"],
      "plan": ["step one"],
      "notes": [],
      "request": "second request",
      "created_at": "2026-01-18T00:00:00Z",
      "updated_at": "2026-01-18T00:00:00Z"
    }
  ]
}"#;

fn write_queue_files(dir: &Path, queue_json: &str, done_json: &str) -> Result<()> {
    let runtime_dir = project_runtime_dir(dir);
    std::fs::create_dir_all(&runtime_dir).context("create runtime dir")?;
    std::fs::write(runtime_dir.join("queue.jsonc"), queue_json).context("write queue.jsonc")?;
    std::fs::write(runtime_dir.join("done.jsonc"), done_json).context("write done.jsonc")?;
    Ok(())
}

pub(crate) fn run_in_dir(dir: &Path, args: &[&str]) -> (ExitStatus, String, String) {
    test_support::run_in_dir(dir, args)
}

pub(crate) fn configure_runner(
    dir: &Path,
    runner: &str,
    model: &str,
    bin_path: Option<&Path>,
) -> Result<()> {
    test_support::configure_runner(dir, runner, model, bin_path)
}

pub(crate) fn create_fake_runner(dir: &Path, runner: &str, script: &str) -> Result<PathBuf> {
    test_support::create_fake_runner(dir, runner, script)
}

pub(crate) fn write_queue_with_two_tasks(dir: &Path) -> Result<()> {
    write_queue_files(dir, TWO_TASK_QUEUE_JSON, EMPTY_DONE_JSON)
}

pub(crate) fn write_empty_queue(dir: &Path) -> Result<()> {
    write_queue_files(dir, EMPTY_QUEUE_JSON, EMPTY_DONE_JSON)
}

pub(crate) fn write_queue_with_one_task(dir: &Path) -> Result<()> {
    write_queue_files(dir, ONE_TASK_QUEUE_JSON, EMPTY_DONE_JSON)
}
