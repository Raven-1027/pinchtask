//! 清单条目操作：添加、更新、删除、重排、标记完成/未完成。

use uuid::Uuid;

use crate::models::task::ChecklistItem;
use crate::store::{StoreError, TaskStore};

/// 向任务清单中添加一个条目。
pub async fn add_checklist_item(
    store: &TaskStore,
    task_id: &str,
    task_name: &str,
    detailed_description: &str,
    context_and_plan: Option<&str>,
) -> Result<crate::models::task::Task, StoreError> {
    let mut task = store.get_task(task_id).await?;
    task.checklist.push(ChecklistItem {
        id: Uuid::new_v4().to_string(),
        task: task_name.to_owned(),
        detailed_description: detailed_description.to_owned(),
        context_and_plan: context_and_plan.map(|s| s.to_owned()),
        done: false,
    });
    store.update_task(&mut task).await?;
    Ok(task)
}

/// 更新指定清单条目的内容。
///
/// 只更新传入的非 None 字段，保留未指定字段的原值。
/// `context_and_plan` 使用 `Option<Option<&str>>` 以区分"未传入"和"传入 None（清空）"。
pub async fn update_checklist_item(
    store: &TaskStore,
    task_id: &str,
    item_index: usize,
    task_name: Option<&str>,
    detailed_description: Option<&str>,
    context_and_plan: Option<Option<&str>>,
    done: Option<bool>,
) -> Result<crate::models::task::Task, StoreError> {
    let mut task = store.get_task(task_id).await?;
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
    if let Some(cap) = context_and_plan {
        item.context_and_plan = cap.map(|s| s.to_owned());
    }
    if let Some(d) = done {
        item.done = d;
    }
    store.update_task(&mut task).await?;
    Ok(task)
}

/// 将指定清单条目标记为已完成。
pub async fn mark_task_done(
    store: &TaskStore,
    task_id: &str,
    item_index: usize,
) -> Result<crate::models::task::Task, StoreError> {
    let mut task = store.get_task(task_id).await?;
    if item_index >= task.checklist.len() {
        return Err(StoreError::NotFound(format!(
            "清单条目索引越界: {item_index}"
        )));
    }
    task.checklist[item_index].done = true;
    store.update_task(&mut task).await?;
    Ok(task)
}

/// 将指定清单条目标记为未完成。
pub async fn mark_task_undone(
    store: &TaskStore,
    task_id: &str,
    item_index: usize,
) -> Result<crate::models::task::Task, StoreError> {
    let mut task = store.get_task(task_id).await?;
    if item_index >= task.checklist.len() {
        return Err(StoreError::NotFound(format!(
            "清单条目索引越界: {item_index}"
        )));
    }
    task.checklist[item_index].done = false;
    store.update_task(&mut task).await?;
    Ok(task)
}

/// 删除指定索引处的清单条目。
pub async fn remove_checklist_item(
    store: &TaskStore,
    task_id: &str,
    item_index: usize,
) -> Result<crate::models::task::Task, StoreError> {
    let mut task = store.get_task(task_id).await?;
    if item_index >= task.checklist.len() {
        return Err(StoreError::NotFound(format!(
            "清单条目索引越界: {item_index}"
        )));
    }
    task.checklist.remove(item_index);
    store.update_task(&mut task).await?;
    Ok(task)
}

/// 将清单条目从 from_index 移动到 to_index。
///
/// 先移除原位置的条目，再插入到目标位置。
pub async fn reorder_checklist_item(
    store: &TaskStore,
    task_id: &str,
    from_index: usize,
    to_index: usize,
) -> Result<crate::models::task::Task, StoreError> {
    let mut task = store.get_task(task_id).await?;
    if from_index >= task.checklist.len() {
        return Err(StoreError::NotFound(format!("源索引越界: {from_index}")));
    }
    // to_index 允许等于 checklist.len()（追加到末尾），但不能更大
    if to_index > task.checklist.len() {
        return Err(StoreError::NotFound(format!("目标索引越界: {to_index}")));
    }
    let item = task.checklist.remove(from_index);
    // 标准做法：先 remove，然后 min(to_index, len) 作为插入点
    let insert_at = to_index.min(task.checklist.len());
    task.checklist.insert(insert_at, item);
    store.update_task(&mut task).await?;
    Ok(task)
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

    /// 辅助：创建一个包含 3 个条目的测试任务，返回 (store, task)。
    async fn task_with_3_items() -> (TaskStore, tempfile::TempDir, crate::models::task::Task) {
        let (store, dir) = temp_store().await;
        let items: Vec<ChecklistItem> = (0..3)
            .map(|i| ChecklistItem {
                id: Uuid::new_v4().to_string(),
                task: format!("条目{i}"),
                detailed_description: format!("描述{i}"),
                context_and_plan: None,
                done: false,
            })
            .collect();
        let task = store
            .create_task("测试任务", None, items, vec![], vec![], None, None)
            .await
            .expect("创建任务失败");
        (store, dir, task)
    }

    // ------------------------------------------------------------------
    // add_checklist_item
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn add_checklist_item_success() {
        let (store, _dir) = temp_store().await;
        let task = store
            .create_task("t", None, vec![], vec![], vec![], None, None)
            .await
            .expect("创建任务失败");

        let result = add_checklist_item(&store, &task.id, "新条目", "详细说明", Some("计划"))
            .await
            .expect("添加条目失败");

        assert_eq!(result.checklist.len(), 1);
        assert_eq!(result.checklist[0].task, "新条目");
        assert_eq!(result.checklist[0].detailed_description, "详细说明");
        assert_eq!(
            result.checklist[0].context_and_plan,
            Some("计划".to_owned())
        );
        assert!(!result.checklist[0].done);
        assert!(!result.checklist[0].id.is_empty());
    }

    #[tokio::test]
    async fn add_checklist_item_without_plan() {
        let (store, _dir) = temp_store().await;
        let task = store
            .create_task("t", None, vec![], vec![], vec![], None, None)
            .await
            .expect("创建任务失败");

        let result = add_checklist_item(&store, &task.id, "条目", "描述", None)
            .await
            .expect("添加条目失败");

        assert_eq!(result.checklist[0].context_and_plan, None);
    }

    #[tokio::test]
    async fn add_checklist_item_nonexistent_task() {
        let (store, _dir) = temp_store().await;
        let result = add_checklist_item(&store, "不存在的ID", "条目", "描述", None).await;
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }

    // ------------------------------------------------------------------
    // update_checklist_item
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn update_checklist_item_all_fields() {
        let (store, _dir, task) = task_with_3_items().await;

        let result = update_checklist_item(
            &store,
            &task.id,
            1,
            Some("新名称"),
            Some("新描述"),
            Some(Some("新计划")),
            Some(true),
        )
        .await
        .expect("更新条目失败");

        assert_eq!(result.checklist[1].task, "新名称");
        assert_eq!(result.checklist[1].detailed_description, "新描述");
        assert_eq!(
            result.checklist[1].context_and_plan,
            Some("新计划".to_owned())
        );
        assert!(result.checklist[1].done);

        // 其他条目不受影响
        assert_eq!(result.checklist[0].task, "条目0");
        assert_eq!(result.checklist[2].task, "条目2");
    }

    #[tokio::test]
    async fn update_checklist_item_partial_fields() {
        let (store, _dir, task) = task_with_3_items().await;

        // 只更新 task_name
        let result = update_checklist_item(&store, &task.id, 0, Some("仅改名称"), None, None, None)
            .await
            .expect("更新条目失败");

        assert_eq!(result.checklist[0].task, "仅改名称");
        assert_eq!(result.checklist[0].detailed_description, "描述0");
    }

    #[tokio::test]
    async fn update_checklist_item_clear_context_and_plan() {
        let (store, _dir, task) = task_with_3_items().await;

        // 先设置 context_and_plan
        let _ = update_checklist_item(&store, &task.id, 0, None, None, Some(Some("有计划")), None)
            .await
            .expect("更新条目失败");

        // 再清空 context_and_plan
        let result = update_checklist_item(
            &store,
            &task.id,
            0,
            None,
            None,
            Some(None), // 传入 None 清空
            None,
        )
        .await
        .expect("更新条目失败");

        assert_eq!(result.checklist[0].context_and_plan, None);
    }

    #[tokio::test]
    async fn update_checklist_item_invalid_index() {
        let (store, _dir, task) = task_with_3_items().await;
        let result = update_checklist_item(
            &store,
            &task.id,
            10, // 越界
            Some("x"),
            None,
            None,
            None,
        )
        .await;
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }

    #[tokio::test]
    async fn update_checklist_item_nonexistent_task() {
        let (store, _dir) = temp_store().await;
        let result =
            update_checklist_item(&store, "不存在的ID", 0, Some("x"), None, None, None).await;
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }

    // ------------------------------------------------------------------
    // mark_task_done / mark_task_undone
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn mark_task_done_success() {
        let (store, _dir, task) = task_with_3_items().await;
        let result = mark_task_done(&store, &task.id, 1)
            .await
            .expect("标记完成失败");
        assert!(result.checklist[1].done);
        assert!(!result.checklist[0].done);
        assert!(!result.checklist[2].done);
    }

    #[tokio::test]
    async fn mark_task_done_invalid_index() {
        let (store, _dir, task) = task_with_3_items().await;
        let result = mark_task_done(&store, &task.id, 99).await;
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }

    #[tokio::test]
    async fn mark_task_undone_success() {
        let (store, _dir) = temp_store().await;
        let item = ChecklistItem {
            id: Uuid::new_v4().to_string(),
            task: "已完成项".to_owned(),
            detailed_description: "".to_owned(),
            context_and_plan: None,
            done: true,
        };
        let task = store
            .create_task("t", None, vec![item], vec![], vec![], None, None)
            .await
            .expect("创建失败");

        let result = mark_task_undone(&store, &task.id, 0)
            .await
            .expect("标记未完成失败");
        assert!(!result.checklist[0].done);
    }

    #[tokio::test]
    async fn mark_task_undone_invalid_index() {
        let (store, _dir, task) = task_with_3_items().await;
        let result = mark_task_undone(&store, &task.id, 99).await;
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }

    #[tokio::test]
    async fn mark_task_done_nonexistent_task() {
        let (store, _dir) = temp_store().await;
        let result = mark_task_done(&store, "不存在的ID", 0).await;
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }

    // ------------------------------------------------------------------
    // remove_checklist_item
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn remove_checklist_item_success() {
        let (store, _dir, task) = task_with_3_items().await;

        let result = remove_checklist_item(&store, &task.id, 1)
            .await
            .expect("删除条目失败");
        assert_eq!(result.checklist.len(), 2);
        assert_eq!(result.checklist[0].task, "条目0");
        assert_eq!(result.checklist[1].task, "条目2");
    }

    #[tokio::test]
    async fn remove_checklist_item_invalid_index() {
        let (store, _dir, task) = task_with_3_items().await;
        let result = remove_checklist_item(&store, &task.id, 5).await;
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }

    #[tokio::test]
    async fn remove_checklist_item_nonexistent_task() {
        let (store, _dir) = temp_store().await;
        let result = remove_checklist_item(&store, "不存在的ID", 0).await;
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }

    #[tokio::test]
    async fn remove_checklist_item_all_and_add() {
        let (store, _dir, task) = task_with_3_items().await;

        // 删除所有条目
        remove_checklist_item(&store, &task.id, 0)
            .await
            .expect("删除失败");
        remove_checklist_item(&store, &task.id, 0)
            .await
            .expect("删除失败");
        remove_checklist_item(&store, &task.id, 0)
            .await
            .expect("删除失败");

        let reloaded = store.get_task(&task.id).await.expect("获取失败");
        assert!(reloaded.checklist.is_empty());

        // 清空后重新添加
        let result = add_checklist_item(&store, &task.id, "新条目", "描述", None)
            .await
            .expect("添加失败");
        assert_eq!(result.checklist.len(), 1);
    }

    // ------------------------------------------------------------------
    // reorder_checklist_item
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn reorder_checklist_item_forward() {
        let (store, _dir, task) = task_with_3_items().await;

        // 从索引 0 移到索引 2
        let result = reorder_checklist_item(&store, &task.id, 0, 2)
            .await
            .expect("重排序失败");
        assert_eq!(result.checklist[0].task, "条目1");
        assert_eq!(result.checklist[1].task, "条目2");
        assert_eq!(result.checklist[2].task, "条目0");
    }

    #[tokio::test]
    async fn reorder_checklist_item_backward() {
        let (store, _dir, task) = task_with_3_items().await;

        // 从索引 2 移到索引 0
        let result = reorder_checklist_item(&store, &task.id, 2, 0)
            .await
            .expect("重排序失败");
        assert_eq!(result.checklist[0].task, "条目2");
        assert_eq!(result.checklist[1].task, "条目0");
        assert_eq!(result.checklist[2].task, "条目1");
    }

    #[tokio::test]
    async fn reorder_checklist_item_to_end() {
        let (store, _dir, task) = task_with_3_items().await;

        // 从索引 0 移到末尾（to_index = 2，等于 len - 1）
        let result = reorder_checklist_item(&store, &task.id, 0, 2)
            .await
            .expect("重排序失败");
        assert_eq!(result.checklist[2].task, "条目0");
    }

    #[tokio::test]
    async fn reorder_checklist_item_to_same_position() {
        let (store, _dir, task) = task_with_3_items().await;

        let result = reorder_checklist_item(&store, &task.id, 1, 1)
            .await
            .expect("重排序失败");
        assert_eq!(result.checklist[0].task, "条目0");
        assert_eq!(result.checklist[1].task, "条目1");
        assert_eq!(result.checklist[2].task, "条目2");
    }

    #[tokio::test]
    async fn reorder_checklist_item_from_index_out_of_bounds() {
        let (store, _dir, task) = task_with_3_items().await;
        let result = reorder_checklist_item(&store, &task.id, 5, 0).await;
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }

    #[tokio::test]
    async fn reorder_checklist_item_to_index_out_of_bounds() {
        let (store, _dir, task) = task_with_3_items().await;
        let result = reorder_checklist_item(&store, &task.id, 0, 10).await;
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }

    #[tokio::test]
    async fn reorder_checklist_item_nonexistent_task() {
        let (store, _dir) = temp_store().await;
        let result = reorder_checklist_item(&store, "不存在的ID", 0, 1).await;
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }
}
