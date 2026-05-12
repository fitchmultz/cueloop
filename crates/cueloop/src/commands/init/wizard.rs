//! Interactive onboarding wizard for CueLoop initialization.
//!
//! Purpose:
//! - Interactive onboarding wizard for CueLoop initialization.
//!
//! Responsibilities:
//! - Display welcome screen and collect user preferences.
//! - Guide users through runner, model, phase, and queue-tracking selection.
//! - Let users opt into explicit ignored-file sync entries for parallel workers.
//! - Optionally enable the CI gate and capture argv for the configured check command.
//! - Optionally create a first task during setup.
//!
//! Not handled here:
//! - File creation (see `super::writers`).
//! - CLI argument parsing (handled by CLI layer).
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Wizard is only run in interactive TTY environments.
//! - User inputs are validated before returning WizardAnswers.

use crate::commands::init::parallel_sync;
use crate::config::{self, CiGateArgvIssue, detect_ci_gate_argv_issue};
use crate::contracts::{Runner, TaskPriority};
use anyhow::{Context, Result};
use dialoguer::{Confirm, Input, MultiSelect, Select};
use std::path::Path;

/// Queue/done tracking mode selected during interactive initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueTrackingMode {
    /// Keep queue/done files trackable for shared team task state.
    TrackedShared,
    /// Gitignore queue/done files for local-only task state.
    LocalIgnored,
}

/// Answers collected from the interactive wizard.
#[derive(Debug, Clone)]
pub struct WizardAnswers {
    /// Selected AI runner.
    pub runner: Runner,
    /// Selected model (as string for flexibility).
    pub model: String,
    /// Number of phases (1, 2, or 3).
    pub phases: u8,
    /// Queue/done tracking choice.
    pub queue_tracking_mode: QueueTrackingMode,
    /// Explicit ignored local files selected for parallel worker sync.
    pub parallel_ignored_file_allowlist: Vec<String>,
    /// Whether to run the configured CI gate before completing tasks.
    pub ci_gate_enabled: bool,
    /// `argv` for [`crate::contracts::CiGateConfig`] when `ci_gate_enabled` is true.
    pub ci_gate_argv: Option<Vec<String>>,
    /// Whether to create a first task.
    pub create_first_task: bool,
    /// Title for the first task (if created).
    pub first_task_title: Option<String>,
    /// Description/request for the first task (if created).
    pub first_task_description: Option<String>,
    /// Priority for the first task.
    pub first_task_priority: TaskPriority,
}

impl Default for WizardAnswers {
    fn default() -> Self {
        Self {
            runner: Runner::Claude,
            model: "sonnet".to_string(),
            phases: 3,
            queue_tracking_mode: QueueTrackingMode::TrackedShared,
            parallel_ignored_file_allowlist: Vec::new(),
            ci_gate_enabled: false,
            ci_gate_argv: None,
            create_first_task: false,
            first_task_title: None,
            first_task_description: None,
            first_task_priority: TaskPriority::Medium,
        }
    }
}

/// Run the interactive onboarding wizard and collect user preferences.
pub fn run_wizard(repo_root: &Path) -> Result<WizardAnswers> {
    // Welcome screen
    print_welcome();

    // Runner selection
    let runners = [
        (
            "Claude",
            "Anthropic's Claude Code CLI - Best for complex reasoning",
        ),
        ("Codex", "OpenAI's Codex CLI - Great for code generation"),
        ("OpenCode", "OpenCode agent - Open source alternative"),
        (
            "Gemini",
            "Google's Gemini CLI - Good for large context windows",
        ),
        ("Cursor", "Cursor's agent mode - IDE-integrated workflow"),
        ("Kimi", "Moonshot AI Kimi - Strong coding capabilities"),
        ("Pi", "Inflection Pi - Conversational AI assistant"),
    ];

    let runner_idx = Select::new()
        .with_prompt("Select your AI runner")
        .items(
            runners
                .iter()
                .map(|(name, desc)| format!("{} - {}", name, desc))
                .collect::<Vec<_>>(),
        )
        .default(0)
        .interact()
        .context("failed to get runner selection")?;

    let runner = match runner_idx {
        0 => Runner::Claude,
        1 => Runner::Codex,
        2 => Runner::Opencode,
        3 => Runner::Gemini,
        4 => Runner::Cursor,
        5 => Runner::Kimi,
        6 => Runner::Pi,
        _ => Runner::Claude, // default fallback
    };

    // Model selection based on runner
    let model = select_model(&runner)?;

    // Phase selection
    let phases = select_phases()?;

    // Queue and parallel-worker setup choices
    let queue_tracking_mode = select_queue_tracking_mode(repo_root)?;
    let parallel_ignored_file_allowlist = select_parallel_sync_allowlist(repo_root)?;

    let (ci_gate_enabled, ci_gate_argv) = select_ci_gate()?;

    // First task creation
    let create_first_task = Confirm::new()
        .with_prompt("Would you like to create your first task now?")
        .default(true)
        .interact()
        .context("failed to get first task confirmation")?;

    let (first_task_title, first_task_description, first_task_priority) = if create_first_task {
        let title: String = Input::new()
            .with_prompt("Task title")
            .allow_empty(false)
            .interact_text()
            .context("failed to get task title")?;

        let description: String = Input::new()
            .with_prompt("Task description (what should be done)")
            .allow_empty(true)
            .interact_text()
            .context("failed to get task description")?;

        let priorities = vec!["Low", "Medium", "High", "Critical"];
        let priority_idx = Select::new()
            .with_prompt("Task priority")
            .items(&priorities)
            .default(1)
            .interact()
            .context("failed to get priority selection")?;

        let priority = match priority_idx {
            0 => TaskPriority::Low,
            1 => TaskPriority::Medium,
            2 => TaskPriority::High,
            3 => TaskPriority::Critical,
            _ => TaskPriority::Medium,
        };

        (Some(title), Some(description), priority)
    } else {
        (None, None, TaskPriority::Medium)
    };

    // Summary and confirmation
    let answers = WizardAnswers {
        runner,
        model,
        phases,
        queue_tracking_mode,
        parallel_ignored_file_allowlist,
        ci_gate_enabled,
        ci_gate_argv,
        create_first_task,
        first_task_title,
        first_task_description,
        first_task_priority,
    };

    print_summary(repo_root, &answers);

    let proceed = Confirm::new()
        .with_prompt("Proceed with setup?")
        .default(true)
        .interact()
        .context("failed to get confirmation")?;

    if !proceed {
        anyhow::bail!("Setup cancelled by user");
    }

    Ok(answers)
}

/// Print the welcome screen with ASCII art.
fn print_welcome() {
    println!();
    println!("{}", colored::Colorize::bright_cyan("CueLoop"));
    println!("{}", colored::Colorize::bright_black("───────"));
    println!();
    println!("{}", colored::Colorize::bold("Welcome to CueLoop!"));
    println!();
    println!("CueLoop is an AI task queue for structured agent workflows.");
    println!("This wizard will help you set up your project and create your first task.");
    println!();
}

/// Select model based on the chosen runner.
fn select_model(runner: &Runner) -> Result<String> {
    let models: Vec<(&str, &str)> = match runner {
        Runner::Claude => vec![
            ("sonnet", "Balanced speed and intelligence (recommended)"),
            ("opus", "Most powerful, best for complex tasks"),
            ("haiku", "Fastest, good for simple tasks"),
            ("custom", "Other model (specify)"),
        ],
        Runner::Codex => vec![
            (
                "gpt-5.4",
                "Latest general GPT-5 model for Codex (recommended)",
            ),
            ("gpt-5.3-codex", "Codex optimized for coding"),
            ("gpt-5.3-codex-spark", "Codex Spark variant for coding"),
            ("gpt-5.3", "General GPT-5.3"),
            ("custom", "Other model (specify)"),
        ],
        Runner::Gemini => vec![
            (
                "zai-coding-plan/glm-4.7",
                "Default Gemini model (recommended)",
            ),
            ("custom", "Other model (specify)"),
        ],
        Runner::Opencode => vec![
            ("zai-coding-plan/glm-4.7", "GLM-4.7 model (recommended)"),
            ("custom", "Other model (specify)"),
        ],
        Runner::Kimi => vec![
            ("kimi-for-coding", "Kimi coding model (recommended)"),
            ("custom", "Other model (specify)"),
        ],
        Runner::Pi => vec![
            (
                "openai-codex/gpt-5.4",
                "Strong Codex model for implementation-heavy workflows (recommended)",
            ),
            (
                "openai-codex/gpt-5.5",
                "Premium Codex model for planning/review",
            ),
            ("gpt-5.4", "General GPT-5.4"),
            ("custom", "Other model (specify)"),
        ],
        Runner::Cursor => vec![
            ("auto", "Let Cursor choose automatically (recommended)"),
            ("custom", "Other model (specify)"),
        ],
        Runner::Plugin(_) => vec![
            ("default", "Use runner default"),
            ("custom", "Specify custom model"),
        ],
    };

    let items: Vec<String> = models
        .iter()
        .map(|(name, desc)| format!("{} - {}", name, desc))
        .collect();

    let idx = Select::new()
        .with_prompt("Select model")
        .items(&items)
        .default(0)
        .interact()
        .context("failed to get model selection")?;

    let selected = models[idx].0;

    if selected == "custom" {
        let custom: String = Input::new()
            .with_prompt("Enter model name")
            .allow_empty(false)
            .interact_text()
            .context("failed to get custom model")?;
        Ok(custom)
    } else {
        Ok(selected.to_string())
    }
}

/// Select the number of phases with explanations.
fn select_phases() -> Result<u8> {
    let phase_options = [
        (
            "3-phase (Full)",
            "Plan → Implement + CI → Review + Complete [Recommended]",
        ),
        (
            "2-phase (Standard)",
            "Plan → Implement (faster, less review)",
        ),
        (
            "1-phase (Quick)",
            "Single-pass execution (simple fixes only)",
        ),
    ];

    let items: Vec<String> = phase_options
        .iter()
        .map(|(name, desc)| format!("{} - {}", name, desc))
        .collect();

    let idx = Select::new()
        .with_prompt("Select workflow mode")
        .items(&items)
        .default(0)
        .interact()
        .context("failed to get phase selection")?;

    Ok(match idx {
        0 => 3,
        1 => 2,
        2 => 1,
        _ => 3,
    })
}

/// Ask whether to enable the CI gate and collect argv when enabled.
fn select_ci_gate() -> Result<(bool, Option<Vec<String>>)> {
    println!();
    println!(
        "{}",
        colored::Colorize::bright_black(
            "CI gate: CueLoop can run a single argv-only command before completing tasks (no shell; use a wrapper script for complex pipelines)."
        )
    );

    let enable = Confirm::new()
        .with_prompt("Enable CI gate checks before task completion?")
        .default(false)
        .interact()
        .context("failed to get CI gate confirmation")?;

    if !enable {
        return Ok((false, None));
    }

    loop {
        let line: String = Input::new()
            .with_prompt("CI command (program and arguments, e.g. make ci or npm test)")
            .default("make ci".to_string())
            .allow_empty(false)
            .interact_text()
            .context("failed to read CI command")?;

        let argv = match shlex::split(&line) {
            Some(parts) if !parts.is_empty() => parts,
            _ => {
                eprintln!(
                    "Could not parse that into argv tokens. Use a direct command such as `make ci` or `./scripts/ci.sh`."
                );
                continue;
            }
        };

        if let Some(issue) = detect_ci_gate_argv_issue(&argv) {
            let hint = match issue {
                CiGateArgvIssue::EmptyArgv => "Enter at least one token (the program name).",
                CiGateArgvIssue::EmptyEntry => "Argv entries must not be empty or whitespace-only.",
                CiGateArgvIssue::ShellLauncher => {
                    "Shell wrappers like `sh -c` are not supported; point argv at a script instead."
                }
            };
            eprintln!(
                "CI argv rejected ({hint}). See docs/configuration/agent-and-runners.md for agent.ci_gate."
            );
            continue;
        }

        return Ok((true, Some(argv)));
    }
}

fn runtime_dir_name(repo_root: &Path) -> String {
    config::project_runtime_dir(repo_root)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(crate::constants::identity::PROJECT_RUNTIME_DIR)
        .to_string()
}

/// Select how queue/done files should be tracked.
fn select_queue_tracking_mode(repo_root: &Path) -> Result<QueueTrackingMode> {
    let runtime_name = runtime_dir_name(repo_root);
    let tracked_desc = format!(
        "Commit {runtime_name}/queue.jsonc and {runtime_name}/done.jsonc so the team shares task state [Recommended]"
    );
    let options = [
        ("Tracked shared queue", tracked_desc.as_str()),
        (
            "Local private queue",
            "Gitignore queue/done so task state stays local to this checkout",
        ),
    ];
    let items = options
        .iter()
        .map(|(name, desc)| format!("{name} - {desc}"))
        .collect::<Vec<_>>();
    let idx = Select::new()
        .with_prompt("How should CueLoop queue files be tracked?")
        .items(&items)
        .default(0)
        .interact()
        .context("failed to get queue tracking selection")?;
    Ok(if idx == 1 {
        QueueTrackingMode::LocalIgnored
    } else {
        QueueTrackingMode::TrackedShared
    })
}

/// Let the user select extra ignored local files for parallel-worker sync.
fn select_parallel_sync_allowlist(repo_root: &Path) -> Result<Vec<String>> {
    let candidates = parallel_sync::discover_parallel_sync_candidates(repo_root)
        .context("discover parallel ignored-file sync candidates")?;
    if candidates.is_empty() {
        println!();
        println!(
            "Parallel sync: no extra ignored local files found. .env and .env.* are synced by default."
        );
        println!(
            "To sync another small ignored file later, set trusted parallel.ignored_file_allowlist in the active runtime config (for example \"local/tool-config.json\"). Directory trees such as \"node_modules/*\" or entries ending in \"/\" are rejected. See docs/configuration/queue-and-parallel.md#ignored-local-file-sync."
        );
        return Ok(Vec::new());
    }

    println!();
    println!(
        "Parallel sync: .env and .env.* are synced by default. Select any additional small ignored files workers need."
    );
    println!(
        "Manual trusted config key: parallel.ignored_file_allowlist (valid: \"local/tool-config.json\"; invalid: \"node_modules/*\" or \"dir/\"). See docs/configuration/queue-and-parallel.md#ignored-local-file-sync."
    );
    let selections = MultiSelect::new()
        .with_prompt("Additional ignored files to sync to parallel workers")
        .items(&candidates)
        .defaults(&vec![true; candidates.len()])
        .interact()
        .context("failed to get parallel ignored-file sync selection")?;

    Ok(selections
        .into_iter()
        .filter_map(|idx| candidates.get(idx).cloned())
        .collect())
}

/// Print a summary of the wizard answers.
fn print_summary(repo_root: &Path, answers: &WizardAnswers) {
    let runtime_name = runtime_dir_name(repo_root);
    println!();
    println!("{}", colored::Colorize::bold("Setup Summary:"));
    println!("{}", colored::Colorize::bright_black("──────────────"));
    println!(
        "Runner: {} ({})",
        colored::Colorize::bright_green(format!("{:?}", answers.runner).as_str()),
        answers.model
    );
    println!(
        "Workflow: {}-phase",
        colored::Colorize::bright_green(format!("{}", answers.phases).as_str())
    );

    println!(
        "Queue mode: {}",
        match answers.queue_tracking_mode {
            QueueTrackingMode::TrackedShared => colored::Colorize::bright_green("tracked shared"),
            QueueTrackingMode::LocalIgnored => colored::Colorize::bright_yellow("local ignored"),
        }
    );
    if answers.parallel_ignored_file_allowlist.is_empty() {
        println!(
            "Parallel sync extras: {}",
            colored::Colorize::bright_black("(none)")
        );
    } else {
        println!(
            "Parallel sync extras: {}",
            colored::Colorize::bright_green(
                answers.parallel_ignored_file_allowlist.join(", ").as_str()
            )
        );
    }

    if answers.ci_gate_enabled {
        let cmd = answers
            .ci_gate_argv
            .as_ref()
            .map(|argv| argv.join(" "))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "(unset)".to_string());
        println!("CI gate: {}", colored::Colorize::bright_green(cmd.as_str()));
    } else {
        println!("CI gate: {}", colored::Colorize::bright_black("disabled"));
    }

    if answers.create_first_task {
        if let Some(ref title) = answers.first_task_title {
            println!(
                "First Task: {}",
                colored::Colorize::bright_green(title.as_str())
            );
        }
    } else {
        println!("First Task: {}", colored::Colorize::bright_black("(none)"));
    }

    println!();
    println!("Files to create:");
    println!("  - {runtime_name}/config.jsonc");
    println!("  - {runtime_name}/queue.jsonc");
    println!("  - {runtime_name}/done.jsonc");
    println!();
}

/// Print completion message with next steps.
pub fn print_completion_message(answers: Option<&WizardAnswers>, queue_path: &Path) {
    let runtime_name = queue_path
        .parent()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or(crate::constants::identity::PROJECT_RUNTIME_DIR);
    println!();
    println!(
        "{}",
        colored::Colorize::bright_green("✓ CueLoop initialized successfully!")
    );
    println!();
    println!("{}", colored::Colorize::bold("Next steps:"));
    println!("  1. Run 'cueloop app open' to open the macOS app (optional)");
    println!("  2. Run 'cueloop run one' to execute your first task");
    println!("  3. Edit {runtime_name}/config.jsonc to customize settings");
    println!(
        "     - Parallel sync extras use trusted parallel.ignored_file_allowlist (valid: \"local/tool-config.json\"; invalid: \"node_modules/*\"); see docs/configuration/queue-and-parallel.md#ignored-local-file-sync"
    );
    println!("  4. Keep {runtime_name}/trust.jsonc untracked; init adds it to .gitignore");

    if let Some(answers) = answers
        && answers.create_first_task
    {
        println!();
        println!("Your first task is ready to go!");
    }

    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wizard_answers_default() {
        let answers = WizardAnswers::default();
        assert_eq!(answers.runner, Runner::Claude);
        assert_eq!(answers.model, "sonnet");
        assert_eq!(answers.phases, 3);
        assert_eq!(
            answers.queue_tracking_mode,
            QueueTrackingMode::TrackedShared
        );
        assert!(answers.parallel_ignored_file_allowlist.is_empty());
        assert!(!answers.ci_gate_enabled);
        assert!(answers.ci_gate_argv.is_none());
        assert!(!answers.create_first_task);
        assert!(answers.first_task_title.is_none());
        assert!(answers.first_task_description.is_none());
        assert_eq!(answers.first_task_priority, TaskPriority::Medium);
    }
}
