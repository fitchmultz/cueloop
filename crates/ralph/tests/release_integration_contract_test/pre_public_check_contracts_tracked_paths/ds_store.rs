//! Exact tracked `.DS_Store` contracts for `pre-public-check.sh`.
//!
//! Purpose:
//! - Keep the `.DS_Store` tracked-path regressions grouped together.
//!
//! Responsibilities:
//! - Verify tracked `.DS_Store` files and broken symlinks remain rejected as local-only files.
//!
//! Scope:
//! - Limited to exact tracked `.DS_Store` coverage.
//!
//! Usage:
//! - Loaded by `pre_public_check_contracts_tracked_paths.rs`.
//!
//! Invariants/Assumptions:
//! - These tests must preserve the existing release-contract assertions verbatim.

use std::process::Command;

use super::super::support::{
    commit_fixture, copy_pre_public_check_fixture, disable_global_excludes, force_stage_all,
    init_git_repo, write_file,
};

#[test]
fn pre_public_check_rejects_tracked_ds_store() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    write_file(&repo_root.join(".DS_Store"), "finder metadata\n");

    init_git_repo(repo_root);
    disable_global_excludes(repo_root);
    force_stage_all(repo_root);
    commit_fixture(repo_root);

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/pre-public-check.sh"))
        .args(["--skip-ci", "--skip-links", "--skip-clean"])
        .current_dir(repo_root)
        .output()
        .expect("run pre-public-check with tracked ds_store");

    assert!(
        !output.status.success(),
        "pre-public-check should reject tracked .DS_Store files\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Tracked local-only files detected") && combined.contains(".DS_Store"),
        "tracked .DS_Store rejection should explain the offending path\noutput:\n{}",
        combined
    );
}

#[cfg(unix)]
#[test]
fn pre_public_check_rejects_tracked_broken_ds_store_symlink() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    symlink("DOES_NOT_EXIST", repo_root.join(".DS_Store")).expect("create broken ds_store symlink");

    init_git_repo(repo_root);
    disable_global_excludes(repo_root);
    force_stage_all(repo_root);
    commit_fixture(repo_root);

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/pre-public-check.sh"))
        .args(["--skip-ci", "--skip-links", "--skip-clean"])
        .current_dir(repo_root)
        .output()
        .expect("run pre-public-check with broken tracked ds_store symlink");

    assert!(
        !output.status.success(),
        "pre-public-check should reject broken tracked .DS_Store symlinks\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Tracked local-only files detected") && combined.contains(".DS_Store"),
        "broken tracked .DS_Store symlink rejection should explain the offending path\noutput:\n{}",
        combined
    );
}
