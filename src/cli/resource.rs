//! 资源操作：link（原 resource add）。

use anyhow::Result;
use clap::Subcommand;

use crate::store::TaskStore;
use crate::tools::task as task_tools;

use super::output;
use super::resolve::resolve_task_id;

// ---------------------------------------------------------------------------
// 命令定义
// ---------------------------------------------------------------------------

/// 资源相关子命令。
#[derive(Subcommand, Debug)]
pub enum ResourceCommand {
    /// 添加资源引用
    Link {
        /// 任务 ID（支持短前缀）
        task_id: String,
        /// 资源名称
        #[arg(long)]
        name: String,
        /// 资源 URL 或文件路径
        #[arg(long)]
        url: String,
        /// 资源描述
        #[arg(short, long)]
        description: Option<String>,
    },
}

// ---------------------------------------------------------------------------
// 命令处理
// ---------------------------------------------------------------------------

/// 执行资源命令。
pub fn run(cmd: &ResourceCommand, store: &TaskStore, json: bool) -> Result<()> {
    match cmd {
        ResourceCommand::Link {
            task_id,
            name,
            url,
            description,
        } => {
            let tasks = store.list_tasks()?;
            let full_id = resolve_task_id(task_id, &tasks)?;
            let _task =
                task_tools::add_resource(store, &full_id, name, url, description.as_deref())?;

            let short_id = &full_id[..8.min(full_id.len())];
            output::print(
                output::Output::Success(format!(
                    "资源 \"{name}\" 已添加到任务 {short_id}"
                )),
                json,
            );
            Ok(())
        }
    }
}
