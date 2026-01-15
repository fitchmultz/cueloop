# Implementation Queue

## Queue
- [ ] RQ-0437 [code]: Support richer queue item metadata (notes/links/extra context) without breaking pin validation or loop parsing; optionally move to a structured format. (ralph_tui/internal/pin/pin.go, ralph_tui/internal/loop/queue.go, .ralph/pin/README.md)
  - Evidence:
    - `pin.ValidatePin()` enforces a strict queue item header format + requires `Evidence` and `Plan`, but there is no supported place for additional structured notes; users report the system "freaks out" when they add extra detail.
    - The loop runner and TUI both treat the queue file as opaque markdown blocks, so adding richer metadata today risks parsing/validation surprises.
  - Plan:
    - Define and document an explicit "extra metadata" convention (e.g., optional `Notes:` field, or a fenced YAML block) that is ignored by the loop selector but validated as safe by `pin.ValidatePin()`.
    - Update pin parsing/validation to allow and preserve the metadata, and update the prompts to encourage using the supported format.
    - Add tests that a queue item containing extra metadata still passes validation and is still selectable/runnable by the loop.
- [ ] RQ-0438 [ui]: Add a safe "edit queue" + "commit pin changes" workflow in the TUI (reduce manual git mistakes and the need to stop the loop to edit files). (ralph_tui/internal/tui/pin_view.go, ralph_tui/internal/tui/model.go, ralph_tui/internal/loop/git.go, ralph_tui/internal/loop/loop.go)
  - Evidence:
    - Today, editing `.ralph/pin/implementation_queue.md` happens outside Ralph; if the user forgets to commit before starting the loop, the loop can treat the repo as dirty and quarantine/reset, losing the edits.
    - The TUI provides pin operations (toggle, move checked, block) but does not offer a first-class way to open the queue in $EDITOR or to commit pin-only changes safely.
  - Plan:
    - Add a Pin-screen action to open the queue file in the user’s editor (respecting `$EDITOR`) and then reload/validate pin on return.
    - Add an option to auto-commit only pin files (queue/done/lookup/specs_builder) with a safe commit message, and wire this into the loop preflight so pin-only dirtiness is handled gracefully.
    - Add tests for the new TUI commands and ensure they don’t run while a loop iteration is actively running.
- [ ] RQ-0439 [code]: Fix reasoning_effort "auto" semantics and policy block accuracy (align what we display vs what we actually pass to codex; make P1 behavior explicit). (ralph_tui/internal/loop/loop.go, ralph_tui/internal/runnerargs/effort.go, ralph_tui/internal/tui/loop_view.go)
  - Evidence:
    - `loop.Run()` computes an `effectiveEffort` (including `[P1] => high`) and prints the "CODEX CONTEXT BUILDER POLICY" block based on it, but `runnerargs.ApplyReasoningEffort(..., "auto")` may inject no args—so the policy block can claim an effort that isn’t actually applied.
    - The Run Loop UI displays "effective" effort using a different path (`runnerargs.ApplyReasoningEffort`), so the UI and the prompt policy can disagree and confuse both users and agents.
  - Plan:
    - Use a single source of truth for "effective effort" based on the final runner args (post-merge + post-injection) and reuse it for both UI display and prompt policy output.
    - Decide on and implement the desired auto behavior for `[P1]` items (either actually inject `high` or show "auto (target: high)" consistently everywhere).
    - Add regression tests for policy block output and the P1 auto-effort behavior so we never misreport the active reasoning mode again.

## Blocked

## Parking Lot
