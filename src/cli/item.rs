//! 清单条目操作：add / check / edit-item / mv / rm-item / summary。
//!
//! 原先的 checklist 模块被拆解为顶层扁平命令，不再需要 checklist 前缀。

use anyhow::Result;
use clap::Subcommand;

use crate::store::TaskStore;
use crate::tools::task as task_tools;

use super::output;
use super::resolve::resolve_task_id;

// ---------------------------------------------------------------------------
// 命令定义
// ---------------------------------------------------------------------------

/// 清单条目相关子命令。
#[derive(Subcommand, Debug)]
pub enum ItemCommand {
    /// 添加清单条目
    Add {
        /// 任务 ID（支持短前缀）
        task_id: String,
        /// 条目标题
        title: String,
        /// 详细描述
        #[arg(short, long, default_value = "")]
        description: String,
        /// 上下文与计划
        #[arg(short, long)]
        plan: Option<String>,
    },
    /// 切换清单条目完成/未完成状态
    Check {
        /// 任务 ID（支持短前缀）
        task_id: String,
        /// 条目索引
        index: usize,
    },
    /// 编辑清单条目
    EditItem {
        /// 任务 ID（支持短前缀）
        task_id: String,
        /// 条目索引
        index: usize,
        /// 新标题
        #[arg(short, long)]
        title: Option<String>,
        /// 新描述
        #[arg(short, long)]
        description: Option<String>,
        /// 新计划
        #[arg(short, long)]
        plan: Option<String>,
        /// 标记为已完成
        #[arg(long, conflicts_with = "undone")]
        done: bool,
        /// 标记为未完成
        #[arg(long, conflicts_with = "done")]
        undone: bool,
    },
    /// 移动清单条目顺序
    Mv {
        /// 任务 ID（支持短前缀）
        task_id: String,
        /// 源索引
        from: usize,
        /// 目标索引
        to: usize,
    },
    /// 删除清单条目
    RmItem {
        /// 任务 ID（支持短前缀）
        task_id: String,
        /// 条目索引
        index: usize,
    },
    /// 查看清单进度摘要
    Summary {
        /// 任务 ID（支持短前缀）
        task_id: String,
    },
}

// ---------------------------------------------------------------------------
// 命令处理
// ---------------------------------------------------------------------------

/// 执行清单条目命令。
pub fn run(cmd: &ItemCommand, store: &TaskStore, json: bool) -> Result<()> {
    match cmd {
        ItemCommand::Add {
            task_id,
            title,
            description,
            plan,
        } => run_add(store, json, task_id, title, description, plan),
        ItemCommand::Check { task_id, index } => run_check(store, json, task_id, *index),
        ItemCommand::EditItem {
            task_id,
            index,
            title,
            description,
            plan,
            done,
            undone,
        } => run_edit_item(
            store, json, task_id, *index, title.as_deref(), description.as_deref(),
            plan.as_deref(), *done, *undone,
        ),
        ItemCommand::Mv {
            task_id,
            from,
            to,
        } => run_mv(store, json, task_id, *from, *to),
        ItemCommand::RmItem { task_id, index } => run_rm_item(store, json, task_id, *index),
        ItemCommand::Summary { task_id } => run_summary(store, json, task_id),
    }
}

fn run_add(
    store: &TaskStore,
    json: bool,
    task_id: &str,
    title: &str,
    description: &str,
    plan: &Option<String>,
) -> Result<()> {
    let tasks = store.list_tasks()?;
    let full_id = resolve_task_id(task_id, &tasks)?;
    let task = task_tools::add_checklist_item(store, &full_id, title, description, plan.as_deref())?;
    output::print(output::Output::Task(&task), json);
    Ok(())
}

fn run_check(store: &TaskStore, json: bool, task_id: &str, index: usize) -> Result<()> {
    let tasks = store.list_tasks()?;
    let full_id = resolve_task_id(task_id, &tasks)?;

    // 读取当前状态以决定 toggle 方向
    let task = store.get_task(&full_id)?;
    if index >= task.checklist.len() {
        anyhow::bail!("清单条目索引越界: {index}（共 {} 项）", task.checklist.len());
    }

    let is_done = task.checklist[index].done;
    let task = if is_done {
        task_tools::mark_task_undone(store, &full_id, index)?
    } else {
        task_tools::mark_task_done(store, &full_id, index)?
    };

    let status = if is_done { "未完成" } else { "已完成" };
    let item_title = &task.checklist[index].task;
    output::print(
        output::Output::Success(format!("[{index}] {item_title} → {status}")),
        json,
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_edit_item(
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
    let tasks = store.list_tasks()?;
    let full_id = resolve_task_id(task_id, &tasks)?;

    // 确定 done 状态: --done => Some(true), --undone => Some(false), 都没传 => None
    let done_flag = if done {
        Some(true)
    } else if undone {
        Some(false)
    } else {
        None
    };

    // plan: 区分"未传入"和"传入空字符串"——用 Option<Option<&str>>
    // 如果用户传了 --plan "xxx" => Some(Some("xxx"))
    // 如果没传 --plan => None
    let plan_opt: Option<Option<&str>> = plan.map(Some);

    let task = task_tools::update_checklist_item(
        store,
        &full_id,
        index,
        title,
        description,
        plan_opt,
        done_flag,
    )?;
    output::print(output::Output::Task(&task), json);
    Ok(())
}

fn run_mv(
    store: &TaskStore,
    json: bool,
    task_id: &str,
    from: usize,
    to: usize,
) -> Result<()> {
    let tasks = store.list_tasks()?;
    let full_id = resolve_task_id(task_id, &tasks)?;
    let task = task_tools::reorder_checklist_item(store, &full_id, from, to)?;
    output::print(output::Output::Task(&task), json);
    Ok(())
}

fn run_rm_item(store: &TaskStore, json: bool, task_id: &str, index: usize) -> Result<()> {
    let tasks = store.list_tasks()?;
    let full_id = resolve_task_id(task_id, &tasks)?;

    // 先获取标题用于输出
    let task = store.get_task(&full_id)?;
    if index >= task.checklist.len() {
        anyhow::bail!("清单条目索引越界: {index}（共 {} 项）", task.checklist.len());
    }
    let item_title = task.checklist[index].task.clone();

    let _task = task_tools::remove_checklist_item(store, &full_id, index)?;
    output::print(
        output::Output::Deleted(format!("[{index}] {item_title} 已删除")),
        json,
    );
    Ok(())
}

fn run_summary(store: &TaskStore, json: bool, task_id: &str) -> Result<()> {
    let tasks = store.list_tasks()?;
    let full_id = resolve_task_id(task_id, &tasks)?;
    let summary = task_tools::get_checklist_summary(store, &full_id)?;
    output::print(output::Output::ChecklistSummary(summary), json);
    Ok(())
}
