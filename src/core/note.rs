//! 笔记操作。

/// 删除任务的指定索引处的笔记。
///
/// # Errors
///
/// 当任务不存在或索引越界时返回 `StoreError`。
pub async fn delete_note(
    store: &crate::store::TaskStore,
    task_id: &str,
    note_index: usize,
) -> Result<crate::models::task::Task, crate::store::StoreError> {
    let mut task = store.get_task(task_id).await?;
    if note_index >= task.notes.len() {
        return Err(crate::store::StoreError::NotFound(format!(
            "笔记索引 {note_index} 越界（共 {} 条）",
            task.notes.len()
        )));
    }
    task.notes.remove(note_index);
    store.update_task(&mut task).await?;
    Ok(task)
}

/// 向任务添加一条笔记。
pub async fn add_note(
    store: &crate::store::TaskStore,
    task_id: &str,
    content: &str,
) -> Result<crate::models::task::Task, crate::store::StoreError> {
    let mut task = store.get_task(task_id).await?;
    task.notes.push(content.to_owned());
    store.update_task(&mut task).await?;
    Ok(task)
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::StoreError;

    /// 辅助：创建一个使用临时目录的 TaskStore。
    async fn temp_store() -> (crate::store::TaskStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let store = crate::store::TaskStore::new(Some(dir.path().to_path_buf()))
            .await
            .expect("创建 TaskStore 失败");
        (store, dir)
    }

    #[tokio::test]
    async fn add_note_success() {
        let (store, _dir) = temp_store().await;
        let task = store
            .create_task("t", None, vec![], vec![], vec![], None, None)
            .await
            .expect("创建任务失败");

        let result = add_note(&store, &task.id, "第一条笔记")
            .await
            .expect("添加笔记失败");
        assert_eq!(result.notes, vec!["第一条笔记"]);

        // 再添加一条
        let result = add_note(&store, &task.id, "第二条笔记")
            .await
            .expect("添加笔记失败");
        assert_eq!(result.notes, vec!["第一条笔记", "第二条笔记"]);
    }

    #[tokio::test]
    async fn add_note_persists_to_db() {
        let (store, _dir) = temp_store().await;
        let task = store
            .create_task("t", None, vec![], vec![], vec![], None, None)
            .await
            .expect("创建任务失败");

        add_note(&store, &task.id, "持久化笔记")
            .await
            .expect("添加笔记失败");

        // 重新从数据库加载验证
        let reloaded = store.get_task(&task.id).await.expect("获取任务失败");
        assert_eq!(reloaded.notes, vec!["持久化笔记"]);
    }

    #[tokio::test]
    async fn add_note_empty_content() {
        let (store, _dir) = temp_store().await;
        let task = store
            .create_task("t", None, vec![], vec![], vec![], None, None)
            .await
            .expect("创建任务失败");

        let result = add_note(&store, &task.id, "").await.expect("添加笔记失败");
        assert_eq!(result.notes, vec![""]);
    }

    #[tokio::test]
    async fn add_note_nonexistent_task() {
        let (store, _dir) = temp_store().await;
        let result = add_note(&store, "不存在的ID", "内容").await;
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }
}
