# Ralph Roadmap

Last updated: 2026-03-15

This is the canonical near-term roadmap for active follow-up work.

## Active roadmap

### 1. Split the remaining oversized Rust command-surface modules after the git/init/app cutover

Why first:
- The `commands/app`, `commands/init`, `git/commit`, and supervision parallel-worker cutovers are now complete, so the highest-churn oversized Rust files have shifted to CLI and command-routing surfaces.
- Decomposing command/CLI modules before deeper runtime helpers avoids shuffling callsites twice while command contracts are still being clarified.
- The remaining command-surface hotspots still mix routing, formatting, validation, and workflow orchestration in single files.

Scope:
- Decompose the current oversized command and CLI modules (`crates/ralph/src/commands/plugin/mod.rs`, `crates/ralph/src/cli/mod.rs`, `crates/ralph/src/cli/queue/issue.rs`, `crates/ralph/src/commands/task/decompose/mod.rs`, `crates/ralph/src/commands/task/update.rs`, `crates/ralph/src/commands/context/wizard.rs`, and adjacent command helpers) into thinner facades plus focused companion files.
- Preserve current CLI/help output, prompt behavior, and queue/task contracts while moving helpers and formatting logic out of the root modules.
- Keep any moved test hubs thin and behavior-grouped when command splits require neighboring test-module moves.

### 2. Split the remaining oversized Rust runtime/support operational modules after the command-surface pass

Why second:
- Once command surfaces stabilize, the next biggest maintenance risk sits in runtime/support modules that still mix orchestration, persistence, and formatting concerns.
- Webhook, queue-maintenance, processor execution, filesystem helpers, and execution-history modules are broad enough that command-surface churn would otherwise force repeated edits.
- Sequencing these after the command pass limits cross-cutting rename churn while the public entrypoints are settling.

Scope:
- Decompose the remaining oversized operational helpers (`crates/ralph/src/webhook/worker.rs`, `crates/ralph/src/webhook/diagnostics.rs`, `crates/ralph/src/queue/prune.rs`, `crates/ralph/src/queue/hierarchy.rs`, `crates/ralph/src/plugins/processor_executor.rs`, `crates/ralph/src/runutil/execution/orchestration.rs`, `crates/ralph/src/fsutil.rs`, `crates/ralph/src/execution_history.rs`, and adjacent support modules) into focused companions.
- Preserve webhook reload/retry contracts, queue safety behavior, and managed-subprocess invariants while extracting helpers from the root modules.
- Keep shared helpers centralized only where duplication is real; otherwise prefer adjacent behavior-grouped modules.

### 3. Split the remaining oversized Rust foundational/shared-data modules after command/runtime stabilization

Why third:
- Foundational helpers such as migration, template, agent-resolution, redaction, ETA, undo, and contract modules are broadly reused; touching them earlier would amplify churn while command/runtime facades are still moving.
- After the command/runtime passes, the remaining large shared modules can be decomposed with a clearer dependency picture and less risk of double-moves.
- These files are important, but they are lower-churn than the active command/runtime hotspots.

Scope:
- Decompose the remaining oversized foundational modules (`crates/ralph/src/migration/config_migrations.rs`, `crates/ralph/src/migration/file_migrations.rs`, `crates/ralph/src/template/variables.rs`, `crates/ralph/src/template/loader.rs`, `crates/ralph/src/agent/resolve.rs`, `crates/ralph/src/redaction.rs`, `crates/ralph/src/eta_calculator.rs`, `crates/ralph/src/undo.rs`, `crates/ralph/src/contracts/task.rs`, and adjacent shared helpers) into thinner facades plus focused companions.
- Preserve schema, normalization, redaction, and task-contract behavior exactly while moving parsing/formatting helpers out of the root files.
- Prefer deterministic helper modules and avoid reopening already-stabilized command/runtime seams unless a true shared abstraction emerges.

### 4. Split the remaining oversized Rust test and fixture hubs after the production-module passes

Why fourth:
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
- Do not reopen the completed macOS Settings/workspace-routing or the completed git/init/app split cutovers unless a new regression appears.
