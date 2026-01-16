# Implementation Queue

## Queue

- [ ] RQ-0478 [code]: Config/path resolution + error reporting fixes (stop silently ignoring path resolution errors; surface actionable messages in UI). (ralph_tui/internal/config/load.go, ralph_tui/internal/config/config.go, ralph_tui/internal/tui/config_editor.go)
  - Evidence: `resolveConfigPaths` silently ignores `resolvePathWithRepo` errors (`config/load.go`), which can leave invalid (relative) paths that later fail `Config.Validate()` with less actionable messages. Config editor field-source refresh also ignores errors, hiding why values can’t be resolved.
  - Plan: Propagate path resolution errors with contextual messages (which field/path failed and why), surface them in the config editor status line, and add unit tests for `{repo}` expansion and relative-root save/load edge cases.

- [ ] RQ-0479 [ops]: Reduce refresh/jitter and background workload to address lag (adaptive refresh, debounce preview rendering, avoid heavy work when screen inactive). (ralph_tui/internal/tui/model.go, ralph_tui/internal/tui/specs_view.go, ralph_tui/internal/tui/repo_status.go)
  - Evidence: `refreshCmd` ticks frequently and triggers repo status sampling + view refresh checks even when screens are inactive (`model.refreshViews`). Specs preview rendering (glamour) can be expensive and is re-triggered on many resizes (`specs_view.Resize` sets `previewDirty=true`), contributing to a laggy experience.
  - Plan: Make refresh adaptive: only run heavy refresh logic when the relevant screen is visible, debounce preview rendering on rapid resize, and add lightweight timing logs at debug level to identify hotspots. Keep a manual "refresh now" as an escape hatch.

## Blocked

## Parking Lot
