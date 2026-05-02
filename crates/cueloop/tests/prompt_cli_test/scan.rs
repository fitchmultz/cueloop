//! Purpose: scan-subcommand parsing coverage for `cueloop prompt`.
//!
//! Responsibilities:
//! - Verify scan flags parse into the expected clap structures.
//! - Preserve scan-mode, repo-prompt, and explain flag assertions.
//!
//! Scope:
//! - `cueloop prompt scan` parsing only.
//!
//! Usage:
//! - Run via the root `prompt_cli_test` integration suite.
//!
//! Invariants/assumptions callers must respect:
//! - Assertions remain unchanged from the original suite.
//! - This module preserves parsing-only coverage with no fixture state.

use super::*;

#[test]
fn prompt_scan_parses_focus() {
    let cli =
        Cli::try_parse_from(["ralph", "prompt", "scan", "--focus", "CI gaps"]).expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Scan(s) => {
                assert_eq!(s.focus, "CI gaps");
            }
            _ => panic!("expected scan command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_scan_parses_mode_maintenance() {
    let cli =
        Cli::try_parse_from(["ralph", "prompt", "scan", "--mode", "maintenance"]).expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Scan(s) => {
                assert_eq!(s.mode, ScanMode::Maintenance);
            }
            _ => panic!("expected scan command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_scan_parses_mode_innovation() {
    let cli =
        Cli::try_parse_from(["ralph", "prompt", "scan", "--mode", "innovation"]).expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Scan(s) => {
                assert_eq!(s.mode, ScanMode::Innovation);
            }
            _ => panic!("expected scan command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_scan_parses_repo_prompt_tools() {
    let cli =
        Cli::try_parse_from(["ralph", "prompt", "scan", "--repo-prompt", "tools"]).expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Scan(s) => {
                assert_eq!(s.repo_prompt, Some(RepoPromptMode::Tools));
            }
            _ => panic!("expected scan command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_scan_parses_explain() {
    let cli = Cli::try_parse_from(["ralph", "prompt", "scan", "--explain"]).expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Scan(s) => {
                assert!(s.explain);
            }
            _ => panic!("expected scan command"),
        },
        _ => panic!("expected prompt command"),
    }
}
