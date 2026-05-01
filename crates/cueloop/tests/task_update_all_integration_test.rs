//! Purpose: thin integration-test hub for `ralph task update` update-all coverage.
//!
//! Responsibilities:
//! - Re-export shared imports and suite-local helpers for update-all integration tests.
//! - Delegate bulk-update and race-condition scenarios to focused companion modules.
//!
//! Scope:
//! - Test-suite wiring only; this root module contains no test functions.
//!
//! Usage:
//! - Companion modules use `use super::*;` to access shared result types, helpers, and fixtures.
//!
//! Invariants/assumptions callers must respect:
//! - Test names, assertions, and CLI behavior coverage remain unchanged from the pre-split monolith.
//! - Suite-local support helpers preserve the original end-to-end CLI setup and fixture contents.

mod test_support;

#[path = "task_update_all_integration_test/support.rs"]
mod support;

pub(crate) use anyhow::{Context, Result};
pub(crate) use support::{
    configure_runner, create_fake_runner, run_in_dir, write_empty_queue, write_queue_with_one_task,
    write_queue_with_two_tasks,
};
pub(crate) use test_support::temp_dir_outside_repo;

#[path = "task_update_all_integration_test/bulk_update.rs"]
mod bulk_update;
#[path = "task_update_all_integration_test/race_conditions.rs"]
mod race_conditions;
