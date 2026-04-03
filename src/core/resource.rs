//! 资源操作。

use crate::models::task::Resource;

/// 向任务添加一个资源引用。
pub async fn add_resource(
    store: &crate::store::TaskStore,
    task_id: &str,
    name: &str,
    url: &str,
    description: Option<&str>,
) -> Result<crate::models::task::Task, crate::store::StoreError> {
    let mut task = store.get_task(task_id).await?;
    task.resources.push(Resource {
        name: name.to_owned(),
        url: url.to_owned(),
        description: description.map(|s| s.to_owned()),
    });
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
    async fn add_resource_with_description() {
        let (store, _dir) = temp_store().await;
        let task = store
            .create_task("t", None, vec![], vec![], vec![], None)
            .await
            .expect("创建任务失败");

        let result = add_resource(
            &store,
            &task.id,
            "API文档",
            "https://docs.example.com",
            Some("官方文档"),
        )
        .await
        .expect("添加资源失败");

        assert_eq!(result.resources.len(), 1);
        assert_eq!(result.resources[0].name, "API文档");
        assert_eq!(result.resources[0].url, "https://docs.example.com");
        assert_eq!(result.resources[0].description, Some("官方文档".to_owned()));
    }

    #[tokio::test]
    async fn add_resource_without_description() {
        let (store, _dir) = temp_store().await;
        let task = store
            .create_task("t", None, vec![], vec![], vec![], None)
            .await
            .expect("创建任务失败");

        let result = add_resource(&store, &task.id, "源码", "https://github.com", None)
            .await
            .expect("添加资源失败");

        assert_eq!(result.resources[0].description, None);
    }

    #[tokio::test]
    async fn add_resource_persists_to_db() {
        let (store, _dir) = temp_store().await;
        let task = store
            .create_task("t", None, vec![], vec![], vec![], None)
            .await
            .expect("创建任务失败");

        add_resource(&store, &task.id, "文档", "https://example.com", None)
            .await
            .expect("添加资源失败");

        let reloaded = store.get_task(&task.id).await.expect("获取任务失败");
        assert_eq!(reloaded.resources.len(), 1);
        assert_eq!(reloaded.resources[0].name, "文档");
    }

    #[tokio::test]
    async fn add_multiple_resources() {
        let (store, _dir) = temp_store().await;
        let task = store
            .create_task("t", None, vec![], vec![], vec![], None)
            .await
            .expect("创建任务失败");

        add_resource(&store, &task.id, "R1", "https://r1.com", Some("desc1"))
            .await
            .expect("添加资源失败");
        add_resource(&store, &task.id, "R2", "https://r2.com", None)
            .await
            .expect("添加资源失败");

        let result = add_resource(&store, &task.id, "R3", "https://r3.com", Some("desc3"))
            .await
            .expect("添加资源失败");
        assert_eq!(result.resources.len(), 3);
        assert_eq!(result.resources[0].name, "R1");
        assert_eq!(result.resources[1].name, "R2");
        assert_eq!(result.resources[2].name, "R3");
    }

    #[tokio::test]
    async fn add_resource_nonexistent_task() {
        let (store, _dir) = temp_store().await;
        let result = add_resource(&store, "不存在的ID", "name", "url", None).await;
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }
}
