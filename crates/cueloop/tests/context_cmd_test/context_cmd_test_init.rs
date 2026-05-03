//! `cueloop context init` integration tests.
//!
//! Purpose:
//! - `cueloop context init` integration tests.
//!
//! Responsibilities:
//! - Cover AGENTS.md creation and generated section expectations.
//! - Verify project-type detection and explicit hint behavior.
//! - Verify file overwrite and custom output-path semantics.
//!
//! Not handled here:
//! - `context validate` or `context update` behaviors.
//! - Interactive flows requiring a TTY.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Each test starts from a fresh temp repo.
//! - Generated files are asserted through on-disk content only.

use anyhow::Result;
use std::fs;

use super::context_cmd_test_support::{run_in_dir, setup_repo};

fn assert_no_false_source_doc_claims(content: &str) -> Result<()> {
    anyhow::ensure!(
        !content.contains("every new/changed source file MUST start"),
        "generated content still contains blanket source-doc mandate"
    );
    Ok(())
}

fn assert_no_makefile_contract_claims(content: &str) -> Result<()> {
    anyhow::ensure!(
        !content.contains("The Makefile is the contract"),
        "generated content still claims Makefile is the contract"
    );
    Ok(())
}

#[test]
fn context_init_creates_agents_md() -> Result<()> {
    let dir = setup_repo()?;

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["context", "init"]);
    anyhow::ensure!(status.success(), "context init failed\nstderr:\n{stderr}");

    let agents_md = dir.path().join("AGENTS.md");
    anyhow::ensure!(agents_md.exists(), "AGENTS.md was not created");

    let content = fs::read_to_string(&agents_md)?;
    anyhow::ensure!(content.contains("# Repository Guidelines"), "missing title");
    anyhow::ensure!(
        content.contains("Non-Negotiables"),
        "missing non-negotiables section"
    );
    anyhow::ensure!(
        content.contains("Repository Map"),
        "missing repository map section"
    );
    anyhow::ensure!(
        content.contains("Build, Test, and CI"),
        "missing build/test/ci section"
    );

    Ok(())
}

#[test]
fn context_init_creates_context_files() -> Result<()> {
    let dir = setup_repo()?;

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["context", "init"]);
    anyhow::ensure!(status.success(), "context init failed\nstderr:\n{stderr}");

    let agents_md = dir.path().join("AGENTS.md");
    anyhow::ensure!(agents_md.exists(), "AGENTS.md context file was not created");

    let content = fs::read_to_string(&agents_md)?;
    anyhow::ensure!(
        content.contains("# Repository Guidelines"),
        "missing title in context file"
    );
    anyhow::ensure!(
        content.contains("## Non-Negotiables"),
        "missing non-negotiables section"
    );
    anyhow::ensure!(
        content.contains("## Repository Map"),
        "missing repository map section"
    );
    anyhow::ensure!(
        content.contains("## Build, Test, and CI"),
        "missing build/test/ci section"
    );

    Ok(())
}

#[test]
fn context_init_detects_rust_project() -> Result<()> {
    let dir = setup_repo()?;
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"test-project\"",
    )?;

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["context", "init"]);
    anyhow::ensure!(status.success(), "context init failed\nstderr:\n{stderr}");

    let content = fs::read_to_string(dir.path().join("AGENTS.md"))?;
    anyhow::ensure!(content.contains("Cargo"), "Rust-specific content missing");
    Ok(())
}

#[test]
fn context_init_detects_python_project() -> Result<()> {
    let dir = setup_repo()?;
    fs::write(
        dir.path().join("pyproject.toml"),
        "[project]\nname = \"test-project\"",
    )?;

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["context", "init"]);
    anyhow::ensure!(status.success(), "context init failed\nstderr:\n{stderr}");

    let content = fs::read_to_string(dir.path().join("AGENTS.md"))?;
    anyhow::ensure!(
        content.contains("Python Conventions") || content.contains("typing expectations"),
        "Python-specific content missing"
    );
    Ok(())
}

#[test]
fn context_init_detects_typescript_project() -> Result<()> {
    let dir = setup_repo()?;
    fs::write(
        dir.path().join("package.json"),
        r#"{"name": "test-project"}"#,
    )?;

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["context", "init"]);
    anyhow::ensure!(status.success(), "context init failed\nstderr:\n{stderr}");

    let content = fs::read_to_string(dir.path().join("AGENTS.md"))?;
    anyhow::ensure!(
        content.contains("npm") || content.contains("node") || content.contains("package"),
        "TypeScript-specific content missing"
    );
    Ok(())
}

#[test]
fn context_init_detects_go_project() -> Result<()> {
    let dir = setup_repo()?;
    fs::write(dir.path().join("go.mod"), "module test-project\n\ngo 1.21")?;

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["context", "init"]);
    anyhow::ensure!(status.success(), "context init failed\nstderr:\n{stderr}");

    let content = fs::read_to_string(dir.path().join("AGENTS.md"))?;
    anyhow::ensure!(
        content.contains("go ") || content.contains("Go "),
        "Go-specific content missing"
    );
    Ok(())
}

#[test]
fn context_init_respects_force_flag() -> Result<()> {
    let dir = setup_repo()?;
    let initial_content = "# Custom AGENTS.md\n\nThis is custom content.";
    fs::write(dir.path().join("AGENTS.md"), initial_content)?;

    let (status, _stdout, _stderr) = run_in_dir(dir.path(), &["context", "init"]);
    anyhow::ensure!(
        status.success(),
        "context init should succeed when file exists"
    );

    let content = fs::read_to_string(dir.path().join("AGENTS.md"))?;
    anyhow::ensure!(
        content == initial_content,
        "content should be preserved without force"
    );

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["context", "init", "--force"]);
    anyhow::ensure!(
        status.success(),
        "context init --force failed\nstderr:\n{stderr}"
    );

    let content = fs::read_to_string(dir.path().join("AGENTS.md"))?;
    anyhow::ensure!(
        content.contains("# Repository Guidelines"),
        "content should be overwritten with force"
    );

    Ok(())
}

#[test]
fn context_init_respects_output_path() -> Result<()> {
    let dir = setup_repo()?;

    let (status, _stdout, stderr) = run_in_dir(
        dir.path(),
        &["context", "init", "--output", "docs/AGENTS.md"],
    );
    anyhow::ensure!(
        status.success(),
        "context init --output failed\nstderr:\n{stderr}"
    );

    let custom_path = dir.path().join("docs/AGENTS.md");
    anyhow::ensure!(
        custom_path.exists(),
        "AGENTS.md was not created at custom path"
    );

    let content = fs::read_to_string(&custom_path)?;
    anyhow::ensure!(content.contains("# Repository Guidelines"), "missing title");
    Ok(())
}

#[test]
fn context_init_respects_project_type_hint() -> Result<()> {
    let dir = setup_repo()?;

    let (status, _stdout, stderr) =
        run_in_dir(dir.path(), &["context", "init", "--project-type", "rust"]);
    anyhow::ensure!(
        status.success(),
        "context init --project-type failed\nstderr:\n{stderr}"
    );

    let content = fs::read_to_string(dir.path().join("AGENTS.md"))?;
    anyhow::ensure!(
        content.contains("cargo") || content.contains("rust") || content.contains("clippy"),
        "Rust-specific content missing"
    );

    Ok(())
}

#[test]
fn context_init_generic_repo_uses_todos_not_false_contracts() -> Result<()> {
    let dir = setup_repo()?;

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["context", "init"]);
    anyhow::ensure!(status.success(), "context init failed\nstderr:\n{stderr}");

    let content = fs::read_to_string(dir.path().join("AGENTS.md"))?;
    assert_no_false_source_doc_claims(&content)?;
    assert_no_makefile_contract_claims(&content)?;
    anyhow::ensure!(
        content.contains("No repo-specific command contract detected"),
        "generic repo should explain missing command contract"
    );
    anyhow::ensure!(
        content.contains("TODO: record this repo's CI command."),
        "generic repo should render TODO command placeholders"
    );

    Ok(())
}

#[test]
fn context_init_rust_repo_without_makefile_uses_verified_defaults() -> Result<()> {
    let dir = setup_repo()?;
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"test-project\"\nversion = \"0.1.0\"\n",
    )?;

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["context", "init"]);
    anyhow::ensure!(status.success(), "context init failed\nstderr:\n{stderr}");

    let content = fs::read_to_string(dir.path().join("AGENTS.md"))?;
    assert_no_false_source_doc_claims(&content)?;
    assert_no_makefile_contract_claims(&content)?;
    anyhow::ensure!(
        content.contains("common Rust verification suite"),
        "rust repo should render Rust default note when no repo command contract is detected"
    );
    anyhow::ensure!(
        content.contains("cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace"),
        "rust repo should render the fallback Rust verification suite"
    );

    Ok(())
}

#[test]
fn context_init_makefile_repo_only_lists_detected_targets() -> Result<()> {
    let dir = setup_repo()?;
    fs::write(
        dir.path().join("Makefile"),
        ".PHONY: ci test fmt\nci:\n\t@echo ci\ntest:\n\t@echo test\nfmt:\n\t@echo fmt\n",
    )?;

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["context", "init"]);
    anyhow::ensure!(status.success(), "context init failed\nstderr:\n{stderr}");

    let content = fs::read_to_string(dir.path().join("AGENTS.md"))?;
    assert_no_false_source_doc_claims(&content)?;
    assert_no_makefile_contract_claims(&content)?;
    anyhow::ensure!(
        content.contains("Detected from Makefile targets"),
        "makefile repo should explain detected command source"
    );
    anyhow::ensure!(
        content.contains("`make ci` — detected from `Makefile` targets."),
        "makefile repo should render detected ci target"
    );
    anyhow::ensure!(
        content.contains("`make test` — detected from `Makefile` targets."),
        "makefile repo should render detected test target"
    );
    anyhow::ensure!(
        content.contains("`make fmt` — detected from `Makefile` targets."),
        "makefile repo should render detected format target"
    );
    anyhow::ensure!(
        !content.contains("make install")
            && !content.contains("make update")
            && !content.contains("make clean"),
        "makefile repo should not invent absent make targets"
    );

    Ok(())
}

#[test]
fn context_init_typescript_package_scripts_detected() -> Result<()> {
    let dir = setup_repo()?;
    fs::write(
        dir.path().join("package.json"),
        r#"{
  "name": "test-project",
  "scripts": {
    "ci": "vitest run && tsc -b",
    "build": "tsc -b",
    "test": "vitest run",
    "lint": "eslint .",
    "format": "prettier --check ."
  }
}"#,
    )?;
    fs::write(dir.path().join("pnpm-lock.yaml"), "lockfileVersion: '9.0'")?;

    let (status, _stdout, stderr) = run_in_dir(dir.path(), &["context", "init"]);
    anyhow::ensure!(status.success(), "context init failed\nstderr:\n{stderr}");

    let content = fs::read_to_string(dir.path().join("AGENTS.md"))?;
    assert_no_false_source_doc_claims(&content)?;
    assert_no_makefile_contract_claims(&content)?;
    anyhow::ensure!(
        content.contains("Detected from package.json scripts"),
        "package-script repo should explain detected command source"
    );
    anyhow::ensure!(
        content.contains("`pnpm run ci` — detected from `package.json` scripts."),
        "typescript repo should detect ci script"
    );
    anyhow::ensure!(
        content.contains("`pnpm run build` — detected from `package.json` scripts."),
        "typescript repo should detect build script"
    );
    anyhow::ensure!(
        content.contains("`pnpm run format` — detected from `package.json` scripts."),
        "typescript repo should detect format script"
    );

    Ok(())
}
