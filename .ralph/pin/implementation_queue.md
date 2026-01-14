# Implementation Queue

## Queue
- [ ] RQ-0100 [code]: Seed TUI queue item. (ralph_tui/internal/pin/pin.go)
  - Evidence: Default queue item for new repos.
  - Plan: Replace with project-specific work.
- [ ] RQ-0200 [ui][code]: Fix loop runner prompt piping + stop behavior; make context cancel kill subprocesses. (ralph_tui/internal/loop/exec.go, ralph_tui/internal/loop/runner.go, ralph_tui/internal/loop/loop.go, ralph_tui/internal/tui/loop_view.go)
  - Evidence: RunCommand overwrites cmd.Stdin, breaking prompt file input; loop uses exec.Command not CommandContext so stop cannot work.
  - Plan: Preserve cmd.Stdin unless nil; thread ctx into RunnerInvoker and use exec.CommandContext; add loopStopping state so UI cannot start overlapping runs; add unit tests for prompt piping and stop.
- [ ] RQ-0201 [ui]: Fix TUI refresh timer duplication and async start/load races. (ralph_tui/internal/tui/model.go, ralph_tui/internal/tui/pin_view.go, ralph_tui/internal/tui/specs_view.go)
  - Evidence: config reload schedules refreshCmd in addition to existing tick chain; reloadAsync/refreshPreviewAsync use tea.Batch with unordered start/done messages.
  - Plan: Implement refresh generation token; schedule refresh only in one place; set loading flags synchronously or use tea.Sequence; add tests for "loading not stuck".
- [ ] RQ-0205 [ui][code]: Implement debug log file and Logs screen; include key events, resize events, errors, and runner start/stop. (ralph_tui/internal/tui/model.go, ralph_tui/internal/tui/screens.go, ralph_tui/internal/tui/*)
  - Evidence: No log file exists; Logs screen is placeholder; agents cannot see runtime state.
  - Plan: Add file logger under cache dir; write structured lines; implement Logs screen tail viewer; add a config knob for log level and file path.

## Blocked

## Parking Lot
- [ ] RQ-0101 [docs]: Placeholder parking lot item. (README.md)
  - Evidence: Example parking lot entry.
  - Plan: Update or remove in real use.
