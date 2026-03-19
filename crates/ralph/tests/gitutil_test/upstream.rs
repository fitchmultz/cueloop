//! Purpose: upstream and ahead/behind integration coverage for `ralph::git`.
//!
//! Responsibilities:
//! - Verify upstream lookup behavior with and without configured tracking branches.
//! - Preserve `is_ahead_of_upstream()` error coverage from the original suite.
//!
//! Scope:
//! - Public upstream-related git APIs exposed through `ralph::git`.
//!
//! Usage:
//! - Uses `use super::*;` for shared imports and suite-local helpers.
//!
//! Invariants/Assumptions:
//! - Branch names are discovered dynamically because host git defaults may vary.
//! - Test behavior remains unchanged from the original suite.

use super::*;

#[test]
fn test_upstream_ref_no_upstream() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    let result = git::upstream_ref(dir.path());
    assert!(result.is_err());
}

#[test]
fn test_upstream_ref_with_upstream() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    commit_file(&dir, "test.txt", "content", "initial");

    let branch_output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(dir.path())
        .output()
        .expect("git rev-parse --abbrev-ref HEAD failed");
    assert!(
        branch_output.status.success(),
        "git rev-parse failed: {}",
        String::from_utf8_lossy(&branch_output.stderr)
    );
    let branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();
    assert!(
        !branch.is_empty() && branch != "HEAD",
        "expected a non-empty branch name, got {:?}",
        branch
    );

    let bare = TempDir::new().expect("create bare repo dir");
    let init_bare_output = Command::new("git")
        .args(["init", "--bare"])
        .current_dir(bare.path())
        .output()
        .expect("git init --bare failed");
    assert!(
        init_bare_output.status.success(),
        "git init --bare failed: {}",
        String::from_utf8_lossy(&init_bare_output.stderr)
    );

    let bare_path = bare
        .path()
        .to_str()
        .expect("bare repo path should be valid UTF-8");

    let remote_output = Command::new("git")
        .args(["remote", "add", "origin", bare_path])
        .current_dir(dir.path())
        .output()
        .expect("git remote add failed");
    assert!(
        remote_output.status.success(),
        "git remote add failed: {}",
        String::from_utf8_lossy(&remote_output.stderr)
    );

    let push_output = Command::new("git")
        .args([
            "-c",
            "protocol.file.allow=always",
            "push",
            "-u",
            "origin",
            branch.as_str(),
        ])
        .current_dir(dir.path())
        .output()
        .expect("git push failed");
    assert!(
        push_output.status.success(),
        "git push -u failed: {}",
        String::from_utf8_lossy(&push_output.stderr)
    );

    let upstream = git::upstream_ref(dir.path()).expect("upstream_ref should succeed");
    assert_eq!(upstream, format!("origin/{branch}"));
}

#[test]
fn test_is_ahead_of_upstream_no_upstream() {
    let dir = TempDir::new().expect("create temp dir");
    init_git_repo(&dir);

    let result = git::is_ahead_of_upstream(dir.path());
    assert!(result.is_err());
}
