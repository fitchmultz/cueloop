//! Purpose: task-builder-subcommand parsing coverage for `ralph prompt`.
//!
//! Responsibilities:
//! - Verify task-builder flags parse into the expected clap structures.
//! - Preserve request, tags, scope, repo-prompt, and explain assertions.
//!
//! Scope:
//! - `ralph prompt task-builder` parsing only.
//!
//! Usage:
//! - Run via the root `prompt_cli_test` integration suite.
//!
//! Invariants/assumptions callers must respect:
//! - Assertions remain unchanged from the original suite.
//! - This module preserves parsing-only coverage with no fixture state.

use super::*;

#[test]
fn prompt_task_builder_parses_request() {
    let cli = Cli::try_parse_from(["ralph", "prompt", "task-builder", "--request", "Add tests"])
        .expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::TaskBuilder(t) => {
                assert_eq!(t.request.as_deref(), Some("Add tests"));
            }
            _ => panic!("expected task-builder command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_task_builder_parses_tags() {
    let cli = Cli::try_parse_from(["ralph", "prompt", "task-builder", "--tags", "rust,tests"])
        .expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::TaskBuilder(t) => {
                assert_eq!(t.tags, "rust,tests");
            }
            _ => panic!("expected task-builder command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_task_builder_parses_scope() {
    let cli = Cli::try_parse_from([
        "ralph",
        "prompt",
        "task-builder",
        "--scope",
        "crates/cueloop",
    ])
    .expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::TaskBuilder(t) => {
                assert_eq!(t.scope, "crates/cueloop");
            }
            _ => panic!("expected task-builder command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_task_builder_parses_repo_prompt_plan() {
    let cli = Cli::try_parse_from(["ralph", "prompt", "task-builder", "--repo-prompt", "plan"])
        .expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::TaskBuilder(t) => {
                assert_eq!(t.repo_prompt, Some(RepoPromptMode::Plan));
            }
            _ => panic!("expected task-builder command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_task_builder_parses_explain() {
    let cli = Cli::try_parse_from(["ralph", "prompt", "task-builder", "--explain"]).expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::TaskBuilder(t) => {
                assert!(t.explain);
            }
            _ => panic!("expected task-builder command"),
        },
        _ => panic!("expected prompt command"),
    }
}
