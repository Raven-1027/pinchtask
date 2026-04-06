//! 清单条目操作：new / edit / check / mv / rm / summary。

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::core;
use crate::store::TaskStore;

use super::output;
use super::resolve::resolve_task_id;

// ---------------------------------------------------------------------------
// 子命令枚举
// ---------------------------------------------------------------------------

/// 清单条目子命令集。
#[derive(Subcommand, Debug)]
pub enum ItemCommands {
    /// 添加清单条目
    New(ItemNewArgs),
    /// 编辑清单条目
    Edit(ItemEditArgs),
    /// 切换清单条目完成/未完成状态
    Check(CheckArgs),
    /// 移动清单条目顺序
    Mv(MvArgs),
    /// 删除清单条目
    Rm(ItemRmArgs),
    /// 查看清单进度摘要
    Summary(SummaryArgs),
}

// ---------------------------------------------------------------------------
// Args 结构体
// ---------------------------------------------------------------------------

/// 添加清单条目
#[derive(Args, Debug)]
pub struct ItemNewArgs {
    /// 任务 ID（支持短前缀）
    pub task_id: String,
    /// 条目标题
    pub title: String,
    /// 详细描述
    #[arg(short, long, default_value = "")]
    pub description: String,
    /// 上下文与计划
    #[arg(short, long)]
    pub plan: Option<String>,
}

/// 编辑清单条目
#[derive(Args, Debug)]
pub struct ItemEditArgs {
    /// 任务 ID（支持短前缀）
    pub task_id: String,
    /// 条目索引
    pub index: usize,
    /// 新标题
    #[arg(short, long)]
    pub title: Option<String>,
    /// 新描述
    #[arg(short, long)]
    pub description: Option<String>,
    /// 新计划
    #[arg(short, long)]
    pub plan: Option<String>,
    /// 标记为已完成
    #[arg(long, conflicts_with = "undone")]
    pub done: bool,
    /// 标记为未完成
    #[arg(long, conflicts_with = "done")]
    pub undone: bool,
}

/// 切换清单条目完成/未完成状态
#[derive(Args, Debug)]
pub struct CheckArgs {
    /// 任务 ID（支持短前缀）
    pub task_id: String,
    /// 条目索引
    pub index: usize,
}

/// 移动清单条目顺序
#[derive(Args, Debug)]
pub struct MvArgs {
    /// 任务 ID（支持短前缀）
    pub task_id: String,
    /// 源索引
    pub from: usize,
    /// 目标索引
    pub to: usize,
}

/// 删除清单条目
#[derive(Args, Debug)]
pub struct ItemRmArgs {
    /// 任务 ID（支持短前缀）
    pub task_id: String,
    /// 条目索引
    pub index: usize,
}

/// 查看清单进度摘要
#[derive(Args, Debug)]
pub struct SummaryArgs {
    /// 任务 ID（支持短前缀）
    pub task_id: String,
}

// ---------------------------------------------------------------------------
// 命令处理
// ---------------------------------------------------------------------------

/// 分发清单条目子命令。
pub async fn run_item(command: &ItemCommands, store: &TaskStore, json: bool) -> Result<()> {
    match command {
        ItemCommands::New(args) => run_new(args, store, json).await,
        ItemCommands::Edit(args) => run_edit(args, store, json).await,
        ItemCommands::Check(args) => run_check(args, store, json).await,
        ItemCommands::Mv(args) => run_mv(args, store, json).await,
        ItemCommands::Rm(args) => run_rm(args, store, json).await,
        ItemCommands::Summary(args) => run_summary(args, store, json).await,
    }
}

/// 添加清单条目。
async fn run_new(args: &ItemNewArgs, store: &TaskStore, json: bool) -> Result<()> {
    let tasks = store.list_tasks().await?;
    let full_id = resolve_task_id(&args.task_id, &tasks)?;
    let task = core::add_checklist_item(store, &full_id, &args.title, &args.description, args.plan.as_deref()).await?;
    output::print(output::Output::Task(&task), json);
    Ok(())
}

/// 编辑清单条目。
async fn run_edit(args: &ItemEditArgs, store: &TaskStore, json: bool) -> Result<()> {
    let tasks = store.list_tasks().await?;
    let full_id = resolve_task_id(&args.task_id, &tasks)?;

    let done_flag = if args.done {
        Some(true)
    } else if args.undone {
        Some(false)
    } else {
        None
    };

    let plan_opt: Option<Option<&str>> = args.plan.as_deref().map(Some);

    let task = core::update_checklist_item(
        store,
        &full_id,
        args.index,
        args.title.as_deref(),
        args.description.as_deref(),
        plan_opt,
        done_flag,
    ).await?;
    output::print(output::Output::Task(&task), json);
    Ok(())
}

/// 切换清单条目完成/未完成状态。
async fn run_check(args: &CheckArgs, store: &TaskStore, json: bool) -> Result<()> {
    let tasks = store.list_tasks().await?;
    let full_id = resolve_task_id(&args.task_id, &tasks)?;

    let task = store.get_task(&full_id).await?;
    if args.index >= task.checklist.len() {
        anyhow::bail!("清单条目索引越界: {}（共 {} 项）", args.index, task.checklist.len());
    }

    let is_done = task.checklist[args.index].done;
    let task = if is_done {
        core::mark_task_undone(store, &full_id, args.index).await?
    } else {
        core::mark_task_done(store, &full_id, args.index).await?
    };

    let status = if is_done { "未完成" } else { "已完成" };
    let item_title = &task.checklist[args.index].task;
    output::print(
        output::Output::Success(format!("[{}] {} → {}", args.index, item_title, status)),
        json,
    );
    Ok(())
}

/// 移动清单条目顺序。
async fn run_mv(args: &MvArgs, store: &TaskStore, json: bool) -> Result<()> {
    let tasks = store.list_tasks().await?;
    let full_id = resolve_task_id(&args.task_id, &tasks)?;
    let task = core::reorder_checklist_item(store, &full_id, args.from, args.to).await?;
    output::print(output::Output::Task(&task), json);
    Ok(())
}

/// 删除清单条目。
async fn run_rm(args: &ItemRmArgs, store: &TaskStore, json: bool) -> Result<()> {
    let tasks = store.list_tasks().await?;
    let full_id = resolve_task_id(&args.task_id, &tasks)?;

    let task = store.get_task(&full_id).await?;
    if args.index >= task.checklist.len() {
        anyhow::bail!("清单条目索引越界: {}（共 {} 项）", args.index, task.checklist.len());
    }
    let item_title = task.checklist[args.index].task.clone();

    let _task = core::remove_checklist_item(store, &full_id, args.index).await?;
    output::print(
        output::Output::Deleted(format!("[{}] {item_title} 已删除", args.index)),
        json,
    );
    Ok(())
}

/// 查看清单进度摘要。
async fn run_summary(args: &SummaryArgs, store: &TaskStore, json: bool) -> Result<()> {
    let tasks = store.list_tasks().await?;
    let full_id = resolve_task_id(&args.task_id, &tasks)?;
    let summary = core::get_checklist_summary(store, &full_id).await?;
    output::print(output::Output::ChecklistSummary(summary), json);
    Ok(())
}
