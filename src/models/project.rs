//! 项目数据模型定义。
//!
//! Project 结构体对应 projects 表，与 Task 通过 tasks.project_id 外键实现一对多关系。

use serde::{Deserialize, Serialize};

/// 项目结构体。
///
/// 每个项目可以关联多个任务，用于组织和分组管理。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// 项目唯一标识（UUID v4）。
    pub id: String,
    /// 项目名称。
    pub name: String,
    /// 项目描述（可选）。
    pub description: Option<String>,
    /// 创建时间（ISO 8601）。
    pub created_at: String,
    /// 最后更新时间（ISO 8601）。
    pub updated_at: String,
}
