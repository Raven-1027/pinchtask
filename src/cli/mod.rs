//! CLI 模块入口 — 顶层命令行参数解析与扁平化子命令分发。

use std::io;
use std::path::PathBuf;

use anyhow::Result;
use clap::{Args as ClapArgs, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, shells::Shell};

use crate::store::TaskStore;

pub mod item;
pub mod logging;
pub mod meta;
pub mod note;
pub mod output;
pub mod resolve;
pub mod resource;
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

// ── 跨模块 Args ────────────────────────────────────────────────────────────

/// 编辑任务或清单条目
#[derive(ClapArgs, Debug)]
struct EditArgs {
    /// 任务 ID（支持短前缀）
    id: String,
    /// 清单条目索引（不传则编辑任务本身）
    index: Option<usize>,
    /// 新的任务描述（任务级）或条目描述（条目级）
    #[arg(short, long)]
    description: Option<String>,
    /// 新的共享上下文（仅任务级）
    #[arg(short, long)]
    context: Option<String>,
    /// 新标题（仅条目级）
    #[arg(short, long)]
    title: Option<String>,
    /// 新计划（仅条目级）
    #[arg(short, long)]
    plan: Option<String>,
    /// 标记为已完成（仅条目级）
    #[arg(long, conflicts_with = "undone")]
    done: bool,
    /// 标记为未完成（仅条目级）
    #[arg(long, conflicts_with = "done")]
    undone: bool,
    /// 优先级 (high / medium / low)（仅任务级）
    #[arg(long)]
    priority: Option<String>,
    /// 标签，逗号分隔（仅任务级）
    #[arg(long)]
    tags: Option<String>,
    /// 预计完成时间，ISO 8601（仅任务级）
    #[arg(long)]
    eta: Option<String>,
}

/// 删除任务或清单条目
#[derive(ClapArgs, Debug)]
struct RmArgs {
    /// 任务 ID（支持短前缀）
    id: String,
    /// 清单条目索引（不传则删除任务本身）
    index: Option<usize>,
}

/// Shell 补全脚本参数
#[derive(ClapArgs, Debug)]
struct CompletionArgs {
    /// 目标 shell (bash, zsh, fish, powershell, elvish)
    shell: Shell,
}

// ── 扁平化命令树 ───────────────────────────────────────────────────────────

#[derive(Subcommand, Debug)]
enum Commands {
    /// 创建新任务
    New(task::NewArgs),
    /// 列出任务
    Ls(task::LsArgs),
    /// 查看任务详情
    Show(task::ShowArgs),
    /// 编辑任务或清单条目
    Edit(EditArgs),
    /// 删除任务或清单条目
    Rm(RmArgs),
    /// 添加清单条目
    Add(item::AddArgs),
    /// 切换清单条目完成/未完成状态
    Check(item::CheckArgs),
    /// 移动清单条目顺序
    Mv(item::MvArgs),
    /// 查看清单进度摘要
    Summary(item::SummaryArgs),
    /// 添加笔记
    Note(note::NoteArgs),
    /// 设置标签和元数据
    Tag(meta::TagArgs),
    /// 添加资源引用
    Link(resource::LinkArgs),
    /// 启动 MCP 服务器（stdio 传输）
    Serve,
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

    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            server::run(data_dir).await?;
            return Ok(());
        }
    };

    // Serve 和 Completion 不需要 TaskStore，直接处理
    match command {
        Commands::Serve => return server::run(data_dir).await,
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
        Commands::New(args) => task::run_new(args, &store, json).await,
        Commands::Ls(args) => task::run_ls(args, &store, json).await,
        Commands::Show(args) => task::run_show(args, &store, json).await,
        Commands::Edit(args) => match args.index {
            Some(idx) => {
                // metadata 字段仅用于任务级编辑，带 index 时禁止传入
                if args.priority.is_some() || args.tags.is_some() || args.eta.is_some() {
                    anyhow::bail!("--priority / --tags / --eta 仅用于任务级编辑，不能与 --index 同时使用");
                }
                item::run_edit_item(
                    &store, json, &args.id, idx,
                    args.title.as_deref(), args.description.as_deref(),
                    args.plan.as_deref(), args.done, args.undone,
                ).await
            }
            None => task::run_edit(
                &store, json, &args.id,
                args.description.as_deref(),
                args.context.as_deref(),
                args.priority.as_deref(),
                args.tags.as_deref(),
                args.eta.as_deref(),
            ).await,
        },
        Commands::Rm(args) => match args.index {
            Some(idx) => item::run_rm_item(&store, json, &args.id, idx).await,
            None => task::run_rm(&store, json, &args.id).await,
        },
        Commands::Add(args) => item::run_add(args, &store, json).await,
        Commands::Check(args) => item::run_check(args, &store, json).await,
        Commands::Mv(args) => item::run_mv(args, &store, json).await,
        Commands::Summary(args) => item::run_summary(args, &store, json).await,
        Commands::Note(args) => note::run_note(args, &store, json).await,
        Commands::Tag(args) => meta::run_tag(args, &store, json).await,
        Commands::Link(args) => resource::run_link(args, &store, json).await,
        // 已在上方 match 中提前返回，不会到达
        Commands::Serve | Commands::Completion(_) => unreachable!(),
    }
}
