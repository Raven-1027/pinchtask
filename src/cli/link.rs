//! 资源引用操作：new（预留 ls / rm）。

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::core;
use crate::store::TaskStore;

use super::output;
use super::resolve::resolve_task_id;

// ---------------------------------------------------------------------------
// 子命令枚举
// ---------------------------------------------------------------------------

/// 资源引用子命令集。
#[derive(Subcommand, Debug)]
pub enum LinkCommands {
    /// 添加资源引用
    New(LinkNewArgs),
}

// ---------------------------------------------------------------------------
// Args 结构体
// ---------------------------------------------------------------------------

/// 添加资源引用
#[derive(Args, Debug)]
pub struct LinkNewArgs {
    /// 任务 ID（支持短前缀）
    pub task_id: String,
    /// 资源名称
    #[arg(long)]
    pub name: String,
    /// 资源 URL 或文件路径
    #[arg(long)]
    pub url: String,
    /// 资源描述
    #[arg(short, long)]
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// 命令处理
// ---------------------------------------------------------------------------

/// 分发资源引用子命令。
pub async fn run_link(command: &LinkCommands, store: &TaskStore, json: bool) -> Result<()> {
    match command {
        LinkCommands::New(args) => run_link_new(args, store, json).await,
    }
}

/// 添加资源引用。
async fn run_link_new(args: &LinkNewArgs, store: &TaskStore, json: bool) -> Result<()> {
    let tasks = store.list_tasks().await?;
    let full_id = resolve_task_id(&args.task_id, &tasks)?;
    let _task =
        core::add_resource(store, &full_id, &args.name, &args.url, args.description.as_deref()).await?;

    let short_id = &full_id[..8.min(full_id.len())];
    output::print(
        output::Output::Success(format!(
            "资源 \"{}\" 已添加到任务 {short_id}",
            args.name
        )),
        json,
    );
    Ok(())
}
