[中文](../zh/tui.md) | English

# TUI (Terminal User Interface)

The interactive terminal interface lets you manage tasks and projects entirely with keyboard shortcuts, without leaving the terminal.

## Launch

The TUI requires the `tui` feature to be enabled at build time:

```bash
# Build with TUI support
cargo build --features tui

# Launch the TUI
pinchtask tui

# Specify a custom data directory
pinchtask tui -D /path/to/data
```

If a `.pinchproject` file exists in the current directory or any parent directory, the TUI automatically selects the corresponding project on launch.

## Views

The TUI uses a split-pane layout with optional overlay dialogs.

### Left Pane — Project List

Displays all projects. Selecting a project loads its associated tasks in the right pane.

### Right Pane — Task List

Shows tasks belonging to the selected project. Supports sorting, filtering via search, and task creation/deletion.

### Right Pane — Task Detail

Displays the full details of a selected task: description, context, metadata, checklist items, notes, and resources. All checklist operations (add, edit, delete, reorder, toggle) are performed here.

### Task Form

A modal form for creating or editing tasks. Fields include: Description, Context, Priority, Tags, ETA. New tasks are automatically associated with the currently selected project.

### Help Panel

Displays available keyboard shortcuts for the current view.

## Task List Shortcuts

| Key | Action |
|-----|--------|
| `↑` / `k` | Move selection up |
| `↓` / `j` | Move selection down |
| `Enter` | View task detail |
| `n` | Create new task |
| `d` | Delete selected task |
| `r` | Refresh task list |
| `Tab` | Cycle sort mode (Created → Priority → Updated) |
| `/` | Enter search mode |
| `Home` | Jump to first task |
| `End` | Jump to last task |
| `←` / `Esc` | Switch to left pane (project list) |
| `?` | Show help |

## Task Detail Shortcuts

| Key | Action |
|-----|--------|
| `↑` / `k` | Move focus to previous checklist item |
| `↓` / `j` | Move focus to next checklist item |
| `Space` / `x` | Toggle completion of focused item |
| `a` | Add new checklist item |
| `e` | Edit focused item name |
| `d` | Delete focused checklist item |
| `E` | Open task edit form |
| `N` | Add a note |
| `D` | Delete a note |
| `L` | Add a resource link (two-step: name → URL) |
| `Ctrl+J` | Move focused item down (reorder) |
| `Ctrl+K` | Move focused item up (reorder) |
| `Ctrl+D` | Delete the entire task |
| `←` / `Esc` | Return to task list |

## Global Shortcuts

| Key | Action |
|-----|--------|
| `Ctrl+C` | Quit immediately |
| `q` | Quit (disabled in form and help views) |
| `?` | Toggle help panel |
| `Esc` | Close help panel / go back |

## Form Shortcuts (Task Form / Project Form)

| Key | Action |
|-----|--------|
| `Tab` | Move to next field |
| `Shift+Tab` | Move to previous field |
| `Enter` | Submit the form |
| `Esc` | Cancel and close the form |
| `Backspace` | Delete last character |
| Character keys | Type into the focused field |

**Task Form — Priority field only:**

| Key | Action |
|-----|--------|
| `Space` / `←` / `→` | Cycle through priority values (empty → low → medium → high) |

## Search Mode

Activated by pressing `/` in the task list. Tasks are filtered in real-time as you type.

| Key | Action |
|-----|--------|
| Character keys | Append to search query |
| `Backspace` | Delete last character |
| `Enter` | Confirm and exit search |
| `Esc` | Cancel and clear search |

## Delete Confirmation

When a delete action is triggered, a confirmation prompt appears.

| Key | Action |
|-----|--------|
| `y` / `Y` | Confirm deletion |
| `n` / `N` / `Esc` | Cancel |

## Data Directory

By default, task data is stored in `~/.pinchtask/tasks.db`. You can customize this location:

- **CLI flag:** `pinchtask tui -D /path/to/data`
- **Environment variable:** `PINCHTASK_DATA_DIR=/path/to/data pinchtask tui`

The CLI flag takes precedence over the environment variable.
