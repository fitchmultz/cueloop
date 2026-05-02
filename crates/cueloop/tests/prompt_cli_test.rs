//! Purpose: thin integration-test hub for `cueloop prompt` CLI parsing coverage.
//!
//! Responsibilities:
//! - Re-export shared parsing types and clap imports for prompt CLI integration tests.
//! - Delegate worker, scan, task-builder, management, and combination parsing coverage to focused companion modules.
//!
//! Scope:
//! - Test-suite wiring only; this root module contains no test functions.
//!
//! Usage:
//! - Companion modules use `use super::*;` to access shared clap/parser imports and prompt CLI types.
//!
//! Invariants/assumptions callers must respect:
//! - Test names, assertions, and parsing behavior remain unchanged from the pre-split monolith.
//! - This suite covers CLI parsing only; prompt rendering/content behavior lives elsewhere.

use clap::Parser;

use cueloop::agent::RepoPromptMode;
use cueloop::cli::scan::ScanMode;
use cueloop::cli::{Cli, Command, prompt::PromptCommand};
use cueloop::promptflow::RunPhase;

#[path = "prompt_cli_test/combination.rs"]
mod combination;
#[path = "prompt_cli_test/management.rs"]
mod management;
#[path = "prompt_cli_test/scan.rs"]
mod scan;
#[path = "prompt_cli_test/task_builder.rs"]
mod task_builder;
#[path = "prompt_cli_test/worker.rs"]
mod worker;
