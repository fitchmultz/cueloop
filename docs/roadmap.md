# Ralph Roadmap

Last updated: 2026-03-15

This is the canonical near-term roadmap for active follow-up work.

## Active roadmap

### 1. Split the remaining oversized Rust operational modules after the app-side cutover

Why first:
- The macOS app/core orchestration split is now complete, so the largest remaining production-structure debt has shifted back to Rust.
- Recent supervision hardening widened regression coverage, but several Rust operational modules still exceed the file-size target and mix orchestration with helpers or contract-specific logic.
- Tackling production Rust modules before the next test-hub pass avoids moving assertions twice while operational APIs are still being decomposed.

Scope:
- Decompose the current oversized Rust production files (`crates/ralph/src/commands/run/supervision/parallel_worker.rs`, `crates/ralph/src/git/commit.rs`, `crates/ralph/src/commands/app.rs`, `crates/ralph/src/commands/init.rs`, and the next-largest adjacent operational modules) into thinner facades plus focused companion files.
- Preserve the newly expanded supervision/revert coverage while moving helpers and test-only seams out of the root modules.
- Keep behavior-grouped test hubs thin when splits require neighboring test-module moves.

### 2. Split the remaining oversized Rust test and fixture hubs after the production-module pass

Why second:
- Once the production Rust facades are stabilized, the biggest remaining non-doc maintenance debt sits in large integration/unit suites and shared test-support hubs.
- Large files such as `task_lifecycle_test.rs`, `run_parallel_test.rs`, `prompt_cli_test.rs`, `phase_settings_matrix.rs`, and queue-operation test modules are now the clearest follow-on churn hotspots.
- Sequencing test-hub splits after the production refactors minimizes duplicate test moves while contracts are still settling.

Scope:
- Break remaining oversized Rust test and fixture files into thin suite roots plus behavior-grouped companions/directories.
- Preserve current coverage names, helper contracts, and `make agent-ci` / `make ci` verification behavior.
- Keep shared test support centralized only where duplication is real; otherwise prefer adjacent grouped helpers.

## Sequencing rules

- Keep completed roadmap items out of this file; replace them with the next active work only.
- Prefer infrastructure and fixture stabilization before broader feature churn.
- Do not reopen the completed Settings/workspace-routing contract cutovers unless a new regression appears.
