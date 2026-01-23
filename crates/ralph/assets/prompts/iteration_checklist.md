<!-- Purpose: Checklist for non-final iterations (refinement mode). -->
## ITERATION CHECKLIST (REFINEMENT MODE)
When refining an already-implemented task, you MUST:
1. Verify behavior against the task requirements and look for regressions or unintended side effects.
2. Simplify or deduplicate code where possible while keeping behavior correct.
3. Tighten tests to cover expected behavior and failure modes uncovered by the review.
4. If the CI gate is enabled ({{config.agent.ci_gate_enabled}}), run `{{config.agent.ci_gate_command}}` and fix failures until it is green.
5. Summarize changes, remaining risks, and any follow-up work needed for the next run.
6. Do NOT run `ralph task done`, and leave the working tree dirty for continued iteration.
