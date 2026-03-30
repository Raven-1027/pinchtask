//! 任务相关的 MCP 工具处理器。
//!
//! 每个公开函数对应一个 MCP 工具，待传输层就绪后注册到服务器。

use crate::models::task::{ChecklistItem, Resource, Task, TaskMetadata};
use crate::protocol::CallToolResult;
use crate::store::{StoreError, TaskStore};
use serde_json::Value;
use uuid::Uuid;

/// 初始化一个新任务并持久化。
///
/// 返回创建后的 `Task` 实例。
pub fn initialize_task(
    store: &TaskStore,
    task_description: &str,
    context_for_all_tasks: Option<&str>,
    initial_checklist: Vec<ChecklistItem>,
    notes: Vec<String>,
    resources: Vec<Resource>,
    metadata: Option<TaskMetadata>,
) -> Result<Task, StoreError> {
    store.create_task(
        task_description,
        context_for_all_tasks,
        initial_checklist,
        notes,
        resources,
        metadata,
    )
}

/// 更新任务的整体描述。
pub fn update_task_description(
    store: &TaskStore,
    task_id: &str,
    new_description: &str,
) -> Result<Task, StoreError> {
    let mut task = store.get_task(task_id)?;
    task.task_description = new_description.to_owned();
    store.update_task(&mut task)?;
    Ok(task)
}

/// 更新所有子任务的共享上下文。
pub fn update_context(
    store: &TaskStore,
    task_id: &str,
    new_context: &str,
) -> Result<Task, StoreError> {
    let mut task = store.get_task(task_id)?;
    task.context_for_all_tasks = Some(new_context.to_owned());
    store.update_task(&mut task)?;
    Ok(task)
}

/// 向任务添加一条笔记。
pub fn add_note(store: &TaskStore, task_id: &str, content: &str) -> Result<Task, StoreError> {
    let mut task = store.get_task(task_id)?;
    task.notes.push(content.to_owned());
    store.update_task(&mut task)?;
    Ok(task)
}

/// 向任务添加一个资源引用。
pub fn add_resource(
    store: &TaskStore,
    task_id: &str,
    name: &str,
    url: &str,
    description: Option<&str>,
) -> Result<Task, StoreError> {
    let mut task = store.get_task(task_id)?;
    task.resources.push(Resource {
        name: name.to_owned(),
        url: url.to_owned(),
        description: description.map(|s| s.to_owned()),
    });
    store.update_task(&mut task)?;
    Ok(task)
}

/// 向任务清单中添加一个条目。
pub fn add_checklist_item(
    store: &TaskStore,
    task_id: &str,
    task_name: &str,
    detailed_description: &str,
    context_and_plan: Option<&str>,
) -> Result<Task, StoreError> {
    let mut task = store.get_task(task_id)?;
    task.checklist.push(ChecklistItem {
        id: Uuid::new_v4().to_string(),
        task: task_name.to_owned(),
        detailed_description: detailed_description.to_owned(),
        context_and_plan: context_and_plan.map(|s| s.to_owned()),
        done: false,
    });
    store.update_task(&mut task)?;
    Ok(task)
}

/// 将指定清单条目标记为已完成。
pub fn mark_task_done(store: &TaskStore, task_id: &str, item_index: usize) -> Result<Task, StoreError> {
    let mut task = store.get_task(task_id)?;
    if item_index >= task.checklist.len() {
        return Err(StoreError::NotFound(format!(
            "清单条目索引越界: {item_index}"
        )));
    }
    task.checklist[item_index].done = true;
    store.update_task(&mut task)?;
    Ok(task)
}

/// 将指定清单条目标记为未完成。
pub fn mark_task_undone(store: &TaskStore, task_id: &str, item_index: usize) -> Result<Task, StoreError> {
    let mut task = store.get_task(task_id)?;
    if item_index >= task.checklist.len() {
        return Err(StoreError::NotFound(format!(
            "清单条目索引越界: {item_index}"
        )));
    }
    task.checklist[item_index].done = false;
    store.update_task(&mut task)?;
    Ok(task)
}

/// 更新任务元数据。
pub fn update_metadata(
    store: &TaskStore,
    task_id: &str,
    metadata: TaskMetadata,
) -> Result<Task, StoreError> {
    let mut task = store.get_task(task_id)?;
    task.metadata = Some(metadata);
    store.update_task(&mut task)?;
    Ok(task)
}

/// 删除指定任务。
pub fn clear_task(store: &TaskStore, task_id: &str) -> Result<(), StoreError> {
    store.delete_task(task_id)
}

/// 获取任务的清单概要（含完成状态）。
pub fn get_checklist_summary(
    store: &TaskStore,
    task_id: &str,
) -> Result<String, StoreError> {
    let task = store.get_task(task_id)?;
    let total = task.checklist.len();
    let done = task.checklist.iter().filter(|i| i.done).count();
    let mut summary = format!("任务: {}\n进度: {done}/{total}\n\n", task.task_description);
    for (i, item) in task.checklist.iter().enumerate() {
        let status = if item.done { "✅" } else { "⬜" };
        summary.push_str(&format!("{status} [{i}] {}\n", item.task));
    }
    Ok(summary)
}

/// 更新指定清单条目的内容。
///
/// 只更新传入的非 None 字段，保留未指定字段的原值。
pub fn update_checklist_item(
    store: &TaskStore,
    task_id: &str,
    item_index: usize,
    task_name: Option<&str>,
    detailed_description: Option<&str>,
    context_and_plan: Option<Option<&str>>,
    done: Option<bool>,
) -> Result<Task, StoreError> {
    let mut task = store.get_task(task_id)?;
    if item_index >= task.checklist.len() {
        return Err(StoreError::NotFound(format!(
            "清单条目索引越界: {item_index}"
        )));
    }
    let item = &mut task.checklist[item_index];
    if let Some(name) = task_name {
        item.task = name.to_owned();
    }
    if let Some(desc) = detailed_description {
        item.detailed_description = desc.to_owned();
    }
    // context_and_plan 使用 Option<Option<&str>> 以区分"未传入"和"传入 None（清空）"
    if let Some(cap) = context_and_plan {
        item.context_and_plan = cap.map(|s| s.to_owned());
    }
    if let Some(d) = done {
        item.done = d;
    }
    store.update_task(&mut task)?;
    Ok(task)
}

/// 删除指定索引处的清单条目。
pub fn remove_checklist_item(
    store: &TaskStore,
    task_id: &str,
    item_index: usize,
) -> Result<Task, StoreError> {
    let mut task = store.get_task(task_id)?;
    if item_index >= task.checklist.len() {
        return Err(StoreError::NotFound(format!(
            "清单条目索引越界: {item_index}"
        )));
    }
    task.checklist.remove(item_index);
    store.update_task(&mut task)?;
    Ok(task)
}

/// 将清单条目从 from_index 移动到 to_index。
///
/// 先移除原位置的条目，再插入到目标位置。
pub fn reorder_checklist_item(
    store: &TaskStore,
    task_id: &str,
    from_index: usize,
    to_index: usize,
) -> Result<Task, StoreError> {
    let mut task = store.get_task(task_id)?;
    if from_index >= task.checklist.len() {
        return Err(StoreError::NotFound(format!(
            "源索引越界: {from_index}"
        )));
    }
    // to_index 允许等于 checklist.len()（追加到末尾），但不能更大
    if to_index > task.checklist.len() {
        return Err(StoreError::NotFound(format!(
            "目标索引越界: {to_index}"
        )));
    }
    let item = task.checklist.remove(from_index);
    // 移除后列表缩短了，需要调整插入位置
    let insert_at = if to_index > from_index {
        to_index // remove 已经让后面的元素前移了 1 位
    } else {
        to_index
    };
    // 但如果 to_index 原本就是基于移除前的索引计算的，需要重新校准
    // 标准做法：先 remove，然后 min(to_index, len) 作为插入点
    let insert_at = to_index.min(task.checklist.len());
    task.checklist.insert(insert_at, item);
    store.update_task(&mut task)?;
    Ok(task)
}

/// 获取第一个未完成的清单条目的详细信息。
///
/// 返回格式化的字符串，包含任务上下文和当前子任务的完整信息。
/// 如果所有子任务均已完成或清单为空，返回提示信息。
pub fn get_current_task_details(
    store: &TaskStore,
    task_id: &str,
) -> Result<String, StoreError> {
    let task = store.get_task(task_id)?;

    let mut result = String::new();
    result.push_str(&format!("任务: {}\n", task.task_description));
    if let Some(ref ctx) = task.context_for_all_tasks {
        result.push_str(&format!("共享上下文: {ctx}\n"));
    }
    result.push('\n');

    // 查找第一个未完成的清单条目
    match task.checklist.iter().enumerate().find(|(_, item)| !item.done) {
        Some((index, item)) => {
            result.push_str(&format!("当前子任务 (索引 {index}):\n"));
            result.push_str(&format!("  名称: {}\n", item.task));
            result.push_str(&format!("  详细描述: {}\n", item.detailed_description));
            if let Some(ref plan) = item.context_and_plan {
                result.push_str(&format!("  上下文与计划: {plan}\n"));
            }
            result.push_str(&format!("  状态: {}\n", if item.done { "已完成" } else { "进行中" }));
        }
        None => {
            let total = task.checklist.len();
            let done = task.checklist.iter().filter(|i| i.done).count();
            if total == 0 {
                result.push_str("清单为空，尚无子任务。\n");
            } else {
                result.push_str(&format!("所有子任务均已完成 ({done}/{total})。\n"));
            }
        }
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// MCP 工具 handler 包装函数
//
// 每个 handler 从 serde_json::Value 中解析参数，调用对应的业务逻辑函数，
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
fn store_err(e: StoreError) -> String {
    format!("{e}")
}

// 1. initialize_task_handler
pub fn initialize_task_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_description = require_str!(args, "task_description");
    let context = optional_str!(args, "context_for_all_tasks");

    let initial_checklist: Vec<ChecklistItem> = args
        .get("initial_checklist")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
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

    let task = initialize_task(
        store,
        task_description,
        context,
        initial_checklist,
        notes,
        resources,
        metadata,
    )
    .map_err(store_err)?;

    task_to_result(&task)
}

// 2. update_task_description_handler
pub fn update_task_description_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    let task_description = require_str!(args, "task_description");
    let task = update_task_description(store, task_id, task_description).map_err(store_err)?;
    task_to_result(&task)
}

// 3. update_context_handler
pub fn update_context_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    let context_for_all_tasks = require_str!(args, "context_for_all_tasks");
    let task = update_context(store, task_id, context_for_all_tasks).map_err(store_err)?;
    task_to_result(&task)
}

// 4. add_checklist_item_handler
pub fn add_checklist_item_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    let task_name = require_str!(args, "task");
    let detailed_description = require_str!(args, "detailed_description");
    let context_and_plan = optional_str!(args, "context_and_plan");
    let task = add_checklist_item(store, task_id, task_name, detailed_description, context_and_plan)
        .map_err(store_err)?;
    task_to_result(&task)
}

// 5. update_checklist_item_handler
pub fn update_checklist_item_handler(
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

    let task = update_checklist_item(
        store,
        task_id,
        index,
        task_name,
        detailed_description,
        context_and_plan,
        done,
    )
    .map_err(store_err)?;
    task_to_result(&task)
}

// 6. mark_task_done_handler
pub fn mark_task_done_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    let index: usize = args
        .get("index")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "Missing or invalid required parameter: index".to_owned())?
        as usize;
    let task = mark_task_done(store, task_id, index).map_err(store_err)?;
    task_to_result(&task)
}

// 7. mark_task_undone_handler
pub fn mark_task_undone_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    let index: usize = args
        .get("index")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "Missing or invalid required parameter: index".to_owned())?
        as usize;
    let task = mark_task_undone(store, task_id, index).map_err(store_err)?;
    task_to_result(&task)
}

// 8. reorder_checklist_item_handler
pub fn reorder_checklist_item_handler(
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
    let task = reorder_checklist_item(store, task_id, from_index, to_index).map_err(store_err)?;
    task_to_result(&task)
}

// 9. remove_checklist_item_handler
pub fn remove_checklist_item_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    let index: usize = args
        .get("index")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "Missing or invalid required parameter: index".to_owned())?
        as usize;
    let task = remove_checklist_item(store, task_id, index).map_err(store_err)?;
    task_to_result(&task)
}

// 10. add_note_handler
pub fn add_note_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    let content = require_str!(args, "content");
    let task = add_note(store, task_id, content).map_err(store_err)?;
    task_to_result(&task)
}

// 11. add_resource_handler
pub fn add_resource_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    let name = require_str!(args, "name");
    let url = require_str!(args, "url");
    let description = optional_str!(args, "description");
    let task = add_resource(store, task_id, name, url, description).map_err(store_err)?;
    task_to_result(&task)
}

// 12. update_metadata_handler
pub fn update_metadata_handler(
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
    let task = update_metadata(store, task_id, metadata).map_err(store_err)?;
    task_to_result(&task)
}

// 13. get_checklist_summary_handler
pub fn get_checklist_summary_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    let _include_descriptions: bool = args
        .get("include_descriptions")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let summary = get_checklist_summary(store, task_id).map_err(store_err)?;
    Ok(CallToolResult::text_result(summary))
}

// 14. clear_task_handler
pub fn clear_task_handler(
    store: &TaskStore,
    args: Value,
) -> Result<CallToolResult, String> {
    let task_id = require_str!(args, "task_id");
    clear_task(store, task_id).map_err(store_err)?;
    Ok(CallToolResult::text_result(format!("任务 {task_id} 已删除")))
}

// 15. list_tasks_handler
pub fn list_tasks_handler(
    store: &TaskStore,
    _args: Value,
) -> Result<CallToolResult, String> {
    let tasks = store.list_tasks().map_err(store_err)?;
    if tasks.is_empty() {
        return Ok(CallToolResult::text_result("当前没有任何任务"));
    }
    let mut summary = String::new();
    for task in &tasks {
        let total = task.checklist.len();
        let done = task.checklist.iter().filter(|i| i.done).count();
        summary.push_str(&format!(
            "ID: {}\n任务: {}\n进度: {done}/{total}\n创建时间: {}\n\n",
            task.id, task.task_description, task.created_at
        ));
    }
    Ok(CallToolResult::text_result(summary))
}

// 16. get_current_task_details_handler
pub fn get_current_task_details_handler(
    store: &TaskStore,
    _args: Value,
) -> Result<CallToolResult, String> {
    let tasks = store.list_tasks().map_err(store_err)?;
    // 找到第一个有未完成清单条目的任务
    let current_task = tasks
        .iter()
        .find(|t| t.checklist.iter().any(|item| !item.done))
        .ok_or_else(|| "没有找到包含未完成子任务的任务".to_owned())?;
    let details = get_current_task_details(store, &current_task.id).map_err(store_err)?;
    Ok(CallToolResult::text_result(details))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 创建使用临时目录的 TaskStore。
    fn temp_store() -> (TaskStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let store = TaskStore::new(Some(dir.path().to_path_buf())).expect("创建 TaskStore 失败");
        (store, dir)
    }

    #[test]
    fn initialize_task_creates_and_persists() {
        let (store, _dir) = temp_store();

        let task = initialize_task(
            &store,
            "测试任务",
            Some("共享上下文"),
            vec![],
            vec![],
            vec![],
            None,
        )
        .expect("创建任务失败");

        assert_eq!(task.task_description, "测试任务");
        assert_eq!(task.context_for_all_tasks, Some("共享上下文".to_owned()));
        assert!(task.id.len() > 0);
    }

    #[test]
    fn add_checklist_item_and_mark_done() {
        let (store, _dir) = temp_store();

        let task = initialize_task(&store, "t1", None, vec![], vec![], vec![], None)
            .expect("创建任务失败");

        let task =
            add_checklist_item(&store, &task.id, "步骤1", "详细描述", Some("计划"))
                .expect("添加清单条目失败");
        assert_eq!(task.checklist.len(), 1);
        assert!(!task.checklist[0].done);

        let task = mark_task_done(&store, &task.id, 0).expect("标记完成失败");
        assert!(task.checklist[0].done);
    }

    #[test]
    fn add_note_and_resource() {
        let (store, _dir) = temp_store();

        let task = initialize_task(&store, "t2", None, vec![], vec![], vec![], None)
            .expect("创建任务失败");

        let task = add_note(&store, &task.id, "一条笔记").expect("添加笔记失败");
        assert_eq!(task.notes, vec!["一条笔记"]);

        let task = add_resource(&store, &task.id, "文档", "https://example.com", Some("示例"))
            .expect("添加资源失败");
        assert_eq!(task.resources.len(), 1);
        assert_eq!(task.resources[0].name, "文档");
    }
}
