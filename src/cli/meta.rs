//! 元数据操作：tag（合并原 metadata 模块的 update 命令）。

use anyhow::Result;
use clap::Subcommand;

use crate::models::task::TaskMetadata;
use crate::store::TaskStore;
use crate::tools::task as task_tools;

use super::output;
use super::resolve::resolve_task_id;

// ---------------------------------------------------------------------------
// 命令定义
// ---------------------------------------------------------------------------

/// 元数据相关子命令。
#[derive(Subcommand, Debug)]
pub enum MetaCommand {
    /// 设置标签和元数据
    Tag {
        /// 任务 ID（支持短前缀）
        task_id: String,
        /// 标签（逗号分隔）
        tags: String,
        /// 优先级 (high / medium / low)
        #[arg(long)]
        priority: Option<String>,
        /// 预计完成时间（ISO 8601）
        #[arg(long)]
        eta: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// 命令处理
// ---------------------------------------------------------------------------

/// 执行元数据命令。
pub fn run(cmd: &MetaCommand, store: &TaskStore, json: bool) -> Result<()> {
    match cmd {
        MetaCommand::Tag {
            task_id,
            tags,
            priority,
            eta,
        } => {
            let tasks = store.list_tasks()?;
            let full_id = resolve_task_id(task_id, &tasks)?;

            // 获取现有元数据以合并
            let existing = store.get_task(&full_id)?;
            let mut metadata = existing.metadata.unwrap_or(TaskMetadata {
                tags: None,
                priority: None,
                estimated_completion_time: None,
            });

            // 解析标签
            metadata.tags = Some(
                tags.split(',')
                    .map(|s| s.trim().to_owned())
                    .filter(|s| !s.is_empty())
                    .collect(),
            );

            // 合并优先级
            if let Some(ref p) = priority {
                metadata.priority = Some(p.clone());
            }

            // 合并预计完成时间
            if let Some(ref e) = eta {
                metadata.estimated_completion_time = Some(e.clone());
            }

            let _task = task_tools::update_metadata(store, &full_id, metadata)?;

            let short_id = &full_id[..8.min(full_id.len())];
            output::print(
                output::Output::Success(format!("任务 {short_id} 元数据已更新")),
                json,
            );
            Ok(())
        }
    }
}
