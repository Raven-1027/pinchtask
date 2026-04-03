//! 事件定义与异步事件总线。
//!
//! 使用 tokio::sync::mpsc channel 桥接 crossterm 终端事件与应用主循环。
//! crossterm 事件在独立任务中轮询，通过 channel 发送到主循环。

use crossterm::event::KeyEvent;

// ── 应用事件 ───────────────────────────────────────────────────────────────

/// TUI 主循环处理的事件类型。
#[derive(Debug)]
pub enum AppEvent {
    /// 键盘输入
    Key(KeyEvent),
    /// 终端尺寸变化
    Resize(u16, u16),
    /// 请求退出 TUI
    Quit,
}

// ── 事件总线 ───────────────────────────────────────────────────────────────

/// 事件总线：从 crossterm 接收终端事件并通过 channel 转发。
///
/// 架构：
/// - 独立 tokio task 轮询 crossterm::event::poll()
/// - 将原始事件转换为 AppEvent 并发送到 mpsc channel
/// - 主循环通过 EventBus::next() 接收
pub struct EventBus {
    // TODO: 添加 sender/receiver 字段
    // rx: tokio::sync::mpsc::UnboundedReceiver<AppEvent>,
}

impl EventBus {
    /// 创建新的事件总线。
    ///
    /// 启动后台轮询任务，监听 crossterm 终端事件。
    /// TODO: 实现 crossterm 事件轮询 + channel 发送
    pub fn new() -> Self {
        Self {
            // rx: ...,
        }
    }

    /// 等待并返回下一个应用事件。
    ///
    /// TODO: 实现 channel 接收
    /// async fn next(&mut self) -> AppEvent {
    ///     self.rx.recv().await.expect("event channel closed")
    /// }
    pub async fn next(&mut self) -> AppEvent {
        // TODO: 替换为真实 channel 接收
        // 骨架阶段直接返回 Quit 以便主循环能结束
        AppEvent::Quit
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
