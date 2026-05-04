//! Task building functionality for creating new tasks via runner invocation.
//!
//! Purpose:
//! - Task building functionality for creating new tasks via runner invocation.
//!
//! Responsibilities:
//! - Build tasks using AI runners via .cueloop/prompts/task_builder.md.
//! - Apply template hints and target contexts when specified.
//! - Validate queue state before and after runner execution.
//! - Position new tasks intelligently in the queue.
//! - Backfill missing task fields (request, timestamps) after creation.
//!
//! Not handled here:
//! - Task updating (see update/mod.rs).
//! - Refactor task generation (see refactor.rs).
//! - CLI argument parsing or command routing.
//! - Direct queue file manipulation outside of runner-driven changes.
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Queue file is the source of truth for task ordering.
//! - Runner execution produces valid task JSON output.
//! - Template loading and merging happens before prompt rendering.
//! - Lock acquisition is optional (controlled by acquire_lock parameter).

use super::{TaskBuildOptions, resolve_task_build_settings};
use crate::commands::run::PhaseType;
use crate::contracts::ProjectType;
use crate::queue::operations::{CreatedTaskNormalization, normalize_created_tasks};
use crate::{config, git, prompts, queue, runner, runutil, timeutil};
use anyhow::{Context, Result, bail};

pub fn build_task(resolved: &config::Resolved, opts: TaskBuildOptions) -> Result<()> {
    build_task_created_tasks(resolved, opts).map(|_| ())
}

pub fn build_task_created_tasks(
    resolved: &config::Resolved,
    opts: TaskBuildOptions,
) -> Result<Vec<crate::contracts::Task>> {
    build_task_impl(resolved, opts, true)
}

pub fn build_task_without_lock(resolved: &config::Resolved, opts: TaskBuildOptions) -> Result<()> {
    build_task_impl(resolved, opts, false).map(|_| ())
}

fn build_task_impl(
    resolved: &config::Resolved,
    mut opts: TaskBuildOptions,
    acquire_lock: bool,
) -> Result<Vec<crate::contracts::Task>> {
    git::require_clean_repo_ignoring_paths(
        &resolved.repo_root,
        opts.force,
        git::CUELOOP_QUEUE_ONLY_ALLOWED_PATHS,
    )?;

    let _queue_lock = if acquire_lock {
        Some(queue::acquire_queue_lock(
            &resolved.repo_root,
            "task",
            opts.force,
        )?)
    } else {
        None
    };

    if opts.request.trim().is_empty() {
        bail!("Missing request: task requires a request description. Provide a non-empty request.");
    }

    // Apply template if specified
    let mut template_context = String::new();
    if let Some(template_name) = opts.template_hint.clone() {
        // Use context-aware loading with validation
        let load_result = crate::template::load_template_with_context(
            &template_name,
            &resolved.repo_root,
            opts.template_target.as_deref(),
            opts.strict_templates,
        );

        match load_result {
            Ok(loaded) => {
                // Log any warnings from template validation
                for warning in &loaded.warnings {
                    log::warn!("Template '{}': {}", template_name, warning);
                }

                crate::template::merge_template_with_options(&loaded.task, &mut opts);
                template_context = crate::template::format_template_context(&loaded.task);
                log::info!("Using template '{}' for task creation", template_name);
            }
            Err(e) => {
                if opts.strict_templates {
                    bail!(
                        "Template '{}' failed strict validation: {}",
                        template_name,
                        e
                    );
                } else {
                    log::warn!("Failed to load template '{}': {}", template_name, e);
                }
            }
        }
    }

    let before = queue::load_queue(&resolved.queue_path)
        .with_context(|| format!("read queue {}", resolved.queue_path.display()))?;

    // Compute insertion strategy from pre-run queue state
    let insert_index = queue::suggest_new_task_insert_index(&before);

    let done = queue::load_queue_or_default(&resolved.done_path)
        .with_context(|| format!("read done {}", resolved.done_path.display()))?;
    let done_ref = if done.tasks.is_empty() && !resolved.done_path.exists() {
        None
    } else {
        Some(&done)
    };
    let max_depth = resolved.config.queue.max_dependency_depth.unwrap_or(10);
    queue::validate_queue_set(
        &before,
        done_ref,
        &resolved.id_prefix,
        resolved.id_width,
        max_depth,
    )
    .context("validate queue set before task")?;
    let before_ids = queue::task_id_set(&before);

    let template = prompts::load_task_builder_prompt(&resolved.repo_root)?;
    let project_type = resolved.config.project_type.unwrap_or(ProjectType::Code);
    let mut prompt = prompts::render_task_builder_prompt(
        &template,
        &opts.request,
        &opts.hint_tags,
        &opts.hint_scope,
        project_type,
        &resolved.config,
    )?;

    // Append template context to prompt if available
    if !template_context.is_empty() {
        prompt.push_str("\n\n--- Template Suggestions ---\n");
        prompt.push_str(&template_context);
    }

    prompt = prompts::wrap_with_repoprompt_requirement(&prompt, opts.repoprompt_tool_injection);
    prompt = prompts::wrap_with_instruction_files(&resolved.repo_root, &prompt, &resolved.config)?;

    let baseline = git::capture_dirty_path_baseline_ignoring_paths(
        &resolved.repo_root,
        git::CUELOOP_QUEUE_ONLY_ALLOWED_PATHS,
    )?;

    let settings = resolve_task_build_settings(resolved, &opts)?;
    let bins = runner::resolve_binaries(&resolved.config.agent);
    // Two-pass mode disabled for task (only generates task, should not implement)

    let retry_policy = runutil::RunnerRetryPolicy::from_config(&resolved.config.agent.runner_retry)
        .unwrap_or_default();

    let _output = runutil::run_prompt_with_handling(
        runutil::RunnerInvocation {
            settings: runutil::RunnerSettings {
                repo_root: &resolved.repo_root,
                runner_kind: settings.runner,
                bins,
                model: settings.model,
                reasoning_effort: settings.reasoning_effort,
                cursor: settings.cursor,
                runner_cli: settings.runner_cli,
                timeout: None,
                permission_mode: settings.permission_mode,
                output_handler: opts.output.output_handler(),
                output_stream: opts.output.output_stream(),
            },
            execution: runutil::RunnerExecutionContext {
                prompt: &prompt,
                phase_type: PhaseType::SinglePhase,
                session_id: None,
            },
            failure: runutil::RunnerFailureHandling {
                revert_on_error: false,
                git_revert_mode: resolved
                    .config
                    .agent
                    .git_revert_mode
                    .unwrap_or(crate::contracts::GitRevertMode::Ask),
                revert_prompt: None,
            },
            retry: runutil::RunnerRetryState {
                policy: retry_policy,
            },
        },
        runutil::RunnerErrorMessages {
            log_label: "task builder",
            interrupted_msg: "Task builder interrupted: the agent run was canceled.",
            timeout_msg: "Task builder timed out: the agent run exceeded the time limit. Changes in the working tree were NOT reverted; review the repo state manually.",
            terminated_msg: "Task builder terminated: the agent was stopped by a signal. Review uncommitted changes before rerunning.",
            non_zero_msg: |code| {
                format!(
                    "Task builder failed: the agent exited with a non-zero code ({}). Review uncommitted changes before rerunning.",
                    code
                )
            },
            other_msg: |err| {
                format!(
                    "Task builder failed: the agent could not be started or encountered an error. Error: {:#}",
                    err
                )
            },
        },
    )?;

    runutil::handle_queue_only_unexpected_mutations(
        &resolved.repo_root,
        "Task builder queue-only mutation boundary",
        git::CUELOOP_QUEUE_ONLY_ALLOWED_PATHS,
        &baseline,
        resolved
            .config
            .agent
            .git_revert_mode
            .unwrap_or(crate::contracts::GitRevertMode::Ask),
        None,
    )?;

    let mut after = match queue::load_queue(&resolved.queue_path)
        .with_context(|| format!("read queue {}", resolved.queue_path.display()))
    {
        Ok(queue) => queue,
        Err(err) => {
            return Err(err);
        }
    };

    let done_after = queue::load_queue_or_default(&resolved.done_path)
        .with_context(|| format!("read done {}", resolved.done_path.display()))?;
    let done_after_ref = if done_after.tasks.is_empty() && !resolved.done_path.exists() {
        None
    } else {
        Some(&done_after)
    };
    queue::validate_queue_set(
        &after,
        done_after_ref,
        &resolved.id_prefix,
        resolved.id_width,
        max_depth,
    )
    .context("validate queue set after task")?;

    let added = queue::added_tasks(&before_ids, &after);
    let mut created_tasks = Vec::new();
    if !added.is_empty() {
        let added_ids: Vec<String> = added.iter().map(|(id, _)| id.clone()).collect();

        let now = timeutil::now_utc_rfc3339_or_fallback();
        normalize_created_tasks(
            &mut after,
            &added_ids,
            &CreatedTaskNormalization {
                insert_at: insert_index,
                default_request: &opts.request,
                now_rfc3339: &now,
                estimated_minutes: opts.estimated_minutes,
            },
        );

        queue::save_queue(&resolved.queue_path, &after)
            .context("save queue with backfilled fields")?;

        created_tasks = after
            .tasks
            .iter()
            .filter(|task| added_ids.contains(&task.id))
            .cloned()
            .collect();
    }
    if added.is_empty() {
        log::info!("Task builder completed. No new tasks detected.");
    } else {
        log::info!("Task builder added {} task(s):", added.len());
        for (id, title) in added.iter().take(10) {
            log::info!("- {}: {}", id, title);
        }
        if added.len() > 10 {
            log::info!("...and {} more.", added.len() - 10);
        }
    }
    Ok(created_tasks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{Config, Model, QueueFile, Runner, RunnerCliOptionsPatch};
    use crate::testsupport::git as git_test;
    use crate::testsupport::runner::create_fake_runner;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn resolved_with_config(config: Config) -> (config::Resolved, TempDir) {
        let dir = TempDir::new().expect("temp dir");
        let repo_root = dir.path().to_path_buf();
        let queue_rel = config
            .queue
            .file
            .clone()
            .unwrap_or_else(|| PathBuf::from(".cueloop/queue.jsonc"));
        let done_rel = config
            .queue
            .done_file
            .clone()
            .unwrap_or_else(|| PathBuf::from(".cueloop/done.jsonc"));
        let id_prefix = config
            .queue
            .id_prefix
            .clone()
            .unwrap_or_else(|| "RQ".to_string());
        let id_width = config.queue.id_width.unwrap_or(4) as usize;

        (
            config::Resolved {
                config,
                repo_root: repo_root.clone(),
                queue_path: repo_root.join(queue_rel),
                done_path: repo_root.join(done_rel),
                id_prefix,
                id_width,
                global_config_path: None,
                project_config_path: Some(repo_root.join(".cueloop/config.jsonc")),
            },
            dir,
        )
    }

    fn initialize_repo(resolved: &config::Resolved) -> anyhow::Result<()> {
        git_test::init_repo(&resolved.repo_root)?;
        std::fs::create_dir_all(
            resolved
                .queue_path
                .parent()
                .expect("queue parent should exist"),
        )?;
        std::fs::write(resolved.repo_root.join("README.md"), "# task build test\n")?;
        queue::save_queue(&resolved.queue_path, &QueueFile::default())?;
        queue::save_queue(&resolved.done_path, &QueueFile::default())?;
        git_test::commit_all(&resolved.repo_root, "init task build repo")?;
        Ok(())
    }

    fn build_opts() -> TaskBuildOptions {
        TaskBuildOptions {
            request: "Build a queue-only task".to_string(),
            hint_tags: String::new(),
            hint_scope: String::new(),
            runner_override: Some(Runner::Codex),
            model_override: Some(Model::Gpt53Codex),
            reasoning_effort_override: None,
            runner_cli_overrides: RunnerCliOptionsPatch::default(),
            force: false,
            repoprompt_tool_injection: false,
            output: super::super::TaskBuildOutputTarget::Quiet,
            template_hint: None,
            template_target: None,
            strict_templates: false,
            estimated_minutes: None,
        }
    }

    #[test]
    fn build_task_rejects_stray_non_queue_mutations() -> anyhow::Result<()> {
        let (mut resolved, _dir) = resolved_with_config(Config::default());
        initialize_repo(&resolved)?;

        let queue_after = serde_json::json!({
            "version": 1,
            "tasks": [{
                "id": "RQ-0001",
                "status": "todo",
                "title": "Built by runner",
                "priority": "medium",
                "tags": [],
                "scope": [],
                "evidence": [],
                "plan": [],
                "notes": [],
                "request": "Build a queue-only task",
                "created_at": "2026-04-23T00:00:00Z",
                "updated_at": "2026-04-23T00:00:00Z"
            }]
        });
        let queue_after_path = resolved
            .repo_root
            .join(".cueloop/cache/build-queue-after.json");
        std::fs::create_dir_all(
            queue_after_path
                .parent()
                .expect("queue-after parent should exist"),
        )?;
        std::fs::write(
            &queue_after_path,
            serde_json::to_string_pretty(&queue_after)?,
        )?;

        let readme_path = resolved.repo_root.join("README.md");
        let runner_script = format!(
            r#"#!/bin/sh
set -e
cat >/dev/null
cp "{queue_after}" "{queue_path}"
printf '\nstray edit\n' >> "{readme_path}"
echo '{{"type":"item.completed","item":{{"type":"agent_message","text":"task complete"}}}}'
"#,
            queue_after = queue_after_path.display(),
            queue_path = resolved.queue_path.display(),
            readme_path = readme_path.display(),
        );
        let runner_dir = TempDir::new()?;
        let runner_path = create_fake_runner(runner_dir.path(), "codex", &runner_script)?;
        resolved.config.agent.codex_bin = Some(runner_path.to_string_lossy().to_string());
        resolved.config.agent.git_revert_mode = Some(crate::contracts::GitRevertMode::Enabled);

        let err = build_task_created_tasks(&resolved, build_opts())
            .expect_err("task build should fail on stray mutation");
        let message = format!("{err:#}");
        assert!(message.contains("Queue-only mutation boundary violated."));
        assert!(message.contains("README.md"));

        let queue = queue::load_queue(&resolved.queue_path)?;
        assert!(queue.tasks.is_empty(), "queue changes should be reverted");
        assert_eq!(
            std::fs::read_to_string(&readme_path)?,
            "# task build test\n"
        );
        Ok(())
    }
}
