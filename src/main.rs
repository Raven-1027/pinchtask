use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

/// mcp-pinchtask: A Rust-based MCP task manager
#[derive(Parser, Debug)]
#[command(name = "mcp-pinchtask", version, about)]
struct Args {
    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info", env = "PINCHTASK_LOG_LEVEL")]
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

    // Start the MCP server
    mcp_pinchtask::server::run().await?;

    tracing::info!("mcp-pinchtask shutting down");
    Ok(())
}
