//! mcp-pinchtask: MCP 任务管理服务器。
//!
//! 基于 Model Context Protocol (MCP) 的任务管理工具，
//! 通过 stdio 与 MCP 客户端通信。

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

/// mcp-pinchtask: A Rust-based MCP task manager
#[derive(Parser, Debug)]
#[command(name = "mcp-pinchtask", version, about)]
struct Args {
    /// 数据存储目录路径（默认: ~/.mcp-pinchtask）
    #[arg(long, env = "PINCHTASK_DATA_DIR")]
    data_dir: Option<PathBuf>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "warn", env = "PINCHTASK_LOG_LEVEL")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&args.log_level)),
        )
        .init();

    tracing::info!("mcp-pinchtask starting");

    // Create TaskStore
    let store = mcp_pinchtask::store::TaskStore::new(args.data_dir)?;
    tracing::info!("TaskStore initialized");

    // Create and run MCP server
    let server = mcp_pinchtask::server::McpServer::new(store);

    // Handle Ctrl+C for graceful shutdown
    let ctrlc = tokio::signal::ctrl_c();
    let server_run = server.run();

    tokio::select! {
        result = server_run => {
            result?;
        }
        _ = ctrlc => {
            tracing::info!("Received Ctrl+C, shutting down gracefully");
        }
    }

    tracing::info!("mcp-pinchtask shutting down");
    Ok(())
}
