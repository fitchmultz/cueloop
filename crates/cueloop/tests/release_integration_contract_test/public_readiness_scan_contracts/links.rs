//! Public-readiness scan link-resolution contracts.
//!
//! Purpose:
//! - Keep markdown target and symlink traversal coverage grouped together.
//!
//! Responsibilities:
//! - Verify escaping-target rejection, in-repo symlink handling, and allowlisted `.ralph` link scanning.
//!
//! Scope:
//! - Limited to public-readiness link-scanning contracts.
//!
//! Usage:
//! - Loaded by `public_readiness_scan_contracts.rs`.
//!
//! Invariants/Assumptions:
//! - These tests must preserve the existing public-readiness helper contract assertions verbatim.

use std::process::Command;

use super::super::support::{
    copy_public_readiness_scan_fixture, public_readiness_scan_python_path, write_file,
};

#[test]
fn public_readiness_scan_rejects_markdown_targets_outside_repo() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path().join("repo");
    std::fs::create_dir(&repo_root).expect("create temp repo root");
    std::fs::write(repo_root.join("README.md"), "[outside](../outside.md)\n")
        .expect("write markdown fixture");
    std::fs::write(temp_dir.path().join("outside.md"), "outside\n")
        .expect("write escaped target fixture");

    let output = Command::new("python3")
        .arg(public_readiness_scan_python_path())
        .arg("links")
        .arg(&repo_root)
        .env("RALPH_PUBLIC_SCAN_EXCLUDES", "")
        .output()
        .expect("run public-readiness scan helper");

    assert_eq!(
        output.status.code(),
        Some(1),
        "public-readiness scan should reject markdown targets that escape the repo root"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("target escapes repo root"),
        "scanner should explain why escaped markdown targets are invalid"
    );
}

#[cfg(unix)]
#[test]
fn public_readiness_scan_ignores_symlinked_repo_files_that_escape_repo() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path().join("repo");
    std::fs::create_dir(&repo_root).expect("create temp repo root");
    let outside_markdown = temp_dir.path().join("outside.md");
    std::fs::write(&outside_markdown, "[outside](../outside.md)\n")
        .expect("write symlink target fixture");
    symlink(&outside_markdown, repo_root.join("README.md")).expect("create markdown symlink");

    let output = Command::new("python3")
        .arg(public_readiness_scan_python_path())
        .arg("links")
        .arg(&repo_root)
        .env("RALPH_PUBLIC_SCAN_EXCLUDES", "")
        .output()
        .expect("run public-readiness scan helper");

    assert_eq!(
        output.status.code(),
        Some(0),
        "public-readiness scan should skip symlinked files instead of following them outside the repo"
    );
    assert!(
        output.stdout.is_empty(),
        "skipped symlinked files should not produce findings"
    );
}

#[cfg(unix)]
#[test]
fn public_readiness_scan_skips_symlinks_into_excluded_repo_paths() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path().join("repo");
    std::fs::create_dir(&repo_root).expect("create temp repo root");
    let excluded_dir = repo_root.join(".ralph/cache");
    std::fs::create_dir_all(&excluded_dir).expect("create excluded dir");
    let secret_value = ["sk_live_", "abcdefghijklmnop"].concat();
    std::fs::write(
        excluded_dir.join("secret.md"),
        format!("{}\n", secret_value),
    )
    .expect("write excluded secret fixture");
    symlink(excluded_dir.join("secret.md"), repo_root.join("README.md"))
        .expect("create excluded-path symlink");

    let output = Command::new("python3")
        .arg(public_readiness_scan_python_path())
        .arg("secrets")
        .arg(&repo_root)
        .env("RALPH_PUBLIC_SCAN_EXCLUDES", ".ralph/cache/")
        .output()
        .expect("run public-readiness scan helper");

    assert_eq!(
        output.status.code(),
        Some(0),
        "public-readiness scan should not follow symlinks into excluded repo paths"
    );
    assert!(
        output.stdout.is_empty(),
        "excluded symlink targets should not produce findings"
    );
}

#[cfg(unix)]
#[test]
fn public_readiness_scan_scans_symlinked_repo_files_that_resolve_within_repo() {
    use std::os::unix::fs::symlink;

    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path().join("repo");
    std::fs::create_dir(&repo_root).expect("create temp repo root");
    let docs_dir = repo_root.join("docs");
    std::fs::create_dir(&docs_dir).expect("create docs dir");
    std::fs::write(docs_dir.join("source.txt"), "[broken](missing.md)\n")
        .expect("write symlinked markdown source");
    std::fs::write(repo_root.join("missing.md"), "present\n")
        .expect("write misleading repo-root target");
    symlink(docs_dir.join("source.txt"), repo_root.join("README.md"))
        .expect("create in-repo markdown symlink");

    let output = Command::new("python3")
        .arg(public_readiness_scan_python_path())
        .arg("links")
        .arg(&repo_root)
        .env("RALPH_PUBLIC_SCAN_EXCLUDES", "")
        .output()
        .expect("run public-readiness scan helper");

    assert_eq!(
        output.status.code(),
        Some(1),
        "public-readiness scan should still inspect symlinked files that resolve within the repo"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        "README.md: missing target -> missing.md",
        "scanner should resolve symlinked markdown links from the file's canonical location"
    );
}

#[test]
fn public_readiness_scan_scans_allowlisted_ralph_markdown_links() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let repo_root = temp_dir.path();

    copy_public_readiness_scan_fixture(repo_root);
    write_file(
        &repo_root.join(".ralph/README.md"),
        "[broken](./definitely-missing-file.md)\n",
    );

    let output = Command::new("bash")
        .arg(repo_root.join("scripts/lib/public_readiness_scan.sh"))
        .arg("links")
        .current_dir(repo_root)
        .output()
        .expect("run public-readiness link scan over allowlisted .ralph file");

    assert_eq!(
        output.status.code(),
        Some(1),
        "public-readiness scan should inspect allowlisted .ralph markdown files"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(".ralph/README.md: missing target -> ./definitely-missing-file.md"),
        "link scan should report missing targets inside allowlisted .ralph files\nstdout:\n{}",
        stdout
    );
}
