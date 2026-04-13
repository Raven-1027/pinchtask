[中文](../zh/mcp.md) | English

# MCP Server

pinchtask exposes all task management capabilities through a [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server. AI agents can invoke tools to create, query, update, and delete tasks, checklist items, notes, resources, and projects.

## Protocol Version

`2024-11-05`

## Launch

```bash
# Explicitly start the MCP server
pinchtask serve

# Or run without subcommand — auto-enters server mode
pinchtask
```

The server listens on **stdio** (stdin/stdout). It does not open a network port.

### Custom Data Directory

```bash
pinchtask serve -D /path/to/data
```

Or via environment variable:

```bash
PINCHTASK_DATA_DIR=/path/to/data pinchtask serve
```

## Transport

The server communicates over stdio using JSON-RPC 2.0 messages. It supports **two input formats** for maximum compatibility:

1. **Newline-delimited JSON** — Each JSON-RPC message is terminated by a newline (`\n`). This is the simplest format and works with most clients.

2. **Content-Length header** — Each message is prefixed with a `Content-Length` header followed by `\r\n\r\n`, similar to the LSP protocol:
   ```
   Content-Length: 123\r\n
   \r\n
   {"jsonrpc":"2.0","id":1,"method":"tools/list",...}
   ```

The server auto-detects which format each incoming message uses. Responses are always sent as newline-delimited JSON.

## Configuration Examples

### Claude Desktop

Add to your Claude Desktop `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "pinchtask": {
      "command": "pinchtask",
      "args": ["serve"],
      "env": {
        "PINCHTASK_DATA_DIR": "/path/to/data"
      }
    }
  }
}
```

### Cursor

Add to your MCP settings:

```json
{
  "mcpServers": {
    "pinchtask": {
      "command": "pinchtask",
      "args": ["serve"]
    }
  }
}
```

### Generic MCP Client

Any MCP-compatible client that supports stdio transport can connect to pinchtask. Provide the path to the `pinchtask` binary and `["serve"]` as arguments.

## Short ID Prefix Matching

All tools that accept a `task_id` or `project_id` parameter support UUID short-prefix matching. Instead of providing the full UUID, you can type just the first 4 or more characters and the system will automatically resolve it to the unique match.

- Prefixes shorter than 4 characters will result in an error.
- When there is exactly one match, the prefix is automatically resolved to the full UUID.
- When there is no match, a "not found" error is returned.
When multiple matches exist, the first 10 candidate tasks/projects are listed, prompting you to provide more characters to disambiguate.

## Workspace Project Association (.pinchproject)

Place a `.pinchproject` file in a project root directory (containing the project UUID) to enable workspace association.

**MCP layer**: `new_task` and `list_tasks` no longer auto-inject workspace project_id. `project_id` must be explicitly provided.

**CLI layer**: `task new` and `task ls` still support auto-using the project ID from `.pinchproject` when `--project` is not specified. See the CLI documentation for details.

## Tools

The server registers **9 tools**. All tool schemas are fully inlined (no `$ref` references) for maximum client compatibility.

---

### `new_task`

Create a new task with a description, optional checklist items, notes, resources, metadata, and project association.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `task_description` | `string` | Yes | A medium-level detailed description about the whole task |
| `context_for_all_tasks` | `string` | No | Information that all tasks in the checklist should include (e.g., tech stack, constraints) |
| `initial_checklist` | `array` | No | Optional initial checklist items |
| `notes` | `array<string>` | No | Optional initial notes |
| `resources` | `array` | No | Optional initial resources |
| `metadata` | `object` | No | Optional metadata (tags, priority, estimated completion time) |
| `project_id` | `string` | Yes | Project ID to associate the task with at creation time (supports short ID prefix matching) |

**`initial_checklist` item structure:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `task` | `string` | Yes | Short name for the checklist item |
| `detailed_description` | `string` | Yes | Detailed description |
| `context_and_plan` | `string` | No | Context and plan |
| `done` | `boolean` | No | Whether the item is already done (default: `false`) |
| `id` | `string` | No | Optional pre-assigned UUID; auto-generated if omitted |

**`resources` item structure:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `name` | `string` | Yes | Resource name |
| `url` | `string` | Yes | Resource URL or file path |
| `description` | `string` | No | Resource description |

**`metadata` structure:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `tags` | `array<string>` | No | Tag list |
| `priority` | `string` | No | Priority level: `high` / `medium` / `low` |
| `estimated_completion_time` | `string` | No | Estimated completion time (ISO timestamp or duration) |

**Returns:** The full task object as JSON.

**Example:**

```json
{
  "task_description": "Implement user authentication",
  "context_for_all_tasks": "Use JWT tokens. Backend is Rust with Axum.",
  "initial_checklist": [
    {
      "task": "Design database schema",
      "detailed_description": "Create users and sessions tables",
      "done": false
    },
    {
      "task": "Implement login endpoint",
      "detailed_description": "POST /api/auth/login",
      "done": false
    }
  ],
  "metadata": {
    "priority": "high",
    "tags": ["backend", "auth"]
  }
}
```

**Example with project association:**

```json
{
  "task_description": "Fix login bug",
  "project_id": "def67890-1234-5678-abcd-ef0123456789",
  "metadata": {
    "priority": "high"
  }
}
```

---

### `update_task`

Update task-level fields: description, context, priority, tags, estimated completion time, and/or project association. Only specified fields are modified. At least one field must be provided.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `task_id` | `string` | Yes | The ID of the task to update (supports short ID prefix matching) |
| `task_description` | `string` | No | The new task description |
| `context_for_all_tasks` | `string` | No | The new context information |
| `priority` | `string` | No | Priority level: `high` / `medium` / `low` |
| `tags` | `string` | No | Comma-separated tags (e.g., `"backend,auth,urgent"`) |
| `eta` | `string` | No | Estimated completion time (ISO timestamp or duration) |
| `project_id` | `string` or `null` | No | Project ID to associate the task with (supports short ID prefix matching). Pass a project UUID to assign, pass `null` to remove from project, omit to keep unchanged |

**Returns:** The updated task object as JSON.

**Example — Assign task to a project:**

```json
{
  "task_id": "abc12345-...",
  "project_id": "def67890-..."
}
```

**Example — Remove task from project:**

```json
{
  "task_id": "abc12345-...",
  "project_id": null
}
```

---

### `manage_checklist_item`

Perform operations on checklist items. This is the single entry point for all checklist item operations. **All checklist item indices are 0-based** (the first item has index 0).

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `action` | `string` | Yes | Operation: `"add"`, `"update"`, `"reorder"`, `"remove"`, or `"batch_update"` |
| `task_id` | `string` | Yes | The ID of the task (supports short ID prefix matching) |
| `task` | `string` | For `add` | Short yet comprehensive name for the item |
| `detailed_description` | `string` | For `add` | Longer description about what to achieve |
| `index` | `integer` | For `update`, `remove` | 0-based index of the checklist item |
| `from_index` | `integer` | For `reorder` | Current 0-based index |
| `to_index` | `integer` | For `reorder` | New 0-based index |
| `context_and_plan` | `string` or `null` | For `update` | Related info and plan. Pass `null` to clear, omit to keep unchanged |
| `done` | `boolean` | For `update` | `true` = mark done, `false` = mark undone |
| `updates` | `array` | For `batch_update` | List of item updates to apply sequentially |

**Action details:**

- **`add`** — Append a new item. Requires `task` and `detailed_description`.
- **`update`** — Modify an existing item. Requires `index`. Only specified fields are changed.
- **`reorder`** — Move an item. Requires `from_index` and `to_index`. After reordering, indices change.
- **`remove`** — Delete an item. Requires `index`. After removal, subsequent indices shift down by 1.
- **`batch_update`** — Update multiple items in a single request. Provide `updates` array, each element specifies `index` and fields to change. Items are updated sequentially. Useful for bulk marking completion.

**Returns:** The updated task object as JSON.

**Example — Add an item:**

```json
{
  "action": "add",
  "task_id": "abc12345-...",
  "task": "Write unit tests",
  "detailed_description": "Cover login and registration flows"
}
```

**Example — Mark item as done:**

```json
{
  "action": "update",
  "task_id": "abc12345-...",
  "index": 2,
  "done": true
}
```

**Example — Reorder item:**

```json
{
  "action": "reorder",
  "task_id": "abc12345-...",
  "from_index": 0,
  "to_index": 2
}
```

**Example — Batch update items:**

```json
{
  "action": "batch_update",
  "task_id": "abc12345-...",
  "updates": [
    {"index": 0, "done": true},
    {"index": 1, "done": true},
    {"index": 2, "done": true}
  ]
}
```

**`updates` item structure:**

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `index` | `integer` | Yes | 0-based index of the checklist item |
| `task` | `string` | No | New short name |
| `detailed_description` | `string` | No | New detailed description |
| `context_and_plan` | `string` or `null` | No | New context and plan (`null` to clear) |
| `done` | `boolean` | No | Whether the item is done |

---

### `add_note`

Add a note to a task. Notes are append-only and useful for recording discoveries, decisions, or context that doesn't fit into checklist items.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `task_id` | `string` | Yes | The ID of the task (supports short ID prefix matching) |
| `content` | `string` | Yes | The content of the note |

**Returns:** The updated task object as JSON.

---

### `add_resource`

Add a resource reference to a task (append-only). Use this to link relevant files, documentation URLs, or API references.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `task_id` | `string` | Yes | The ID of the task (supports short ID prefix matching) |
| `name` | `string` | Yes | Name of the resource |
| `url` | `string` | Yes | URL or file path of the resource |
| `description` | `string` | No | Description of the resource |

**Returns:** The updated task object as JSON.

**Example:**

```json
{
  "task_id": "abc12345-...",
  "name": "API Documentation",
  "url": "https://docs.example.com/api/v2",
  "description": "REST API reference for the new endpoints"
}
```

---

### `get_checklist_summary`

Get a summary of the task checklist with completion status. Useful for a quick progress overview.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `task_id` | `string` | Yes | The ID of the task (supports short ID prefix matching) |
| `include_descriptions` | `boolean` | No | Whether to include detailed descriptions alongside item names (default: `false`) |

**Returns:** A text summary of checklist progress.

---

### `clear_task`

Delete a task by its ID. **This is an irreversible operation.** Confirm the `task_id` before calling.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `task_id` | `string` | Yes | The ID of the task to delete (supports short ID prefix matching) |

**Returns:** A confirmation message.

---

### `list_tasks`

List tasks grouped by status. Tasks are organized into three groups — In Progress → Not Started → Completed — with items sorted by priority (high > medium > low) within each group. When the total number of tasks exceeds 10, the Not Started and Completed groups are automatically truncated to show only the first 3 items, along with a summary count.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `project_id` | `string` | Yes | Project ID (supports short ID prefix matching). Pass `"*"` to query across all projects. |

**Returns:** A concise text summary grouped by status (not full details). Use `new_task` or individual task queries for full information.

**Example — Query a specific project:**

```json
{
  "project_id": "def67890-..."
}
```

**Example — Query across all projects:**

```json
{
  "project_id": "*"
}
```

---

### `manage_project`

Perform operations on projects. Projects are containers for organizing related tasks. Each task can belong to at most one project. Typical workflow: use `list` to find existing projects, then create tasks under a project via `new_task` (with `project_id`), or assign existing tasks via `update_task` (with `project_id`).

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `action` | `string` | Yes | Operation: `"create"`, `"get"`, `"update"`, `"delete"`, or `"list"` |
| `project_id` | `string` | For `get`, `update`, `delete` | The ID of the project (supports short ID prefix matching) |
| `name` | `string` | For `create`; optional for `update` | The name of the project |
| `description` | `string` | No | The description of the project (optional for `create`/`update`) |
| `delete_tasks` | `boolean` | No (for `delete` only) | Whether to also delete all associated tasks (default: `false`) |

**Action details:**

- **`create`** — Create a new project. Requires `name`.
- **`get`** — Get a project by its ID. Requires `project_id`.
- **`update`** — Update a project's name and/or description. Requires `project_id`.
- **`delete`** — Delete a project. Requires `project_id`. If `delete_tasks` is `true`, all associated tasks are also deleted; otherwise tasks are kept with `project_id` cleared.
- **`list`** — List all projects. No additional parameters needed.

**Returns:** For `create`/`get`/`update`: the project object as JSON. For `list`: an array of project objects. For `delete`: a confirmation message.

**Example — Create a project:**

```json
{
  "action": "create",
  "name": "Website Redesign",
  "description": "Q3 2025 website overhaul project"
}
```

**Example — Delete a project and its tasks:**

```json
{
  "action": "delete",
  "project_id": "def67890-...",
  "delete_tasks": true
}
```

## Task-Project Relationship

Tasks and projects have a **one-to-many** relationship. Each task belongs to at most one project (via the `project_id` foreign key). When a project is deleted without `delete_tasks: true`, its tasks are preserved but their `project_id` is set to `null`.

## Error Handling

When an operation fails, the server returns a JSON-RPC error response with an `error` object containing:
- `code` — Error code
- `message` — Human-readable error description

Common errors include:
- Task or project not found (invalid ID)
- Index out of bounds for checklist operations
- Missing required parameters
- Database errors
