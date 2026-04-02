//! 任务实体操作：new / ls / show / edit / rm。

use anyhow::Result;
use clap::Subcommand;

use crate::store::TaskStore;
use crate::tools::task as task_tools;

use super::output;
use super::resolve::resolve_task_id;

// ---------------------------------------------------------------------------
// 命令定义
// ---------------------------------------------------------------------------

/// 任务实体相关子命令。
#[derive(Subcommand, Debug)]
pub enum TaskCommand {
    /// 创建新任务
    New {
        /// 任务描述
        description: String,
        /// 共享上下文
        #[arg(short, long)]
        context: Option<String>,
    },
    /// 列出任务
    Ls {
        /// 显示全部（active + done）
        #[arg(short, long, conflicts_with = "done")]
        all: bool,
        /// 只显示已完成任务
        #[arg(short, long, conflicts_with = "all")]
        done: bool,
        /// 详细模式（显示更多列）
        #[arg(short, long)]
        long: bool,
        /// 限制显示数量
        #[arg(short, long, default_value = "10")]
        limit: usize,
        /// 排序字段: time / priority / progress
        #[arg(long, default_value = "time")]
        sort: String,
    },
    /// 查看任务详情
    Show {
        /// 任务 ID（支持短前缀）
        id: String,
    },
    /// 编辑任务描述或上下文
    Edit {
        /// 任务 ID（支持短前缀）
        id: String,
        /// 新的任务描述
        #[arg(short, long)]
        description: Option<String>,
        /// 新的共享上下文
        #[arg(short, long)]
        context: Option<String>,
    },
    /// 删除任务
    Rm {
        /// 任务 ID（支持短前缀）
        id: String,
    },
}

// ---------------------------------------------------------------------------
// 命令处理
// ---------------------------------------------------------------------------

/// 执行任务实体命令。
pub fn run(cmd: &TaskCommand, store: &TaskStore, json: bool) -> Result<()> {
    match cmd {
        TaskCommand::New { description, context } => run_new(store, json, description, context),
        TaskCommand::Ls {
            all,
            done,
            long,
            limit,
            sort,
        } => run_ls(store, json, *all, *done, *long, *limit, sort),
        TaskCommand::Show { id } => run_show(store, json, id),
        TaskCommand::Edit {
            id,
            description,
            context,
        } => run_edit(store, json, id, description.as_deref(), context.as_deref()),
        TaskCommand::Rm { id } => run_rm(store, json, id),
    }
}

fn run_new(
    store: &TaskStore,
    json: bool,
    description: &str,
    context: &Option<String>,
) -> Result<()> {
    let task = task_tools::initialize_task(
        store,
        description,
        context.as_deref(),
        vec![],
        vec![],
        vec![],
        None,
    )?;
    output::print(output::Output::Task(&task), json);
    Ok(())
}

fn run_ls(
    store: &TaskStore,
    json: bool,
    show_all: bool,
    show_done_only: bool,
    long: bool,
    limit: usize,
    sort: &str,
) -> Result<()> {
    let tasks = store.list_tasks()?;

    // 过滤
    let filtered: Vec<_> = if show_done_only {
        // -d: 只显示已完成
        tasks
            .iter()
            .filter(|t| {
                !t.checklist.is_empty() && t.checklist.iter().all(|i| i.done)
            })
            .collect()
    } else if show_all {
        // -a: 显示全部
        tasks.iter().collect()
    } else {
        // 默认: 只显示活跃任务（有未完成清单项，或清单为空的新任务）
        tasks
            .iter()
            .filter(|t| t.checklist.is_empty() || t.checklist.iter().any(|i| !i.done))
            .collect()
    };

    // 排序
    let mut sorted = filtered;
    match sort {
        "priority" => {
            sorted.sort_by(|a, b| {
                let pa = a.metadata.as_ref().and_then(|m| m.priority.as_deref()).unwrap_or("medium");
                let pb = b.metadata.as_ref().and_then(|m| m.priority.as_deref()).unwrap_or("medium");
                priority_order(pa).cmp(&priority_order(pb))
            });
        }
        "progress" => {
            sorted.sort_by(|a, b| {
                let ra = progress_ratio(a);
                let rb = progress_ratio(b);
                ra.cmp(&rb)
            });
        }
        // "time" 或默认: 按创建时间排序（store 已按创建时间排序）
        _ => {}
    }

    // 限制数量
    let limited: Vec<_> = sorted.into_iter().take(limit).collect();

    // 构建列表条目
    let entries: Vec<output::TaskListEntry> =
        limited.iter().map(|t| output::task_to_list_entry(t)).collect();

    output::print(output::Output::TaskList { tasks: entries, long }, json);
    Ok(())
}

fn run_show(store: &TaskStore, json: bool, id: &str) -> Result<()> {
    let tasks = store.list_tasks()?;
    let full_id = resolve_task_id(id, &tasks)?;
    let task = store.get_task(&full_id)?;
    output::print(output::Output::Task(&task), json);
    Ok(())
}

fn run_edit(
    store: &TaskStore,
    json: bool,
    id: &str,
    description: Option<&str>,
    context: Option<&str>,
) -> Result<()> {
    let tasks = store.list_tasks()?;
    let full_id = resolve_task_id(id, &tasks)?;

    // 至少需要修改一个字段
    if description.is_none() && context.is_none() {
        anyhow::bail!("至少需要指定 --description 或 --context 之一");
    }

    let mut task = store.get_task(&full_id)?;

    if let Some(desc) = description {
        task = task_tools::update_task_description(store, &full_id, desc)?;
    }
    if let Some(ctx) = context {
        task = task_tools::update_context(store, &full_id, ctx)?;
    }

    output::print(output::Output::Task(&task), json);
    Ok(())
}

fn run_rm(store: &TaskStore, json: bool, id: &str) -> Result<()> {
    let tasks = store.list_tasks()?;
    let full_id = resolve_task_id(id, &tasks)?;
    task_tools::clear_task(store, &full_id)?;
    let short_id = &full_id[..8.min(full_id.len())];
    output::print(
        output::Output::Deleted(format!("任务 {short_id} 已删除")),
        json,
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

/// 优先级排序权重（high=0, medium=1, low=2, 其他=3）。
fn priority_order(p: &str) -> u8 {
    match p {
        "high" => 0,
        "medium" => 1,
        "low" => 2,
        _ => 3,
    }
}

/// 任务进度比（已完成为 1.0，空清单为 0.0）。
fn progress_ratio(task: &crate::models::task::Task) -> (usize, usize) {
    let total = task.checklist.len();
    if total == 0 {
        return (0, 1);
    }
    let done = task.checklist.iter().filter(|i| i.done).count();
    (done, total)
}
