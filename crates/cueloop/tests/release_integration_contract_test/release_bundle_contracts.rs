//! Xcode project, Makefile, and release pipeline bundling contracts.
//!
//! Purpose:
//! - Xcode project, Makefile, and release pipeline bundling contracts.
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

use std::path::Path;

use super::support::{read_repo_file, swift_file_names};

fn collect_cli_input_files(path: &Path, out: &mut Vec<String>) {
    let entries =
        std::fs::read_dir(path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    for entry in entries {
        let entry = entry.expect("read directory entry");
        let child = entry.path();
        if child.is_dir() {
            collect_cli_input_files(&child, out);
            continue;
        }
        let rel = child
            .strip_prefix(super::support::repo_root())
            .expect("path should be under repo root")
            .to_string_lossy()
            .replace('\\', "/");
        let include = rel == "crates/cueloop/Cargo.toml"
            || rel == "crates/cueloop/build.rs"
            || (rel.starts_with("crates/cueloop/src/") && rel.ends_with(".rs"))
            || rel.starts_with("crates/cueloop/assets/");
        if include {
            out.push(format!("$(SRCROOT)/../../{rel}"));
        }
    }
}

#[test]
fn xcode_project_references_all_committed_swift_sources() {
    let project = read_repo_file("apps/CueLoopMac/CueLoopMac.xcodeproj/project.pbxproj");

    for relative_dir in [
        "apps/CueLoopMac/CueLoopCore",
        "apps/CueLoopMac/CueLoopCoreTests",
        "apps/CueLoopMac/CueLoopMac",
        "apps/CueLoopMac/CueLoopMacUITests",
    ] {
        for file_name in swift_file_names(relative_dir) {
            let file_ref_marker = format!("/* {file_name} */");
            let build_marker = format!("/* {file_name} in Sources */");
            assert!(
                project.contains(&file_ref_marker),
                "Xcode project is missing file reference for {relative_dir}/{file_name}"
            );
            assert!(
                project.contains(&build_marker),
                "Xcode project is missing Sources membership for {relative_dir}/{file_name}"
            );
        }
    }
}

#[test]
fn xcode_build_phase_uses_shared_cli_bundle_entrypoint() {
    let project = read_repo_file("apps/CueLoopMac/CueLoopMac.xcodeproj/project.pbxproj");
    assert!(
        project.contains("scripts/cueloop-cli-bundle.sh"),
        "Xcode project should call the shared CLI bundling script"
    );
    assert!(
        !project.contains("cargo ${BUILD_ARGS}") && !project.contains("target/debug/cueloop"),
        "Xcode project should not embed its own Cargo invocation policy or debug hardcoded CLI paths"
    );
    assert!(
        project.contains("cueloop-cli-bundle.sh") && !project.contains("target/release/cueloop"),
        "Release should always route through cueloop-cli-bundle.sh instead of copying a possibly stale target/release CLI"
    );
    assert!(
        project.contains("CueLoopCLIInputs.xcfilelist"),
        "The Xcode bundle phase should use a Rust input file list so dependency analysis can run only when the embedded CLI inputs change"
    );
    assert!(
        !project.contains("alwaysOutOfDate = 1;"),
        "The Xcode bundle phase should not be forced out of date when it has a committed Rust input file list"
    );
}

#[test]
fn shared_cli_bundle_script_supports_configuration_and_bundle_dir() {
    let script = read_repo_file("scripts/cueloop-cli-bundle.sh");
    assert!(
        script.contains("--configuration") && script.contains("--bundle-dir"),
        "shared CLI bundle script should accept configuration and bundle destination inputs"
    );
    assert!(
        script.contains("PRIMARY_BIN_NAME=\"cueloop\"")
            && !script.contains(&format!("{}{}", "LEGACY", "_BIN_NAME=")),
        "shared CLI bundle script should build only the primary cueloop binary"
    );
    assert!(
        script.contains("cueloop_activate_pinned_rust_toolchain"),
        "shared CLI bundle script should honor the pinned rustup toolchain"
    );
    assert!(
        script.contains("--target") && script.contains("--jobs"),
        "shared CLI bundle script should act as the canonical build entrypoint for both native and cross-target builds"
    );
    assert!(
        !script.contains("CUELOOP_BIN_PATH"),
        "shared CLI bundle script should not allow callers to bypass the canonical build contract with an arbitrary binary override"
    );
}

#[test]
fn release_pipeline_uses_github_draft_then_publish_flow() {
    let script = read_repo_file("scripts/lib/release_publish_pipeline.sh");
    assert!(
        script.contains("gh release create \"v$VERSION\"")
            && script.contains("--draft")
            && script.contains("gh release edit \"v$VERSION\" --draft=false"),
        "release publish pipeline should prepare a draft release before final publication"
    );
    assert!(
        script.find("gh release create \"v$VERSION\"")
            < script.find("cargo publish -p \"$CRATE_PACKAGE_NAME\" --locked"),
        "GitHub draft preparation should happen before crates.io publish"
    );
    assert!(
        script.find("cargo publish -p \"$CRATE_PACKAGE_NAME\" --locked")
            < script.find("gh release edit \"v$VERSION\" --draft=false"),
        "GitHub release publication should happen only after crates.io publish"
    );
}

#[test]
fn makefile_release_build_uses_shared_bundle_entrypoint() {
    let make_surface = format!(
        "{}\n{}",
        read_repo_file("Makefile"),
        read_repo_file("mk/rust.mk")
    );
    assert!(
        make_surface.contains("scripts/cueloop-cli-bundle.sh --configuration Release"),
        "Makefile release builds should route through the shared CLI bundling entrypoint"
    );
    assert!(
        !make_surface.contains("cargo build --workspace --release --locked"),
        "Makefile should not keep a separate direct cargo release-build path"
    );
    assert!(
        !make_surface.contains("publish-crate:"),
        "Makefile should not expose a direct crates.io publish bypass outside the release transaction"
    );
}

#[test]
fn xcode_cli_input_file_list_matches_committed_cli_inputs() {
    let mut expected = vec![
        "$(SRCROOT)/../../Cargo.toml".to_string(),
        "$(SRCROOT)/../../Cargo.lock".to_string(),
        "$(SRCROOT)/../../VERSION".to_string(),
        "$(SRCROOT)/../../rust-toolchain.toml".to_string(),
        "$(SRCROOT)/../../scripts/cueloop-cli-bundle.sh".to_string(),
        "$(SRCROOT)/../../scripts/lib/cueloop-shell.sh".to_string(),
    ];
    collect_cli_input_files(
        &super::support::repo_root().join("crates/cueloop"),
        &mut expected,
    );
    expected.sort();

    let mut actual = read_repo_file("apps/CueLoopMac/CueLoopCLIInputs.xcfilelist")
        .lines()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    actual.sort();

    assert_eq!(
        actual, expected,
        "apps/CueLoopMac/CueLoopCLIInputs.xcfilelist must stay synchronized with CLI source/assets used by the bundled app"
    );
}

#[test]
fn makefile_xcode_derived_data_cleanup_is_explicit_not_default() {
    let make_surface = format!(
        "{}\n{}",
        read_repo_file("Makefile"),
        read_repo_file("mk/macos.mk")
    );
    assert!(
        make_surface.contains("CUELOOP_XCODE_CLEAN_DERIVED_DATA ?= 0"),
        "local Xcode builds should keep DerivedData by default and clean only when explicitly requested"
    );
    assert!(
        make_surface.contains("macos-build-clean:")
            && make_surface.contains("macos-test-clean:")
            && make_surface.contains("macos-ci-clean:"),
        "Makefile should expose explicit clean Xcode targets instead of hiding cache deletion in normal builds"
    );
    assert!(
        make_surface.contains("if [ \"$(CUELOOP_XCODE_CLEAN_DERIVED_DATA)\" = \"1\" ]; then rm -rf \"$$derived_data_path\""),
        "DerivedData deletion should be guarded by the positive clean flag, not an inverted keep flag"
    );
    assert!(
        !make_surface.contains("CUELOOP_XCODE_KEEP_DERIVED_DATA"),
        "the old inverted keep flag should be removed instead of kept as confusing compatibility behavior"
    );
}

#[test]
fn makefile_does_not_expose_release_verify_as_dry_run() {
    let make_surface = format!(
        "{}\n{}",
        read_repo_file("Makefile"),
        read_repo_file("mk/rust.mk")
    );
    assert!(
        make_surface.contains("release-verify:"),
        "Makefile should keep release-verify as the canonical release verification target"
    );
    assert!(
        !make_surface.contains("release-dry-run"),
        "Makefile should not expose or advertise release-dry-run because release verification mutates local release metadata"
    );
    assert!(
        make_surface.contains("Mutating local preflight: prepares the exact release snapshot that make release will publish"),
        "Makefile help should describe release-verify as a mutating local preflight"
    );
}
