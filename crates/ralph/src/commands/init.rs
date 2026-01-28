//! Initialization workflow for creating `.ralph` state and starter files.
//!
//! Responsibilities:
//! - Create initial `.ralph/` directory structure and files.
//! - Run interactive onboarding wizard when requested.
//! - Generate config and queue with user-specified or default values.
//!
//! Not handled here:
//! - CLI argument parsing (see `crate::cli::init`).
//! - TTY detection (handled by CLI layer).
//!
//! Invariants/assumptions:
//! - Wizard answers are validated before file creation.
//! - Non-interactive mode produces identical output to pre-wizard behavior.

use crate::config;
use crate::contracts::{Config, QueueFile, Runner, Task, TaskPriority, TaskStatus};
use crate::fsutil;
use crate::prompts;
use crate::queue;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

const DEFAULT_RALPH_README: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/ralph_readme.md"
));

/// Current version of the embedded README template.
/// Increment this when updating the template to trigger update detection.
pub const README_VERSION: u32 = 3;

/// Options for initializing Ralph files.
pub struct InitOptions {
    /// Overwrite existing files if they already exist.
    pub force: bool,
    /// Force remove stale locks.
    pub force_lock: bool,
    /// Run interactive onboarding wizard.
    pub interactive: bool,
    /// Update README if it exists (force overwrite with latest template).
    pub update_readme: bool,
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
            create_first_task: false,
            first_task_title: None,
            first_task_description: None,
            first_task_priority: TaskPriority::Medium,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileInitStatus {
    Created,
    Valid,
    Updated,
}

/// Result of checking if README is current.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadmeCheckResult {
    /// README is current with the specified version.
    Current(u32),
    /// README is outdated (has older version).
    Outdated {
        current_version: u32,
        embedded_version: u32,
    },
    /// README is missing.
    Missing,
    /// README not applicable (prompts don't reference it).
    NotApplicable,
}

#[derive(Debug)]
pub struct InitReport {
    pub queue_status: FileInitStatus,
    pub done_status: FileInitStatus,
    pub config_status: FileInitStatus,
    /// (status, version) tuple - version is Some if README was read/created
    pub readme_status: Option<(FileInitStatus, Option<u32>)>,
}

pub fn run_init(resolved: &config::Resolved, opts: InitOptions) -> Result<InitReport> {
    let ralph_dir = resolved.repo_root.join(".ralph");
    fs::create_dir_all(&ralph_dir).with_context(|| format!("create {}", ralph_dir.display()))?;

    let _queue_lock = queue::acquire_queue_lock(&resolved.repo_root, "init", opts.force_lock)?;

    // Run wizard if interactive mode is enabled
    let wizard_answers = if opts.interactive {
        Some(run_wizard()?)
    } else {
        None
    };

    let queue_status = write_queue(
        &resolved.queue_path,
        opts.force,
        &resolved.id_prefix,
        resolved.id_width,
        wizard_answers.as_ref(),
    )?;
    let done_status = write_done(
        &resolved.done_path,
        opts.force,
        &resolved.id_prefix,
        resolved.id_width,
    )?;
    let config_path = resolved
        .project_config_path
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("project config path unavailable"))?;
    let config_status = write_config(config_path, opts.force, wizard_answers.as_ref())?;

    let mut readme_status = None;
    if prompts::prompts_reference_readme(&resolved.repo_root)? {
        let readme_path = resolved.repo_root.join(".ralph/README.md");
        let (status, version) = write_readme(&readme_path, opts.force, opts.update_readme)?;
        readme_status = Some((status, version));
    }

    // Print completion message for interactive mode
    if opts.interactive {
        print_completion_message(wizard_answers.as_ref(), &resolved.queue_path);
    }

    Ok(InitReport {
        queue_status,
        done_status,
        config_status,
        readme_status,
    })
}

/// Run the interactive onboarding wizard and collect user preferences.
fn run_wizard() -> Result<WizardAnswers> {
    use dialoguer::{Confirm, Input, Select};

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
    ];

    let runner_idx = Select::new()
        .with_prompt("Select your AI runner")
        .items(
            &runners
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
        _ => Runner::Claude, // default fallback
    };

    // Model selection based on runner
    let model = select_model(runner)?;

    // Phase selection
    let phases = select_phases()?;

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
        create_first_task,
        first_task_title,
        first_task_description,
        first_task_priority,
    };

    print_summary(&answers);

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
    println!(
        "{}",
        colored::Colorize::bright_cyan(r"    ____       __        __")
    );
    println!(
        "{}",
        colored::Colorize::bright_cyan(r"   / __ \___  / /_____  / /_____ ___")
    );
    println!(
        "{}",
        colored::Colorize::bright_cyan(r"  / /_/ / _ \/ __/ __ \/ __/ __ `__ \ ")
    );
    println!(
        "{}",
        colored::Colorize::bright_cyan(r" / _, _/  __/ /_/ /_/ / /_/ / / / / /")
    );
    println!(
        "{}",
        colored::Colorize::bright_cyan(r"/_/ |_|\___/\__/ .___/\__/_/ /_/ /_/")
    );
    println!("{}", colored::Colorize::bright_cyan(r"             /_/"));
    println!();
    println!("{}", colored::Colorize::bold("Welcome to Ralph!"));
    println!();
    println!("Ralph is an AI task queue for structured agent workflows.");
    println!("This wizard will help you set up your project and create your first task.");
    println!();
}

/// Select model based on the chosen runner.
fn select_model(runner: Runner) -> Result<String> {
    use dialoguer::{Input, Select};

    let models: Vec<(&str, &str)> = match runner {
        Runner::Claude => vec![
            ("sonnet", "Balanced speed and intelligence (recommended)"),
            ("opus", "Most powerful, best for complex tasks"),
            ("haiku", "Fastest, good for simple tasks"),
            ("custom", "Other model (specify)"),
        ],
        Runner::Codex => vec![
            ("gpt-5.2-codex", "Codex optimized for coding (recommended)"),
            ("gpt-5.2", "General GPT-5.2"),
            ("custom", "Other model (specify)"),
        ],
        Runner::Gemini => vec![
            (
                "zai-coding-plan/glm-4.7",
                "Default Gemini model (recommended)",
            ),
            ("custom", "Other model (specify)"),
        ],
        _ => vec![
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
    use dialoguer::Select;

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

/// Print a summary of the wizard answers.
fn print_summary(answers: &WizardAnswers) {
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
    println!("  - .ralph/config.json");
    println!("  - .ralph/queue.json");
    println!("  - .ralph/done.json");
    println!();
}

/// Print completion message with next steps.
fn print_completion_message(answers: Option<&WizardAnswers>, _queue_path: &Path) {
    println!();
    println!(
        "{}",
        colored::Colorize::bright_green("✓ Ralph initialized successfully!")
    );
    println!();
    println!("{}", colored::Colorize::bold("Next steps:"));
    println!("  1. Run 'ralph tui' to launch the interactive UI");
    println!("  2. Run 'ralph run one' to execute your first task");
    println!("  3. Edit .ralph/config.json to customize settings");

    if let Some(answers) = answers {
        if answers.create_first_task {
            println!();
            println!("Your first task is ready to go!");
        }
    }

    println!();
}

fn write_queue(
    path: &Path,
    force: bool,
    id_prefix: &str,
    id_width: usize,
    wizard_answers: Option<&WizardAnswers>,
) -> Result<FileInitStatus> {
    if path.exists() && !force {
        // Validate existing file by trying to load it
        let queue = queue::load_queue(path)?;
        queue::validate_queue(&queue, id_prefix, id_width)
            .with_context(|| format!("validate existing queue {}", path.display()))?;
        return Ok(FileInitStatus::Valid);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }

    let mut queue = QueueFile::default();

    // Add first task if wizard provided one
    if let Some(answers) = wizard_answers {
        if answers.create_first_task {
            if let (Some(title), Some(description)) = (
                answers.first_task_title.clone(),
                answers.first_task_description.clone(),
            ) {
                let now = time::OffsetDateTime::now_utc();
                let timestamp = now
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_else(|_| now.to_string());

                let task_id = format!("{}-{:0>width$}", id_prefix, 1, width = id_width);

                let task = Task {
                    id: task_id,
                    status: TaskStatus::Todo,
                    title,
                    priority: answers.first_task_priority,
                    tags: vec!["onboarding".to_string()],
                    scope: vec![],
                    evidence: vec![],
                    plan: vec![],
                    notes: vec![],
                    request: Some(description),
                    agent: None,
                    created_at: Some(timestamp.clone()),
                    updated_at: Some(timestamp),
                    completed_at: None,
                    depends_on: vec![],
                    custom_fields: std::collections::HashMap::new(),
                };

                queue.tasks.push(task);
            }
        }
    }

    let rendered = serde_json::to_string_pretty(&queue).context("serialize queue JSON")?;
    fsutil::write_atomic(path, rendered.as_bytes())
        .with_context(|| format!("write queue JSON {}", path.display()))?;
    Ok(FileInitStatus::Created)
}

fn write_done(
    path: &Path,
    force: bool,
    id_prefix: &str,
    id_width: usize,
) -> Result<FileInitStatus> {
    if path.exists() && !force {
        // Validate existing file by trying to load it
        let queue = queue::load_queue(path)?;
        queue::validate_queue(&queue, id_prefix, id_width)
            .with_context(|| format!("validate existing done {}", path.display()))?;
        return Ok(FileInitStatus::Valid);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let queue = QueueFile::default();
    let rendered = serde_json::to_string_pretty(&queue).context("serialize done JSON")?;
    fsutil::write_atomic(path, rendered.as_bytes())
        .with_context(|| format!("write done JSON {}", path.display()))?;
    Ok(FileInitStatus::Created)
}

fn write_config(
    path: &Path,
    force: bool,
    wizard_answers: Option<&WizardAnswers>,
) -> Result<FileInitStatus> {
    if path.exists() && !force {
        // Validate existing config by trying to parse it
        let raw =
            fs::read_to_string(path).with_context(|| format!("read config {}", path.display()))?;
        serde_json::from_str::<Config>(&raw).with_context(|| {
            format!(
                "Config file exists but is invalid JSON: {}. Use --force to overwrite.",
                path.display()
            )
        })?;
        return Ok(FileInitStatus::Valid);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }

    // Build config with wizard answers or defaults
    let config_json = if let Some(answers) = wizard_answers {
        let runner_str = format!("{:?}", answers.runner).to_lowercase();
        let model_str = if answers.model.contains("/") || answers.model.len() > 20 {
            // Custom model string
            answers.model.clone()
        } else {
            answers.model.clone()
        };

        serde_json::json!({
            "version": 1,
            "agent": {
                "runner": runner_str,
                "model": model_str,
                "phases": answers.phases
            }
        })
    } else {
        serde_json::json!({ "version": 1 })
    };

    let rendered = serde_json::to_string_pretty(&config_json).context("serialize config JSON")?;
    fsutil::write_atomic(path, rendered.as_bytes())
        .with_context(|| format!("write config JSON {}", path.display()))?;
    Ok(FileInitStatus::Created)
}

/// Extract version from README content.
/// Looks for `<!-- RALPH_README_VERSION: X -->` marker.
fn extract_readme_version(content: &str) -> Option<u32> {
    // Look for version marker: <!-- RALPH_README_VERSION: X -->
    let marker_start = "<!-- RALPH_README_VERSION:";
    if let Some(start_idx) = content.find(marker_start) {
        let after_marker = &content[start_idx + marker_start.len()..];
        if let Some(end_idx) = after_marker.find("-->") {
            let version_str = &after_marker[..end_idx];
            return version_str.trim().parse::<u32>().ok();
        }
    }
    // Legacy: no version marker means version 1
    Some(1)
}

/// Check if README is current without modifying it.
/// Returns the check result with version information.
pub fn check_readme_current(resolved: &config::Resolved) -> Result<ReadmeCheckResult> {
    // First check if README is applicable
    if !prompts::prompts_reference_readme(&resolved.repo_root)? {
        return Ok(ReadmeCheckResult::NotApplicable);
    }

    let readme_path = resolved.repo_root.join(".ralph/README.md");

    if !readme_path.exists() {
        return Ok(ReadmeCheckResult::Missing);
    }

    let content = fs::read_to_string(&readme_path)
        .with_context(|| format!("read {}", readme_path.display()))?;

    let current_version = extract_readme_version(&content).unwrap_or(1);

    if current_version >= README_VERSION {
        Ok(ReadmeCheckResult::Current(current_version))
    } else {
        Ok(ReadmeCheckResult::Outdated {
            current_version,
            embedded_version: README_VERSION,
        })
    }
}

fn write_readme(path: &Path, force: bool, update: bool) -> Result<(FileInitStatus, Option<u32>)> {
    if path.exists() && !force && !update {
        // Check version for reporting purposes
        let content =
            fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let version = extract_readme_version(&content);
        return Ok((FileInitStatus::Valid, version));
    }

    // Check if we need to update (version mismatch)
    let should_update = if update && path.exists() && !force {
        let content =
            fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let current_version = extract_readme_version(&content).unwrap_or(1);
        current_version < README_VERSION
    } else {
        true // Create new or force overwrite
    };

    if !should_update {
        // Version is current, no update needed
        let content =
            fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let version = extract_readme_version(&content);
        return Ok((FileInitStatus::Valid, version));
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }

    let is_update = path.exists();
    fsutil::write_atomic(path, DEFAULT_RALPH_README.as_bytes())
        .with_context(|| format!("write readme {}", path.display()))?;

    if is_update {
        Ok((FileInitStatus::Updated, Some(README_VERSION)))
    } else {
        Ok((FileInitStatus::Created, Some(README_VERSION)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::ProjectType;
    use tempfile::TempDir;

    fn resolved_for(dir: &TempDir) -> config::Resolved {
        let repo_root = dir.path().to_path_buf();
        let queue_path = repo_root.join(".ralph/queue.json");
        let done_path = repo_root.join(".ralph/done.json");
        let project_config_path = Some(repo_root.join(".ralph/config.json"));
        config::Resolved {
            config: Config::default(),
            repo_root,
            queue_path,
            done_path,
            id_prefix: "RQ".to_string(),
            id_width: 4,
            global_config_path: None,
            project_config_path,
        }
    }

    #[test]
    fn init_creates_missing_files() -> Result<()> {
        let dir = TempDir::new()?;
        let resolved = resolved_for(&dir);
        let report = run_init(
            &resolved,
            InitOptions {
                force: false,
                force_lock: false,
                interactive: false,
                update_readme: false,
            },
        )?;
        assert_eq!(report.queue_status, FileInitStatus::Created);
        assert_eq!(report.done_status, FileInitStatus::Created);
        assert_eq!(report.config_status, FileInitStatus::Created);
        assert!(matches!(
            report.readme_status,
            Some((FileInitStatus::Created, Some(3)))
        ));
        let queue = crate::queue::load_queue(&resolved.queue_path)?;
        assert_eq!(queue.version, 1);
        let done = crate::queue::load_queue(&resolved.done_path)?;
        assert_eq!(done.version, 1);
        let raw_cfg = std::fs::read_to_string(resolved.project_config_path.as_ref().unwrap())?;
        let cfg: Config = serde_json::from_str(&raw_cfg)?;
        assert_eq!(cfg.version, 1);
        let readme_path = resolved.repo_root.join(".ralph/README.md");
        assert!(readme_path.exists());
        let readme_raw = std::fs::read_to_string(readme_path)?;
        assert!(readme_raw.contains("# Ralph runtime files"));
        Ok(())
    }

    #[test]
    fn init_generates_readme_with_correct_archive_command() -> Result<()> {
        let dir = TempDir::new()?;
        let resolved = resolved_for(&dir);
        run_init(
            &resolved,
            InitOptions {
                force: false,
                force_lock: false,
                interactive: false,
                update_readme: false,
            },
        )?;
        let readme_path = resolved.repo_root.join(".ralph/README.md");
        let readme_raw = std::fs::read_to_string(readme_path)?;
        // Verify the correct command is present
        assert!(
            readme_raw.contains("ralph queue archive"),
            "README should contain 'ralph queue archive' command"
        );
        // Verify the stale command is NOT present (regression check)
        assert!(
            !readme_raw.contains("ralph queue done"),
            "README should NOT contain stale 'ralph queue done' command"
        );
        Ok(())
    }

    #[test]
    fn init_skips_existing_when_not_forced() -> Result<()> {
        let dir = TempDir::new()?;
        let resolved = resolved_for(&dir);
        std::fs::create_dir_all(resolved.repo_root.join(".ralph"))?;
        let queue_json = r#"{
  "version": 1,
  "tasks": [
    {
      "id": "RQ-0001",
      "status": "todo",
      "title": "Keep",
      "tags": ["code"],
      "scope": ["x"],
      "evidence": ["y"],
      "plan": ["z"],
      "request": "test",
      "created_at": "2026-01-18T00:00:00Z",
      "updated_at": "2026-01-18T00:00:00Z"
    }
  ]
}"#;
        std::fs::write(&resolved.queue_path, queue_json)?;
        let done_json = r#"{
  "version": 1,
  "tasks": [
    {
      "id": "RQ-0002",
      "status": "done",
      "title": "Done",
      "tags": ["code"],
      "scope": ["x"],
      "evidence": ["y"],
      "plan": ["z"],
      "request": "test",
      "created_at": "2026-01-18T00:00:00Z",
      "updated_at": "2026-01-18T00:00:00Z",
      "completed_at": "2026-01-18T00:00:00Z"
    }
  ]
}"#;
        std::fs::write(&resolved.done_path, done_json)?;
        let config_json = r#"{
  "version": 1,
  "queue": {
    "file": ".ralph/queue.json"
  }
}"#;
        std::fs::write(resolved.project_config_path.as_ref().unwrap(), config_json)?;
        let report = run_init(
            &resolved,
            InitOptions {
                force: false,
                force_lock: false,
                interactive: false,
                update_readme: false,
            },
        )?;
        assert_eq!(report.queue_status, FileInitStatus::Valid);
        assert_eq!(report.done_status, FileInitStatus::Valid);
        assert_eq!(report.config_status, FileInitStatus::Valid);
        assert!(matches!(
            report.readme_status,
            Some((FileInitStatus::Created, Some(3)))
        ));
        let raw = std::fs::read_to_string(&resolved.queue_path)?;
        assert!(raw.contains("Keep"));
        let done_raw = std::fs::read_to_string(&resolved.done_path)?;
        assert!(done_raw.contains("Done"));
        Ok(())
    }

    #[test]
    fn init_overwrites_when_forced() -> Result<()> {
        let dir = TempDir::new()?;
        let resolved = resolved_for(&dir);
        std::fs::create_dir_all(resolved.repo_root.join(".ralph"))?;
        std::fs::write(&resolved.queue_path, r#"{"version":1,"tasks":[]}"#)?;
        std::fs::write(&resolved.done_path, r#"{"version":1,"tasks":[]}"#)?;
        std::fs::write(
            resolved.project_config_path.as_ref().unwrap(),
            r#"{"version":1,"project_type":"docs"}"#,
        )?;
        let report = run_init(
            &resolved,
            InitOptions {
                force: true,
                force_lock: false,
                interactive: false,
                update_readme: false,
            },
        )?;
        assert_eq!(report.queue_status, FileInitStatus::Created);
        assert_eq!(report.done_status, FileInitStatus::Created);
        assert_eq!(report.config_status, FileInitStatus::Created);
        assert!(matches!(
            report.readme_status,
            Some((FileInitStatus::Created, Some(3)))
        ));
        let cfg_raw = std::fs::read_to_string(resolved.project_config_path.as_ref().unwrap())?;
        let cfg: Config = serde_json::from_str(&cfg_raw)?;
        assert_eq!(cfg.project_type, Some(ProjectType::Code));
        assert_eq!(
            cfg.queue.file,
            Some(std::path::PathBuf::from(".ralph/queue.json"))
        );
        assert_eq!(
            cfg.queue.done_file,
            Some(std::path::PathBuf::from(".ralph/done.json"))
        );
        assert_eq!(cfg.queue.id_prefix, Some("RQ".to_string()));
        assert_eq!(cfg.queue.id_width, Some(4));
        assert_eq!(cfg.agent.runner, Some(crate::contracts::Runner::Claude));
        assert_eq!(
            cfg.agent.model,
            Some(crate::contracts::Model::Custom("sonnet".to_string()))
        );
        assert_eq!(
            cfg.agent.reasoning_effort,
            Some(crate::contracts::ReasoningEffort::Medium)
        );
        assert_eq!(cfg.agent.iterations, Some(1));
        assert_eq!(cfg.agent.followup_reasoning_effort, None);
        assert_eq!(cfg.agent.gemini_bin, Some("gemini".to_string()));
        Ok(())
    }

    #[test]
    fn init_creates_json_for_new_install() -> Result<()> {
        let dir = TempDir::new()?;
        let resolved = resolved_for(&dir);
        let report = run_init(
            &resolved,
            InitOptions {
                force: false,
                force_lock: false,
                interactive: false,
                update_readme: false,
            },
        )?;
        assert_eq!(report.queue_status, FileInitStatus::Created);
        assert_eq!(report.done_status, FileInitStatus::Created);
        assert_eq!(report.config_status, FileInitStatus::Created);

        // Verify JSON files were created
        let queue_raw = std::fs::read_to_string(&resolved.queue_path)?;
        assert!(queue_raw.contains("{"));
        let done_raw = std::fs::read_to_string(&resolved.done_path)?;
        assert!(done_raw.contains("{"));
        let cfg_raw = std::fs::read_to_string(resolved.project_config_path.as_ref().unwrap())?;
        assert!(cfg_raw.contains("{"));
        Ok(())
    }

    #[test]
    fn init_skips_readme_when_not_referenced() -> Result<()> {
        let dir = TempDir::new()?;
        let resolved = resolved_for(&dir);

        // Override all prompts to ensure none reference the README.
        let overrides = resolved.repo_root.join(".ralph/prompts");
        fs::create_dir_all(&overrides)?;
        let prompt_files = [
            "worker.md",
            "worker_phase1.md",
            "worker_phase2.md",
            "worker_phase2_handoff.md",
            "worker_phase3.md",
            "worker_single_phase.md",
            "task_builder.md",
            "task_updater.md",
            "scan.md",
            "completion_checklist.md",
            "code_review.md",
            "phase2_handoff_checklist.md",
            "iteration_checklist.md",
        ];
        for file in prompt_files {
            fs::write(overrides.join(file), "no reference")?;
        }

        let report = run_init(
            &resolved,
            InitOptions {
                force: false,
                force_lock: false,
                interactive: false,
                update_readme: false,
            },
        )?;
        assert_eq!(report.readme_status, None);
        let readme_path = resolved.repo_root.join(".ralph/README.md");
        assert!(!readme_path.exists());
        Ok(())
    }

    #[test]
    fn init_fails_on_invalid_existing_queue() -> Result<()> {
        let dir = TempDir::new()?;
        let resolved = resolved_for(&dir);
        std::fs::create_dir_all(resolved.repo_root.join(".ralph"))?;

        // Create a queue with an invalid ID prefix (WRONG-0001 vs RQ)
        let queue_json = r#"{
  "version": 1,
  "tasks": [
    {
      "id": "WRONG-0001",
      "status": "todo",
      "title": "Bad ID",
      "tags": [],
      "scope": [],
      "evidence": [],
      "plan": [],
      "request": "test",
      "created_at": "2026-01-18T00:00:00Z",
      "updated_at": "2026-01-18T00:00:00Z"
    }
  ]
}"#;
        std::fs::write(&resolved.queue_path, queue_json)?;
        std::fs::write(&resolved.done_path, r#"{"version":1,"tasks":[]}"#)?;
        std::fs::write(
            resolved.project_config_path.as_ref().unwrap(),
            r#"{"version":1,"project_type":"code"}"#,
        )?;

        let result = run_init(
            &resolved,
            InitOptions {
                force: false,
                force_lock: false,
                interactive: false,
                update_readme: false,
            },
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("validate existing queue"));
        Ok(())
    }

    #[test]
    fn init_fails_on_invalid_existing_done() -> Result<()> {
        let dir = TempDir::new()?;
        let resolved = resolved_for(&dir);
        std::fs::create_dir_all(resolved.repo_root.join(".ralph"))?;

        std::fs::write(&resolved.queue_path, r#"{"version":1,"tasks":[]}"#)?;

        // Create a done file with a task that has invalid status for done file (todo instead of done)
        // Or we could use ID prefix mismatch again. Let's use ID prefix mismatch for simplicity and certainty.
        let done_json = r#"{
  "version": 1,
  "tasks": [
    {
      "id": "WRONG-0002",
      "status": "done",
      "title": "Bad ID",
      "tags": [],
      "scope": [],
      "evidence": [],
      "plan": [],
      "request": "test",
      "created_at": "2026-01-18T00:00:00Z",
      "updated_at": "2026-01-18T00:00:00Z"
    }
  ]
}"#;
        std::fs::write(&resolved.done_path, done_json)?;
        std::fs::write(
            resolved.project_config_path.as_ref().unwrap(),
            r#"{"version":1,"project_type":"code"}"#,
        )?;

        let result = run_init(
            &resolved,
            InitOptions {
                force: false,
                force_lock: false,
                interactive: false,
                update_readme: false,
            },
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("validate existing done"));
        Ok(())
    }

    #[test]
    fn init_with_wizard_answers_creates_configured_files() -> Result<()> {
        let dir = TempDir::new()?;
        let resolved = resolved_for(&dir);

        let wizard_answers = WizardAnswers {
            runner: Runner::Codex,
            model: "gpt-5.2-codex".to_string(),
            phases: 2,
            create_first_task: true,
            first_task_title: Some("Test task".to_string()),
            first_task_description: Some("Test description".to_string()),
            first_task_priority: TaskPriority::High,
        };

        let report = run_init(
            &resolved,
            InitOptions {
                force: false,
                force_lock: false,
                interactive: false,
                update_readme: false,
            },
        )?;

        // Manually write the queue with wizard answers to test the write_queue function
        write_queue(
            &resolved.queue_path,
            true,
            &resolved.id_prefix,
            resolved.id_width,
            Some(&wizard_answers),
        )?;

        write_config(
            resolved.project_config_path.as_ref().unwrap(),
            true,
            Some(&wizard_answers),
        )?;

        assert_eq!(report.done_status, FileInitStatus::Created);

        // Verify config has correct runner and phases
        let cfg_raw = std::fs::read_to_string(resolved.project_config_path.as_ref().unwrap())?;
        let cfg: Config = serde_json::from_str(&cfg_raw)?;
        assert_eq!(cfg.agent.runner, Some(Runner::Codex));
        assert_eq!(cfg.agent.phases, Some(2));

        // Verify queue has first task
        let queue = crate::queue::load_queue(&resolved.queue_path)?;
        assert_eq!(queue.tasks.len(), 1);
        assert_eq!(queue.tasks[0].title, "Test task");
        assert_eq!(queue.tasks[0].priority, TaskPriority::High);

        Ok(())
    }

    #[test]
    fn wizard_answers_default() {
        let answers = WizardAnswers::default();
        assert_eq!(answers.runner, Runner::Claude);
        assert_eq!(answers.model, "sonnet");
        assert_eq!(answers.phases, 3);
        assert!(!answers.create_first_task);
        assert!(answers.first_task_title.is_none());
        assert!(answers.first_task_description.is_none());
        assert_eq!(answers.first_task_priority, TaskPriority::Medium);
    }

    #[test]
    fn extract_readme_version_finds_version_marker() {
        let content = "<!-- RALPH_README_VERSION: 5 -->\n# Heading";
        assert_eq!(extract_readme_version(content), Some(5));

        let content_v2 = "<!-- RALPH_README_VERSION: 2 -->\n# Ralph";
        assert_eq!(extract_readme_version(content_v2), Some(2));
    }

    #[test]
    fn extract_readme_version_returns_none_for_no_marker() {
        let content = "# Ralph runtime files\nSome content";
        // Legacy content without marker returns Some(1) as default
        assert_eq!(extract_readme_version(content), Some(1));
    }

    #[test]
    fn write_readme_creates_new_file_with_version() -> Result<()> {
        let dir = TempDir::new()?;
        let readme_path = dir.path().join("README.md");

        let (status, version) = write_readme(&readme_path, false, false)?;

        assert_eq!(status, FileInitStatus::Created);
        assert_eq!(version, Some(README_VERSION));
        assert!(readme_path.exists());

        let content = std::fs::read_to_string(&readme_path)?;
        assert!(content.contains("RALPH_README_VERSION"));
        Ok(())
    }

    #[test]
    fn write_readme_preserves_existing_when_no_update() -> Result<()> {
        let dir = TempDir::new()?;
        let readme_path = dir.path().join("README.md");

        // Create an existing README with old version
        let old_content = "<!-- RALPH_README_VERSION: 1 -->\n# Old content";
        std::fs::write(&readme_path, old_content)?;

        let (status, version) = write_readme(&readme_path, false, false)?;

        assert_eq!(status, FileInitStatus::Valid);
        assert_eq!(version, Some(1));

        // Content should be preserved
        let content = std::fs::read_to_string(&readme_path)?;
        assert!(content.contains("Old content"));
        Ok(())
    }

    #[test]
    fn write_readme_updates_when_version_mismatch() -> Result<()> {
        let dir = TempDir::new()?;
        let readme_path = dir.path().join("README.md");

        // Create an existing README with old version
        let old_content = "<!-- RALPH_README_VERSION: 1 -->\n# Old content";
        std::fs::write(&readme_path, old_content)?;

        let (status, version) = write_readme(&readme_path, false, true)?;

        assert_eq!(status, FileInitStatus::Updated);
        assert_eq!(version, Some(README_VERSION));

        // Content should be updated
        let content = std::fs::read_to_string(&readme_path)?;
        assert!(!content.contains("Old content"));
        assert!(content.contains("Ralph runtime files"));
        Ok(())
    }

    #[test]
    fn write_readme_skips_update_when_current() -> Result<()> {
        let dir = TempDir::new()?;
        let readme_path = dir.path().join("README.md");

        // Create an existing README with current version
        let current_content = format!(
            "<!-- RALPH_README_VERSION: {} -->\n# Current",
            README_VERSION
        );
        std::fs::write(&readme_path, &current_content)?;

        let (status, version) = write_readme(&readme_path, false, true)?;

        // Should be Valid since version is current
        assert_eq!(status, FileInitStatus::Valid);
        assert_eq!(version, Some(README_VERSION));

        // Content should be preserved
        let content = std::fs::read_to_string(&readme_path)?;
        assert!(content.contains("Current"));
        Ok(())
    }

    #[test]
    fn write_readme_force_overwrites_regardless() -> Result<()> {
        let dir = TempDir::new()?;
        let readme_path = dir.path().join("README.md");

        // Create an existing README
        std::fs::write(&readme_path, "<!-- RALPH_README_VERSION: 99 -->\n# Custom")?;

        let (status, version) = write_readme(&readme_path, true, false)?;

        // When force-overwriting an existing file, status is Updated
        assert_eq!(status, FileInitStatus::Updated);
        assert_eq!(version, Some(README_VERSION));

        // Content should be overwritten
        let content = std::fs::read_to_string(&readme_path)?;
        assert!(!content.contains("Custom"));
        Ok(())
    }

    #[test]
    fn check_readme_current_detects_missing() -> Result<()> {
        let dir = TempDir::new()?;
        let resolved = resolved_for(&dir);

        let result = check_readme_current(&resolved)?;

        // README is applicable but missing
        assert!(matches!(result, ReadmeCheckResult::Missing));
        Ok(())
    }

    #[test]
    fn check_readme_current_detects_outdated() -> Result<()> {
        let dir = TempDir::new()?;
        let resolved = resolved_for(&dir);

        // Create README with old version
        fs::create_dir_all(resolved.repo_root.join(".ralph"))?;
        let old_readme = "<!-- RALPH_README_VERSION: 1 -->\n# Old";
        fs::write(resolved.repo_root.join(".ralph/README.md"), old_readme)?;

        let result = check_readme_current(&resolved)?;

        assert!(
            matches!(result, ReadmeCheckResult::Outdated { current_version: 1, embedded_version } if embedded_version == README_VERSION)
        );
        Ok(())
    }

    #[test]
    fn check_readme_current_detects_current() -> Result<()> {
        let dir = TempDir::new()?;
        let resolved = resolved_for(&dir);

        // Create README with current version
        fs::create_dir_all(resolved.repo_root.join(".ralph"))?;
        let current_readme = format!(
            "<!-- RALPH_README_VERSION: {} -->\n# Current",
            README_VERSION
        );
        fs::write(resolved.repo_root.join(".ralph/README.md"), current_readme)?;

        let result = check_readme_current(&resolved)?;

        assert!(matches!(result, ReadmeCheckResult::Current(v) if v == README_VERSION));
        Ok(())
    }

    #[test]
    fn init_update_readme_flag_updates_outdated() -> Result<()> {
        let dir = TempDir::new()?;
        let resolved = resolved_for(&dir);

        // Create an existing README with old version
        fs::create_dir_all(resolved.repo_root.join(".ralph"))?;
        let old_readme = "<!-- RALPH_README_VERSION: 1 -->\n# Old content";
        fs::write(resolved.repo_root.join(".ralph/README.md"), old_readme)?;
        fs::write(&resolved.queue_path, r#"{"version":1,"tasks":[]}"#)?;
        fs::write(&resolved.done_path, r#"{"version":1,"tasks":[]}"#)?;
        fs::write(
            resolved.project_config_path.as_ref().unwrap(),
            r#"{"version":1}"#,
        )?;

        let report = run_init(
            &resolved,
            InitOptions {
                force: false,
                force_lock: false,
                interactive: false,
                update_readme: true,
            },
        )?;

        // README should be updated
        assert!(matches!(
            report.readme_status,
            Some((FileInitStatus::Updated, Some(3)))
        ));

        // Content should be new
        let content = std::fs::read_to_string(resolved.repo_root.join(".ralph/README.md"))?;
        assert!(!content.contains("Old content"));
        assert!(content.contains("Ralph runtime files"));
        Ok(())
    }
}
