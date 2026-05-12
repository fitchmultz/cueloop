# Decisions

Status: Active
Owner: Maintainers
Source of truth: this document
Parent: [CueLoop Documentation](index.md)
Related: [Project Operating Constitution](guides/project-operating-constitution.md)
Last updated: 2026-05-12

This is the canonical decision log for project-level decisions that affect
CueLoop architecture, operations, documentation, release flow, or contributor and
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

## 2026-05-12: Keep install verification out of user-global bin directories

Decision: `make ci` and `make install-verify` must verify CLI install mechanics without replacing binaries in `$(BIN_DIR)`, `~/.local/bin`, `/usr/local/bin`, or other user-global locations. Only explicit operator install commands such as `make install` may write to those locations.

Date: 2026-05-12

Owner: Maintainers

Context: The full Rust gate includes `install-verify`. Before this decision, `install-verify` copied the freshly built release CLI into the writable configured bin directory, usually `~/.local/bin/cueloop`. That made routine validation mutate developer state on both Linux and macOS hosts and could replace the same binary that launched a long-running CueLoop self-development loop.

Chosen option: Have `install-verify` copy the release binary into a temporary bin directory, execute `cueloop --help` from that temporary install path, and clean it up automatically. Keep `make install` as the explicit command that installs the CLI and, on macOS, the app bundle.

Rejected options: Keep writing to `~/.local/bin` during CI because the behavior was documented; add a knob to opt out of user-global install verification; remove install verification entirely.

Reason: Validation should be safe and repeatable by default. A temporary install path still verifies executable permissions, binary naming, and basic launch behavior without hidden global side effects.

Expected consequences: `make ci`, `make agent-ci`, and Linux/macOS release-shaped validation no longer replace a developer's installed `cueloop` binary. Operators who want to update their local CLI still run `make install` explicitly.

Follow-up actions: None.

Review date, if any: None.

## 2026-04-27: Align CueLoop's source-build MSRV with the pinned Rust toolchain

Decision: Treat the repo-local `rust-toolchain.toml` channel as CueLoop's source-build Rust baseline and keep the CLI crate's `rust-version` aligned to the same minor Rust release.

Date: 2026-04-27

Owner: Maintainers

Context: CueLoop is a source-built CLI and macOS app project whose local development, release builds, schema generation, and app bundling all run through the pinned rustup toolchain. The system global stable toolchain moved to Rust `1.95.0`, but the repository still pinned `1.94.1`, which masked the global update and caused release-note research to start from the wrong baseline.

Chosen option: Bump `rust-toolchain.toml` to Rust `1.95.0` and bump `crates/cueloop/Cargo.toml` `rust-version` to `1.95` in the same cutover.

Rejected options: Keep `rust-version = "1.94"` as a lower consumer MSRV while validating contributors and releases only on Rust `1.95.0`; remove the repo-local toolchain override and rely on each contributor's global stable; teach release-version sync scripts to also mutate Rust baseline metadata.

Reason: Advertising a lower source-build MSRV than the validated pinned toolchain creates avoidable ambiguity for contributors, release automation, and app bundling. Keeping the MSRV and pinned baseline aligned makes the supported compiler floor explicit. Release semver sync remains a separate concern from Rust baseline ownership.

Expected consequences: Contributors and release builds use Rust `1.95.0` for this baseline. Future Rust baseline bumps should update the repo-local toolchain, crate `rust-version`, active stack audit, and validation evidence together.

Follow-up actions: Existing Rust 1.95.0 follow-up tasks RQ-0051 through RQ-0055 cover language/library adoption, compatibility audits, dependency/security/rustdoc refresh, and drift-check codification.

Review date, if any: None.

## 2026-04-27: Accept current binary replacement behavior during CueLoop self-development loops

Decision: Superseded on 2026-05-12 by "Keep install verification out of user-global bin directories" for normal validation. Explicit `make install` can still replace the installed `cueloop` binary, and CueLoop keeps the current long-running loop behavior for that operator-requested install path. Do not add automatic re-exec, stop-on-change, run-pinned executable copies, or queued remediation for explicit install replacement unless the maintainer explicitly reopens it.

Date: 2026-04-27

Owner: Maintainers

Context: This decision originally covered CueLoop workers that changed CueLoop itself and then ran `make agent-ci`. At the time, Rust crate changes could route to `make ci`, and `make ci` ran `install-verify`, which installed the release CLI into the writable bin directory. That validation-time behavior is no longer accepted. The remaining accepted case is an explicit `make install`, where the operator has requested replacement of their installed CLI or app bundle.

Chosen option: Accept the current long-running loop behavior only for explicit operator installs. If a maintainer wants to use a clean binary generation after `make install`, they can manually restart the loop. Do not change runtime behavior now.

Rejected options: Stop the loop whenever the binary changes; automatically
re-exec the coordinator at sequential or parallel safe boundaries; drain
parallel workers before re-exec; pin every run to a copied executable; spawn
future workers from a different binary discovery mechanism.

Reason: No concrete failures have been observed from the current behavior, while
the alternatives add non-trivial orchestration complexity or materially reduce
parallel-loop productivity. Stop-on-change would make CueLoop self-development
loops behave like one task at a time because most meaningful CueLoop changes can
alter the binary. Parallel safe-boundary re-exec would still make throughput
beholden to the slowest in-flight worker after each binary-changing task. More
aggressive hot-swap/re-exec designs risk introducing worker lifecycle, queue,
state, and cleanup bugs.

Expected consequences: CueLoop self-development loops may continue to run with an old in-memory coordinator after an explicit install replaces `~/.local/bin/cueloop` or another configured install path. This is an accepted risk for explicit installs, not for normal validation. Operators who want a fully fresh generation should restart the loop manually.

Follow-up actions: None. Do not automatically add this as an outstanding task
from `cueloop scan`, audits, TODO sweeps, or agent-created follow-up queues. Only
create work for this topic if a maintainer explicitly asks to revisit it or a
concrete reproducible failure is reported.

Review date, if any: None.

## 2026-05-04: Relax file-size gate to advisories plus extreme-file fail threshold

Decision: Replace raw 800/1000 LOC warn/fail behavior with a less noisy policy:
soft advisory above 1,500 LOC, review advisory above 3,000 LOC, and blocking
failure only above 5,000 LOC unless covered by a reasoned allowlist entry in
`scripts/file-size-allowlist.txt`.

Date: 2026-05-04

Owner: Maintainers

Context: The previous raw line-count gate produced noisy warnings on cohesive
files and made 1,000 LOC feel like a universal correctness boundary even when
reviewability, cohesion, and complexity were acceptable.

Chosen option: Keep a lightweight visibility guardrail, but make normal large
files advisory-only. Fail only on extreme human-authored files and permit tracked
allowlist entries using `glob | reason` when keeping a file intact is justified.

Rejected options: Delete the guardrail entirely; keep 800/1000 LOC thresholds;
make 3,000 LOC blocking by default.

Reason: Raw LOC alone should not block normal work, but the repository still
benefits from preventing accidental monster files and documenting exceptional
cases.

Expected consequences: `make ci-docs`, `make ci-fast`, and `make agent-ci` stop
nagging on moderately large files while still surfacing cleanup candidates and
blocking unreviewed files above 5,000 LOC.

Follow-up actions: Split large files when it improves cohesion; keep allowlist
entries rare and remove them when the reason expires.

Review date, if any: None.

## 2026-04-26: Enforce repository file-size policy in local CI tiers

Decision: Enforce CueLoop's documented file-size policy through a deterministic
local guardrail (`scripts/check-file-size-limits.sh`) wired into both
`make ci-docs` and `make ci-fast`. This decision was superseded by the
2026-05-04 threshold update above.

Date: 2026-04-26

Owner: Maintainers

Context: File-size limits were documented in [AGENTS.md](../AGENTS.md) but not
enforced by the canonical local gates, allowing oversized files to accumulate
without immediate feedback.

Chosen option: Add a dedicated script that scans tracked and untracked
non-ignored human-authored files, warns when files exceed the soft limit, fails
when files exceed the hard limit, and keeps generated/machine-owned exclusions
explicit and narrow.

Rejected options: Keep limits as documentation-only policy; fail immediately on
all soft-limit offenders; add broad source-tree exclusions to suppress current
offenders.

Reason: Warn-on-soft/fail-on-hard creates immediate visibility while preventing
new hard-limit debt, without turning existing soft-limit cleanup into a
permanent blocker.

Expected consequences: Docs-only and code-oriented local gates now surface
actionable offender paths and line counts. New hard-limit violations fail early
in the canonical local workflow.

Follow-up actions: Track and split current soft-limit offenders over time.

Review date, if any: None.

## 2026-04-26: Track CueLoopMac parity by scenario-level proof

Decision: Treat scenario-level proof entries in
[crates/cueloop/src/cli/app_parity.rs](../crates/cueloop/src/cli/app_parity.rs) as
the authoritative CueLoopMac parity signal, while keeping root-command coverage
only as a secondary structural guard.

Date: 2026-04-26

Owner: Maintainers

Context: Top-level command-family parity labels were too coarse to catch the
cross-surface drift found in the CueLoop audit. Important user-visible gaps lived
inside specific scenarios such as empty versus blocked loop summaries, Stop
After Current, custom queue-path resolution, execution-control visibility, and
continuation next-step mapping.

Chosen option: Store parity as explicit scenario entries that each name the
machine contract anchors, app-doc anchors, native surface, Rust proof tests,
and CueLoopMac proof tests for the scenario.

Rejected options: Continue using broad command-family parity as the
authoritative tracker; rely on freeform prose or roadmap notes instead of proof
anchors; treat Advanced Runner support as parity completion.

Reason: Scenario-level proof makes parity drift actionable and reviewable. It
lets maintainers see exactly which user-visible behavior is covered and which
Rust plus CueLoopMac tests prove that alignment.

Expected consequences: Parity changes now require updating the scenario
registry, keeping machine/app docs aligned, and adding proof tests when a new
scenario appears. Missing anchors should fail local validation loudly instead
of giving false confidence.

Follow-up actions: None.

Review date, if any: None.

## 2026-04-23: Adopt Project Operating Constitution

Decision: Adopt a project operating constitution as the canonical rule set for
accepting, modifying, and closing CueLoop project work.

Date: 2026-04-23

Owner: Maintainers

Context: CueLoop has multiple human-facing and agent-facing surfaces, including
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
