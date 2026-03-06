# Local Smoke Test (5-10 minutes)

Purpose: provide a deterministic install and verification path without requiring external runner setup.

## Preconditions

- Fresh clone of repository
- Rust toolchain + GNU Make available

## Steps

```bash
# from repo root
make install

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

## Expected Results

- help commands succeed
- queue validation/list commands succeed
- `ralph doctor` completes without critical failures in repo root
- `make agent-ci` passes (Rust/CLI gate; app gate only if app paths changed)

## Troubleshooting

- GNU Make mismatch: use `gmake` on macOS Homebrew setups
- env/runtime artifact failures: run `make pre-public-check` for detailed diagnostics
- additional help: [docs/troubleshooting.md](../troubleshooting.md)
