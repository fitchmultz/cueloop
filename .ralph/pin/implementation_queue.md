# Implementation Queue

## Queue
- [ ] RQ-0466 [bug][ui]: Execute fixup when pressing `f` and capture failures. (ralph_tui/internal/tui/dashboard_view.go, ralph_tui/internal/tui/model.go, ralph_tui/internal/loop/fixup.go)
  - Evidence: User reports "Fixup: Scanned 1 | Eligible 1 | Requeued 0 | Skipped 0 | Failed 1" and pressing `f` only identifies an item without running the fixup.
  - Plan: Trace the dashboard fixup action, ensure `f` triggers the actual fixup execution path, and surface execution/failure details in logs/status with tests.
- [ ] RQ-0467 [docs][ux]: Add project-type aware bug-sweep prompt entry for spec/queue builder. (ralph_tui/internal/prompts/defaults, ralph_tui/internal/specs, ralph_tui/internal/pin, .ralph/pin)
  - Evidence: Need a standardized prompt that adapts by project type and batches 15+ findings into queue tasks, populating `.ralph/pin/implementation_queue.md` and `.ralph/pin/lookup_table.md`.
  - Plan: Add a dedicated prompt template for spec/queue builder mode that inserts the project type, ensure it targets the pin queue format, and document usage in pin/specs builder guidance.
## Blocked

## Parking Lot
