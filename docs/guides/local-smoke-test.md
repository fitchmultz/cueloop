# Local Smoke Test (5-10 minutes)

Purpose: provide a deterministic install and verification path without requiring external runner setup.

## Preconditions

- Fresh clone of repository
- Rust toolchain + GNU Make available

## Steps

```bash
# from repo root
make install
# macOS/Homebrew GNU Make users: gmake install

# initialize repo-local runtime state (safe to rerun)
ralph init

# verify command surface
ralph --help
ralph run one --help
ralph scan --help

# verify local repo state + diagnostics
ralph queue validate
ralph queue list
ralph doctor

# required quality gate
make agent-ci
```

If you want a shorter reviewer-oriented version of this flow, use [evaluator-path.md](evaluator-path.md).

## Expected Results

- help commands succeed
- queue validation/list commands succeed
- `ralph doctor` completes without critical failures in repo root
- `make agent-ci` passes (Rust/CLI gate; macOS app gate auto-runs when the dependency surface can affect the app bundle)

## Troubleshooting

- GNU Make mismatch: use `gmake` on macOS Homebrew setups
- env/runtime artifact failures: run `make pre-public-check` for detailed diagnostics
- additional help: [docs/troubleshooting.md](../troubleshooting.md)
