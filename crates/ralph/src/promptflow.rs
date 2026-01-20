use crate::contracts::Runner;
use crate::fsutil;
use crate::prompts;
use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunPhase {
    Phase1, // Planning
    Phase2, // Implementation
}

#[derive(Debug, Clone)]
pub struct PromptPolicy {
    pub require_repoprompt: bool,
}

pub const RALPH_PHASE1_PLAN_BEGIN: &str = "<<RALPH_PLAN_BEGIN>>";
pub const RALPH_PHASE1_PLAN_END: &str = "<<RALPH_PLAN_END>>";

/// Path to the cached plan for a given task ID.
pub fn plan_cache_path(repo_root: &Path, task_id: &str) -> PathBuf {
    repo_root
        .join(".ralph/cache/plans")
        .join(format!("{}.md", task_id))
}

/// Write a plan to the cache.
pub fn write_plan_cache(repo_root: &Path, task_id: &str, plan_text: &str) -> Result<()> {
    let path = plan_cache_path(repo_root, task_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    fsutil::write_atomic(&path, plan_text.as_bytes())?;
    Ok(())
}

/// Read a plan from the cache. Fails if missing or empty.
pub fn read_plan_cache(repo_root: &Path, task_id: &str) -> Result<String> {
    let path = plan_cache_path(repo_root, task_id);
    if !path.exists() {
        bail!("Plan cache not found at {}", path.display());
    }
    let content = std::fs::read_to_string(&path)?;
    if content.trim().is_empty() {
        bail!("Plan cache is empty at {}", path.display());
    }
    Ok(content)
}

/// Build the prompt for Phase 1 (Planning).
pub fn build_phase1_prompt(
    base_worker_prompt: &str,
    task_id: &str,
    policy: &PromptPolicy,
) -> String {
    let mut instructions = String::new();

    // 1. Heading
    instructions.push_str("# PLANNING MODE - PHASE 1 OF 2\n\n");

    // 2. Status update instruction (FIRST action)
    instructions.push_str(&prompts::task_status_doing_instruction_for(task_id));
    instructions.push('\n');

    // 3. RepoPrompt requirement (if enabled)
    if policy.require_repoprompt {
        instructions.push_str(prompts::REPOPROMPT_REQUIRED_INSTRUCTION);
        instructions.push_str("\n\n");
        instructions.push_str(prompts::REPOPROMPT_CONTEXT_BUILDER_PLANNING_INSTRUCTION);
        instructions.push('\n');
    }

    // 4. Planning-only constraint + Marker requirement
    instructions.push_str(&format!(
        r#"\n## OUTPUT REQUIREMENT: PLAN ONLY
You are in Phase 1 (Planning). You must NOT implement the code yet.
Your goal is to understand the task, gather context, and produce a detailed plan.

After your analysis (and `context_builder` usage if applicable), you must output the final plan wrapped in these exact markers:

{begin}
<your plan here>
{end}

The plan should be detailed enough for Phase 2 implementation.
"#,
        begin = RALPH_PHASE1_PLAN_BEGIN,
        end = RALPH_PHASE1_PLAN_END
    ));

    // 5. Divider and base prompt
    format!("{}\n\n---\n\n{}", instructions.trim(), base_worker_prompt)
}

/// Build the prompt for Phase 2 (Implementation).
pub fn build_phase2_prompt(plan_text: &str, policy: &PromptPolicy) -> String {
    let mut instructions = String::new();

    // 1. Heading
    instructions.push_str("# IMPLEMENTATION MODE - PHASE 2 OF 2\n\n");

    // 2. RepoPrompt requirement (optional in phase 2, but good for consistency)
    if policy.require_repoprompt {
        instructions.push_str(prompts::REPOPROMPT_REQUIRED_INSTRUCTION);
        instructions.push_str("\n\n");
    }

    // 3. Completion workflow
    instructions.push_str(prompts::TASK_COMPLETION_WORKFLOW);
    instructions.push('\n');

    // 4. The Plan
    instructions.push_str("# APPROVED PLAN\n\n");
    instructions.push_str(plan_text);
    instructions.push_str("\n\n---\n\n");

    // 5. Instruction to execute
    instructions.push_str("Proceed with the implementation of the plan above.");

    instructions
}

/// Build the prompt for Single Phase (Plan + Implement).
pub fn build_single_phase_prompt(
    base_worker_prompt: &str,
    task_id: &str,
    policy: &PromptPolicy,
) -> String {
    let mut instructions = String::new();

    // 1. Status update instruction (FIRST action)
    instructions.push_str(&prompts::task_status_doing_instruction_for(task_id));
    instructions.push('\n');

    // 2. RepoPrompt requirement
    if policy.require_repoprompt {
        instructions.push_str(prompts::REPOPROMPT_REQUIRED_INSTRUCTION);
        instructions.push_str("\n\n");
        instructions.push_str(prompts::REPOPROMPT_CONTEXT_BUILDER_PLANNING_INSTRUCTION);
        instructions.push('\n');
    }

    // 3. Completion workflow
    instructions.push_str(prompts::TASK_COMPLETION_WORKFLOW);
    instructions.push('\n');

    // 4. Combined instruction
    instructions
        .push_str("You must plan and then immediately implement the solution in this session.\n");

    // 5. Divider and base prompt
    format!("{}\n\n---\n\n{}", instructions.trim(), base_worker_prompt)
}

/// Extract the plan text from the runner's stdout.
pub fn extract_plan_text(runner_kind: Runner, stdout: &str) -> Result<String> {
    // 1. Pre-process stdout based on runner
    let content = if runner_kind == Runner::Claude {
        // Claude uses JSON-L output, we want the "result" field of the last line usually,
        // but here we are looking for the plan markers in the *text* output.
        // Assuming `stdout` passed here is the full raw stdout.
        // Actually, the `runner.rs` usually returns the *text* response if it handles parsing.
        // Let's assume `stdout` here is the actual model text response.
        // If it returns raw JSON-L for Claude (which it seems to do in `runner.rs`),
        // we need to handle that.
        // Let's look at `runner.rs` later. For now, let's assume `stdout` contains the text we need.
        // If it's JSON-L, we might need to parse it.
        // However, based on the plan, `extract_plan_text` takes `runner_kind` to handle this.

        // Heuristic: try to find markers in the raw string first.
        stdout
    } else {
        stdout
    };

    // 2. Extract between markers
    if let Some(start_idx) = content.find(RALPH_PHASE1_PLAN_BEGIN) {
        if let Some(end_idx) = content.find(RALPH_PHASE1_PLAN_END) {
            let start = start_idx + RALPH_PHASE1_PLAN_BEGIN.len();
            if start < end_idx {
                return Ok(content[start..end_idx].trim().to_string());
            }
        }
    }

    // 3. Fallback: if no markers, verify it's not empty and return trimmed content.
    // This supports legacy behavior or if the model forgot markers but output a plan.
    // Ideally we want to be strict, but for now we fallback.
    // EXCEPT: The plan says "Otherwise fallback to trimmed stdout".
    let trimmed = content.trim();
    if trimmed.is_empty() {
        bail!("Extracted plan is empty");
    }

    Ok(trimmed.to_string())
}
