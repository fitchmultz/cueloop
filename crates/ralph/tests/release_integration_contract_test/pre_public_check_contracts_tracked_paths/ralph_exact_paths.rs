//! Exact tracked `.ralph` path contracts for `pre-public-check.sh`.
//!
//! Purpose:
//! - Keep the exact tracked `.ralph` rejection coverage grouped together.
//!
//! Responsibilities:
//! - Verify tracked root `.ralph` files and symlinks remain outside the public allowlist.
//!
//! Scope:
//! - Limited to exact tracked `.ralph` path coverage.
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
fn pre_public_check_rejects_tracked_exact_ralph_root_file() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    write_file(&repo_root.join(".ralph"), "tracked root ralph file\n");

    init_git_repo(repo_root);
    disable_global_excludes(repo_root);
    force_stage_all(repo_root);
    commit_fixture(repo_root);

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/pre-public-check.sh"))
        .args([
            "--skip-ci",
            "--skip-links",
            "--skip-secrets",
            "--skip-clean",
        ])
        .current_dir(repo_root)
        .output()
        .expect("run pre-public-check with tracked exact .ralph root file");

    assert!(
        !output.status.success(),
        "pre-public-check should reject tracked exact .ralph root files\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Tracked .ralph files outside the public allowlist detected")
            && combined.contains(".ralph"),
        "tracked exact .ralph root file rejection should explain the offending path\noutput:\n{}",
        combined
    );
}

#[cfg(unix)]
#[test]
fn pre_public_check_rejects_tracked_exact_ralph_root_symlink() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let outside_dir = tempfile::tempdir().expect("create outside dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    std::fs::write(outside_dir.path().join("outside.txt"), "outside\n")
        .expect("write outside file");
    symlink(
        outside_dir.path().join("outside.txt"),
        repo_root.join(".ralph"),
    )
    .expect("create tracked root .ralph symlink");

    init_git_repo(repo_root);
    disable_global_excludes(repo_root);
    force_stage_all(repo_root);
    commit_fixture(repo_root);

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/pre-public-check.sh"))
        .args([
            "--skip-ci",
            "--skip-links",
            "--skip-secrets",
            "--skip-clean",
        ])
        .current_dir(repo_root)
        .output()
        .expect("run pre-public-check with tracked exact .ralph root symlink");

    assert!(
        !output.status.success(),
        "pre-public-check should reject tracked exact .ralph root symlinks\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Tracked .ralph files outside the public allowlist detected")
            && combined.contains(".ralph"),
        "tracked exact .ralph root symlink rejection should explain the offending path\noutput:\n{}",
        combined
    );
}
