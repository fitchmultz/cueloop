#!/usr/bin/env bash
#
# Purpose: Ship-gate profiling orchestration for Ralph CI.
# Responsibilities:
#   - Run timed CI/build/test stages and capture per-stage durations.
#   - Write a profiling bundle (timings.tsv, summary.md, JSONL artifacts).
#   - Clean profiling bundles on request.
# Scope:
#   - Orchestration only; actual CI/build/test targets live in the Makefile.
# Usage:
#   scripts/profile-ship-gate.sh run     # capture a profiling bundle
#   scripts/profile-ship-gate.sh clean   # remove all profiling bundles
#   scripts/profile-ship-gate.sh -h
# Invariants/Assumptions:
#   - Must be run from the repo root where the Makefile lives.
#   - MAKE and RALPH_ENV_RESET are injected by the Makefile wrapper.
#   - RALPH_CI_JOBS and RALPH_XCODE_JOBS are propagated through make.

set -euo pipefail

readonly PROFILING_ROOT="target/profiling"
MAKE="${MAKE:-make}"
RALPH_ENV_RESET="${RALPH_ENV_RESET:-:}"

# ---------------------------------------------------------------------------
# Help
# ---------------------------------------------------------------------------

usage() {
    cat <<'EOF'
Usage: scripts/profile-ship-gate.sh <command>

Commands:
  run    Capture a canonical ship-gate profiling bundle under target/profiling/.
  clean  Remove all ship-gate profiling bundles.

Options:
  -h, --help  Show this help message and exit.

Environment:
  MAKE               Make binary (default: make). Injected by Makefile wrapper.
  RALPH_ENV_RESET    Shell prefix to activate the pinned Rust toolchain.
  RALPH_CI_JOBS      Cap parallel jobs for Cargo/nextest (0 = tool default).
  RALPH_XCODE_JOBS   Cap parallel jobs for xcodebuild (0 = tool default).

Examples:
  scripts/profile-ship-gate.sh run
  scripts/profile-ship-gate.sh clean
  RALPH_CI_JOBS=4 scripts/profile-ship-gate.sh run

Exit codes:
  0  Success (all stages passed, or clean completed).
  1  One or more profiling stages failed (summary still written).
EOF
}

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

run_timed_shell() {
    local label="$1"
    local command="$2"
    local start end duration status

    start="$(date +%s)"
    set +e
    bash -c "$command"
    status="$?"
    set -e
    end="$(date +%s)"
    duration="$((end - start))"

    printf '%s\t%s\t%s\n' "$label" "$duration" "$status" >> "$timings_path"
    return "$status"
}

write_summary() {
    {
        echo '# Ship-gate profiling baseline'
        echo
        echo "- date: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
        echo "- profile_dir: ${profile_dir}"
        echo '- retention: timestamped bundles are retained until explicit cleanup'
        echo '- cleanup: make profile-ship-gate-clean'
        echo
        echo '## Environment'
        echo
        echo "- uname: $(uname -a)"
        echo "- xcodebuild: $(xcodebuild -version 2>/dev/null | tr '\n' ' ' | sed 's/  */ /g' || echo unavailable)"
        echo "- RALPH_CI_JOBS: ${RALPH_CI_JOBS:-0}"
        echo "- RALPH_XCODE_JOBS: ${RALPH_XCODE_JOBS:-0}"
        echo
        echo '## Timings'
        echo
        awk 'NR == 1 { next } { printf "- %s: %ss (exit %s)\n", $1, $2, $3 }' "$timings_path"
        echo
        echo '## Slowest surfaces'
        echo
        tail -n +2 "$timings_path" | sort -k2,2nr | head -3 | awk '{ printf "- %s: %ss\n", $1, $2 }'
    } > "$summary_path"
}

# ---------------------------------------------------------------------------
# Commands
# ---------------------------------------------------------------------------

cmd_run() {
    local timestamp profile_dir timings_path summary_path

    timestamp="$(date +%Y%m%d-%H%M%S)"
    profile_dir="${PROFILING_ROOT}/${timestamp}-ship-gate"
    timings_path="${profile_dir}/timings.tsv"
    summary_path="${profile_dir}/summary.md"

    mkdir -p "$profile_dir"
    printf 'label\tseconds\tstatus\n' > "$timings_path"

    echo "→ Capturing ship-gate profiling bundle under ${profile_dir}..."

    run_timed_shell ci "${MAKE} --no-print-directory ci" \
        || { write_summary; exit 1; }
    run_timed_shell nextest_run_parallel_test "${RALPH_ENV_RESET}; NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 cargo nextest run --workspace --locked --test run_parallel_test --show-progress none --status-level none --final-status-level none --message-format libtest-json-plus > '${profile_dir}/nextest.run_parallel_test.jsonl'" \
        || { write_summary; exit 1; }
    run_timed_shell nextest_parallel_direct_push_test "${RALPH_ENV_RESET}; NEXTEST_EXPERIMENTAL_LIBTEST_JSON=1 cargo nextest run --workspace --locked --test parallel_direct_push_test --show-progress none --status-level none --final-status-level none --message-format libtest-json-plus > '${profile_dir}/nextest.parallel_direct_push_test.jsonl'" \
        || { write_summary; exit 1; }
    run_timed_shell macos_build "${MAKE} --no-print-directory macos-build" \
        || { write_summary; exit 1; }
    run_timed_shell macos_test "${MAKE} --no-print-directory macos-test" \
        || { write_summary; exit 1; }
    run_timed_shell macos_test_contracts "${MAKE} --no-print-directory macos-test-contracts" \
        || { write_summary; exit 1; }

    write_summary
    echo "  ✓ Profiling bundle: ${profile_dir}"
    echo "  ✓ Summary: ${summary_path}"
    echo "  ℹ Retained until: make profile-ship-gate-clean"
}

cmd_clean() {
    echo "→ Removing ship-gate profiling bundles..."
    rm -rf "$PROFILING_ROOT"
    echo "  ✓ Ship-gate profiling bundles removed"
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

case "${1:-}" in
    run)   cmd_run ;;
    clean) cmd_clean ;;
    -h|--help|help) usage; exit 0 ;;
    *)     usage; exit 1 ;;
esac
