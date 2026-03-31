//! CLI 模块入口。
//!
//! 定义顶层命令行参数解析与子命令分发逻辑。

pub mod server;
pub mod task;
pub mod checklist;
pub mod note;
pub mod resource;
pub mod metadata;
pub mod output;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::store::TaskStore;

/// mcp-pinchtask: 基于 MCP 的任务管理工具
#[derive(Parser, Debug)]
#[command(name = "mcp-pinchtask", version, about, subcommand_required = false)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// 数据存储目录路径（默认: ~/.mcp-pinchtask）
    #[arg(long, global = true, env = "PINCHTASK_DATA_DIR")]
    data_dir: Option<PathBuf>,

    /// 日志级别 (trace, debug, info, warn, error)
    #[arg(long, global = true, env = "PINCHTASK_LOG_LEVEL")]
    log_level: Option<String>,

    /// 详细输出（等价于 --log-level debug）
    #[arg(short, long, global = true, conflicts_with = "quiet")]
    verbose: bool,

    /// 安静模式（等价于 --log-level error）
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    quiet: bool,

    /// 以 JSON 格式输出（适用于查询类子命令）
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// 启动 MCP 服务器（stdio 传输）
    Server,
    /// 任务管理
    Task(task::TaskCmd),
    /// 清单管理
    Checklist(checklist::ChecklistCmd),
    /// 笔记管理
    Note(note::NoteCmd),
    /// 资源管理
    Resource(resource::ResourceCmd),
    /// 元数据管理
    Metadata(metadata::MetadataCmd),
}

/// 解析日志级别。
fn resolve_log_level(cli: &Cli) -> String {
    if cli.verbose {
        "debug".to_owned()
    } else if cli.quiet {
        "error".to_owned()
    } else {
        cli.log_level.clone().unwrap_or_else(|| "warn".to_owned())
    }
}

/// 初始化日志。
fn init_logging(log_level: &str) {
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(log_level)),
        )
        .init();
}

/// CLI 入口函数。
pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    let log_level = resolve_log_level(&cli);
    init_logging(&log_level);

    let json_output = cli.json;
    let data_dir = cli.data_dir;

    // 当没有指定子命令时，默认执行 server
    let command = match cli.command {
        Some(cmd) => cmd,
        None => Commands::Server,
    };

    match command {
        Commands::Server => server::run(data_dir).await,
        Commands::Task(cmd) => {
            let store = TaskStore::new(data_dir)?;
            task::run(cmd, &store, json_output).await
        }
        Commands::Checklist(cmd) => {
            let store = TaskStore::new(data_dir)?;
            checklist::run(cmd, &store, json_output).await
        }
        Commands::Note(cmd) => {
            let store = TaskStore::new(data_dir)?;
            note::run(cmd, &store, json_output).await
        }
        Commands::Resource(cmd) => {
            let store = TaskStore::new(data_dir)?;
            resource::run(cmd, &store, json_output).await
        }
        Commands::Metadata(cmd) => {
            let store = TaskStore::new(data_dir)?;
            metadata::run(cmd, &store, json_output).await
        }
    }
}
