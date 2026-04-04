//! MCP 工具参数结构体定义。
//!
//! 为每个 MCP 工具定义强类型的参数结构体，
//! 实现 `Deserialize` 用于 JSON 反序列化，`JsonSchema` 用于自动生成 inputSchema。

use schemars::JsonSchema;
use serde::Deserialize;

// ---------------------------------------------------------------------------
// 辅助 / 嵌套类型
// ---------------------------------------------------------------------------

/// 初始化任务时的清单条目输入。
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InitialChecklistItem {
    #[schemars(description = "Short name for the checklist item")]
    pub task: String,
    #[schemars(description = "Detailed description")]
    pub detailed_description: String,
    #[serde(default)]
    #[schemars(description = "Context and plan")]
    pub context_and_plan: Option<String>,
    #[serde(default)]
    #[schemars(description = "Whether the item is already done")]
    pub done: bool,
    #[serde(default)]
    #[schemars(description = "Optional pre-assigned ID (UUID); auto-generated if omitted")]
    pub id: Option<String>,
}

impl Default for InitialChecklistItem {
    fn default() -> Self {
        Self {
            task: String::new(),
            detailed_description: String::new(),
            context_and_plan: None,
            done: false,
            id: None,
        }
    }
}

/// 资源引用输入。
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ResourceInput {
    #[schemars(description = "Resource name")]
    pub name: String,
    #[schemars(description = "Resource URL or file path")]
    pub url: String,
    #[serde(default)]
    #[schemars(description = "Resource description")]
    pub description: Option<String>,
}

/// 任务元数据输入。
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct TaskMetadataInput {
    #[serde(default)]
    #[schemars(description = "Tag list")]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    #[schemars(description = "Priority level: high / medium / low")]
    pub priority: Option<String>,
    #[serde(default)]
    #[schemars(description = "Estimated completion time (ISO timestamp or duration)")]
    pub estimated_completion_time: Option<String>,
}

impl Default for TaskMetadataInput {
    fn default() -> Self {
        Self {
            tags: None,
            priority: None,
            estimated_completion_time: None,
        }
    }
}

// ---------------------------------------------------------------------------
// 工具参数结构体（按 server.rs 中 register_builtin_tools 的顺序）
// ---------------------------------------------------------------------------

/// `initialize_task` 参数。
#[derive(Debug, Deserialize, JsonSchema)]
pub struct InitializeTaskParams {
    #[schemars(description = "A medium-level detailed description about the whole task")]
    pub task_description: String,
    #[serde(default)]
    #[schemars(description = "Information that all tasks in the checklist should include")]
    pub context_for_all_tasks: Option<String>,
    #[serde(default)]
    #[schemars(description = "Optional initial checklist items")]
    pub initial_checklist: Option<Vec<InitialChecklistItem>>,
    #[serde(default)]
    #[schemars(description = "Optional initial notes")]
    pub notes: Option<Vec<String>>,
    #[serde(default)]
    #[schemars(description = "Optional initial resources")]
    pub resources: Option<Vec<ResourceInput>>,
    #[serde(default)]
    #[schemars(description = "Optional metadata for the task")]
    pub metadata: Option<TaskMetadataInput>,
}

/// `update_task` 参数（统一更新多个字段）。
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateTaskParams {
    #[schemars(description = "The ID of the task to update")]
    pub task_id: String,
    #[serde(default)]
    #[schemars(description = "The new task description")]
    pub task_description: Option<String>,
    #[serde(default)]
    #[schemars(description = "The new context information")]
    pub context_for_all_tasks: Option<String>,
    #[serde(default)]
    #[schemars(description = "Priority level: high / medium / low")]
    pub priority: Option<String>,
    #[serde(default)]
    #[schemars(description = "Comma-separated tags")]
    pub tags: Option<String>,
    #[serde(default)]
    #[schemars(description = "Estimated completion time (ISO timestamp or duration)")]
    pub eta: Option<String>,
}

/// Action type for checklist item operations.
///
/// Serialized as lowercase strings: "add", "update", "reorder", "remove".
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// Append a new item to the end of the checklist.
    Add,
    /// Modify an existing item's fields. Only specified fields are changed.
    Update,
    /// Move an item to a new position. After reordering, indices change.
    Reorder,
    /// Delete an item. After removal, subsequent indices shift down by 1.
    Remove,
}

/// `manage_checklist_item` 参数。
///
/// 扁平结构，所有操作共享一个 struct，通过 `action` 字段区分操作类型。
/// `action` 是必填字段，其他字段根据 action 类型有不同含义。
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ManageChecklistItemParams {
    /// The operation to perform. Must be one of: "add", "update", "reorder", "remove".
    #[schemars(
        description = "The operation to perform. Must be one of: \"add\", \"update\", \"reorder\", \"remove\""
    )]
    pub action: Action,

    #[schemars(description = "The ID of the task")]
    pub task_id: String,

    // --- Add 专用 ---
    #[serde(default)]
    #[schemars(description = "A short yet comprehensive name for the item (required for Add)")]
    pub task: Option<String>,
    #[serde(default)]
    #[schemars(
        description = "A longer description about what we want to achieve (required for Add)"
    )]
    pub detailed_description: Option<String>,

    // --- Update / Remove 专用 ---
    #[serde(default)]
    #[schemars(description = "0-based index of the checklist item (required for Update/Remove)")]
    pub index: Option<u64>,

    // --- Reorder 专用 ---
    #[serde(default)]
    #[schemars(description = "Current 0-based index (required for Reorder)")]
    pub from_index: Option<u64>,
    #[serde(default)]
    #[schemars(description = "New 0-based index (required for Reorder)")]
    pub to_index: Option<u64>,

    // --- Update 专用 ---
    /// 三态语义：字段未传入 → `None` → 不修改；传入 `null` → `Some(None)` → 清空；传入字符串 → `Some(Some("..."))` → 更新。
    #[serde(default)]
    #[schemars(
        description = "Related information and a detailed plan (pass null to clear, omit to keep unchanged)"
    )]
    pub context_and_plan: Option<Option<String>>,
    #[serde(default)]
    #[schemars(
        description = "Whether the item is completed (for Update only). true=done, false=undone"
    )]
    pub done: Option<bool>,
}

impl Default for ManageChecklistItemParams {
    fn default() -> Self {
        Self {
            action: Action::Add,
            task_id: String::new(),
            task: None,
            detailed_description: None,
            index: None,
            from_index: None,
            to_index: None,
            context_and_plan: None,
            done: None,
        }
    }
}

/// `add_note` 参数。
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddNoteParams {
    #[schemars(description = "The ID of the task")]
    pub task_id: String,
    #[schemars(description = "The content of the note")]
    pub content: String,
}

/// `add_resource` 参数。
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddResourceParams {
    #[schemars(description = "The ID of the task")]
    pub task_id: String,
    #[schemars(description = "Name of the resource")]
    pub name: String,
    #[schemars(description = "URL or file path of the resource")]
    pub url: String,
    #[serde(default)]
    #[schemars(description = "Description of the resource")]
    pub description: Option<String>,
}

/// `get_checklist_summary` 参数。
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetChecklistSummaryParams {
    #[schemars(description = "The ID of the task")]
    pub task_id: String,
    #[serde(default)]
    #[schemars(description = "Whether to include detailed descriptions")]
    pub include_descriptions: Option<bool>,
}

/// `clear_task` 参数。
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClearTaskParams {
    #[schemars(description = "The ID of the task to delete")]
    pub task_id: String,
}

/// `list_tasks` 参数（无额外参数）。
#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct ListTasksParams {}

/// `get_current_task_details` 参数（无额外参数）。
#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct GetCurrentTaskDetailsParams {}
