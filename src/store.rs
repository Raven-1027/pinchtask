//! SQLite 持久化层。
//!
//! `TaskStore` 使用 sqlx + SQLite 存储任务数据，
//! 默认路径为 `~/.mcp-pinchtask/tasks.db`。

use std::path::PathBuf;

use chrono::Utc;
use thiserror::Error;
use uuid::Uuid;

use crate::models::task::{ChecklistItem, Resource, Task, TaskMetadata};

/// 持久化层自定义错误类型。
#[derive(Debug, Error)]
pub enum StoreError {
    /// IO 错误（目录创建等非数据库 IO 操作）。
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    /// 数据库错误（SQL 执行、连接等）。
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),

    /// 指定任务不存在。
    #[error("任务不存在: {0}")]
    NotFound(String),
}

/// 初始化建表 SQL（从 migrations 目录嵌入）。
const MIGRATION_SQL: &str = include_str!("../migrations/20250101000000_init.sql");

/// 任务 SQLite 持久化存储。
pub struct TaskStore {
    pool: sqlx::SqlitePool,
}

impl TaskStore {
    /// 创建 TaskStore 实例。
    ///
    /// 若数据目录不存在会自动递归创建，然后初始化 SQLite 数据库并执行迁移。
    ///
    /// # 参数
    /// - `data_dir`: 数据目录路径，传入 `None` 则使用默认路径 `~/.mcp-pinchtask`。
    pub async fn new(data_dir: Option<PathBuf>) -> Result<Self, StoreError> {
        let data_dir = data_dir.unwrap_or_else(default_data_dir);
        std::fs::create_dir_all(&data_dir)?;

        let db_path = data_dir.join("tasks.db");
        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
        let pool = sqlx::SqlitePool::connect(&db_url).await?;

        // 启用外键约束（SQLite 默认关闭）
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await?;

        // 执行建表迁移
        sqlx::query(MIGRATION_SQL)
            .execute(&pool)
            .await?;

        Ok(Self { pool })
    }

    // ------------------------------------------------------------------
    // 公开 API
    // ------------------------------------------------------------------

    /// 创建新任务并持久化到数据库。
    ///
    /// 使用事务同时写入 tasks 表和关联的 checklist_items/notes/resources 表。
    /// 自动生成 UUID v4、ISO 8601 时间戳。
    pub async fn create_task(
        &self,
        task_description: &str,
        context_for_all_tasks: Option<&str>,
        initial_checklist: Vec<ChecklistItem>,
        notes: Vec<String>,
        resources: Vec<Resource>,
        metadata: Option<TaskMetadata>,
    ) -> Result<Task, StoreError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        let metadata_json = metadata
            .as_ref()
            .map(|m| serde_json::to_string(m).unwrap_or_default());

        let mut tx = self.pool.begin().await?;

        // 插入主任务行
        sqlx::query(
            "INSERT INTO tasks (id, task_description, context_for_all_tasks, metadata, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(task_description)
        .bind(context_for_all_tasks)
        .bind(&metadata_json)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

        // 插入清单条目
        for (i, item) in initial_checklist.iter().enumerate() {
            sqlx::query(
                "INSERT INTO checklist_items (id, task_id, sort_order, task, detailed_description, context_and_plan, done)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&item.id)
            .bind(&id)
            .bind(i as i64)
            .bind(&item.task)
            .bind(&item.detailed_description)
            .bind(&item.context_and_plan)
            .bind(item.done)
            .execute(&mut *tx)
            .await?;
        }

        // 插入笔记
        for (i, note) in notes.iter().enumerate() {
            sqlx::query("INSERT INTO notes (task_id, sort_order, content) VALUES (?, ?, ?)")
                .bind(&id)
                .bind(i as i64)
                .bind(note)
                .execute(&mut *tx)
                .await?;
        }

        // 插入资源
        for res in &resources {
            sqlx::query(
                "INSERT INTO resources (task_id, name, url, description) VALUES (?, ?, ?, ?)",
            )
            .bind(&id)
            .bind(&res.name)
            .bind(&res.url)
            .bind(&res.description)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        Ok(Task {
            id,
            task_description: task_description.to_owned(),
            context_for_all_tasks: context_for_all_tasks.map(|s| s.to_owned()),
            checklist: initial_checklist,
            notes,
            resources,
            metadata,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    /// 根据 ID 获取单个任务（含完整清单、笔记、资源）。
    pub async fn get_task(&self, id: &str) -> Result<Task, StoreError> {
        let row = sqlx::query_as::<_, TaskRow>(
            "SELECT id, task_description, context_for_all_tasks, metadata, created_at, updated_at
             FROM tasks WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| StoreError::NotFound(id.to_owned()))?;

        let checklist = self.load_checklist_items(id).await?;
        let notes = self.load_notes(id).await?;
        let resources = self.load_resources(id).await?;
        let metadata = row
            .metadata
            .and_then(|s| serde_json::from_str(&s).ok());

        Ok(Task {
            id: row.id,
            task_description: row.task_description,
            context_for_all_tasks: row.context_for_all_tasks,
            checklist,
            notes,
            resources,
            metadata,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }

    /// 更新已有任务（整体覆盖写入）。
    ///
    /// 使用事务：更新主表 + 删除并重建关联的 checklist_items/notes/resources 行。
    /// 自动刷新 `updated_at` 时间戳。
    pub async fn update_task(&self, task: &mut Task) -> Result<(), StoreError> {
        // 确认任务存在
        let exists = sqlx::query("SELECT 1 FROM tasks WHERE id = ?")
            .bind(&task.id)
            .fetch_optional(&self.pool)
            .await?
            .is_some();
        if !exists {
            return Err(StoreError::NotFound(task.id.clone()));
        }

        task.updated_at = Utc::now().to_rfc3339();
        let metadata_json = task
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(m).unwrap_or_default());

        let mut tx = self.pool.begin().await?;

        // 更新主任务行
        sqlx::query(
            "UPDATE tasks
             SET task_description = ?, context_for_all_tasks = ?, metadata = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(&task.task_description)
        .bind(&task.context_for_all_tasks)
        .bind(&metadata_json)
        .bind(&task.updated_at)
        .bind(&task.id)
        .execute(&mut *tx)
        .await?;

        // 删除并重建清单条目
        sqlx::query("DELETE FROM checklist_items WHERE task_id = ?")
            .bind(&task.id)
            .execute(&mut *tx)
            .await?;
        for (i, item) in task.checklist.iter().enumerate() {
            sqlx::query(
                "INSERT INTO checklist_items (id, task_id, sort_order, task, detailed_description, context_and_plan, done)
                 VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&item.id)
            .bind(&task.id)
            .bind(i as i64)
            .bind(&item.task)
            .bind(&item.detailed_description)
            .bind(&item.context_and_plan)
            .bind(item.done)
            .execute(&mut *tx)
            .await?;
        }

        // 删除并重建笔记
        sqlx::query("DELETE FROM notes WHERE task_id = ?")
            .bind(&task.id)
            .execute(&mut *tx)
            .await?;
        for (i, note) in task.notes.iter().enumerate() {
            sqlx::query("INSERT INTO notes (task_id, sort_order, content) VALUES (?, ?, ?)")
                .bind(&task.id)
                .bind(i as i64)
                .bind(note)
                .execute(&mut *tx)
                .await?;
        }

        // 删除并重建资源
        sqlx::query("DELETE FROM resources WHERE task_id = ?")
            .bind(&task.id)
            .execute(&mut *tx)
            .await?;
        for res in &task.resources {
            sqlx::query(
                "INSERT INTO resources (task_id, name, url, description) VALUES (?, ?, ?, ?)",
            )
            .bind(&task.id)
            .bind(&res.name)
            .bind(&res.url)
            .bind(&res.description)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// 根据 ID 删除任务（关联行通过 CASCADE 自动删除）。
    pub async fn delete_task(&self, id: &str) -> Result<(), StoreError> {
        let result = sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(StoreError::NotFound(id.to_owned()));
        }
        Ok(())
    }

    /// 列出所有已存储的任务。
    ///
    /// 按 `created_at` 升序排列返回，每个任务包含完整清单、笔记和资源。
    pub async fn list_tasks(&self) -> Result<Vec<Task>, StoreError> {
        let rows = sqlx::query_as::<_, TaskRow>(
            "SELECT id, task_description, context_for_all_tasks, metadata, created_at, updated_at
             FROM tasks ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut tasks = Vec::with_capacity(rows.len());
        for row in rows {
            let checklist = self.load_checklist_items(&row.id).await?;
            let notes = self.load_notes(&row.id).await?;
            let resources = self.load_resources(&row.id).await?;
            let metadata = row
                .metadata
                .and_then(|s| serde_json::from_str(&s).ok());
            tasks.push(Task {
                id: row.id,
                task_description: row.task_description,
                context_for_all_tasks: row.context_for_all_tasks,
                checklist,
                notes,
                resources,
                metadata,
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }

        Ok(tasks)
    }

    // ------------------------------------------------------------------
    // 内部辅助方法
    // ------------------------------------------------------------------

    /// 加载指定任务的所有清单条目（按 sort_order 排序）。
    async fn load_checklist_items(&self, task_id: &str) -> Result<Vec<ChecklistItem>, StoreError> {
        let rows = sqlx::query_as::<_, ChecklistItemRow>(
            "SELECT id, task, detailed_description, context_and_plan, done
             FROM checklist_items WHERE task_id = ? ORDER BY sort_order ASC",
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// 加载指定任务的所有笔记（按 sort_order 排序）。
    async fn load_notes(&self, task_id: &str) -> Result<Vec<String>, StoreError> {
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT content FROM notes WHERE task_id = ? ORDER BY sort_order ASC",
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    /// 加载指定任务的所有资源。
    async fn load_resources(&self, task_id: &str) -> Result<Vec<Resource>, StoreError> {
        let rows = sqlx::query_as::<_, ResourceRow>(
            "SELECT name, url, description FROM resources WHERE task_id = ?",
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}

// ---------------------------------------------------------------------------
// 内部行映射结构体（仅用于 sqlx::FromRow 反序列化）
// ---------------------------------------------------------------------------

/// tasks 表行映射。
#[derive(sqlx::FromRow)]
struct TaskRow {
    id: String,
    task_description: String,
    context_for_all_tasks: Option<String>,
    metadata: Option<String>,
    created_at: String,
    updated_at: String,
}

/// checklist_items 表行映射。
#[derive(sqlx::FromRow)]
struct ChecklistItemRow {
    id: String,
    task: String,
    detailed_description: String,
    context_and_plan: Option<String>,
    done: bool,
}

impl From<ChecklistItemRow> for ChecklistItem {
    fn from(row: ChecklistItemRow) -> Self {
        ChecklistItem {
            id: row.id,
            task: row.task,
            detailed_description: row.detailed_description,
            context_and_plan: row.context_and_plan,
            done: row.done,
        }
    }
}

/// resources 表行映射。
#[derive(sqlx::FromRow)]
struct ResourceRow {
    name: String,
    url: String,
    description: Option<String>,
}

impl From<ResourceRow> for Resource {
    fn from(row: ResourceRow) -> Self {
        Resource {
            name: row.name,
            url: row.url,
            description: row.description,
        }
    }
}

// ---------------------------------------------------------------------------
// 路径辅助
// ---------------------------------------------------------------------------

/// 返回默认数据目录 `~/.mcp-pinchtask`。
fn default_data_dir() -> PathBuf {
    dirs_home_dir().join(".mcp-pinchtask")
}

/// 获取用户 HOME 目录的跨平台辅助函数。
///
/// 优先使用 `HOME` / `USERPROFILE` 环境变量，兜底回退到当前目录。
fn dirs_home_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(&home);
        if p.is_dir() {
            return p;
        }
    }
    if let Ok(home) = std::env::var("USERPROFILE") {
        let p = PathBuf::from(&home);
        if p.is_dir() {
            return p;
        }
    }
    PathBuf::from(".")
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// 辅助：创建一个使用临时目录的异步 TaskStore。
    async fn temp_store() -> (TaskStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let store =
            TaskStore::new(Some(dir.path().to_path_buf())).await.expect("创建 TaskStore 失败");
        (store, dir)
    }

    #[tokio::test]
    async fn create_and_get_task() {
        let (store, _dir) = temp_store().await;

        let item = ChecklistItem {
            id: Uuid::new_v4().to_string(),
            task: "子任务1".to_owned(),
            detailed_description: "完成数据模型".to_owned(),
            context_and_plan: None,
            done: false,
        };

        let created = store
            .create_task(
                "整体任务描述",
                Some("共享上下文"),
                vec![item],
                vec!["笔记1".to_owned()],
                vec![],
                None,
            )
            .await
            .expect("创建任务失败");

        // 从数据库重新读取
        let loaded = store.get_task(&created.id).await.expect("获取任务失败");
        assert_eq!(loaded.task_description, "整体任务描述");
        assert_eq!(
            loaded.context_for_all_tasks,
            Some("共享上下文".to_owned())
        );
        assert_eq!(loaded.checklist.len(), 1);
        assert_eq!(loaded.notes, vec!["笔记1"]);
        assert!(
            loaded.created_at.ends_with("+00:00") || loaded.created_at.len() > 10
        );
    }

    #[tokio::test]
    async fn update_task_refreshes_updated_at() {
        let (store, _dir) = temp_store().await;

        let mut task = store
            .create_task("t1", None, vec![], vec![], vec![], None)
            .await
            .expect("创建任务失败");
        let original_updated = task.updated_at.clone();

        // 稍作等待以确保时间戳不同
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        task.notes.push("新增笔记".to_owned());
        store.update_task(&mut task).await.expect("更新任务失败");

        assert_ne!(task.updated_at, original_updated);
    }

    #[tokio::test]
    async fn delete_task_removes_row() {
        let (store, _dir) = temp_store().await;

        let task = store
            .create_task("t2", None, vec![], vec![], vec![], None)
            .await
            .expect("创建任务失败");

        store.delete_task(&task.id).await.expect("删除任务失败");
        assert!(store.get_task(&task.id).await.is_err());
    }

    #[tokio::test]
    async fn list_tasks_returns_all() {
        let (store, _dir) = temp_store().await;

        store
            .create_task("t-a", None, vec![], vec![], vec![], None)
            .await
            .expect("创建任务失败");
        store
            .create_task("t-b", None, vec![], vec![], vec![], None)
            .await
            .expect("创建任务失败");

        let tasks = store.list_tasks().await.expect("列出任务失败");
        assert_eq!(tasks.len(), 2);
    }

    #[tokio::test]
    async fn get_nonexistent_task_returns_not_found() {
        let (store, _dir) = temp_store().await;
        let result = store.get_task("nonexistent-id").await;
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }

    #[tokio::test]
    async fn delete_nonexistent_task_returns_not_found() {
        let (store, _dir) = temp_store().await;
        let result = store.delete_task("nonexistent-id").await;
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }

    #[tokio::test]
    async fn update_nonexistent_task_returns_not_found() {
        let (store, _dir) = temp_store().await;
        let mut task = Task {
            id: "nonexistent-id".to_owned(),
            task_description: "desc".to_owned(),
            context_for_all_tasks: None,
            checklist: vec![],
            notes: vec![],
            resources: vec![],
            metadata: None,
            created_at: "2024-01-01T00:00:00+00:00".to_owned(),
            updated_at: "2024-01-01T00:00:00+00:00".to_owned(),
        };
        let result = store.update_task(&mut task).await;
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }

    #[tokio::test]
    async fn round_trip_preserves_data() {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let store1 =
            TaskStore::new(Some(dir.path().to_path_buf())).await.expect("创建 TaskStore 失败");

        let item = ChecklistItem {
            id: Uuid::new_v4().to_string(),
            task: "子任务".to_owned(),
            detailed_description: "详情".to_owned(),
            context_and_plan: Some("计划".to_owned()),
            done: true,
        };
        let meta = TaskMetadata {
            tags: Some(vec!["rust".to_owned(), "test".to_owned()]),
            priority: Some("high".to_owned()),
            estimated_completion_time: Some("2024-06-01T00:00:00+00:00".to_owned()),
        };
        let res = Resource {
            name: "docs".to_owned(),
            url: "https://example.com".to_owned(),
            description: Some("文档".to_owned()),
        };

        let created = store1
            .create_task(
                "往返测试任务",
                Some("共享上下文"),
                vec![item],
                vec!["笔记A".to_owned(), "笔记B".to_owned()],
                vec![res],
                Some(meta),
            )
            .await
            .expect("创建任务失败");

        // 用全新的 TaskStore 实例从同一数据库加载
        let store2 =
            TaskStore::new(Some(dir.path().to_path_buf())).await.expect("创建 TaskStore 失败");
        let loaded = store2
            .get_task(&created.id)
            .await
            .expect("获取任务失败");

        assert_eq!(loaded.id, created.id);
        assert_eq!(loaded.task_description, "往返测试任务");
        assert_eq!(
            loaded.context_for_all_tasks,
            Some("共享上下文".to_owned())
        );
        assert_eq!(loaded.checklist.len(), 1);
        assert_eq!(loaded.checklist[0].task, "子任务");
        assert_eq!(loaded.checklist[0].done, true);
        assert_eq!(loaded.notes, vec!["笔记A", "笔记B"]);
        assert_eq!(loaded.resources.len(), 1);
        assert_eq!(loaded.resources[0].name, "docs");
        assert!(loaded.metadata.is_some());
        let m = loaded.metadata.unwrap();
        assert_eq!(
            m.tags,
            Some(vec!["rust".to_owned(), "test".to_owned()])
        );
        assert_eq!(m.priority, Some("high".to_owned()));
    }

    #[tokio::test]
    async fn list_tasks_sorted_by_created_at() {
        let (store, _dir) = temp_store().await;

        let t1 = store
            .create_task("first", None, vec![], vec![], vec![], None)
            .await
            .expect("创建任务失败");
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let t2 = store
            .create_task("second", None, vec![], vec![], vec![], None)
            .await
            .expect("创建任务失败");
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let t3 = store
            .create_task("third", None, vec![], vec![], vec![], None)
            .await
            .expect("创建任务失败");

        let tasks = store.list_tasks().await.expect("列出任务失败");
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0].id, t1.id);
        assert_eq!(tasks[1].id, t2.id);
        assert_eq!(tasks[2].id, t3.id);
    }

    #[tokio::test]
    async fn cascade_delete_removes_associated_rows() {
        let (store, _dir) = temp_store().await;

        let item = ChecklistItem {
            id: Uuid::new_v4().to_string(),
            task: "将被级联删除的条目".to_owned(),
            detailed_description: "详情".to_owned(),
            context_and_plan: None,
            done: false,
        };
        let res = Resource {
            name: "将被删除的资源".to_owned(),
            url: "https://example.com".to_owned(),
            description: None,
        };

        let task = store
            .create_task(
                "级联删除测试",
                None,
                vec![item],
                vec!["将被删除的笔记".to_owned()],
                vec![res],
                None,
            )
            .await
            .expect("创建任务失败");

        // 删除任务后，关联的 checklist_items/notes/resources 应被 CASCADE 删除
        store.delete_task(&task.id).await.expect("删除任务失败");

        // 直接查询关联表确认数据已清除
        let checklist: Vec<(String,)> =
            sqlx::query_as("SELECT id FROM checklist_items WHERE task_id = ?")
                .bind(&task.id)
                .fetch_all(&store.pool)
                .await
                .expect("查询 checklist_items 失败");
        assert!(checklist.is_empty());

        let notes: Vec<(String,)> =
            sqlx::query_as("SELECT content FROM notes WHERE task_id = ?")
                .bind(&task.id)
                .fetch_all(&store.pool)
                .await
                .expect("查询 notes 失败");
        assert!(notes.is_empty());

        let resources: Vec<(String,)> =
            sqlx::query_as("SELECT name FROM resources WHERE task_id = ?")
                .bind(&task.id)
                .fetch_all(&store.pool)
                .await
                .expect("查询 resources 失败");
        assert!(resources.is_empty());
    }
}
