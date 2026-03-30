//! Stdio 传输层。
//!
//! 通过 stdin/stdout 进行 JSON-RPC 消息的读写，
//! 支持两种 MCP 标准传输格式：
//! - **换行分隔 JSON**（默认）：每行一个 JSON 对象
//! - **Content-Length 头**：以 `Content-Length: N\r\n\r\n` 开头，后跟 N 字节的 JSON

use std::io::{BufRead, Read, Write};

use anyhow::{Context, Result};
use serde_json::Value;

use crate::protocol::{JsonRpcRequest, JsonRpcResponse};

/// 基于 stdin/stdout 的 JSON-RPC 传输层。
pub struct StdioTransport;

impl StdioTransport {
    /// 创建新的 StdioTransport 实例。
    pub fn new() -> Self {
        Self
    }

    /// 从 stdin 读取一条 JSON-RPC 请求。
    ///
    /// 自动检测传输格式：
    /// - 如果首字符为 `{`，按换行分隔 JSON 解析
    /// - 如果首字符为数字或字母（Content-Length 头），按头格式解析
    ///
    /// 遇到 EOF 时返回 `None`，表示客户端已关闭连接。
    pub async fn read_request(&self) -> Result<Option<JsonRpcRequest>> {
        tokio::task::spawn_blocking(|| {
            let stdin = std::io::stdin();
            let mut handle = stdin.lock();

            // 先窥探第一个字符以判断传输格式
            let mut first_byte = [0u8; 1];
            let n = handle.read(&mut first_byte).context("读取 stdin 失败")?;
            if n == 0 {
                return Ok(None); // EOF
            }

            let json_str = match first_byte[0] {
                // 首字符为 '{'，按换行分隔 JSON 格式
                b'{' => {
                    let mut rest = String::from("{");
                    let bytes_read = handle.read_line(&mut rest).context("读取行失败")?;
                    if bytes_read == 0 && !rest.ends_with('\n') {
                        // 可能是最后一行没有换行符
                    }
                    rest.trim_end().to_owned()
                }
                // 首字符为其他值（字母/数字），按 Content-Length 头格式解析
                _ => {
                    // 读取完整的 header 行
                    let mut header_line = String::new();
                    header_line.push(first_byte[0] as char);
                    handle.read_line(&mut header_line).context("读取头失败")?;
                    let header_line = header_line.trim();

                    // 解析 Content-Length 值
                    let content_length = header_line
                        .strip_prefix("Content-Length:")
                        .or_else(|| header_line.strip_prefix("Content-Length :"))
                        .map(|v| v.trim())
                        .and_then(|v| v.parse::<usize>().ok())
                        .ok_or_else(|| {
                            anyhow::anyhow!("无法解析 Content-Length 头: {header_line}")
                        })?;

                    // 读取并跳过空行（\r\n 分隔符）
                    let mut sep = [0u8; 2];
                    handle.read_exact(&mut sep).context("读取分隔符失败")?;

                    // 读取指定长度的 JSON 体
                    let mut buf = vec![0u8; content_length];
                    handle.read_exact(&mut buf).context("读取消息体失败")?;
                    String::from_utf8(buf).context("消息体不是有效的 UTF-8")?
                }
            };

            let request: JsonRpcRequest =
                serde_json::from_str(&json_str).context("解析 JSON-RPC 请求失败")?;
            Ok(Some(request))
        })
        .await
        .context("read_request 任务执行失败")?
    }

    /// 将 JSON-RPC 响应写入 stdout（使用换行分隔格式）。
    pub async fn write_response(&self, response: &JsonRpcResponse) -> Result<()> {
        let json = serde_json::to_string(response).context("序列化 JSON-RPC 响应失败")?;
        self.write_line(&json).await
    }

    /// 将 JSON-RPC 通知写入 stdout。
    pub async fn write_notification(&self, method: &str, params: Value) -> Result<()> {
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        let json =
            serde_json::to_string(&notification).context("序列化 JSON-RPC 通知失败")?;
        self.write_line(&json).await
    }

    /// 写入一行到 stdout（带换行符）。
    async fn write_line(&self, line: &str) -> Result<()> {
        tokio::task::spawn_blocking(move || {
            let mut stdout = std::io::stdout().lock();
            writeln!(stdout, "{line}").context("写入 stdout 失败")?;
            stdout.flush().context("flush stdout 失败")
        })
        .await
        .context("write_line 任务执行失败")?
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}
