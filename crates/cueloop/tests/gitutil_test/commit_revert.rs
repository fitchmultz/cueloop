//! Purpose: commit and revert integration coverage for `cueloop::git` working-tree helpers.
//!
//! Responsibilities:
//! - Verify `commit_all()` validation and success behavior.
//! - Verify `revert_uncommitted()` restores tracked content and preserves expected local env files.
//!
//! Scope:
//! - Public commit/revert APIs only.
//!
//! Usage:
//! - Uses `use super::*;` for shared imports and repo setup helpers.
//!
//! Invariants/Assumptions:
//! - Empty-message and no-change error assertions must remain unchanged.
//! - Revert coverage preserves the `.env` file contract exactly.

use super::*;

#[test]
fn test_commit_all_empty_message_fails() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    let result = git::commit_all(dir.path(), "");
    assert!(result.is_err());

    if let Err(git::GitError::EmptyCommitMessage) = result {
    } else {
        panic!("Expected EmptyCommitMessage error");
    }
}

#[test]
fn test_commit_all_whitespace_message_fails() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    let result = git::commit_all(dir.path(), "   ");
    assert!(result.is_err());

    if let Err(git::GitError::EmptyCommitMessage) = result {
    } else {
        panic!("Expected EmptyCommitMessage error");
    }
}

#[test]
fn test_commit_all_no_changes_fails() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    commit_file(&dir, "test.txt", "content", "initial");

    let result = git::commit_all(dir.path(), "test commit");
    assert!(result.is_err());

    if let Err(git::GitError::NoChangesToCommit) = result {
    } else {
        panic!("Expected NoChangesToCommit error");
    }
}

#[test]
fn test_commit_all_success() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    commit_file(&dir, "test.txt", "content", "initial");

    let file_path = dir.path().join("test.txt");
    fs::write(&file_path, "modified content").expect("failed to modify file");

    let result = git::commit_all(dir.path(), "test commit");
    assert!(result.is_ok());

    let status = git::status_porcelain(dir.path()).unwrap();
    assert!(
        status.trim().is_empty(),
        "Repo should be clean after commit"
    );
}

#[test]
fn test_revert_uncommitted_restores_clean_state() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    commit_file(&dir, "test.txt", "original content", "initial");

    let file_path = dir.path().join("test.txt");
    fs::write(&file_path, "modified content").expect("failed to modify file");

    let untracked_path = dir.path().join("untracked.txt");
    fs::write(&untracked_path, "untracked").expect("failed to write file");

    let status_before = git::status_porcelain(dir.path()).unwrap();
    assert!(!status_before.trim().is_empty());

    let result = git::revert_uncommitted(dir.path());
    assert!(result.is_ok());

    let status_after = git::status_porcelain(dir.path()).unwrap();
    assert!(
        status_after.trim().is_empty(),
        "Repo should be clean after revert"
    );

    let restored_content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(restored_content, "original content");

    assert!(!untracked_path.exists());
}

#[test]
fn test_revert_uncommitted_preserves_env_files() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    commit_file(&dir, "test.txt", "content", "initial");

    let file_path = dir.path().join("test.txt");
    fs::write(&file_path, "modified").expect("failed to modify");

    let env_path = dir.path().join(".env");
    fs::write(&env_path, "SECRET=value").expect("failed to write .env");

    let env_local_path = dir.path().join(".env.local");
    fs::write(&env_local_path, "LOCAL=value").expect("failed to write .env.local");

    git::revert_uncommitted(dir.path()).unwrap();

    assert!(env_path.exists());
    assert!(env_local_path.exists());

    let env_content = fs::read_to_string(&env_path).unwrap();
    assert_eq!(env_content, "SECRET=value");
}
