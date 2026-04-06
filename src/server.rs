//! MCP 服务器实现（基于 rmcp crate）。
//!
//! `PinchTaskServer` 使用 rmcp 的 `ServerHandler` + `ToolRouter` + `#[tool]` 宏
//! 实现 MCP 协议，替代原先手动实现的 JSON-RPC 协议栈。

use std::sync::Arc;

use rmcp::{
    ErrorData, ServerHandler,
    handler::server::router::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use uuid::Uuid;

use crate::core;
use crate::models::task::{ChecklistItem, Resource, Task, TaskMetadata};
use crate::store::{StoreError, TaskStore};
use crate::tools::params::*;

// ---------------------------------------------------------------------------
// PinchTaskServer
// ---------------------------------------------------------------------------

/// MCP 服务器，基于 rmcp crate。
#[derive(Clone)]
pub struct PinchTaskServer {
    store: Arc<TaskStore>,
    tool_router: ToolRouter<Self>,
}

impl PinchTaskServer {
    /// 创建新的 MCP 服务器实例。
    pub fn new(store: TaskStore) -> Self {
        Self {
            store: Arc::new(store),
            tool_router: Self::tool_router(),
        }
    }
}

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

/// 将 Task 序列化为 CallToolResult（成功）。
fn task_to_result(task: &Task) -> CallToolResult {
    let json =
        serde_json::to_string_pretty(task).unwrap_or_else(|e| format!("序列化任务失败: {e}"));
    text_result(json, false)
}

/// 构造文本类型的 CallToolResult。
fn text_result(text: String, is_error: bool) -> CallToolResult {
    let content = vec![Content::text(text)];
    if is_error {
        CallToolResult::error(content)
    } else {
        CallToolResult::success(content)
    }
}

/// 将 `Result<T, StoreError>` 转换为 `Result<CallToolResult, ErrorData>`。
///
/// 业务逻辑错误（StoreError）映射为 `is_error: true` 的工具错误结果，
/// 与迁移前的行为保持一致。
trait StoreResultExt<T> {
    fn into_tool_result(
        self,
        ok_fn: impl FnOnce(T) -> CallToolResult,
    ) -> Result<CallToolResult, ErrorData>;
}

impl<T> StoreResultExt<T> for Result<T, StoreError> {
    fn into_tool_result(
        self,
        ok_fn: impl FnOnce(T) -> CallToolResult,
    ) -> Result<CallToolResult, ErrorData> {
        match self {
            Ok(v) => Ok(ok_fn(v)),
            Err(e) => Ok(text_result(format!("{e}"), true)),
        }
    }
}

/// 将 `Result<(), StoreError>` 转换为 `Result<CallToolResult, ErrorData>`。
fn void_result(
    result: Result<(), StoreError>,
    success_msg: String,
) -> Result<CallToolResult, ErrorData> {
    match result {
        Ok(()) => Ok(text_result(success_msg, false)),
        Err(e) => Ok(text_result(format!("{e}"), true)),
    }
}

// ---------------------------------------------------------------------------
// 工具方法（#[tool_router] 自动注册到 ToolRouter）
// ---------------------------------------------------------------------------

#[tool_router]
impl PinchTaskServer {
    // ------------------------------------------------------------------
    // 1. new_task
    // ------------------------------------------------------------------
    #[tool(
        name = "new_task",
        description = "Create a new task with a description, optional checklist items, notes, resources, and metadata. Usage: For multi-step tasks, provide initial_checklist to plan sub-tasks upfront. Use context_for_all_tasks to share background information across all sub-tasks (e.g., tech stack, constraints).",
        input_schema = crate::tools::params::json_schema_for::<InitializeTaskParams>()
    )]
    pub async fn new_task(
        &self,
        Parameters(params): Parameters<InitializeTaskParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let initial_checklist: Vec<ChecklistItem> = params
            .initial_checklist
            .unwrap_or_default()
            .into_iter()
            .map(|item| ChecklistItem {
                id: item.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
                task: item.task,
                detailed_description: item.detailed_description,
                context_and_plan: item.context_and_plan,
                done: item.done,
            })
            .collect();

        let resources = params
            .resources
            .unwrap_or_default()
            .into_iter()
            .map(|r| Resource {
                name: r.name,
                url: r.url,
                description: r.description,
            })
            .collect::<Vec<_>>();

        let metadata: Option<TaskMetadata> = params.metadata.map(|m| TaskMetadata {
            tags: m.tags,
            priority: m.priority,
            estimated_completion_time: m.estimated_completion_time,
        });

        core::initialize_task(
            &self.store,
            &params.task_description,
            params.context_for_all_tasks.as_deref(),
            initial_checklist,
            params.notes.unwrap_or_default(),
            resources,
            metadata,
            params.project_id.as_deref(),
        )
        .await
        .into_tool_result(|t| task_to_result(&t))
    }

    // ------------------------------------------------------------------
    // 2. update_task（统一更新多个字段）
    // ------------------------------------------------------------------
    #[tool(
        name = "update_task",
        description = "Update task-level fields: description, context, priority, tags, and/or estimated completion time. Only specified fields are modified. Usage: This is the single entry point for all task-level updates. At least one field must be specified.",
        input_schema = crate::tools::params::json_schema_for::<UpdateTaskParams>()
    )]
    pub async fn update_task(
        &self,
        Parameters(params): Parameters<UpdateTaskParams>,
    ) -> Result<CallToolResult, ErrorData> {
        if params.task_description.is_none()
            && params.context_for_all_tasks.is_none()
            && params.priority.is_none()
            && params.tags.is_none()
            && params.eta.is_none()
        {
            return Ok(text_result(
                "至少需要指定一个可修改的字段 (task_description / context_for_all_tasks / priority / tags / eta)"
                    .to_owned(),
                true,
            ));
        }

        // 更新 description
        if let Some(desc) = &params.task_description {
            match core::update_task_description(&self.store, &params.task_id, desc).await {
                Ok(_) => {}
                Err(e) => return Ok(text_result(format!("{e}"), true)),
            }
        }
        // 更新 context
        if let Some(ctx) = &params.context_for_all_tasks {
            match core::update_context(&self.store, &params.task_id, ctx).await {
                Ok(_) => {}
                Err(e) => return Ok(text_result(format!("{e}"), true)),
            }
        }
        // 更新 metadata
        if params.priority.is_some() || params.tags.is_some() || params.eta.is_some() {
            let existing = match self.store.get_task(&params.task_id).await {
                Ok(t) => t,
                Err(e) => return Ok(text_result(format!("{e}"), true)),
            };
            let mut metadata = existing.metadata.unwrap_or(TaskMetadata {
                tags: None,
                priority: None,
                estimated_completion_time: None,
            });
            if let Some(p) = &params.priority {
                metadata.priority = Some(p.clone());
            }
            if let Some(t) = &params.tags {
                metadata.tags = Some(
                    t.split(',')
                        .map(|s| s.trim().to_owned())
                        .filter(|s| !s.is_empty())
                        .collect(),
                );
            }
            if let Some(e) = &params.eta {
                metadata.estimated_completion_time = Some(e.clone());
            }
            match core::update_metadata(&self.store, &params.task_id, metadata).await {
                Ok(_) => {}
                Err(e) => return Ok(text_result(format!("{e}"), true)),
            }
        }

        let task = match self.store.get_task(&params.task_id).await {
            Ok(t) => t,
            Err(e) => return Ok(text_result(format!("{e}"), true)),
        };
        Ok(task_to_result(&task))
    }

    // ------------------------------------------------------------------
    // 5. manage_checklist_item
    // ------------------------------------------------------------------
    #[tool(
        name = "manage_checklist_item",
        description = "Perform operations on checklist items. NOTE: All checklist item indices are 0-based (the first item has index 0).\n\nSelect the action based on what you need:\n\n- \"add\": Append a new item. Provide task (short name) and detailed_description. context_and_plan is optional.\n\n- \"update\": Modify an existing item's fields. Provide index (0-based). Only specified fields are changed. Set done=true to mark completed, done=false to revert. Pass context_and_plan=null to clear it, or omit to keep unchanged.\n\n- \"reorder\": Move an item to a new position. Provide from_index and to_index (both 0-based). After reordering, indices change — refresh task data before further index operations.\n\n- \"remove\": Delete an item. Provide index (0-based). After removal, subsequent indices shift down by 1.\n\nThis is the single entry point for all checklist item operations.",
        input_schema = crate::tools::params::json_schema_for::<ManageChecklistItemParams>()
    )]
    pub async fn manage_checklist_item(
        &self,
        Parameters(params): Parameters<ManageChecklistItemParams>,
    ) -> Result<CallToolResult, ErrorData> {
        match params.action {
            Action::Add => {
                let task = params.task.ok_or_else(|| {
                    ErrorData::invalid_params("task is required for add action", None)
                })?;
                let detailed_description = params.detailed_description.ok_or_else(|| {
                    ErrorData::invalid_params(
                        "detailed_description is required for add action",
                        None,
                    )
                })?;
                let context_and_plan = params.context_and_plan.flatten();
                core::add_checklist_item(
                    &self.store,
                    &params.task_id,
                    &task,
                    &detailed_description,
                    context_and_plan.as_deref(),
                )
                .await
                .into_tool_result(|t| task_to_result(&t))
            }
            Action::Update => {
                let index = params.index.ok_or_else(|| {
                    ErrorData::invalid_params("index is required for update action", None)
                })? as usize;
                core::update_checklist_item(
                    &self.store,
                    &params.task_id,
                    index,
                    params.task.as_deref(),
                    params.detailed_description.as_deref(),
                    params.context_and_plan.as_ref().map(|o| o.as_deref()),
                    params.done,
                )
                .await
                .into_tool_result(|t| task_to_result(&t))
            }
            Action::Reorder => {
                let from_index = params.from_index.ok_or_else(|| {
                    ErrorData::invalid_params("from_index is required for reorder action", None)
                })? as usize;
                let to_index = params.to_index.ok_or_else(|| {
                    ErrorData::invalid_params("to_index is required for reorder action", None)
                })? as usize;
                core::reorder_checklist_item(&self.store, &params.task_id, from_index, to_index)
                    .await
                    .into_tool_result(|t| task_to_result(&t))
            }
            Action::Remove => {
                let index = params.index.ok_or_else(|| {
                    ErrorData::invalid_params("index is required for remove action", None)
                })? as usize;
                core::remove_checklist_item(&self.store, &params.task_id, index)
                    .await
                    .into_tool_result(|t| task_to_result(&t))
            }
        }
    }

    // ------------------------------------------------------------------
    // 11. add_note
    // ------------------------------------------------------------------
    #[tool(
        name = "add_note",
        description = "Add a note to the task. Usage: Record discoveries, decisions, or context that doesn't fit into checklist items. Notes are append-only and useful for audit trails."
    )]
    pub async fn add_note(
        &self,
        Parameters(params): Parameters<AddNoteParams>,
    ) -> Result<CallToolResult, ErrorData> {
        core::add_note(&self.store, &params.task_id, &params.content)
            .await
            .into_tool_result(|t| task_to_result(&t))
    }

    // ------------------------------------------------------------------
    // 12. add_resource
    // ------------------------------------------------------------------
    #[tool(
        name = "add_resource",
        description = "Add a resource reference to the task (append-only). Usage: Link relevant files (use file:// or absolute path as url), documentation URLs, or API references.",
        input_schema = crate::tools::params::json_schema_for::<AddResourceParams>()
    )]
    pub async fn add_resource(
        &self,
        Parameters(params): Parameters<AddResourceParams>,
    ) -> Result<CallToolResult, ErrorData> {
        core::add_resource(
            &self.store,
            &params.task_id,
            &params.name,
            &params.url,
            params.description.as_deref(),
        )
        .await
        .into_tool_result(|t| task_to_result(&t))
    }

    // ------------------------------------------------------------------
    // 14. get_checklist_summary
    // ------------------------------------------------------------------
    #[tool(
        name = "get_checklist_summary",
        description = "Get a summary of the task checklist with completion status. Usage: Call for a quick progress overview. Set include_descriptions to true to see detailed descriptions alongside item names.",
        input_schema = crate::tools::params::json_schema_for::<GetChecklistSummaryParams>()
    )]
    pub async fn get_checklist_summary(
        &self,
        Parameters(params): Parameters<GetChecklistSummaryParams>,
    ) -> Result<CallToolResult, ErrorData> {
        let _include_descriptions = params.include_descriptions.unwrap_or(false);
        core::get_checklist_summary(&self.store, &params.task_id)
            .await
            .into_tool_result(|s| text_result(s, false))
    }

    // ------------------------------------------------------------------
    // 15. clear_task
    // ------------------------------------------------------------------
    #[tool(
        name = "clear_task",
        description = "Delete a task by its ID. Usage: Irreversible operation. Confirm the task_id before calling. Consider using get_checklist_summary to verify the task is the intended target."
    )]
    pub async fn clear_task(
        &self,
        Parameters(params): Parameters<ClearTaskParams>,
    ) -> Result<CallToolResult, ErrorData> {
        void_result(
            core::clear_task(&self.store, &params.task_id).await,
            format!("任务 {} 已删除", params.task_id),
        )
    }

    // ------------------------------------------------------------------
    // 16. list_tasks
    // ------------------------------------------------------------------
    #[tool(
        name = "list_tasks",
        description = "List all tasks sorted by creation time. Usage: Call at the start of a session to get an overview of all existing tasks and their progress. Returns a concise summary, not full details."
    )]
    pub async fn list_tasks(
        &self,
        Parameters(_params): Parameters<ListTasksParams>,
    ) -> Result<CallToolResult, ErrorData> {
        core::list_tasks_summary(&self.store)
            .await
            .into_tool_result(|s| text_result(s, false))
    }

    // ------------------------------------------------------------------
    // 17. manage_project
    // ------------------------------------------------------------------
    #[tool(
        name = "manage_project",
        description = "Perform operations on projects. Projects are containers for organizing related tasks. Each task can belong to at most one project.\n\nSelect the action based on what you need:\n\n- \"create\": Create a new project. Provide name (required) and description (optional).\n\n- \"get\": Get a project by its ID. Provide project_id.\n\n- \"update\": Update a project's name and/or description. Provide project_id and the fields to change.\n\n- \"delete\": Delete a project. Provide project_id. Set delete_tasks=true to also delete all associated tasks (otherwise tasks are kept with project_id cleared).\n\n- \"list\": List all projects. No additional parameters needed.\n\nThis is the single entry point for all project operations.",
        input_schema = crate::tools::params::json_schema_for::<ManageProjectParams>()
    )]
    pub async fn manage_project(
        &self,
        Parameters(params): Parameters<ManageProjectParams>,
    ) -> Result<CallToolResult, ErrorData> {
        match params.action {
            ProjectAction::Create => {
                let name = params.name.ok_or_else(|| {
                    ErrorData::invalid_params("name is required for create action", None)
                })?;
                core::create_project(&self.store, &name, params.description.as_deref())
                    .await
                    .into_tool_result(|p| {
                        let json = serde_json::to_string_pretty(&p)
                            .unwrap_or_else(|e| format!("序列化项目失败: {e}"));
                        text_result(json, false)
                    })
            }
            ProjectAction::Get => {
                let project_id = params.project_id.ok_or_else(|| {
                    ErrorData::invalid_params("project_id is required for get action", None)
                })?;
                core::get_project(&self.store, &project_id)
                    .await
                    .into_tool_result(|p| {
                        let json = serde_json::to_string_pretty(&p)
                            .unwrap_or_else(|e| format!("序列化项目失败: {e}"));
                        text_result(json, false)
                    })
            }
            ProjectAction::Update => {
                let project_id = params.project_id.ok_or_else(|| {
                    ErrorData::invalid_params("project_id is required for update action", None)
                })?;
                core::update_project(
                    &self.store,
                    &project_id,
                    params.name.as_deref(),
                    params.description.as_deref(),
                )
                .await
                .into_tool_result(|p| {
                    let json = serde_json::to_string_pretty(&p)
                        .unwrap_or_else(|e| format!("序列化项目失败: {e}"));
                    text_result(json, false)
                })
            }
            ProjectAction::Delete => {
                let project_id = params.project_id.ok_or_else(|| {
                    ErrorData::invalid_params("project_id is required for delete action", None)
                })?;
                if params.delete_tasks.unwrap_or(false) {
                    void_result(
                        core::delete_project_with_tasks(&self.store, &project_id).await,
                        format!("项目 {} 及其关联任务已删除", project_id),
                    )
                } else {
                    void_result(
                        core::delete_project(&self.store, &project_id).await,
                        format!("项目 {} 已删除（关联任务保留）", project_id),
                    )
                }
            }
            ProjectAction::List => {
                core::list_projects(&self.store)
                    .await
                    .into_tool_result(|projects| {
                        if projects.is_empty() {
                            text_result("当前没有任何项目".to_owned(), false)
                        } else {
                            let json = serde_json::to_string_pretty(&projects)
                                .unwrap_or_else(|e| format!("序列化项目列表失败: {e}"));
                            text_result(json, false)
                        }
                    })
            }
        }
    }
}

// ---------------------------------------------------------------------------
// ServerHandler 实现（#[tool_handler] 自动实现 call_tool / list_tools / get_tool）
// ---------------------------------------------------------------------------

#[tool_handler]
impl ServerHandler for PinchTaskServer {
    fn get_info(&self) -> ServerInfo {
        let capabilities = ServerCapabilities::builder().enable_tools().build();
        // ServerInfo (= InitializeResult) 是 #[non_exhaustive]，
        // 通过 serde_json 中转构造完整结构。
        let value = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": capabilities,
            "serverInfo": {
                "name": "pinchtask",
                "version": env!("CARGO_PKG_VERSION")
            }
        });
        serde_json::from_value(value).expect("构造 ServerInfo 不应失败")
    }
}
