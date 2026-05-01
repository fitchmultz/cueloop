//! `pre-public-check.sh` contract coverage (tracked-path and git snapshot section).
//!
//! Purpose:
//! - Group the tracked-path and git-snapshot contracts into smaller behavior modules.
//!
//! Responsibilities:
//! - Keep local-only, release-context, and runtime-artifact regressions focused.
//!
//! Scope:
//! - Limited to this file's owning feature boundary.
//!
//! Usage:
//! - Loaded by the release integration contract test harness.
//!
//! Invariants/Assumptions:
//! - Test grouping must preserve the existing `pre-public-check.sh` tracked-path contract assertions.

#[path = "pre_public_check_contracts_mid/local_only.rs"]
mod local_only;

#[path = "pre_public_check_contracts_mid/runtime_artifacts.rs"]
mod runtime_artifacts;
