#!/usr/bin/env bash
set -euo pipefail
cargo test -p cueloop --test agent_ci_surface_contract_test -- --include-ignored >/dev/null
