//! Release script contract tests.
//!
//! These tests guard release-script invariants that should not regress silently.
//! They do not run an end-to-end release or require GitHub/crates.io credentials.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    process::Command,
};

fn repo_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .expect("crate directory should have a parent")
        .parent()
        .expect("crates directory should have a parent repo root")
        .to_path_buf()
}

fn read_repo_file(relative_path: &str) -> String {
    std::fs::read_to_string(repo_root().join(relative_path))
        .unwrap_or_else(|err| panic!("failed to read {relative_path}: {err}"))
}

fn parse_shell_string_arrays(script: &str) -> BTreeMap<String, Vec<String>> {
    let mut arrays = BTreeMap::new();
    let mut lines = script.lines();

    while let Some(line) = lines.next() {
        let Some(name) = line.trim().strip_suffix("=(") else {
            continue;
        };

        let mut entries = Vec::new();
        for array_line in lines.by_ref() {
            let array_line = array_line.trim();
            if array_line == ")" {
                break;
            }
            if let Some(entry) = array_line
                .strip_prefix('"')
                .and_then(|line| line.strip_suffix('"'))
            {
                entries.push(entry.to_owned());
            }
        }
        arrays.insert(name.to_owned(), entries);
    }

    arrays
}

fn parse_shell_string_array(script: &str, name: &str) -> Vec<String> {
    parse_shell_string_arrays(script)
        .remove(name)
        .unwrap_or_else(|| panic!("release policy should define {name}"))
}

fn parse_release_metadata_paths(script: &str) -> BTreeSet<String> {
    parse_shell_string_array(script, "RELEASE_METADATA_PATHS")
        .into_iter()
        .collect()
}

fn path_with_prefix(prefix: &std::path::Path) -> String {
    let old_path = std::env::var_os("PATH").unwrap_or_default();
    let separator = if cfg!(windows) { ";" } else { ":" };
    format!(
        "{}{}{}",
        prefix.display(),
        separator,
        old_path.to_string_lossy()
    )
}

fn parse_makefile_generated_schema_paths(makefile: &str) -> BTreeSet<String> {
    makefile
        .lines()
        .filter_map(|line| {
            let (_, output) = line.split_once('>')?;
            let path = output.trim();
            (path.starts_with("schemas/") && path.ends_with(".schema.json"))
                .then(|| path.to_owned())
        })
        .collect()
}

#[test]
fn release_policy_arrays_do_not_repeat_entries() {
    let script = read_repo_file("scripts/lib/release_policy.sh");
    let arrays = parse_shell_string_arrays(&script);

    assert!(
        !arrays.is_empty(),
        "release policy should define shell string arrays"
    );

    let mut duplicates_by_array = BTreeMap::new();
    for (array, entries) in arrays {
        let mut seen = BTreeSet::new();
        let duplicates: Vec<_> = entries
            .into_iter()
            .filter(|entry| !seen.insert(entry.clone()))
            .collect();
        if !duplicates.is_empty() {
            duplicates_by_array.insert(array, duplicates);
        }
    }

    assert!(
        duplicates_by_array.is_empty(),
        "release policy arrays should not contain duplicate entries: {duplicates_by_array:?}"
    );
}

#[test]
fn generate_changelog_modes_fail_when_git_cliff_fails() {
    let temp = tempfile::tempdir().expect("tempdir");
    let stub_path = temp.path().join("git-cliff");
    std::fs::write(
        &stub_path,
        "#!/usr/bin/env bash\necho git-cliff failed >&2\nexit 42\n",
    )
    .expect("write stub");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&stub_path)
            .expect("metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&stub_path, permissions).expect("chmod stub");
    }

    for mode in ["--check", "--dry-run"] {
        let output = Command::new("bash")
            .arg("scripts/generate-changelog.sh")
            .arg(mode)
            .current_dir(repo_root())
            .env("PATH", path_with_prefix(temp.path()))
            .output()
            .unwrap_or_else(|err| panic!("run generate-changelog {mode}: {err}"));

        assert!(
            !output.status.success(),
            "generate-changelog {mode} should fail when git-cliff fails\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn release_changelog_curated_content_requires_bullet_entries() {
    let temp = tempfile::tempdir().expect("tempdir");
    let heading_only = temp.path().join("heading-only.md");
    let with_entry = temp.path().join("with-entry.md");
    std::fs::write(
        &heading_only,
        "# Changelog\n\n## [Unreleased]\n\n### Added\n\n## [0.1.0] - 2026-01-01\n",
    )
    .expect("write heading-only changelog");
    std::fs::write(
        &with_entry,
        "# Changelog\n\n## [Unreleased]\n\n### Added\n\n- Added a release note.\n\n## [0.1.0] - 2026-01-01\n",
    )
    .expect("write populated changelog");

    let script = "source scripts/lib/release_verify_pipeline.sh; release_changelog_has_curated_unreleased_content \"$1\"";
    let heading_status = Command::new("bash")
        .arg("-c")
        .arg(script)
        .arg("bash")
        .arg(&heading_only)
        .current_dir(repo_root())
        .status()
        .expect("run heading-only probe");
    let entry_status = Command::new("bash")
        .arg("-c")
        .arg(script)
        .arg("bash")
        .arg(&with_entry)
        .current_dir(repo_root())
        .status()
        .expect("run populated probe");

    assert!(
        !heading_status.success(),
        "heading-only Unreleased section should not count as curated content"
    );
    assert!(
        entry_status.success(),
        "Unreleased section with a bullet should count as curated content"
    );
}

#[test]
fn release_script_derives_repo_url_from_origin_remote() {
    let script = read_repo_file("scripts/lib/release_verify_pipeline.sh");
    assert!(
        script.contains("cueloop_get_repo_http_url"),
        "release pipeline should derive the repo URL from git remote origin"
    );
    assert!(
        !script.contains("https://github.com/fitchmultz/cueloop/compare"),
        "release pipeline should not hardcode compare links to a specific owner"
    );
}

#[test]
fn release_notes_template_uses_repo_placeholders() {
    let template = read_repo_file(".github/release-notes-template.md");
    assert!(
        template.contains("{{REPO_URL}}"),
        "release-notes template should use repo URL placeholders"
    );
    assert!(
        template.contains("{{REPO_CLONE_URL}}"),
        "release-notes template should use clone URL placeholders"
    );
}

#[test]
fn release_policy_treats_cargo_lock_as_release_metadata() {
    let script = read_repo_file("scripts/lib/release_policy.sh");
    assert!(
        script.contains("\"Cargo.lock\""),
        "release policy should treat Cargo.lock as release metadata"
    );
}

#[test]
fn release_metadata_includes_every_generated_schema() {
    let make_surface = format!(
        "{}\n{}",
        read_repo_file("Makefile"),
        read_repo_file("mk/rust.mk")
    );
    let generated_schema_paths = parse_makefile_generated_schema_paths(&make_surface);
    assert!(
        !generated_schema_paths.is_empty(),
        "Makefile generate should produce committed schemas"
    );

    let release_metadata_paths =
        parse_release_metadata_paths(&read_repo_file("scripts/lib/release_policy.sh"));
    let missing_paths: Vec<_> = generated_schema_paths
        .difference(&release_metadata_paths)
        .cloned()
        .collect();
    assert!(
        missing_paths.is_empty(),
        "release metadata should include every schema generated by Makefile generate; missing: {missing_paths:?}"
    );
}

#[test]
fn release_artifact_builder_cleans_release_artifacts_directory() {
    let script = read_repo_file("scripts/build-release-artifacts.sh");
    let cleanup_count = script.matches("rm -rf \"$RELEASE_ARTIFACTS_DIR\"").count();
    assert!(
        cleanup_count >= 1,
        "artifact builder should clean target/release-artifacts before packaging"
    );
}

#[test]
fn release_artifact_builder_packages_primary_binary() {
    let script = read_repo_file("scripts/build-release-artifacts.sh");
    assert!(
        script.contains("cueloop-${version}-${platform_name}.tar.gz"),
        "artifact tarballs should use the primary cueloop name"
    );
    assert!(
        script.contains("tar -czf \"$RELEASE_ARTIFACTS_DIR/$tarball_name\" -C \"$(dirname \"$binary_path\")\" cueloop"),
        "artifact tarballs should include the primary cueloop binary"
    );

    let publish_pipeline = read_repo_file("scripts/lib/release_publish_pipeline.sh");
    assert!(
        publish_pipeline.contains("/cueloop-\"${VERSION}\"-*.tar.gz"),
        "release upload should target cueloop-named artifacts"
    );
}

#[test]
fn release_policy_uses_target_transaction_state() {
    let script = read_repo_file("scripts/lib/release_state.sh");
    assert!(
        script.contains("target/release-transactions"),
        "release state should keep release transaction state under target/release-transactions"
    );
}

#[test]
fn release_script_publishes_only_after_local_release_is_prepared() {
    let script = read_repo_file("scripts/lib/release_publish_pipeline.sh");
    assert!(
        read_repo_file("scripts/lib/release_verify_pipeline.sh")
            .contains("release_prepare_verified_snapshot")
            && script.contains("release_create_commit_and_tag")
            && script.contains("release_create_or_update_github_release_draft")
            && script.contains("release_publish_crate"),
        "release pipeline should verify locally before publishing externally"
    );
    assert!(
        script.find("release_push_remote_main") < script.find("release_publish_crate"),
        "release pipeline should push origin/main before crates.io publish"
    );
    assert!(
        script.find("release_push_remote_tag") < script.find("release_publish_crate"),
        "release pipeline should push the release tag before crates.io publish"
    );
    assert!(
        script.find("release_create_or_update_github_release_draft")
            < script.find("release_publish_crate"),
        "release pipeline should draft the GitHub release before crates.io publish"
    );
    assert!(
        script.find("release_publish_crate") < script.find("release_publish_github_release"),
        "release pipeline should only publish the GitHub release after crates.io succeeds"
    );
    assert!(
        script.contains("--draft") && script.contains("--draft=false"),
        "release pipeline should split GitHub release draft creation from final publication"
    );
}

#[test]
fn release_script_reconciles_without_legacy_skip_flags() {
    let script = read_repo_file("scripts/release.sh");
    assert!(
        script.contains("scripts/release.sh reconcile")
            || script.contains("reconcile 0.2.0")
            || script.contains("reconcile <version>"),
        "release.sh should document durable transaction-state reconciliation"
    );
    assert!(
        !script.contains("CUELOOP_RELEASE_SKIP_PUBLISH"),
        "release.sh should not rely on the old manual skip-publish recovery flag"
    );
    assert!(
        !script.contains("CUELOOP_RELEASE_ALLOW_EXISTING_TAG"),
        "release.sh should not retain the old existing-tag override flag"
    );
}

#[test]
fn release_verify_allows_release_metadata_drift_after_version_sync() {
    let script = read_repo_file("scripts/release.sh");
    assert!(
        script.contains("release_validate_repo_state 0 1"),
        "execute mode should allow release-only metadata drift after verify prepares the publish snapshot"
    );
}

#[test]
fn release_execute_initializes_transaction_after_validation() {
    let script = read_repo_file("scripts/release.sh");
    assert!(
        script.find("release_validate_repo_state 0")
            < script.find("release_state_init \"execute\""),
        "execute mode should not create transaction state before repo-state validation succeeds"
    );
}

#[test]
fn release_verify_records_a_reusable_snapshot() {
    let script = read_repo_file("scripts/release.sh");
    assert!(
        script.contains("release_verify_state_init")
            && script.contains("release_prepare_verified_snapshot")
            && script.contains("release_verify_assert_ready_for_execute"),
        "release.sh should prepare a durable verify snapshot and require it before execute"
    );
}

#[test]
fn release_verify_state_uses_target_release_verifications_directory() {
    let script = read_repo_file("scripts/lib/release_verify_state.sh");
    assert!(
        script.contains("target/release-verifications"),
        "verified release state should live under target/release-verifications"
    );
}
