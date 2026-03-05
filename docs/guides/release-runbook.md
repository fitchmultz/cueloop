# Release Runbook

Purpose: define a repeatable release flow with explicit verification and rollback points.

## Preconditions

- Working tree is clean
- Version and changelog updates are staged
- Local toolchain is healthy

## Release Steps

1. Run required gates:

```bash
make agent-ci
make ci
make pre-public-check
```

2. If app changes are included:

```bash
make macos-ci
```

3. Dry-run release workflow:

```bash
RELEASE_DRY_RUN=1 scripts/release.sh <version>
```

4. Build artifacts:

```bash
make release-artifacts VERSION=<version>
```

5. Final human review:

- README + docs links
- release notes/changelog entries
- publication checklist completion

## Rollback Notes

If release prep fails before tagging:

- stop and fix issues on the branch
- rerun full gate sequence

If a bad release commit is created locally:

- reset or revert before public push
- regenerate artifacts after fixes

## Evidence to Capture

- command logs for required gates
- final `git status --short`
- release readiness report update
