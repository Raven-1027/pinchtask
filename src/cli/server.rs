//! MCP 服务器子命令。

use std::path::PathBuf;

use anyhow::Result;

/// 启动 MCP 服务器。
pub async fn run(data_dir: Option<PathBuf>) -> Result<()> {
    tracing::info!("mcp-pinchtask server starting");

    let store = crate::store::TaskStore::new(data_dir).await?;
    tracing::info!("TaskStore initialized");

    let server = crate::server::McpServer::new(store);

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

    tracing::info!("mcp-pinchtask server shutting down");
    Ok(())
}
