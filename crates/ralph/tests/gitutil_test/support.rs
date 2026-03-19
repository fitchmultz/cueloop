//! Purpose: suite-local fixture helpers for `gitutil_test`.
//!
//! Responsibilities:
//! - Preserve the original raw git repository bootstrap behavior used by this suite.
//! - Centralize file creation and commit setup helpers shared across companion modules.
//!
//! Scope:
//! - Helpers used only by the `gitutil_test` integration suite.
//!
//! Usage:
//! - Call `init_git_repo(&dir)` before exercising `ralph::git` APIs.
//! - Call `commit_file(&dir, filename, content, message)` when a test needs a committed tracked file.
//!
//! Invariants/Assumptions:
//! - These helpers intentionally preserve the pre-split behavior exactly.
//! - `git init` uses the host default branch name.
//! - Helper failures are fixture bootstrap failures and should panic immediately.

use super::*;

pub(crate) fn init_git_repo(dir: &TempDir) {
    Command::new("git")
        .arg("init")
        .current_dir(dir.path())
        .output()
        .expect("git init failed");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir.path())
        .output()
        .expect("git config user.email failed");

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(dir.path())
        .output()
        .expect("git config user.name failed");
}

pub(crate) fn commit_file(dir: &TempDir, filename: &str, content: &str, message: &str) {
    let file_path = dir.path().join(filename);
    fs::write(&file_path, content).expect("failed to write file");

    Command::new("git")
        .args(["add", filename])
        .current_dir(dir.path())
        .output()
        .expect("git add failed");

    Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(dir.path())
        .output()
        .expect("git commit failed");
}
