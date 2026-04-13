//! 任务实体操作：new / ls / show / edit / rm。

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::core;
use crate::models::task::TaskMetadata;
use crate::store::TaskStore;

use super::output;
use super::resolve::{resolve_project_id, resolve_task_id};

// ---------------------------------------------------------------------------
// 子命令枚举
// ---------------------------------------------------------------------------

/// 任务子命令集。
#[derive(Subcommand, Debug)]
pub enum TaskCommands {
    /// 创建新任务
    New(NewArgs),
    /// 列出任务
    Ls(LsArgs),
    /// 查看任务详情
    Show(ShowArgs),
    /// 编辑任务描述、上下文或元数据
    Edit(TaskEditArgs),
    /// 删除任务
    Rm(TaskRmArgs),
}

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
    /// 关联到指定项目
    #[arg(short = 'p', long = "project")]
    pub project: Option<String>,
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
    /// 按项目筛选（支持短 ID 前缀）
    #[arg(short = 'p', long)]
    pub project: Option<String>,
}

/// 查看任务详情
#[derive(Args, Debug)]
pub struct ShowArgs {
    /// 任务 ID（支持短前缀）
    pub id: String,
}

/// 编辑任务
#[derive(Args, Debug)]
pub struct TaskEditArgs {
    /// 任务 ID（支持短前缀）
    pub id: String,
    /// 新的任务描述
    #[arg(short, long)]
    pub description: Option<String>,
    /// 新的共享上下文
    #[arg(short, long)]
    pub context: Option<String>,
    /// 优先级 (high / medium / low)
    #[arg(long)]
    pub priority: Option<String>,
    /// 标签，逗号分隔
    #[arg(long)]
    pub tags: Option<String>,
    /// 预计完成时间，ISO 8601
    #[arg(long)]
    pub eta: Option<String>,
}

/// 删除任务
#[derive(Args, Debug)]
pub struct TaskRmArgs {
    /// 任务 ID（支持短前缀）
    pub id: String,
}

// ---------------------------------------------------------------------------
// 命令处理
// ---------------------------------------------------------------------------

/// 分发任务子命令。
pub async fn run_task(
    command: &TaskCommands,
    store: &TaskStore,
    json: bool,
    workspace_project_id: Option<&str>,
) -> Result<()> {
    match command {
        TaskCommands::New(args) => run_new(args, store, json, workspace_project_id).await,
        TaskCommands::Ls(args) => run_ls(args, store, json, workspace_project_id).await,
        TaskCommands::Show(args) => run_show(args, store, json).await,
        TaskCommands::Edit(args) => run_edit(args, store, json).await,
        TaskCommands::Rm(args) => run_rm(args, store, json).await,
    }
}

/// 创建新任务。
async fn run_new(
    args: &NewArgs,
    store: &TaskStore,
    json: bool,
    workspace_project_id: Option<&str>,
) -> Result<()> {
    // 优先级：显式 --project > .pinchproject > None
    let project_id = args.project.as_deref().or(workspace_project_id);
    if let Some(pid) = workspace_project_id
        && args.project.is_none()
    {
        tracing::info!(
            project_id = pid,
            "auto-associating task with workspace project"
        );
    }
    let project_id = project_id.ok_or_else(|| {
        anyhow::anyhow!(
            "未指定项目。请使用 --project <项目ID> 指定项目，或在项目目录中创建 .pinchproject 文件。\n  提示: 使用 `pinchtask project ls` 查看可用项目"
        )
    })?;

    let task = core::initialize_task(
        store,
        &args.description,
        args.context.as_deref(),
        vec![],
        vec![],
        vec![],
        None,
        Some(project_id),
    )
    .await?;
    output::print(output::Output::Task(&task), json);
    Ok(())
}

/// 列出任务。
async fn run_ls(
    args: &LsArgs,
    store: &TaskStore,
    json: bool,
    workspace_project_id: Option<&str>,
) -> Result<()> {
    // 优先级：显式 --project > .pinchproject > None
    let effective_project = args.project.as_deref().or(workspace_project_id);
    if let Some(pid) = workspace_project_id
        && args.project.is_none()
    {
        tracing::info!(
            project_id = pid,
            "auto-filtering tasks by workspace project"
        );
    }

    let tasks = if let Some(project_prefix) = effective_project {
        let projects = core::list_projects(store).await?;
        let full_project_id = resolve_project_id(project_prefix, &projects)?;
        store.get_tasks_for_project(&full_project_id).await?
    } else {
        store.list_tasks().await?
    };

    // 过滤
    let filtered: Vec<_> = if args.done {
        tasks
            .iter()
            .filter(|t| !t.checklist.is_empty() && t.checklist.iter().all(|i| i.done))
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
                let pa = a
                    .metadata
                    .as_ref()
                    .and_then(|m| m.priority.as_deref())
                    .unwrap_or("medium");
                let pb = b
                    .metadata
                    .as_ref()
                    .and_then(|m| m.priority.as_deref())
                    .unwrap_or("medium");
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

    let entries: Vec<output::TaskListEntry> = limited
        .iter()
        .map(|t| output::task_to_list_entry(t))
        .collect();

    output::print(
        output::Output::TaskList {
            tasks: entries,
            long: args.long,
        },
        json,
    );
    Ok(())
}

/// 查看任务详情。
async fn run_show(args: &ShowArgs, store: &TaskStore, json: bool) -> Result<()> {
    let tasks = store.list_tasks().await?;
    let full_id = resolve_task_id(&args.id, &tasks)?;
    let task = store.get_task(&full_id).await?;
    output::print(output::Output::Task(&task), json);
    Ok(())
}

/// 编辑任务描述、上下文或元数据。
async fn run_edit(args: &TaskEditArgs, store: &TaskStore, json: bool) -> Result<()> {
    let tasks = store.list_tasks().await?;
    let full_id = resolve_task_id(&args.id, &tasks)?;

    if args.description.is_none()
        && args.context.is_none()
        && args.priority.is_none()
        && args.tags.is_none()
        && args.eta.is_none()
    {
        anyhow::bail!(
            "至少需要指定一个可修改的字段 (--description / --context / --priority / --tags / --eta)"
        );
    }

    // 更新 description
    if let Some(desc) = &args.description {
        core::update_task_description(store, &full_id, desc).await?;
    }
    // 更新 context
    if let Some(ctx) = &args.context {
        core::update_context(store, &full_id, ctx).await?;
    }
    // 更新 metadata（priority / tags / eta）
    if args.priority.is_some() || args.tags.is_some() || args.eta.is_some() {
        let existing = store.get_task(&full_id).await?;
        let mut metadata = existing.metadata.unwrap_or(TaskMetadata {
            tags: None,
            priority: None,
            estimated_completion_time: None,
        });
        if let Some(ref p) = args.priority {
            metadata.priority = Some(p.clone());
        }
        if let Some(ref t) = args.tags {
            metadata.tags = Some(
                t.split(',')
                    .map(|s| s.trim().to_owned())
                    .filter(|s| !s.is_empty())
                    .collect(),
            );
        }
        if let Some(ref e) = args.eta {
            metadata.estimated_completion_time = Some(e.clone());
        }
        core::update_metadata(store, &full_id, metadata).await?;
    }

    // 最终输出更新后的任务
    let task = store.get_task(&full_id).await?;
    output::print(output::Output::Task(&task), json);
    Ok(())
}

/// 删除任务。
async fn run_rm(args: &TaskRmArgs, store: &TaskStore, json: bool) -> Result<()> {
    let tasks = store.list_tasks().await?;
    let full_id = resolve_task_id(&args.id, &tasks)?;
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
