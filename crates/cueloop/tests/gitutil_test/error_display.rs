//! Purpose: GitError display and classification contract coverage for `cueloop::git`.
//!
//! Responsibilities:
//! - Verify user-facing display text for representative GitError variants.
//! - Preserve push-error classification display assertions from the original suite.
//!
//! Scope:
//! - Public `GitError` formatting behavior only.
//!
//! Usage:
//! - Uses `use super::*;` for shared `git` access.
//!
//! Invariants/Assumptions:
//! - These tests remain pure formatting/classification checks with no repo fixture setup.
//! - Assertion text fragments must remain unchanged.

use super::*;

#[test]
fn test_git_error_display() {
    let err = git::GitError::DirtyRepo {
        details: "test details".to_string(),
    };
    let err_str = format!("{}", err);
    assert!(err_str.contains("repo is dirty"));
    assert!(err_str.contains("test details"));

    let err = git::GitError::EmptyCommitMessage;
    let err_str = format!("{}", err);
    assert!(err_str.contains("commit message is empty"));

    let err = git::GitError::NoChangesToCommit;
    let err_str = format!("{}", err);
    assert!(err_str.contains("no changes to commit"));
}

#[test]
fn test_classify_push_error_no_upstream() {
    let err = git::GitError::NoUpstream;
    let err_str = format!("{}", err);
    assert!(err_str.contains("no upstream"));
}

#[test]
fn test_classify_push_error_auth_failed() {
    let err = git::GitError::AuthFailed;
    let err_str = format!("{}", err);
    assert!(err_str.contains("authentication"));
}
