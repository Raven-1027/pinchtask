//! 任务相关的 MCP 工具处理器。
//!
//! 每个 handler 从 `serde_json::Value` 中解析参数，调用 `crate::core` 中的
//! 纯业务逻辑函数，返回 MCP 协议所需的 `CallToolResult`。

use crate::core;
use crate::models::task::{ChecklistItem, Resource, Task, TaskMetadata};
use crate::protocol::CallToolResult;
use crate::store::TaskStore;
use serde_json::Value;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// MCP 工具 handler 包装函数
//
// 每个 handler 从 serde_json::Value 中解析参数，调用对应的 core 层函数，
// 返回 CallToolResult 或 Err(String)。
// ---------------------------------------------------------------------------

/// 辅助宏：从 JSON Value 中提取必填字符串字段。
macro_rules! require_str {
    ($args:expr, $field:expr) => {
        $args.get($field)
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("Missing or invalid required parameter: {}", $field))?
    };
}

/// 辅助宏：从 JSON Value 中提取可选字符串字段。
macro_rules! optional_str {
    ($args:expr, $field:expr) => {
        $args.get($field).and_then(|v| v.as_str())
    };
}

/// 辅助函数：将 Task 序列化为 CallToolResult。
fn task_to_result(task: &Task) -> Result<CallToolResult, String> {
    let json = serde_json::to_string_pretty(task)
        .map_err(|e| format!("序列化任务失败: {e}"))?;
    Ok(CallToolResult::text_result(json))
}

/// 辅助函数：将 StoreError 转换为 String。
fn store_err(e: crate::store::StoreError) -> String {
    format!("{e}")
}

// 1. initialize_task_handler
pub async fn initialize_task_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_description = require_str!(args, "task_description");
    let context = optional_str!(args, "context_for_all_tasks");

    // 手动解析 initial_checklist，为缺少 id/done 的条目生成默认值
    let initial_checklist: Vec<ChecklistItem> = args
        .get("initial_checklist")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter().filter_map(|item| {
                let task = item.get("task")?.as_str()?;
                let detailed_description = item
                    .get("detailed_description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Some(ChecklistItem {
                    id: item.get("id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_owned())
                        .unwrap_or_else(|| Uuid::new_v4().to_string()),
                    task: task.to_owned(),
                    detailed_description: detailed_description.to_owned(),
                    context_and_plan: item
                        .get("context_and_plan")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_owned()),
                    done: item.get("done")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                })
            }).collect()
        })
        .unwrap_or_default();

    let notes: Vec<String> = args
        .get("notes")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let resources: Vec<Resource> = args
        .get("resources")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let metadata: Option<TaskMetadata> = args
        .get("metadata")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    let task = core::initialize_task(
        store,
        task_description,
        context,
        initial_checklist,
        notes,
        resources,
        metadata,
    )
    .await
    .map_err(store_err)?;

    task_to_result(&task)
}

// 2. update_task_description_handler
pub async fn update_task_description_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    let task_description = require_str!(args, "task_description");
    let task = core::update_task_description(store, task_id, task_description).await.map_err(store_err)?;
    task_to_result(&task)
}

// 3. update_context_handler
pub async fn update_context_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    let context_for_all_tasks = require_str!(args, "context_for_all_tasks");
    let task = core::update_context(store, task_id, context_for_all_tasks).await.map_err(store_err)?;
    task_to_result(&task)
}

// 统一任务更新 handler：支持一次性修改 description、context 及 metadata
pub async fn update_task_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    let description = optional_str!(args, "task_description");
    let context = optional_str!(args, "context_for_all_tasks");
    let priority = optional_str!(args, "priority");
    let tags_raw = optional_str!(args, "tags");
    let eta = optional_str!(args, "eta");

    if description.is_none()
        && context.is_none()
        && priority.is_none()
        && tags_raw.is_none()
        && eta.is_none()
    {
        return Err("至少需要指定一个可修改的字段 (task_description / context_for_all_tasks / priority / tags / eta)".to_owned());
    }

    // 更新 description
    if let Some(desc) = description {
        core::update_task_description(store, task_id, desc).await.map_err(store_err)?;
    }
    // 更新 context
    if let Some(ctx) = context {
        core::update_context(store, task_id, ctx).await.map_err(store_err)?;
    }
    // 更新 metadata
    if priority.is_some() || tags_raw.is_some() || eta.is_some() {
        let existing = store.get_task(task_id).await.map_err(store_err)?;
        let mut metadata = existing.metadata.unwrap_or(TaskMetadata {
            tags: None,
            priority: None,
            estimated_completion_time: None,
        });
        if let Some(p) = priority {
            metadata.priority = Some(p.to_owned());
        }
        if let Some(t) = tags_raw {
            metadata.tags = Some(
                t.split(',')
                    .map(|s| s.trim().to_owned())
                    .filter(|s| !s.is_empty())
                    .collect(),
            );
        }
        if let Some(e) = eta {
            metadata.estimated_completion_time = Some(e.to_owned());
        }
        core::update_metadata(store, task_id, metadata).await.map_err(store_err)?;
    }

    let task = store.get_task(task_id).await.map_err(store_err)?;
    task_to_result(&task)
}

// 4. add_checklist_item_handler
pub async fn add_checklist_item_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    let task_name = require_str!(args, "task");
    let detailed_description = require_str!(args, "detailed_description");
    let context_and_plan = optional_str!(args, "context_and_plan");
    let task = core::add_checklist_item(store, task_id, task_name, detailed_description, context_and_plan)
        .await
        .map_err(store_err)?;
    task_to_result(&task)
}

// 5. update_checklist_item_handler
pub async fn update_checklist_item_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    let index: usize = args
        .get("index")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "Missing or invalid required parameter: index".to_owned())?
        as usize;
    let task_name = optional_str!(args, "task");
    let detailed_description = optional_str!(args, "detailed_description");

    // context_and_plan: 区分"未传入"(None)和"传入null"(Some(None))
    let context_and_plan: Option<Option<&str>> = if args.get("context_and_plan").is_some() {
        Some(optional_str!(args, "context_and_plan"))
    } else {
        None
    };

    let done: Option<bool> = args.get("done").and_then(|v| v.as_bool());

    let task = core::update_checklist_item(
        store,
        task_id,
        index,
        task_name,
        detailed_description,
        context_and_plan,
        done,
    )
    .await
    .map_err(store_err)?;
    task_to_result(&task)
}

// 6. mark_task_done_handler
pub async fn mark_task_done_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    let index: usize = args
        .get("index")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "Missing or invalid required parameter: index".to_owned())?
        as usize;
    let task = core::mark_task_done(store, task_id, index).await.map_err(store_err)?;
    task_to_result(&task)
}

// 7. mark_task_undone_handler
pub async fn mark_task_undone_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    let index: usize = args
        .get("index")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "Missing or invalid required parameter: index".to_owned())?
        as usize;
    let task = core::mark_task_undone(store, task_id, index).await.map_err(store_err)?;
    task_to_result(&task)
}

// 8. reorder_checklist_item_handler
pub async fn reorder_checklist_item_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    let from_index: usize = args
        .get("from_index")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "Missing or invalid required parameter: from_index".to_owned())?
        as usize;
    let to_index: usize = args
        .get("to_index")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "Missing or invalid required parameter: to_index".to_owned())?
        as usize;
    let task = core::reorder_checklist_item(store, task_id, from_index, to_index).await.map_err(store_err)?;
    task_to_result(&task)
}

// 9. remove_checklist_item_handler
pub async fn remove_checklist_item_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    let index: usize = args
        .get("index")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "Missing or invalid required parameter: index".to_owned())?
        as usize;
    let task = core::remove_checklist_item(store, task_id, index).await.map_err(store_err)?;
    task_to_result(&task)
}

// 10. add_note_handler
pub async fn add_note_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    let content = require_str!(args, "content");
    let task = core::add_note(store, task_id, content).await.map_err(store_err)?;
    task_to_result(&task)
}

// 11. add_resource_handler
pub async fn add_resource_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    let name = require_str!(args, "name");
    let url = require_str!(args, "url");
    let description = optional_str!(args, "description");
    let task = core::add_resource(store, task_id, name, url, description).await.map_err(store_err)?;
    task_to_result(&task)
}

// 12. update_metadata_handler
pub async fn update_metadata_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    let metadata: TaskMetadata = args
        .get("metadata")
        .cloned()
        .ok_or_else(|| "Missing required parameter: metadata".to_owned())
        .and_then(|v| {
            serde_json::from_value(v).map_err(|e| format!("Invalid metadata: {e}"))
        })?;
    let task = core::update_metadata(store, task_id, metadata).await.map_err(store_err)?;
    task_to_result(&task)
}

// 13. get_checklist_summary_handler
pub async fn get_checklist_summary_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    let _include_descriptions: bool = args
        .get("include_descriptions")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let summary = core::get_checklist_summary(store, task_id).await.map_err(store_err)?;
    Ok(CallToolResult::text_result(summary))
}

// 14. clear_task_handler
pub async fn clear_task_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    core::clear_task(store, task_id).await.map_err(store_err)?;
    Ok(CallToolResult::text_result(format!("任务 {task_id} 已删除")))
}

// 15. list_tasks_handler
pub async fn list_tasks_handler(
    store: &TaskStore,
    _args: Value,
) -> Result<CallToolResult, String> {
    let summary = core::list_tasks_summary(store).await.map_err(store_err)?;
    Ok(CallToolResult::text_result(summary))
}

// 16. get_current_task_details_handler
pub async fn get_current_task_details_handler(
    store: &TaskStore,
    _args: Value,
) -> Result<CallToolResult, String> {
    let tasks = store.list_tasks().await.map_err(store_err)?;
    // 找到第一个有未完成清单条目的任务
    let current_task = tasks
        .iter()
        .find(|t| t.checklist.iter().any(|item| !item.done))
        .ok_or_else(|| "没有找到包含未完成子任务的任务".to_owned())?;
    let details = core::get_current_task_details(store, &current_task.id).await.map_err(store_err)?;
    Ok(CallToolResult::text_result(details))
}

#[cfg(test)]
mod tests {
    use crate::core;
    use crate::store::TaskStore;

    /// 创建使用临时目录的 TaskStore。
    async fn temp_store() -> (TaskStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let store = TaskStore::new(Some(dir.path().to_path_buf()))
            .await
            .expect("创建 TaskStore 失败");
        (store, dir)
    }

    #[tokio::test]
    async fn initialize_task_creates_and_persists() {
        let (store, _dir) = temp_store().await;

        let task = core::initialize_task(
            &store,
            "测试任务",
            Some("共享上下文"),
            vec![],
            vec![],
            vec![],
            None,
        )
        .await
        .expect("创建任务失败");

        assert_eq!(task.task_description, "测试任务");
        assert_eq!(task.context_for_all_tasks, Some("共享上下文".to_owned()));
        assert!(!task.id.is_empty());
    }

    #[tokio::test]
    async fn add_checklist_item_and_mark_done() {
        let (store, _dir) = temp_store().await;

        let task = core::initialize_task(&store, "t1", None, vec![], vec![], vec![], None)
            .await
            .expect("创建任务失败");

        let task =
            core::add_checklist_item(&store, &task.id, "步骤1", "详细描述", Some("计划"))
                .await
                .expect("添加清单条目失败");
        assert_eq!(task.checklist.len(), 1);
        assert!(!task.checklist[0].done);

        let task = core::mark_task_done(&store, &task.id, 0).await.expect("标记完成失败");
        assert!(task.checklist[0].done);
    }

    #[tokio::test]
    async fn add_note_and_resource() {
        let (store, _dir) = temp_store().await;

        let task = core::initialize_task(&store, "t2", None, vec![], vec![], vec![], None)
            .await
            .expect("创建任务失败");

        let task = core::add_note(&store, &task.id, "一条笔记").await.expect("添加笔记失败");
        assert_eq!(task.notes, vec!["一条笔记"]);

        let task = core::add_resource(&store, &task.id, "文档", "https://example.com", Some("示例"))
            .await
            .expect("添加资源失败");
        assert_eq!(task.resources.len(), 1);
        assert_eq!(task.resources[0].name, "文档");
    }
}
