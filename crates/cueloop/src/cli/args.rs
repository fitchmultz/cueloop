//! Top-level Clap argument definitions for CueLoop CLI binaries.
//!
//! Purpose:
//! - Top-level Clap argument definitions for the `cueloop` command.
//!
//! Responsibilities:
//! - Define the root `Cli` parser and the top-level command enum.
//! - Keep top-level command documentation and long-help text together.
//! - Re-export the small `cli-spec` format args used by the machine/app tooling.
//!
//! Not handled here:
//! - Command execution logic.
//! - Shared queue/list helper functions.
//! - Parse-regression tests.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Subcommands validate their own inputs and config dependencies.
//! - CLI parsing happens after argument normalization in `main`.

use clap::{Args, Parser, Subcommand, ValueEnum};

use super::{
    app, cleanup, color::ColorArg, completions, config, context, daemon, doctor, init, machine,
    migrate, plugin, prd, productivity, prompt, queue, run, runner, scan, task, tutorial, undo,
    version, watch, webhook,
};

#[derive(Parser)]
#[command(name = "cueloop")]
#[command(about = "CueLoop CLI")]
#[command(version)]
#[command(after_long_help = r#"Runner selection:
  - CLI flags override project config, which overrides global config, which overrides built-in defaults.
  - Default runner/model come from config files: project config (.cueloop/config.jsonc, with .cueloop fallback) > global config (~/.config/cueloop/config.jsonc, with ~/.config/cueloop fallback) > built-in.
  - `task` and `scan` accept --runner/--model/--effort as one-off overrides.
  - `run one` and `run loop` accept --runner/--model/--effort as one-off overrides; otherwise they use task.agent overrides when present; otherwise config agent defaults.

Config example (.cueloop/config.jsonc):
  {
    "version": 2,
    "agent": {
      "runner": "codex",
      "model": "gpt-5.4",
      "codex_bin": "codex",
      "gemini_bin": "gemini",
      "claude_bin": "claude"
    }
  }

Notes:
  - Allowed runners: codex, opencode, gemini, claude, cursor, kimi, pi
  - Allowed models: gpt-5.4, gpt-5.3-codex, gpt-5.3-codex-spark, gpt-5.3, zai-coding-plan/glm-4.7, gemini-3-pro-preview, gemini-3-flash-preview, sonnet, opus, kimi-for-coding (codex supports only gpt-5.4 + gpt-5.3-codex + gpt-5.3-codex-spark + gpt-5.3; opencode/gemini/claude/cursor/kimi/pi accept arbitrary model ids))
  - CueLoop is the product and primary executable name; `cueloop` remains a compatibility alias in this phase.
  - New repos default to `.cueloop/`; legacy `.cueloop/` remains supported. Use `cueloop migrate runtime-dir --apply` when ready.
  - On macOS: use `cueloop app open` to launch the GUI (app bundle rename is out of scope for this phase).
  - App-launched runs are noninteractive: they stream output, but interactive approvals remain terminal-only.

Examples:
  cueloop app open
  cueloop queue list
  cueloop queue show RQ-0008
  cueloop queue next --with-title
  cueloop scan --runner opencode --model gpt-5.3 --focus "CI gaps"
  cueloop task --runner codex --model gpt-5.4 --effort high "Fix the flaky test"
  cueloop scan --runner gemini --model gemini-3-flash-preview --focus "risk audit"
  cueloop scan --runner claude --model sonnet --focus "risk audit"
  cueloop task --runner claude --model opus "Add tests for X"
  cueloop scan --runner cursor --model claude-opus-4-5-20251101 --focus "risk audit"
  cueloop task --runner cursor --model claude-opus-4-5-20251101 "Add tests for X"
  cueloop scan --runner kimi --focus "risk audit"
  cueloop task --runner kimi --model kimi-for-coding "Add tests for X"
  cueloop run one
  cueloop run loop --max-tasks 1
  cueloop run loop

More help:
  - Default help shows core commands only.
  - Run `cueloop help-all` to see advanced and experimental commands."#)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Force operations (e.g., bypass clean-repo safety checks for commands that enforce them, e.g., `run one`, `run loop`, and `scan`).
    #[arg(long, global = true)]
    pub force: bool,

    /// Increase output verbosity (sets log level to info).
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Color output control.
    #[arg(long, value_enum, default_value = "auto", global = true)]
    pub color: ColorArg,

    /// Disable colored output (alias for `--color never`).
    /// Also respects the NO_COLOR environment variable.
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Automatically approve all migrations and fixes without prompting.
    /// Useful for CI/scripting environments.
    #[arg(long, global = true, conflicts_with = "no_sanity_checks")]
    pub auto_fix: bool,

    /// Skip startup sanity checks (migrations and unknown-key prompts).
    #[arg(long, global = true, conflicts_with = "auto_fix")]
    pub no_sanity_checks: bool,
}

#[derive(Subcommand)]
pub enum Command {
    Queue(queue::QueueArgs),
    Config(config::ConfigArgs),
    Run(Box<run::RunArgs>),
    Task(Box<task::TaskArgs>),
    Scan(scan::ScanArgs),
    Init(init::InitArgs),
    /// macOS app integration commands.
    App(app::AppArgs),
    /// Show core, advanced, and experimental command groups.
    HelpAll,
    /// Versioned machine-facing JSON API for the macOS app.
    #[command(hide = true)]
    Machine(Box<machine::MachineArgs>),
    /// Render and print the final compiled prompts used by CueLoop (for debugging/auditing).
    #[command(
        hide = true,
        after_long_help = "Examples:\n  cueloop prompt worker --phase 1 --repo-prompt plan\n  cueloop prompt worker --phase 2 --task-id RQ-0001 --plan-file .cueloop/cache/plans/RQ-0001.md\n  cueloop prompt scan --focus \"CI gaps\" --repo-prompt off\n  cueloop prompt task-builder --request \"Add tests\" --tags rust,tests --scope crates/cueloop --repo-prompt tools\n"
    )]
    Prompt(prompt::PromptArgs),
    /// Verify environment readiness and configuration.
    #[command(
        hide = true,
        after_long_help = "Examples:\n  cueloop doctor\n  cueloop doctor --auto-fix\n  cueloop doctor --no-sanity-checks\n  cueloop doctor --format json\n  cueloop doctor --format json --auto-fix"
    )]
    Doctor(doctor::DoctorArgs),
    /// Manage project context (AGENTS.md) for AI agents.
    #[command(
        hide = true,
        after_long_help = "Examples:\n  cueloop context init\n  cueloop context init --project-type rust\n  cueloop context update --section troubleshooting\n  cueloop context validate\n  cueloop context update --dry-run"
    )]
    Context(context::ContextArgs),
    /// Manage the CueLoop daemon (background service).
    #[command(
        hide = true,
        after_long_help = "Examples:\n  cueloop daemon start\n  cueloop daemon start --empty-poll-ms 5000\n  cueloop daemon stop\n  cueloop daemon status"
    )]
    Daemon(daemon::DaemonArgs),
    /// Convert PRD (Product Requirements Document) markdown to tasks.
    #[command(
        hide = true,
        after_long_help = "Examples:\n  cueloop prd create docs/prd/new-feature.md\n  cueloop prd create docs/prd/new-feature.md --multi\n  cueloop prd create docs/prd/new-feature.md --dry-run\n  cueloop prd create docs/prd/new-feature.md --priority high --tag feature\n  cueloop prd create docs/prd/new-feature.md --draft"
    )]
    Prd(prd::PrdArgs),
    /// Generate shell completion scripts.
    #[command(
        hide = true,
        after_long_help = "Examples:\n  cueloop completions bash\n  cueloop completions bash > ~/.local/share/bash-completion/completions/cueloop\n  cueloop completions zsh > ~/.zfunc/_cueloop\n  cueloop completions fish > ~/.config/fish/completions/cueloop.fish\n  cueloop completions powershell\n\nInstallation locations by shell:\n  Bash:   ~/.local/share/bash-completion/completions/cueloop\n  Zsh:    ~/.zfunc/_cueloop (and add 'fpath+=~/.zfunc' to ~/.zshrc)\n  Fish:   ~/.config/fish/completions/cueloop.fish\n  PowerShell: Add to $PROFILE (see: $PROFILE | Get-Member -Type NoteProperty)"
    )]
    Completions(completions::CompletionsArgs),
    /// Check and apply migrations for config and project files.
    #[command(
        hide = true,
        after_long_help = "Examples:\n  cueloop migrate              # Check for pending config/file migrations\n  cueloop migrate --check      # Exit with error code if migrations pending (CI)\n  cueloop migrate --apply      # Apply all pending config/file migrations\n  cueloop migrate --list       # List all migrations and their status\n  cueloop migrate status       # Show detailed migration status\n  cueloop migrate runtime-dir --check  # Check whether .cueloop should be moved to .cueloop\n  cueloop migrate runtime-dir --apply  # Explicitly move .cueloop project state to .cueloop"
    )]
    Migrate(migrate::MigrateArgs),
    /// Clean up temporary files created by CueLoop.
    #[command(
        hide = true,
        after_long_help = "Examples:\n  cueloop cleanup              # Clean temp files older than 7 days\n  cueloop cleanup --force      # Clean all CueLoop and legacy CueLoop temp files\n  cueloop cleanup --dry-run    # Show what would be deleted without deleting"
    )]
    Cleanup(cleanup::CleanupArgs),
    /// Display version information.
    #[command(after_long_help = "Examples:\n  cueloop version\n  cueloop version --verbose")]
    Version(version::VersionArgs),
    /// Watch files for changes and auto-detect tasks from TODO/FIXME/HACK/XXX comments.
    #[command(
        hide = true,
        after_long_help = "Examples:\n  cueloop watch\n  cueloop watch src/\n  cueloop watch --patterns \"*.rs,*.toml\"\n  cueloop watch --auto-queue\n  cueloop watch --notify\n  cueloop watch --comments todo,fixme\n  cueloop watch --debounce-ms 1000\n  cueloop watch --ignore-patterns \"vendor/,target/,node_modules/\""
    )]
    Watch(watch::WatchArgs),
    /// Webhook management commands.
    #[command(
        hide = true,
        after_long_help = "Examples:\n  cueloop webhook test\n  cueloop webhook test --event task_completed\n  cueloop webhook status --format json\n  cueloop webhook replay --dry-run --id wf-1700000000-1"
    )]
    Webhook(webhook::WebhookArgs),

    /// Productivity analytics (streaks, velocity, milestones).
    #[command(
        hide = true,
        after_long_help = "Examples:\n  cueloop productivity summary\n  cueloop productivity velocity\n  cueloop productivity streak"
    )]
    Productivity(productivity::ProductivityArgs),

    /// Plugin management commands.
    #[command(
        hide = true,
        after_long_help = "Examples:\n  cueloop plugin init my.plugin\n  cueloop plugin init my.plugin --scope global\n  cueloop plugin list\n  cueloop plugin validate\n  cueloop plugin install ./my-plugin --scope project\n  cueloop plugin uninstall my.plugin --scope project"
    )]
    Plugin(plugin::PluginArgs),

    /// Runner management commands (capabilities, list).
    #[command(
        hide = true,
        after_long_help = "Examples:\n  cueloop runner capabilities codex\n  cueloop runner capabilities claude --format json\n  cueloop runner list\n  cueloop runner list --format json"
    )]
    Runner(runner::RunnerArgs),

    /// Run the interactive CueLoop onboarding tutorial.
    #[command(
        hide = true,
        after_long_help = "Examples:\n  cueloop tutorial\n  cueloop tutorial --keep-sandbox\n  cueloop tutorial --non-interactive"
    )]
    Tutorial(tutorial::TutorialArgs),

    /// Restore or preview an earlier continuation checkpoint.
    #[command(
        after_long_help = "Continuation workflow:\n  - `cueloop undo --list` shows the checkpoints CueLoop created before queue-changing operations.\n  - `cueloop undo --dry-run` previews the restore path without modifying queue files.\n  - `cueloop undo` restores the most recent checkpoint; `--id` restores a specific one.\n  - After restoring, run `cueloop queue validate` and then continue normal work.\n\nExamples:\n  cueloop undo\n  cueloop undo --list\n  cueloop undo --dry-run\n  cueloop undo --id undo-20260215073000000000\n\nCheckpoints are created automatically before queue mutations such as:\n  - cueloop task mutate / task decompose --write\n  - cueloop task done/reject/start/ready/schedule\n  - cueloop task edit/field/clone/split\n  - cueloop task relate/blocks/mark-duplicate\n  - cueloop queue archive/prune/sort/import/repair\n  - cueloop queue issue publish/publish-many\n  - cueloop task batch operations"
    )]
    Undo(undo::UndoArgs),

    /// Emit a machine-readable CLI specification (JSON) for tooling and legacy clients.
    #[command(name = "cli-spec", alias = "__cli-spec", hide = true)]
    CliSpec(CliSpecArgs),
}

#[derive(Args, Debug, Clone)]
pub struct CliSpecArgs {
    /// Output format.
    #[arg(long, value_enum, default_value_t = CliSpecFormatArg::Json)]
    pub format: CliSpecFormatArg,
}

#[derive(ValueEnum, Debug, Copy, Clone, PartialEq, Eq)]
pub enum CliSpecFormatArg {
    Json,
}
