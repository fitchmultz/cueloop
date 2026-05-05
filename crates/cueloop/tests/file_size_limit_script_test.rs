//! File-size guard script contract tests.
//!
//! Purpose:
//! - Verify the behavior of `scripts/check-file-size-limits.sh` and its helper policy scanner.
//!
//! Responsibilities:
//! - Validate help output and argument error behavior.
//! - Validate pass/advisory/review/fail outcomes and reasoned fail allowlists.
//! - Validate exclude behavior for machine-owned/generated paths and configurable excludes.
//! - Validate that untracked monitored files are included in policy checks.
//!
//! Not handled here:
//! - Full end-to-end Makefile gate orchestration.
//! - Policy threshold decisions (sourced from AGENTS.md + script defaults).
//!
//! Usage:
//! - Executed as part of the Rust integration-test suite.
//!
//! Invariants/assumptions:
//! - Bash and git are available locally.
//! - Script paths remain stable at `scripts/check-file-size-limits.sh` and
//!   `scripts/lib/file_size_limits.py`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

fn repo_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .expect("crate directory should have a parent")
        .parent()
        .expect("crates directory should have a parent repo root")
        .to_path_buf()
}

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent directories");
    }
    std::fs::write(path, content).unwrap_or_else(|err| panic!("write {}: {err}", path.display()));
}

fn write_lines(path: &Path, count: usize) {
    let mut body = String::new();
    for index in 0..count {
        body.push_str(&format!("line {index}\n"));
    }
    write_file(path, &body);
}

fn copy_repo_file(repo_root: &Path, temp_repo: &Path, relative_path: &str) {
    let source = repo_root.join(relative_path);
    let content = std::fs::read_to_string(&source)
        .unwrap_or_else(|err| panic!("read {}: {err}", source.display()));
    write_file(&temp_repo.join(relative_path), &content);
}

fn git(temp_repo: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(temp_repo)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn init_temp_repo() -> TempDir {
    let root = repo_root();
    let temp_repo = tempfile::tempdir().expect("create temp repo");
    let repo_path = temp_repo.path();

    copy_repo_file(&root, repo_path, "scripts/check-file-size-limits.sh");
    copy_repo_file(&root, repo_path, "scripts/lib/file_size_limits.py");
    copy_repo_file(&root, repo_path, "scripts/file-size-allowlist.txt");

    write_file(&repo_path.join("README.md"), "# Temp repo\n");

    git(repo_path, &["init", "-b", "main"]);
    git(repo_path, &["config", "user.name", "Codex"]);
    git(repo_path, &["config", "user.email", "codex@example.com"]);
    git(repo_path, &["add", "."]);
    git(repo_path, &["commit", "-m", "initial"]);

    temp_repo
}

fn run_check_script(temp_repo: &Path, args: &[&str]) -> Output {
    Command::new("bash")
        .arg(temp_repo.join("scripts/check-file-size-limits.sh"))
        .args(args)
        .current_dir(temp_repo)
        .output()
        .expect("run check-file-size-limits.sh")
}

fn output_text(output: &Output) -> (String, String) {
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn check_file_size_limits_help_lists_usage_and_exit_codes() {
    let temp_repo = init_temp_repo();

    let output = run_check_script(temp_repo.path(), &["--help"]);
    let (stdout, stderr) = output_text(&output);

    assert!(
        output.status.success(),
        "expected --help to succeed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(stdout.contains("Usage:"), "missing usage block\n{stdout}");
    assert!(
        stdout.contains("Exit codes:"),
        "missing exit-codes block\n{stdout}"
    );
}

#[test]
fn check_file_size_limits_passes_when_all_files_are_within_limits() {
    let temp_repo = init_temp_repo();
    let repo_path = temp_repo.path();

    write_lines(&repo_path.join("crates/cueloop/src/lib.rs"), 12);

    let output = run_check_script(repo_path, &[]);
    let (stdout, stderr) = output_text(&output);

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected success status\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("OK: file-size limits within policy"),
        "expected success marker\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn check_file_size_limits_reports_soft_advisory_without_failing() {
    let temp_repo = init_temp_repo();
    let repo_path = temp_repo.path();

    write_lines(&repo_path.join("docs/guides/large.md"), 1501);

    let output = run_check_script(repo_path, &[]);
    let (stdout, stderr) = output_text(&output);

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected soft advisory to stay non-blocking\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("ADVISORY: soft file-size threshold exceeded:"),
        "missing soft advisory header\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("docs/guides/large.md"),
        "expected offender path in soft advisory\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn check_file_size_limits_reports_review_threshold_without_failing() {
    let temp_repo = init_temp_repo();
    let repo_path = temp_repo.path();

    write_lines(&repo_path.join("crates/cueloop/src/review.rs"), 3001);

    let output = run_check_script(repo_path, &[]);
    let (stdout, stderr) = output_text(&output);

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected review threshold to stay non-blocking\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("WARN: review file-size threshold exceeded:"),
        "missing review warning header\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("crates/cueloop/src/review.rs"),
        "expected offender path in review warning\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn check_file_size_limits_fails_on_fail_threshold_violation() {
    let temp_repo = init_temp_repo();
    let repo_path = temp_repo.path();

    write_lines(&repo_path.join("crates/cueloop/src/huge.rs"), 5001);

    let output = run_check_script(repo_path, &[]);
    let (stdout, stderr) = output_text(&output);

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected fail-threshold failure\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("ERROR: fail file-size threshold exceeded:"),
        "missing fail-error header\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("crates/cueloop/src/huge.rs"),
        "expected offender path in fail error\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn check_file_size_limits_allows_fail_threshold_with_reasoned_allowlist() {
    let temp_repo = init_temp_repo();
    let repo_path = temp_repo.path();

    write_lines(&repo_path.join("crates/cueloop/src/huge.rs"), 5001);
    write_file(
        &repo_path.join("scripts/file-size-allowlist.txt"),
        "crates/cueloop/src/huge.rs | test fixture intentionally exceeds fail threshold\n",
    );

    let output = run_check_script(repo_path, &[]);
    let (stdout, stderr) = output_text(&output);

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected reasoned allowlist to avoid failure\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("ALLOWLISTED: fail file-size threshold exceeded:"),
        "missing allowlisted header\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("test fixture intentionally exceeds fail threshold"),
        "expected allowlist reason in output\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn check_file_size_limits_rejects_allowlist_entries_without_reasons() {
    let temp_repo = init_temp_repo();
    let repo_path = temp_repo.path();

    write_file(
        &repo_path.join("scripts/file-size-allowlist.txt"),
        "crates/cueloop/src/huge.rs\n",
    );

    let output = run_check_script(repo_path, &[]);
    let (stdout, stderr) = output_text(&output);

    assert_eq!(
        output.status.code(),
        Some(2),
        "expected malformed allowlist to be a usage/config error\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("allowlist entries must use 'glob | reason'"),
        "expected allowlist format error\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn check_file_size_limits_ignores_default_generated_path_excludes() {
    let temp_repo = init_temp_repo();
    let repo_path = temp_repo.path();

    write_lines(&repo_path.join("schemas/config.schema.json"), 5001);

    let output = run_check_script(repo_path, &[]);
    let (stdout, stderr) = output_text(&output);

    assert_eq!(
        output.status.code(),
        Some(0),
        "excluded schema path should not fail policy\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        !stdout.contains("schemas/config.schema.json"),
        "excluded path should not be listed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn check_file_size_limits_ignores_default_runtime_bookkeeping_excludes() {
    let temp_repo = init_temp_repo();
    let repo_path = temp_repo.path();

    write_lines(&repo_path.join(".cueloop/done.jsonc"), 5001);
    write_lines(&repo_path.join(".cueloop/done.jsonc"), 5001);

    let output = run_check_script(repo_path, &[]);
    let (stdout, stderr) = output_text(&output);

    assert_eq!(
        output.status.code(),
        Some(0),
        "excluded runtime bookkeeping paths should not fail policy\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        !stdout.contains(".cueloop/done.jsonc"),
        "excluded legacy bookkeeping path should not be listed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        !stdout.contains(".cueloop/done.jsonc"),
        "excluded CueLoop bookkeeping path should not be listed\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn check_file_size_limits_includes_untracked_monitored_files() {
    let temp_repo = init_temp_repo();
    let repo_path = temp_repo.path();

    let untracked_path = repo_path.join("scratch/oversized.md");
    write_lines(&untracked_path, 5001);

    let git_status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .expect("run git status --porcelain");
    assert!(git_status.status.success(), "git status should succeed");
    assert!(
        String::from_utf8_lossy(&git_status.stdout).contains("?? scratch"),
        "expected oversized file to remain untracked"
    );

    let output = run_check_script(repo_path, &[]);
    let (stdout, stderr) = output_text(&output);

    assert_eq!(
        output.status.code(),
        Some(1),
        "untracked fail-threshold markdown should fail\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("scratch/oversized.md"),
        "expected untracked offender path in output\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn check_file_size_limits_supports_configurable_exclude_glob() {
    let temp_repo = init_temp_repo();
    let repo_path = temp_repo.path();

    write_lines(&repo_path.join("generated/manual-long.md"), 5001);

    let output = run_check_script(repo_path, &["--exclude-glob", "generated/**"]);
    let (stdout, stderr) = output_text(&output);

    assert_eq!(
        output.status.code(),
        Some(0),
        "custom exclude should suppress the generated path\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        !stdout.contains("generated/manual-long.md"),
        "custom-excluded path should not appear\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}

#[test]
fn check_file_size_limits_invalid_arg_exits_with_usage_error() {
    let temp_repo = init_temp_repo();

    let output = run_check_script(temp_repo.path(), &["--definitely-not-valid"]);
    let (stdout, stderr) = output_text(&output);

    assert_eq!(
        output.status.code(),
        Some(2),
        "invalid arguments should return usage exit code\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let combined = format!("{stdout}\n{stderr}").to_lowercase();
    assert!(
        combined.contains("usage:"),
        "expected usage text for invalid args\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
}
