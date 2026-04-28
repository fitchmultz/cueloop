## PHASE 2 HANDOFF CHECKLIST (3-PHASE WORKFLOW)
Follow this checklist. REQUIRED items are workflow-critical.

1. REQUIRED: if `agent.ci_gate.enabled` is true (`{{config.agent.ci_gate_enabled}}`) and you made changes, run `{{config.agent.ci_gate_display}}` and fix failures until it is green. If `agent.ci_gate.enabled=false`, only the configured CI command is skipped; Phase 2 implementation and handoff still continue.
2. REQUIRED: do not run `ralph task done`, `git commit`, or `git push` in Phase 2.
3. REQUIRED: leave the working tree dirty with the task changes for Phase 3 review (do not revert or stash).
4. PREFERRED: resolve follow-ups, inconsistencies, missing tests, or suspicious leads in Phase 2 instead of deferring them.
5. REQUIRED: if you discovered independent follow-up work, mention whether `.ralph/cache/followups/<current-task-id>.json` was written so Phase 3 can apply it before terminal bookkeeping.
6. If you are truly blocked, clearly summarize the blocker and include exact remediation steps for the next run.
7. PREFERRED: summarize what changed and how to verify it with exact commands when practical.
8. REQUIRED: stop after configured Phase 2 validation is satisfied. If changes were made and `agent.ci_gate.enabled` is true, that means the configured CI gate is green. If no changes were made, state that CI validation was unnecessary because there were no changes. If `agent.ci_gate.enabled=false`, state that configured CI validation was skipped by configuration. Phase 3 still owns review, refinement, follow-up proposal application, and terminal task bookkeeping.
