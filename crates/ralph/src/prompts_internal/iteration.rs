//! Shared prompt blocks for multi-iteration refinement behavior.

pub const ITERATION_CONTEXT_REFINEMENT: &str = r#"
## REFINEMENT CONTEXT
A prior execution of this task already occurred in this run. Focus on refinement:
- identify regressions or unintended behavior changes
- simplify or harden the implementation where possible
- do NOT assume the task is complete

The working tree may already be dirty from earlier work. Do NOT stop because the repo is dirty.
"#;

pub const ITERATION_COMPLETION_BLOCK: &str = r#"
## ITERATION COMPLETION RULES
This run must NOT complete the task.
- Do NOT run `ralph task done` or `ralph task reject`.
- Leave the task status as `doing`.
- Leave the working tree dirty for continued iteration.
"#;

pub const PHASE3_COMPLETION_GUIDANCE_FINAL: &str =
    "Task status is already set to `doing` by Ralph. Do NOT change it (use `ralph task done` when finished).";

pub const PHASE3_COMPLETION_GUIDANCE_NONFINAL: &str =
    "Task status is already set to `doing` by Ralph. Do NOT change it. Do NOT run `ralph task done` or `ralph task reject` in this run.";
