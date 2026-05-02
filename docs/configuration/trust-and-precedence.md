# Configuration: Trust and Precedence
Status: Active
Owner: Maintainers
Source of truth: this document for CueLoop configuration trust, JSONC, locations, and precedence
Parent: [Configuration](../configuration.md)

Purpose: Document how CueLoop discovers, parses, layers, and safety-gates configuration.

## Overview
CueLoop reads JSON configuration from two locations, with project config taking precedence over global. Use the primary `cueloop` executable:
- Global: `~/.config/cueloop/config.jsonc` (legacy fallback: `~/.config/cueloop/config.jsonc`)
- Project: `.cueloop/config.jsonc` (legacy fallback: `.cueloop/config.jsonc`)

CLI flags override both for a single run. Defaults are defined by `schemas/config.schema.json`.

## Repo execution trust

Project `.cueloop/config.jsonc` may define execution-sensitive settings (for example `agent.*_bin`, project-level Cursor selection, plugin runner IDs, `agent.ci_gate`, `plugins.*`, and `parallel.ignored_file_allowlist`). CueLoop applies those project-layer values only when the repository is explicitly marked trusted via a **local-only** `.cueloop/trust.jsonc` file. Legacy `.cueloop/config.jsonc` and `.cueloop/trust.jsonc` remain supported for repos that have not moved yet; legacy `.cueloop/trust.json` is ignored. `trusted_at` is optional in the file; `allow_project_commands: true` is what marks the repo trusted.

**Supported ways to create the trust file (explicit opt-in):**

- **`cueloop init`** — Preferred repository bootstrap. Creates or updates `.cueloop/trust.jsonc` by default for new repos, uses `.cueloop/trust.jsonc` for legacy repos, resolves initialization without enforcing trust before the file exists, and adds the trust file to `.gitignore`.
- **`cueloop config trust init`** — Trust-only repair for already-initialized repos. Creates the active runtime directory if needed, then creates or merges `trust.jsonc` with `allow_project_commands: true` and a `trusted_at` RFC3339 UTC timestamp when the file is missing. If the file already marks the repo trusted (both flags set), the command leaves the file byte-for-byte unchanged. If `allow_project_commands` is true but `trusted_at` is absent, the file is updated to add a timestamp.

CueLoop prints a short warning before writing or changing the trust file. **Do not commit** `.cueloop/trust.jsonc` or legacy `.cueloop/trust.jsonc`; keep it untracked (see repository `AGENTS.md`).

Manual example:

```jsonc
{
  "allow_project_commands": true,
  "trusted_at": "2026-04-19T00:00:00Z"
}
```

## JSONC Support (JSON with Comments)

CueLoop supports JSONC (JSON with Comments) for configuration and queue files. This allows you to add comments to your config and task files for better documentation.

### Supported Comment Styles
- Single-line comments: `// This is a comment`
- Multi-line comments: `/* This is a multi-line comment */`
- Trailing commas in objects and arrays

### File Extensions
- `.jsonc` - JSON with Comments support for runtime config and queue files
- `.json` - Standard JSON used only where a strict JSON contract is required, such as schemas

When writing files, CueLoop always outputs standard JSON format (comments are not preserved on rewrite).

### Example JSONC Config

```jsonc
{
  // Schema version - must be 2
  "version": 2,
  "agent": {
    /* Runner configuration.
       Built-in runner IDs: codex, opencode, gemini, claude, cursor, kimi, pi.
       Plugin runner IDs are also supported as non-empty strings. */
    "runner": "codex",
    "model": "gpt-5.4",
    "phases": 3, // 1 = single-pass, 2 = plan+implement, 3 = plan+implement+review
  }
}
```

### Notes
- Schema files (`schemas/*.schema.json`) remain strict JSON for validator compatibility
- Comments are for human editing only; CueLoop outputs standard JSON when saving

For the field index and chapter map, start at [Configuration](../configuration.md#top-level-fields).

## Precedence
1. CLI flags (single run)
2. Project config (`.cueloop/config.jsonc`, with legacy `.cueloop/config.jsonc` fallback)
3. Global config (`~/.config/cueloop/config.jsonc`, with legacy `~/.config/cueloop/config.jsonc` fallback)
4. Schema defaults (`schemas/config.schema.json`)

## App Safety Warnings (macOS)

When editing configuration in the macOS app, certain high-risk settings display inline warnings:

- **Danger level** (⚠): Settings like `git_publish_mode` that can cause irreversible actions. The app prompts for confirmation before enabling these.
- **Warning level** (ℹ): Settings like `approval_mode` and `claude_permission_mode` that reduce safety checks. These show descriptive text but don't require confirmation.

The confirmation dialog for Danger-level settings explains the risk and requires an explicit confirmation to proceed.
