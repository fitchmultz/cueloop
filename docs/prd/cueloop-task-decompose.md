# CueLoop Task Decompose
Status: Active
Owner: Maintainers
Source of truth: this document for its stated scope
Parent: [CueLoop Documentation](../index.md)


## Introduction

CueLoop currently supports several task creation and task-structuring workflows, but none of them make recursive decomposition a first-class experience.

- `cueloop task build` creates one strong task from a freeform request.
- `cueloop task split` manually breaks one existing task into a fixed number of child tasks.
- `cueloop prd create` converts a document into one or more tasks.
- `cueloop scan` discovers opportunities and adds them to the queue.

This leaves a product gap for users who start from a high-level engineering goal and want CueLoop to propose a structured tree of executable subtasks before any work runs.

The proposed `cueloop task decompose` command fills that gap. It brings a dedicated decomposition workflow into CueLoop while preserving CueLoop’s core architecture:

- Queue-backed, durable state in `.cueloop/queue.jsonc`
- Existing hierarchy support via `parent_id`
- Preview-first workflow before mutating queue state
- Separation between planning/decomposition and execution

The feature should feel native to CueLoop rather than bolted on. Users should be able to decompose a new goal or decompose an existing task using the same safety, validation, and undo expectations as the rest of the task surface.

## Goals

- Introduce `cueloop task decompose` as the dedicated workflow for recursive task decomposition.
- Let users decompose a freeform request, a plan file, or an existing queue task.
- Generate durable CueLoop tasks, not ephemeral planner-only output.
- Reuse existing task hierarchy, queue validation, ID generation, and undo mechanisms.
- Keep the workflow preview-first so users can inspect the proposed tree before writing.
- Preserve clean separation between decomposition and execution.

## Non-Goals

- Automatically executing decomposed tasks as part of the same command.
- Replacing `cueloop task build`, `cueloop task split`, `cueloop prd create`, or `cueloop scan`.
- Introducing another persistent queue field for decomposition actionability beyond the existing `Task.kind` actionability contract.
- Automatically inferring dependencies outside the generated sibling group.
- Merging or rewriting unrelated existing hierarchy automatically.
- Replacing PRD-specific imports; `cueloop prd create` remains a parser-backed PRD workflow while `cueloop task decompose --from-file` is planner-backed for arbitrary plan documents.

## User Stories

### US-001: Decompose a New Goal into a Task Tree

As a user starting from a high-level engineering goal,
I want to run `cueloop task decompose "<SOURCE>"` and preview a structured task tree,
so that I can turn an abstract goal into reviewable, executable queue entries.

#### Acceptance Criteria

- Running `cueloop task decompose "Build OAuth login with GitHub and Google"` produces a preview by default.
- The preview includes a hierarchy tree, node counts, and warnings when limits or heuristics affect the result.
- The preview does not modify `.cueloop/queue.jsonc` unless `--write` is explicitly provided.
- When `--write` is provided, CueLoop creates a root task and descendant tasks with unique IDs and valid timestamps.
- Created grouping/root/phase tasks persist `kind: group`; executable leaves remain `kind: work_item`.
- Created tasks use `parent_id` to represent hierarchy.
- Generated tasks can be viewed with existing commands such as `cueloop queue tree` and `cueloop task children`.

### US-002: Decompose a Plan File into a Task Tree

As a user with a plan document in the repository or on disk,
I want to run `cueloop task decompose --from-file <path>` and preview a queue tree for the whole plan,
so that I can turn an existing plan into durable CueLoop work without copying it into the shell.

#### Acceptance Criteria

- Running `cueloop task decompose --from-file docs/plans/oauth.md` reads the full file and previews by default.
- Relative paths are resolved from the process current directory; leading `~` is expanded.
- Preview output and JSON include the recorded source path. Paths inside the repo are recorded repo-relative; outside files are recorded as absolute paths.
- The full file content is passed to the planner but is not serialized into machine/app JSON preview payloads.
- Plan-file decomposition preserves every meaningful source section, headline, or ordered phase in the preview/write task tree unless a warning explains that the section was omitted or merged.
- Ordered plan phases appear in logical execution order; with `--with-dependencies`, phase prerequisite relationships are represented as sibling `depends_on` edges.
- `--attach-to <TASK_ID>`, `--child-policy`, `--with-dependencies`, `--format json`, and runner override flags work for plan-file sources.
- Missing files, directories, empty or whitespace-only files, non-UTF-8 files, unreadable files, and files over the conservative size limit fail before planner invocation with clear diagnostics.

### US-003: Decompose an Existing Task In Place

As a user with an existing broad task in the queue,
I want to decompose that task into child tasks,
so that I can preserve the original task context while making the work more actionable.

#### Acceptance Criteria

- Running `cueloop task decompose RQ-0123` previews child tasks under the existing task.
- By default, the existing task is preserved as the parent rather than rejected or archived.
- Generated child tasks use `parent_id = RQ-0123`.
- The preserved source task is marked `kind: group` when write mode turns it into a decomposition umbrella.
- The command refuses to mutate a non-existent task ID.
- The command refuses to decompose tasks from the done archive unless an explicit opt-in is provided in a future or explicit override mode.
- The command records a clear human-readable note on the source task indicating that it was decomposed.

### US-004: Control Granularity and Complexity

As a user with different planning needs,
I want controls for decomposition depth and fanout,
so that CueLoop does not over-decompose or generate unmanageable trees.

#### Acceptance Criteria

- The command supports `--max-depth` to limit recursive depth.
- The command supports `--max-children` to cap per-node fanout.
- The command supports a total-node safety cap, whether user-configurable or defaulted.
- When a limit is reached, the preview and write output explain which limit applied.
- When the model cannot confidently split a node further, CueLoop treats it as a leaf and reports that behavior without failing the entire operation.

### US-006: Attach and Extend an Existing Epic

As a user with an existing epic or parent task,
I want to attach a newly decomposed subtree under that task,
so that I can expand an established plan without replacing the parent itself.

#### Acceptance Criteria

- Running `cueloop task decompose --attach-to RQ-0042 "Plan webhook reliability work"` previews a new subtree under `RQ-0042`.
- When `--write` is provided, CueLoop creates a new root child under the attach target and nests descendants beneath that new root.
- When the attach target already has children, `--child-policy fail|append|replace` governs write behavior deterministically.
- `--child-policy replace` refuses the write when tasks outside the subtree still reference descendant IDs that would be removed.

### US-007: Infer Sibling Dependencies and Emit JSON

As a user automating planning flows,
I want optional sibling dependency inference and stable JSON output,
so that I can review or consume decompositions programmatically.

#### Acceptance Criteria

- Running `cueloop task decompose <SOURCE> --with-dependencies` resolves sibling-only `depends_on` edges from planner keys or sibling titles.
- Self-dependencies, unknown dependencies, and non-sibling references are dropped with warnings.
- Running `cueloop task decompose <SOURCE> --format json` emits a stable versioned JSON payload for preview or write mode.
- JSON output includes actionability metadata identifying the root/group task and first actionable leaf without requiring consumers to parse human text.

### US-005: Use the Workflow Reliably in Non-Interactive Environments

As a user running CueLoop in non-interactive contexts,
I want decomposition to behave safely and predictably,
so that it does not mutate queue state unless I explicitly request it.

#### Acceptance Criteria

- The command supports non-interactive operation without TTY prompts.
- Preview remains the default in non-interactive environments.
- Queue mutation still requires `--write` explicitly.
- Validation failures produce deterministic, human-readable output.

## Functional Requirements

1. CueLoop SHALL add a new `cueloop task decompose` subcommand under the existing `task` command group.
2. CueLoop SHALL accept a freeform request, a `--from-file <PATH>` plan document, or an existing task ID as the decomposition source.
3. CueLoop SHALL support a preview-first workflow and SHALL NOT mutate queue state unless `--write` is provided.
4. CueLoop SHALL generate durable queue tasks rather than ephemeral planner-only output.
5. CueLoop SHALL represent task hierarchy using the existing `parent_id` field.
6. CueLoop SHALL reuse existing queue ID allocation so generated task IDs remain unique across queue and done archives.
7. CueLoop SHALL create valid `created_at` and `updated_at` timestamps for all newly written tasks.
8. CueLoop SHALL preserve the decomposed source task by default when decomposing an existing active task.
9. CueLoop SHALL support configurable recursion depth limits.
10. CueLoop SHALL support configurable per-node fanout limits.
11. CueLoop SHALL enforce a total generated node safety limit before writing queue state.
12. CueLoop SHALL treat hierarchy and execution ordering as separate concepts.
13. CueLoop SHALL reuse queue locking and undo snapshot behavior for write operations.
14. CueLoop SHALL validate queue state before and after decomposition writes.
15. CueLoop SHALL include deterministic human-readable preview output that can be inspected before write.
16. CueLoop SHALL integrate with existing hierarchy navigation commands such as `cueloop queue tree`, `cueloop task children`, and `cueloop task parent`.
17. CueLoop SHALL support runner, model, reasoning-effort, RepoPrompt, and runner CLI override flags consistent with other runner-backed task creation flows.
18. CueLoop SHALL use a dedicated decomposition prompt/template rather than overloading task-builder or scan prompts.
19. CueLoop SHALL fail safely when planner output is malformed, incomplete, or inconsistent with queue rules.
20. CueLoop SHALL support `--attach-to <TASK_ID>` for freeform request and plan-file decomposition under an existing active parent task.
21. CueLoop SHALL support `--child-policy fail|append|replace` for effective parents with existing child trees.
22. CueLoop SHALL support optional sibling dependency inference behind `--with-dependencies`.
23. CueLoop SHALL emit stable versioned JSON output when `--format json` is requested.
24. CueLoop SHALL persist generated decomposition grouping nodes as `kind: group` and generated leaves as `kind: work_item`.
25. CueLoop SHALL report root/group and first actionable leaf metadata in preview and write outputs.

## User Experience

### Primary CLI Examples

```bash
cueloop task decompose "Build OAuth login with GitHub and Google"
cueloop task decompose "Improve webhook reliability" --write
cueloop task decompose RQ-0123 --max-depth 3 --preview
cueloop task decompose RQ-0123 --child-policy append --with-dependencies --write
cueloop task decompose --from-file docs/plans/oauth.md
cueloop task decompose --from-file docs/plans/oauth.md --preview
cueloop task decompose --from-file docs/plans/oauth.md --attach-to RQ-0042 --child-policy append --write
cueloop task decompose --from-file docs/plans/oauth.md --format json
cueloop task decompose --attach-to RQ-0042 --format json "Plan webhook reliability work"
```

### Preview Output Expectations

Preview output should communicate:

- what is being decomposed
- whether the source is a new request, a plan file, or an existing task
- proposed hierarchy
- total node and leaf counts
- warnings about caps, degenerate splits, or dropped invalid output
- the root/group node and the first actionable leaf where execution review should begin

### Write Output Expectations

Write output should communicate:

- root/group task affected or created
- first actionable leaf task ID
- number of tasks created
- list of created task IDs
- whether the parent task was preserved or annotated

## Data Model and State

The implementation uses the existing CueLoop task schema and the durable actionability contract.

Recommended persistent representation:

- `parent_id` for tree structure
- `kind: group` for non-executable decomposition umbrella/root/phase nodes
- default `kind: work_item` for executable leaf tasks
- `plan` for task-local implementation guidance
- `request` for original top-level user intent
- `tags` and `scope` for seeded inheritance

Missing `kind` remains backward-compatible and deserializes as `work_item`.

## Planner and Prompt Requirements

The decomposition system should use a dedicated prompt that asks for structured recursive output.

The planner output should be able to represent:

- task title
- optional description
- optional plan items
- optional tags
- optional scope hints
- optional child nodes

Planner guidance should emphasize:

- minimizing overlap between sibling tasks
- preferring directly actionable leaves
- avoiding low-value placeholder tasks unless clearly justified
- stopping decomposition when a task is runnable without additional planning

## Validation and Safety Requirements

- Preview mode must not acquire a queue mutation lock unless implementation details make read-side locking necessary.
- Write mode must acquire the queue lock before mutation.
- Write mode must create an undo snapshot before saving.
- Queue validation failures before planning must abort the operation.
- Queue validation failures after materialization must abort without partial writes.
- Malformed planner output must fail safely with actionable diagnostics.
- Existing task IDs must never be reused.
- Generated parent-child relationships must not create parent cycles.

## Edge Cases and Failure Modes

### Plan File Input Errors

- Missing paths fail with a clear "plan file not found" diagnostic.
- Directory paths fail before planner invocation.
- Empty or whitespace-only files are rejected.
- Files larger than the decomposition source limit are rejected before reading into the prompt.
- Non-UTF-8 and unreadable files report the path and UTF-8 read context.

### Existing Parent Already Has Children

- Default behavior when decomposing an existing task with children should be to refuse write.
- Preview should still work and clearly explain the conflict.
- `--child-policy append` should preserve the existing subtree and insert the new subtree immediately after it.
- `--child-policy replace` should remove the existing descendant subtree only when no outside task still references it.

### Done or Rejected Source Task

- CueLoop should refuse to decompose done or rejected tasks by default.
- If future support is added, it should require explicit opt-in and remain preview-first.

### Degenerate Planner Output

- Empty child arrays for a node expected to split should result in a warning and a leaf fallback.
- Repeated one-child recursion should be collapsed to prevent unhelpful chains.
- Excessive node counts should be capped and reported.

### Queue Ordering

- New root decompositions should respect existing “doing task first” insertion behavior.
- Child tasks for an existing task should be inserted deterministically near the source task.

### Non-Interactive Environments

- The command should not prompt when stdin is not a TTY.
- Non-interactive runs must require explicit source and flags rather than relying on interactive disambiguation.

## Product Decisions

### Preview Default

Preview SHALL be the hard default in all environments.

- `cueloop task decompose <SOURCE>` performs a preview only.
- Queue mutation requires explicit `--write`.
- There is no TTY-only “safety behavior” split for preview vs write.
- This keeps the command predictable, scriptable, and safe.

Rationale:

- Decomposition is a high-blast-radius planning command that can create many tasks at once.
- Hidden environment-dependent behavior is a bad fit for automation and a bad fit for user trust.
- Users should never have to remember whether they were in a terminal, CI shell, or app bridge to know whether queue state changed.

### Existing Parent with Existing Children

Default behavior when decomposing an existing task that already has children SHALL be to refuse write unless the caller explicitly chooses `--child-policy append` or `--child-policy replace`.

Preview still succeeds and shows the conflict or selected policy.

Rationale:

- `fail` remains the safest default.
- `append` gives users an explicit non-destructive extension path.
- `replace` is acceptable only with strict reference checks and undo coverage.

### Dependency Inference Scope

Dependency inference SHALL be optional and limited to siblings within the same generated parent group.

Rationale:

- Sibling-only inference captures the most useful ordering constraints without exposing the planner to arbitrary queue-wide references.
- Restricting inference scope keeps validation and debugging tractable.

### Parent Annotation Strategy

Decomposed parent tasks SHALL receive a human-readable note in v1.

Rationale:

- Notes help humans scanning queue history.
- Custom fields can be added later when there is a concrete consumer for them.

## Success Metrics

- Users can create a decomposed task tree from a high-level request without manually chaining `task build` and `task split`.
- Generated trees are valid under existing queue validation rules.
- Preview-to-write flow is understandable and safe.
- Users can immediately inspect results with existing `queue tree` and `task children` commands.
- A representative ordered plan-file fixture decomposes into a complete queue subtree whose tasks cover all source sections, preserve logical execution order, validate with `cueloop queue validate`, and can be inspected with `cueloop queue tree` and `cueloop task children`.
- The feature reduces manual queue shaping for multi-step goals.

## Implementation Status

The implementation now includes:

- preview-first decomposition for freeform requests, plan files, and existing tasks
- `--from-file <PATH>` plan-file loading with repo-relative provenance when possible
- `--attach-to` subtree attachment for freeform requests and plan files
- `--child-policy fail|append|replace`
- optional sibling dependency inference via `--with-dependencies`
- stable versioned JSON output via `--format json` and `cueloop machine task decompose`
- CueLoopMac workspace/model/UI support for plan-file decomposition

Richer visual review/edit flows can build on the same machine JSON contract rather than introducing a separate decomposition path.
