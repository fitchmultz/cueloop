//! Release publish-pipeline runtime contract tests.
//!
//! Responsibilities:
//! - Exercise shell helpers in `scripts/lib/release_publish_pipeline.sh` with a fake `gh` CLI.
//! - Guard missing-release probing so it degrades to `missing` without parser tracebacks.
//!
//! Not handled here:
//! - Real GitHub or crates.io interactions.
//! - End-to-end release execution.
//!
//! Invariants/assumptions:
//! - Bash is available for sourcing the release publish pipeline.
//! - Tests can override `PATH` with a fake `gh` executable.

use std::ffi::OsStr;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

fn repo_root() -> PathBuf {
    let exe = std::env::current_exe().expect("resolve current test executable path");
    let exe_dir = exe
        .parent()
        .expect("test executable should have a parent directory");
    let profile_dir = if exe_dir.file_name() == Some(OsStr::new("deps")) {
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

fn write_fake_gh(temp_dir: &Path) -> PathBuf {
    let gh_path = temp_dir.join("gh");
    fs::write(
        &gh_path,
        r#"#!/usr/bin/env bash
set -euo pipefail

mode="${FAKE_GH_MODE:-missing}"
if [ "${1:-}" != "release" ] || [ "${2:-}" != "view" ]; then
  echo "unexpected gh invocation: $*" >&2
  exit 64
fi

case "$mode" in
  missing)
    exit 1
    ;;
  draft)
    printf 'true\n'
    ;;
  published)
    printf 'false\n'
    ;;
  *)
    echo "unsupported FAKE_GH_MODE=$mode" >&2
    exit 65
    ;;
esac
"#,
    )
    .expect("write fake gh script");

    let mut permissions = fs::metadata(&gh_path)
        .expect("stat fake gh script")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&gh_path, permissions).expect("chmod fake gh script");
    gh_path
}

fn run_release_query(mode: &str) -> (String, String) {
    let temp_dir = TempDir::new().expect("create temp dir");
    let fake_gh_path = write_fake_gh(temp_dir.path());
    let repo_root = repo_root();
    let script_path = repo_root.join("scripts/lib/release_publish_pipeline.sh");
    let inherited_path = std::env::var("PATH").unwrap_or_default();
    let combined_path = format!("{}:{}", temp_dir.path().display(), inherited_path);

    let output = Command::new("bash")
        .arg("-c")
        .arg(format!(
            "source '{}' && VERSION=0.3.0 && release_query_github_release_state",
            script_path.display()
        ))
        .env("FAKE_GH_MODE", mode)
        .env("PATH", &combined_path)
        .output()
        .expect("run release_query_github_release_state");

    assert!(
        output.status.success(),
        "expected helper to succeed for mode {mode}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fake_gh_path;

    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn release_query_reports_missing_without_traceback_noise() {
    let (stdout, stderr) = run_release_query("missing");
    assert_eq!(stdout.trim(), "missing");
    assert!(
        stderr.trim().is_empty(),
        "missing release lookup should stay quiet\nstderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("Traceback") && !stderr.contains("JSONDecodeError"),
        "missing release lookup should not leak parser tracebacks\nstderr:\n{stderr}"
    );
}

#[test]
fn release_query_maps_draft_and_published_states() {
    let (draft_stdout, draft_stderr) = run_release_query("draft");
    assert_eq!(draft_stdout.trim(), "draft");
    assert!(
        draft_stderr.trim().is_empty(),
        "draft stderr:\n{draft_stderr}"
    );

    let (published_stdout, published_stderr) = run_release_query("published");
    assert_eq!(published_stdout.trim(), "published");
    assert!(
        published_stderr.trim().is_empty(),
        "published stderr:\n{published_stderr}"
    );
}
