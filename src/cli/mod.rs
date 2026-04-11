//! CLI 模块入口 — 顶层命令行参数解析与嵌套子命令分发。

use std::io;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args as ClapArgs, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, shells::Shell};

use crate::store::TaskStore;

pub mod item;
pub mod link;
pub mod logging;
pub mod note;
pub mod output;
pub mod project;
pub mod resolve;
pub mod server;
pub mod task;

// ── 顶层参数 ───────────────────────────────────────────────────────────────

/// pinchtask: 基于 MCP 的任务管理工具
#[derive(Parser, Debug)]
#[command(name = "pinchtask", version, about, subcommand_required = false)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    /// 数据存储目录路径（默认: ~/.pinchtask）
    #[arg(short = 'D', long, global = true, env = "PINCHTASK_DATA_DIR")]
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

// ── Shell 补全参数 ─────────────────────────────────────────────────────────

/// Shell 补全脚本参数
#[derive(ClapArgs, Debug)]
struct CompletionArgs {
    /// 目标 shell (bash, zsh, fish, powershell, elvish)
    shell: Shell,
}

// ── 嵌套命令树 ─────────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
enum Commands {
    /// 任务管理
    #[command(subcommand)]
    Task(task::TaskCommands),
    /// 清单条目管理
    #[command(subcommand)]
    Item(item::ItemCommands),
    /// 笔记管理
    #[command(subcommand)]
    Note(note::NoteCommands),
    /// 资源引用管理
    #[command(subcommand)]
    Link(link::LinkCommands),
    /// 项目管理
    #[command(subcommand)]
    Project(project::ProjectCommands),
    /// 启动 MCP 服务器（stdio 传输）
    Serve,
    /// 启动交互式 TUI 界面
    #[cfg(feature = "tui")]
    Tui,
    /// 生成 shell 补全脚本
    Completion(CompletionArgs),
}

// ── 入口函数 ───────────────────────────────────────────────────────────────

/// CLI 入口函数。
pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    logging::init(cli.verbose, cli.quiet, cli.log_level.as_deref());

    let json = cli.json;
    let data_dir = cli.data_dir;

    // 从工作区 .pinchproject 文件发现项目 ID
    let workspace_project_id = crate::core::discover_project_id();
    if let Some(ref pid) = workspace_project_id {
        tracing::info!(project_id = %pid, "auto-detected workspace project from .pinchproject");
    }

    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            server::run(data_dir).await?;
            return Ok(());
        }
    };

    // Serve、Tui 和 Completion 不需要 TaskStore，直接处理
    match command {
        Commands::Serve => return server::run(data_dir).await,
        #[cfg(feature = "tui")]
        Commands::Tui => return crate::tui::run(data_dir, workspace_project_id).await,
        Commands::Completion(args) => {
            let mut cmd = Cli::command();
            generate(args.shell, &mut cmd, "pinchtask", &mut io::stdout());
            return Ok(());
        }
        _ => {}
    }

    // 其余命令需要 TaskStore，创建一次供所有分支使用
    let store = TaskStore::new(data_dir).await?;

    match &command {
        Commands::Task(cmd) => {
            task::run_task(cmd, &store, json, workspace_project_id.as_deref()).await
        }
        Commands::Item(cmd) => item::run_item(cmd, &store, json).await,
        Commands::Note(cmd) => note::run_note(cmd, &store, json).await,
        Commands::Link(cmd) => link::run_link(cmd, &store, json).await,
        Commands::Project(cmd) => project::run_project(cmd, &store, json).await,
        // 已在上方 match 中提前返回，不会到达
        Commands::Serve => unreachable!(),
        #[cfg(feature = "tui")]
        Commands::Tui => unreachable!(),
        Commands::Completion(_) => unreachable!(),
    }
}
