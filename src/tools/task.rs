//! Task-related MCP tool handlers.
//!
//! Each public function corresponds to one MCP tool and will be registered
//! with the server once the transport layer is in place.

use crate::models::task::{Task, TaskStatus};

/// Initialize a new task.
///
/// **Placeholder** — returns an empty `Task` stub.
pub fn initialize_task(
    name: &str,
    description: &str,
) -> Task {
    Task {
        id: uuid_placeholder(),
        name: name.to_owned(),
        description: description.to_owned(),
        status: TaskStatus::Todo,
        dependencies: vec![],
        checklist: vec![],
        notes: vec![],
    }
}

/// Update a task's status.
///
/// **Placeholder** — mutates the given task in place.
pub fn update_task_status(task: &mut Task, status: TaskStatus) {
    task.status = status;
}

/// Generate a placeholder UUID.
///
/// Will be replaced with a proper UUID implementation in a later iteration.
fn uuid_placeholder() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("task-{ts:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_task_sets_todo_status() {
        let task = initialize_task("demo", "a demo task");
        assert_eq!(task.name, "demo");
        assert_eq!(task.status, TaskStatus::Todo);
        assert!(task.id.starts_with("task-"));
    }

    #[test]
    fn update_task_status_transitions() {
        let mut task = initialize_task("demo", "");
        update_task_status(&mut task, TaskStatus::InProgress);
        assert_eq!(task.status, TaskStatus::InProgress);
        update_task_status(&mut task, TaskStatus::Done);
        assert_eq!(task.status, TaskStatus::Done);
    }
}
