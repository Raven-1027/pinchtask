//! Task model definition.

use serde::{Deserialize, Serialize};

/// Represents a task record with metadata and status tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier.
    pub id: String,
    /// Human-readable task name.
    pub name: String,
    /// Detailed task description.
    pub description: String,
    /// Current status of the task.
    pub status: TaskStatus,
    /// List of task IDs this task depends on.
    pub dependencies: Vec<String>,
    /// Ordered list of checklist items.
    pub checklist: Vec<ChecklistItem>,
    /// Free-form notes attached to the task.
    pub notes: Vec<String>,
}

/// Possible states for a task lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is defined but not yet started.
    Todo,
    /// Task is actively being worked on.
    InProgress,
    /// Task has been completed.
    Done,
}

/// A single item within a task's checklist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    /// Description of the checklist step.
    pub description: String,
    /// Whether this step has been completed.
    pub done: bool,
}
