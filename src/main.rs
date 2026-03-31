//! mcp-pinchtask: MCP 任务管理工具。
//!
//! 基于 Model Context Protocol (MCP) 的任务管理工具，
//! 支持 CLI 本地操作与 MCP 服务器模式。

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    mcp_pinchtask::cli::run().await
}
