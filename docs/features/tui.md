# Ralph TUI (Terminal User Interface)

The Terminal User Interface (TUI) is Ralph's primary user-facing entry point, providing an interactive interface for queue management, task execution, and monitoring.

![TUI Interface](assets/images/2026-02-07-tui-interface.png)

---

## Overview

The Ralph TUI provides a rich, keyboard-driven interface for managing tasks without leaving your terminal. It combines:

- **Task list view** with filtering, sorting, and search capabilities
- **Detailed task inspection** with Markdown rendering and syntax highlighting
- **Live execution view** with real-time runner output
- **Visual overlays** for dependency graphs, workflow flowcharts, and parallel run state
- **Command palette** for quick access to all commands

The TUI is built on [ratatui](https://github.com/ratatui-org/ratatui) and provides a modern, responsive terminal experience with mouse support, Unicode box-drawing, and truecolor support where available.

---

## Launching the TUI

### Primary Entry Point

```bash
ralph tui
```

### Alternate Entry Points

The TUI can also be launched through run commands:

```bash
# Run one task interactively
ralph run one -i

# Run loop interactively (auto-starts loop mode)
ralph run loop -i
```

### Launch Options

```bash
# Read-only mode (disable execution)
ralph tui --read-only

# Show workflow flowchart on startup
ralph run one -i --visualize

# Specify runner and model
ralph tui --runner claude --model sonnet --effort high

# Disable mouse capture (for terminals with broken mouse support)
ralph tui --no-mouse

# Use ASCII borders instead of Unicode
ralph tui --ascii-borders

# Force/disable colors
ralph tui --color always    # Force colors
ralph tui --color never     # Disable colors
```

---

## Normal Mode (Task List View)

Normal mode is the default view when the TUI starts. It displays the task list on the left and task details on the right.

### Layout

```
┌────────────────────────────────────────────────────────────────────┐
│ Ralph - Task Queue (12 tasks, 3 filtered)           Loop: OFF      │
├──────────────────┬─────────────────────────────────────────────────┤
│ RQ-0001 ● Task 1 │ Title: Implement feature X                      │
│ RQ-0002 ○ Task 2 │ Status: Todo                                    │
│ RQ-0003 ○ Task 3 │ Priority: High                                  │
│ ...              │ Tags: feature, urgent                           │
│                  │                                                 │
│                  │ Evidence:                                       │
│                  │ • User reported issue #123                      │
│                  │ • Performance degraded in v2.0                  │
│                  │                                                 │
│                  │ Plan:                                           │
│                  │ 1. Analyze current implementation               │
│                  │ 2. Design solution                              │
│                  │ 3. Implement and test                           │
└──────────────────┴─────────────────────────────────────────────────┘
↑↓ nav | Enter run | ? help | : palette | L loop | a archive | q quit
```

### Board (Kanban) View

Press `b` to switch to Board view, which displays tasks in Kanban-style columns organized by status.

```
┌────────────────────────────────────────────────────────────────────┐
│ Ralph - Board View (12 tasks, 3 filtered)           Loop: OFF      │
├─────────────┬─────────────┬─────────────┬─────────────┬────────────┤
│ DRAFT       │ TODO        │ DOING       │ DONE        │ REJECTED   │
│ (2)         │ (5)         │ (2)         │ (2)         │ (1)        │
├─────────────┼─────────────┼─────────────┼─────────────┼────────────┤
│ ● RQ-0001   │   RQ-0003   │   RQ-0007   │   RQ-0010   │   RQ-0012  │
│   Fix auth  │   Add tests │   Refactor  │   Update    │   Wontfix  │
│   ★ High    │   ↑ High    │   → Medium  │   ✓ Done    │   ✗ Reject │
│             │             │             │             │            │
│   RQ-0002   │   RQ-0004   │   RQ-0008   │   RQ-0011   │            │
│   Docs      │   Cleanup   │             │             │            │
│   ↓ Low     │   → Medium  │             │             │            │
└─────────────┴─────────────┴─────────────┴─────────────┴────────────┘
←→ column | ↑↓ task | Enter run | l list view | ? help | q quit
```

**Board View Features:**
- **5 columns**: Draft, Todo, Doing, Done, Rejected
- **Task cards** show ID, truncated title, and priority
- **Priority colors**: Card borders are color-coded by priority
- **Column counts**: Shows number of tasks in each column
- **Auto-fallback**: If terminal is too narrow (< 60 cols), shows message to use list view or resize

**Board View Navigation:**
| Key | Action |
|-----|--------|
| `←` / `→` | Move between columns |
| `↑` / `↓` or `j` / `k` | Navigate tasks in column |
| `l` | Switch back to list view |
| `Enter` | Run selected task |

### Visual Indicators

| Indicator | Meaning |
|-----------|---------|
| `●` | Selected task |
| `○` | Unselected task |
| `✓` | Done status |
| `✗` | Rejected status |
| `⏵` | Doing status |
| `✎` | Draft status |
| `★` | Critical priority |
| `↑` | High priority |
| `→` | Medium priority |
| `↓` | Low priority |

### Task List Status Icons

Tasks in the list are prefixed with status indicators:

- **Draft** (`✎`): Task is in draft state
- **Todo** (`○`): Task is ready to work on
- **Doing** (`⏵`): Task is currently being worked on
- **Done** (`✓`): Task is complete
- **Rejected** (`✗`): Task was rejected

---

## Execution View

When a task is running, the TUI switches to execution view showing live runner output.

### Layout

```
┌────────────────────────────────────────────────────────────────────┐
│ Executing: RQ-0001 - Implement feature X              Phase 1/3    │
├────────────────────────────────────────────────────────────────────┤
│ ▶ Planning (01:23)  ○ Implementation  ○ Review                     │
├────────────────────────────────────────────────────────────────────┤
│ > Analyzing codebase structure...                                  │
│ > Identified 3 files that need modification                        │
│                                                                    │
│ ## Plan                                                            │
│                                                                    │
│ 1. Update src/main.rs to add new endpoint                          │
│ 2. Modify src/lib.rs to expose new function                        │
│ 3. Add tests in tests/integration.rs                               │
│                                                                    │
│ Proceeding to implementation...                                    │
│                                                                    │
│ ...                                                                │
└────────────────────────────────────────────────────────────────────┘
Esc return | ↑↓ scroll | PgUp/PgDn page | a autoscroll | L stop loop
```

### Progress Panel

The execution view includes a progress panel showing:

- **Phase indicators**: Visual indicators for each phase
  - `▶` (yellow): Currently active phase
  - `✓` (green): Completed phase
  - `○` (gray): Pending phase
- **Phase timing**: Elapsed time per phase in MM:SS format
- **Total execution time**: Overall duration since task start

Press `p` in execution view to toggle the progress panel visibility.

### Phase Tracking

The TUI automatically tracks phase transitions from runner output (e.g., "# IMPLEMENTATION MODE" headers). The flowchart adapts to the configured workflow:

- **1-phase**: Shows "Single Phase" (Execute task)
- **2-phase**: Shows Planning → Implementation
- **3-phase** (default): Shows Planning → Implementation → Review

---

## Keybindings Reference

### Normal Mode Keybindings

#### Navigation

| Key | Action |
|-----|--------|
| `↑` / `↓` or `j` / `k` | Move selection up/down |
| `PgUp` / `PgDn` | Page up/down (focused panel) |
| `Home` / `End` | Jump to top/bottom (focused panel) |
| `Tab` / `Shift+Tab` | Switch focus between list/details |
| `K` / `J` | Move selected task up/down (reorder) |
| `G` | Jump to task by ID |
| `Enter` | Run selected task |

#### Actions

| Key | Action |
|-----|--------|
| `?` or `h` | Open help overlay |
| `l` | Switch to list view |
| `b` | Switch to board view |
| `L` | Toggle loop mode |
| `a` | Archive done/rejected tasks (confirmation) |
| `d` | Delete selected task (confirmation) |
| `e` | Edit selected task fields |
| `n` | Create new task (title only) |
| `N` | Build task with agent (full structure) |
| `c` | Edit project config |
| `g` | Scan repository |
| `v` | View dependency graph |
| `P` | View parallel run state (read-only) |
| `r` | Reload queue from disk |
| `O` | Open selected task scope in `$EDITOR` |
| `y` | Copy file:line refs from notes/evidence |
| `q` / `Esc` | Quit (may prompt if runner active) |
| `Ctrl+C` / `Ctrl+Q` | Quit (same as `q`/`Esc`) |

#### Command Palette

| Key | Action |
|-----|--------|
| `:` | Open command palette |
| `Ctrl+P` | Command palette (shortcut) |

#### Filters & Search

| Key | Action |
|-----|--------|
| `/` | Search tasks (free-text) |
| `Ctrl+F` | Search tasks (shortcut) |
| `t` | Filter by tags |
| `o` | Filter by scope |
| `f` | Cycle status filter |
| `x` | Clear filters |
| `C` | Toggle case-sensitive search |
| `R` | Toggle regex search |

#### Quick Changes

| Key | Action |
|-----|--------|
| `s` | Cycle selected task status |
| `p` | Cycle selected task priority |

### Execution View Keybindings

| Key | Action |
|-----|--------|
| `Esc` | Return to task list |
| `↑` / `↓` or `j` / `k` | Scroll logs |
| `PgUp` / `PgDn` | Page logs |
| `a` | Toggle auto-scroll |
| `L` | Stop loop mode |
| `p` | Toggle progress panel visibility |
| `f` | Toggle flowchart overlay |

### Help Overlay Keybindings

| Key | Action |
|-----|--------|
| `Esc` / `?` / `h` | Close help overlay |
| `↑` / `↓` | Scroll help |
| `PgUp` / `PgDn` | Page help |
| `Home` / `End` / `g` / `G` | Jump to top/bottom |

### Board View Keybindings

| Key | Action |
|-----|--------|
| `←` / `→` | Move between columns |
| `↑` / `↓` or `j` / `k` | Navigate tasks in column |
| `l` | Switch to list view |
| `Enter` | Run selected task |
| `e` | Edit selected task |
| `d` | Delete selected task |
| `s` | Cycle task status |
| `p` | Cycle priority |
| All other keys | Same as Normal mode |

### Command Palette Keybindings

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate entries |
| `PgUp` / `PgDn` | Page entries |
| `Home` / `End` | Jump to first/last entry |
| `Enter` | Execute selected command |
| `Esc` | Close palette |
| Type to filter | Filter commands by typing |

### Multi-Select Mode Keybindings

| Key | Action |
|-----|--------|
| `m` | Toggle multi-select mode |
| `Space` | Toggle selection of current task |
| `d` | Batch delete selected tasks |
| `a` | Batch archive selected tasks |
| `Esc` | Clear selection and exit multi-select mode |

---

## Task Operations

### Creating Tasks

#### Quick Create (Title Only)

1. Press `n` in Normal mode
2. Type the task title
3. Press `Enter` to create, `Esc` to cancel

#### Task Builder (Full Structure)

1. Press `N` in Normal mode
2. Enter a natural language description of what needs to be done
3. Press `Enter` to continue to advanced options
4. Configure optional overrides (or leave as defaults):
   - **Tags hint**: Comma-separated tags to suggest
   - **Scope hint**: Comma-separated scope paths
   - **Runner**: Override the agent runner
   - **Model**: Override the model
   - **Reasoning effort**: Override effort level
   - **RepoPrompt mode**: Override RepoPrompt behavior
5. Navigate to "[ Build Task ]" and press `Enter`

The TUI switches to an Executing view showing the task builder progress. On completion, the TUI reloads `queue.json` and returns to Normal mode.

### Editing Tasks

1. Select the task you want to edit
2. Press `e`
3. Navigate to the field you want to edit with `↑`/`↓`
4. Press `Enter` to edit/cycle the value
5. For text fields: edit the value, then press `Enter` to save or `Esc` to cancel
6. For enum fields (status, priority): press `Enter` to cycle through values
7. Press `x` to clear a text/list/map field

Editable fields include:
- `title`, `status`, `priority`
- `tags`, `scope`, `evidence`, `plan`, `notes`, `depends_on`
- `request`
- `custom_fields`
- Timestamps (`created_at`, `updated_at`, `completed_at`)

### Changing Status

**Method 1: Quick Cycle**
- Press `s` to cycle: Draft → Todo → Doing → Done → Rejected → Draft

**Method 2: Direct Set (via palette)**
- Press `:` and type "set status"
- Choose: Draft, Todo, Doing, Done, or Rejected

### Changing Priority

**Method 1: Quick Cycle**
- Press `p` to cycle: Critical → High → Medium → Low → Critical

**Method 2: Direct Set (via palette)**
- Press `:` and type "set priority"
- Choose: Critical, High, Medium, or Low

### Deleting Tasks

1. Select the task
2. Press `d`
3. Confirm with `y` or cancel with `n`/`Esc`

### Archiving Tasks

Archiving moves all Done/Rejected tasks from `queue.json` to `done.json`:

1. Press `a` in Normal mode
2. Confirm with `y` or cancel with `n`/`Esc`

### Reordering Tasks

- Press `K` to move the selected task up
- Press `J` to move the selected task down

### Jumping to Task by ID

1. Press `G` (uppercase)
2. Type the task ID (e.g., `RQ-0001`, case-insensitive)
3. Press `Enter` to jump

If the task is filtered out, filters are automatically cleared.

---

## Filters & Search

### Text Search

Press `/` or `Ctrl+F` to open the search prompt:

- Type to search task titles, IDs, and content
- Results update live as you type
- Press `Enter` to confirm, `Esc` to clear

### Tag Filters

Press `t` to filter by tags:

- Enter comma-separated tags (e.g., `bug, urgent`)
- Shows tasks matching ANY of the specified tags
- Press `Enter` to apply, `Esc` to cancel

### Scope Filters

Press `o` to filter by scope:

- Enter comma-separated scope paths (e.g., `src/tui, src/cli`)
- Shows tasks matching ANY of the specified scopes
- Press `Enter` to apply, `Esc` to cancel

### Status Filter

Press `f` to cycle through status filters:

- All tasks (no filter)
- Draft only
- Todo only
- Doing only
- Done only
- Rejected only

### Search Options

| Key | Option |
|-----|--------|
| `C` | Toggle case-sensitive search |
| `R` | Toggle regex search |
| `x` | Clear all filters |

### Regex Search

When regex search is enabled (`R`):

- Search patterns are interpreted as regular expressions
- Invalid regex shows an error in the status bar
- Case sensitivity still applies

Example patterns:
```
# Match tasks with IDs RQ-0001 through RQ-0009
RQ-000[1-9]

# Match tasks containing "fix" or "bug"
(fix|bug)

# Match high priority tasks
priority:.*high
```

---

## Command Palette

The command palette provides quick access to all TUI commands. Press `:` or `Ctrl+P` to open it.

### Available Commands

| Command | Description |
|---------|-------------|
| Run selected task | Execute the currently selected task |
| Run next runnable task | Find and run the next task that can execute |
| Start/Stop loop | Toggle loop mode |
| Archive done/rejected tasks | Move terminal tasks to done archive |
| Create new task | Quick create a new task |
| Build task with agent | Create a task using the task builder |
| Edit selected task | Open task editor |
| Edit project config | Open config editor |
| Scan repository for tasks | Run a scan with the given focus |
| Search tasks | Open search prompt |
| Filter by tags | Open tag filter |
| Filter by scope | Open scope filter |
| Clear filters | Remove all active filters |
| Cycle selected task status | Quick status change |
| Cycle selected task priority | Quick priority change |
| Set status: Draft/Todo/Doing/Done/Rejected | Direct status set |
| Set priority: Critical/High/Medium/Low | Direct priority set |
| Toggle case-sensitive search | Toggle search case sensitivity |
| Toggle regex search | Toggle regex mode |
| Toggle fuzzy search | Toggle fuzzy matching |
| Reload queue from disk | Refresh tasks from disk |
| Move selected task up/down | Reorder tasks |
| Jump to task by ID | Quick navigation |
| Repair queue | Validate and fix queue issues |
| Repair queue (dry run) | Validate without modifying |
| Unlock queue | Remove stale queue lock |
| Quit | Exit the TUI |
| Open task scope in editor | Open scope files in `$EDITOR` |
| Copy file:line references | Copy refs to clipboard |

### Fuzzy Matching

The palette uses fuzzy matching to filter commands:

- Type partial words to match command names
- Matches are scored by relevance
- Exact matches rank highest
- Word boundary matches rank higher than substring matches

Examples:
```
"rt" → "Run selected task", "Reload queue"
"arch" → "Archive done/rejected tasks"
"set s" → "Set status: ..." commands
```

---

## Overlays

### Help Overlay

Press `?` or `h` to open the help overlay showing all available keybindings.

**Features:**
- Categorized keybindings (Navigation, Actions, Filters, etc.)
- Scrollable content
- Big "RALPH" ASCII header (when terminal is wide enough)
- Fade-in animation

### Dependency Graph Overlay

Press `v` to view the dependency graph for the selected task.

**Features:**
- Shows upstream dependencies (`depends_on`) by default
- Toggle to show downstream dependents (what this task blocks)
- Critical path highlighting
- Visual status indicators for each task

**Controls:**
| Key | Action |
|-----|--------|
| `v` / `Esc` | Close overlay |
| `d` | Toggle between dependencies and dependents view |
| `c` | Toggle critical path highlighting |

### Flowchart Overlay

Press `f` in execution view to open the workflow flowchart.

**Features:**
- Visual representation of the 3-phase workflow
- Current position indicator
- Phase timing information
- Adapts to configured workflow (1/2/3 phases)

**Phase Indicators:**
| Symbol | Meaning |
|--------|---------|
| `▶` (yellow) | Currently active phase |
| `✓` (green) | Completed phase |
| `○` (gray) | Pending phase |

**Controls:**
| Key | Action |
|-----|--------|
| `f` / `Esc` / `h` / `?` | Close flowchart |

### Parallel State Overlay

Press `P` (uppercase) to view the parallel run state. This is a read-only overlay for monitoring parallel execution.

**Tabs:**

1. **In-Flight Tasks** - Currently running worker tasks:
   - Task ID
   - Workspace path
   - Branch name
   - Process ID (PID)

2. **PRs** - Pull request records:
   - Task ID
   - PR number
   - State (open/closed/merged)
   - Merge blockers (if any)
   - PR URL

3. **Finished Without PR** - Tasks that completed without creating a PR:
   - Task ID
   - Success/failure status
   - Reason code
   - Message

**Controls:**
| Key | Action |
|-----|--------|
| `Esc` / `P` | Close overlay |
| `Tab` / `←` / `→` | Switch between tabs |
| `r` | Reload state from disk |
| `↑` / `↓` / `j` / `k` | Navigate within tab |
| `PgUp` / `PgDn` | Page up/down |
| `Home` / `End` / `g` / `G` | Jump to top/bottom |
| `Enter` / `o` | Open selected PR URL in browser |
| `y` | Copy selected PR URL to clipboard |

When no parallel run is active, the overlay shows instructions on how to start one.

---

## Visual Features

### Markdown Rendering

The TUI supports rich Markdown rendering for task content:

**Supported Elements:**
- **Headings**: `# H1`, `## H2`, `### H3+` - Distinct colors and bold styling
- **Emphasis**: `*italic*`, `**bold**` - Properly styled text emphasis
- **Inline code**: `` `code` `` - Highlighted with distinct background
- **Code blocks**: ` ```language ` - Fenced code blocks with syntax highlighting
- **Lists**:
  - Unordered: `- item`, `* item` - Bullet points
  - Ordered: `1. item`, `2. item` - Numbered lists
- **Line breaks**: Soft and hard breaks handled correctly

**Where Markdown is Rendered:**
- Task Evidence
- Task Plan
- Task Notes
- Task Description (Task Builder)
- Help Overlay

### Syntax Highlighting

Code blocks with language hints receive syntax highlighting:

```markdown
```rust
fn main() {
    println!("Hello, Ralph!");
}
```
```

**Currently Supported Languages:**
- **Rust** (`rust`, `rs`) - Full Tree-sitter-based highlighting

For unsupported languages, code blocks render with monospace styling and a visual gutter.

### Animations

The TUI includes subtle animations for visual polish:

- **Fade-in effect**: Overlays fade in smoothly when opened
- **Frame-based timing**: Consistent timing across terminal refresh rates
- **Graceful degradation**: Disabled when:
  - `NO_COLOR` environment variable is set
  - `TERM=dumb` is detected
  - `RALPH_TUI_NO_ANIM=1` or `true` is set

To disable animations manually:
```bash
RALPH_TUI_NO_ANIM=1 ralph tui
```

### Big Text Headers

Large ASCII art headers appear in select screens:

- **Help overlay**: Big "RALPH" header at the top
- **Empty queue welcome**: Big "RALPH" header when no tasks exist

**Features:**
- Auto-scales to fit terminal width (font sizes: Block → Shade → Slick → Tiny)
- Falls back to plain text on narrow terminals (< 22 columns)
- Gracefully handles very small terminals

---

## Terminal Compatibility

### Color Support

The TUI automatically detects terminal capabilities:

| Mode | Detection |
|------|-----------|
| Truecolor (24-bit) | `COLORTERM=truecolor` or `24bit` |
| 256-color | `TERM` contains `256color` |
| 16-color | Default for most terminals |
| Monochrome | `--color never` or `NO_COLOR` |

Environment variables checked:
- `TERM`
- `COLORTERM`
- `TERM_PROGRAM`
- `NO_COLOR`

### Mouse Support

Mouse support is enabled by default on terminals that support it:

- Click to select tasks
- Scroll wheel for scrolling
- Use `--no-mouse` to disable for terminals with broken mouse support

### Unicode Support

Uses Unicode box-drawing characters by default:

- Full Unicode borders on capable terminals
- `--ascii-borders` for ASCII-only terminals (`+`, `-`, `|`)

### Tested Terminals

| Terminal | Color | Mouse | Unicode | Notes |
|----------|-------|-------|---------|-------|
| iTerm2 (macOS) | Full | Yes | Yes | Primary development target |
| Terminal.app (macOS) | Full | Yes | Yes | Default macOS terminal |
| Windows Terminal | Full | Yes | Yes | Modern Windows terminal |
| GNOME Terminal | Full | Yes | Yes | Common Linux terminal |
| Konsole | Full | Yes | Yes | KDE terminal |
| Alacritty | Full | Yes | Yes | GPU-accelerated terminal |
| WezTerm | Full | Yes | Yes | Modern terminal emulator |
| tmux | Full | Yes | Yes | Terminal multiplexer |
| screen | 16-color | Basic | Yes | Legacy multiplexer |
| VS Code terminal | Full | Yes | Yes | Embedded terminal |

### Compatibility Flags

```bash
# Disable mouse capture
ralph tui --no-mouse

# Force colors even in pipes
ralph tui --color always

# Disable colors entirely
ralph tui --color never

# Use ASCII borders
ralph tui --ascii-borders

# Combine options for maximum compatibility
ralph tui --no-mouse --color never --ascii-borders
```

---

## CLI Parity

### What's Available in TUI vs CLI

| Feature | TUI | CLI |
|---------|-----|-----|
| **Task Management** | | |
| Create task (quick) | `n` | `ralph task` |
| Create task (builder) | `N` | `ralph task build` |
| Edit task | `e` | `ralph task edit` |
| Delete task | `d` | `ralph task delete` |
| Set status | `s` or palette | `ralph task status` |
| Set priority | `p` or palette | `ralph task priority` |
| Archive tasks | `a` | `ralph queue archive` |
| **Execution** | | |
| Run task | `Enter` | `ralph run one` |
| Run loop | `L` | `ralph run loop` |
| Scan | `g` | `ralph scan` |
| **Navigation** | | |
| Search/filter | `/`, `t`, `o`, `f` | `ralph queue list --filter` |
| Jump to ID | `G` | N/A |
| **Views** | | |
| Dependency graph | `v` | `ralph queue graph` |
| Flowchart | `f` (exec view) | N/A |
| Parallel state | `P` | `ralph run status` |
| **Configuration** | | |
| Edit config | `c` | Edit file directly |
| Repair queue | palette | `ralph queue repair` |
| Unlock queue | palette | `ralph queue unlock` |

### Differences from CLI

1. **No direct "operate by TASK_ID" targeting**
   - CLI: `ralph task done RQ-0001`
   - TUI: Navigate to and select the task first

2. **No bulk scripting**
   - CLI can be scripted in shell loops
   - TUI is inherently interactive

3. **No phase-specific overrides in task builder**
   - CLI: Supports `--runner-phaseN`, `--model-phaseN`, `--effort-phaseN`
   - TUI: Task builder only supports global overrides
   - Workaround: Use config `agent.phase_overrides` or CLI instead

---

## TUI-Specific Configuration

### `auto_archive_terminal`

Controls auto-archive behavior when setting tasks to Done/Rejected in the TUI:

```json
{
  "tui": {
    "auto_archive_terminal": "never"
  }
}
```

| Value | Behavior |
|-------|----------|
| `"never"` (default) | No auto-archive; tasks remain in queue until you press `a` |
| `"prompt"` | Ask for confirmation before archiving when setting Done/Rejected |
| `"always"` | Archive immediately when setting Done/Rejected (no confirmation) |

**Note:** This is distinct from `queue.auto_archive_terminal_after_days`, which controls a background sweep that runs on TUI startup/reload.

### Example Configuration

```json
{
  "version": 1,
  "tui": {
    "auto_archive_terminal": "prompt"
  },
  "queue": {
    "auto_archive_terminal_after_days": 7
  }
}
```

---

## Common Workflows

### Creating and Running a Task

```
1. Press `n` to create a quick task
2. Type title: "Fix login bug"
3. Press `Enter`
4. Select the new task
5. Press `e` to edit
6. Add tags: "bug, auth"
7. Add evidence describing the issue
8. Press `Enter` to run the task
```

### Using the Task Builder

```
1. Press `N` to start task builder
2. Type: "Add user authentication with JWT tokens, 
          including login/logout endpoints and middleware"
3. Press `Enter`
4. In advanced options, set:
   - Tags hint: "auth, api"
   - Scope hint: "src/auth, src/middleware"
   - Effort: high
5. Navigate to "[ Build Task ]" and press `Enter`
6. Wait for agent to build task structure
7. Review created task and press `Enter` to run
```

### Filtering and Finding Tasks

```
1. Press `t` to filter by tags
2. Type: "bug, urgent"
3. Press `Enter`
4. Press `/` to search within results
5. Type: "authentication"
6. Press `Enter`
7. Navigate through matching tasks with `↑`/`↓`
8. Press `x` to clear filters when done
```

### Batch Operations with Multi-Select

```
1. Press `m` to enter multi-select mode
2. Navigate to first task to select
3. Press `Space` to select it
4. Navigate to additional tasks
5. Press `Space` to toggle each
6. Press `a` to archive all selected
7. Confirm with `y`
8. Press `Esc` to exit multi-select mode
```

### Monitoring a Long-Running Task

```
1. Select task and press `Enter` to run
2. Watch execution view for progress
3. Press `p` to toggle progress panel if needed
4. Press `f` to view workflow flowchart
5. Press `Esc` to close flowchart, continue monitoring
6. When complete, press `Esc` to return to task list
7. Press `s` to set status to Done
8. Press `a` to archive
```

### Checking Parallel Run Status

```
1. While parallel run is active in another terminal
2. Press `P` to open parallel state overlay
3. Press `Tab` to switch between tabs
4. Navigate to PR of interest with `↑`/`↓`
5. Press `o` to open PR in browser
6. Press `y` to copy PR URL
7. Press `P` or `Esc` to close overlay
```

---

## Troubleshooting

### Display Issues

| Issue | Solution |
|-------|----------|
| Garbled borders | Use `--ascii-borders` |
| Mouse not working | Use `--no-mouse` |
| Wrong colors | Use `--color never` or check `TERM` |
| Animations too slow | Set `RALPH_TUI_NO_ANIM=1` |

### Performance Issues

| Issue | Solution |
|-------|----------|
| Slow with many tasks | Use filters to reduce visible tasks |
| High CPU usage | Disable animations; reduce terminal size |
| Slow scrolling | Reduce terminal height |

### Common Errors

| Error | Cause | Solution |
|-------|-------|----------|
| "Queue locked" | Another ralph process is running | Wait or use `ralph queue unlock` |
| "No task selected" | Tried action with empty queue | Create tasks first |
| "Task not found" | Jumped to non-existent ID | Check ID and try again |
| "Invalid regex" | Bad search pattern | Fix regex syntax |

---

## See Also

- [CLI Reference](../cli.md) - Complete CLI documentation
- [TUI Task Management](../tui-task-management.md) - Detailed task operations guide
- [Configuration](../configuration.md) - Configuration options including TUI settings
- [Queue and Tasks](../queue-and-tasks.md) - Task queue concepts and structure
