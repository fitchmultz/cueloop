# Decisions

Status: Active
Owner: Maintainers
Source of truth: this document
Parent: [Ralph Documentation](index.md)
Related: [Project Operating Constitution](guides/project-operating-constitution.md)
Last updated: 2026-04-26

This is the canonical decision log for project-level decisions that affect
Ralph architecture, operations, documentation, release flow, or contributor and
agent behavior. Keep execution instructions in their canonical operating docs;
record only the decision and its rationale here.

## Decision Template

```text
Decision:
Date:
Owner:
Context:
Chosen option:
Rejected options:
Reason:
Expected consequences:
Follow-up actions:
Review date, if any:
```

## 2026-04-26: Track RalphMac parity by scenario-level proof

Decision: Treat scenario-level proof entries in
[crates/ralph/src/cli/app_parity.rs](../crates/ralph/src/cli/app_parity.rs) as
the authoritative RalphMac parity signal, while keeping root-command coverage
only as a secondary structural guard.

Date: 2026-04-26

Owner: Maintainers

Context: Top-level command-family parity labels were too coarse to catch the
cross-surface drift found in the Ralph audit. Important user-visible gaps lived
inside specific scenarios such as empty versus blocked loop summaries, Stop
After Current, custom queue-path resolution, execution-control visibility, and
continuation next-step mapping.

Chosen option: Store parity as explicit scenario entries that each name the
machine contract anchors, app-doc anchors, native surface, Rust proof tests,
and RalphMac proof tests for the scenario.

Rejected options: Continue using broad command-family parity as the
authoritative tracker; rely on freeform prose or roadmap notes instead of proof
anchors; treat Advanced Runner support as parity completion.

Reason: Scenario-level proof makes parity drift actionable and reviewable. It
lets maintainers see exactly which user-visible behavior is covered and which
Rust plus RalphMac tests prove that alignment.

Expected consequences: Parity changes now require updating the scenario
registry, keeping machine/app docs aligned, and adding proof tests when a new
scenario appears. Missing anchors should fail local validation loudly instead
of giving false confidence.

Follow-up actions: None.

Review date, if any: None.

## 2026-04-23: Adopt Project Operating Constitution

Decision: Adopt a project operating constitution as the canonical rule set for
accepting, modifying, and closing Ralph project work.

Date: 2026-04-23

Owner: Maintainers

Context: Ralph has multiple human-facing and agent-facing surfaces, including
the Rust CLI, machine contracts, the macOS app, documentation, release scripts,
and local CI gates. Work in one area can easily create unmanaged drift if source
of truth, canonical path, downstream dependents, and validation are not explicit.

Chosen option: Store the constitution in
[docs/guides/project-operating-constitution.md](guides/project-operating-constitution.md),
link it from [docs/index.md](index.md), and point agent instructions in
[AGENTS.md](../AGENTS.md) to that canonical document instead of duplicating the
full rule set.

Rejected options: Keep the rules only in chat; paste the full rules into
AGENTS.md; maintain separate human and agent copies.

Reason: A single canonical document prevents conflicting instructions while
still making the rules discoverable to both humans and agents.

Expected consequences: Future work must identify source of truth, keep one
canonical path, remove or archive obsolete paths, update downstream dependents,
record important decisions, and complete meaningful validation before being
declared done.

Follow-up actions: None.

Review date, if any: None.
