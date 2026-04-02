//! 笔记操作：note。

use anyhow::Result;
use clap::Subcommand;

use crate::store::TaskStore;
use crate::tools::task as task_tools;

use super::output;
use super::resolve::resolve_task_id;

// ---------------------------------------------------------------------------
// 命令定义
// ---------------------------------------------------------------------------

/// 笔记相关子命令。
#[derive(Subcommand, Debug)]
pub enum NoteCommand {
    /// 添加笔记
    Note {
        /// 任务 ID（支持短前缀）
        task_id: String,
        /// 笔记内容
        content: String,
    },
}

// ---------------------------------------------------------------------------
// 命令处理
// ---------------------------------------------------------------------------

/// 执行笔记命令。
pub fn run(cmd: &NoteCommand, store: &TaskStore, json: bool) -> Result<()> {
    match cmd {
        NoteCommand::Note { task_id, content } => {
            let tasks = store.list_tasks()?;
            let full_id = resolve_task_id(task_id, &tasks)?;
            let _task = task_tools::add_note(store, &full_id, content)?;

            let short_id = &full_id[..8.min(full_id.len())];
            output::print(
                output::Output::Success(format!("笔记已添加到任务 {short_id}")),
                json,
            );
            Ok(())
        }
    }
}
