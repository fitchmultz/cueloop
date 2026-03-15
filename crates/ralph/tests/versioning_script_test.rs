//! Versioning script contract tests.
//!
//! Purpose:
//! - Guard the public behavior of `scripts/versioning.sh` used by release and local verification flows.
//!
//! Responsibilities:
//! - Verify the script prints the canonical repo version.
//! - Verify metadata drift checks succeed in the synchronized repository state.
//! - Confirm the check path does not require ripgrep on `PATH`.
//!
//! Scope:
//! - Script contract coverage only; not end-to-end release publication.
//!
//! Usage:
//! - Executed as part of the Rust integration-test suite.
//!
//! Invariants/assumptions:
//! - The versioning script exists at `scripts/versioning.sh` relative to repo root.
//! - Checked-in version metadata is synchronized before the test suite runs.

use std::path::PathBuf;
use std::process::{Command, ExitStatus};

fn repo_root() -> PathBuf {
    let exe = std::env::current_exe().expect("resolve current test executable path");
    let exe_dir = exe
        .parent()
        .expect("test executable should have a parent directory");

    let profile_dir = if exe_dir.file_name() == Some(std::ffi::OsStr::new("deps")) {
        exe_dir
            .parent()
            .expect("deps directory should have a parent directory")
    } else {
        exe_dir
    };

    profile_dir
        .parent()
        .expect("profile directory should have a parent (target)")
        .parent()
        .expect("target directory should have a parent (repo root)")
        .to_path_buf()
}

fn version_script() -> PathBuf {
    repo_root().join("scripts").join("versioning.sh")
}

fn canonical_version() -> String {
    std::fs::read_to_string(repo_root().join("VERSION"))
        .expect("read VERSION")
        .trim()
        .to_string()
}

fn run_script(args: &[&str]) -> (ExitStatus, String, String) {
    run_script_with_path(args, None)
}

fn run_script_with_path(
    args: &[&str],
    path_override: Option<&str>,
) -> (ExitStatus, String, String) {
    let mut command = Command::new("bash");
    command.arg(version_script()).args(args);
    if let Some(path) = path_override {
        command.env("PATH", path);
    }

    let output = command.output().expect("failed to execute versioning.sh");
    (
        output.status,
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn versioning_script_current_matches_version_file() {
    let expected = canonical_version();
    let (status, stdout, stderr) = run_script(&["current"]);
    assert!(
        status.success(),
        "expected versioning.sh current to succeed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert_eq!(stdout.trim(), expected);
}

#[test]
fn versioning_script_check_succeeds_when_metadata_is_synced() {
    let (status, stdout, stderr) = run_script(&["check"]);
    assert!(
        status.success(),
        "expected versioning.sh check to succeed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("versioning: OK"),
        "expected success marker in stdout\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn versioning_script_check_succeeds_without_ripgrep_on_path() {
    let (status, stdout, stderr) = run_script_with_path(&["check"], Some("/usr/bin:/bin"));
    assert!(
        status.success(),
        "expected versioning.sh check to succeed without ripgrep on PATH\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("versioning: OK"),
        "expected success marker in stdout\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("rg: command not found"),
        "versioning.sh should not shell out to ripgrep\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn versioning_script_sync_refreshes_lockfile() {
    let script = std::fs::read_to_string(version_script()).expect("read versioning.sh");
    assert!(
        script.contains("update -w --offline"),
        "versioning.sh sync should refresh Cargo.lock for the workspace root package"
    );
}

#[test]
fn versioning_script_check_reports_lockfile_drift() {
    let script = std::fs::read_to_string(version_script()).expect("read versioning.sh");
    assert!(
        script.contains("Cargo.lock version drifted"),
        "versioning.sh check should fail explicitly when Cargo.lock is out of sync"
    );
}
