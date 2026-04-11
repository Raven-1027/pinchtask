//! pinchtask: MCP 任务管理工具。
//!
//! 基于 Model Context Protocol (MCP) 的任务管理工具，
//! 支持 CLI 本地操作与 MCP 服务器模式。

use std::process::ExitCode;

use pinchtask::store::StoreError;

// ---------------------------------------------------------------------------
// 退出码常量
// ---------------------------------------------------------------------------

/// 一般错误（未被分类的 anyhow 错误）。
const EXIT_ERR_GENERAL: u8 = 1;
/// 任务未找到。
const EXIT_ERR_NOT_FOUND: u8 = 2;
/// IO / 数据损坏 / 配置错误。
const EXIT_ERR_IO: u8 = 3;

// ---------------------------------------------------------------------------
// 入口
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> ExitCode {
    match pinchtask::cli::run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            report_error(&e);
            exit_code_from_error(&e)
        }
    }
}

// ---------------------------------------------------------------------------
// 错误报告
// ---------------------------------------------------------------------------

/// 将 `anyhow::Error` 分类并输出用户友好的错误消息到 stderr。
fn report_error(err: &anyhow::Error) {
    if let Some(store_err) = err.downcast_ref::<StoreError>() {
        match store_err {
            StoreError::NotFound(id) => {
                eprintln!("错误: 任务不存在: {id}");
            }
            StoreError::ProjectNotFound(id) => {
                eprintln!("错误: 项目不存在: {id}");
            }
            StoreError::InvalidIdPrefix { .. }
            | StoreError::AmbiguousTaskId { .. }
            | StoreError::AmbiguousProjectId { .. } => {
                eprintln!("错误: {store_err}");
            }
            StoreError::Io(io_err) => {
                eprintln!("错误: 文件操作失败");
                eprintln!("  详情: {io_err}");
                eprintln!("  提示: 请检查数据目录权限和磁盘空间");
            }
            StoreError::Database(db_err) => {
                eprintln!("错误: 数据库操作失败");
                eprintln!("  详情: {db_err}");
                eprintln!("  提示: 可手动检查 ~/.pinchtask/ 下的数据库文件");
            }
        }
    } else {
        // 非存储层错误：输出 anyhow 完整错误链
        eprintln!("错误: {err:#}");
    }
}

/// 根据错误类型返回语义正确的退出码。
fn exit_code_from_error(err: &anyhow::Error) -> ExitCode {
    if let Some(store_err) = err.downcast_ref::<StoreError>() {
        match store_err {
            StoreError::NotFound(_) => ExitCode::from(EXIT_ERR_NOT_FOUND),
            StoreError::ProjectNotFound(_) => ExitCode::from(EXIT_ERR_NOT_FOUND),
            StoreError::InvalidIdPrefix { .. }
            | StoreError::AmbiguousTaskId { .. }
            | StoreError::AmbiguousProjectId { .. } => ExitCode::from(EXIT_ERR_GENERAL),
            StoreError::Io(_) | StoreError::Database(_) => ExitCode::from(EXIT_ERR_IO),
        }
    } else {
        ExitCode::from(EXIT_ERR_GENERAL)
    }
}
