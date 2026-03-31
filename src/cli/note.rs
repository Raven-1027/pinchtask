//! 笔记管理子命令。

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::store::TaskStore;
use crate::tools::task as task_tools;

#[derive(Args, Debug)]
pub struct NoteCmd {
    #[command(subcommand)]
    pub action: NoteAction,
}

#[derive(Subcommand, Debug)]
pub enum NoteAction {
    /// 添加笔记
    Add {
        /// 任务 ID
        task_id: String,
        /// 笔记内容
        content: String,
    },
}

pub async fn run(cmd: NoteCmd, store: &TaskStore, json: bool) -> Result<()> {
    match cmd.action {
        NoteAction::Add { task_id, content } => {
            let task = task_tools::add_note(store, &task_id, &content)?;
            if json {
                let json_str = serde_json::to_string_pretty(&task)?;
                println!("{json_str}");
            } else {
                println!("笔记已添加到任务 {task_id}");
            }
        }
    }
    Ok(())
}
