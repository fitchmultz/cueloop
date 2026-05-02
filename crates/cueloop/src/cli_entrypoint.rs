//! Shared CueLoop CLI entrypoint and command routing.
//!
//! Purpose:
//! - Shared entrypoint and command routing for the primary `cueloop` binary and legacy `ralph` alias.
//!
//! Responsibilities:
//! - Load environment defaults, parse CLI args, and dispatch to command handlers.
//! - Initialize logging/redaction and apply CLI-level behavior toggles.
//!
//! Not handled here:
//! - CLI flag definitions (see `crate::cli`).
//! - Queue persistence, prompt rendering, or runner execution.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - CLI arguments are normalized before Clap parsing.
//! - Command handlers enforce their own safety checks and validation.

use crate::{cli, redaction, sanity};
use anyhow::{Context, Result};
use clap::{CommandFactory, FromArgMatches};
use std::ffi::OsString;

pub fn main() {
    let args = normalize_repo_prompt_args(std::env::args_os());
    let is_machine_command = is_machine_command_args(&args);

    if let Err(err) = run(args, is_machine_command) {
        if is_machine_command {
            if let Err(print_err) = crate::cli::machine::print_machine_error(&err) {
                use colored::Colorize;
                let msg = format!(
                    "{:#}\nfailed to emit machine error JSON: {print_err:#}",
                    err
                );
                let redacted = redaction::redact_text(&msg);
                eprintln!("{} {}", "Error:".red().bold(), redacted);
            }
        } else {
            use colored::Colorize;
            let msg = format!("{:#}", err);
            let redacted = redaction::redact_text(&msg);
            eprintln!("{} {}", "Error:".red().bold(), redacted);
        }
        std::process::exit(1);
    }
}

fn run(args: Vec<OsString>, is_machine_command: bool) -> Result<()> {
    // Load .env file, warning on errors but ignoring "not found"
    if let Err(e) = dotenvy::dotenv() {
        // Only warn on non-NotFound errors (e.g., permission denied, parse errors)
        if is_not_found_error(&e) {
            // Silently ignore - no .env file is expected
        } else if is_machine_command {
            // Machine commands must keep stderr JSON-only. The logger is not initialized
            // yet, so suppress startup dotenv prose until structured machine output owns
            // the streams.
        } else {
            // Note: Logger isn't initialized yet, use eprintln
            // Redact to avoid accidentally logging secrets from malformed .env files
            let msg = format!("Warning: failed to load .env file: {e}");
            eprintln!("{}", redaction::redact_text(&msg));
        }
    }
    let cli = parse_cli(args);

    // Initialize color output settings early, before any colored output
    cli::color::init_color(cli.color, cli.no_color);

    let mut builder = env_logger::Builder::from_default_env();
    if cli.verbose {
        builder.filter_level(log::LevelFilter::Debug);
    } else if std::env::var("RUST_LOG").is_err() {
        builder.filter_level(log::LevelFilter::Info);
    }

    // We want to capture the max level *before* we consume the builder into a logger,
    // but env_logger::Builder doesn't expose it easily after build.
    // However, we can set the global max level ourselves after init if we knew it.
    // A simpler approach with env_logger 0.11+ is to let it parse env vars, then build.
    // But `builder.init()` consumes the builder and sets the logger.
    // We need `builder.build()` to get the logger, then wrap it.
    let logger = builder.build();
    let max_level = logger.filter();
    redaction::RedactedLogger::init(Box::new(logger), max_level)
        .context("initialize redacted logger")?;

    // Run temp cleanup on every invocation to catch orphaned files from crashed sessions
    if let Err(err) =
        crate::fsutil::cleanup_default_temp_dirs(crate::constants::timeouts::TEMP_RETENTION)
    {
        log::debug!("startup temp cleanup: {:#}", err);
    }

    if should_emit_legacy_compat_warnings(&cli) {
        emit_legacy_compat_warnings();
    }

    // Ensure README guidance stays current for agent-facing commands even when
    // full sanity checks are skipped for this command.
    let sanity_mode = sanity::startup_sanity_mode(&cli.command);
    let should_run_sanity = !matches!(sanity_mode, sanity::StartupSanityMode::None);
    let should_refresh_readme = sanity::should_refresh_readme_for_command(&cli.command);
    if should_refresh_readme && (cli.no_sanity_checks || !should_run_sanity) {
        let resolved = crate::config::resolve_from_cwd_for_doctor()?;
        if let Some(msg) = sanity::refresh_readme_if_needed(&resolved)? {
            log::info!("{}", msg);
        }
    }

    // Run full sanity checks before commands that need them
    if should_run_sanity && !cli.no_sanity_checks {
        let resolved = crate::config::resolve_from_cwd_for_doctor()?;
        // Extract non_interactive flag from run commands
        let run_non_interactive = match &cli.command {
            cli::Command::Run(run_args) => match &run_args.command {
                cli::run::RunCommand::One(one_args) => one_args.non_interactive,
                cli::run::RunCommand::Loop(loop_args) => loop_args.non_interactive,
                cli::run::RunCommand::Resume(resume_args) => resume_args.non_interactive,
                cli::run::RunCommand::Parallel(_) => true, // Parallel ops are non-interactive
            },
            _ => false,
        };
        let options = match sanity_mode {
            sanity::StartupSanityMode::Mutating => sanity::SanityOptions {
                auto_fix: cli.auto_fix,
                skip: false,
                non_interactive: run_non_interactive,
                write_policy: sanity::SanityWritePolicy::AllowWrites,
            },
            sanity::StartupSanityMode::ReadOnly => sanity::SanityOptions {
                auto_fix: false,
                skip: false,
                non_interactive: true,
                write_policy: sanity::SanityWritePolicy::ReadOnly,
            },
            sanity::StartupSanityMode::None => unreachable!("checked above"),
        };
        let sanity_result = sanity::run_sanity_checks(&resolved, &options)?;

        // If there are issues that need attention and we're not in auto-fix mode,
        // we might want to warn the user
        if !sanity::report_sanity_results(&sanity_result, options.auto_fix) {
            anyhow::bail!(
                "Sanity checks failed. Please resolve the issues above or run with --auto-fix."
            );
        }
    }

    match cli.command {
        cli::Command::Queue(args) => cli::queue::handle_queue(args.command, cli.force),
        cli::Command::Config(args) => cli::config::handle_config(args.command),
        cli::Command::HelpAll => {
            cli::handle_help_all();
            Ok(())
        }
        cli::Command::Machine(args) => cli::machine::handle_machine(*args, cli.force),
        cli::Command::Run(args) => cli::run::handle_run(args.command, cli.force),
        cli::Command::Task(args) => cli::task::handle_task(*args, cli.force),
        cli::Command::Scan(args) => cli::scan::handle_scan(args, cli.force),
        cli::Command::Init(args) => cli::init::handle_init(args, cli.force),
        cli::Command::App(args) => cli::app::handle_app(args.command),
        cli::Command::Prompt(args) => cli::prompt::handle_prompt(args),
        cli::Command::Doctor(args) => cli::doctor::handle_doctor(args),
        cli::Command::Context(args) => cli::context::handle_context(args),
        cli::Command::Prd(args) => cli::prd::handle_prd(args, cli.force),
        cli::Command::Completions(args) => cli::completions::handle_completions(args),
        cli::Command::Migrate(args) => cli::migrate::handle_migrate(args),
        cli::Command::Cleanup(args) => cli::cleanup::handle_cleanup(args),
        cli::Command::Version(args) => cli::version::handle_version(args),
        cli::Command::Watch(args) => cli::watch::handle_watch(args, cli.force),
        cli::Command::Webhook(args) => {
            let resolved = crate::config::resolve_from_cwd()?;
            cli::webhook::handle_webhook(&args, &resolved)
        }
        cli::Command::Productivity(args) => cli::productivity::handle(args),
        cli::Command::Plugin(args) => {
            let resolved = crate::config::resolve_from_cwd()?;
            crate::commands::plugin::run(&args, &resolved)
        }
        cli::Command::Runner(args) => match args.command {
            cli::runner::RunnerCommand::Capabilities(cap_args) => {
                cli::runner::handle_runner_capabilities(cap_args)
            }
            cli::runner::RunnerCommand::List(list_args) => {
                cli::runner::handle_runner_list(list_args)
            }
        },
        cli::Command::Daemon(args) => cli::daemon::handle_daemon(args.command),
        cli::Command::Tutorial(args) => cli::tutorial::handle_tutorial(args),
        cli::Command::Undo(args) => cli::undo::handle(args, cli.force),
        cli::Command::CliSpec(args) => cli::handle_cli_spec(args),
    }
}

/// Decide whether to emit human-only legacy compatibility warnings.
fn should_emit_legacy_compat_warnings(cli: &cli::Cli) -> bool {
    match &cli.command {
        cli::Command::Machine(_)
        | cli::Command::CliSpec(_)
        | cli::Command::Completions(_)
        | cli::Command::HelpAll
        | cli::Command::Queue(_) => false,
        cli::Command::Config(args) => !matches!(
            &args.command,
            cli::config::ConfigCommand::Show(show_args)
                if matches!(show_args.format, cli::config::ConfigShowFormat::Json)
        ),
        cli::Command::Doctor(args) => !matches!(args.format, cli::doctor::DoctorFormat::Json),
        cli::Command::Init(_) | cli::Command::Migrate(_) => true,
        cli::Command::Plugin(args) => !matches!(
            &args.command,
            cli::plugin::PluginCommand::List { json: true }
        ),
        cli::Command::Prompt(args) => matches!(&args.command, cli::prompt::PromptCommand::List),
        _ => false,
    }
}

fn parse_cli(args: Vec<OsString>) -> cli::Cli {
    let invoked_name = args
        .first()
        .and_then(|arg| std::path::Path::new(arg).file_name())
        .and_then(|name| name.to_str())
        .map(|name| name.to_owned())
        .unwrap_or_else(|| crate::constants::identity::CLI_BIN_NAME.to_owned());
    let matches = cli::Cli::command()
        .bin_name(invoked_name.clone())
        .display_name(invoked_name)
        .get_matches_from(args);

    cli::Cli::from_arg_matches(&matches).unwrap_or_else(|err| err.exit())
}

fn emit_legacy_compat_warnings() {
    let Ok(cwd) = std::env::current_dir() else {
        return;
    };
    let repo_root = crate::config::find_repo_root(&cwd);
    let current_runtime = repo_root.join(crate::constants::identity::PROJECT_RUNTIME_DIR);
    let legacy_runtime = repo_root.join(crate::constants::identity::LEGACY_PROJECT_RUNTIME_DIR);

    if current_runtime.is_dir() && legacy_runtime.is_dir() {
        log::warn!(
            "CueLoop found both .cueloop and legacy .cueloop runtime directories. .cueloop takes precedence; resolve or remove .ralph when ready."
        );
    } else if matches!(
        crate::config::project_runtime_layout(&repo_root),
        crate::config::ProjectRuntimeLayout::Legacy
    ) {
        log::warn!(
            "CueLoop is using legacy project runtime directory .ralph. This remains supported for now. Run `cueloop migrate runtime-dir --apply` to move project state to .cueloop."
        );
    }

    if let Some(legacy_global) = crate::config::legacy_global_config_path()
        && legacy_global.exists()
    {
        let current_global_exists =
            crate::config::global_config_path().is_some_and(|path| path.exists());
        let suffix = if current_global_exists {
            " Current ~/.config/cueloop/config.jsonc still takes precedence."
        } else {
            " Move it to ~/.config/cueloop/config.jsonc when ready."
        };
        log::warn!(
            "CueLoop found legacy global config {}.{}",
            legacy_global.display(),
            suffix
        );
    }
}

/// Check if a dotenvy error is a "file not found" error.
/// This is the only error we silently ignore.
fn is_not_found_error(error: &dotenvy::Error) -> bool {
    use std::io;
    match error {
        dotenvy::Error::Io(io_err) if io_err.kind() == io::ErrorKind::NotFound => true,
        // Also check for the generic "not found" case from dotenvy's internal handling
        _ => {
            let err_str = error.to_string().to_lowercase();
            err_str.contains("not found") || err_str.contains("no such file")
        }
    }
}

fn is_machine_command_args(args: &[OsString]) -> bool {
    let mut iter = args.iter().skip(1);

    while let Some(arg) = iter.next() {
        let Some(value) = arg.to_str() else {
            return false;
        };

        match value {
            "--force" | "-f" | "--verbose" | "-v" | "--no-color" | "--auto-fix"
            | "--no-sanity-checks" => continue,
            "--color" => {
                let _ = iter.next();
                continue;
            }
            _ if value.starts_with("--color=") => continue,
            _ => return value == "machine",
        }
    }

    false
}

fn normalize_repo_prompt_args<I>(args: I) -> Vec<OsString>
where
    I: IntoIterator<Item = OsString>,
{
    let mut normalized = Vec::new();
    let mut passthrough = false;

    for arg in args {
        if passthrough {
            normalized.push(arg);
            continue;
        }

        if arg == std::ffi::OsStr::new("--") {
            passthrough = true;
            normalized.push(arg);
            continue;
        }

        let as_str = arg.to_str();
        if as_str == Some("-rp") {
            normalized.push(OsString::from("--repo-prompt"));
            continue;
        }
        if let Some(value) = as_str.and_then(|s| s.strip_prefix("-rp=")) {
            let mut rewritten = OsString::from("--repo-prompt=");
            rewritten.push(value);
            normalized.push(rewritten);
            continue;
        }

        normalized.push(arg);
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_repo_prompt_args_rewrites_short_flag() {
        let args = vec![
            OsString::from("ralph"),
            OsString::from("-rp"),
            OsString::from("plan"),
        ];
        let normalized = normalize_repo_prompt_args(args);
        assert_eq!(
            normalized,
            vec![
                OsString::from("ralph"),
                OsString::from("--repo-prompt"),
                OsString::from("plan")
            ]
        );
    }

    #[test]
    fn normalize_repo_prompt_args_rewrites_equals_form() {
        let args = vec![OsString::from("ralph"), OsString::from("-rp=tools")];
        let normalized = normalize_repo_prompt_args(args);
        assert_eq!(
            normalized,
            vec![
                OsString::from("ralph"),
                OsString::from("--repo-prompt=tools")
            ]
        );
    }

    #[test]
    fn normalize_repo_prompt_args_respects_double_dash() {
        let args = vec![
            OsString::from("ralph"),
            OsString::from("--"),
            OsString::from("-rp"),
            OsString::from("plan"),
        ];
        let normalized = normalize_repo_prompt_args(args);
        assert_eq!(
            normalized,
            vec![
                OsString::from("ralph"),
                OsString::from("--"),
                OsString::from("-rp"),
                OsString::from("plan")
            ]
        );
    }

    #[test]
    fn is_machine_command_args_detects_machine_after_globals() {
        let args = vec![
            OsString::from("ralph"),
            OsString::from("--no-color"),
            OsString::from("--color=never"),
            OsString::from("machine"),
            OsString::from("queue"),
            OsString::from("read"),
        ];

        assert!(is_machine_command_args(&args));
    }

    #[test]
    fn is_machine_command_args_rejects_non_machine_commands() {
        let args = vec![
            OsString::from("ralph"),
            OsString::from("--verbose"),
            OsString::from("queue"),
            OsString::from("read"),
        ];

        assert!(!is_machine_command_args(&args));
    }
}
