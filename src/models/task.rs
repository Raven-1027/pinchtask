//! 任务数据模型定义。
//!
//! 包含 Task、ChecklistItem、Resource、TaskMetadata 等核心结构体，
//! 参照 mcp-shrimp-task-manager 的数据结构设计。

use serde::{Deserialize, Serialize};

/// 任务清单中的单个条目。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    /// 清单条目唯一标识（UUID）。
    pub id: String,
    /// 简短的任务名称。
    pub task: String,
    /// 详细描述。
    pub detailed_description: String,
    /// 上下文信息与执行计划（可选）。
    pub context_and_plan: Option<String>,
    /// 是否已完成。
    pub done: bool,
}

/// 外部资源引用。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// 资源名称。
    pub name: String,
    /// 资源 URL 或文件路径。
    pub url: String,
    /// 资源描述（可选）。
    pub description: Option<String>,
}

/// 任务元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    /// 标签列表（可选）。
    pub tags: Option<Vec<String>>,
    /// 优先级: high / medium / low（可选）。
    pub priority: Option<String>,
    /// 预计完成时间，ISO 8601 时间戳或时长字符串（可选）。
    pub estimated_completion_time: Option<String>,
}

/// 核心任务结构体。
///
/// 每个任务独立存储为一个 JSON 文件，包含描述、清单、笔记、资源与元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// 任务唯一标识（UUID v4）。
    pub id: String,
    /// 任务整体描述。
    pub task_description: String,
    /// 所有子任务共享的上下文信息（可选）。
    pub context_for_all_tasks: Option<String>,
    /// 有序的清单条目列表。
    pub checklist: Vec<ChecklistItem>,
    /// 自由格式的笔记列表。
    pub notes: Vec<String>,
    /// 关联的外部资源列表。
    pub resources: Vec<Resource>,
    /// 任务元数据（可选）。
    pub metadata: Option<TaskMetadata>,
    /// 创建时间（ISO 8601）。
    pub created_at: String,
    /// 最后更新时间（ISO 8601）。
    pub updated_at: String,
}
