//! Purpose: worker-subcommand parsing coverage for `cueloop prompt`.
//!
//! Responsibilities:
//! - Verify worker flags parse into the expected clap structures.
//! - Verify invalid phase values and conflicting worker flags are rejected.
//!
//! Scope:
//! - `cueloop prompt worker` parsing only.
//!
//! Usage:
//! - Run via the root `prompt_cli_test` integration suite.
//!
//! Invariants/assumptions callers must respect:
//! - Assertions and error-text expectations remain unchanged from the original suite.
//! - This module preserves parsing-only coverage with no fixture state.

use super::*;

#[test]
fn prompt_worker_parses_phase_1() {
    let cli = Cli::try_parse_from(["cueloop", "prompt", "worker", "--phase", "1"]).expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Worker(w) => {
                assert_eq!(w.phase, Some(RunPhase::Phase1));
                assert!(!w.single);
            }
            _ => panic!("expected worker command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_worker_parses_phase_2() {
    let cli = Cli::try_parse_from(["cueloop", "prompt", "worker", "--phase", "2"]).expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Worker(w) => {
                assert_eq!(w.phase, Some(RunPhase::Phase2));
                assert!(!w.single);
            }
            _ => panic!("expected worker command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_worker_parses_phase_3() {
    let cli = Cli::try_parse_from(["cueloop", "prompt", "worker", "--phase", "3"]).expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Worker(w) => {
                assert_eq!(w.phase, Some(RunPhase::Phase3));
                assert!(!w.single);
            }
            _ => panic!("expected worker command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_worker_parses_single() {
    let cli = Cli::try_parse_from(["cueloop", "prompt", "worker", "--single"]).expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Worker(w) => {
                assert!(w.single);
                assert!(w.phase.is_none());
            }
            _ => panic!("expected worker command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_worker_single_conflicts_with_phase() {
    let err = Cli::try_parse_from(["cueloop", "prompt", "worker", "--single", "--phase", "1"])
        .err()
        .expect("parse failure");
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("conflict") || msg.contains("cannot be used"),
        "unexpected error: {msg}"
    );
}

#[test]
fn prompt_worker_parses_task_id() {
    let cli = Cli::try_parse_from(["cueloop", "prompt", "worker", "--task-id", "RQ-0001"])
        .expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Worker(w) => {
                assert_eq!(w.task_id.as_deref(), Some("RQ-0001"));
            }
            _ => panic!("expected worker command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_worker_parses_plan_file() {
    let cli = Cli::try_parse_from([
        "cueloop",
        "prompt",
        "worker",
        "--plan-file",
        "/path/to/plan.md",
    ])
    .expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Worker(w) => {
                assert_eq!(
                    w.plan_file,
                    Some(std::path::PathBuf::from("/path/to/plan.md"))
                );
            }
            _ => panic!("expected worker command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_worker_parses_plan_text() {
    let cli = Cli::try_parse_from(["cueloop", "prompt", "worker", "--plan-text", "my plan"])
        .expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Worker(w) => {
                assert_eq!(w.plan_text.as_deref(), Some("my plan"));
            }
            _ => panic!("expected worker command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_worker_parses_iterations() {
    let cli =
        Cli::try_parse_from(["cueloop", "prompt", "worker", "--iterations", "3"]).expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Worker(w) => {
                assert_eq!(w.iterations, 3);
            }
            _ => panic!("expected worker command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_worker_parses_iteration_index() {
    let cli = Cli::try_parse_from(["cueloop", "prompt", "worker", "--iteration-index", "2"])
        .expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Worker(w) => {
                assert_eq!(w.iteration_index, 2);
            }
            _ => panic!("expected worker command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_worker_parses_repo_prompt_tools() {
    let cli = Cli::try_parse_from(["cueloop", "prompt", "worker", "--repo-prompt", "tools"])
        .expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Worker(w) => {
                assert_eq!(w.repo_prompt, Some(RepoPromptMode::Tools));
            }
            _ => panic!("expected worker command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_worker_parses_repo_prompt_plan() {
    let cli = Cli::try_parse_from(["cueloop", "prompt", "worker", "--repo-prompt", "plan"])
        .expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Worker(w) => {
                assert_eq!(w.repo_prompt, Some(RepoPromptMode::Plan));
            }
            _ => panic!("expected worker command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_worker_parses_repo_prompt_off() {
    let cli = Cli::try_parse_from(["cueloop", "prompt", "worker", "--repo-prompt", "off"])
        .expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Worker(w) => {
                assert_eq!(w.repo_prompt, Some(RepoPromptMode::Off));
            }
            _ => panic!("expected worker command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_worker_parses_explain() {
    let cli = Cli::try_parse_from(["cueloop", "prompt", "worker", "--explain"]).expect("parse");
    match cli.command {
        Command::Prompt(args) => match args.command {
            PromptCommand::Worker(w) => {
                assert!(w.explain);
            }
            _ => panic!("expected worker command"),
        },
        _ => panic!("expected prompt command"),
    }
}

#[test]
fn prompt_worker_rejects_phase_0() {
    let err = Cli::try_parse_from(["cueloop", "prompt", "worker", "--phase", "0"])
        .err()
        .expect("parse failure");
    let msg = err.to_string();
    assert!(msg.contains("invalid phase"), "unexpected error: {msg}");
}

#[test]
fn prompt_worker_rejects_phase_4() {
    let err = Cli::try_parse_from(["cueloop", "prompt", "worker", "--phase", "4"])
        .err()
        .expect("parse failure");
    let msg = err.to_string();
    assert!(msg.contains("invalid phase"), "unexpected error: {msg}");
}

#[test]
fn prompt_worker_rejects_phase_5() {
    let err = Cli::try_parse_from(["cueloop", "prompt", "worker", "--phase", "5"])
        .err()
        .expect("parse failure");
    let msg = err.to_string();
    assert!(msg.contains("invalid phase"), "unexpected error: {msg}");
}

#[test]
fn prompt_worker_rejects_phase_word() {
    let err = Cli::try_parse_from(["cueloop", "prompt", "worker", "--phase", "one"])
        .err()
        .expect("parse failure");
    let msg = err.to_string();
    assert!(msg.contains("invalid phase"), "unexpected error: {msg}");
}
