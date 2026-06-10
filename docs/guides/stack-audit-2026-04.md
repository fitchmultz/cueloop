# Stack Audit (2026-04)
Status: Active
Owner: Maintainers
Source of truth: current language/toolchain/dependency baseline and historical Rust baseline migration notes
Parent: [CueLoop Documentation](../index.md)
Related: [CI and Test Strategy](ci-strategy.md), [Decisions](../decisions.md), [Archived March Stack Audit](stack-audit-2026-03.md)

Purpose: record CueLoop's current source-build toolchain baseline, explain the historical Rust 1.95.0 cutover from the stale repo-local 1.94.1 override, capture the release-note compatibility checklist that drives follow-up Rust modernization tasks, and preserve the dependency/security/lint/rustdoc evidence for the current audited baseline.

## Scope

- Rust CLI workspace under `crates/cueloop/`
- macOS SwiftUI app under `apps/CueLoopMac/`
- Local build/test entrypoints in `Makefile`
- Release/versioning surfaces that consume the pinned Rust toolchain

## Current Versions

Audit date: `2026-06-08`

### Languages and Toolchains

- Rust toolchain: `1.96.0` stable (`rust-toolchain.toml`)
- Cargo manifest MSRV floor: `1.96` (`crates/cueloop/Cargo.toml`)
- Rust edition: `2024`
- Xcode: `26.5`
- Swift language mode: `6.3`
- macOS deployment target: `26.0`
- GNU Make: `>= 4`

## Rust 1.96.0 Baseline Update

Audit date: `2026-06-02`

CueLoop now pins Rust `1.96.0` in `rust-toolchain.toml` and declares `rust-version = "1.96"` in the CLI crate manifest. This preserves the active policy that the crate MSRV follows the repository's pinned source-build baseline because local development, release builds, schema generation, and macOS app bundling are validated through the same pinned rustup toolchain.

The update was triggered by `make release-gate` failing `make rust-toolchain-drift-check`: global stable had advanced to Rust `1.96.0` while the repository still pinned `1.95.0`. The release-note compatibility review below records the Rust `1.96.0` changes that matter for CueLoop's current source-build baseline.

## Rust 1.96.0 Release-Note Checklist

Rust 1.96.0 is a source-build baseline bump with no required CueLoop code migration identified in this release pass. Items to keep in mind for future modernization or compatibility reviews:

- Language/compiler: account for `cfg` accepting `expr` metavariables, always-coerced never types in tuple expressions, s390x vector-register inline assembly support, LoongArch link relaxation, and the Fuchsia RISC-V baseline update.
- Libraries/stabilized APIs: review future opportunities around `assert_matches!`, `debug_assert_matches!`, `From<T>` for `AssertUnwindSafe` / `LazyCell` / `LazyLock`, `core::range` iterator types, and iterating over ranges of `NonZero` integers.
- Cargo: note that dependencies may now specify both a git repository and alternate registry, `target.'cfg(..)'.rustdocflags` is supported, and Cargo includes fixes for CVE-2026-5222 and CVE-2026-5223.
- Rustdoc: deprecation notes now render through normal documentation flow, missing-doc-code-examples no longer fires on impl items, and sidebars separate methods from associated functions.
- Compatibility: watch for layout fixes for some `#[repr(Int)]` enums with uninhabited zero-sized fields, stricter `Pin<Foo>` unsizing, wasm target linker-flag changes, `return-position-impl-trait-in-traits` privacy errors, dependency reporting of the deny-by-default `uninhabited_static` lint, typed const-generic checks, removal of `-Csoft-float`, stricter `::{self as name}` imports from structs, repeated `export_name`/`link_name`/`link_section` precedence, external LLVM 21 minimum, and AVR `c_double` mapping to `f32`.

## Rust 1.96.0 Validation Evidence

Audit date: `2026-06-08`

Commands run for the current baseline refresh:

```bash
rustc --version
cargo --version
rustup show active-toolchain
make rust-toolchain-drift-check
cargo install cargo-audit --locked --root target/release-tooling
PATH="$PWD/target/release-tooling/bin:$PATH" make security-audit
make ci && make macos-build && make install
```

Observed toolchain versions:

- `rustc 1.96.0 (ac68faa20 2026-05-25)`
- `cargo 1.96.0 (30a34c682 2026-05-25)`
- active toolchain: `1.96.0-aarch64-apple-darwin` from `rust-toolchain.toml`
- `cargo-audit 0.22.2` installed under `target/release-tooling/bin` for local validation

Outcome:

- Toolchain drift: `make rust-toolchain-drift-check` passed and confirmed `rust-toolchain.toml`, crate `rust-version`, `rustc`, `cargo`, `rustfmt`, `clippy`, and global stable all resolve to Rust `1.96.0`.
- Release gate: `make ci && make macos-build && make install` passed after synchronizing `apps/CueLoopMac/CueLoopCLIInputs.xcfilelist` with committed CLI inputs. The command rebuilt the Rust release CLI, validated schemas/tests, built the macOS release app, and installed `CueLoopMac.app`.
- Security audit: `PATH="$PWD/target/release-tooling/bin:$PATH" make security-audit` passed against `Cargo.lock` after installing `cargo-audit` locally under `target/release-tooling`.

## Historical Rust 1.95.0 Baseline

CueLoop previously pinned Rust `1.95.0` in `rust-toolchain.toml` and declared `rust-version = "1.95"` in the CLI crate manifest. The crate MSRV intentionally follows the repository's pinned source-build baseline because local development, release builds, schema generation, and macOS app bundling are all validated through the same pinned rustup toolchain.

This is a source-build baseline decision, not release-semver metadata. Release version synchronization remains owned by `VERSION` and `scripts/versioning.sh sync`; Rust baseline changes are owned by `rust-toolchain.toml`, crate `rust-version`, and the validation gates documented here.

## Root Cause of the 1.94.1 / 1.95.0 Mismatch

The system global stable toolchain had moved to Rust `1.95.0`, but entering the repository activated the repo-local `rust-toolchain.toml` override pinned to `1.94.1`. Checking only `rustc --version` from inside the repository therefore reported the stale override rather than the global stable toolchain.

Future toolchain audits should compare the global default, the repo-local active override, and a directory outside the override:

```bash
rustup default
rustup show active-toolchain
(cd /tmp && rustc --version && cargo --version)
```

## Rust 1.95.0 Release-Note Checklist

Rust 1.95.0 introduces enough language, library, compiler, rustdoc, and compatibility changes that adoption should be handled through focused follow-up tasks rather than hidden in the baseline bump.

High-level checklist:

- Language: review opportunities and compatibility effects from stabilized `if let` guards on match arms, keyword imports with renaming, PowerPC inline assembly support, pattern-matching semantic updates, and const-promotion/const-eval changes.
- Libraries: evaluate stabilized APIs where they simplify CueLoop code, including `MaybeUninit`/`Cell` array helpers, `bool: TryFrom<{integer}>`, atomic `update`/`try_update`, `cfg_select!`, `core::range`, `core::hint::cold_path`, raw-pointer unchecked reference helpers, `Vec::push_mut`/`insert_mut`, collection `*_mut` insertion helpers, `Layout` helpers, const `fmt::from_fn`, and const `ControlFlow` predicates.
- Compiler/security: account for stabilized `--remap-path-scope`, vendored musl security patches for CVE-2026-6042 and CVE-2026-40200, and the LLVM 22 backend update.
- Platform: note Tier 2 promotions for Apple tvOS/watchOS/visionOS targets and `powerpc64-unknown-linux-musl`.
- Rustdoc: review whether deprecated item hiding and changed unstable search ranking affect generated docs or contributor expectations.
- Compatibility: audit array coercion inference changes, stricter `$crate` self-import errors, rare const-padding errors, the `ambiguous_glob_imported_traits` future-incompatibility warning, stricter lifetime-bound and visibility import checking, `Eq::assert_receiver_is_total_eq` deprecation/future warnings on manual impls, non-exhaustive enum discriminant reads, removal of accidental `mut ref` shorthand allowance, derive-helper/built-in attribute conflict warnings, and JSON target spec gating behind unstable options.

Existing queue follow-ups CL-0051 through CL-0055 covered the modernization and compatibility work planned after the historical Rust 1.95.0 baseline cutover.

## Rust 1.95.0 Dependency, Security, Clippy, and Rustdoc Refresh

Audit date: `2026-04-27`

Commands run for the dependency/security/docs refresh:

```bash
rustc --version
cargo --version
rustup show active-toolchain
cargo audit --version
cargo outdated --root-deps-only --depth 1
make update
cargo outdated --root-deps-only --depth 1
make security-audit
make lint
make docs
make agent-ci
```

Tooling note: `make update` today runs `cargo update` and refreshes `Cargo.lock` to the latest versions compatible with existing `Cargo.toml` requirements; bumping minimum direct dependency versions is still an explicit manifest edit. The audit below used that workflow as written at the time.

Observed toolchain and helper versions:

- `rustc 1.95.0 (59807616e 2026-04-14)`
- `cargo 1.95.0 (f2d3ce0bd 2026-03-21)`
- active toolchain: `1.95.0-aarch64-apple-darwin` from `rust-toolchain.toml`
- `cargo-audit 0.22.1`

Outcome:

- Dependency drift: `make update` bumped direct dependency `clap_complete` from `4.6.2` to `4.6.3`, refreshed `zbus`/`zvariant` transitive dependencies in `Cargo.lock`, and deduplicated the old `winnow 0.7.x` lockfile entry. Phase 3 also removed the direct `atty 0.2` dependency after `cargo audit --deny warnings` reported `RUSTSEC-2024-0375` and `RUSTSEC-2021-0145`; CueLoop now uses the standard-library `std::io::IsTerminal` APIs already required by the Rust 1.95 baseline. After the refresh, `cargo outdated --root-deps-only --depth 1` reported: `All dependencies are up to date, yay!`.
- Security posture: `make security-audit` is the local RustSec advisory gate (`cargo audit --deny warnings`) and passed after the `atty` removal. The compiler-side Rust 1.95.0 context also includes vendored musl patches for CVE-2026-6042 and CVE-2026-40200; CueLoop inherits those through the pinned toolchain rather than a crate-level dependency update.
- Clippy: `make lint` passed with all targets/features and warnings denied.
- Rustdoc: `make docs` passed for workspace docs with all features and no dependencies. Rust 1.95 rustdoc search now ranks unstable items lower and provides a hide-deprecated-items setting; no CueLoop API changes were required, but generated documentation review should account for deprecated-item visibility settings when comparing screenshots/search results across toolchain versions.
- Full local gate: `make agent-ci` passed after the dependency, audit-target, code, and documentation changes.

## Verification

For the current Rust `1.96.0` baseline, use the evidence and commands recorded in [Rust 1.96.0 Validation Evidence](#rust-1960-validation-evidence). For future full stack refreshes, rerun:

```bash
make version-check
make security-audit
make lint
make docs
make agent-ci
```

Because `rust-toolchain.toml` is in the Tier D routing set, expect `make agent-ci` to route to `make macos-ci` on macOS unless the classifier behavior changes. `make security-audit` is intentionally separate from the default day-to-day gate so RustSec advisory database/network/tool availability does not make every local edit depend on external advisory freshness.

## Sources

- Rust `1.96.0` announcement: <https://blog.rust-lang.org/2026/05/28/Rust-1.96.0/>
- Rust `1.96.0` detailed release notes: <https://doc.rust-lang.org/stable/releases.html#version-1960-2026-05-28>
- Rust `1.96.0` release tag: <https://github.com/rust-lang/rust/releases/tag/1.96.0>
- Rust `1.95.0` announcement: <https://blog.rust-lang.org/2026/04/16/Rust-1.95.0/>
- Rust `1.95.0` detailed release notes: <https://doc.rust-lang.org/stable/releases.html#version-1950-2026-04-16>
- Rust `1.95.0` release tag: <https://github.com/rust-lang/rust/releases/tag/1.95.0>
- CI and Test Strategy: [ci-strategy.md](ci-strategy.md)
- Archived March stack audit: [stack-audit-2026-03.md](stack-audit-2026-03.md)
