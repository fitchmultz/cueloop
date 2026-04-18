//! Agent CI surface classifier contract tests.
//!
//! Responsibilities:
//! - Verify the classifier reasons about committed branch delta, not only dirty files.
//! - Guard representative path routing for `ci-docs`, `ci-fast`, `ci`, and `macos-ci`.
//!
//! Not handled here:
//! - Executing the Makefile targets selected by the classifier.
//! - Exhaustive path-matrix testing for every policy branch.
//!
//! Invariants/assumptions:
//! - The classifier script and shared shell libs live at stable repo-relative paths.
//! - Git is available locally for temporary repository setup.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

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

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent directories");
    }
    std::fs::write(path, content).unwrap_or_else(|err| panic!("write {}: {err}", path.display()));
}

fn copy_script(repo_root: &Path, temp_repo: &Path, relative_path: &str) {
    let src = repo_root.join(relative_path);
    let dest = temp_repo.join(relative_path);
    let content =
        std::fs::read_to_string(&src).unwrap_or_else(|err| panic!("read {}: {err}", src.display()));
    write_file(&dest, &content);
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

fn run_classifier(temp_repo: &Path, mode: &str) -> String {
    let output = Command::new("bash")
        .arg(temp_repo.join("scripts/agent-ci-surface.sh"))
        .arg(mode)
        .current_dir(temp_repo)
        .output()
        .expect("run agent-ci classifier");
    assert!(
        output.status.success(),
        "classifier {mode} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn init_temp_repo() -> TempDir {
    let repo_root = repo_root();
    let temp_repo = tempfile::tempdir().expect("create temp repo");
    let repo_path = temp_repo.path();

    copy_script(&repo_root, repo_path, "scripts/agent-ci-surface.sh");
    copy_script(&repo_root, repo_path, "scripts/lib/ralph-shell.sh");
    copy_script(&repo_root, repo_path, "scripts/lib/release_policy.sh");

    write_file(&repo_path.join("README.md"), "# Temp repo\n");
    write_file(&repo_path.join("docs/guide.md"), "# Docs\n");
    write_file(&repo_path.join("Makefile"), "help:\n\t@echo ok\n");
    write_file(
        &repo_path.join("crates/ralph/src/lib.rs"),
        "// stub crate\n",
    );

    git(repo_path, &["init", "-b", "main"]);
    git(repo_path, &["config", "user.name", "Codex"]);
    git(repo_path, &["config", "user.email", "codex@example.com"]);
    git(repo_path, &["add", "."]);
    git(repo_path, &["commit", "-m", "initial"]);

    temp_repo
}

#[test]
fn classifier_routes_clean_docs_only_branch_delta_to_ci_docs() {
    let temp_repo = init_temp_repo();
    let repo_path = temp_repo.path();

    git(repo_path, &["checkout", "-b", "feature/docs-only"]);
    write_file(&repo_path.join("docs/guide.md"), "# Docs\n\nupdated\n");
    git(repo_path, &["add", "docs/guide.md"]);
    git(repo_path, &["commit", "-m", "docs only"]);

    assert_eq!(run_classifier(repo_path, "--target"), "ci-docs");
}

#[test]
fn classifier_routes_clean_non_app_branch_delta_to_ci_fast() {
    let temp_repo = init_temp_repo();
    let repo_path = temp_repo.path();

    git(repo_path, &["checkout", "-b", "feature/non-app"]);
    write_file(&repo_path.join(".gitignore"), "target/\n");
    git(repo_path, &["add", ".gitignore"]);
    git(repo_path, &["commit", "-m", "touch non-app surface"]);

    assert_eq!(run_classifier(repo_path, "--target"), "ci-fast");
    assert!(
        run_classifier(repo_path, "--reason").contains("Rust/CLI verification"),
        "expected Rust/CLI routing explanation"
    );
}

#[test]
fn classifier_routes_clean_makefile_branch_delta_to_macos_ci() {
    let temp_repo = init_temp_repo();
    let repo_path = temp_repo.path();

    git(repo_path, &["checkout", "-b", "feature/build-surface"]);
    write_file(&repo_path.join("Makefile"), "help:\n\t@echo changed\n");
    git(repo_path, &["add", "Makefile"]);
    git(repo_path, &["commit", "-m", "touch build surface"]);

    assert_eq!(run_classifier(repo_path, "--target"), "macos-ci");
    assert!(
        run_classifier(repo_path, "--reason").contains("dependency-surface change"),
        "expected dependency-surface explanation"
    );
}

#[test]
fn classifier_routes_clean_main_without_branch_delta_to_ci_fast() {
    let temp_repo = init_temp_repo();
    let repo_path = temp_repo.path();

    assert_eq!(run_classifier(repo_path, "--target"), "ci-fast");
}

#[test]
fn classifier_routes_clean_crates_branch_delta_to_ci() {
    let temp_repo = init_temp_repo();
    let repo_path = temp_repo.path();

    git(repo_path, &["checkout", "-b", "feature/rust-surface"]);
    write_file(
        &repo_path.join("crates/ralph/src/lib.rs"),
        "// stub crate\n\npub fn touched() {}\n",
    );
    git(repo_path, &["add", "crates/ralph/src/lib.rs"]);
    git(repo_path, &["commit", "-m", "touch rust surface"]);

    assert_eq!(run_classifier(repo_path, "--target"), "ci");
    assert!(
        run_classifier(repo_path, "--reason").contains("Rust crate"),
        "expected Rust release gate routing explanation"
    );
}

#[test]
fn classifier_routes_schemas_branch_delta_to_macos_ci() {
    let temp_repo = init_temp_repo();
    let repo_path = temp_repo.path();

    git(repo_path, &["checkout", "-b", "feature/schema"]);
    write_file(&repo_path.join("schemas/config.schema.json"), "{}\n");
    git(repo_path, &["add", "schemas/config.schema.json"]);
    git(repo_path, &["commit", "-m", "touch schema"]);

    assert_eq!(run_classifier(repo_path, "--target"), "macos-ci");
}

#[test]
fn classifier_routes_apps_ralphmac_branch_delta_to_macos_ci() {
    let temp_repo = init_temp_repo();
    let repo_path = temp_repo.path();

    git(repo_path, &["checkout", "-b", "feature/swift"]);
    write_file(
        &repo_path.join("apps/RalphMac/Stub.swift"),
        "// placeholder\n",
    );
    git(repo_path, &["add", "apps/RalphMac/Stub.swift"]);
    git(repo_path, &["commit", "-m", "touch app surface"]);

    assert_eq!(run_classifier(repo_path, "--target"), "macos-ci");
}

#[test]
fn classifier_emit_eval_exports_assignments() {
    let temp_repo = init_temp_repo();
    let repo_path = temp_repo.path();

    let output = Command::new("bash")
        .arg(repo_path.join("scripts/agent-ci-surface.sh"))
        .arg("--emit-eval")
        .current_dir(repo_path)
        .output()
        .expect("run emit-eval");

    assert!(
        output.status.success(),
        "emit-eval failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("RALPH_AGENT_CI_TARGET=") && stdout.contains("ci-fast"),
        "emit-eval should assign RALPH_AGENT_CI_TARGET=ci-fast on clean tree:\n{stdout}"
    );
    assert!(
        stdout.contains("RALPH_AGENT_CI_REASON="),
        "emit-eval should assign RALPH_AGENT_CI_REASON:\n{stdout}"
    );
}
