//! `pre-public-check.sh` contract coverage (exact paths and local-only artifacts).
//!
//! Purpose:
//! - Group exact-path rejection contracts into smaller behavior modules.
//!
//! Responsibilities:
//! - Keep `.cueloop`, runtime-artifact, and `.DS_Store` regressions separate and readable.
//!
//! Scope:
//! - Limited to this file's owning feature boundary.
//!
//! Usage:
//! - Loaded by the release integration contract test harness.
//!
//! Invariants/Assumptions:
//! - Test grouping must preserve the existing exact-path rejection assertions.

#[path = "pre_public_check_contracts_tracked_paths/cueloop_exact_paths.rs"]
mod cueloop_exact_paths;

#[path = "pre_public_check_contracts_tracked_paths/runtime_exact_paths.rs"]
mod runtime_exact_paths;

#[path = "pre_public_check_contracts_tracked_paths/ds_store.rs"]
mod ds_store;
