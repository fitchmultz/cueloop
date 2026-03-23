//! Makefile routing and macOS gate contract tests.
//!
//! Responsibilities:
//! - Verify agent-ci routing and surface-classifier delegation.
//! - Verify macOS targets depend on preflight and isolate DerivedData state.
//! - Verify Makefile bootstraps the pinned rustup toolchain.
//!
//! Not handled here:
//! - Canonical CI step sequence equality.
//! - Inline parser fixtures or clean-target smoke tests.
//!
//! Invariants/assumptions:
//! - Routing contracts are asserted against the repo Makefile block text.
//! - macOS target assertions treat lock orchestration as part of the public contract.

use anyhow::{Context, Result};

use super::makefile_ci_contract_test_support::{
    extract_target_block, extract_target_dependencies, read_repo_makefile,
};

#[test]
fn test_macos_targets_gate_with_preflight_and_isolate_derived_data() -> Result<()> {
    let makefile = read_repo_makefile()?;

    assert!(
        makefile.contains("macos-preflight:"),
        "Makefile should define macos-preflight target"
    );

    let macos_build_deps = extract_target_dependencies(&makefile, "macos-build")
        .context("extract macos-build deps")?;
    assert!(
        macos_build_deps.contains(&"macos-preflight".to_string()),
        "macos-build should depend on macos-preflight"
    );

    let macos_test_deps =
        extract_target_dependencies(&makefile, "macos-test").context("extract macos-test deps")?;
    assert!(
        macos_test_deps.contains(&"macos-preflight".to_string()),
        "macos-test should depend on macos-preflight"
    );

    assert!(
        makefile.contains("macos-ci: macos-preflight"),
        "macos-ci should depend on macos-preflight"
    );
    assert!(
        makefile.contains(
            "macos-test-contracts: macos-test-settings-smoke macos-test-workspace-routing-contract"
        ),
        "Makefile should define a deterministic macOS contract aggregate target"
    );
    assert!(
        makefile.contains("derived_data_path=\"$(XCODE_DERIVED_DATA_ROOT)/build\""),
        "macos-build should use an isolated build DerivedData path"
    );
    assert!(
        makefile.contains("derived_data_path=\"$(XCODE_DERIVED_DATA_ROOT)/test\""),
        "macos-test should use an isolated test DerivedData path"
    );
    assert!(
        makefile.contains("rm -rf \"$$derived_data_path\""),
        "macOS targets should clear DerivedData before running xcodebuild"
    );
    assert!(
        makefile.contains("XCODE_BUILD_LOCK_DIR ?= target/tmp/locks/xcodebuild.lock"),
        "Makefile should define a dedicated Xcode build lock path"
    );
    assert!(
        makefile.contains("Waiting for Xcode build lock"),
        "macOS Xcode targets should serialize concurrent xcodebuild invocations"
    );

    for target in [
        "macos-build",
        "macos-test",
        "macos-ui-build-for-testing",
        "macos-ui-retest",
        "macos-test-window-shortcuts",
    ] {
        let block = extract_target_block(&makefile, target)
            .with_context(|| format!("extract {target} block"))?;
        assert!(
            block.contains("lock_dir=\"$(XCODE_BUILD_LOCK_DIR)\""),
            "{target} should acquire the shared Xcode build lock"
        );
        assert!(
            block.contains("while ! mkdir \"$$lock_dir\""),
            "{target} should wait for exclusive Xcode build access"
        );
        assert!(
            block.contains("wait_notified=0"),
            "{target} should initialize one-time lock wait logging state"
        );
        assert!(
            block.contains("if [ \"$$wait_notified\" = \"0\" ]; then"),
            "{target} should avoid repeating the lock wait message every poll cycle"
        );
    }

    for target in ["macos-ui-retest", "macos-test-window-shortcuts"] {
        let block = extract_target_block(&makefile, target)
            .with_context(|| format!("extract {target} cleanup block"))?;
        assert!(
            block.contains("app_binary="),
            "{target} should track the launched RalphMac test app binary for cleanup"
        );
        assert!(
            block.contains("runner_binary="),
            "{target} should track the launched UI test runner binary for cleanup"
        );
        assert!(
            block.contains("pkill -TERM -f \"$$runner_binary\""),
            "{target} should terminate any lingering UI test runner before exiting"
        );
        assert!(
            block.contains("pkill -TERM -f \"$$app_binary\""),
            "{target} should terminate any lingering RalphMac UI test app before exiting"
        );
        assert!(
            block.contains("left a lingering UI test app or runner process"),
            "{target} should fail loudly if UI test processes survive the run"
        );
    }

    let shortcuts_block = extract_target_block(&makefile, "macos-test-window-shortcuts")
        .context("extract macos-test-window-shortcuts selectors")?;
    assert!(
        shortcuts_block.contains("RalphMacUITests/RalphMacUIWindowRoutingTests/test_windowShortcuts_affectOnlyFocusedWindow"),
        "macos-test-window-shortcuts should target the focused-window routing suite"
    );
    assert!(
        shortcuts_block.contains("RalphMacUITests/RalphMacUIWindowRoutingTests/test_commandPaletteNewTab_affectsOnlyFocusedWindow"),
        "macos-test-window-shortcuts should target the focused-window command-palette routing suite"
    );

    Ok(())
}

#[test]
fn test_macos_ui_artifact_target_preserves_result_bundle_and_summary() -> Result<()> {
    let makefile = read_repo_makefile()?;
    let block = extract_target_block(&makefile, "macos-test-ui-artifacts")
        .context("extract macos-test-ui-artifacts block")?;

    assert!(
        block.contains("result_bundle_path=\"$$artifact_dir/RalphMacUITests.xcresult\""),
        "macos-test-ui-artifacts should preserve the xcresult bundle"
    );
    assert!(
        block.contains("targeted_test: $${RALPH_UI_ONLY_TESTING:-all}"),
        "macos-test-ui-artifacts summary should record whether the run was targeted"
    );
    assert!(
        !block.contains("xcresulttool export attachments"),
        "macos-test-ui-artifacts should not depend on empty xcresult attachment export"
    );
    assert!(
        !block.contains("screenshots_dir="),
        "macos-test-ui-artifacts should not carry dead screenshot-export plumbing"
    );

    Ok(())
}

#[test]
fn test_agent_ci_routes_between_docs_ci_fast_and_macos_ci() -> Result<()> {
    let makefile = read_repo_makefile()?;
    let agent_ci_block =
        extract_target_block(&makefile, "agent-ci").context("extract agent-ci block")?;

    assert!(
        agent_ci_block.contains("docs, Rust, and macOS ship gates"),
        "agent-ci should advertise the three-way routing contract"
    );
    assert!(
        agent_ci_block.contains("scripts/agent-ci-surface.sh --target"),
        "agent-ci must route through the shared dependency-surface classifier"
    );
    assert!(
        agent_ci_block.contains("$(MAKE) --no-print-directory \"$$target_name\""),
        "agent-ci must dispatch to the classifier-selected gate target"
    );
    assert!(
        agent_ci_block.contains("target_reason"),
        "agent-ci should surface the classifier's routing reason"
    );

    Ok(())
}

#[test]
fn test_makefile_auto_prefers_pinned_rustup_toolchain() -> Result<()> {
    let makefile = read_repo_makefile()?;

    assert!(
        makefile.contains("rustup which rustc --toolchain"),
        "Makefile should resolve the pinned rustup toolchain from rust-toolchain.toml"
    );
    assert!(
        makefile.contains("export PATH=\"$(RALPH_PINNED_RUST_BIN_DIR):$$PATH\"; export RUSTC=\"$(RALPH_PINNED_RUSTC)\""),
        "Makefile should inject the pinned Rust toolchain into PATH and RUSTC"
    );

    Ok(())
}
