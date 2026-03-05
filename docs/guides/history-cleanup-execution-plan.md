# History Cleanup Execution Plan (March 4, 2026)

Purpose: execute a single, reviewer-friendly history rewrite before public release, using the current `main` branch log exactly as of `f96ab95b`.

## Baseline Snapshot

- Branch: `main`
- HEAD: `f96ab95b`
- Rewrite window: last 50 commits (`git log --oneline --reverse -n 50`)
- Strategy: interactive rebase with explicit commit grouping and noise-drop commits

## Safety Preconditions

1. Confirm repository is private and you are the only person relying on current `main` history.
2. Start from a clean worktree.
3. Create rollback anchors:

```bash
git branch backup/pre-public-history-20260304
git tag backup-pre-public-20260304
```

4. Validate baseline before rewrite:

```bash
make agent-ci
```

## Commit Grouping Map (from current log)

### Group A — Parallel worker recovery, push safety, and resume hardening
Target commit subjects (3 commits after squash/fixup):

1. `feat(parallel): harden post-run bookkeeping drift detection and push recovery`
2. `fix(parallel): harden rebase-aware push finalization and workspace path restoration`
3. `fix(runner): unify continue/retry recovery and timestamp-normalized queue validation`

Source commits:

- `4867805a` Auto-recover non-fast-forward pushes in post-run supervision
- `1e9a9b2d` Fix parallel worker bookkeeping restore paths
- `fa973051` Fail fast on parallel bookkeeping drift
- `cf8fe86a` Harden rebase-aware push for stale task branches
- `ee4ccfd4` Harden parallel worker finalization and rebase push retries
- `95f83505` Remove path override env vars and harden repo path resolution
- `1cce47e2` Enhance parallel worker bookkeeping restoration by adding task ID support and implementing plan cache cleanup functionality
- `deab3f97` Fix parallel worker queue/done mapping to workspace paths
- `001fef9a` Fix parallel task selection to honor queue order
- `fd85d7af` Unify continue recovery and harden Pi session fallback
- `56b64af7` Harden runner resume fallbacks and stream parsing
- `13617ada` Increase default max_push_attempts from 5 to 50 across configuration and documentation
- `07c6c44c` Enhance queue loading and validation with timestamp normalization

### Group B — Daemon signal handling
Target commit subject:

`RQ-0922: add SIGTERM handling to daemon serve`

Source commit:

- `c8c42081` RQ-0922: Add SIGTERM signal handler to daemon serve (#26)

### Group C — Direct-push cutover and architecture cleanup
Target commit subject:

`refactor(parallel): remove merge-agent path and finalize direct-push architecture`

Source commits:

- `18c518b2` refactor: remove merge-agent functionality and transition to direct-push mode
- `54afe4cb` refactor: remove parallel mode rewrite specification document
- `52a82db4` parallel: make integration agent-owned direct-push
- `a585ad7c` Rewrite parallel direct-push flow for base-branch workers
- `5ea7112a` Harden run-loop CI and parallel worker reliability
- `952cf904` parallel: sanitize integration prompt bytes for runner
- `a1018b68` parallel: satisfy clippy in prompt sanitization
- `8c348dfc` feat: integrate task title handling in phase execution and logging
- `e8a99b47` refactor: comprehensive hygiene cleanup - DRY violations, hardcoded constants, file size limits
- `a5ac72f0` chore: enhance logging and error handling in various modules
- `61d9c065` chore: update .gitignore and remove obsolete CLI skill files

### Group D — CI/test contract hardening
Target commit subject:

`test(ci): consolidate reliability coverage for queue/run/watch/parallel paths`

Source commits:

- `1d0c9afd` fix: Makefile test target race condition (RQ-0907)
- `c682ec7b` RQ-0923: Add tests for runner error classification functions (#27)
- `736462a0` RQ-0916: Add tests for ralph queue next-id command error paths
- `dfa077f3` tests: stabilize fake runner stdin handling in run_one integration
- `61bc73a8` test: add watch module coverage and stabilize git dirty checks
- `ace3f368` RQ-0940: Add tests for watch command submodules
- `73026245` test(parallel): validate queue-order scheduling with retained worker state (RQ-0959)
- `7cec3d71` test: add parallel done.json safety validation tests (RQ-0958)

### Group E — JSONC migration and runtime contract checks
Target commit subject:

`RQ-0957: migrate defaults to JSONC and harden runtime validation contracts`

Source commits:

- `c743fd48` Implement migration of Ralph defaults from .json to .jsonc, ensuring legacy support and automatic upgrades
- `2167cb8e` RQ-0957: Migrate Ralph defaults from .json to .jsonc with automatic upgrade
- `4f96f2c8` Seed worker bookkeeping files for migrated jsonc parallel runs
- `4f3b3626` Harden JSONC bookkeeping and keep agent README fresh
- `06835a74` Add .jsonc files for task tracking and migration history
- `db8ee600` feat: Add runtime validation for config queue thresholds
- `db4db74f` fix: add consistent task_id validation across all worker phase render functions
- `55ef7f65` fix: remove unused in_header variable in extract_content_from_exported (RQ-0927)
- `702910cf` fix: Add Ctrl-C check during backoff sleep in retry logic (RQ-0929)

### Group F — Dependency lockfile refresh
Target commit subject:

`chore(deps): refresh Cargo.lock to current non-yanked dependency set`

Source commit:

- `2f4fe284` Update dependencies in Cargo.lock to latest versions

### Group G — Public-facing docs and repo hygiene polish
Target commit subject:

`RQ-0960: polish public-facing docs and repository hygiene`

Source commits:

- `46194ff4` docs: refresh README and align docs with current CLI
- `f96ab95b` RQ-0960: polish public-facing docs and repo hygiene

### Drop as noise-only history (retain final tree state through later commits)

- `62b44433` RQ-0923: Add tests for runner error classification functions (#28)
  - Rationale: `.ralph`-only duplicate task artifact; behavior already captured by `c682ec7b` in Group D.
- `e4765972` Complete RQ-0923: Add tests for runner error classification functions
  - Rationale: `.ralph`-only duplicate task artifact; behavior already captured by `c682ec7b` in Group D.
- `33ed0c79` Prioritize queue for parallel stabilization testing
  - Rationale: queue-priority grooming noise in `.ralph`; superseded by later queue/bookkeeping final state.
- `7d429964` Trim queue to two parallel validation tasks
  - Rationale: queue-shape grooming noise in `.ralph`; superseded by later queue/bookkeeping final state.
- `b5a96ce5` Reframe parallel queue tasks as validation smoke checks
  - Rationale: queue wording/state grooming in `.ralph`; superseded by later queue/bookkeeping final state.

## Exact Interactive Rebase Todo (`git rebase -i HEAD~50`)

Preflight guard (run before opening interactive rebase):

```bash
test "$(git rev-parse --short HEAD)" = "f96ab95b"
git log --oneline --reverse -n 50 | sed -n '1p;$p'
# expected first: 4867805a ...
# expected last:  f96ab95b ...
```

Paste the following into the rebase todo editor exactly:

```text
reword 4867805a Auto-recover non-fast-forward pushes in post-run supervision
fixup 1e9a9b2d Fix parallel worker bookkeeping restore paths
fixup fa973051 Fail fast on parallel bookkeeping drift
reword cf8fe86a Harden rebase-aware push for stale task branches
fixup ee4ccfd4 Harden parallel worker finalization and rebase push retries
fixup 95f83505 Remove path override env vars and harden repo path resolution
fixup 1cce47e2 Enhance parallel worker bookkeeping restoration by adding task ID support and implementing plan cache cleanup functionality. Update related tests to verify correct restoration behavior.
fixup deab3f97 Fix parallel worker queue/done mapping to workspace paths
fixup 001fef9a Fix parallel task selection to honor queue order
reword fd85d7af Unify continue recovery and harden Pi session fallback
fixup 56b64af7 Harden runner resume fallbacks and stream parsing
fixup 13617ada Increase default max_push_attempts from 5 to 50 across configuration and documentation
fixup 07c6c44c Enhance queue loading and validation with timestamp normalization
reword c8c42081 RQ-0922: Add SIGTERM signal handler to daemon serve (#26)
reword 18c518b2 refactor: remove merge-agent functionality and transition to direct-push mode
fixup 54afe4cb refactor: remove parallel mode rewrite specification document
fixup 52a82db4 parallel: make integration agent-owned direct-push
fixup a585ad7c Rewrite parallel direct-push flow for base-branch workers
fixup 5ea7112a Harden run-loop CI and parallel worker reliability
fixup 952cf904 parallel: sanitize integration prompt bytes for runner
fixup a1018b68 parallel: satisfy clippy in prompt sanitization
fixup 8c348dfc feat: integrate task title handling in phase execution and logging
fixup e8a99b47 refactor: comprehensive hygiene cleanup - DRY violations, hardcoded constants, file size limits
fixup a5ac72f0 chore: enhance logging and error handling in various modules
fixup 61d9c065 chore: update .gitignore and remove obsolete CLI skill files
reword 1d0c9afd fix: Makefile test target race condition (RQ-0907)
fixup c682ec7b RQ-0923: Add tests for runner error classification functions (#27)
fixup 736462a0 RQ-0916: Add tests for ralph queue next-id command error paths
fixup dfa077f3 tests: stabilize fake runner stdin handling in run_one integration
fixup 61bc73a8 test: add watch module coverage and stabilize git dirty checks
fixup ace3f368 RQ-0940: Add tests for watch command submodules
fixup 73026245 test(parallel): validate queue-order scheduling with retained worker state (RQ-0959)
fixup 7cec3d71 test: add parallel done.json safety validation tests (RQ-0958)
reword c743fd48 Implement migration of Ralph defaults from .json to .jsonc, ensuring legacy support and automatic upgrades for new projects. Update related configurations and .gitignore handling to facilitate a smooth transition.
fixup 2167cb8e RQ-0957: Migrate Ralph defaults from .json to .jsonc with automatic upgrade
fixup 4f96f2c8 Seed worker bookkeeping files for migrated jsonc parallel runs
fixup 4f3b3626 Harden JSONC bookkeeping and keep agent README fresh
fixup 06835a74 Add .jsonc files for task tracking and migration history
fixup db8ee600 feat: Add runtime validation for config queue thresholds
fixup db4db74f fix: add consistent task_id validation across all worker phase render functions
fixup 55ef7f65 fix: remove unused in_header variable in extract_content_from_exported (RQ-0927)
fixup 702910cf fix: Add Ctrl-C check during backoff sleep in retry logic (RQ-0929)
reword 2f4fe284 Update dependencies in Cargo.lock to latest versions
drop 62b44433 RQ-0923: Add tests for runner error classification functions (#28)
drop e4765972 Complete RQ-0923: Add tests for runner error classification functions
drop 33ed0c79 Prioritize queue for parallel stabilization testing
drop 7d429964 Trim queue to two parallel validation tasks
drop b5a96ce5 Reframe parallel queue tasks as validation smoke checks
reword 46194ff4 docs: refresh README and align docs with current CLI
fixup f96ab95b RQ-0960: polish public-facing docs and repo hygiene
```

## Reword Targets (final subjects to use)

Use these subjects when rebase prompts for `reword` commits (in todo order):

1. `feat(parallel): harden post-run bookkeeping drift detection and push recovery`
2. `fix(parallel): harden rebase-aware push finalization and workspace path restoration`
3. `fix(runner): unify continue/retry recovery and timestamp-normalized queue validation`
4. `RQ-0922: add SIGTERM handling to daemon serve`
5. `refactor(parallel): remove merge-agent path and finalize direct-push architecture`
6. `test(ci): consolidate reliability coverage for queue/run/watch/parallel paths`
7. `RQ-0957: migrate defaults to JSONC and harden runtime validation contracts`
8. `chore(deps): refresh Cargo.lock to current non-yanked dependency set`
9. `RQ-0960: polish public-facing docs and repository hygiene`

## Post-Rebase Verification

Run this sequence before force-push:

```bash
# check rewritten stack shape
git log --oneline --reverse -n 20

# ensure no content drift relative to backup
git diff --stat backup/pre-public-history-20260304..HEAD

# project gates
make agent-ci
scripts/pre-public-check.sh --skip-clean --skip-ci
```

If all checks pass and repo is still private/solo-maintained:

```bash
git push --force-with-lease origin main
```

Rollback at any point:

```bash
git rebase --abort || true
git reset --hard backup/pre-public-history-20260304
```
