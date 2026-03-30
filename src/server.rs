//! MCP server entry point.
//!
//!

use anyhow::Result;

/// Start the MCP server and listen for incoming connections.
///
/// Currently a placeholder — will be implemented with the MCP protocol
/// transport layer (stdio / HTTP SSE) in subsequent iterations.
pub async fn run() -> Result<()> {
    tracing::info!("MCP server placeholder — not yet connected");
    // TODO: initialize MCP transport and register tools
    // TODO: listen for requests and dispatch to tool handlers
    Ok(())
}
