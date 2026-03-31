//! 清单管理子命令。

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::models::task::Task;
use crate::store::TaskStore;
use crate::tools::task as task_tools;

use super::output;

/// 清单管理命令。
#[derive(Args, Debug)]
pub struct ChecklistCmd {
    #[command(subcommand)]
    pub action: ChecklistAction,
}

#[derive(Subcommand, Debug)]
pub enum ChecklistAction {
    /// 添加清单条目
    Add {
        /// 任务 ID
        task_id: String,
        /// 条目标题
        #[arg(short, long)]
        title: String,
        /// 详细描述
        #[arg(short, long, default_value = "")]
        description: String,
        /// 上下文与计划
        #[arg(short, long)]
        plan: Option<String>,
    },
    /// 更新清单条目
    Update {
        /// 任务 ID
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
        plan: Option<Option<String>>,
        /// 完成状态
        #[arg(short, long)]
        done: Option<bool>,
    },
    /// 标记完成
    Done {
        /// 任务 ID
        task_id: String,
        /// 条目索引
        index: usize,
    },
    /// 标记未完成
    Undone {
        /// 任务 ID
        task_id: String,
        /// 条目索引
        index: usize,
    },
    /// 重排顺序
    Reorder {
        /// 任务 ID
        task_id: String,
        /// 源索引
        #[arg(long)]
        from: usize,
        /// 目标索引
        #[arg(long)]
        to: usize,
    },
    /// 删除条目
    Remove {
        /// 任务 ID
        task_id: String,
        /// 条目索引
        index: usize,
    },
}

pub async fn run(cmd: ChecklistCmd, store: &TaskStore, json: bool) -> Result<()> {
    match cmd.action {
        ChecklistAction::Add {
            task_id,
            title,
            description,
            plan,
        } => {
            let task =
                task_tools::add_checklist_item(store, &task_id, &title, &description, plan.as_deref())?;
            output_task(&task, json);
        }
        ChecklistAction::Update {
            task_id,
            index,
            title,
            description,
            plan,
            done,
        } => {
            // plan: Option<Option<String>> 需要特殊处理
            let plan_ref: Option<Option<&str>> = plan.as_ref().map(|p| p.as_deref());
            let task = task_tools::update_checklist_item(
                store,
                &task_id,
                index,
                title.as_deref(),
                description.as_deref(),
                plan_ref,
                done,
            )?;
            output_task(&task, json);
        }
        ChecklistAction::Done { task_id, index } => {
            let task = task_tools::mark_task_done(store, &task_id, index)?;
            output_task(&task, json);
        }
        ChecklistAction::Undone { task_id, index } => {
            let task = task_tools::mark_task_undone(store, &task_id, index)?;
            output_task(&task, json);
        }
        ChecklistAction::Reorder { task_id, from, to } => {
            let task = task_tools::reorder_checklist_item(store, &task_id, from, to)?;
            output_task(&task, json);
        }
        ChecklistAction::Remove { task_id, index } => {
            let task = task_tools::remove_checklist_item(store, &task_id, index)?;
            output_task(&task, json);
        }
    }
    Ok(())
}

fn output_task(task: &Task, json: bool) {
    if json {
        let json_str = serde_json::to_string_pretty(task).unwrap();
        println!("{json_str}");
    } else {
        println!("{}", output::format_checklist_summary(task));
    }
}
