# Repository Guidelines

## Project Structure & Module Organization
- `bin/`: legacy shell/Python scripts (standalone workflow).
- `specs/`: legacy spec templates used by the scripts.
- `prompt.md`, `prompt_opencode.md`, `supervisor_prompt.md`: prompt templates.
- `README.md`: entry points and cross-repo context (TUI vs legacy).
- `pyproject.toml` + `uv.lock`: Python dependencies for legacy scripts.

## Build, Test, and Development Commands
- `uv sync --dev`: install Python deps for legacy scripts.
- `uv run ruff check --fix`: lint and auto-fix Python.
- `uv run ruff format`: format Python.
- `uv run ty check`: type-check Python.
- Legacy entrypoints (shell): `bin/ralph_loop.sh`, `bin/build_specs.sh`, `bin/validate_pin.sh`, `bin/ralph_unlock.sh`.

## Coding Style & Naming Conventions
- Python 3.11+, typed where possible.
- Format/lint with Ruff; type-check with Astral Ty.
- Shell scripts live in `bin/` and follow existing naming (`snake_case.sh`).
- Prompt/spec files are Markdown; keep them generalized (no project-specific assumptions).

## Testing Guidelines
- No automated tests in this legacy folder today.
- If you add tests, place them under `tests/` and document how to run them.

## Commit & Pull Request Guidelines
- Commit messages are short, sentence-case summaries (e.g., “Move legacy scripts to ralph_legacy/bin”).
- No formal PR template; include a brief summary, commands run, and any prompt/spec changes.

## Configuration & Security
- Prefer a single project-root `.env` if configuration is needed; keep `.env.example` in sync.
- Do not include real secrets if this repo is public.

## Agent Notes
- Legacy workflow lives here; the Go TUI entrypoint is in `ralph_tui/` (see top-level README).
