//! MCP 服务器子命令。

use std::path::PathBuf;

use anyhow::Result;
use rmcp::ServiceExt;

/// 启动 MCP 服务器。
pub async fn run(data_dir: Option<PathBuf>) -> Result<()> {
    tracing::info!("mcp-pinchtask server starting");

    let store = crate::store::TaskStore::new(data_dir).await?;
    tracing::info!("TaskStore initialized");

    let server = crate::server::PinchTaskServer::new(store);

    let ctrlc = tokio::signal::ctrl_c();
    let server_run = server.serve(rmcp::transport::stdio());

    tokio::select! {
        result = server_run => {
            match result {
                Ok(service) => {
                    match service.waiting().await {
                        Ok(reason) => tracing::info!("Server quit: {reason:?}"),
                        Err(e) => tracing::error!("Server waiting error: {e}"),
                    }
                }
                Err(e) => {
                    tracing::error!("Server error: {e}");
                    return Err(anyhow::anyhow!("{e}"));
                }
            }
        }
        _ = ctrlc => {
            tracing::info!("Received Ctrl+C, shutting down gracefully");
        }
    }

    tracing::info!("mcp-pinchtask server shutting down");
    Ok(())
}
