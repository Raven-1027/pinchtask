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
// 任务状态推导、优先级排序、智能格式化（MCP 层专用）
// ---------------------------------------------------------------------------

/// 任务状态，根据清单完成情况推导。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    /// 清单非空且全部完成。
    Completed,
    /// 清单非空且部分完成。
    InProgress,
    /// 清单为空，或清单非空但全部未完成。
    NotStarted,
}

/// 根据任务清单推导状态。
pub fn derive_task_status(task: &Task) -> TaskStatus {
    if task.checklist.is_empty() {
        return TaskStatus::NotStarted;
    }
    let done_count = task.checklist.iter().filter(|i| i.done).count();
    if done_count == task.checklist.len() {
        TaskStatus::Completed
    } else if done_count > 0 {
        TaskStatus::InProgress
    } else {
        TaskStatus::NotStarted
    }
}

/// 将优先级映射为排序权重，越小越靠前。
pub fn priority_rank(task: &Task) -> u8 {
    match task.metadata.as_ref().and_then(|m| m.priority.as_deref()) {
        Some("high") => 0,
        Some("medium") => 1,
        Some("low") => 2,
        _ => 3,
    }
}

/// format_task_list_smart 的选项。
#[derive(Debug, Clone, Default)]
pub struct ListTasksOptions {
    /// 按状态过滤，None 表示显示全部。
    pub status_filter: Option<TaskStatus>,
    /// 是否跳过截断，显示全部任务。
    pub include_all: bool,
}

/// 智能格式化任务列表：按状态分组、按优先级排序、超量截断。
pub fn format_task_list_smart(tasks: &[Task], options: &ListTasksOptions) -> String {
    if tasks.is_empty() {
        return "当前没有任何任务".to_owned();
    }

    let mut in_progress: Vec<&Task> = Vec::new();
    let mut not_started: Vec<&Task> = Vec::new();
    let mut completed: Vec<&Task> = Vec::new();

    for task in tasks {
        let status = derive_task_status(task);
        // 如果设置了状态过滤，只保留匹配的任务
        if let Some(ref filter) = options.status_filter
            && status != *filter
        {
            continue;
        }
        match status {
            TaskStatus::InProgress => in_progress.push(task),
            TaskStatus::NotStarted => not_started.push(task),
            TaskStatus::Completed => completed.push(task),
        }
    }

    // 每组内按 priority_rank 升序，同优先级按 created_at 升序
    let sort_key = |t: &&Task| (priority_rank(t), t.created_at.clone());
    in_progress.sort_by_key(sort_key);
    not_started.sort_by_key(sort_key);
    completed.sort_by_key(sort_key);

    let total = in_progress.len() + not_started.len() + completed.len();
    let need_truncate = !options.include_all && total > 10;

    let mut output = String::new();

    // --- 进行中 ---
    if !in_progress.is_empty() {
        output.push_str(&format!("## 进行中 ({})\n\n", in_progress.len()));
        for task in &in_progress {
            output.push_str(&format_task_line(task));
        }
        output.push('\n');
    }

    // --- 未开始 ---
    if !not_started.is_empty() {
        let show_count = if need_truncate && not_started.len() > 3 {
            3
        } else {
            not_started.len()
        };
        output.push_str(&format!("## 未开始 ({})\n\n", not_started.len()));
        for task in not_started.iter().take(show_count) {
            output.push_str(&format_task_line(task));
        }
        if show_count < not_started.len() {
            let remaining = not_started.len() - show_count;
            output.push_str(&format!("... 还有 {} 个未开始的任务未展示\n", remaining));
        }
        output.push('\n');
    }

    // --- 已完成 ---
    if !completed.is_empty() {
        let show_count = if need_truncate && completed.len() > 3 {
            3
        } else {
            completed.len()
        };
        output.push_str(&format!("## 已完成 ({})\n\n", completed.len()));
        for task in completed.iter().take(show_count) {
            output.push_str(&format_task_line_completed(task));
        }
        if show_count < completed.len() {
            let remaining = completed.len() - show_count;
            output.push_str(&format!("... 还有 {} 个已完成的任务未展示\n", remaining));
        }
    }

    // 去掉末尾多余的空行
    while output.ends_with("\n\n") {
        output.pop();
    }
    output
}

/// 格式化单个任务条目（多行格式）。
fn format_task_entry(task: &Task, icon: &str) -> String {
    let short_id = &task.id[..8.min(task.id.len())];
    let done = task.checklist.iter().filter(|i| i.done).count();
    let total = task.checklist.len();
    let meta_line = match task.metadata.as_ref().and_then(|m| m.priority.as_deref()) {
        Some(p) => format!("   进度: {}/{} · 优先级: {}\n", done, total, p),
        None => format!("   进度: {}/{}\n", done, total),
    };
    format!(
        "{} [{}] {}\n{}",
        icon, short_id, task.task_description, meta_line
    )
}

/// 格式化任务条目（进行中 / 未开始）。
fn format_task_line(task: &Task) -> String {
    format_task_entry(task, priority_icon(task))
}

/// 格式化任务条目（已完成）。
fn format_task_line_completed(task: &Task) -> String {
    format_task_entry(task, "✅")
}

/// 优先级图标。
fn priority_icon(task: &Task) -> &'static str {
    match task.metadata.as_ref().and_then(|m| m.priority.as_deref()) {
        Some("high") => "🔴",
        Some("medium") => "🟡",
        _ => "⚪",
    }
}

/// 解析状态筛选字符串。
pub fn parse_status_filter(s: &str) -> Option<TaskStatus> {
    match s {
        "in_progress" => Some(TaskStatus::InProgress),
        "not_started" => Some(TaskStatus::NotStarted),
        "completed" => Some(TaskStatus::Completed),
        _ => None,
    }
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

    // ------------------------------------------------------------------
    // derive_task_status / priority_rank / format_task_list_smart 纯函数测试
    // ------------------------------------------------------------------

    /// 辅助：快速构造 ChecklistItem。
    fn make_checklist_item(task: &str, done: bool) -> ChecklistItem {
        ChecklistItem {
            id: uuid::Uuid::new_v4().to_string(),
            task: task.to_owned(),
            detailed_description: String::new(),
            context_and_plan: None,
            done,
        }
    }

    /// 辅助：快速构造 Task（不经过 store）。
    fn make_task(desc: &str, checklist: Vec<ChecklistItem>, priority: Option<&str>) -> Task {
        Task {
            id: uuid::Uuid::new_v4().to_string(),
            task_description: desc.to_owned(),
            context_for_all_tasks: None,
            checklist,
            notes: vec![],
            resources: vec![],
            metadata: priority.map(|p| TaskMetadata {
                tags: None,
                priority: Some(p.to_owned()),
                estimated_completion_time: None,
            }),
            project_id: None,
            created_at: "2025-01-01T00:00:00Z".to_owned(),
            updated_at: "2025-01-01T00:00:00Z".to_owned(),
        }
    }

    // ------------------------------------------------------------------
    // derive_task_status
    // ------------------------------------------------------------------

    #[test]
    fn derive_task_status_empty_checklist() {
        let task = make_task("空清单", vec![], None);
        assert!(matches!(derive_task_status(&task), TaskStatus::NotStarted));
    }

    #[test]
    fn derive_task_status_all_done() {
        let task = make_task(
            "全部完成",
            vec![
                make_checklist_item("项1", true),
                make_checklist_item("项2", true),
            ],
            None,
        );
        assert!(matches!(derive_task_status(&task), TaskStatus::Completed));
    }

    #[test]
    fn derive_task_status_partial_done() {
        let task = make_task(
            "部分完成",
            vec![
                make_checklist_item("项1", true),
                make_checklist_item("项2", false),
            ],
            None,
        );
        assert!(matches!(derive_task_status(&task), TaskStatus::InProgress));
    }

    #[test]
    fn derive_task_status_none_done() {
        let task = make_task(
            "全部未完成",
            vec![
                make_checklist_item("项1", false),
                make_checklist_item("项2", false),
            ],
            None,
        );
        assert!(matches!(derive_task_status(&task), TaskStatus::NotStarted));
    }

    // ------------------------------------------------------------------
    // priority_rank
    // ------------------------------------------------------------------

    #[test]
    fn priority_rank_high() {
        let task = make_task("高优先级", vec![], Some("high"));
        assert_eq!(priority_rank(&task), 0);
    }

    #[test]
    fn priority_rank_medium() {
        let task = make_task("中优先级", vec![], Some("medium"));
        assert_eq!(priority_rank(&task), 1);
    }

    #[test]
    fn priority_rank_low() {
        let task = make_task("低优先级", vec![], Some("low"));
        assert_eq!(priority_rank(&task), 2);
    }

    #[test]
    fn priority_rank_none() {
        let task = make_task("无优先级", vec![], None);
        assert_eq!(priority_rank(&task), 3);
    }

    #[test]
    fn priority_rank_unknown() {
        let task = make_task("未知优先级", vec![], Some("urgent"));
        assert_eq!(priority_rank(&task), 3);
    }

    // ------------------------------------------------------------------
    // format_task_list_smart
    // ------------------------------------------------------------------

    #[test]
    fn format_task_list_smart_empty() {
        let output = format_task_list_smart(&[], &super::ListTasksOptions::default());
        assert_eq!(output, "当前没有任何任务");
    }

    #[test]
    fn format_task_list_smart_full_display() {
        let tasks = vec![
            make_task(
                "进行中任务",
                vec![
                    make_checklist_item("项1", true),
                    make_checklist_item("项2", false),
                ],
                None,
            ),
            make_task("未开始任务", vec![], None),
            make_task("已完成任务", vec![make_checklist_item("项1", true)], None),
        ];
        let output = format_task_list_smart(&tasks, &super::ListTasksOptions::default());
        assert!(output.contains("## 进行中 (1)"), "应包含进行中分组标题");
        assert!(output.contains("## 未开始 (1)"), "应包含未开始分组标题");
        assert!(output.contains("## 已完成 (1)"), "应包含已完成分组标题");
        assert!(output.contains("进行中任务"), "应包含任务描述");
        assert!(!output.contains("还有"), "≤10 个任务不应截断");
    }

    #[test]
    fn format_task_list_smart_truncated() {
        // 12 个任务：4 进行中 + 5 未开始 + 3 已完成 → 总数 > 10 触发截断
        let mut tasks = Vec::new();
        for i in 0..4 {
            tasks.push(make_task(
                &format!("进行中{}", i),
                vec![
                    make_checklist_item("项", true),
                    make_checklist_item("项", false),
                ],
                None,
            ));
        }
        for i in 0..5 {
            tasks.push(make_task(&format!("未开始{}", i), vec![], None));
        }
        for i in 0..3 {
            tasks.push(make_task(
                &format!("已完成{}", i),
                vec![make_checklist_item("项", true)],
                None,
            ));
        }
        let output = format_task_list_smart(&tasks, &super::ListTasksOptions::default());
        assert!(
            output.contains("还有 2 个未开始的任务未展示"),
            "未开始组应截断并提示剩余数量"
        );
    }

    #[test]
    fn format_task_list_smart_priority_sort() {
        // 两个未开始任务：低优先级创建时间更早，高优先级创建时间更晚。
        // 高优先级应排在前面（priority_rank 权重优先于 created_at）。
        let low_task = Task {
            id: "aaaaaaaa-0000-0000-0000-000000000000".to_owned(),
            task_description: "低优先级任务".to_owned(),
            context_for_all_tasks: None,
            checklist: vec![],
            notes: vec![],
            resources: vec![],
            metadata: Some(TaskMetadata {
                tags: None,
                priority: Some("low".to_owned()),
                estimated_completion_time: None,
            }),
            project_id: None,
            created_at: "2025-01-01T00:00:00Z".to_owned(),
            updated_at: "2025-01-01T00:00:00Z".to_owned(),
        };
        let high_task = Task {
            id: "bbbbbbbb-0000-0000-0000-000000000000".to_owned(),
            task_description: "高优先级任务".to_owned(),
            context_for_all_tasks: None,
            checklist: vec![],
            notes: vec![],
            resources: vec![],
            metadata: Some(TaskMetadata {
                tags: None,
                priority: Some("high".to_owned()),
                estimated_completion_time: None,
            }),
            project_id: None,
            created_at: "2025-01-02T00:00:00Z".to_owned(),
            updated_at: "2025-01-02T00:00:00Z".to_owned(),
        };
        let output =
            format_task_list_smart(&[low_task, high_task], &super::ListTasksOptions::default());
        let low_pos = output.find("低优先级任务").expect("应包含低优先级任务");
        let high_pos = output.find("高优先级任务").expect("应包含高优先级任务");
        assert!(high_pos < low_pos, "高优先级任务应排在低优先级任务之前");
    }

    // ------------------------------------------------------------------
    // parse_status_filter
    // ------------------------------------------------------------------

    #[test]
    fn parse_status_filter_valid() {
        assert!(matches!(
            super::parse_status_filter("in_progress"),
            Some(super::TaskStatus::InProgress)
        ));
        assert!(matches!(
            super::parse_status_filter("not_started"),
            Some(super::TaskStatus::NotStarted)
        ));
        assert!(matches!(
            super::parse_status_filter("completed"),
            Some(super::TaskStatus::Completed)
        ));
    }

    #[test]
    fn parse_status_filter_invalid() {
        assert!(super::parse_status_filter("invalid").is_none());
        assert!(super::parse_status_filter("").is_none());
        assert!(super::parse_status_filter("InProgress").is_none()); // 大小写敏感
    }

    // ------------------------------------------------------------------
    // format_task_list_smart with options
    // ------------------------------------------------------------------

    #[test]
    fn format_task_list_smart_status_filter_in_progress() {
        let tasks = vec![
            make_task(
                "进行中",
                vec![
                    make_checklist_item("项", true),
                    make_checklist_item("项", false),
                ],
                None,
            ),
            make_task("未开始", vec![], None),
            make_task("已完成", vec![make_checklist_item("项", true)], None),
        ];
        let options = super::ListTasksOptions {
            status_filter: Some(super::TaskStatus::InProgress),
            include_all: false,
        };
        let output = format_task_list_smart(&tasks, &options);
        assert!(output.contains("## 进行中 (1)"), "应包含进行中分组");
        assert!(!output.contains("## 未开始"), "不应包含未开始分组");
        assert!(!output.contains("## 已完成"), "不应包含已完成分组");
        assert!(output.contains("进行中"));
    }

    #[test]
    fn format_task_list_smart_include_all_no_truncate() {
        // 超过 10 个任务但 include_all=true 时不截断
        let mut tasks = Vec::new();
        for i in 0..5 {
            tasks.push(make_task(
                &format!("进行中{}", i),
                vec![
                    make_checklist_item("项", true),
                    make_checklist_item("项", false),
                ],
                None,
            ));
        }
        for i in 0..5 {
            tasks.push(make_task(&format!("未开始{}", i), vec![], None));
        }
        for i in 0..3 {
            tasks.push(make_task(
                &format!("已完成{}", i),
                vec![make_checklist_item("项", true)],
                None,
            ));
        }
        let options = super::ListTasksOptions {
            status_filter: None,
            include_all: true,
        };
        let output = format_task_list_smart(&tasks, &options);
        assert!(!output.contains("还有"), "include_all=true 时不应截断");
        assert!(
            output.contains("## 未开始 (5)"),
            "应展示全部 5 个未开始任务"
        );
    }

    #[test]
    fn format_task_list_smart_invalid_status_filter() {
        // 无效的 status_filter 值应被忽略（parse_status_filter 返回 None → 等同于不过滤）
        let tasks = vec![make_task(
            "进行中",
            vec![
                make_checklist_item("项", true),
                make_checklist_item("项", false),
            ],
            None,
        )];
        let options = super::ListTasksOptions {
            status_filter: None, // parse_status_filter("invalid") 返回 None
            include_all: false,
        };
        let output = format_task_list_smart(&tasks, &options);
        assert!(output.contains("进行中"), "无效过滤应等同不过滤");
    }
}
