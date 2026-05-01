//! `pre-public-check.sh` contract coverage (early section).
//!
//! Purpose:
//! - Group early `pre-public-check.sh` contract coverage into focused behavior modules.
//!
//! Responsibilities:
//! - Keep source-snapshot, git-prerequisite, and `--allow-no-git` regressions easy to scan.
//!
//! Scope:
//! - Limited to this file's owning feature boundary.
//!
//! Usage:
//! - Loaded by the release integration contract test harness.
//!
//! Invariants/Assumptions:
//! - Test grouping must preserve the existing `pre-public-check.sh` release contract assertions.

#[path = "pre_public_check_contracts_early/discovery_and_git.rs"]
mod discovery_and_git;

#[path = "pre_public_check_contracts_early/source_snapshot_safety.rs"]
mod source_snapshot_safety;
