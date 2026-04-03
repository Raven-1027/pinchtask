//! 笔记操作：note。

use anyhow::Result;
use clap::Args;

use crate::core;
use crate::store::TaskStore;

use super::output;
use super::resolve::resolve_task_id;

// ---------------------------------------------------------------------------
// Args 结构体
// ---------------------------------------------------------------------------

/// 添加笔记
#[derive(Args, Debug)]
pub struct NoteArgs {
    /// 任务 ID（支持短前缀）
    pub task_id: String,
    /// 笔记内容
    pub content: String,
}

// ---------------------------------------------------------------------------
// 命令处理
// ---------------------------------------------------------------------------

/// 添加笔记。
pub async fn run_note(args: &NoteArgs, store: &TaskStore, json: bool) -> Result<()> {
    let tasks = store.list_tasks().await?;
    let full_id = resolve_task_id(&args.task_id, &tasks)?;
    let _task = core::add_note(store, &full_id, &args.content).await?;

    let short_id = &full_id[..8.min(full_id.len())];
    output::print(
        output::Output::Success(format!("笔记已添加到任务 {short_id}")),
        json,
    );
    Ok(())
}
