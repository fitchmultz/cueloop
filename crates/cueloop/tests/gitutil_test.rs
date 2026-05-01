//! Purpose: thin integration-test hub for `cueloop::git` contract coverage.
//!
//! Responsibilities:
//! - Re-export shared imports and suite-local helpers for git utility integration tests.
//! - Delegate status, clean-repo, commit/revert, upstream, and error coverage to focused modules.
//!
//! Scope:
//! - Integration tests for the public `cueloop::git` API only.
//!
//! Usage:
//! - Companion modules use `use super::*;` to access shared imports and helpers.
//!
//! Invariants/Assumptions:
//! - This root module contains no test functions.
//! - Test names and assertions remain unchanged from the pre-split suite.

mod test_support;

#[path = "gitutil_test/support.rs"]
mod support;

pub(crate) use cueloop::git;
pub(crate) use std::fs;
pub(crate) use std::process::Command;
pub(crate) use support::{commit_file, init_git_repo};
pub(crate) use tempfile::TempDir;

#[path = "gitutil_test/clean_repo.rs"]
mod clean_repo;
#[path = "gitutil_test/commit_revert.rs"]
mod commit_revert;
#[path = "gitutil_test/error_display.rs"]
mod error_display;
#[path = "gitutil_test/status.rs"]
mod status;
#[path = "gitutil_test/upstream.rs"]
mod upstream;
