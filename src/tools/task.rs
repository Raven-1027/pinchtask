//! 任务相关的测试。
//!
//! 旧的工具 handler 已迁移到 server.rs（使用 rmcp），
//! 此模块仅保留测试 core 层的单元测试。

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

        let task = core::initialize_task(&store, "t1", None, vec![], vec![], vec![], None, None)
            .await
            .expect("创建任务失败");

        let task = core::add_checklist_item(&store, &task.id, "步骤1", "详细描述", Some("计划"))
            .await
            .expect("添加清单条目失败");
        assert_eq!(task.checklist.len(), 1);
        assert!(!task.checklist[0].done);

        let task = core::mark_task_done(&store, &task.id, 0)
            .await
            .expect("标记完成失败");
        assert!(task.checklist[0].done);
    }

    #[tokio::test]
    async fn add_note_and_resource() {
        let (store, _dir) = temp_store().await;

        let task = core::initialize_task(&store, "t2", None, vec![], vec![], vec![], None, None)
            .await
            .expect("创建任务失败");

        let task = core::add_note(&store, &task.id, "一条笔记")
            .await
            .expect("添加笔记失败");
        assert_eq!(task.notes, vec!["一条笔记"]);

        let task = core::add_resource(
            &store,
            &task.id,
            "文档",
            "https://example.com",
            Some("示例"),
        )
        .await
        .expect("添加资源失败");
        assert_eq!(task.resources.len(), 1);
        assert_eq!(task.resources[0].name, "文档");
    }
}
