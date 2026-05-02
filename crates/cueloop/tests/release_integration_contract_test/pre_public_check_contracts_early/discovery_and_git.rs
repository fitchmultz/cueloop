//! Early `pre-public-check.sh` discovery and git-prerequisite contracts.
//!
//! Purpose:
//! - Keep the early discovery wiring and git-backed prerequisite checks together.
//!
//! Responsibilities:
//! - Verify repo-wide scan wiring, git-worktree requirements, and required-file symlink handling.
//!
//! Scope:
//! - Limited to early `pre-public-check.sh` release-contract coverage.
//!
//! Usage:
//! - Loaded by `pre_public_check_contracts_early.rs`.
//!
//! Invariants/Assumptions:
//! - These tests must preserve the existing release-contract assertions verbatim.

use std::process::Command;

use super::super::support::{
    commit_fixture, copy_pre_public_check_fixture, copy_repo_file, disable_global_excludes,
    force_stage_all, init_git_repo, read_repo_file,
};

#[test]
fn pre_public_check_uses_repo_wide_markdown_discovery() {
    let script = read_repo_file("scripts/pre-public-check.sh");
    let focused_scan_helper = read_repo_file("scripts/lib/public_readiness_scan.sh");
    assert!(
        script.contains("bash \"$SCRIPT_DIR/lib/public_readiness_scan.sh\" all"),
        "pre-public check should delegate default content scanning to the combined repo-wide scan helper"
    );
    assert!(
        focused_scan_helper.contains("public_readiness_scan.py")
            && focused_scan_helper.contains("python3 \"$scan_py_path\" links \"$repo_root\""),
        "focused public-readiness scan helper should drive repo-wide markdown discovery from the working tree"
    );
    assert!(
        focused_scan_helper.contains("python3 \"$scan_py_path\" all \"$repo_root\""),
        "focused public-readiness scan helper should expose the combined all-content scan mode"
    );
    assert!(
        script.contains("--release-context"),
        "pre-public check should expose an explicit release-context mode"
    );
    assert!(
        script.contains("--allow-no-git"),
        "pre-public check should expose a source-snapshot safety mode for check-env-safety"
    );
}

#[test]
fn pre_public_check_requires_git_worktree() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_repo_file("scripts/pre-public-check.sh", repo_root);
    copy_repo_file("scripts/lib/cueloop-shell.sh", repo_root);
    copy_repo_file("scripts/lib/release_policy.sh", repo_root);

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/pre-public-check.sh"))
        .arg("--skip-ci")
        .current_dir(repo_root)
        .output()
        .expect("run pre-public-check outside git worktree");

    assert!(
        !output.status.success(),
        "pre-public-check should fail when git metadata is unavailable"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("require a git worktree"),
        "pre-public-check should explain that git-backed readiness checks are unavailable\nstderr:\n{}",
        stderr
    );
}

#[cfg(unix)]
#[test]
fn pre_public_check_rejects_symlinked_required_files_in_source_snapshots() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let outside_dir = tempfile::tempdir().expect("create outside dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    std::fs::remove_file(repo_root.join("LICENSE")).expect("remove copied license");
    std::fs::write(outside_dir.path().join("LICENSE.txt"), "external license\n")
        .expect("write external license");
    symlink(
        outside_dir.path().join("LICENSE.txt"),
        repo_root.join("LICENSE"),
    )
    .expect("create symlinked license");

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/pre-public-check.sh"))
        .args(["--skip-ci", "--skip-clean", "--allow-no-git"])
        .current_dir(repo_root)
        .output()
        .expect("run pre-public-check with symlinked required file in source snapshot");

    assert!(
        !output.status.success(),
        "pre-public-check should reject symlinked required files in source snapshots\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Required file must be a regular repo file, not a symlink: LICENSE"),
        "symlinked required file rejection should explain the offending path\noutput:\n{}",
        combined
    );
}

#[cfg(unix)]
#[test]
fn pre_public_check_rejects_symlinked_required_files_in_git_snapshots() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_pre_public_check_fixture(repo_root);
    std::fs::create_dir_all(repo_root.join(".cueloop")).expect("create .cueloop dir");
    std::fs::write(repo_root.join(".cueloop/trust.json"), "{}\n").expect("write trust file");
    std::fs::remove_file(repo_root.join("LICENSE")).expect("remove copied license");
    symlink(
        repo_root.join(".cueloop/trust.json"),
        repo_root.join("LICENSE"),
    )
    .expect("create symlinked license");

    init_git_repo(repo_root);
    disable_global_excludes(repo_root);
    force_stage_all(repo_root);
    commit_fixture(repo_root);

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/pre-public-check.sh"))
        .args(["--skip-ci"])
        .current_dir(repo_root)
        .output()
        .expect("run pre-public-check with symlinked required file in git snapshot");

    assert!(
        !output.status.success(),
        "pre-public-check should reject symlinked required files in git snapshots\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Required file must be a regular repo file, not a symlink: LICENSE"),
        "tracked symlinked required file rejection should explain the offending path\noutput:\n{}",
        combined
    );
}
