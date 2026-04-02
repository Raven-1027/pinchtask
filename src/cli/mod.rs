//! CLI 模块入口。
//!
//! 定义顶层命令行参数解析与扁平化子命令分发逻辑。
//! 无子命令时默认启动 MCP 服务器（serve）。

use clap::CommandFactory;
use clap_complete::{generate, shells::Shell};
use std::io;

pub mod server;
pub mod task;
pub mod item;
pub mod note;
pub mod meta;
pub mod resource;
pub mod output;
pub mod resolve;

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

// ---------------------------------------------------------------------------
// 扁平化命令树
// ---------------------------------------------------------------------------

#[derive(Subcommand, Debug)]
enum Commands {
    // ── 任务实体 ──────────────────────────────────────
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

    // ── 清单条目 ──────────────────────────────────────
    /// 添加清单条目
    Add {
        /// 任务 ID（支持短前缀）
        task_id: String,
        /// 条目标题
        title: String,
        /// 详细描述
        #[arg(short, long, default_value = "")]
        description: String,
        /// 上下文与计划
        #[arg(short, long)]
        plan: Option<String>,
    },
    /// 切换清单条目完成/未完成状态
    Check {
        /// 任务 ID（支持短前缀）
        task_id: String,
        /// 条目索引
        index: usize,
    },
    /// 编辑清单条目
    EditItem {
        /// 任务 ID（支持短前缀）
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
        plan: Option<String>,
        /// 标记为已完成
        #[arg(long, conflicts_with = "undone")]
        done: bool,
        /// 标记为未完成
        #[arg(long, conflicts_with = "done")]
        undone: bool,
    },
    /// 移动清单条目顺序
    Mv {
        /// 任务 ID（支持短前缀）
        task_id: String,
        /// 源索引
        from: usize,
        /// 目标索引
        to: usize,
    },
    /// 删除清单条目
    RmItem {
        /// 任务 ID（支持短前缀）
        task_id: String,
        /// 条目索引
        index: usize,
    },
    /// 查看清单进度摘要
    Summary {
        /// 任务 ID（支持短前缀）
        task_id: String,
    },

    // ── 其他维度 ──────────────────────────────────────
    /// 添加笔记
    Note {
        /// 任务 ID（支持短前缀）
        task_id: String,
        /// 笔记内容
        content: String,
    },
    /// 设置标签和元数据
    Tag {
        /// 任务 ID（支持短前缀）
        task_id: String,
        /// 标签（逗号分隔）
        tags: String,
        /// 优先级 (high / medium / low)
        #[arg(long)]
        priority: Option<String>,
        /// 预计完成时间（ISO 8601）
        #[arg(long)]
        eta: Option<String>,
    },
    /// 添加资源引用
    Link {
        /// 任务 ID（支持短前缀）
        task_id: String,
        /// 资源名称
        #[arg(long)]
        name: String,
        /// 资源 URL 或文件路径
        #[arg(long)]
        url: String,
        /// 资源描述
        #[arg(short, long)]
        description: Option<String>,
    },

    // ── 服务 ──────────────────────────────────────────
    /// 启动 MCP 服务器（stdio 传输）
    Serve,

    /// 生成 shell 补全脚本
    Completion {
        /// 目标 shell (bash, zsh, fish, powershell, elvish)
        shell: Shell,
    },
}

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// 入口函数
// ---------------------------------------------------------------------------

/// CLI 入口函数。
pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    let log_level = resolve_log_level(&cli);
    init_logging(&log_level);

    let json_output = cli.json;
    let data_dir = cli.data_dir;

    // 当没有指定子命令时，默认执行 serve
    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            server::run(data_dir).await?;
            return Ok(());
        }
    };

    match command {
        // 任务实体
        Commands::New { description, context } => {
            let store = TaskStore::new(data_dir)?;
            task::run(&task::TaskCommand::New { description, context }, &store, json_output)
        }
        Commands::Ls {
            all,
            done,
            long,
            limit,
            sort,
        } => {
            let store = TaskStore::new(data_dir)?;
            task::run(&task::TaskCommand::Ls { all, done, long, limit, sort }, &store, json_output)
        }
        Commands::Show { id } => {
            let store = TaskStore::new(data_dir)?;
            task::run(&task::TaskCommand::Show { id }, &store, json_output)
        }
        Commands::Edit {
            id,
            description,
            context,
        } => {
            let store = TaskStore::new(data_dir)?;
            task::run(
                &task::TaskCommand::Edit { id, description, context },
                &store,
                json_output,
            )
        }
        Commands::Rm { id } => {
            let store = TaskStore::new(data_dir)?;
            task::run(&task::TaskCommand::Rm { id }, &store, json_output)
        }

        // 清单条目
        Commands::Add {
            task_id,
            title,
            description,
            plan,
        } => {
            let store = TaskStore::new(data_dir)?;
            item::run(
                &item::ItemCommand::Add { task_id, title, description, plan },
                &store,
                json_output,
            )
        }
        Commands::Check { task_id, index } => {
            let store = TaskStore::new(data_dir)?;
            item::run(&item::ItemCommand::Check { task_id, index }, &store, json_output)
        }
        Commands::EditItem {
            task_id,
            index,
            title,
            description,
            plan,
            done,
            undone,
        } => {
            let store = TaskStore::new(data_dir)?;
            item::run(
                &item::ItemCommand::EditItem {
                    task_id,
                    index,
                    title,
                    description,
                    plan,
                    done,
                    undone,
                },
                &store,
                json_output,
            )
        }
        Commands::Mv {
            task_id,
            from,
            to,
        } => {
            let store = TaskStore::new(data_dir)?;
            item::run(
                &item::ItemCommand::Mv { task_id, from, to },
                &store,
                json_output,
            )
        }
        Commands::RmItem { task_id, index } => {
            let store = TaskStore::new(data_dir)?;
            item::run(&item::ItemCommand::RmItem { task_id, index }, &store, json_output)
        }
        Commands::Summary { task_id } => {
            let store = TaskStore::new(data_dir)?;
            item::run(&item::ItemCommand::Summary { task_id }, &store, json_output)
        }

        // 其他维度
        Commands::Note { task_id, content } => {
            let store = TaskStore::new(data_dir)?;
            note::run(
                &note::NoteCommand::Note { task_id, content },
                &store,
                json_output,
            )
        }
        Commands::Tag {
            task_id,
            tags,
            priority,
            eta,
        } => {
            let store = TaskStore::new(data_dir)?;
            meta::run(
                &meta::MetaCommand::Tag {
                    task_id,
                    tags,
                    priority,
                    eta,
                },
                &store,
                json_output,
            )
        }
        Commands::Link {
            task_id,
            name,
            url,
            description,
        } => {
            let store = TaskStore::new(data_dir)?;
            resource::run(
                &resource::ResourceCommand::Link {
                    task_id,
                    name,
                    url,
                    description,
                },
                &store,
                json_output,
            )
        }

        // 服务
        Commands::Serve => server::run(data_dir).await,
        Commands::Completion { shell } => {
            let mut cli = Cli::command();
            generate(shell, &mut cli, "mcp-pinchtask", &mut io::stdout());
            Ok(())
        }
    }
}
