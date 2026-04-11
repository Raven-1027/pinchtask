//! 短 ID 前缀匹配（core 层共享）。
//!
//! CLI 和 MCP handler 均通过此模块解析短 ID 前缀。
//! 纯函数设计：接收已查询的切片，不依赖 store 或 async。

use crate::models::project::Project;
use crate::models::task::Task;
use crate::store::{StoreError, TaskStore};

/// 最少前缀长度。
const MIN_PREFIX_LEN: usize = 4;

/// 根据前缀解析任务 ID。
///
/// - 前缀长度不足 4 位时返回 `StoreError::InvalidIdPrefix`。
/// - 唯一匹配时返回完整 UUID。
/// - 0 匹配返回 `StoreError::NotFound`。
/// - 多匹配时返回 `StoreError::AmbiguousTaskId`，列出前 `max_candidates` 个候选。
pub fn resolve_task_id(
    prefix: &str,
    tasks: &[Task],
    max_candidates: usize,
) -> Result<String, StoreError> {
    if prefix.len() < MIN_PREFIX_LEN {
        return Err(StoreError::InvalidIdPrefix {
            prefix: prefix.to_owned(),
            min_len: MIN_PREFIX_LEN,
            actual_len: prefix.len(),
        });
    }

    let matches: Vec<_> = tasks.iter().filter(|t| t.id.starts_with(prefix)).collect();

    match matches.len() {
        0 => Err(StoreError::NotFound(format!("未找到匹配的任务: {prefix}"))),
        1 => Ok(matches[0].id.clone()),
        n => {
            let candidates = format_candidates(
                matches,
                max_candidates,
                |t| {
                    let short_id = &t.id[..8.min(t.id.len())];
                    let desc_len = 40.min(t.task_description.len());
                    let desc = &t.task_description[..desc_len];
                    format!("  {short_id}  {desc}")
                },
                n,
            );
            Err(StoreError::AmbiguousTaskId {
                prefix: prefix.to_owned(),
                count: n,
                candidates,
            })
        }
    }
}

/// 根据前缀解析项目 ID。
///
/// - 前缀长度不足 4 位时返回 `StoreError::InvalidIdPrefix`。
/// - 唯一匹配时返回完整 UUID。
/// - 0 匹配返回 `StoreError::ProjectNotFound`。
/// - 多匹配时返回 `StoreError::AmbiguousProjectId`，列出前 `max_candidates` 个候选。
pub fn resolve_project_id(
    prefix: &str,
    projects: &[Project],
    max_candidates: usize,
) -> Result<String, StoreError> {
    if prefix.len() < MIN_PREFIX_LEN {
        return Err(StoreError::InvalidIdPrefix {
            prefix: prefix.to_owned(),
            min_len: MIN_PREFIX_LEN,
            actual_len: prefix.len(),
        });
    }

    let matches: Vec<_> = projects
        .iter()
        .filter(|p| p.id.starts_with(prefix))
        .collect();

    match matches.len() {
        0 => Err(StoreError::ProjectNotFound(format!(
            "未找到匹配的项目: {prefix}"
        ))),
        1 => Ok(matches[0].id.clone()),
        n => {
            let candidates = format_candidates(
                matches,
                max_candidates,
                |p| {
                    let short_id = &p.id[..8.min(p.id.len())];
                    format!("  {short_id}  {}", p.name)
                },
                n,
            );
            Err(StoreError::AmbiguousProjectId {
                prefix: prefix.to_owned(),
                count: n,
                candidates,
            })
        }
    }
}

/// 格式化候选列表，超过 `max_candidates` 时追加省略提示。
fn format_candidates<T>(
    matches: Vec<&T>,
    max_candidates: usize,
    fmt: impl Fn(&T) -> String,
    total: usize,
) -> String {
    let mut lines: Vec<String> = matches
        .iter()
        .take(max_candidates)
        .map(|item| fmt(item))
        .collect();
    if total > max_candidates {
        lines.push(format!(
            "  ... 还有 {} 个匹配项未显示",
            total - max_candidates
        ));
    }
    lines.join("\n")
}

/// 异步版本：根据前缀解析任务 ID。
///
/// 从 store 中查询所有任务，然后委托给同步 `resolve_task_id`。
/// 适用于 MCP 服务器、TUI 等异步上下文。
pub async fn resolve_task_id_async(
    store: &TaskStore,
    prefix: &str,
    max_candidates: usize,
) -> Result<String, StoreError> {
    let tasks = store.list_tasks().await?;
    resolve_task_id(prefix, &tasks, max_candidates)
}

/// 异步版本：根据前缀解析项目 ID。
///
/// 从 store 中查询所有项目，然后委托给同步 `resolve_project_id`。
/// 适用于 MCP 服务器、TUI 等异步上下文。
pub async fn resolve_project_id_async(
    store: &TaskStore,
    prefix: &str,
    max_candidates: usize,
) -> Result<String, StoreError> {
    let projects = store.list_projects().await?;
    resolve_project_id(prefix, &projects, max_candidates)
}
