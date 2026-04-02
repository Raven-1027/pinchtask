//! 元数据管理子命令。

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::models::task::TaskMetadata;
use crate::store::TaskStore;
use crate::tools::task as task_tools;

#[derive(Args, Debug)]
pub struct MetadataCmd {
    #[command(subcommand)]
    command: MetadataAction,
}

#[derive(Subcommand, Debug)]
enum MetadataAction {
    /// 更新元数据
    Update {
        /// 任务 ID
        task_id: String,
        /// 标签（逗号分隔）
        #[arg(short, long)]
        tags: Option<String>,
        /// 优先级 (high, medium, low)
        #[arg(short, long)]
        priority: Option<String>,
        /// 预计完成时间
        #[arg(short, long)]
        eta: Option<String>,
    },
}

pub async fn run(cmd: MetadataCmd, store: &TaskStore, json: bool) -> Result<()> {
    match cmd.command {
        MetadataAction::Update {
            task_id,
            tags,
            priority,
            eta,
        } => {
            // 获取现有任务以合并元数据
            let existing = store.get_task(&task_id)?;
            let mut metadata = existing.metadata.unwrap_or(TaskMetadata {
                tags: None,
                priority: None,
                estimated_completion_time: None,
            });

            if let Some(ref t) = tags {
                metadata.tags = Some(
                    t.split(',')
                        .map(|s| s.trim().to_owned())
                        .filter(|s| !s.is_empty())
                        .collect(),
                );
            }
            if let Some(ref p) = priority {
                metadata.priority = Some(p.clone());
            }
            if let Some(ref e) = eta {
                metadata.estimated_completion_time = Some(e.clone());
            }

            let task = task_tools::update_metadata(store, &task_id, metadata)?;
            if json {
                let json_str = serde_json::to_string_pretty(&task)?;
                println!("{json_str}");
            } else {
                println!("元数据已更新");
            }
        }
    }
    Ok(())
}
