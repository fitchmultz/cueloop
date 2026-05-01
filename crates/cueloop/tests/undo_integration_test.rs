//! Purpose: thin integration-test hub for `ralph undo` CLI coverage.
//!
//! Responsibilities:
//! - Re-export shared imports and suite-local helpers for undo integration tests.
//! - Delegate list, restore, dry-run, specific-id, error, and mutation-source coverage to focused companion modules.
//!
//! Scope:
//! - Test-suite wiring only; this root module contains no test functions.
//!
//! Usage:
//! - Companion modules use `use super::*;` to access shared imports, test helpers, and suite-local support helpers.
//!
//! Invariants/assumptions callers must respect:
//! - Test names, assertions, and CLI coverage remain unchanged from the pre-split monolith.
//! - Suite-local helpers preserve the original end-to-end CLI setup flow through shared `test_support` helpers.

mod test_support;

#[path = "undo_integration_test/support.rs"]
mod support;

pub(crate) use anyhow::Result;
pub(crate) use cueloop::contracts::TaskStatus;
pub(crate) use std::fs;
pub(crate) use support::{setup_undo_repo, snapshot_ids_from_list_output};
pub(crate) use tempfile::TempDir;
pub(crate) use test_support::{
    git_init, make_test_task, read_done, read_queue, run_in_dir, seed_ralph_dir,
    temp_dir_outside_repo, write_done, write_queue,
};

#[path = "undo_integration_test/dry_run.rs"]
mod dry_run;
#[path = "undo_integration_test/errors.rs"]
mod errors;
#[path = "undo_integration_test/list.rs"]
mod list;
#[path = "undo_integration_test/mutation_sources.rs"]
mod mutation_sources;
#[path = "undo_integration_test/restore.rs"]
mod restore;
#[path = "undo_integration_test/specific_id.rs"]
mod specific_id;
