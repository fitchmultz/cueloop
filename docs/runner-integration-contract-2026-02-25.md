# Runner Integration Contract (2026-02-25)

Purpose: define the observed command/runtime contract for Ralph's built-in runners (`opencode`, `claude`, `pi`, `codex`, `kimi`, `gemini`) based on real CLI probes and source inspection.

Scope:
- Covers non-interactive start/resume shapes, stream format expectations, session behavior, and failure semantics.
- Covers how Ralph should parse output and manage session IDs reliably across CLIs.

Out of scope:
- User-level setup docs for each external CLI.
- Full runner UX docs (see `docs/features/runners.md`).

Evidence bundle:
- `/tmp/ralph-runner-contracts-20260225T162522Z`

## 1) Canonical Ralph Expectations

Ralph's execution layer should treat these as invariants across runners:

1. Process capture is authoritative: retain raw `stdout`, raw `stderr`, and exit status for all invocations.
2. Success/failure must primarily follow process exit code, not presence/absence of JSON parseable lines.
3. Session extraction must tolerate runner-specific field names and envelope styles.
4. Resume flow must support both:
   - explicit session resume when caller/session state provides an ID, and
   - fallback fresh invocation when resume is not possible or deterministically fails.

## 2) Per-Runner Contract Snapshot

| Runner | Start shape (non-interactive) | Resume shape | Stream format for parsing | Session behavior | Failure behavior to preserve |
|---|---|---|---|---|---|
| `codex` | `codex exec --json ...` | `codex exec resume <ID> --json ...` (or `--last`) | JSONL events on stdout | Emits resumable session/thread identity in stream events | Invalid resume/session returns non-zero; parse payload can still be partially valid |
| `opencode` | `opencode run --format json ...` | `opencode run --session <ID> ...` or `--continue` | JSON-capable output mode, but ancillary logs can appear | Session IDs expected to follow opencode conventions (`ses*`) | Invalid session can print validation errors on stderr even when exit code is `0` |
| `claude` | `claude -p --output-format stream-json ...` | `claude -p --resume <ID> --output-format stream-json ...` | `stream-json` on stdout | Resume responses may include `session_id` in error or success records | Invalid resume returns non-zero and emits structured JSON error object |
| `gemini` | `gemini --output-format stream-json ...` | `gemini --resume <ID> --output-format stream-json ...` | `stream-json` on stdout | Resume supports IDs and guided recovery via session listing | Invalid resume exits `42` with actionable stderr guidance |
| `kimi` | `kimi --print --output-format stream-json --session <ID> ...` | same shape with existing `--session` | `stream-json` on stdout | Works best with caller-managed session IDs; Ralph should always provide one | Empty/invalid `--session` usage can exit `2` (usage/validation) |
| `pi` | `pi ...` (runner-specific args from plugin) | `pi --session <ID> ...` | Runner stream output can vary; parser must remain tolerant | Resume can fail if session store has no matching entry | Missing session emits `No session found matching ...` and exits non-zero |

## 3) Session ID Extraction Contract

Current extraction keys in Ralph are directionally correct and should remain supported:

- `thread_id`
- `session_id`
- `sessionID`
- session envelope style: `{ "type": "session", "id": "..." }`

Contract additions:

1. Session extraction must scan both success and structured error envelopes when present.
2. Session extraction should be best-effort and never alone decide success/failure.
3. If a runner is configured as managed-session (for example Kimi), missing extracted session ID should not overwrite an existing known session ID.

## 4) Resume/Fallback Lifecycle Contract

Required behavior for run supervision:

1. Attempt same-session resume when a prior session ID exists.
2. If resume deterministically fails with known "session missing/invalid" class errors, run fresh.
3. Persist the post-attempt session identity:
   - resumed session ID if provided,
   - otherwise newly generated/managed session ID,
   - otherwise keep prior known ID (do not clear blindly).
4. Emit structured debug logs that distinguish:
   - resume attempted,
   - resume failed (reason class),
   - fallback fresh invocation executed.

## 5) Risk Register and Required Hardening

1. `opencode` invalid-session stderr with exit `0` can look superficially successful.
   - Mitigation: classify invocation as failed when stderr matches known fatal validation patterns and no assistant completion signal is observed.
2. Structured error-on-stdout runners (`claude` stream-json mode) can be misread as normal events.
   - Mitigation: parser should tag known error event shapes and surface them distinctly.
3. Exit-code diversity (`gemini` uses `42` for invalid resume) should not be collapsed to generic messaging.
   - Mitigation: retain exact status in `RunnerOutput` and include mapped diagnostic hints.
4. Managed-session runners (`kimi`) can drift if caller accidentally omits/rotates IDs.
   - Mitigation: keep `requires_managed_session_id` semantics strict and test phase-scoped ID propagation.

## 6) Implementation Plan (Ralph)

1. Add runner-contract parsing tests using captured event/error samples for all six runners.
2. Expand failure classification in execution response handling:
   - separate `process_failed`, `stream_error_event`, and `semantic_stderr_failure` buckets.
3. Add targeted resume/fallback tests:
   - `pi` missing session -> fallback fresh,
   - `gemini` invalid resume (exit `42`) -> fallback/diagnostic path,
   - `claude` stream-json error object -> classified resume failure.
4. Add `opencode` stderr-pattern classification test proving fatal validation output is not reported as success even with exit `0`.
5. Update docs in `docs/features/session-management.md` and `docs/features/runners.md` once behavior changes land.

## 7) Test Matrix to Add

- `runner/execution/tests/parsing.rs`
  - Validate mixed valid JSON + structured error lines for each runner format.
- `runner/execution/tests/response.rs`
  - Validate outcome classification when exit code and output semantics disagree.
- `commands/run/supervision/*` tests
  - Validate resume failure classification and fallback-to-fresh behavior.

## 8) Acceptance Criteria

1. All six runners have deterministic tests for start and invalid-resume paths.
2. Resume-fallback behavior is explicit, logged, and covered by tests.
3. Session ID persistence is stable under success, structured error, and fallback cases.
4. `make agent-ci` passes after changes.
