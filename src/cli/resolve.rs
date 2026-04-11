//! 短 ID 前缀匹配。
//!
//! 允许用户输入 UUID 前 4 位以上的前缀，自动匹配唯一任务。
//! 0 匹配报错"未找到"，多匹配报错并列出候选。
//!
//! 本模块是 `core::resolve` 的薄包装，保持 CLI 同步调用风格。

use crate::models::project::Project;
use crate::models::task::Task;
use anyhow::Result;

/// 默认最大候选数量。
const DEFAULT_MAX_CANDIDATES: usize = 10;

/// 根据前缀匹配任务 ID。
///
/// - 前缀长度不足 4 位时报错提示。
/// - 唯一匹配时返回完整 UUID。
/// - 0 匹配报错"未找到"。
/// - 多匹配报错并列出候选列表。
pub fn resolve_task_id(prefix: &str, tasks: &[Task]) -> Result<String> {
    crate::core::resolve_task_id(prefix, tasks, DEFAULT_MAX_CANDIDATES)
        .map_err(|e| anyhow::anyhow!("{e}"))
}

/// 根据前缀匹配项目 ID。
pub fn resolve_project_id(prefix: &str, projects: &[Project]) -> Result<String> {
    crate::core::resolve_project_id(prefix, projects, DEFAULT_MAX_CANDIDATES)
        .map_err(|e| anyhow::anyhow!("{e}"))
}
