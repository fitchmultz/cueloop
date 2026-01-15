// Package pin defines default content for Ralph pin files.
package pin

const defaultQueueContent = `# Implementation Queue

## Queue
- [ ] RQ-0001 [ops]: Bootstrap the initial queue for this repo. (AGENTS.md, .ralph/pin/implementation_queue.md)
  - Evidence: Fresh repos need a seed queue item so pin validation passes.
  - Plan: Replace this placeholder with real, evidence-backed queue items.

## Blocked

## Parking Lot
`

const defaultDoneContent = `# Implementation Done

## Done
`

const defaultLookupContent = `# Lookup Table

| Area | Notes |
| --- | --- |
| pin | Default pin fixtures for the Ralph TUI/CLI. |
`

const defaultReadmeContent = `# Ralph Pin Files

These pin files drive the Ralph TUI/CLI workflow. If you are in a fresh repo, run:

  ralph init

The pin directory should include:
- implementation_queue.md
- implementation_done.md
- lookup_table.md
- specs_builder.md

## Queue item metadata
Queue items require ` + "`Evidence`" + ` and ` + "`Plan`" + ` bullets. You may add extra metadata after those bullets using
indented notes/links or an indented YAML block. Keep extra metadata indented by two spaces so it stays
inside the queue item block.

Example:

  - [ ] RQ-1234 [code]: Add richer queue metadata support. (ralph_tui/internal/pin/pin.go)
    - Evidence: Users want extra context without breaking parsing.
    - Plan: Support indented Notes/Links and a YAML metadata block.
    - Notes: Optional extra context.
      - Link: https://example.com/spec
    ` + "```yaml" + `
    owner: ralph-team
    severity: medium
    links:
      - https://example.com/spec
    ` + "```" + `
`

const defaultSpecsBuilderContent = `# MISSION
You are the Ralph specs builder for this repository.

# CONTEXT (READ IN ORDER)
1. ` + "`AGENTS.md`" + `
2. ` + "`.ralph/pin/README.md`" + `
3. ` + "`.ralph/pin/implementation_queue.md`" + `
4. ` + "`.ralph/pin/lookup_table.md`" + `

{{INTERACTIVE_INSTRUCTIONS}}
{{INNOVATE_INSTRUCTIONS}}
{{SCOUT_WORKFLOW}}

# INSTRUCTIONS
- Update ` + "`.ralph/pin/implementation_queue.md`" + ` with actionable items.
- Keep queue items in the required format: ID, routing tag(s), title, scope list, ` + "`Evidence`" + `, and ` + "`Plan`" + `.
- Optional extra metadata is allowed after ` + "`Plan`" + ` using indented Notes/Links bullets or an indented ` + "```yaml" + ` block (see ` + "`.ralph/pin/README.md`" + `).
- Add/update ` + "`.ralph/pin/lookup_table.md`" + ` entries when new areas appear.
- If the queue is empty, seed it with pragmatic, evidence-based work items.

# OUTPUT
Provide a brief summary of what changed.
`
