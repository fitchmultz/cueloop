//! Purpose: suite-local helpers for `undo_integration_test` integration coverage.
//!
//! Responsibilities:
//! - Centralize disposable repo bootstrap for undo CLI integration tests.
//! - Keep shared snapshot-list parsing logic in one place.
//!
//! Scope:
//! - Helpers used only by `crates/ralph/tests/undo_integration_test.rs` companion modules.
//!
//! Usage:
//! - Call `setup_undo_repo()` at the start of each test.
//! - Call `snapshot_ids_from_list_output()` when a test needs to parse `ralph undo --list` output.
//!
//! Invariants/assumptions callers must respect:
//! - Helpers preserve the original end-to-end CLI setup flow: temp dir outside repo, git init, seed `.ralph/`.
//! - Snapshot ID parsing intentionally preserves the original `ID:`-based extraction logic exactly.

use super::*;

pub(crate) fn setup_undo_repo() -> Result<TempDir> {
    let dir = temp_dir_outside_repo();
    git_init(dir.path())?;
    seed_ralph_dir(dir.path())?;
    Ok(dir)
}

pub(crate) fn snapshot_ids_from_list_output(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter(|line| line.contains("ID:"))
        .filter_map(|line| line.split("ID:").nth(1))
        .map(|s| s.trim().to_string())
        .collect()
}
