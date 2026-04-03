//! 任务实体操作：new / ls / show / edit / rm。

use anyhow::Result;
use clap::Args;

use crate::core;
use crate::models::task::TaskMetadata;
use crate::store::TaskStore;

use super::output;
use super::resolve::resolve_task_id;

// ---------------------------------------------------------------------------
// Args 结构体
// ---------------------------------------------------------------------------

/// 创建新任务
#[derive(Args, Debug)]
pub struct NewArgs {
    /// 任务描述
    pub description: String,
    /// 共享上下文
    #[arg(short, long)]
    pub context: Option<String>,
}

/// 列出任务
#[derive(Args, Debug)]
pub struct LsArgs {
    /// 显示全部（active + done）
    #[arg(short, long, conflicts_with = "done")]
    pub all: bool,
    /// 只显示已完成任务
    #[arg(short, long, conflicts_with = "all")]
    pub done: bool,
    /// 详细模式（显示更多列）
    #[arg(short, long)]
    pub long: bool,
    /// 限制显示数量
    #[arg(short = 'n', long, default_value = "10")]
    pub limit: usize,
    /// 排序字段: time / priority / progress
    #[arg(long, default_value = "time")]
    pub sort: String,
}

/// 查看任务详情
#[derive(Args, Debug)]
pub struct ShowArgs {
    /// 任务 ID（支持短前缀）
    pub id: String,
}

// ---------------------------------------------------------------------------
// 命令处理
// ---------------------------------------------------------------------------

/// 创建新任务。
pub async fn run_new(args: &NewArgs, store: &TaskStore, json: bool) -> Result<()> {
    let task = core::initialize_task(
        store,
        &args.description,
        args.context.as_deref(),
        vec![],
        vec![],
        vec![],
        None,
    )
    .await?;
    output::print(output::Output::Task(&task), json);
    Ok(())
}

/// 列出任务。
pub async fn run_ls(args: &LsArgs, store: &TaskStore, json: bool) -> Result<()> {
    let tasks = store.list_tasks().await?;

    // 过滤
    let filtered: Vec<_> = if args.done {
        tasks
            .iter()
            .filter(|t| {
                !t.checklist.is_empty() && t.checklist.iter().all(|i| i.done)
            })
            .collect()
    } else if args.all {
        tasks.iter().collect()
    } else {
        tasks
            .iter()
            .filter(|t| t.checklist.is_empty() || t.checklist.iter().any(|i| !i.done))
            .collect()
    };

    // 排序
    let mut sorted = filtered;
    match args.sort.as_str() {
        "priority" => {
            sorted.sort_by(|a, b| {
                let pa = a.metadata.as_ref().and_then(|m| m.priority.as_deref()).unwrap_or("medium");
                let pb = b.metadata.as_ref().and_then(|m| m.priority.as_deref()).unwrap_or("medium");
                priority_order(pa).cmp(&priority_order(pb))
            });
        }
        "progress" => {
            sorted.sort_by(|a, b| {
                let (ad, at) = progress_ratio(a);
                let (bd, bt) = progress_ratio(b);
                (ad * bt).cmp(&(bd * at))
            });
        }
        _ => {}
    }

    // 限制数量
    let limited: Vec<_> = sorted.into_iter().take(args.limit).collect();

    let entries: Vec<output::TaskListEntry> =
        limited.iter().map(|t| output::task_to_list_entry(t)).collect();

    output::print(output::Output::TaskList { tasks: entries, long: args.long }, json);
    Ok(())
}

/// 查看任务详情。
pub async fn run_show(args: &ShowArgs, store: &TaskStore, json: bool) -> Result<()> {
    let tasks = store.list_tasks().await?;
    let full_id = resolve_task_id(&args.id, &tasks)?;
    let task = store.get_task(&full_id).await?;
    output::print(output::Output::Task(&task), json);
    Ok(())
}

/// 编辑任务描述、上下文或元数据（由 mod.rs Edit 分流调用）。
///
/// 传入的字段会在一次调用中全部更新，未传入的字段保持不变。
pub async fn run_edit(
    store: &TaskStore,
    json: bool,
    id: &str,
    description: Option<&str>,
    context: Option<&str>,
    priority: Option<&str>,
    tags: Option<&str>,
    eta: Option<&str>,
) -> Result<()> {
    let tasks = store.list_tasks().await?;
    let full_id = resolve_task_id(id, &tasks)?;

    if description.is_none()
        && context.is_none()
        && priority.is_none()
        && tags.is_none()
        && eta.is_none()
    {
        anyhow::bail!("至少需要指定一个可修改的字段 (--description / --context / --priority / --tags / --eta)");
    }

    // 更新 description
    if let Some(desc) = description {
        core::update_task_description(store, &full_id, desc).await?;
    }
    // 更新 context
    if let Some(ctx) = context {
        core::update_context(store, &full_id, ctx).await?;
    }
    // 更新 metadata（priority / tags / eta）
    if priority.is_some() || tags.is_some() || eta.is_some() {
        let existing = store.get_task(&full_id).await?;
        let mut metadata = existing.metadata.unwrap_or(TaskMetadata {
            tags: None,
            priority: None,
            estimated_completion_time: None,
        });
        if let Some(p) = priority {
            metadata.priority = Some(p.to_owned());
        }
        if let Some(t) = tags {
            metadata.tags = Some(
                t.split(',')
                    .map(|s| s.trim().to_owned())
                    .filter(|s| !s.is_empty())
                    .collect(),
            );
        }
        if let Some(e) = eta {
            metadata.estimated_completion_time = Some(e.to_owned());
        }
        core::update_metadata(store, &full_id, metadata).await?;
    }

    // 最终输出更新后的任务
    let task = store.get_task(&full_id).await?;
    output::print(output::Output::Task(&task), json);
    Ok(())
}

/// 删除任务（由 mod.rs Rm 分流调用）。
pub async fn run_rm(store: &TaskStore, json: bool, id: &str) -> Result<()> {
    let tasks = store.list_tasks().await?;
    let full_id = resolve_task_id(id, &tasks)?;
    core::clear_task(store, &full_id).await?;
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
