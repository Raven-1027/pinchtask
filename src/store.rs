//! 文件持久化层。
//!
//! `TaskStore` 负责将每个任务以独立 JSON 文件的形式存储在可配置的数据目录中，
//! 默认路径为 `~/.mcp-pinchtask`。

use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use thiserror::Error;
use uuid::Uuid;

use crate::models::task::{ChecklistItem, Resource, Task, TaskMetadata};

/// 持久化层自定义错误类型。
#[derive(Debug, Error)]
pub enum StoreError {
    /// IO 错误（文件读写、目录创建等）。
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    /// JSON 序列化/反序列化错误。
    #[error("JSON 解析错误: {0}")]
    Json(#[from] serde_json::Error),

    /// 指定任务不存在。
    #[error("任务不存在: {0}")]
    NotFound(String),
}

/// 任务文件持久化存储。
pub struct TaskStore {
    /// 数据目录的绝对路径。
    data_dir: PathBuf,
}

impl TaskStore {
    /// 创建 TaskStore 实例。
    ///
    /// 若数据目录不存在会自动递归创建。
    ///
    /// # 参数
    /// - `data_dir`: 数据目录路径，传入 `None` 则使用默认路径 `~/.mcp-pinchtask`。
    pub fn new(data_dir: Option<PathBuf>) -> Result<Self, StoreError> {
        let data_dir = data_dir.unwrap_or_else(default_data_dir);
        fs::create_dir_all(&data_dir)?;
        Ok(Self { data_dir })
    }

    // ------------------------------------------------------------------
    // 公开 API
    // ------------------------------------------------------------------

    /// 创建新任务并持久化到磁盘。
    ///
    /// 自动生成 UUID v4、ISO 8601 时间戳。
    pub fn create_task(
        &self,
        task_description: &str,
        context_for_all_tasks: Option<&str>,
        initial_checklist: Vec<ChecklistItem>,
        notes: Vec<String>,
        resources: Vec<Resource>,
        metadata: Option<TaskMetadata>,
    ) -> Result<Task, StoreError> {
        let now = Utc::now().to_rfc3339();
        let task = Task {
            id: Uuid::new_v4().to_string(),
            task_description: task_description.to_owned(),
            context_for_all_tasks: context_for_all_tasks.map(|s| s.to_owned()),
            checklist: initial_checklist,
            notes,
            resources,
            metadata,
            created_at: now.clone(),
            updated_at: now,
        };
        self.save_task(&task)?;
        Ok(task)
    }

    /// 根据 ID 获取单个任务。
    pub fn get_task(&self, id: &str) -> Result<Task, StoreError> {
        let path = self.task_file_path(id);
        if !path.exists() {
            return Err(StoreError::NotFound(id.to_owned()));
        }
        let data = fs::read_to_string(&path)?;
        let task: Task = serde_json::from_str(&data)?;
        Ok(task)
    }

    /// 更新已有任务（整体覆盖写入）。
    ///
    /// 自动刷新 `updated_at` 时间戳。
    pub fn update_task(&self, task: &mut Task) -> Result<(), StoreError> {
        // 确认任务文件存在
        let path = self.task_file_path(&task.id);
        if !path.exists() {
            return Err(StoreError::NotFound(task.id.clone()));
        }
        task.updated_at = Utc::now().to_rfc3339();
        self.save_task(task)?;
        Ok(())
    }

    /// 根据 ID 删除任务。
    pub fn delete_task(&self, id: &str) -> Result<(), StoreError> {
        let path = self.task_file_path(id);
        if !path.exists() {
            return Err(StoreError::NotFound(id.to_owned()));
        }
        fs::remove_file(&path)?;
        Ok(())
    }

    /// 列出所有已存储的任务。
    ///
    /// 扫描数据目录中所有 `.json` 文件并按创建时间升序排列返回。
    pub fn list_tasks(&self) -> Result<Vec<Task>, StoreError> {
        let mut tasks = Vec::new();
        let entries = fs::read_dir(&self.data_dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let data = fs::read_to_string(&path)?;
                match serde_json::from_str::<Task>(&data) {
                    Ok(task) => tasks.push(task),
                    Err(e) => {
                        // 跳过无法解析的文件，记录警告日志
                        tracing::warn!("跳过无法解析的文件 {}: {e}", path.display());
                    }
                }
            }
        }

        // 按创建时间升序排列
        tasks.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(tasks)
    }

    // ------------------------------------------------------------------
    // 内部辅助方法
    // ------------------------------------------------------------------

    /// 获取任务对应的 JSON 文件路径。
    fn task_file_path(&self, id: &str) -> PathBuf {
        self.data_dir.join(format!("{id}.json"))
    }

    /// 将任务序列化为 JSON 并写入磁盘（美化格式，便于调试查看）。
    fn save_task(&self, task: &Task) -> Result<(), StoreError> {
        let path = self.task_file_path(&task.id);
        let json = serde_json::to_string_pretty(task)?;
        fs::write(&path, json)?;
        Ok(())
    }
}

/// 返回默认数据目录 `~/.mcp-pinchtask`。
fn default_data_dir() -> PathBuf {
    let home = dirs_home_dir();
    home.join(".mcp-pinchtask")
}

/// 获取用户 HOME 目录的跨平台辅助函数。
///
/// 优先使用 `HOME` / `USERPROFILE` 环境变量，兜底回退到当前目录。
fn dirs_home_dir() -> PathBuf {
    // 尝试常见环境变量
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
    // 兜底：当前工作目录
    PathBuf::from(".")
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::task::ChecklistItem;

    /// 辅助：创建一个使用临时目录的 TaskStore。
    fn temp_store() -> (TaskStore, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("创建临时目录失败");
        let store = TaskStore::new(Some(dir.path().to_path_buf())).expect("创建 TaskStore 失败");
        (store, dir)
    }

    #[test]
    fn create_and_get_task() {
        let (store, _dir) = temp_store();

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
            .expect("创建任务失败");

        // 从磁盘重新读取
        let loaded = store.get_task(&created.id).expect("获取任务失败");
        assert_eq!(loaded.task_description, "整体任务描述");
        assert_eq!(loaded.context_for_all_tasks, Some("共享上下文".to_owned()));
        assert_eq!(loaded.checklist.len(), 1);
        assert_eq!(loaded.notes, vec!["笔记1"]);
        assert!(loaded.created_at.ends_with("+00:00") || loaded.created_at.len() > 10);
    }

    #[test]
    fn update_task_refreshes_updated_at() {
        let (store, _dir) = temp_store();

        let mut task = store
            .create_task("t1", None, vec![], vec![], vec![], None)
            .expect("创建任务失败");
        let original_updated = task.updated_at.clone();

        // 稍作等待以确保时间戳不同
        std::thread::sleep(std::time::Duration::from_millis(10));

        task.notes.push("新增笔记".to_owned());
        store.update_task(&mut task).expect("更新任务失败");

        assert_ne!(task.updated_at, original_updated);
    }

    #[test]
    fn delete_task_removes_file() {
        let (store, _dir) = temp_store();

        let task = store
            .create_task("t2", None, vec![], vec![], vec![], None)
            .expect("创建任务失败");

        store.delete_task(&task.id).expect("删除任务失败");
        assert!(store.get_task(&task.id).is_err());
    }

    #[test]
    fn list_tasks_returns_all() {
        let (store, _dir) = temp_store();

        store
            .create_task("t-a", None, vec![], vec![], vec![], None)
            .expect("创建任务失败");
        store
            .create_task("t-b", None, vec![], vec![], vec![], None)
            .expect("创建任务失败");

        let tasks = store.list_tasks().expect("列出任务失败");
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn get_nonexistent_task_returns_not_found() {
        let (store, _dir) = temp_store();
        let result = store.get_task("nonexistent-id");
        assert!(matches!(result, Err(StoreError::NotFound(_))));
    }
}
