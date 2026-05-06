# CueLoop Dogfood Harness
Status: Active
Owner: Maintainers
Source of truth: this document for repeatable local CueLoop dogfooding
Parent: [CueLoop Documentation](../index.md)

Purpose: provide a repeatable end-to-end fixture project that exercises CueLoop setup, queue/task surfaces, and a real three-phase runner execution.

## Command

From the CueLoop repo root:

```bash
scripts/dogfood-cueloop.sh
```

The default run uses:

- runner: `pi`
- requested model: `zai-glm-5.1`
- effective pi model id: `zai/glm-5.1`
- phases: `3`
- fixture location: `target/dogfood-cueloop/<timestamp>/cueloop-dogfood-fixture`
- report: `target/dogfood-cueloop/<timestamp>/report.md`

Use `--skip-real-agent` for a fast non-runner check of fixture creation, `cueloop init`, config/doctor/prompt preview, queue/task surfaces, machine queue read, and run dry-run. The `pi`/`zai/glm-5.1` pairing is a disposable dogfood/test-workspace convention for real agent launches; do not change this repository's root `.cueloop/config.jsonc` just to enforce it.

## What It Covers

### Phase 1 — Bootstrap and diagnostics

- Creates a disposable git project with a tiny Python CLI, PRD fixture, TODO fixture, and CI script.
- Runs `cueloop init --non-interactive`.
- Writes trusted repo config for `pi`, `zai/glm-5.1`, three phases, commit-only publish mode, disabled webhooks, and fixture-local CI.
- Exercises config, doctor, runner, prompt preview, version, CLI spec, shell completions, context init, daemon help discovery, and top-level help for every CueLoop command category.

### Phase 2 — Command-surface breadth sweep

- Seeds `.cueloop/queue.jsonc` with a deterministic task.
- Exercises representative read, dry-run, preview, machine, and safe write paths across config, daemon status, queue, task, prompt, context, PRD, migrate, cleanup, webhook, productivity, plugin, runner, undo, run dry-run, and app help surfaces.
- Commits the initialized CueLoop runtime state so the real run starts from a clean git tree.

This is not a replacement for unit/integration tests. It is an operator-level dogfood sweep that proves the command surfaces compose in a real disposable repository.

### Phase 3 — Real three-phase execution

- Runs `cueloop run one --phases 3 --runner pi --model zai/glm-5.1` against the fixture task.
- Expects the agent to implement `greeter.py --name`, add tests, run fixture CI, complete the task, and commit the result.
- Re-runs fixture CI and queue validation after CueLoop finishes.

## Current Baseline Evidence

A full default run passed on 2026-04-28 with 94 command/probe passes and no probe failures. Artifacts:

```text
target/dogfood-cueloop/20260428T021541Z/report.md
```

One operator-facing friction point was found before the successful run: the requested model string `zai-glm-5.1` is not accepted by the current pi CLI; `pi --list-models` exposes `zai/glm-5.1`. The harness normalizes that known local alias so the default command remains repeatable while the report records both requested and effective model ids.
