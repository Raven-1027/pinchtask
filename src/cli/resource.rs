//! 资源管理子命令。

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::store::TaskStore;
use crate::tools::task as task_tools;

#[derive(Args, Debug)]
pub struct ResourceCmd {
    #[command(subcommand)]
    command: ResourceAction,
}

#[derive(Subcommand, Debug)]
enum ResourceAction {
    /// 添加资源引用
    Add {
        /// 任务 ID
        task_id: String,
        /// 资源名称
        #[arg(short, long)]
        name: String,
        /// 资源 URL 或文件路径
        #[arg(short, long)]
        url: String,
        /// 资源描述
        #[arg(short, long)]
        description: Option<String>,
    },
}

pub async fn run(cmd: ResourceCmd, store: &TaskStore, json: bool) -> Result<()> {
    match cmd.command {
        ResourceAction::Add {
            task_id,
            name,
            url,
            description,
        } => {
            let task =
                task_tools::add_resource(store, &task_id, &name, &url, description.as_deref())?;
            if json {
                let json_str = serde_json::to_string_pretty(&task)?;
                println!("{json_str}");
            } else {
                println!("资源 \"{name}\" 已添加到任务 {task_id}");
            }
        }
    }
    Ok(())
}
