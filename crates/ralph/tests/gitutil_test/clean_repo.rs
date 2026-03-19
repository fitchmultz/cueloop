//! Purpose: clean-repo enforcement integration coverage for `ralph::git`.
//!
//! Responsibilities:
//! - Verify clean/dirty repo detection and error classification.
//! - Verify allowed-path and force-bypass behavior.
//! - Preserve indirect coverage of path allowlist normalization behavior.
//!
//! Scope:
//! - Tests for `require_clean_repo_ignoring_paths()` and related contract behavior.
//!
//! Usage:
//! - Uses `use super::*;` for shared imports and fixture helpers.
//!
//! Invariants/Assumptions:
//! - Indirect allowlist helper tests remain in this module.
//! - All original assertions stay byte-for-byte equivalent in intent.

use super::*;

#[test]
fn test_require_clean_repo_ignoring_paths_clean() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    let result = git::require_clean_repo_ignoring_paths(dir.path(), false, &[]);
    assert!(result.is_ok());
}

#[test]
fn test_require_clean_repo_ignoring_paths_with_dirty_changes() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    commit_file(&dir, "test.txt", "content", "initial");

    let file_path = dir.path().join("test.txt");
    fs::write(&file_path, "modified").expect("failed to modify file");

    let result = git::require_clean_repo_ignoring_paths(dir.path(), false, &[]);
    assert!(result.is_err());

    if let Err(git::GitError::DirtyRepo { details }) = result {
        assert!(details.contains("Tracked changes"));
    } else {
        panic!("Expected DirtyRepo error");
    }
}

#[test]
fn test_require_clean_repo_ignoring_paths_with_untracked_files() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    let file_path = dir.path().join("untracked.txt");
    fs::write(&file_path, "content").expect("failed to write file");

    let result = git::require_clean_repo_ignoring_paths(dir.path(), false, &[]);
    assert!(result.is_err());

    if let Err(git::GitError::DirtyRepo { details }) = result {
        assert!(details.contains("Untracked files"));
    } else {
        panic!("Expected DirtyRepo error");
    }
}

#[test]
fn test_require_clean_repo_ignoring_paths_with_allowed_paths() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    commit_file(&dir, "allowed.txt", "content", "initial");

    let file_path = dir.path().join("allowed.txt");
    fs::write(&file_path, "modified").expect("failed to modify file");

    let untracked_path = dir.path().join("also-allowed.txt");
    fs::write(&untracked_path, "untracked").expect("failed to write file");

    let result = git::require_clean_repo_ignoring_paths(
        dir.path(),
        false,
        &["allowed.txt", "also-allowed.txt"],
    );
    assert!(result.is_ok(), "Should allow changes in specified paths");
}

#[test]
fn test_require_clean_repo_ignoring_paths_force_bypasses_check() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    let file_path = dir.path().join("untracked.txt");
    fs::write(&file_path, "content").expect("failed to write file");

    let result = git::require_clean_repo_ignoring_paths(dir.path(), true, &[]);
    assert!(result.is_ok(), "Force flag should bypass dirty check");
}

#[test]
fn test_require_clean_repo_ignoring_paths_with_mixed_changes() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    commit_file(&dir, "allowed.txt", "content", "initial");
    commit_file(&dir, "not-allowed.txt", "content", "initial");

    fs::write(dir.path().join("allowed.txt"), "modified").expect("failed to modify");
    fs::write(dir.path().join("not-allowed.txt"), "modified").expect("failed to modify");

    let result = git::require_clean_repo_ignoring_paths(dir.path(), false, &["allowed.txt"]);
    assert!(
        result.is_err(),
        "Should fail due to not-allowed.txt changes"
    );

    if let Err(git::GitError::DirtyRepo { details }) = result {
        assert!(details.contains("not-allowed.txt"));
    } else {
        panic!("Expected DirtyRepo error");
    }
}

#[test]
fn test_path_is_allowed_helper() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    commit_file(&dir, "test.txt", "content", "initial");

    fs::write(dir.path().join("test.txt"), "modified").expect("failed to modify");

    let result = git::require_clean_repo_ignoring_paths(dir.path(), false, &["other.txt"]);
    assert!(result.is_err());

    let result = git::require_clean_repo_ignoring_paths(dir.path(), false, &["test.txt"]);
    assert!(result.is_ok());
}

#[test]
fn test_path_is_allowed_with_dot_prefix() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    commit_file(&dir, "test.txt", "content", "initial");

    fs::write(dir.path().join("test.txt"), "modified").expect("failed to modify");

    let result = git::require_clean_repo_ignoring_paths(dir.path(), false, &["./test.txt"]);
    assert!(result.is_ok());
}
