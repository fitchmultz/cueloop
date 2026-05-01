//! Session persistence tests.
//!
//! Purpose:
//! - Session persistence tests.
//!
//! Responsibilities:
//! - Verify session file save/load/clear behavior and git HEAD discovery.
//!
//! Scope:
//! - Persistence behavior only; validation and resume decisions live in sibling modules.
//!
//! Usage:
//! - Compiled through `crate::session::tests`.
//!
//! Invariants/assumptions:
//! - Tests use temporary repositories and never mutate the real workspace.

use super::*;
use crate::testsupport::git as git_test;
use tempfile::TempDir;

#[test]
fn get_git_head_commit_returns_current_head() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    git_test::init_repo(temp_dir.path())?;
    std::fs::write(temp_dir.path().join("README.md"), "session commit")?;
    git_test::commit_all(temp_dir.path(), "init")?;

    let commit = get_git_head_commit(temp_dir.path());
    let expected = git_test::git_output(temp_dir.path(), &["rev-parse", "HEAD"])?;

    assert_eq!(commit.as_deref(), Some(expected.as_str()));
    Ok(())
}

#[test]
fn save_and_load_session_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let session = test_session("RQ-0001");

    save_session(temp_dir.path(), &session).unwrap();
    let loaded = load_session(temp_dir.path()).unwrap().unwrap();

    assert_eq!(loaded.session_id, session.session_id);
    assert_eq!(loaded.task_id, session.task_id);
    assert_eq!(loaded.iterations_planned, session.iterations_planned);
}

#[test]
fn clear_session_removes_file() {
    let temp_dir = TempDir::new().unwrap();
    let session = test_session("RQ-0001");

    save_session(temp_dir.path(), &session).unwrap();
    assert!(session_exists(temp_dir.path()));

    clear_session(temp_dir.path()).unwrap();
    assert!(!session_exists(temp_dir.path()));
}

#[test]
fn session_path_returns_correct_path() {
    let temp_dir = TempDir::new().unwrap();
    assert_eq!(
        session_path(temp_dir.path()),
        temp_dir.path().join("session.jsonc")
    );
}
