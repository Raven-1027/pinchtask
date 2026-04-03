//! 清单条目操作：add / check / edit-item / mv / rm-item / summary。

use anyhow::Result;
use clap::Args;

use crate::core;
use crate::store::TaskStore;

use super::output;
use super::resolve::resolve_task_id;

// ---------------------------------------------------------------------------
// Args 结构体
// ---------------------------------------------------------------------------

/// 添加清单条目
#[derive(Args, Debug)]
pub struct AddArgs {
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

/// 查看清单进度摘要
#[derive(Args, Debug)]
pub struct SummaryArgs {
    /// 任务 ID（支持短前缀）
    pub task_id: String,
}

// ---------------------------------------------------------------------------
// 命令处理
// ---------------------------------------------------------------------------

/// 添加清单条目。
pub async fn run_add(args: &AddArgs, store: &TaskStore, json: bool) -> Result<()> {
    let tasks = store.list_tasks().await?;
    let full_id = resolve_task_id(&args.task_id, &tasks)?;
    let task = core::add_checklist_item(store, &full_id, &args.title, &args.description, args.plan.as_deref()).await?;
    output::print(output::Output::Task(&task), json);
    Ok(())
}

/// 切换清单条目完成/未完成状态。
pub async fn run_check(args: &CheckArgs, store: &TaskStore, json: bool) -> Result<()> {
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

/// 编辑清单条目（由 mod.rs Edit 分流调用）。
#[allow(clippy::too_many_arguments)]
pub async fn run_edit_item(
    store: &TaskStore,
    json: bool,
    task_id: &str,
    index: usize,
    title: Option<&str>,
    description: Option<&str>,
    plan: Option<&str>,
    done: bool,
    undone: bool,
) -> Result<()> {
    let tasks = store.list_tasks().await?;
    let full_id = resolve_task_id(task_id, &tasks)?;

    let done_flag = if done {
        Some(true)
    } else if undone {
        Some(false)
    } else {
        None
    };

    let plan_opt: Option<Option<&str>> = plan.map(Some);

    let task = core::update_checklist_item(
        store,
        &full_id,
        index,
        title,
        description,
        plan_opt,
        done_flag,
    ).await?;
    output::print(output::Output::Task(&task), json);
    Ok(())
}

/// 移动清单条目顺序。
pub async fn run_mv(args: &MvArgs, store: &TaskStore, json: bool) -> Result<()> {
    let tasks = store.list_tasks().await?;
    let full_id = resolve_task_id(&args.task_id, &tasks)?;
    let task = core::reorder_checklist_item(store, &full_id, args.from, args.to).await?;
    output::print(output::Output::Task(&task), json);
    Ok(())
}

/// 删除清单条目（由 mod.rs Rm 分流调用）。
pub async fn run_rm_item(store: &TaskStore, json: bool, task_id: &str, index: usize) -> Result<()> {
    let tasks = store.list_tasks().await?;
    let full_id = resolve_task_id(task_id, &tasks)?;

    let task = store.get_task(&full_id).await?;
    if index >= task.checklist.len() {
        anyhow::bail!("清单条目索引越界: {index}（共 {} 项）", task.checklist.len());
    }
    let item_title = task.checklist[index].task.clone();

    let _task = core::remove_checklist_item(store, &full_id, index).await?;
    output::print(
        output::Output::Deleted(format!("[{index}] {item_title} 已删除")),
        json,
    );
    Ok(())
}

/// 查看清单进度摘要。
pub async fn run_summary(args: &SummaryArgs, store: &TaskStore, json: bool) -> Result<()> {
    let tasks = store.list_tasks().await?;
    let full_id = resolve_task_id(&args.task_id, &tasks)?;
    let summary = core::get_checklist_summary(store, &full_id).await?;
    output::print(output::Output::ChecklistSummary(summary), json);
    Ok(())
}
