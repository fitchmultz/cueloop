# Release Runbook

Purpose: define a repeatable release flow with explicit verification and rollback points.

## Preconditions

- Working tree is clean
- `CHANGELOG.md` has final release-worthy notes
- Local toolchain is healthy
- `rustc --version` matches [`rust-toolchain.toml`](../../rust-toolchain.toml); if not, prefix commands with the rustup toolchain bin dir

Recommended toolchain prefix when macOS resolves a stale Homebrew Rust first:

```bash
TOOL="$HOME/.rustup/toolchains/1.94.0-aarch64-apple-darwin/bin"
PATH="$TOOL:$PATH" RUSTC="$TOOL/rustc"
```

For Makefile-driven gates, Ralph now auto-prefers the pinned `rust-toolchain.toml` toolchain when `rustup` is available.

## Release Steps

1. Run the preferred automated preflight:

```bash
make release-verify VERSION=<version>
# macOS/Homebrew users: gmake release-verify VERSION=<version>
```

This target:

- syncs version metadata
- verifies version drift is gone
- runs public-readiness checks that still make sense after version mutation
- runs the ship gate (`macos-ci` on macOS with Xcode, otherwise `ci`)
- dry-runs `scripts/release.sh`
- explicitly allows an already-existing local `v<version>` tag during that dry-run so re-verifying a cut release does not fail on tag state alone

2. If you need the underlying manual sequence instead:

```bash
make agent-ci
make ci
make pre-public-check
```

2. If app changes are included:

```bash
make macos-ci
```

3. Sync version metadata from `VERSION` and verify drift is gone:

```bash
./scripts/versioning.sh sync --version <version>
./scripts/versioning.sh check
git diff -- VERSION Cargo.lock crates/ralph/Cargo.toml \
  apps/RalphMac/RalphMac.xcodeproj/project.pbxproj \
  apps/RalphMac/RalphCore/VersionValidator.swift
```

4. Dry-run release workflow:

```bash
RELEASE_DRY_RUN=1 scripts/release.sh <version>
```

5. Real release:

```bash
scripts/release.sh <version>
```

6. Optional local artifact inspection before upload-only debugging:

```bash
make release-artifacts VERSION=<version>
```

7. Final human review:

- README + docs links
- release notes/changelog entries
- publication checklist completion
- GitHub release page and uploaded asset names/checksums

## Known Gotchas

- `Cargo.lock` is part of release metadata. If it changes during `versioning.sh sync`, that is expected and must be committed.
- `make release-verify` intentionally uses `scripts/pre-public-check.sh --skip-ci --skip-clean` after version sync so post-sync metadata changes do not cause a false failure.
- `make release-verify` also sets `RALPH_RELEASE_ALLOW_EXISTING_TAG=1` for the dry-run release script invocation. Real releases still fail on an existing tag; only explicit dry-run verification is allowed to continue.
- `scripts/release.sh` clears `target/release-artifacts/` before packaging so stale tarballs are not uploaded.
- `cargo package --list` runs with `--allow-dirty` during release prep because the release commit is created after packaging review.

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
