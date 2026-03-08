//! Worker lifecycle tests for parallel execution helpers.
//!
//! Responsibilities:
//! - Verify worker command construction, task selection, and exclusion rules.
//! - Keep the heavy scenario coverage out of the production worker facade.
//!
//! Does not handle:
//! - Production worker orchestration logic.

use super::*;
use crate::agent::AgentOverrides;
use crate::commands::run::parallel::state;
use crate::config;
use crate::git::WorkspaceSpec;
use crate::queue;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use tempfile::TempDir;

#[path = "worker_tests/command.rs"]
mod command;
#[path = "worker_tests/exclusion_and_locking.rs"]
mod exclusion_and_locking;
#[path = "worker_tests/ordering_and_blocking.rs"]
mod ordering_and_blocking;
#[path = "worker_tests/repair_and_validation.rs"]
mod repair_and_validation;
