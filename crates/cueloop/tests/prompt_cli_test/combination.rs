//! Purpose: multi-flag combination parsing coverage for `cueloop prompt`.
//!
//! Responsibilities:
//! - Verify representative full command combinations parse into the expected clap structures.
//! - Preserve mixed-flag worker, scan, and task-builder regression coverage.
//!
//! Scope:
//! - Combination parsing scenarios spanning worker, scan, and task-builder subcommands.
//!
//! Usage:
//! - Run via the root `prompt_cli_test` integration suite.
//!
//! Invariants/assumptions callers must respect:
//! - Assertions remain unchanged from the original suite.
//! - This module preserves parsing-only coverage with no fixture state.

use super::*;

#[test]
fn prompt_worker_parses_full_combination() {
    let cli = Cli::try_parse_from([
        "ralph",
        "prompt",
        "worker",
        "--phase",
        "2",
        "--task-id",
        "RQ-0001",
        "--plan-file",
        "/path/to/plan.md",
        "--iterations",
        "3",
        "--iteration-index",
        "2",
        "--repo-prompt",
        "plan",
        "--explain",
    ])
    .expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Worker(w) => {
                assert_eq!(w.phase, Some(RunPhase::Phase2));
                assert_eq!(w.task_id.as_deref(), Some("RQ-0001"));
                assert_eq!(
                    w.plan_file,
                    Some(std::path::PathBuf::from("/path/to/plan.md"))
                );
                assert_eq!(w.iterations, 3);
                assert_eq!(w.iteration_index, 2);
                assert_eq!(w.repo_prompt, Some(RepoPromptMode::Plan));
                assert!(w.explain);
            }
            _ => panic!("expected worker command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_worker_parses_single_with_other_flags() {
    let cli = Cli::try_parse_from([
        "ralph",
        "prompt",
        "worker",
        "--single",
        "--task-id",
        "RQ-0001",
        "--repo-prompt",
        "tools",
    ])
    .expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Worker(w) => {
                assert!(w.single);
                assert!(w.phase.is_none());
                assert_eq!(w.task_id.as_deref(), Some("RQ-0001"));
                assert_eq!(w.repo_prompt, Some(RepoPromptMode::Tools));
            }
            _ => panic!("expected worker command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_scan_parses_full_combination() {
    let cli = Cli::try_parse_from([
        "ralph",
        "prompt",
        "scan",
        "--focus",
        "security audit",
        "--mode",
        "maintenance",
        "--repo-prompt",
        "off",
        "--explain",
    ])
    .expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Scan(s) => {
                assert_eq!(s.focus, "security audit");
                assert_eq!(s.mode, ScanMode::Maintenance);
                assert_eq!(s.repo_prompt, Some(RepoPromptMode::Off));
                assert!(s.explain);
            }
            _ => panic!("expected scan command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_task_builder_parses_full_combination() {
    let cli = Cli::try_parse_from([
        "ralph",
        "prompt",
        "task-builder",
        "--request",
        "Add comprehensive tests",
        "--tags",
        "rust,testing,cli",
        "--scope",
        "crates/cueloop/src/cli",
        "--repo-prompt",
        "tools",
        "--explain",
    ])
    .expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::TaskBuilder(t) => {
                assert_eq!(t.request.as_deref(), Some("Add comprehensive tests"));
                assert_eq!(t.tags, "rust,testing,cli");
                assert_eq!(t.scope, "crates/cueloop/src/cli");
                assert_eq!(t.repo_prompt, Some(RepoPromptMode::Tools));
                assert!(t.explain);
            }
            _ => panic!("expected task-builder command"),
        },
        _ => panic!("expected prompt command"),
    }
}
