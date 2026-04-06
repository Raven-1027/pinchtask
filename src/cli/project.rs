//! 项目实体操作：new / ls / show / rm / add-task / rm-task。

use anyhow::Result;
use clap::Subcommand;

use crate::core;
use crate::store::TaskStore;

use super::output;

// ---------------------------------------------------------------------------
// 子命令枚举
// ---------------------------------------------------------------------------

/// 项目子命令集。
#[derive(Subcommand, Debug)]
pub enum ProjectCommands {
    /// 创建新项目
    New {
        /// 项目名称
        name: String,
        /// 项目描述
        #[arg(short, long)]
        description: Option<String>,
    },
    /// 列出所有项目
    Ls,
    /// 查看项目详情（含关联任务）
    Show {
        /// 项目 ID（支持短前缀）
        id: String,
    },
    /// 删除项目
    Rm {
        /// 项目 ID（支持短前缀）
        id: String,
        /// 同时删除项目关联的所有任务
        #[arg(long)]
        with_tasks: bool,
    },
    /// 将任务添加到项目
    #[command(name = "add-task")]
    AddTask {
        /// 项目 ID（支持短前缀）
        project_id: String,
        /// 任务 ID（支持短前缀）
        task_id: String,
    },
    /// 将任务从项目中移除
    #[command(name = "rm-task")]
    RmTask {
        /// 项目 ID（支持短前缀）
        project_id: String,
        /// 任务 ID（支持短前缀）
        task_id: String,
    },
}

// ---------------------------------------------------------------------------
// 命令处理
// ---------------------------------------------------------------------------

/// 分发项目子命令。
pub async fn run_project(command: &ProjectCommands, store: &TaskStore, json: bool) -> Result<()> {
    match command {
        ProjectCommands::New { name, description } => {
            run_create(store, name, description.as_deref(), json).await
        }
        ProjectCommands::Ls => run_list(store, json).await,
        ProjectCommands::Show { id } => run_show(store, id, json).await,
        ProjectCommands::Rm { id, with_tasks } => run_delete(store, id, *with_tasks, json).await,
        ProjectCommands::AddTask {
            project_id,
            task_id,
        } => run_add_task(store, project_id, task_id, json).await,
        ProjectCommands::RmTask {
            project_id,
            task_id,
        } => run_remove_task(store, project_id, task_id, json).await,
    }
}

/// 创建新项目。
async fn run_create(
    store: &TaskStore,
    name: &str,
    description: Option<&str>,
    json: bool,
) -> Result<()> {
    let project = core::create_project(store, name, description).await?;
    if json {
        let json_str = serde_json::to_string_pretty(&project).expect("序列化 Project 不应失败");
        println!("{json_str}");
    } else {
        println!(
            "项目已创建: {} ({})",
            project.name,
            &project.id[..8.min(project.id.len())]
        );
    }
    Ok(())
}

/// 列出所有项目。
async fn run_list(store: &TaskStore, json: bool) -> Result<()> {
    let projects = core::list_projects(store).await?;
    if json {
        let json_str = serde_json::to_string_pretty(&projects).expect("序列化 Projects 不应失败");
        println!("{json_str}");
    } else if projects.is_empty() {
        println!("当前没有任何项目");
    } else {
        for project in &projects {
            let short_id = &project.id[..8.min(project.id.len())];
            let desc = project.description.as_deref().unwrap_or("");
            if desc.is_empty() {
                println!("{}  {}", short_id, project.name);
            } else {
                println!("{}  {}  ({})", short_id, project.name, desc);
            }
        }
    }
    Ok(())
}

/// 查看项目详情。
async fn run_show(store: &TaskStore, id: &str, json: bool) -> Result<()> {
    // 尝试短前缀匹配
    let projects = core::list_projects(store).await?;
    let full_id = resolve_project_id(id, &projects)?;
    let project = core::get_project(store, &full_id).await?;
    let tasks = core::get_tasks_for_project(store, &full_id).await?;

    if json {
        let obj = serde_json::json!({
            "project": project,
            "tasks": tasks,
        });
        let json_str = serde_json::to_string_pretty(&obj).expect("序列化不应失败");
        println!("{json_str}");
    } else {
        println!("ID: {}", project.id);
        println!("名称: {}", project.name);
        if let Some(ref desc) = project.description {
            println!("描述: {desc}");
        }
        println!("创建时间: {}", project.created_at);
        println!("更新时间: {}", project.updated_at);

        if tasks.is_empty() {
            println!("\n关联任务: 无");
        } else {
            println!("\n关联任务:");
            for (i, task) in tasks.iter().enumerate() {
                let total = task.checklist.len();
                let done = task.checklist.iter().filter(|item| item.done).count();
                let short_task_id = &task.id[..8.min(task.id.len())];
                println!(
                    "  {}. {} ({}) {}/{}",
                    i + 1,
                    task.task_description,
                    short_task_id,
                    done,
                    total
                );
            }
        }
    }
    Ok(())
}

/// 删除项目。
async fn run_delete(store: &TaskStore, id: &str, with_tasks: bool, json: bool) -> Result<()> {
    let projects = core::list_projects(store).await?;
    let full_id = resolve_project_id(id, &projects)?;

    if with_tasks {
        core::delete_project_with_tasks(store, &full_id).await?;
        output::print(
            output::Output::Deleted(format!(
                "项目 {} 及其关联任务已删除",
                &full_id[..8.min(full_id.len())]
            )),
            json,
        );
    } else {
        core::delete_project(store, &full_id).await?;
        output::print(
            output::Output::Deleted(format!(
                "项目 {} 已删除（关联任务保留）",
                &full_id[..8.min(full_id.len())]
            )),
            json,
        );
    }
    Ok(())
}

/// 将任务添加到项目。
async fn run_add_task(
    store: &TaskStore,
    project_id: &str,
    task_id: &str,
    json: bool,
) -> Result<()> {
    // 解析项目 ID
    let projects = core::list_projects(store).await?;
    let full_project_id = resolve_project_id(project_id, &projects)?;

    // 解析任务 ID
    let tasks = store.list_tasks().await?;
    let full_task_id = super::resolve::resolve_task_id(task_id, &tasks)?;

    core::set_task_project(store, &full_task_id, Some(&full_project_id)).await?;
    output::print(
        output::Output::Success(format!(
            "任务 {} 已添加到项目 {}",
            &full_task_id[..8.min(full_task_id.len())],
            &full_project_id[..8.min(full_project_id.len())]
        )),
        json,
    );
    Ok(())
}

/// 将任务从项目中移除。
async fn run_remove_task(
    store: &TaskStore,
    _project_id: &str,
    task_id: &str,
    json: bool,
) -> Result<()> {
    // 解析任务 ID
    let tasks = store.list_tasks().await?;
    let full_task_id = super::resolve::resolve_task_id(task_id, &tasks)?;

    core::set_task_project(store, &full_task_id, None).await?;
    output::print(
        output::Output::Success(format!(
            "任务 {} 已从项目中移除",
            &full_task_id[..8.min(full_task_id.len())]
        )),
        json,
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

/// 最少前缀长度。
const MIN_PREFIX_LEN: usize = 4;

/// 根据前缀匹配项目 ID。
///
/// 与 `resolve_task_id` 逻辑一致，支持短前缀匹配。
fn resolve_project_id(
    prefix: &str,
    projects: &[crate::models::project::Project],
) -> Result<String> {
    if prefix.len() < MIN_PREFIX_LEN {
        anyhow::bail!(
            "ID 前缀至少需要 {MIN_PREFIX_LEN} 位，当前输入: \"{prefix}\"（{} 位）",
            prefix.len()
        );
    }

    let matches: Vec<&crate::models::project::Project> = projects
        .iter()
        .filter(|p| p.id.starts_with(prefix))
        .collect();

    match matches.len() {
        0 => {
            anyhow::bail!("未找到匹配的项目: {prefix}")
        }
        1 => Ok(matches[0].id.clone()),
        n => {
            let candidates: Vec<String> = matches
                .iter()
                .map(|p| {
                    let short_id = &p.id[..8.min(p.id.len())];
                    format!("  {short_id}  {}", p.name)
                })
                .collect();
            anyhow::bail!(
                "前缀 \"{prefix}\" 匹配到 {n} 个项目，请多输入几位以消除歧义:\n{}",
                candidates.join("\n")
            )
        }
    }
}
