# Repository Guidelines

## Project Structure & Module Organization
- `cmd/ralph/`: Cobra-based CLI/TUI entrypoint (`go run ./cmd/ralph`).
- `internal/`: Core application packages (`config`, `loop`, `pin`, `prompts`, `specs`, `tui`, etc.).
- `ralph_legacy/`: Legacy scripts and prompt/spec templates (`bin/`, `specs/`), not required for the TUI.
- `.ralph/`: Runtime/config defaults and pin files (`.ralph/ralph.json`, `.ralph/pin/`); cache defaults to `~/.ralph/cache/{repo}` unless overridden.
- Tests live alongside code as `*_test.go` (no separate test directory).

## Build, Test, and Development Commands
- `go run ./cmd/ralph`: Run the CLI/TUI locally.
- `go run ./cmd/ralph --help`: List commands and flags.
- `go build ./cmd/ralph`: Build the CLI binary.
- `go test ./...`: Run all Go unit tests.
- `go test ./... -run TestName`: Run a single test by name.
- `make install`: Install Python deps via `uv` and download Go modules.
- `make build`: Build the Go CLI binary.
- `make test`: Run Go tests; runs Python tests if any exist under `ralph_legacy/`.
- `make format`: Format Python (Ruff) and Go (`gofmt`).
- `make lint`: Lint Python (Ruff) and Go (`go vet`).
- `make type-check`: Run Python type checks (Astral Ty) and a no-op Go test pass for type safety.
- `make generate`: Generate API types when OpenAPI is involved.
- `make ci`: Local gate; runs generate/format/type-check/lint/build/test.

## Coding Style & Naming Conventions
- Go version is pinned in `go.mod`.
- Use standard Go formatting: `go fmt ./...` (or `gofmt -w` for specific files).
- Keep packages lower-case, concise, and aligned to their domain (e.g., `internal/loop`).
- File names are lower_snake_case; tests use `*_test.go`.
- Python (legacy scripts) should be typed where practical and formatted/linted with Ruff.

## Testing Guidelines
- Go tests use the standard `testing` package and live alongside code as `*_test.go`.
- Prefer table-driven tests for multiple scenarios.
- Legacy Python tests (if added) live under `ralph_legacy/tests/` and run with `pytest`.
- Run `go vet ./...` alongside tests for baseline static checks.

## Commit & Pull Request Guidelines
- Commit messages often include queue IDs: `RQ-####: <short summary>`.
- If work is not tied to a queue item, use a short imperative summary.
- PRs should include a clear summary, the commands run (especially `make ci` or `go test ./...`), and notes on TUI behavior changes.
- Add screenshots or recordings when TUI UI behavior changes.

## Configuration & Runtime Files
- Local repo config is stored in `.ralph/ralph.json`; pins live in `.ralph/pin/`.
- Use a single project-root `.env` if configuration is needed; keep `.env.example` in sync.
- Use `ralph migrate` to move legacy pin files into the `.ralph/pin` layout when needed.

## Agent-Specific Instructions
- Keep `AGENTS.md` concise, accurate, and up to date when repo guidance changes.
- All Python files must include a top-level docstring describing purpose.
- Executable scripts must provide a useful `--help` with examples.
- Prefer centralization and consistency: if the same issue appears elsewhere, fix it everywhere and refactor into shared helpers.
- Use TODOs only in the planning thread/tooling, not in repo files.
