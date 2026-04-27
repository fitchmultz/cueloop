# Task Operations
Status: Active
Owner: Maintainers
Source of truth: this document for task creation, editing, templates, batch operations, and quick CLI workflows
Parent: [Task System](tasks.md)

This page shows how operators create and mutate Ralph tasks from the CLI and templates. For raw task fields, see [Task Schema and Field Reference](task-schema.md).

## Related Task Docs

- [Task System](tasks.md) — overview and task documentation index.
- [Task Schema and Field Reference](task-schema.md) — task JSON fields, queue-file basics, per-task agent overrides, examples, and schema validation.
- [Task Lifecycle and Priority](task-lifecycle.md) — statuses, lifecycle timestamps, runnability basics, and priority semantics.
- [Task Relationships](task-relationships.md) — dependency, blocking, relation, duplicate, and hierarchy semantics.
- [Queue](queue.md) — queue file operations, ordering, archive, repair, import/export, and locks.

---

## Task Creation

### Methods Overview

| Method | Command | Use Case |
|--------|---------|----------|
| Direct CLI | `ralph task "description"` | Quick task creation |
| Task Builder | `ralph task build "description"` | AI-assisted task generation |
| Template | `ralph task template build <name>` | From predefined template |
| Refactor Scan | `ralph task refactor` | Auto-generate from large files |
| Import | `ralph queue import` | Bulk import from CSV/JSON |
| Clone | `ralph task clone RQ-0001` | Duplicate existing task |
| App (macOS) | `ralph app open` | Visual task creation and triage |

### Direct CLI Creation

```bash
# Create task from description
ralph task "Add user authentication to API"

# With tags and scope hints
ralph task "Fix memory leak" --tags bug,rust --scope src/memory.rs

# With runner override
ralph task "Complex analysis" --runner claude --effort high
```

**Positioning:** New tasks are inserted at the top of the queue (position 0), or position 1 if the first task is already `doing`.

### Task Builder (AI-Assisted)

```bash
# AI generates task fields from description
ralph task build "Implement OAuth2 flow with Google and GitHub providers"

# With template hint
ralph task build "Fix race condition" --template bug

# With strict template validation
ralph task build "Add feature" --template feature --strict-templates
```

The task builder uses the prompt at `.ralph/prompts/task_builder.md` (or embedded default) to guide AI task generation.

### Template-Based Creation

```bash
# List available templates
ralph task template list

# Show template details
ralph task template show bug

# Create from template
ralph task template build bug "Login form validation fails on Safari"

# Create with target substitution
ralph task template build refactor "Split large module" --target src/main.rs
```

**Built-in Templates:**
- `bug` - Bug fix tasks
- `feature` - New feature tasks
- `refactor` - Code refactoring tasks
- `test` - Test writing tasks
- `docs` - Documentation tasks

**Custom Templates:** Place JSON files in `.ralph/templates/` to override or extend.

### Refactor Scan

```bash
# Scan for large files and create refactor tasks
ralph task refactor

# With custom threshold (default: 500 LOC)
ralph task refactor --threshold 800

# Dry run to preview
ralph task refactor --dry-run

# Batch modes
ralph task refactor --batch never       # One task per file
ralph task refactor --batch auto        # Group related files (default)
ralph task refactor --batch aggressive  # Group by directory
```

Scans for `.rs` files exceeding the LOC threshold (excluding comments/empty lines).

### Import

```bash
# Import from JSON
ralph queue import --format json --input tasks.json

# Import from CSV with preview
ralph queue import --format csv --input tasks.csv --dry-run

# Handle duplicates
ralph queue import --format json --input tasks.json --on-duplicate rename
```

**Normalization during import:**
- Trims all fields, drops empty list items
- Backfills missing timestamps
- Sets `completed_at` for terminal statuses
- Generates IDs for tasks without them

### Clone

```bash
# Clone existing task
ralph task clone RQ-0001

# Clone with status override
ralph task clone RQ-0001 --status todo

# Clone with title prefix
ralph task clone RQ-0001 --title-prefix "[Follow-up] "
```

Creates a new task with copied fields (except ID and timestamps) and a reference in `relates_to`.

---

## Task Editing

### Edit Commands

```bash
# Edit single field
ralph task edit priority high RQ-0001
ralph task edit status doing RQ-0001
ralph task edit tags "rust,cli" RQ-0001

# Edit multiple tasks
ralph task edit priority low RQ-0001 RQ-0002 RQ-0003

# Edit by tag filter
ralph task edit status doing --tag-filter rust

# Dry run to preview
ralph task edit scope "src/auth.rs" RQ-0001 --dry-run
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
| `custom_fields` | key=value pairs | `severity=high,owner=ralph` |

### Custom Field Editing

```bash
# Set custom fields
ralph task field severity high RQ-0001
ralph task field owner platform RQ-0001
ralph task field story-points 5 RQ-0001

# Set on multiple tasks
ralph task field sprint 24 RQ-0001 RQ-0002 RQ-0003
```

### AI-Powered Update

```bash
# AI updates fields based on repository state
ralph task update RQ-0001

# Update specific fields
ralph task update RQ-0001 --fields scope,evidence

# Update all tasks
ralph task update --fields all

# Dry run
ralph task update RQ-0001 --dry-run
```

Uses the prompt at `.ralph/prompts/task_updater.md` to guide AI field updates.

### Batch Operations

```bash
# Batch status change
ralph task batch status doing RQ-0001 RQ-0002

# Batch with tag filter
ralph task batch status done --tag-filter "completed"

# Batch field edit
ralph task batch edit priority high RQ-0001 RQ-0002

# Continue on error
ralph task batch status doing RQ-0001 RQ-0002 --continue-on-error

# Dry run
ralph task batch edit priority low --tag-filter backlog --dry-run
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
ralph task template build refactor "Split module" --target src/main.rs
```

### Template Locations

1. **Built-in**: Embedded in Ralph binary
2. **Custom**: `.ralph/templates/<name>.json`
3. **Project overrides**: Custom templates shadow built-ins with same name

### Creating Custom Templates

```bash
# Create template directory
mkdir -p .ralph/templates

# Create template file
cat > .ralph/templates/security.json << 'EOF'
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
| Create task | `ralph task "description"` |
| Build with AI | `ralph task build "description"` |
| Show task | `ralph task show RQ-0001` |
| Edit field | `ralph task edit <field> <value> RQ-0001` |
| Set custom field | `ralph task field <key> <value> RQ-0001` |
| Change status | `ralph task status <status> RQ-0001` |
| Mark done | `ralph task done RQ-0001` |
| Clone task | `ralph task clone RQ-0001` |
| Add dependency | `ralph task edit depends_on "RQ-0001,RQ-0002" RQ-0003` |
| Relate tasks | `ralph task relate RQ-0001 RQ-0002` |
| Mark duplicate | `ralph task mark-duplicate RQ-0001 RQ-0002` |
| List children | `ralph task children RQ-0001` |
| Show parent | `ralph task parent RQ-0002` |
| Validate queue | `ralph queue validate` |

---

## See Also

- [CLI](../cli.md) — complete command reference.
- [Prompts](prompts.md) — prompt customization for task builder and updater flows.
- [Import/Export](import-export.md) — queue import and export workflows.
