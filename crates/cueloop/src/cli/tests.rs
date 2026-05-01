//! Parse-regression tests for the top-level CLI surface.
//!
//! Purpose:
//! - Parse-regression tests for the top-level CLI surface.
//!
//! Responsibilities:
//! - Verify key top-level command routes and rejected legacy flags.
//! - Keep root CLI parsing coverage out of the root facade file.
//! - Assert version/help behaviors exposed by Clap.
//!
//! Not handled here:
//! - Exhaustive per-subcommand argument validation owned by submodules.
//! - Runtime execution behavior after parsing succeeds.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Tests exercise the public `Cli` parser exactly as end users invoke it.
//! - Removed flags/subcommands must remain rejected.

use super::{Cli, Command};
use crate::cli::app_parity::{
    APP_PARITY_SCENARIO_REGISTRY, app_parity_scenario_coverage_issues, app_parity_scenario_report,
    unclassified_human_cli_commands,
};
use crate::cli::{machine, queue, run, task};
use clap::Parser;
use clap::error::ErrorKind;
use std::path::{Path, PathBuf};

fn assert_proof_anchor_exists(anchor: &str) {
    let (relative_path, symbol) = anchor
        .rsplit_once("::")
        .unwrap_or_else(|| panic!("proof anchor must include a path and symbol: {anchor}"));
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source_path: PathBuf = repo_root.join(relative_path);
    assert!(
        source_path.is_file(),
        "proof anchor path does not exist: {anchor}"
    );
    let source = std::fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read proof anchor source {anchor}: {err}"));
    assert!(
        source.contains(symbol),
        "proof anchor symbol does not exist in source: {anchor}"
    );
}

#[test]
fn app_parity_registry_classifies_every_human_cli_root_command() {
    let missing = unclassified_human_cli_commands();
    assert!(
        missing.is_empty(),
        "new human-facing CLI commands need RalphMac parity registry entries: {missing:?}"
    );
}

#[test]
fn app_parity_registry_tracks_required_scenarios_with_proof() {
    let required = [
        "run_loop_empty_queue_summary",
        "run_loop_blocked_queue_summary",
        "run_loop_failure_after_run_started",
        "run_stop_after_current_machine_contract",
        "workspace_custom_queue_path_resolution",
        "execution_controls_plugin_runner_visibility",
        "execution_controls_parallel_workers_above_menu_default",
        "continuation_next_steps_native_actions",
    ];

    for scenario in required {
        assert!(
            APP_PARITY_SCENARIO_REGISTRY
                .iter()
                .any(|entry| entry.scenario == scenario),
            "required app parity scenario is missing: {scenario}"
        );
    }
}

#[test]
fn app_parity_registry_requires_contract_and_test_anchors() {
    let issues = app_parity_scenario_coverage_issues();
    assert!(issues.is_empty(), "{}", app_parity_scenario_report());
}

#[test]
fn app_parity_registry_proof_anchors_point_to_real_tests() {
    for entry in APP_PARITY_SCENARIO_REGISTRY {
        for anchor in entry.rust_tests {
            assert_proof_anchor_exists(anchor);
        }
        for anchor in entry.app_tests {
            assert_proof_anchor_exists(anchor);
        }
    }
}

#[test]
fn cli_parses_queue_list_smoke() {
    let cli = Cli::try_parse_from(["ralph", "queue", "list"]).expect("parse");
    match cli.command {
        Command::Queue(_) => {}
        other => panic!(
            "expected queue command, got {:?}",
            std::mem::discriminant(&other)
        ),
    }
}

#[test]
fn cli_parses_queue_archive_subcommand() {
    let cli = Cli::try_parse_from(["ralph", "queue", "archive"]).expect("parse");
    match cli.command {
        Command::Queue(queue::QueueArgs { command }) => match command {
            queue::QueueCommand::Archive(_) => {}
            _ => panic!("expected queue archive command"),
        },
        _ => panic!("expected queue command"),
    }
}

#[test]
fn cli_rejects_invalid_prompt_phase() {
    let err = Cli::try_parse_from(["ralph", "prompt", "worker", "--phase", "4"])
        .err()
        .expect("parse failure");
    let msg = err.to_string();
    assert!(msg.contains("invalid phase"), "unexpected error: {msg}");
}

#[test]
fn cli_parses_run_git_revert_mode() {
    let cli = Cli::try_parse_from(["ralph", "run", "one", "--git-revert-mode", "disabled"])
        .expect("parse");
    match cli.command {
        Command::Run(args) => match args.command {
            run::RunCommand::One(args) => {
                assert_eq!(args.agent.git_revert_mode.as_deref(), Some("disabled"));
            }
            _ => panic!("expected run one command"),
        },
        _ => panic!("expected run command"),
    }
}

#[test]
fn cli_parses_run_git_publish_mode() {
    let cli =
        Cli::try_parse_from(["ralph", "run", "one", "--git-publish-mode", "off"]).expect("parse");
    match cli.command {
        Command::Run(args) => match args.command {
            run::RunCommand::One(args) => {
                assert_eq!(args.agent.git_publish_mode.as_deref(), Some("off"));
            }
            _ => panic!("expected run one command"),
        },
        _ => panic!("expected run command"),
    }
}

#[test]
fn cli_parses_run_include_draft() {
    let cli = Cli::try_parse_from(["ralph", "run", "one", "--include-draft"]).expect("parse");
    match cli.command {
        Command::Run(args) => match args.command {
            run::RunCommand::One(args) => {
                assert!(args.agent.include_draft);
            }
            _ => panic!("expected run one command"),
        },
        _ => panic!("expected run command"),
    }
}

#[test]
fn cli_parses_run_one_debug() {
    let cli = Cli::try_parse_from(["ralph", "run", "one", "--debug"]).expect("parse");
    match cli.command {
        Command::Run(args) => match args.command {
            run::RunCommand::One(args) => {
                assert!(args.debug);
            }
            _ => panic!("expected run one command"),
        },
        _ => panic!("expected run command"),
    }
}

#[test]
fn cli_parses_run_loop_debug() {
    let cli = Cli::try_parse_from(["ralph", "run", "loop", "--debug"]).expect("parse");
    match cli.command {
        Command::Run(args) => match args.command {
            run::RunCommand::Loop(args) => {
                assert!(args.debug);
            }
            _ => panic!("expected run loop command"),
        },
        _ => panic!("expected run command"),
    }
}

#[test]
fn cli_parses_machine_run_loop_parallel_override() {
    let cli =
        Cli::try_parse_from(["ralph", "machine", "run", "loop", "--parallel", "3"]).expect("parse");
    match cli.command {
        Command::Machine(args) => match args.command {
            machine::MachineCommand::Run(args) => match args.command {
                machine::MachineRunCommand::Loop(args) => {
                    assert_eq!(args.parallel, Some(3));
                }
                _ => panic!("expected machine run loop command"),
            },
            _ => panic!("expected machine run command"),
        },
        _ => panic!("expected machine command"),
    }
}

#[test]
fn cli_parses_machine_run_loop_parallel_default_missing_value() {
    let cli =
        Cli::try_parse_from(["ralph", "machine", "run", "loop", "--parallel"]).expect("parse");
    match cli.command {
        Command::Machine(args) => match args.command {
            machine::MachineCommand::Run(args) => match args.command {
                machine::MachineRunCommand::Loop(args) => {
                    assert_eq!(args.parallel, Some(2));
                }
                _ => panic!("expected machine run loop command"),
            },
            _ => panic!("expected machine run command"),
        },
        _ => panic!("expected machine command"),
    }
}

#[test]
fn cli_parses_machine_run_stop_dry_run() {
    let cli = Cli::try_parse_from(["ralph", "machine", "run", "stop", "--dry-run"]).expect("parse");
    match cli.command {
        Command::Machine(args) => match args.command {
            machine::MachineCommand::Run(args) => match args.command {
                machine::MachineRunCommand::Stop(args) => {
                    assert!(args.dry_run);
                }
                _ => panic!("expected machine run stop command"),
            },
            _ => panic!("expected machine run command"),
        },
        _ => panic!("expected machine command"),
    }
}

#[test]
fn cli_parses_machine_task_build_input() {
    let cli = Cli::try_parse_from([
        "ralph",
        "machine",
        "task",
        "build",
        "--input",
        "request.json",
    ])
    .expect("parse");
    match cli.command {
        Command::Machine(args) => match args.command {
            machine::MachineCommand::Task(args) => match args.command {
                machine::MachineTaskCommand::Build(args) => {
                    assert_eq!(args.input.as_deref(), Some("request.json"));
                }
                _ => panic!("expected machine task build command"),
            },
            _ => panic!("expected machine task command"),
        },
        _ => panic!("expected machine command"),
    }
}

#[test]
fn cli_parses_run_one_id() {
    let cli = Cli::try_parse_from(["ralph", "run", "one", "--id", "RQ-0001"]).expect("parse");
    match cli.command {
        Command::Run(args) => match args.command {
            run::RunCommand::One(args) => {
                assert_eq!(args.id.as_deref(), Some("RQ-0001"));
            }
            _ => panic!("expected run one command"),
        },
        _ => panic!("expected run command"),
    }
}

#[test]
fn cli_parses_task_update_without_id() {
    let cli = Cli::try_parse_from(["ralph", "task", "update"]).expect("parse");
    match cli.command {
        Command::Task(args) => match args.command {
            Some(task::TaskCommand::Update(args)) => {
                assert!(args.task_id.is_none());
            }
            _ => panic!("expected task update command"),
        },
        _ => panic!("expected task command"),
    }
}

#[test]
fn cli_parses_task_update_with_id() {
    let cli = Cli::try_parse_from(["ralph", "task", "update", "RQ-0001"]).expect("parse");
    match cli.command {
        Command::Task(args) => match args.command {
            Some(task::TaskCommand::Update(args)) => {
                assert_eq!(args.task_id.as_deref(), Some("RQ-0001"));
            }
            _ => panic!("expected task update command"),
        },
        _ => panic!("expected task command"),
    }
}

#[test]
fn cli_rejects_removed_run_one_interactive_flag_short() {
    let err = Cli::try_parse_from(["ralph", "run", "one", "-i"])
        .err()
        .expect("parse failure");
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("unexpected") || msg.contains("unrecognized") || msg.contains("unknown"),
        "unexpected error: {msg}"
    );
}

#[test]
fn cli_rejects_removed_run_one_interactive_flag_long() {
    let err = Cli::try_parse_from(["ralph", "run", "one", "--interactive"])
        .err()
        .expect("parse failure");
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("unexpected") || msg.contains("unrecognized") || msg.contains("unknown"),
        "unexpected error: {msg}"
    );
}

#[test]
fn cli_parses_task_default_subcommand() {
    let cli = Cli::try_parse_from(["ralph", "task", "Add", "tests"]).expect("parse");
    match cli.command {
        Command::Task(args) => {
            assert!(args.command.is_none(), "expected implicit build subcommand");
            assert_eq!(
                args.build.request,
                vec!["Add".to_string(), "tests".to_string()]
            );
        }
        _ => panic!("expected task command"),
    }
}

#[test]
fn cli_parses_task_ready_subcommand() {
    let cli = Cli::try_parse_from(["ralph", "task", "ready", "RQ-0005"]).expect("parse");
    match cli.command {
        Command::Task(args) => match args.command {
            Some(task::TaskCommand::Ready(args)) => {
                assert_eq!(args.task_id, "RQ-0005");
            }
            _ => panic!("expected task ready command"),
        },
        _ => panic!("expected task command"),
    }
}

#[test]
fn cli_parses_task_done_subcommand() {
    let cli = Cli::try_parse_from(["ralph", "task", "done", "RQ-0001"]).expect("parse");
    match cli.command {
        Command::Task(args) => match args.command {
            Some(task::TaskCommand::Done(args)) => {
                assert_eq!(args.task_id, "RQ-0001");
            }
            _ => panic!("expected task done command"),
        },
        _ => panic!("expected task command"),
    }
}

#[test]
fn cli_parses_task_reject_subcommand() {
    let cli = Cli::try_parse_from(["ralph", "task", "reject", "RQ-0002"]).expect("parse");
    match cli.command {
        Command::Task(args) => match args.command {
            Some(task::TaskCommand::Reject(args)) => {
                assert_eq!(args.task_id, "RQ-0002");
            }
            _ => panic!("expected task reject command"),
        },
        _ => panic!("expected task command"),
    }
}

#[test]
fn cli_rejects_queue_set_status_subcommand() {
    let result = Cli::try_parse_from(["ralph", "queue", "set-status", "RQ-0001", "doing"]);
    assert!(result.is_err(), "expected queue set-status to be rejected");
    let msg = result
        .err()
        .expect("queue set-status error")
        .to_string()
        .to_lowercase();
    assert!(
        msg.contains("unrecognized") || msg.contains("unexpected") || msg.contains("unknown"),
        "unexpected error: {msg}"
    );
}

#[test]
fn cli_rejects_removed_run_loop_interactive_flag_short() {
    let err = Cli::try_parse_from(["ralph", "run", "loop", "-i"])
        .err()
        .expect("parse failure");
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("unexpected") || msg.contains("unrecognized") || msg.contains("unknown"),
        "unexpected error: {msg}"
    );
}

#[test]
fn cli_rejects_removed_run_loop_interactive_flag_long() {
    let err = Cli::try_parse_from(["ralph", "run", "loop", "--interactive"])
        .err()
        .expect("parse failure");
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("unexpected") || msg.contains("unrecognized") || msg.contains("unknown"),
        "unexpected error: {msg}"
    );
}

#[test]
fn cli_rejects_removed_tui_command() {
    let err = Cli::try_parse_from(["ralph", "tui"])
        .err()
        .expect("parse failure");
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("unexpected") || msg.contains("unrecognized") || msg.contains("unknown"),
        "unexpected error: {msg}"
    );
}

#[test]
fn cli_rejects_run_loop_with_id_flag() {
    let err = Cli::try_parse_from(["ralph", "run", "loop", "--id", "RQ-0001"])
        .err()
        .expect("parse failure");
    let msg = err.to_string();
    assert!(
        msg.contains("unexpected") || msg.contains("unrecognized") || msg.contains("unknown"),
        "unexpected error: {msg}"
    );
}

#[test]
fn cli_supports_top_level_version_flag_long() {
    let err = Cli::try_parse_from(["cueloop", "--version"])
        .err()
        .expect("expected clap to render version and exit");
    assert_eq!(err.kind(), ErrorKind::DisplayVersion);
    let rendered = err.to_string();
    assert!(rendered.contains("cueloop"));
    assert!(rendered.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn cli_supports_top_level_version_flag_short() {
    let err = Cli::try_parse_from(["cueloop", "-V"])
        .err()
        .expect("expected clap to render version and exit");
    assert_eq!(err.kind(), ErrorKind::DisplayVersion);
    let rendered = err.to_string();
    assert!(rendered.contains("cueloop"));
    assert!(rendered.contains(env!("CARGO_PKG_VERSION")));
}
