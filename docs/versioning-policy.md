# Versioning Policy

Purpose: define how Ralph versions releases and communicates compatibility changes.

## Scheme

Ralph follows semantic versioning:

- `MAJOR`: breaking CLI/config/behavior changes
- `MINOR`: backward-compatible features
- `PATCH`: backward-compatible fixes

## Compatibility Expectations

- Public command behavior changes must be documented in:
  - `CHANGELOG.md`
  - relevant docs under `docs/`
- Breaking changes require migration notes in release docs
- Config schema changes must keep validation/error messaging explicit

## Deprecation Policy

- Prefer explicit deprecation windows for user-facing commands/options
- Document deprecations in changelog before removal when feasible
- Remove dead/deprecated paths promptly once cutover is complete

## Release Hygiene

Before tagging:

```bash
make ci
make pre-public-check
```

If macOS app changes are included:

```bash
make macos-ci
```
