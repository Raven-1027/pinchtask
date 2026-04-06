//! 项目级操作：创建、查询、更新、删除、任务关联。

use crate::models::project::Project;
use crate::models::task::Task;
use crate::store::{StoreError, TaskStore};

/// 创建新项目。
pub async fn create_project(
    store: &TaskStore,
    name: &str,
    description: Option<&str>,
) -> Result<Project, StoreError> {
    store.create_project(name, description).await
}

/// 根据 ID 获取项目。
pub async fn get_project(store: &TaskStore, id: &str) -> Result<Project, StoreError> {
    store.get_project(id).await
}

/// 更新项目名称和/或描述。
///
/// 传入的字段会在一次调用中全部更新，未传入的字段保持不变。
pub async fn update_project(
    store: &TaskStore,
    id: &str,
    name: Option<&str>,
    description: Option<&str>,
) -> Result<Project, StoreError> {
    let mut project = store.get_project(id).await?;
    if let Some(n) = name {
        project.name = n.to_owned();
    }
    if let Some(d) = description {
        project.description = Some(d.to_owned());
    }
    store.update_project(&mut project).await?;
    Ok(project)
}

/// 删除项目（保留关联任务，task_projects 通过 CASCADE 自动清理）。
pub async fn delete_project(store: &TaskStore, id: &str) -> Result<(), StoreError> {
    store.delete_project(id).await
}

/// 删除项目及其所有关联任务。
pub async fn delete_project_with_tasks(store: &TaskStore, id: &str) -> Result<(), StoreError> {
    store.delete_project_with_tasks(id).await
}

/// 列出所有项目。
pub async fn list_projects(store: &TaskStore) -> Result<Vec<Project>, StoreError> {
    store.list_projects().await
}

/// 将任务添加到项目。
pub async fn add_task_to_project(
    store: &TaskStore,
    task_id: &str,
    project_id: &str,
) -> Result<(), StoreError> {
    store.add_task_to_project(task_id, project_id).await
}

/// 将任务从项目中移除。
pub async fn remove_task_from_project(
    store: &TaskStore,
    task_id: &str,
    project_id: &str,
) -> Result<(), StoreError> {
    store.remove_task_from_project(task_id, project_id).await
}

/// 获取指定任务关联的所有项目。
pub async fn get_projects_for_task(
    store: &TaskStore,
    task_id: &str,
) -> Result<Vec<Project>, StoreError> {
    store.get_projects_for_task(task_id).await
}

/// 获取指定项目关联的所有任务。
pub async fn get_tasks_for_project(
    store: &TaskStore,
    project_id: &str,
) -> Result<Vec<Task>, StoreError> {
    store.get_tasks_for_project(project_id).await
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

    #[tokio::test]
    async fn create_project_basic() {
        let (store, _dir) = temp_store().await;
        let project = create_project(&store, "测试项目", Some("项目描述"))
            .await
            .expect("创建项目失败");

        assert!(!project.id.is_empty());
        assert_eq!(project.name, "测试项目");
        assert_eq!(project.description, Some("项目描述".to_owned()));
        assert!(!project.created_at.is_empty());
    }

    #[tokio::test]
    async fn create_project_without_description() {
        let (store, _dir) = temp_store().await;
        let project = create_project(&store, "简洁项目", None)
            .await
            .expect("创建项目失败");
        assert_eq!(project.name, "简洁项目");
        assert!(project.description.is_none());
    }

    #[tokio::test]
    async fn get_project_success() {
        let (store, _dir) = temp_store().await;
        let created = create_project(&store, "查询项目", None)
            .await
            .expect("创建项目失败");

        let loaded = get_project(&store, &created.id).await.expect("获取项目失败");
        assert_eq!(loaded.id, created.id);
        assert_eq!(loaded.name, "查询项目");
    }

    #[tokio::test]
    async fn get_project_nonexistent() {
        let (store, _dir) = temp_store().await;
        let result = get_project(&store, "不存在").await;
        assert!(matches!(result, Err(StoreError::ProjectNotFound(_))));
    }

    #[tokio::test]
    async fn update_project_name() {
        let (store, _dir) = temp_store().await;
        let created = create_project(&store, "旧名称", None)
            .await
            .expect("创建项目失败");

        let updated = update_project(&store, &created.id, Some("新名称"), None)
            .await
            .expect("更新项目失败");
        assert_eq!(updated.name, "新名称");
        assert!(updated.description.is_none());
    }

    #[tokio::test]
    async fn update_project_description() {
        let (store, _dir) = temp_store().await;
        let created = create_project(&store, "项目", None)
            .await
            .expect("创建项目失败");

        let updated = update_project(&store, &created.id, None, Some("新增描述"))
            .await
            .expect("更新项目失败");
        assert_eq!(updated.description, Some("新增描述".to_owned()));
    }

    #[tokio::test]
    async fn list_projects_empty() {
        let (store, _dir) = temp_store().await;
        let projects = list_projects(&store).await.expect("列出项目失败");
        assert!(projects.is_empty());
    }

    #[tokio::test]
    async fn list_projects_multiple() {
        let (store, _dir) = temp_store().await;
        create_project(&store, "项目A", None)
            .await
            .expect("创建项目失败");
        create_project(&store, "项目B", None)
            .await
            .expect("创建项目失败");

        let projects = list_projects(&store).await.expect("列出项目失败");
        assert_eq!(projects.len(), 2);
        assert_eq!(projects[0].name, "项目A");
        assert_eq!(projects[1].name, "项目B");
    }

    #[tokio::test]
    async fn delete_project_success() {
        let (store, _dir) = temp_store().await;
        let created = create_project(&store, "待删除", None)
            .await
            .expect("创建项目失败");

        delete_project(&store, &created.id)
            .await
            .expect("删除项目失败");
        assert!(get_project(&store, &created.id).await.is_err());
    }

    #[tokio::test]
    async fn delete_project_nonexistent() {
        let (store, _dir) = temp_store().await;
        let result = delete_project(&store, "不存在").await;
        assert!(matches!(result, Err(StoreError::ProjectNotFound(_))));
    }

    #[tokio::test]
    async fn add_and_remove_task_to_project() {
        let (store, _dir) = temp_store().await;
        let project = create_project(&store, "关联项目", None)
            .await
            .expect("创建项目失败");
        let task = store
            .create_task("关联任务", None, vec![], vec![], vec![], None, None)
            .await
            .expect("创建任务失败");

        // 添加关联
        add_task_to_project(&store, &task.id, &project.id)
            .await
            .expect("添加关联失败");

        // 查询验证
        let projects = get_projects_for_task(&store, &task.id)
            .await
            .expect("获取项目失败");
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "关联项目");

        let tasks = get_tasks_for_project(&store, &project.id)
            .await
            .expect("获取任务失败");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task_description, "关联任务");

        // 移除关联
        remove_task_from_project(&store, &task.id, &project.id)
            .await
            .expect("移除关联失败");
        let projects = get_projects_for_task(&store, &task.id)
            .await
            .expect("获取项目失败");
        assert!(projects.is_empty());
    }

    #[tokio::test]
    async fn delete_project_cascade_removes_associations() {
        let (store, _dir) = temp_store().await;
        let project = create_project(&store, "级联项目", None)
            .await
            .expect("创建项目失败");
        let task = store
            .create_task("级联任务", None, vec![], vec![], vec![], None, None)
            .await
            .expect("创建任务失败");

        add_task_to_project(&store, &task.id, &project.id)
            .await
            .expect("添加关联失败");

        // 删除项目后，任务应保留但关联应被清理
        delete_project(&store, &project.id)
            .await
            .expect("删除项目失败");

        // 任务仍存在
        let loaded = store.get_task(&task.id).await.expect("任务应仍存在");
        assert_eq!(loaded.task_description, "级联任务");
    }

    #[tokio::test]
    async fn delete_project_with_tasks_removes_all() {
        let (store, _dir) = temp_store().await;
        let project = create_project(&store, "智能删除项目", None)
            .await
            .expect("创建项目失败");
        let task = store
            .create_task("将被删除的任务", None, vec![], vec![], vec![], None, None)
            .await
            .expect("创建任务失败");

        add_task_to_project(&store, &task.id, &project.id)
            .await
            .expect("添加关联失败");

        // 智能删除：项目和关联任务都应被删除
        delete_project_with_tasks(&store, &project.id)
            .await
            .expect("智能删除失败");

        assert!(get_project(&store, &project.id).await.is_err());
        assert!(store.get_task(&task.id).await.is_err());
    }

    #[tokio::test]
    async fn create_task_with_project_ids() {
        let (store, _dir) = temp_store().await;
        let project = create_project(&store, "初始关联项目", None)
            .await
            .expect("创建项目失败");

        let pids = vec![project.id.clone()];
        let task = store
            .create_task(
                "带项目的任务",
                None,
                vec![],
                vec![],
                vec![],
                None,
                Some(&pids),
            )
            .await
            .expect("创建任务失败");

        let projects = get_projects_for_task(&store, &task.id)
            .await
            .expect("获取项目失败");
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "初始关联项目");
    }
}
