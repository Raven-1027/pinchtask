//! 任务相关的 MCP 工具处理器。
//!
//! 每个公开函数对应一个 MCP 工具，待传输层就绪后注册到服务器。

use crate::models::task::{ChecklistItem, Resource, Task, TaskMetadata};
use crate::store::{StoreError, TaskStore};
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
