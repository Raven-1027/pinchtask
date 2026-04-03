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

/// `update_task_description` 参数。
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateTaskDescriptionParams {
    #[schemars(description = "The ID of the task to update")]
    pub task_id: String,
    #[schemars(description = "The new task description")]
    pub task_description: String,
}

/// `update_context` 参数。
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateContextParams {
    #[schemars(description = "The ID of the task")]
    pub task_id: String,
    #[schemars(description = "The new context information")]
    pub context_for_all_tasks: String,
}

/// `add_checklist_item` 参数。
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddChecklistItemParams {
    #[schemars(description = "The ID of the task")]
    pub task_id: String,
    #[schemars(description = "A short yet comprehensive name for the task")]
    pub task: String,
    #[schemars(description = "A longer description about what we want to achieve")]
    pub detailed_description: String,
    #[serde(default)]
    #[schemars(description = "Related information and a detailed plan")]
    pub context_and_plan: Option<String>,
}

/// `update_checklist_item` 参数。
///
/// 注意 `context_and_plan` 使用 `Option<Option<String>>`：
/// - 字段未传入 → `None` → 不修改
/// - 传入 `null` → `Some(None)` → 清空
/// - 传入字符串 → `Some(Some("..."))` → 更新
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateChecklistItemParams {
    #[schemars(description = "The ID of the task")]
    pub task_id: String,
    #[schemars(description = "0-based index of the checklist item to update")]
    pub index: u64,
    #[serde(default)]
    #[schemars(description = "New short name")]
    pub task: Option<String>,
    #[serde(default)]
    #[schemars(description = "New detailed description")]
    pub detailed_description: Option<String>,
    #[serde(default)]
    #[schemars(description = "New context and plan (pass null to clear)")]
    pub context_and_plan: Option<Option<String>>,
    #[serde(default)]
    #[schemars(description = "Whether the item is completed")]
    pub done: Option<bool>,
}

/// `mark_task_done` 参数。
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MarkTaskDoneParams {
    #[schemars(description = "The ID of the task")]
    pub task_id: String,
    #[schemars(description = "0-based index of the checklist item")]
    pub index: u64,
}

/// `mark_task_undone` 参数。
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MarkTaskUndoneParams {
    #[schemars(description = "The ID of the task")]
    pub task_id: String,
    #[schemars(description = "0-based index of the checklist item")]
    pub index: u64,
}

/// `reorder_checklist_item` 参数。
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReorderChecklistItemParams {
    #[schemars(description = "The ID of the task")]
    pub task_id: String,
    #[schemars(description = "Current 0-based index")]
    pub from_index: u64,
    #[schemars(description = "New 0-based index")]
    pub to_index: u64,
}

/// `remove_checklist_item` 参数。
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveChecklistItemParams {
    #[schemars(description = "The ID of the task")]
    pub task_id: String,
    #[schemars(description = "0-based index of the checklist item to remove")]
    pub index: u64,
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

/// `update_metadata` 参数。
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateMetadataParams {
    #[schemars(description = "The ID of the task")]
    pub task_id: String,
    #[schemars(description = "The metadata object to set")]
    pub metadata: TaskMetadataInput,
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
