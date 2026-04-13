[中文](../zh/cli.md) | English

# CLI Reference

pinchtask provides a full-featured command-line interface for managing tasks, checklist items, notes, resource references, and projects. All commands support short ID prefix matching (minimum 4 characters) for task and project IDs.

## Synopsis

```bash
pinchtask [OPTIONS] [COMMAND]
```

## Global Options

| Option | Short | Description |
|--------|-------|-------------|
| `--data-dir <DIR>` | `-D` | Data storage directory (default: `~/.pinchtask`). Can also be set via `PINCHTASK_DATA_DIR` environment variable. |
| `--log-level <LEVEL>` | | Log level: `trace`, `debug`, `info`, `warn`, `error`. Can also be set via `PINCHTASK_LOG_LEVEL` environment variable. |
| `--verbose` | `-v` | Verbose output (equivalent to `--log-level debug`). Conflicts with `--quiet`. |
| `--quiet` | `-q` | Quiet mode (equivalent to `--log-level error`). Conflicts with `--verbose`. |
| `--json` | | Output in JSON format. Works with all query commands (`ls`, `show`, `summary`). |

## Commands

### `task` — Task Management

#### `task new` — Create a New Task

```bash
pinchtask task new <DESCRIPTION> [OPTIONS]
```

| Option | Short | Description |
|--------|-------|-------------|
| `<DESCRIPTION>` | | Task description (required, positional) |
| `--context <TEXT>` | `-c` | Shared context for all sub-tasks |
| `--project <ID>` | `-p` | Associate with a project (short ID prefix supported) |

**Examples:**

```bash
# Create a simple task
pinchtask task new "Implement user login"

# Create a task with context and project association
pinchtask task new "Set up CI pipeline" -c "Use GitHub Actions" -p abcd1234
```

#### `task ls` — List Tasks

```bash
pinchtask task ls [OPTIONS]
```

| Option | Short | Description |
|--------|-------|-------------|
| `--all` | `-a` | Show all tasks (active + completed). Conflicts with `--done`. |
| `--done` | | Show only completed tasks. Conflicts with `--all`. |
| `--long` | `-l` | Detailed mode (shows priority, tags, creation date). |
| `--limit <N>` | `-n` | Maximum number of tasks to display (default: 10). |
| `--sort <FIELD>` | | Sort field: `time` (default), `priority`, `progress`. |
| `--project <ID>` | `-p` | Filter by project (short ID prefix supported). |

**Default behavior:** Shows active tasks (tasks with no checklist, or tasks with at least one incomplete item).

**Examples:**

```bash
# List active tasks
pinchtask task ls

# List all tasks in detailed mode
pinchtask task ls --all --long

# List completed tasks sorted by priority
pinchtask task ls --done --sort priority

# Show up to 50 tasks
pinchtask task ls -n 50

# Filter by project
pinchtask task ls -p abcd1234

# Filter by project, show all tasks
pinchtask task ls --all -p abcd1234
```

#### `task show` — View Task Details

```bash
pinchtask task show <ID>
```

| Argument | Description |
|----------|-------------|
| `<ID>` | Task ID (short prefix, minimum 4 characters) |

Displays full task details including description, context, metadata, checklist items, notes, and resources.

**Example:**

```bash
pinchtask task show a1b2c3d4
```

#### `task edit` — Edit Task

```bash
pinchtask task edit <ID> [OPTIONS]
```

| Option | Short | Description |
|--------|-------|-------------|
| `<ID>` | | Task ID (short prefix supported) |
| `--description <TEXT>` | `-d` | New task description |
| `--context <TEXT>` | `-c` | New shared context |
| `--priority <LEVEL>` | | Priority level: `high`, `medium`, `low` |
| `--tags <TAGS>` | | Comma-separated tags |
| `--eta <TIME>` | | Estimated completion time (ISO 8601 format) |

At least one option must be specified.

**Examples:**

```bash
# Update description and priority
pinchtask task edit a1b2c3d4 -d "New description" --priority high

# Add tags
pinchtask task edit a1b2c3d4 --tags "frontend,urgent"

# Set estimated completion time
pinchtask task edit a1b2c3d4 --eta "2025-02-01T18:00:00Z"
```

#### `task rm` — Delete Task

```bash
pinchtask task rm <ID>
```

| Argument | Description |
|----------|-------------|
| `<ID>` | Task ID (short prefix supported) |

**Example:**

```bash
pinchtask task rm a1b2c3d4
```

---

### `item` — Checklist Item Management

All `item` commands require a task ID as the first argument.

#### `item new` — Add a Checklist Item

```bash
pinchtask item new <TASK_ID> <TITLE> [OPTIONS]
```

| Option | Short | Description |
|--------|-------|-------------|
| `<TASK_ID>` | | Task ID (short prefix supported) |
| `<TITLE>` | | Item title (required, positional) |
| `--description <TEXT>` | `-d` | Detailed description (default: empty) |
| `--plan <TEXT>` | `-p` | Context and plan |

**Example:**

```bash
pinchtask item new a1b2c3d4 "Design database schema" -d "Users table and sessions table" -p "Follow normalization rules"
```

#### `item check` — Toggle Item Completion

```bash
pinchtask item check <TASK_ID> <INDEX>
```

| Argument | Description |
|----------|-------------|
| `<TASK_ID>` | Task ID (short prefix supported) |
| `<INDEX>` | 0-based item index |

Toggles the completion status of the specified item. If the item is done, it becomes undone; if undone, it becomes done.

**Example:**

```bash
pinchtask item check a1b2c3d4 0
```

#### `item edit` — Edit a Checklist Item

```bash
pinchtask item edit <TASK_ID> <INDEX> [OPTIONS]
```

| Option | Short | Description |
|--------|-------|-------------|
| `<TASK_ID>` | | Task ID (short prefix supported) |
| `<INDEX>` | | 0-based item index |
| `--title <TEXT>` | `-t` | New item title |
| `--description <TEXT>` | `-d` | New detailed description |
| `--plan <TEXT>` | `-p` | New context and plan |
| `--done` | | Mark as completed (conflicts with `--undone`) |
| `--undone` | | Mark as not completed (conflicts with `--done`) |

**Examples:**

```bash
# Rename an item
pinchtask item edit a1b2c3d4 0 -t "Updated item name"

# Mark as done and update description
pinchtask item edit a1b2c3d4 0 --done -d "New description"
```

#### `item mv` — Reorder a Checklist Item

```bash
pinchtask item mv <TASK_ID> <FROM> <TO>
```

| Argument | Description |
|----------|-------------|
| `<TASK_ID>` | Task ID (short prefix supported) |
| `<FROM>` | Current 0-based index |
| `<TO>` | Target 0-based index |

After reordering, indices change. Refresh task data before further index operations.

**Example:**

```bash
# Move item from index 2 to index 0
pinchtask item mv a1b2c3d4 2 0
```

#### `item rm` — Delete a Checklist Item

```bash
pinchtask item rm <TASK_ID> <INDEX>
```

| Argument | Description |
|----------|-------------|
| `<TASK_ID>` | Task ID (short prefix supported) |
| `<INDEX>` | 0-based item index |

After removal, subsequent indices shift down by 1.

**Example:**

```bash
pinchtask item rm a1b2c3d4 1
```

#### `item summary` — View Checklist Progress

```bash
pinchtask item summary <TASK_ID>
```

| Argument | Description |
|----------|-------------|
| `<TASK_ID>` | Task ID (short prefix supported) |

Displays a summary of checklist completion status.

**Example:**

```bash
pinchtask item summary a1b2c3d4
```

---

### `note` — Note Management

#### `note new` — Add a Note

```bash
pinchtask note new <TASK_ID> <CONTENT>
```

| Argument | Description |
|----------|-------------|
| `<TASK_ID>` | Task ID (short prefix supported) |
| `<CONTENT>` | Note content (required, positional) |

Notes are append-only and useful for recording discoveries, decisions, or context that doesn't fit into checklist items.

**Example:**

```bash
pinchtask note new a1b2c3d4 "Remember to update the API documentation after changes"
```

---

### `link` — Resource Reference Management

#### `link new` — Add a Resource Reference

```bash
pinchtask link new <TASK_ID> --name <NAME> --url <URL> [OPTIONS]
```

| Option | Short | Description |
|--------|-------|-------------|
| `<TASK_ID>` | | Task ID (short prefix supported) |
| `--name <NAME>` | | Resource name (required) |
| `--url <URL>` | | Resource URL or file path (required) |
| `--description <TEXT>` | `-d` | Resource description |

**Examples:**

```bash
# Add a URL reference
pinchtask link new a1b2c3d4 --name "API Docs" --url "https://api.example.com/docs" -d "REST API reference"

# Add a local file reference
pinchtask link new a1b2c3d4 --name "Schema" --url "/path/to/schema.sql"
```

---

### `project` — Project Management

Projects are containers for organizing related tasks. Each task can belong to at most one project.

#### `project new` — Create a Project

```bash
pinchtask project new <NAME> [OPTIONS]
```

| Option | Short | Description |
|--------|-------|-------------|
| `<NAME>` | | Project name (required, positional) |
| `--description <TEXT>` | `-d` | Project description |

**Example:**

```bash
pinchtask project new "Website Redesign" -d "Complete overhaul of the company website"
```

#### `project ls` — List Projects

```bash
pinchtask project ls
```

Lists all projects with their short IDs and names.

**Example:**

```bash
pinchtask project ls
```

#### `project show` — View Project Details

```bash
pinchtask project show <ID>
```

| Argument | Description |
|----------|-------------|
| `<ID>` | Project ID (short prefix, minimum 4 characters) |

Displays project details including associated tasks with their progress.

**Example:**

```bash
pinchtask project show abcd1234
```

#### `project rm` — Delete a Project

```bash
pinchtask project rm <ID> [OPTIONS]
```

| Option | Short | Description |
|--------|-------|-------------|
| `<ID>` | | Project ID (short prefix supported) |
| `--with-tasks` | | Also delete all associated tasks |

By default, deleting a project keeps its tasks (with `project_id` cleared). Use `--with-tasks` to delete associated tasks as well.

**Examples:**

```bash
# Delete project, keep tasks
pinchtask project rm abcd1234

# Delete project and all its tasks
pinchtask project rm abcd1234 --with-tasks
```

#### `project add-task` — Add Task to Project

```bash
pinchtask project add-task <PROJECT_ID> <TASK_ID>
```

| Argument | Description |
|----------|-------------|
| `<PROJECT_ID>` | Project ID (short prefix supported) |
| `<TASK_ID>` | Task ID (short prefix supported) |

**Example:**

```bash
pinchtask project add-task abcd1234 a1b2c3d4
```

#### `project rm-task` — Remove Task from Project

```bash
pinchtask project rm-task <PROJECT_ID> <TASK_ID>
```

| Argument | Description |
|----------|-------------|
| `<PROJECT_ID>` | Project ID (short prefix supported) |
| `<TASK_ID>` | Task ID (short prefix supported) |

Removes the task from the project but does not delete the task itself.

**Example:**

```bash
pinchtask project rm-task abcd1234 a1b2c3d4
```

#### `project init` — Initialize Workspace Project File

```bash
pinchtask project init <PROJECT_ID> [OPTIONS]
```

| Option | Short | Description |
|--------|-------|-------------|
| `<PROJECT_ID>` | | Project ID (short prefix supported) |
| `--force` | `-f` | Overwrite an existing `.pinchproject` file |

Creates a `.pinchproject` file in the current directory containing the specified project ID. This enables automatic project association for CLI, TUI, and MCP operations in this directory and its subdirectories.

If a `.pinchproject` file already exists, the command will fail unless `--force` is specified.

**Examples:**

```bash
# Initialize workspace for a project
pinchtask project init abcd1234

# Overwrite existing .pinchproject file
pinchtask project init abcd1234 --force
```

---

### `serve` — Start MCP Server

```bash
pinchtask serve [OPTIONS]
```

Starts the MCP server using stdio transport. This is the mode used by AI clients.

If no subcommand is provided when running `pinchtask`, it automatically enters server mode.

**Example:**

```bash
pinchtask serve
# Equivalent to:
pinchtask
```

---

### `completion` — Generate Shell Completions

```bash
pinchtask completion <SHELL>
```

| Argument | Description |
|----------|-------------|
| `<SHELL>` | Target shell: `bash`, `zsh`, `fish`, `powershell`, `elvish` |

**Examples:**

```bash
# Generate bash completions
pinchtask completion bash > /etc/bash_completion.d/pinchtask

# Generate zsh completions
pinchtask completion zsh > ~/.zsh/completions/_pinchtask
```

---

## Workspace Project Association (.pinchproject)

Place a `.pinchproject` file in your project root directory containing the project UUID (supports `#` comments):

```
# .pinchproject
550e8400-e29b-41d4-a716-446655440000
```

Alternatively, you can create this file using the `project init` command:

```bash
pinchtask project init abcd1234
```

When running CLI commands from this directory (or any subdirectory), the project ID from the file is automatically used:

- `task new` — If `--project` is not specified, the task is automatically associated with the project from `.pinchproject`
- `task ls` — If `--project` is not specified, tasks are automatically filtered by that project

**Priority**: Explicit `--project` > `.pinchproject` > No project

The system searches upward from the current directory for `.pinchproject` files and uses the nearest one.

## Short ID Matching

Instead of typing the full UUID, you can use the first 4+ characters of a task or project ID. The system will:

- **Unique match:** Automatically resolve to the full ID.
- **No match:** Report an error indicating the task/project was not found.
- **Multiple matches:** Report an error listing all candidates so you can provide more characters.

```bash
# Instead of the full UUID: a1b2c3d4-e5f6-7890-abcd-ef1234567890
# You can use:
pinchtask task show a1b2
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Task or project not found |
| 3 | Database or configuration error |

## JSON Output

All query commands support the `--json` flag for machine-readable output:

```bash
# Get task list as JSON
pinchtask task ls --json

# Get task details as JSON
pinchtask task show a1b2c3d4 --json

# Get checklist summary as JSON
pinchtask item summary a1b2c3d4 --json

# Get project list as JSON
pinchtask project ls --json
```
