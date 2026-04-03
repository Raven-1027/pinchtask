//! 日志初始化。

use tracing_subscriber::EnvFilter;

/// 根据命令行参数初始化 tracing 日志。
///
/// 优先使用 `RUST_LOG` 环境变量，其次使用 `--verbose` / `--quiet` / `--log-level`。
pub fn init(verbose: bool, quiet: bool, log_level: Option<&str>) {
    let level = if verbose {
        "debug"
    } else if quiet {
        "error"
    } else {
        log_level.unwrap_or("warn")
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level)),
        )
        .init();
}
