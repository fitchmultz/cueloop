# Support Policy

Purpose: clarify supported platforms/tooling and what maintainers can realistically support.

## Supported Platforms

- Linux: supported for CLI workflows
- macOS: supported for CLI workflows and the SwiftUI app
- Windows: best-effort for CLI where dependency chain permits; no first-class app support

## Tooling Baseline

- Rust toolchain pinned by `rust-toolchain.toml`
- GNU Make >= 4 required for project targets
- Optional tools:
  - `cargo-nextest` (faster non-doc test runs)
  - `cargo-llvm-cov` (coverage)
  - Xcode (macOS app build/test)

## Support Windows

- Current release line: actively supported
- Older releases: best-effort only unless explicitly called out in release notes

## Issue Triage Expectations

When filing issues, include:

- exact command + output
- OS + toolchain versions
- whether failure reproduces on clean clone

Use:

- bug reports: `.github/ISSUE_TEMPLATE/bug_report.md`
- feature requests: `.github/ISSUE_TEMPLATE/feature_request.md`
