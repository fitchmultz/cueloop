//! Init command tests.
//!
//! Purpose:
//! - Init command tests.
//!
//! Responsibilities:
//! - Provide focused implementation or regression coverage for this file's owning feature.
//!
//! Scope:
//! - Limited to this file's owning feature boundary.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/Assumptions:
//! - Keep behavior aligned with CueLoop's canonical CLI, machine-contract, and queue semantics.

use super::*;

#[test]
fn init_creates_agents_md() -> Result<()> {
    let dir = TempDir::new()?;
    let resolved = create_test_resolved(&dir);
    fs::create_dir_all(resolved.repo_root.join("src"))?;

    let output_path = resolved.repo_root.join("AGENTS.md");
    let report = run_context_init(
        &resolved,
        ContextInitOptions {
            force: false,
            project_type_hint: None,
            output_path: output_path.clone(),
            interactive: false,
        },
    )?;

    assert_eq!(report.status, FileInitStatus::Created);
    assert!(output_path.exists());

    let content = fs::read_to_string(&output_path)?;
    assert!(content.contains("# Repository Guidelines"));
    assert!(content.contains("Non-Negotiables"));
    assert!(content.contains("Repository Map"));

    Ok(())
}

#[test]
fn init_skips_existing_without_force() -> Result<()> {
    let dir = TempDir::new()?;
    let resolved = create_test_resolved(&dir);

    let output_path = resolved.repo_root.join("AGENTS.md");
    fs::write(&output_path, "existing content")?;

    let report = run_context_init(
        &resolved,
        ContextInitOptions {
            force: false,
            project_type_hint: None,
            output_path: output_path.clone(),
            interactive: false,
        },
    )?;

    assert_eq!(report.status, FileInitStatus::Valid);
    let content = fs::read_to_string(&output_path)?;
    assert_eq!(content, "existing content");

    Ok(())
}

#[test]
fn init_overwrites_with_force() -> Result<()> {
    let dir = TempDir::new()?;
    let resolved = create_test_resolved(&dir);

    let output_path = resolved.repo_root.join("AGENTS.md");
    fs::write(&output_path, "existing content")?;

    let report = run_context_init(
        &resolved,
        ContextInitOptions {
            force: true,
            project_type_hint: None,
            output_path: output_path.clone(),
            interactive: false,
        },
    )?;

    assert_eq!(report.status, FileInitStatus::Created);
    let content = fs::read_to_string(&output_path)?;
    assert!(content.contains("# Repository Guidelines"));

    Ok(())
}

#[test]
fn init_templates_avoid_blanket_source_doc_claims() -> Result<()> {
    for (project_type, seed_file, seed_contents) in [
        (DetectedProjectType::Generic, None, None),
        (
            DetectedProjectType::Rust,
            Some("Cargo.toml"),
            Some("[package]\nname = \"test-project\"\nversion = \"0.1.0\"\n"),
        ),
        (
            DetectedProjectType::Python,
            Some("pyproject.toml"),
            Some("[project]\nname = \"test-project\"\nversion = \"0.1.0\"\n"),
        ),
        (
            DetectedProjectType::TypeScript,
            Some("package.json"),
            Some("{\"name\":\"test-project\"}"),
        ),
        (
            DetectedProjectType::Go,
            Some("go.mod"),
            Some("module test-project\n\ngo 1.21\n"),
        ),
    ] {
        let dir = TempDir::new()?;
        let resolved = create_test_resolved(&dir);
        if let (Some(seed_file), Some(seed_contents)) = (seed_file, seed_contents) {
            fs::write(resolved.repo_root.join(seed_file), seed_contents)?;
        }

        let output_path = resolved.repo_root.join("AGENTS.md");
        run_context_init(
            &resolved,
            ContextInitOptions {
                force: false,
                project_type_hint: None,
                output_path: output_path.clone(),
                interactive: false,
            },
        )?;

        let content = fs::read_to_string(&output_path)?;
        assert!(
            !content.contains("every new/changed source file MUST start"),
            "blanket source-doc claim returned for {project_type:?}"
        );
    }

    Ok(())
}
