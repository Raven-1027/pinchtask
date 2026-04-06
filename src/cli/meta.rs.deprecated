//! 元数据操作：tag。

use anyhow::Result;
use clap::Args;

use crate::core;
use crate::models::task::TaskMetadata;
use crate::store::TaskStore;

use super::output;
use super::resolve::resolve_task_id;

// ---------------------------------------------------------------------------
// Args 结构体
// ---------------------------------------------------------------------------

/// 设置标签和元数据
#[derive(Args, Debug)]
pub struct TagArgs {
    /// 任务 ID（支持短前缀）
    pub task_id: String,
    /// 标签（逗号分隔，不传则保留现有标签）
    pub tags: Option<String>,
    /// 优先级 (high / medium / low)
    #[arg(long)]
    pub priority: Option<String>,
    /// 预计完成时间（ISO 8601）
    #[arg(long)]
    pub eta: Option<String>,
}

// ---------------------------------------------------------------------------
// 命令处理
// ---------------------------------------------------------------------------

/// 设置标签和元数据。
pub async fn run_tag(args: &TagArgs, store: &TaskStore, json: bool) -> Result<()> {
    let tasks = store.list_tasks().await?;
    let full_id = resolve_task_id(&args.task_id, &tasks)?;

    let existing = store.get_task(&full_id).await?;
    let mut metadata = existing.metadata.unwrap_or(TaskMetadata {
        tags: None,
        priority: None,
        estimated_completion_time: None,
    });

    if let Some(ref t) = args.tags {
        metadata.tags = Some(
            t.split(',')
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
                .collect(),
        );
    }

    if let Some(ref p) = args.priority {
        metadata.priority = Some(p.clone());
    }

    if let Some(ref e) = args.eta {
        metadata.estimated_completion_time = Some(e.clone());
    }

    let _task = core::update_metadata(store, &full_id, metadata).await?;

    let short_id = &full_id[..8.min(full_id.len())];
    output::print(
        output::Output::Success(format!("任务 {short_id} 元数据已更新")),
        json,
    );
    Ok(())
}
