//! Phase execution for run-one.
//!
//! Purpose:
//! - Phase execution for run-one.
//!
//! Responsibilities:
//! - Execute iteration phases based on phase count (1, 2, or 3 phases).
//! - Build phase invocations with common fields populated.
//! - Apply followup reasoning effort for multi-iteration runs.
//!
//! Not handled here:
//! - Context preparation (see context.rs).
//! - Task setup (see execution_setup.rs).
//! - Webhook notifications beyond phase-level wrappers (see webhooks.rs).
//!
//!
//! Usage:
//! - Used through the crate module tree or integration test harness.
//!
//! Invariants/assumptions:
//! - Phase count is validated to be 1, 2, or 3 before execution.
//! - Iteration settings have been resolved before calling execute.

use crate::agent::AgentOverrides;
use crate::commands::run::{
    RunEvent, RunEventHandler,
    iteration::apply_followup_reasoning_effort,
    phases::{self, PhaseInvocation, PostRunMode},
    supervision::PushPolicy,
};
use crate::config;
use crate::contracts::Task;
use crate::plugins::registry::PluginRegistry;
use crate::promptflow;
use crate::runutil::RevertPromptHandler;
use crate::{prompts, runner};
use anyhow::{Result, bail};

use super::{ResumeExecutionProgress, orchestration::TaskExecutionSetup};

/// Execute iteration phases based on phase count.
#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_iteration_phases(
    resolved: &config::Resolved,
    queue_lock: Option<&crate::lock::DirLock>,
    agent_overrides: &AgentOverrides,
    task: &Task,
    task_id: &str,
    setup: &TaskExecutionSetup,
    resume_progress: Option<ResumeExecutionProgress>,
    base_prompt: &str,
    policy: &promptflow::PromptPolicy,
    output_handler: Option<runner::OutputHandler>,
    run_event_handler: Option<RunEventHandler>,
    output_stream: runner::OutputStream,
    project_type: crate::contracts::ProjectType,
    git_revert_mode: crate::contracts::GitRevertMode,
    git_publish_mode: crate::contracts::GitPublishMode,
    push_policy: PushPolicy,
    revert_prompt: Option<RevertPromptHandler>,
    post_run_mode: PostRunMode,
    parallel_target_branch: Option<&str>,
    plugins: &PluginRegistry,
) -> Result<()> {
    PhaseExecutionContext {
        resolved,
        queue_lock,
        agent_overrides,
        task,
        task_id,
        setup,
        resume_progress,
        base_prompt,
        policy,
        output_handler,
        run_event_handler,
        output_stream,
        project_type,
        git_revert_mode,
        git_publish_mode,
        push_policy,
        revert_prompt,
        post_run_mode,
        parallel_target_branch,
        plugins,
        ci_gate_enabled: resolved.config.agent.ci_gate_enabled(),
        webhook_config: &resolved.config.agent.webhook,
    }
    .execute()
}

struct PhaseExecutionContext<'a> {
    resolved: &'a config::Resolved,
    queue_lock: Option<&'a crate::lock::DirLock>,
    agent_overrides: &'a AgentOverrides,
    task: &'a Task,
    task_id: &'a str,
    setup: &'a TaskExecutionSetup<'a>,
    resume_progress: Option<ResumeExecutionProgress>,
    base_prompt: &'a str,
    policy: &'a promptflow::PromptPolicy,
    output_handler: Option<runner::OutputHandler>,
    run_event_handler: Option<RunEventHandler>,
    output_stream: runner::OutputStream,
    project_type: crate::contracts::ProjectType,
    git_revert_mode: crate::contracts::GitRevertMode,
    git_publish_mode: crate::contracts::GitPublishMode,
    push_policy: PushPolicy,
    revert_prompt: Option<RevertPromptHandler>,
    post_run_mode: PostRunMode,
    parallel_target_branch: Option<&'a str>,
    plugins: &'a PluginRegistry,
    ci_gate_enabled: bool,
    webhook_config: &'a crate::contracts::WebhookConfig,
}

fn resume_start_iteration(
    resume_progress: Option<ResumeExecutionProgress>,
    iteration_count: u8,
) -> u8 {
    let Some(progress) = resume_progress else {
        return 1;
    };
    progress
        .iterations_completed
        .saturating_add(1)
        .clamp(1, iteration_count)
}

fn resume_start_phase_for_iteration(
    resume_progress: Option<ResumeExecutionProgress>,
    resume_start_iteration: u8,
    iteration_index: u8,
    phase_count: u8,
) -> u8 {
    let Some(progress) = resume_progress else {
        return 1;
    };
    if iteration_index != resume_start_iteration {
        return 1;
    }
    progress.current_phase.clamp(1, phase_count.max(1))
}

struct IterationExecution<'a> {
    phase2_settings: runner::AgentSettings,
    iteration_context: &'a str,
    iteration_completion_block: &'a str,
    phase3_completion_guidance: &'a str,
    is_final_iteration: bool,
    is_followup_iteration: bool,
    allow_dirty_repo: bool,
}

impl<'a> PhaseExecutionContext<'a> {
    fn emit_phase_entered(&self, phase: crate::progress::ExecutionPhase) {
        if let Some(handler) = &self.run_event_handler {
            handler(RunEvent::PhaseEntered { phase });
        }
    }

    fn emit_phase_completed(&self, phase: crate::progress::ExecutionPhase) {
        if let Some(handler) = &self.run_event_handler {
            handler(RunEvent::PhaseCompleted { phase });
        }
    }

    fn persist_session_phase(&self, phase: u8) {
        let cache_dir = crate::config::project_runtime_dir(&self.resolved.repo_root).join("cache");
        if let Err(err) = crate::session::set_session_phase(&cache_dir, phase) {
            log::warn!("Failed to persist session phase {}: {}", phase, err);
        }
    }

    fn mark_iteration_complete(&self, iteration_index: u8) {
        let cache_dir = crate::config::project_runtime_dir(&self.resolved.repo_root).join("cache");
        if let Err(err) = crate::session::mark_session_iteration_complete(&cache_dir) {
            log::warn!(
                "Failed to persist completion for iteration {}: {}",
                iteration_index,
                err
            );
        }
    }

    fn execute(&self) -> Result<()> {
        let start_iteration = self.resume_start_iteration();
        for iteration_index in start_iteration..=self.setup.iteration_settings.count {
            self.log_iteration(iteration_index);
            let iteration = self.build_iteration(iteration_index);
            let start_phase = self.resume_start_phase_for_iteration(iteration_index);

            match self.setup.phases {
                1 => self.execute_single_phase_iteration(&iteration)?,
                2 => self.execute_planned_iteration(&iteration, false, start_phase)?,
                3 => self.execute_planned_iteration(&iteration, true, start_phase)?,
                _ => {
                    bail!(
                        "Invalid phases value: {} (expected 1, 2, or 3). \
                         This indicates a configuration error or internal inconsistency.",
                        self.setup.phases
                    );
                }
            }
            self.mark_iteration_complete(iteration_index);
        }

        Ok(())
    }

    fn resume_start_iteration(&self) -> u8 {
        resume_start_iteration(self.resume_progress, self.setup.iteration_settings.count)
    }

    fn resume_start_phase_for_iteration(&self, iteration_index: u8) -> u8 {
        resume_start_phase_for_iteration(
            self.resume_progress,
            self.resume_start_iteration(),
            iteration_index,
            self.setup.phases,
        )
    }

    fn log_iteration(&self, iteration_index: u8) {
        let is_followup_iteration = iteration_index > 1;

        log::info!(
            "Task {}: iteration {iteration_index}/{}",
            self.task_id,
            self.setup.iteration_settings.count
        );

        if self.setup.iteration_settings.count > 1 {
            if is_followup_iteration {
                eprintln!();
            }
            eprintln!(
                "━━━ Iteration {iteration_index}/{} ━━━",
                self.setup.iteration_settings.count
            );
        }
    }

    fn build_iteration(&self, iteration_index: u8) -> IterationExecution<'static> {
        let is_followup_iteration = iteration_index > 1;
        let is_final_iteration = iteration_index == self.setup.iteration_settings.count;

        IterationExecution {
            phase2_settings: apply_followup_reasoning_effort(
                &self.setup.phase_matrix.phase2.to_agent_settings(),
                self.setup.iteration_settings.followup_reasoning_effort,
                is_followup_iteration,
            ),
            iteration_context: if is_followup_iteration {
                prompts::ITERATION_CONTEXT_REFINEMENT
            } else {
                ""
            },
            iteration_completion_block: if is_final_iteration {
                ""
            } else {
                prompts::ITERATION_COMPLETION_BLOCK
            },
            phase3_completion_guidance: if is_final_iteration {
                prompts::PHASE3_COMPLETION_GUIDANCE_FINAL
            } else {
                prompts::PHASE3_COMPLETION_GUIDANCE_NONFINAL
            },
            is_final_iteration,
            is_followup_iteration,
            allow_dirty_repo: is_followup_iteration || self.setup.preexisting_dirty_allowed,
        }
    }

    fn execute_planned_iteration(
        &self,
        iteration: &IterationExecution<'_>,
        include_review: bool,
        start_phase: u8,
    ) -> Result<()> {
        let plan_text = if start_phase <= 2 {
            Some(if start_phase <= 1 {
                let phase1_settings = self.setup.phase_matrix.phase1.to_agent_settings();
                self.execute_phase1(iteration, &phase1_settings)?
            } else {
                log::info!(
                    "Task {}: resume skipping Phase 1; loading cached plan",
                    self.task_id
                );
                promptflow::read_plan_cache(&self.resolved.repo_root, self.task_id)?
            })
        } else {
            None
        };

        if start_phase <= 2 {
            self.execute_phase2(
                iteration,
                &iteration.phase2_settings,
                plan_text.as_deref().unwrap_or_default(),
            )?;
        } else {
            log::info!("Task {}: resume skipping Phase 2", self.task_id);
        }

        if include_review {
            let phase3_settings = self.setup.phase_matrix.phase3.to_agent_settings();
            self.execute_phase3(iteration, &phase3_settings)?;
        }

        Ok(())
    }

    fn execute_single_phase_iteration(&self, iteration: &IterationExecution<'_>) -> Result<()> {
        let invocation = self.build_phase_invocation(iteration, &iteration.phase2_settings);
        self.persist_session_phase(2);
        self.emit_phase_entered(crate::progress::ExecutionPhase::Implementation);

        let result = super::webhooks::execute_impl_phase_with_webhooks(
            2,
            self.setup.phases,
            self.task_id,
            &self.task.title,
            self.webhook_config,
            self.ci_gate_enabled,
            &iteration.phase2_settings,
            self.resolved,
            &invocation,
            phases::execute_single_phase,
        );
        if result.is_ok() {
            self.emit_phase_completed(crate::progress::ExecutionPhase::Implementation);
        }
        result
    }

    fn execute_phase1(
        &self,
        iteration: &IterationExecution<'_>,
        settings: &runner::AgentSettings,
    ) -> Result<String> {
        let invocation = self.build_phase_invocation(iteration, settings);
        self.persist_session_phase(1);
        self.emit_phase_entered(crate::progress::ExecutionPhase::Planning);
        let result = super::webhooks::execute_phase1_with_webhooks(
            self.setup.phases,
            self.task_id,
            &self.task.title,
            self.webhook_config,
            self.ci_gate_enabled,
            settings,
            self.resolved,
            &invocation,
        );
        if result.is_ok() {
            self.emit_phase_completed(crate::progress::ExecutionPhase::Planning);
        }
        result
    }

    fn execute_phase2(
        &self,
        iteration: &IterationExecution<'_>,
        settings: &runner::AgentSettings,
        plan_text: &str,
    ) -> Result<()> {
        let invocation = self.build_phase_invocation(iteration, settings);
        self.persist_session_phase(2);
        self.emit_phase_entered(crate::progress::ExecutionPhase::Implementation);

        let result = super::webhooks::execute_impl_phase_with_webhooks(
            2,
            self.setup.phases,
            self.task_id,
            &self.task.title,
            self.webhook_config,
            self.ci_gate_enabled,
            settings,
            self.resolved,
            &invocation,
            |phase_invocation| {
                phases::execute_phase2_implementation(
                    phase_invocation,
                    self.setup.phases,
                    plan_text,
                )
            },
        );
        if result.is_ok() {
            self.emit_phase_completed(crate::progress::ExecutionPhase::Implementation);
        }
        result
    }

    fn execute_phase3(
        &self,
        iteration: &IterationExecution<'_>,
        settings: &runner::AgentSettings,
    ) -> Result<()> {
        let invocation = self.build_phase_invocation(iteration, settings);
        self.persist_session_phase(3);
        self.emit_phase_entered(crate::progress::ExecutionPhase::Review);

        let result = super::webhooks::execute_impl_phase_with_webhooks(
            3,
            self.setup.phases,
            self.task_id,
            &self.task.title,
            self.webhook_config,
            self.ci_gate_enabled,
            settings,
            self.resolved,
            &invocation,
            phases::execute_phase3_review,
        );
        if result.is_ok() {
            self.emit_phase_completed(crate::progress::ExecutionPhase::Review);
        }
        result
    }

    fn build_phase_invocation<'b>(
        &'b self,
        iteration: &'b IterationExecution<'b>,
        settings: &'b runner::AgentSettings,
    ) -> PhaseInvocation<'b> {
        PhaseInvocation {
            resolved: self.resolved,
            queue_lock: self.queue_lock,
            settings,
            bins: self.setup.bins,
            task_id: self.task_id,
            task_title: Some(&self.task.title),
            base_prompt: self.base_prompt,
            policy: self.policy,
            output_handler: self.output_handler.clone(),
            output_stream: self.output_stream,
            run_event_handler: self.run_event_handler.clone(),
            project_type: self.project_type,
            git_revert_mode: self.git_revert_mode,
            git_publish_mode: self.git_publish_mode,
            push_policy: self.push_policy,
            revert_prompt: self.revert_prompt.clone(),
            iteration_context: iteration.iteration_context,
            iteration_completion_block: iteration.iteration_completion_block,
            phase3_completion_guidance: iteration.phase3_completion_guidance,
            is_final_iteration: iteration.is_final_iteration,
            is_followup_iteration: iteration.is_followup_iteration,
            allow_dirty_repo: iteration.allow_dirty_repo,
            post_run_mode: self.post_run_mode,
            parallel_target_branch: self.parallel_target_branch,
            notify_on_complete: self.agent_overrides.notify_on_complete,
            notify_sound: self.agent_overrides.notify_sound,
            lfs_check: self.agent_overrides.lfs_check.unwrap_or(false),
            no_progress: self.agent_overrides.no_progress.unwrap_or(false),
            execution_timings: self.setup.execution_timings.as_ref(),
            plugins: Some(self.plugins),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resume_phase_plan_starts_at_saved_phase_for_first_resumed_iteration() {
        let progress = Some(ResumeExecutionProgress {
            iterations_completed: 0,
            current_phase: 3,
        });

        let start_iteration = resume_start_iteration(progress, 2);

        assert_eq!(start_iteration, 1);
        assert_eq!(
            resume_start_phase_for_iteration(progress, start_iteration, 1, 3),
            3
        );
        assert_eq!(
            resume_start_phase_for_iteration(progress, start_iteration, 2, 3),
            1
        );
    }

    #[test]
    fn resume_phase_plan_starts_after_completed_iterations() {
        let progress = Some(ResumeExecutionProgress {
            iterations_completed: 1,
            current_phase: 2,
        });

        let start_iteration = resume_start_iteration(progress, 3);

        assert_eq!(start_iteration, 2);
        assert_eq!(
            resume_start_phase_for_iteration(progress, start_iteration, 2, 3),
            2
        );
        assert_eq!(
            resume_start_phase_for_iteration(progress, start_iteration, 3, 3),
            1
        );
    }

    #[test]
    fn resume_phase_plan_clamps_out_of_range_progress() {
        let progress = Some(ResumeExecutionProgress {
            iterations_completed: 99,
            current_phase: 99,
        });

        let start_iteration = resume_start_iteration(progress, 3);

        assert_eq!(start_iteration, 3);
        assert_eq!(
            resume_start_phase_for_iteration(progress, start_iteration, 3, 2),
            2
        );
    }
}
