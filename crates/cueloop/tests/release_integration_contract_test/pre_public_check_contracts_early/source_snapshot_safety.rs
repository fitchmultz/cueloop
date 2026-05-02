//! Early `pre-public-check.sh` source-snapshot safety contract hub.
//!
//! Purpose:
//! - Split `--allow-no-git` source-snapshot coverage into smaller behavior modules.
//!
//! Responsibilities:
//! - Keep runtime-artifact and `.ralph`-path regressions focused without changing assertions.
//!
//! Scope:
//! - Limited to early `pre-public-check.sh` source-snapshot coverage.
//!
//! Usage:
//! - Loaded by `pre_public_check_contracts_early.rs`.
//!
//! Invariants/Assumptions:
//! - Test grouping must preserve the existing `--allow-no-git` contract assertions verbatim.

#[path = "source_snapshot_safety/runtime_artifacts.rs"]
mod runtime_artifacts;

#[path = "source_snapshot_safety/cueloop_paths.rs"]
mod cueloop_paths;
