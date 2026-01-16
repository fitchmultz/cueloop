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
- specs_builder_docs.md (docs projects)

## Queue IDs
To allocate the next queue ID, run:

  ralph pin next-id

This scans the queue and done logs and prints the next available RQ-####.

If duplicates slip in, repair them with:

  ralph pin fix-ids

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

# INSTRUCTIONS
- The code is riddled with bugs and the user experience is poor. There are at least 20 bugs present that need identified and squashed. Identify 15+ (no upper limit) bugs/issues/flaws/etc, and batch the individual findings into remediation tasks. 
- Some items to look for: laggy interfaces, limited or incomplete functionality, logical design flaws and oversights, lack of standardization, violation of DRY principals, functionality that outright don't work, etc. This list is not comprehensive. 
- When you have your batches of tasks, add them to the ` + "`.ralph/pin/implementation_queue.md`" + ` queue file according to the required spec queue formatting. Each task in the queue (each batch of findings) will be executed sequentially by an agent. Feel free to innovate, refactor, redo things, reorganize, etc. Do NOT be afraid of large scale changes if they are required to move the project in the correct direction.
- Add the highest priority items to the top of the task queue.
- Use unique task IDs (e.g. RQ-1234) for each task. Use ` + "`ralph pin next-id`" + ` to get the next available ID (it scans queue + done).
- Keep queue items in the required format: ID, routing tag(s), title, scope list, ` + "`Evidence`" + `, and ` + "`Plan`" + `. Keep extra metadata indented by two spaces so it stays inside the queue item block.
- Optional extra metadata is allowed after ` + "`Plan`" + ` using indented Notes/Links bullets or an indented ` + "```yaml" + ` block (see ` + "`.ralph/pin/README.md`" + `).
- Add/update ` + "`.ralph/pin/lookup_table.md`" + ` entries when new areas appear and it is incomplete.

{{INTERACTIVE_INSTRUCTIONS}}
{{INNOVATE_INSTRUCTIONS}}
{{SCOUT_WORKFLOW}}

# OUTPUT
Provide a brief summary of what changed.
`

const defaultSpecsBuilderDocsContent = `# MISSION
You are the Ralph specs builder for this repository (documentation-focused).

# CONTEXT (READ IN ORDER)
1. ` + "`AGENTS.md`" + `
2. ` + "`.ralph/pin/README.md`" + `
3. ` + "`.ralph/pin/implementation_queue.md`" + `
4. ` + "`.ralph/pin/lookup_table.md`" + `

# INSTRUCTIONS
- This repo is documentation-first. Focus on doc maintenance, link checks, research synthesis, and content structure.
- Identify at least 15 doc-centric issues: broken/obsolete links, outdated or missing references, unclear sections, missing cross-links, inconsistent terminology, or weak navigation.
- If code changes are required to support docs workflows, include them only when the documentation goal depends on it.
- When you have your batches of tasks, add them to the ` + "`.ralph/pin/implementation_queue.md`" + ` queue file according to the required spec queue formatting. Each task in the queue (each batch of findings) will be executed sequentially by an agent.
- Add the highest priority items to the top of the task queue.
- Use unique task IDs (e.g. RQ-1234) for each task. Use ` + "`ralph pin next-id`" + ` to get the next available ID (it scans queue + done).
- Keep queue items in the required format: ID, routing tag(s), title, scope list, ` + "`Evidence`" + `, and ` + "`Plan`" + `. Keep extra metadata indented by two spaces so it stays inside the queue item block.
- Optional extra metadata is allowed after ` + "`Plan`" + ` using indented Notes/Links bullets or an indented ` + "```yaml" + ` block (see ` + "`.ralph/pin/README.md`" + `).
- Add/update ` + "`.ralph/pin/lookup_table.md`" + ` entries when new areas appear and it is incomplete.

{{INTERACTIVE_INSTRUCTIONS}}
{{INNOVATE_INSTRUCTIONS}}
{{SCOUT_WORKFLOW}}

# OUTPUT
Provide a brief summary of what changed.
`
