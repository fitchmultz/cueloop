//! Phase-specific execution logic for `ralph run`.
//!
//! This module isolates multi-phase runner workflows (planning, implementation,
//! code review) from higher-level orchestration in `crate::run_cmd`.

use crate::completions;
use crate::config;
use crate::contracts::{ProjectType, TaskStatus};
use crate::{gitutil, promptflow, prompts, queue, runner, runutil, timeutil};
use anyhow::{anyhow, bail, Result};

use super::logging;

/// Shared inputs for executing a run phase workflow.
///
/// This struct intentionally groups parameters to keep function signatures small and
/// avoid clippy `too_many_arguments`, while preserving exact behaviors from
/// `run_cmd.rs`.
#[derive(Clone)]
pub struct PhaseInvocation<'a> {
    pub resolved: &'a config::Resolved,
    pub settings: &'a runner::AgentSettings,
    pub bins: runner::RunnerBinaries<'a>,
    pub task_id: &'a str,
    pub base_prompt: &'a str,
    pub policy: &'a promptflow::PromptPolicy,
    pub output_handler: Option<runner::OutputHandler>,
    pub project_type: ProjectType,
    pub git_revert_mode: crate::contracts::GitRevertMode,
    pub git_commit_push_enabled: bool,
    pub revert_prompt: Option<runutil::RevertPromptHandler>,
}

pub fn execute_phase1_planning(ctx: &PhaseInvocation<'_>, total_phases: u8) -> Result<String> {
    let label = logging::phase_label(1, total_phases, "Planning", ctx.task_id);

    logging::with_scope(&label, || {
        let p1_template = prompts::load_worker_phase1_prompt(&ctx.resolved.repo_root)?;
        let p1_prompt = promptflow::build_phase1_prompt(
            &p1_template,
            ctx.base_prompt,
            ctx.task_id,
            total_phases,
            ctx.policy,
            &ctx.resolved.config,
        )?;
        let output = execute_runner_pass(
            ctx.resolved,
            ctx.settings,
            ctx.bins,
            &p1_prompt,
            ctx.output_handler.clone(),
            true,
            ctx.git_revert_mode,
            ctx.revert_prompt.clone(),
            "Planning",
        )?;

        let mut continue_session = super::supervision::ContinueSession {
            runner: ctx.settings.runner,
            model: ctx.settings.model.clone(),
            reasoning_effort: ctx.settings.reasoning_effort,
            session_id: output.session_id.clone(),
            output_handler: ctx.output_handler.clone(),
        };

        // ENFORCEMENT: Phase 1 must not implement.
        // It may only edit `.ralph/queue.json` / `.ralph/done.json` (status bookkeeping)
        // plus the plan cache file for the current task.
        let plan_cache_rel = format!(".ralph/cache/plans/{}.md", ctx.task_id);
        let allowed_paths = [
            ".ralph/queue.json",
            ".ralph/done.json",
            plan_cache_rel.as_str(),
        ];
        loop {
            match gitutil::require_clean_repo_ignoring_paths(
                &ctx.resolved.repo_root,
                false,
                &allowed_paths,
            ) {
                Ok(()) => break,
                Err(err) => {
                    let outcome = runutil::apply_git_revert_mode(
                        &ctx.resolved.repo_root,
                        ctx.git_revert_mode,
                        "Phase 1 plan-only violation",
                        ctx.revert_prompt.as_ref(),
                    )?;
                    match outcome {
                        runutil::RevertOutcome::Continue { message } => {
                            super::supervision::resume_continue_session(
                                ctx.resolved,
                                &mut continue_session,
                                &message,
                            )?;
                            continue;
                        }
                        _ => {
                            bail!(
                                "{} Error: {:#}",
                                runutil::format_revert_failure_message(
                                    "Phase 1 violated plan-only contract: it modified files outside allowed queue bookkeeping.",
                                    outcome,
                                ),
                                err
                            );
                        }
                    }
                }
            }
        }

        // Read plan from cache (Phase 1 writes it directly).
        let plan_text = promptflow::read_plan_cache(&ctx.resolved.repo_root, ctx.task_id)?;
        log::info!(
            "Plan cached for {} at {}",
            ctx.task_id,
            promptflow::plan_cache_path(&ctx.resolved.repo_root, ctx.task_id).display()
        );

        Ok(plan_text)
    })
}

pub fn execute_phase2_implementation(
    ctx: &PhaseInvocation<'_>,
    total_phases: u8,
    plan_text: &str,
) -> Result<()> {
    let label = logging::phase_label(2, total_phases, "Implementation", ctx.task_id);

    logging::with_scope(&label, || {
        if total_phases == 3 {
            let handoff_template = prompts::load_phase2_handoff_checklist(&ctx.resolved.repo_root)?;
            let handoff_checklist =
                prompts::render_phase2_handoff_checklist(&handoff_template, &ctx.resolved.config)?;
            let p2_template = prompts::load_worker_phase2_handoff_prompt(&ctx.resolved.repo_root)?;
            let p2_prompt = promptflow::build_phase2_handoff_prompt(
                &p2_template,
                ctx.base_prompt,
                plan_text,
                &handoff_checklist,
                ctx.task_id,
                total_phases,
                ctx.policy,
                &ctx.resolved.config,
            )?;

            let output = execute_runner_pass(
                ctx.resolved,
                ctx.settings,
                ctx.bins,
                &p2_prompt,
                ctx.output_handler.clone(),
                true,
                ctx.git_revert_mode,
                ctx.revert_prompt.clone(),
                "Implementation",
            )?;

            let mut continue_session = super::supervision::ContinueSession {
                runner: ctx.settings.runner,
                model: ctx.settings.model.clone(),
                reasoning_effort: ctx.settings.reasoning_effort,
                session_id: output.session_id.clone(),
                output_handler: ctx.output_handler.clone(),
            };

            loop {
                match super::supervision::run_ci_gate(ctx.resolved) {
                    Ok(()) => break,
                    Err(err) => {
                        let outcome = runutil::apply_git_revert_mode(
                            &ctx.resolved.repo_root,
                            ctx.git_revert_mode,
                            "Phase 2 CI failure",
                            ctx.revert_prompt.as_ref(),
                        )?;
                        match outcome {
                            runutil::RevertOutcome::Continue { message } => {
                                super::supervision::resume_continue_session(
                                    ctx.resolved,
                                    &mut continue_session,
                                    &message,
                                )?;
                                continue;
                            }
                            _ => {
                                bail!(
                                    "{} Error: {:#}",
                                    runutil::format_revert_failure_message(
                                        "CI gate failed after Phase 2. Fix issues reported by CI and rerun.",
                                        outcome,
                                    ),
                                    err
                                );
                            }
                        }
                    }
                }
            }

            return Ok(());
        }

        let checklist_template = prompts::load_completion_checklist(&ctx.resolved.repo_root)?;
        let completion_checklist = prompts::render_completion_checklist(
            &checklist_template,
            ctx.task_id,
            &ctx.resolved.config,
        )?;
        let p2_template = prompts::load_worker_phase2_prompt(&ctx.resolved.repo_root)?;
        let p2_prompt = promptflow::build_phase2_prompt(
            &p2_template,
            ctx.base_prompt,
            plan_text,
            &completion_checklist,
            ctx.task_id,
            total_phases,
            ctx.policy,
            &ctx.resolved.config,
        )?;

        execute_runner_pass(
            ctx.resolved,
            ctx.settings,
            ctx.bins,
            &p2_prompt,
            ctx.output_handler.clone(),
            true,
            ctx.git_revert_mode,
            ctx.revert_prompt.clone(),
            "Implementation",
        )?;

        super::post_run_supervise(
            ctx.resolved,
            ctx.task_id,
            ctx.git_revert_mode,
            ctx.git_commit_push_enabled,
            ctx.revert_prompt.clone(),
        )?;
        Ok(())
    })
}

pub fn execute_phase3_review(ctx: &PhaseInvocation<'_>) -> Result<()> {
    let label = logging::phase_label(3, 3, "Review", ctx.task_id);

    logging::with_scope(&label, || {
        let review_template = prompts::load_code_review_prompt(&ctx.resolved.repo_root)?;
        let review_body = prompts::render_code_review_prompt(
            &review_template,
            ctx.task_id,
            ctx.project_type,
            &ctx.resolved.config,
        )?;

        let checklist_template = prompts::load_completion_checklist(&ctx.resolved.repo_root)?;
        let completion_checklist = prompts::render_completion_checklist(
            &checklist_template,
            ctx.task_id,
            &ctx.resolved.config,
        )?;
        let p3_template = prompts::load_worker_phase3_prompt(&ctx.resolved.repo_root)?;
        let p3_prompt = promptflow::build_phase3_prompt(
            &p3_template,
            ctx.base_prompt,
            &review_body,
            ctx.task_id,
            &completion_checklist,
            3,
            ctx.policy,
            &ctx.resolved.config,
        )?;

        let output = runutil::run_prompt_with_handling(
            runutil::RunnerInvocation {
                repo_root: &ctx.resolved.repo_root,
                runner_kind: ctx.settings.runner,
                bins: ctx.bins,
                model: ctx.settings.model.clone(),
                reasoning_effort: ctx.settings.reasoning_effort,
                prompt: &p3_prompt,
                timeout: None,
                permission_mode: ctx.resolved.config.agent.claude_permission_mode,
                revert_on_error: false,
                git_revert_mode: ctx.git_revert_mode,
                output_handler: ctx.output_handler.clone(),
                revert_prompt: ctx.revert_prompt.clone(),
            },
            runutil::RunnerErrorMessages {
            log_label: "Code review",
            interrupted_msg: "Code review interrupted: the agent run was canceled. Review the working tree and rerun Phase 3 to complete the task.",
            timeout_msg: "Code review timed out: the agent run exceeded the time limit. Review the working tree and rerun Phase 3 to complete the task.",
            terminated_msg: "Code review terminated: the agent was stopped by a signal. Review the working tree and rerun Phase 3 to complete the task.",
                non_zero_msg: |code| {
                    format!(
                        "Code review failed: the agent exited with a non-zero code ({code}). Review the working tree and rerun Phase 3 to complete the task."
                    )
                },
                other_msg: |err| {
                    format!(
                        "Code review failed: the agent could not be started or encountered an error. Review the working tree and rerun Phase 3. Error: {:#}",
                        err
                    )
                },
            },
        )?;

        let mut continue_session = super::supervision::ContinueSession {
            runner: ctx.settings.runner,
            model: ctx.settings.model.clone(),
            reasoning_effort: ctx.settings.reasoning_effort,
            session_id: output.session_id.clone(),
            output_handler: ctx.output_handler.clone(),
        };

        loop {
            if let Some(status) = apply_phase3_completion_signal(ctx.resolved, ctx.task_id)? {
                if status == TaskStatus::Done {
                    super::post_run_supervise(
                        ctx.resolved,
                        ctx.task_id,
                        ctx.git_revert_mode,
                        ctx.git_commit_push_enabled,
                        ctx.revert_prompt.clone(),
                    )?;
                }
            }

            match ensure_phase3_completion(ctx.resolved, ctx.task_id, ctx.git_commit_push_enabled) {
                Ok(()) => break,
                Err(err) => {
                    let outcome = runutil::apply_git_revert_mode(
                        &ctx.resolved.repo_root,
                        ctx.git_revert_mode,
                        "Phase 3 completion check",
                        ctx.revert_prompt.as_ref(),
                    )?;
                    match outcome {
                        runutil::RevertOutcome::Continue { message } => {
                            super::supervision::resume_continue_session(
                                ctx.resolved,
                                &mut continue_session,
                                &message,
                            )?;
                            continue;
                        }
                        _ => {
                            bail!(
                                "{} Error: {:#}",
                                runutil::format_revert_failure_message(
                                    "Phase 3 incomplete: task was not archived with a terminal status.",
                                    outcome,
                                ),
                                err
                            );
                        }
                    }
                }
            }
        }
        Ok(())
    })
}

pub fn execute_single_phase(ctx: &PhaseInvocation<'_>) -> Result<()> {
    let label = logging::single_phase_label("SinglePhase (Execution)", ctx.task_id);

    logging::with_scope(&label, || {
        let checklist_template = prompts::load_completion_checklist(&ctx.resolved.repo_root)?;
        let completion_checklist = prompts::render_completion_checklist(
            &checklist_template,
            ctx.task_id,
            &ctx.resolved.config,
        )?;
        let single_template = prompts::load_worker_single_phase_prompt(&ctx.resolved.repo_root)?;
        let prompt = promptflow::build_single_phase_prompt(
            &single_template,
            ctx.base_prompt,
            &completion_checklist,
            ctx.task_id,
            ctx.policy,
            &ctx.resolved.config,
        )?;

        execute_runner_pass(
            ctx.resolved,
            ctx.settings,
            ctx.bins,
            &prompt,
            ctx.output_handler.clone(),
            true,
            ctx.git_revert_mode,
            ctx.revert_prompt.clone(),
            "Execution",
        )?;

        super::post_run_supervise(
            ctx.resolved,
            ctx.task_id,
            ctx.git_revert_mode,
            ctx.git_commit_push_enabled,
            ctx.revert_prompt.clone(),
        )?;
        Ok(())
    })
}

#[allow(clippy::too_many_arguments)]
pub fn execute_runner_pass(
    resolved: &config::Resolved,
    settings: &runner::AgentSettings,
    bins: runner::RunnerBinaries,
    prompt: &str,
    output_handler: Option<runner::OutputHandler>,
    revert_on_error: bool,
    git_revert_mode: crate::contracts::GitRevertMode,
    revert_prompt: Option<runutil::RevertPromptHandler>,
    log_label: &str,
) -> Result<runner::RunnerOutput> {
    let permission_mode = resolved.config.agent.claude_permission_mode;

    runutil::run_prompt_with_handling(
        runutil::RunnerInvocation {
            repo_root: &resolved.repo_root,
            runner_kind: settings.runner,
            bins,
            model: settings.model.clone(),
            reasoning_effort: settings.reasoning_effort,
            prompt,
            timeout: None,
            permission_mode,
            revert_on_error,
            git_revert_mode,
            output_handler,
            revert_prompt,
        },
        runutil::RunnerErrorMessages {
            log_label,
            interrupted_msg: "Runner interrupted: the execution was canceled by the user or system.",
            timeout_msg: "Runner timed out: the execution exceeded the allowed time limit. Changes in the working tree were NOT reverted; review the repo state manually.",
            terminated_msg: "Runner terminated: the agent was stopped by a signal. Rerunning the task is recommended.",
            non_zero_msg: |code| {
                format!(
                    "Runner failed: the agent exited with a non-zero code ({code}). Rerunning the task is recommended after investigating the cause."
                )
            },
            other_msg: |err| {
                format!(
                    "Runner invocation failed: the agent could not be started or encountered an error. Rerunning the task is recommended. Error: {:#}",
                    err
                )
            },
        },
    )
}

pub fn apply_phase3_completion_signal(
    resolved: &config::Resolved,
    task_id: &str,
) -> Result<Option<TaskStatus>> {
    let Some(signal) = completions::take_completion_signal(&resolved.repo_root, task_id)? else {
        return Ok(None);
    };

    let now = timeutil::now_utc_rfc3339()?;
    let status = signal.status;
    queue::complete_task(
        &resolved.queue_path,
        &resolved.done_path,
        task_id,
        status,
        &now,
        &signal.notes,
        &resolved.id_prefix,
        resolved.id_width,
    )?;
    log::info!(
        "Supervisor finalized task {} with status {:?} from Phase 3 completion signal.",
        task_id,
        status
    );
    Ok(Some(status))
}

pub fn ensure_phase3_completion(
    resolved: &config::Resolved,
    task_id: &str,
    git_commit_push_enabled: bool,
) -> Result<()> {
    let queue_file = queue::load_queue(&resolved.queue_path)?;
    let done_file = queue::load_queue_or_default(&resolved.done_path)?;
    let done_ref = if done_file.tasks.is_empty() && !resolved.done_path.exists() {
        None
    } else {
        Some(&done_file)
    };
    queue::validate_queue_set(
        &queue_file,
        done_ref,
        &resolved.id_prefix,
        resolved.id_width,
    )?;

    let (status, _title, in_done) = super::find_task_status(&queue_file, &done_file, task_id)
        .ok_or_else(|| anyhow!("task {task_id} not found in queue or done"))?;

    if !in_done || !(status == TaskStatus::Done || status == TaskStatus::Rejected) {
        bail!(
            "Phase 3 incomplete: task {task_id} is not archived with a terminal status. Run `ralph task done` in Phase 3 before finishing."
        );
    }

    if git_commit_push_enabled {
        gitutil::require_clean_repo_ignoring_paths(&resolved.repo_root, false, &[])?;
    } else {
        log::info!(
            "Auto git commit/push disabled; skipping clean-repo enforcement for Phase 3 completion."
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{
        ClaudePermissionMode, Config, GitRevertMode, Model, QueueConfig, QueueFile,
        ReasoningEffort, Runner, Task, TaskPriority, TaskStatus,
    };
    use crate::queue;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tempfile::TempDir;

    fn git_init(dir: &Path) -> Result<()> {
        let status = Command::new("git")
            .current_dir(dir)
            .args(["init"])
            .status()?;
        anyhow::ensure!(status.success(), "git init failed");

        let gitignore_path = dir.join(".gitignore");
        std::fs::write(&gitignore_path, ".ralph/lock\n.ralph/cache/\nbin/\n")?;
        Command::new("git")
            .current_dir(dir)
            .args(["add", ".gitignore"])
            .status()?;
        Command::new("git")
            .current_dir(dir)
            .args(["commit", "-m", "add gitignore"])
            .status()?;

        Ok(())
    }

    fn create_fake_runner(dir: &Path, name: &str, script: &str) -> Result<PathBuf> {
        let bin_dir = dir.join("bin");
        std::fs::create_dir(&bin_dir)?;
        let runner_path = bin_dir.join(name);
        std::fs::write(&runner_path, script)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&runner_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&runner_path, perms)?;
        }

        Ok(runner_path)
    }

    fn resolved_for_repo(repo_root: PathBuf, opencode_bin: &Path) -> crate::config::Resolved {
        let mut cfg = Config::default();
        cfg.agent.runner = Some(Runner::Opencode);
        cfg.agent.model = Some(Model::Custom("zai-coding-plan/glm-4.7".to_string()));
        cfg.agent.reasoning_effort = Some(ReasoningEffort::Medium);
        cfg.agent.phases = Some(2);
        cfg.agent.claude_permission_mode = Some(ClaudePermissionMode::BypassPermissions);
        cfg.agent.git_revert_mode = Some(GitRevertMode::Ask);
        cfg.agent.git_commit_push_enabled = Some(true);
        cfg.agent.require_repoprompt = Some(false);
        cfg.agent.opencode_bin = Some(opencode_bin.display().to_string());
        cfg.queue = QueueConfig {
            file: Some(PathBuf::from(".ralph/queue.json")),
            done_file: Some(PathBuf::from(".ralph/done.json")),
            id_prefix: Some("RQ".to_string()),
            id_width: Some(4),
        };

        crate::config::Resolved {
            config: cfg,
            repo_root: repo_root.clone(),
            queue_path: repo_root.join(".ralph/queue.json"),
            done_path: repo_root.join(".ralph/done.json"),
            id_prefix: "RQ".to_string(),
            id_width: 4,
            global_config_path: None,
            project_config_path: Some(repo_root.join(".ralph/config.json")),
        }
    }

    fn resolved_for_completion(repo_root: PathBuf) -> crate::config::Resolved {
        crate::config::Resolved {
            config: Config::default(),
            repo_root: repo_root.clone(),
            queue_path: repo_root.join(".ralph/queue.json"),
            done_path: repo_root.join(".ralph/done.json"),
            id_prefix: "RQ".to_string(),
            id_width: 4,
            global_config_path: None,
            project_config_path: Some(repo_root.join(".ralph/config.json")),
        }
    }

    fn write_queue_and_done(repo_root: &Path, status: TaskStatus) -> Result<()> {
        std::fs::create_dir_all(repo_root.join(".ralph"))?;
        let task = Task {
            id: "RQ-0001".to_string(),
            status,
            title: "Test task".to_string(),
            priority: TaskPriority::Medium,
            tags: vec!["tests".to_string()],
            scope: vec!["crates/ralph".to_string()],
            evidence: vec!["observed".to_string()],
            plan: vec!["do thing".to_string()],
            notes: vec![],
            request: None,
            agent: None,
            created_at: Some("2026-01-18T00:00:00Z".to_string()),
            updated_at: Some("2026-01-18T00:00:00Z".to_string()),
            completed_at: Some("2026-01-18T00:00:00Z".to_string()),
            depends_on: vec![],
            custom_fields: std::collections::HashMap::new(),
        };

        queue::save_queue(
            &repo_root.join(".ralph/queue.json"),
            &QueueFile {
                version: 1,
                tasks: vec![],
            },
        )?;
        queue::save_queue(
            &repo_root.join(".ralph/done.json"),
            &QueueFile {
                version: 1,
                tasks: vec![task],
            },
        )?;
        Ok(())
    }

    #[test]
    fn phase1_continue_resumes_and_recovers_from_plan_only_violation() -> Result<()> {
        let temp = TempDir::new()?;
        git_init(temp.path())?;
        std::fs::create_dir_all(temp.path().join(".ralph/cache/plans"))?;

        let script = format!(
            r#"#!/bin/sh
set -e
plan="{root}/.ralph/cache/plans/RQ-0001.md"
dirty="{root}/dirty-file.txt"
if [ -f "$dirty" ]; then
  /bin/rm -f "$dirty"
else
  echo "dirty" > "$dirty"
fi
echo "plan content" > "$plan"
echo '{{"type":"text","part":{{"text":"ok"}}}}'
echo '{{"sessionID":"sess-123"}}'
"#,
            root = temp.path().display()
        );
        let runner_path = create_fake_runner(temp.path(), "opencode", &script)?;

        let resolved = resolved_for_repo(temp.path().to_path_buf(), &runner_path);
        let settings = runner::AgentSettings {
            runner: Runner::Opencode,
            model: Model::Custom("zai-coding-plan/glm-4.7".to_string()),
            reasoning_effort: None,
        };
        let bins = runner::RunnerBinaries {
            codex: "codex",
            opencode: runner_path.to_str().expect("runner path"),
            gemini: "gemini",
            claude: "claude",
        };
        let policy = promptflow::PromptPolicy {
            require_repoprompt: false,
        };

        let calls = Arc::new(AtomicUsize::new(0));
        let prompt_handler: runutil::RevertPromptHandler = Arc::new({
            let calls = Arc::clone(&calls);
            move |_label: &str| {
                if calls.fetch_add(1, Ordering::SeqCst) == 0 {
                    runutil::RevertDecision::Continue {
                        message: "continue".to_string(),
                    }
                } else {
                    runutil::RevertDecision::Keep
                }
            }
        });

        let invocation = PhaseInvocation {
            resolved: &resolved,
            settings: &settings,
            bins,
            task_id: "RQ-0001",
            base_prompt: "base prompt",
            policy: &policy,
            output_handler: None,
            project_type: ProjectType::Code,
            git_revert_mode: GitRevertMode::Ask,
            git_commit_push_enabled: true,
            revert_prompt: Some(prompt_handler),
        };

        let plan_text = execute_phase1_planning(&invocation, 2)?;
        assert_eq!(plan_text.trim(), "plan content");

        let status = Command::new("git")
            .current_dir(temp.path())
            .args(["status", "--porcelain"])
            .output()?;
        let stdout = String::from_utf8_lossy(&status.stdout);
        anyhow::ensure!(
            stdout.trim().is_empty(),
            "expected clean repo after resume, got:\n{}",
            stdout
        );

        Ok(())
    }

    #[test]
    fn ensure_phase3_completion_requires_clean_repo_when_enabled() -> Result<()> {
        let temp = TempDir::new()?;
        git_init(temp.path())?;
        write_queue_and_done(temp.path(), TaskStatus::Done)?;

        let resolved = resolved_for_completion(temp.path().to_path_buf());
        assert!(ensure_phase3_completion(&resolved, "RQ-0001", true).is_err());
        Ok(())
    }

    #[test]
    fn ensure_phase3_completion_allows_dirty_repo_when_disabled() -> Result<()> {
        let temp = TempDir::new()?;
        git_init(temp.path())?;
        write_queue_and_done(temp.path(), TaskStatus::Done)?;

        let resolved = resolved_for_completion(temp.path().to_path_buf());
        ensure_phase3_completion(&resolved, "RQ-0001", false)?;
        Ok(())
    }
}
