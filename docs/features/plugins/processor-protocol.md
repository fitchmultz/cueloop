# Plugin Processor Protocol
Status: Active
Owner: Maintainers
Source of truth: this document for task processor hook invocation and lifecycle semantics
Parent: [Ralph Plugin System](../plugins.md)

Purpose: Define processor hook timing, inputs, ordering, environment, mutation rules, and failure behavior.

---

## Hook Types

| Hook | When Invoked | Input File Contents | Failure Behavior |
|------|--------------|---------------------|------------------|
| `validate_task` | After task selection, before marking `doing` | Full task JSON | Fatal; aborts before work begins |
| `pre_prompt` | After prompt compilation, before runner spawn | Prompt text | Fatal; aborts before runner starts |
| `post_run` | After each successful runner `run` and each successful `resume`/Continue | Runner stdout (NDJSON) | Fatal at failure point |

`post_run` may run multiple times in one overall task execution.

## Hook Protocol

For each hook invocation:

```bash
<processor-bin> <HOOK> <TASK_ID> <FILEPATH>
```

Arguments:

1. `HOOK`: `validate_task`, `pre_prompt`, or `post_run`
2. `TASK_ID`: task ID (for example `RQ-0001`)
3. `FILEPATH`: temporary file path with hook input

Execution context:

- working directory: repository root
- environment:
  - `RALPH_PLUGIN_ID`
  - `RALPH_PLUGIN_CONFIG_JSON`

## Processor Ordering

Enabled processors execute in ascending lexicographic order by `plugin_id` for deterministic behavior.

```text
a.plugin -> b.plugin -> my.plugin -> z.plugin
```

## Exit Code Contract

| Exit Code | Meaning |
|-----------|---------|
| `0` | Success; continue |
| Non-zero | Failure; abort and surface redacted stderr |

## Pre-Prompt In-Place Mutation

`pre_prompt` hooks may mutate the prompt file in place. Ralph reads the final file contents after all hooks complete.

## Related Docs

- [Examples](examples.md)
- [Troubleshooting and Compatibility](troubleshooting.md)
- [Plugin Operations](operations.md)
