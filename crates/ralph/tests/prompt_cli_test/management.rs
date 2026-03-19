//! Purpose: management-subcommand parsing coverage for `ralph prompt`.
//!
//! Responsibilities:
//! - Verify list, show, export, sync, and diff subcommands parse into the expected clap structures.
//! - Preserve flag-combination assertions for management commands.
//!
//! Scope:
//! - `ralph prompt` management subcommands other than worker/scan/task-builder.
//!
//! Usage:
//! - Run via the root `prompt_cli_test` integration suite.
//!
//! Invariants/assumptions callers must respect:
//! - Assertions remain unchanged from the original suite.
//! - This module preserves parsing-only coverage with no fixture state.

use super::*;

#[test]
fn prompt_list_subcommand() {
    let cli = Cli::try_parse_from(["ralph", "prompt", "list"]).expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::List => {}
            _ => panic!("expected list command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_show_parses_name() {
    let cli = Cli::try_parse_from(["ralph", "prompt", "show", "worker"]).expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Show(s) => {
                assert_eq!(s.name, "worker");
                assert!(!s.raw);
            }
            _ => panic!("expected show command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_show_parses_raw() {
    let cli = Cli::try_parse_from(["ralph", "prompt", "show", "worker", "--raw"]).expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Show(s) => {
                assert_eq!(s.name, "worker");
                assert!(s.raw);
            }
            _ => panic!("expected show command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_export_parses_all() {
    let cli = Cli::try_parse_from(["ralph", "prompt", "export", "--all"]).expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Export(e) => {
                assert!(e.all);
                assert!(e.name.is_none());
            }
            _ => panic!("expected export command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_export_parses_name() {
    let cli = Cli::try_parse_from(["ralph", "prompt", "export", "worker"]).expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Export(e) => {
                assert_eq!(e.name.as_deref(), Some("worker"));
                assert!(!e.all);
            }
            _ => panic!("expected export command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_export_parses_force() {
    let cli =
        Cli::try_parse_from(["ralph", "prompt", "export", "worker", "--force"]).expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Export(e) => {
                assert_eq!(e.name.as_deref(), Some("worker"));
                assert!(e.force);
            }
            _ => panic!("expected export command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_sync_parses_dry_run() {
    let cli = Cli::try_parse_from(["ralph", "prompt", "sync", "--dry-run"]).expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Sync(s) => {
                assert!(s.dry_run);
                assert!(!s.force);
            }
            _ => panic!("expected sync command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_sync_parses_force() {
    let cli = Cli::try_parse_from(["ralph", "prompt", "sync", "--force"]).expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Sync(s) => {
                assert!(!s.dry_run);
                assert!(s.force);
            }
            _ => panic!("expected sync command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_sync_parses_dry_run_and_force() {
    let cli =
        Cli::try_parse_from(["ralph", "prompt", "sync", "--dry-run", "--force"]).expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Sync(s) => {
                assert!(s.dry_run);
                assert!(s.force);
            }
            _ => panic!("expected sync command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_diff_parses_name() {
    let cli = Cli::try_parse_from(["ralph", "prompt", "diff", "worker"]).expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Diff(d) => {
                assert_eq!(d.name, "worker");
            }
            _ => panic!("expected diff command"),
        },
        _ => panic!("expected prompt command"),
    }
}
