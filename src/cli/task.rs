//! 任务管理子命令。

use anyhow::Result;
use clap::{Args, Subcommand};
use serde::Serialize;

use crate::models::task::Task;
use crate::store::TaskStore;
use crate::tools::task as task_tools;

use super::output;

/// 任务管理命令。
#[derive(Args, Debug)]
pub struct TaskCmd {
    #[command(subcommand)]
    pub action: TaskAction,
}

#[derive(Subcommand, Debug)]
pub enum TaskAction {
    /// 创建新任务
    Create {
        /// 任务描述
        #[arg(short, long)]
        description: String,
        /// 共享上下文
        #[arg(short, long)]
        context: Option<String>,
    },
    /// 列出所有任务
    List {
        /// 按状态过滤 (all, active, done)
        #[arg(short, long, default_value = "all")]
        status: String,
        /// 限制输出数量
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// 查看任务详情
    Show {
        /// 任务 ID
        id: String,
    },
    /// 删除任务
    Delete {
        /// 任务 ID
        id: String,
    },
    /// 更新任务描述
    Update {
        /// 任务 ID
        id: String,
        /// 新的任务描述
        #[arg(short, long)]
        description: String,
    },
    /// 更新共享上下文
    Context {
        /// 任务 ID
        id: String,
        /// 新的上下文
        context: String,
    },
    /// 获取清单完成进度摘要
    Summary {
        /// 任务 ID
        id: String,
    },
}

/// 轻量级列表条目（用于 JSON 输出）。
#[derive(Serialize)]
struct TaskListItem {
    id: String,
    task_description: String,
    progress: String,
    created_at: String,
}

pub async fn run(cmd: TaskCmd, store: &TaskStore, json: bool) -> Result<()> {
    match cmd.action {
        TaskAction::Create { description, context } => {
            let task = task_tools::initialize_task(
                store,
                &description,
                context.as_deref(),
                vec![],
                vec![],
                vec![],
                None,
            )?;
            output_result(&task, json);
        }
        TaskAction::List { status, limit } => {
            let tasks = store.list_tasks()?;
            let filtered: Vec<&Task> = match status.as_str() {
                "active" => tasks
                    .iter()
                    .filter(|t| t.checklist.iter().any(|i| !i.done))
                    .collect(),
                "done" => tasks
                    .iter()
                    .filter(|t| !t.checklist.is_empty() && t.checklist.iter().all(|i| i.done))
                    .collect(),
                _ => tasks.iter().collect(),
            };

            let limited: Vec<&Task> = match limit {
                Some(n) => filtered.into_iter().take(n).collect(),
                None => filtered,
            };

            if json {
                let items: Vec<TaskListItem> = limited
                    .iter()
                    .map(|t| {
                        let total = t.checklist.len();
                        let done = t.checklist.iter().filter(|i| i.done).count();
                        TaskListItem {
                            id: t.id.clone(),
                            task_description: t.task_description.clone(),
                            progress: format!("{done}/{total}"),
                            created_at: t.created_at.clone(),
                        }
                    })
                    .collect();
                let json_str = serde_json::to_string_pretty(&items)?;
                println!("{json_str}");
            } else {
                if limited.is_empty() {
                    println!("当前没有任何任务");
                } else {
                    for task in limited {
                        println!("{}", output::format_task_summary(task));
                        println!();
                    }
                }
            }
        }
        TaskAction::Show { id } => {
            let task = store.get_task(&id)?;
            if json {
                output_result(&task, json);
            } else {
                println!("{}", output::format_task_detail(&task));
            }
        }
        TaskAction::Delete { id } => {
            task_tools::clear_task(store, &id)?;
            println!("任务 {id} 已删除");
        }
        TaskAction::Update { id, description } => {
            let task = task_tools::update_task_description(store, &id, &description)?;
            output_result(&task, json);
        }
        TaskAction::Context { id, context } => {
            let task = task_tools::update_context(store, &id, &context)?;
            output_result(&task, json);
        }
        TaskAction::Summary { id } => {
            let summary = task_tools::get_checklist_summary(store, &id)?;
            println!("{summary}");
        }
    }
    Ok(())
}

fn output_result(task: &Task, json: bool) {
    if json {
        let json_str = serde_json::to_string_pretty(task).unwrap();
        println!("{json_str}");
    } else {
        println!("{}", output::format_task_detail(task));
    }
}
