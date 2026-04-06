//! 任务级操作：创建、更新、删除、列表、摘要。

use crate::models::task::{ChecklistItem, Resource, Task, TaskMetadata};
use crate::store::{StoreError, TaskStore};

/// 初始化一个新任务并持久化。
///
/// 如果提供了 `project_id`，任务创建时直接关联到指定项目。
#[allow(clippy::too_many_arguments)]
pub async fn initialize_task(
    store: &TaskStore,
    task_description: &str,
    context_for_all_tasks: Option<&str>,
    initial_checklist: Vec<ChecklistItem>,
    notes: Vec<String>,
    resources: Vec<Resource>,
    metadata: Option<TaskMetadata>,
    project_id: Option<&str>,
) -> Result<Task, StoreError> {
    store
        .create_task(
            task_description,
            context_for_all_tasks,
            initial_checklist,
            notes,
            resources,
            metadata,
            project_id,
        )
        .await
}

/// 更新任务的整体描述。
pub async fn update_task_description(
    store: &TaskStore,
    task_id: &str,
    new_description: &str,
) -> Result<Task, StoreError> {
    let mut task = store.get_task(task_id).await?;
    task.task_description = new_description.to_owned();
    store.update_task(&mut task).await?;
    Ok(task)
}

/// 更新所有子任务的共享上下文。
pub async fn update_context(
    store: &TaskStore,
    task_id: &str,
    new_context: &str,
) -> Result<Task, StoreError> {
    let mut task = store.get_task(task_id).await?;
    task.context_for_all_tasks = Some(new_context.to_owned());
    store.update_task(&mut task).await?;
    Ok(task)
}

/// 更新任务元数据。
pub async fn update_metadata(
    store: &TaskStore,
    task_id: &str,
    metadata: TaskMetadata,
) -> Result<Task, StoreError> {
    let mut task = store.get_task(task_id).await?;
    task.metadata = Some(metadata);
    store.update_task(&mut task).await?;
    Ok(task)
}

/// 删除指定任务。
pub async fn clear_task(store: &TaskStore, task_id: &str) -> Result<(), StoreError> {
    store.delete_task(task_id).await
}

/// 获取任务的清单概要（含完成状态）。
pub async fn get_checklist_summary(store: &TaskStore, task_id: &str) -> Result<String, StoreError> {
    let task = store.get_task(task_id).await?;
    let total = task.checklist.len();
    let done = task.checklist.iter().filter(|i| i.done).count();
    let mut summary = format!("任务: {}\n进度: {done}/{total}\n\n", task.task_description);
    for (i, item) in task.checklist.iter().enumerate() {
        let status = if item.done { "✅" } else { "⬜" };
        summary.push_str(&format!("{status} [{i}] {}\n", item.task));
    }
    Ok(summary)
}

/// 列出所有任务并生成文本摘要。
pub async fn list_tasks_summary(store: &TaskStore) -> Result<String, StoreError> {
    let tasks = store.list_tasks().await?;
    if tasks.is_empty() {
        return Ok("当前没有任何任务".to_owned());
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
    Ok(summary)
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// 辅助：创建一个使用临时目录的 TaskStore。
    async fn temp_store() -> (TaskStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let store = TaskStore::new(Some(dir.path().to_path_buf()))
            .await
            .expect("创建 TaskStore 失败");
        (store, dir)
    }

    // ------------------------------------------------------------------
    // initialize_task
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn initialize_task_basic() {
        let (store, _dir) = temp_store().await;
        let task = initialize_task(&store, "基础任务", None, vec![], vec![], vec![], None, None)
            .await
            .expect("创建任务失败");

        assert!(!task.id.is_empty());
        assert_eq!(task.task_description, "基础任务");
        assert!(task.context_for_all_tasks.is_none());
        assert!(task.checklist.is_empty());
        assert!(task.notes.is_empty());
        assert!(task.resources.is_empty());
        assert!(task.metadata.is_none());
        assert!(task.project_id.is_none());
        assert!(!task.created_at.is_empty());
        assert_eq!(task.created_at, task.updated_at);
    }

    #[tokio::test]
    async fn initialize_task_with_all_fields() {
        let (store, _dir) = temp_store().await;
        let item = ChecklistItem {
            id: uuid::Uuid::new_v4().to_string(),
            task: "子任务".to_owned(),
            detailed_description: "详细描述".to_owned(),
            context_and_plan: Some("计划".to_owned()),
            done: false,
        };
        let res = Resource {
            name: "文档".to_owned(),
            url: "https://example.com".to_owned(),
            description: Some("参考文档".to_owned()),
        };
        let meta = TaskMetadata {
            tags: Some(vec!["重要".to_owned()]),
            priority: Some("high".to_owned()),
            estimated_completion_time: Some("P3D".to_owned()),
        };

        let task = initialize_task(
            &store,
            "完整任务",
            Some("共享上下文"),
            vec![item],
            vec!["笔记".to_owned()],
            vec![res],
            Some(meta),
            None,
        )
        .await
        .expect("创建任务失败");

        assert_eq!(task.context_for_all_tasks, Some("共享上下文".to_owned()));
        assert_eq!(task.checklist.len(), 1);
        assert_eq!(task.notes, vec!["笔记"]);
        assert_eq!(task.resources.len(), 1);
        assert!(task.metadata.is_some());
    }

    #[tokio::test]
    async fn initialize_task_empty_description() {
        let (store, _dir) = temp_store().await;
        let task = initialize_task(&store, "", None, vec![], vec![], vec![], None, None)
            .await
            .expect("创建任务失败");
        assert_eq!(task.task_description, "");
    }

    #[tokio::test]
    async fn initialize_task_long_description() {
        let (store, _dir) = temp_store().await;
        let long_desc = "x".repeat(10_000);
        let task = initialize_task(&store, &long_desc, None, vec![], vec![], vec![], None, None)
            .await
            .expect("创建任务失败");
        assert_eq!(task.task_description.len(), 10_000);
    }

    // ------------------------------------------------------------------
    // update_task_description
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn update_task_description_success() {
        let (store, _dir) = temp_store().await;
        let task = initialize_task(&store, "旧描述", None, vec![], vec![], vec![], None, None)
            .await
            .expect("创建任务失败");

        let updated = update_task_description(&store, &task.id, "新描述")
            .await
            .expect("更新描述失败");
        assert_eq!(updated.task_description, "新描述");

        // 验证持久化
        let reloaded = store.get_task(&task.id).await.expect("获取任务失败");
        assert_eq!(reloaded.task_description, "新描述");
    }

    #[tokio::test]
    async fn update_task_description_nonexistent() {
        let (store, _dir) = temp_store().await;
        let result = update_task_description(&store, "不存在的ID", "新描述").await;
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }

    // ------------------------------------------------------------------
    // update_context
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn update_context_success() {
        let (store, _dir) = temp_store().await;
        let task = initialize_task(&store, "任务", None, vec![], vec![], vec![], None, None)
            .await
            .expect("创建任务失败");

        let updated = update_context(&store, &task.id, "新的上下文")
            .await
            .expect("更新上下文失败");
        assert_eq!(updated.context_for_all_tasks, Some("新的上下文".to_owned()));
    }

    #[tokio::test]
    async fn update_context_nonexistent() {
        let (store, _dir) = temp_store().await;
        let result = update_context(&store, "不存在的ID", "上下文").await;
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }

    // ------------------------------------------------------------------
    // update_metadata
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn update_metadata_success() {
        let (store, _dir) = temp_store().await;
        let task = initialize_task(&store, "任务", None, vec![], vec![], vec![], None, None)
            .await
            .expect("创建任务失败");

        let meta = TaskMetadata {
            tags: Some(vec!["v2".to_owned()]),
            priority: Some("medium".to_owned()),
            estimated_completion_time: None,
        };
        let updated = update_metadata(&store, &task.id, meta)
            .await
            .expect("更新元数据失败");
        let m = updated.metadata.expect("元数据不应为空");
        assert_eq!(m.tags, Some(vec!["v2".to_owned()]));
        assert_eq!(m.priority, Some("medium".to_owned()));
        assert!(m.estimated_completion_time.is_none());
    }

    #[tokio::test]
    async fn update_metadata_nonexistent() {
        let (store, _dir) = temp_store().await;
        let meta = TaskMetadata {
            tags: None,
            priority: None,
            estimated_completion_time: None,
        };
        let result = update_metadata(&store, "不存在的ID", meta).await;
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }

    // ------------------------------------------------------------------
    // clear_task
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn clear_task_success() {
        let (store, _dir) = temp_store().await;
        let task = initialize_task(&store, "待删除", None, vec![], vec![], vec![], None, None)
            .await
            .expect("创建任务失败");

        clear_task(&store, &task.id).await.expect("删除任务失败");
        assert!(store.get_task(&task.id).await.is_err());
    }

    #[tokio::test]
    async fn clear_task_nonexistent() {
        let (store, _dir) = temp_store().await;
        let result = clear_task(&store, "不存在的ID").await;
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }

    // ------------------------------------------------------------------
    // get_checklist_summary
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn get_checklist_summary_with_items() {
        let (store, _dir) = temp_store().await;
        let item_done = ChecklistItem {
            id: uuid::Uuid::new_v4().to_string(),
            task: "已完成项".to_owned(),
            detailed_description: "".to_owned(),
            context_and_plan: None,
            done: true,
        };
        let item_pending = ChecklistItem {
            id: uuid::Uuid::new_v4().to_string(),
            task: "待办项".to_owned(),
            detailed_description: "".to_owned(),
            context_and_plan: None,
            done: false,
        };
        let task = initialize_task(
            &store,
            "摘要任务",
            None,
            vec![item_done, item_pending],
            vec![],
            vec![],
            None,
            None,
        )
        .await
        .expect("创建任务失败");

        let summary = get_checklist_summary(&store, &task.id)
            .await
            .expect("获取摘要失败");
        assert!(summary.contains("进度: 1/2"));
        assert!(summary.contains("✅"));
        assert!(summary.contains("⬜"));
    }

    #[tokio::test]
    async fn get_checklist_summary_nonexistent() {
        let (store, _dir) = temp_store().await;
        let result = get_checklist_summary(&store, "不存在的ID").await;
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }

    // ------------------------------------------------------------------
    // list_tasks_summary
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn list_tasks_summary_empty() {
        let (store, _dir) = temp_store().await;
        let summary = list_tasks_summary(&store).await.expect("列出失败");
        assert_eq!(summary, "当前没有任何任务");
    }

    #[tokio::test]
    async fn list_tasks_summary_with_tasks() {
        let (store, _dir) = temp_store().await;
        let t1 = initialize_task(&store, "任务A", None, vec![], vec![], vec![], None, None)
            .await
            .expect("创建失败");
        let summary = list_tasks_summary(&store).await.expect("列出失败");
        assert!(summary.contains(&t1.id));
        assert!(summary.contains("任务A"));
    }
}
