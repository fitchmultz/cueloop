# Implementation Queue

## Queue
- [ ] RQ-0305 [code]: Remove busy tick; add log batching with run scoping for loop output. (ralph_tui/internal/tui/loop_view.go, ralph_tui/internal/tui/stream_writer.go)
  - Evidence: loopView tickCmd wakes every 500ms while running; loop log channel drops lines when buffer is full; no run ID to ignore stale messages if a new run starts.
  - Plan: Replace tickCmd with log-only updates; introduce a batched log message helper with run IDs; drain log channel into batches to reduce UI churn; ignore stale run batches; add loop_view_log_batch_test.
- [ ] RQ-0306 [code]: Coalesce preview refreshes and batch specs run output. (ralph_tui/internal/tui/specs_view.go, ralph_tui/internal/tui/stream_writer.go)
  - Evidence: refreshPreviewAsync can be triggered while previewLoading is true; rapid toggles can overlap goroutines; run output updates rebuild the log viewport for every line.
  - Plan: Gate preview refresh if loading and set previewDirty for a follow-up pass; add a single-flight refresh with queued rerender; batch run log writes via streamWriter batching; add specs_view_preview_queue_test.
- [ ] RQ-0307 [ui]: Expose runner/args/effort for Loop and Specs; parse args lines. (ralph_tui/internal/tui/loop_view.go, ralph_tui/internal/tui/specs_view.go, ralph_tui/internal/tui/config_editor.go, ralph_tui/internal/config/config.go)
  - Evidence: Loop runner is hard-coded to "codex" with empty args; Specs view defaults to codex with empty args; no UI for setting runner args or reasoning effort.
  - Plan: Add config-backed settings for runner, args (one per line), and reasoning effort; add parse/format helpers in TUI; wire through loop/specs builders; update settings help text.
- [ ] RQ-0308 [ui]: Make Logs screen readable and optionally raw. (ralph_tui/internal/tui/logs_view.go)
  - Evidence: Logs format toggle exists but renderContent always returns raw JSONL; no formatted view is applied to debug/loop/specs sections.
  - Plan: Parse JSONL entries into concise, human-readable lines; keep raw/formatted toggle; preserve status line with resolved log path.

## Blocked

## Parking Lot
