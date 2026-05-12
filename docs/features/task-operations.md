# Task Operations
Status: Active
Owner: Maintainers
Source of truth: this document for task creation, editing, templates, batch operations, and quick CLI workflows
Parent: [Task System](tasks.md)

This page shows how operators create and mutate CueLoop tasks from the CLI and templates. For raw task fields, see [Task Schema and Field Reference](task-schema.md).

## Related Task Docs

- [Task System](tasks.md) — overview and task documentation index.
- [Task Schema and Field Reference](task-schema.md) — task JSON fields, queue-file basics, per-task agent overrides, examples, and schema validation.
- [Task Lifecycle and Priority](task-lifecycle.md) — statuses, lifecycle timestamps, runnability basics, and priority semantics.
- [Task Relationships](task-relationships.md) — dependency, blocking, relation, duplicate, and hierarchy semantics.
- [Queue](queue.md) — queue file operations, ordering, archive, repair, import/export, and locks.
- [Machine Contract](../machine-contract.md) — versioned JSON for `cueloop machine task create`, `build`, and other machine surfaces used by agents and the macOS app.

---

## Task Creation

### Methods Overview

| Method | Command | Use Case |
|--------|---------|----------|
| Direct CLI | `cueloop task "description"` | Quick task creation |
| Task Builder | `cueloop task build "description"` | AI-assisted task generation |
| Machine create | `cueloop machine task create --input …` | Append one `todo` task (or template-guided single task) with stable JSON I/O for agents and automation |
| Machine build | `cueloop machine task build --input …` | Same task-builder stack as `task build`; stdout is only the versioned `MachineTaskBuildDocument` |
| Template | `cueloop task template build <name>` | From predefined template |
| Refactor Scan | `cueloop task refactor` | Auto-generate from large files |
| Import | `cueloop queue import` | Bulk import from CSV/JSON |
| Clone | `cueloop task clone RQ-0001` | Duplicate existing task |
| App (macOS) | `cueloop app open` | Visual task creation and triage |

### Direct CLI Creation

```bash
# Create task from description
cueloop task "Add user authentication to API"

# With tags and scope hints
cueloop task "Fix memory leak" --tags bug,rust --scope src/memory.rs

# With runner override
cueloop task "Complex analysis" --runner claude --effort high
```

**Positioning:** New tasks are inserted at the top of the queue (position 0), or position 1 if the first task is already `doing`.

### Task Builder (AI-Assisted)

```bash
# AI generates task fields from description
cueloop task build "Implement OAuth2 flow with Google and GitHub providers"

# With template hint
cueloop task build "Fix race condition" --template bug

# With strict template validation
cueloop task build "Add feature" --template feature --strict-templates
```

The task builder uses the prompt at `.cueloop/prompts/task_builder.md` (or embedded default) to guide AI task generation.

### Machine task create and build (agents and automation)

For coding agents, CI, and other callers that need **only JSON on stdout** and structured failures on stderr, use the machine commands. They read a JSON request from `--input <path>` **or from stdin** when `--input` is omitted (stdin must be non-empty JSON).

```bash
cueloop machine task create --input task-create.json
cueloop machine task build --input task-build-request.json
# Optional: pipe a request document
printf '%s' '{"version":1,"title":"Fix flaky test","priority":"normal"}' | cueloop machine task create
```

- **Create:** Request fields match `MachineTaskCreateRequest` in the crate contracts (`version`, `title`, `priority`, optional `description` / `tags` / `scope`, optional `template` / `target`). Without `template`, CueLoop acquires the queue lock, appends one new `todo` task, creates undo, and prints `MachineTaskCreateDocument`. With `template`, the task-builder runner runs with `strict_templates: true` and must produce exactly one task.
- **Build:** Request fields match `MachineTaskBuildRequest` (`version`, `request` prompt, optional template hints, `strict_templates`, `estimated_minutes`, etc.). CLI flags from `AgentArgs` (for example `--runner`, `--model`, `--effort`) apply the same way as on other machine runner surfaces.

Authoritative wire format and response fields: [Machine Contract](../machine-contract.md#machine-task-create-version-1) and [Machine Contract](../machine-contract.md#machine-task-build-version-1). JSON Schemas: `cueloop machine schema` (keys `task_create_request`, `task_create`, `task_build_request`, `task_build`).

### Template-Based Creation

```bash
# List available templates
cueloop task template list

# Show template details
cueloop task template show bug

# Create from template
cueloop task template build bug "Login form validation fails on Safari"

# Create with target substitution
cueloop task template build refactor "Split large module" --target src/main.rs
```

**Built-in Templates:**
- `bug` - Bug fix tasks
- `feature` - New feature tasks
- `refactor` - Code refactoring tasks
- `test` - Test writing tasks
- `docs` - Documentation tasks

**Custom Templates:** Place JSON files in `.cueloop/templates/` to override or extend.

### Refactor Scan

```bash
# Scan for large files and create refactor tasks
cueloop task refactor

# With custom threshold (default: 500 LOC)
cueloop task refactor --threshold 800

# Dry run to preview
cueloop task refactor --dry-run

# Batch modes
cueloop task refactor --batch never       # One task per file
cueloop task refactor --batch auto        # Group related files (default)
cueloop task refactor --batch aggressive  # Group by directory
```

Scans for `.rs` files exceeding the LOC threshold (excluding comments/empty lines).

### Import

```bash
# Import from JSON
cueloop queue import --format json --input tasks.json

# Import from CSV with preview
cueloop queue import --format csv --input tasks.csv --dry-run

# Handle duplicates
cueloop queue import --format json --input tasks.json --on-duplicate rename
```

**Normalization during import:**
- Trims all fields, drops empty list items
- Backfills missing timestamps
- Sets `completed_at` for terminal statuses
- Generates IDs for tasks without them

### Clone

```bash
# Clone existing task
cueloop task clone RQ-0001

# Clone with status override
cueloop task clone RQ-0001 --status todo

# Clone with title prefix
cueloop task clone RQ-0001 --title-prefix "[Follow-up] "
```

Creates a new task with copied fields (except ID and timestamps) and a reference in `relates_to`.

---

## Task Editing

### Edit Commands

```bash
# Edit single field
cueloop task edit priority high RQ-0001
cueloop task edit status doing RQ-0001
cueloop task edit tags "rust,cli" RQ-0001

# Edit multiple tasks
cueloop task edit priority low RQ-0001 RQ-0002 RQ-0003

# Edit by tag filter
cueloop task edit status doing --tag-filter rust

# Dry run to preview
cueloop task edit scope "src/auth.rs" RQ-0001 --dry-run
```

### Editable Fields

| Field | Input Format | Example |
|-------|--------------|---------|
| `title` | string | `"New title"` |
| `status` | enum or empty (cycles) | `doing`, `""` |
| `priority` | enum or empty (cycles) | `high`, `""` |
| `tags` | comma/newline separated | `rust,cli` |
| `scope` | comma/newline separated | `src/main.rs,src/lib.rs` |
| `evidence` | comma/newline separated | `logs/error.txt` |
| `plan` | comma/newline separated | `Step 1, Step 2` |
| `notes` | comma/newline separated | `Note 1; Note 2` |
| `depends_on` | comma/newline separated | `RQ-0001,RQ-0002` |
| `blocks` | comma/newline separated | `RQ-0003` |
| `relates_to` | comma/newline separated | `RQ-0004` |
| `duplicates` | string or empty | `RQ-0005`, `""` |
| `custom_fields` | key=value pairs | `severity=high,owner=cueloop` |

### Custom Field Editing

```bash
# Set custom fields
cueloop task field severity high RQ-0001
cueloop task field owner platform RQ-0001
cueloop task field story-points 5 RQ-0001

# Set on multiple tasks
cueloop task field sprint 24 RQ-0001 RQ-0002 RQ-0003
```

### AI-Powered Update

```bash
# AI updates fields based on repository state
cueloop task update RQ-0001

# Update specific fields
cueloop task update RQ-0001 --fields scope,evidence

# Update all tasks
cueloop task update --fields all

# Dry run
cueloop task update RQ-0001 --dry-run
```

Uses the prompt at `.cueloop/prompts/task_updater.md` to guide AI field updates.

### Batch Operations

```bash
# Batch status change
cueloop task batch status doing RQ-0001 RQ-0002

# Batch with tag filter
cueloop task batch status done --tag-filter "completed"

# Batch field edit
cueloop task batch edit priority high RQ-0001 RQ-0002

# Continue on error
cueloop task batch status doing RQ-0001 RQ-0002 --continue-on-error

# Dry run
cueloop task batch edit priority low --tag-filter backlog --dry-run
```

---

## Task Templates

### Template Structure

Templates are partial Task JSON objects:

```json
{
  "title": "",
  "status": "todo",
  "priority": "medium",
  "tags": ["bug"],
  "scope": [],
  "evidence": [],
  "plan": [
    "Reproduce the issue",
    "Identify root cause",
    "Implement fix",
    "Add regression test",
    "Verify fix"
  ]
}
```

### Variable Substitution

Templates support variable substitution:

```json
{
  "title": "Refactor ${TARGET}",
  "scope": ["${TARGET}"]
}
```

Usage:
```bash
cueloop task template build refactor "Split module" --target src/main.rs
```

### Template Locations

1. **Built-in**: Embedded in CueLoop binary
2. **Custom**: `.cueloop/templates/<name>.json`
3. **Project overrides**: Custom templates shadow built-ins with same name

### Creating Custom Templates

```bash
# Create template directory
mkdir -p .cueloop/templates

# Create template file
cat > .cueloop/templates/security.json << 'EOF'
{
  "tags": ["security"],
  "priority": "critical",
  "plan": [
    "Assess security impact",
    "Identify affected components",
    "Implement security fix",
    "Add security tests",
    "Request security review"
  ],
  "evidence": ["Security audit findings"]
}
EOF
```

---

## CLI Quick Reference

| Operation | Command |
|-----------|---------|
| Create task | `cueloop task "description"` |
| Build with AI | `cueloop task build "description"` |
| Show task | `cueloop task show RQ-0001` |
| Edit field | `cueloop task edit <field> <value> RQ-0001` |
| Set custom field | `cueloop task field <key> <value> RQ-0001` |
| Change status | `cueloop task status <status> RQ-0001` |
| Mark done | `cueloop task done RQ-0001` |
| Clone task | `cueloop task clone RQ-0001` |
| Add dependency | `cueloop task edit depends_on "RQ-0001,RQ-0002" RQ-0003` |
| Relate tasks | `cueloop task relate RQ-0001 RQ-0002` |
| Mark duplicate | `cueloop task mark-duplicate RQ-0001 RQ-0002` |
| List children | `cueloop task children RQ-0001` |
| Show parent | `cueloop task parent RQ-0002` |
| Validate queue | `cueloop queue validate` |

---

## See Also

- [CLI](../cli.md) — complete command reference.
- [Prompts](prompts.md) — prompt customization for task builder and updater flows.
- [Import/Export](import-export.md) — queue import and export workflows.
