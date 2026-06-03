# Archive and Audit Policy
Status: Active
Owner: Maintainers
Source of truth: this document for archive and point-in-time documentation policy
Parent: [CueLoop Documentation](../index.md)

CueLoop keeps historical documents when they explain why a decision was made, capture audit evidence, or preserve useful investigation context. Most documents listed here are not active operating instructions. The exception is the current stack audit, which remains an active toolchain/dependency baseline until superseded.

## How to read archived material

- Treat archived docs and audit snapshots as **point-in-time evidence** unless the document explicitly says it is the current active baseline.
- Trust active product docs, CLI help, generated schemas, and source code when they disagree with older artifacts.
- Do not add new current behavior to an archived document. Update the canonical active page instead, then link to the archive if history matters.
- When a historical item produces active follow-up work, track that work in the queue, roadmap, decision log, or canonical feature doc rather than editing the old snapshot.

## Current historical and point-in-time docs

| Document | Purpose |
| --- | --- |
| [Thermo-Nuclear Code Quality Review (2026-05-21)](../audits/thermo-nuclear-code-quality-review-2026-05-21.md) | Point-in-time maintainability review and remediation inventory |
| [Comprehensive Codebase Audit (2026-03-31)](../audits/codebase-audit-2026-03-31.md) | Point-in-time codebase audit and cleanup inventory |
| [CueLoopMac Settings Window Investigation (2026-03-13)](../audits/2026-03-13-cueloopmac-settings-window-investigation.md) | Resolved UI investigation notes |
| [Stack Audit (2026-04)](../guides/stack-audit-2026-04.md) | Active toolchain/dependency baseline and Rust 1.96.0 review notes until superseded |
| [Stack Audit (2026-03)](../guides/stack-audit-2026-03.md) | Older baseline kept for comparison with the April stack audit |
| [Roadmap Archive](../roadmap.md) | Historical roadmap context and completed planning notes |

## Why audit files stay in `docs/audits/`

Audit snapshots remain under `docs/audits/` so existing links, review artifacts, and queue evidence paths stay stable. This archive policy page owns their interpretation: audits are preserved evidence, not live runbooks.

If a future docs migration moves audit files, move **all** audit snapshots together, update this page, update inbound links, and run the markdown link checks before opening the PR.
