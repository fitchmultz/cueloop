//! Ralph CLI entrypoint and command routing.

mod agent;
mod cli;
mod completions;
mod config;
mod contracts;
mod doctor_cmd;
mod fsutil;
mod gitutil;
mod init_cmd;
mod outpututil;
mod prompt_cmd;
mod promptflow;
mod prompts;
mod queue;
mod redaction;
mod reports;
mod run_cmd;
mod runner;
mod runutil;
mod scan_cmd;
mod task_cmd;
mod timeutil;
mod tui;

use anyhow::{Context, Result};
use clap::Parser;

fn main() {
    if let Err(err) = run() {
        use colored::Colorize;
        let msg = format!("{:#}", err);
        let redacted = redaction::redact_text(&msg);
        eprintln!("{} {}", "Error:".red().bold(), redacted);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = cli::Cli::parse();

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

    match cli.command {
        cli::Command::Queue(args) => crate::cli::queue::handle_queue(args.command, cli.force),
        cli::Command::Config(args) => crate::cli::config::handle_config(args.command),
        cli::Command::Run(args) => crate::cli::run::handle_run(args.command, cli.force),
        cli::Command::Task(args) => crate::cli::task::handle_task(args.command, cli.force),
        cli::Command::Scan(args) => crate::cli::scan::handle_scan(args, cli.force),
        cli::Command::Init(args) => crate::cli::init::handle_init(args, cli.force),
        cli::Command::Prompt(args) => crate::cli::prompt::handle_prompt(args),
        cli::Command::Doctor => crate::cli::doctor::handle_doctor(),
    }
}
