//! 事件定义与异步事件总线。
//!
//! 使用 tokio::sync::mpsc channel 桥接 crossterm 终端事件与应用主循环。
//! crossterm 事件在独立 tokio task 中轮询，通过 channel 发送到主循环。

use std::time::Duration;

use crossterm::event::{Event as CrosstermEvent, KeyEvent};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::models::task::Task;

// ── 轮询超时 ───────────────────────────────────────────────────────────────

/// crossterm 事件轮询超时（毫秒）。
const POLL_TIMEOUT_MS: u64 = 250;

// ── 异步操作结果 ───────────────────────────────────────────────────────────

/// 后台异步任务完成后的结果类型。
///
/// 用于将 store 操作的结果回传给主循环，更新 App 状态。
/// 携带最新 Task 数据以刷新详情视图。
#[derive(Debug)]
pub enum Action {
    /// 任务列表加载完成
    TasksLoaded(Vec<Task>),
    /// 单个任务加载完成（查看详情）
    TaskDetailLoaded(Task),
    /// 清单条目完成状态已切换（携带更新后的任务）
    ItemToggled(Task),
    /// 清单条目已添加（携带更新后的任务）
    ItemAdded(Task),
    /// 清单条目已移除（携带更新后的任务）
    ItemRemoved(Task),
    /// 清单条目已编辑（携带更新后的任务）
    ItemEdited(Task),
    /// 清单条目已重排（携带更新后的任务）
    ItemReordered(Task),
    /// 任务已删除（携带被删除任务的 ID）
    TaskDeleted(String),
    /// 任务已创建（携带新任务）
    TaskCreated(Task),
    /// 任务已更新（携带更新后的任务）
    TaskUpdated(Task),
    /// 笔记已添加（携带更新后的任务）
    NoteAdded(Task),
    /// 笔记已删除（携带更新后的任务）
    NoteDeleted(Task),
    /// 资源已添加（携带更新后的任务）
    ResourceAdded(Task),
    /// 资源已删除（携带更新后的任务）
    ResourceDeleted(Task),
    /// 操作出错
    Error(String),
}

// ── 应用事件 ───────────────────────────────────────────────────────────────

/// TUI 主循环处理的事件类型。
#[derive(Debug)]
pub enum AppEvent {
    /// 键盘输入
    Key(KeyEvent),
    /// 终端尺寸变化
    Resize(u16, u16),
    /// 异步操作结果
    Action(Action),
}

// ── 事件总线 ───────────────────────────────────────────────────────────────

/// 事件总线：从 crossterm 接收终端事件并通过 channel 转发。
///
/// 架构：
/// - 独立 tokio task 轮询 crossterm::event::poll()
/// - 将原始事件转换为 AppEvent 并发送到 mpsc channel
/// - 主循环通过 EventBus::next() 接收
pub struct EventBus {
    /// channel 接收端，主循环从此取出事件
    rx: UnboundedReceiver<AppEvent>,
    /// channel 发送端句柄（供外部发送 Action 事件）
    tx: UnboundedSender<AppEvent>,
}

impl EventBus {
    /// 创建新的事件总线。
    ///
    /// 启动后台 tokio task，以 POLL_TIMEOUT_MS 间隔轮询 crossterm 终端事件，
    /// 将 KeyEvent / Resize 转换为 AppEvent 并通过 channel 发送。
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let sender = tx.clone();

        tokio::spawn(async move {
            loop {
                // 以固定超时轮询，避免阻塞
                if crossterm::event::poll(Duration::from_millis(POLL_TIMEOUT_MS))
                    .unwrap_or(false)
                {
                    match crossterm::event::read() {
                        Ok(event) => {
                            let app_event = match event {
                                CrosstermEvent::Key(key) => AppEvent::Key(key),
                                CrosstermEvent::Resize(w, h) => AppEvent::Resize(w, h),
                                // 鼠标事件暂不处理
                                _ => continue,
                            };
                            // channel 关闭说明主循环已退出，结束轮询
                            if sender.send(app_event).is_err() {
                                break;
                            }
                        }
                        Err(_) => {
                            // crossterm 读取失败，发送端已关闭时退出
                            break;
                        }
                    }
                }
            }
        });

        Self { rx, tx }
    }

    /// 等待并返回下一个应用事件。
    ///
    /// 阻塞当前 tokio task 直到有事件可用。
    /// 返回 `None` 表示 channel 已关闭（所有发送端已 drop）。
    pub async fn next(&mut self) -> Option<AppEvent> {
        self.rx.recv().await
    }

    /// 返回 channel 发送端的克隆引用。
    ///
    /// 用于从主循环向自身发送 Action 事件（如异步操作完成后回传结果）。
    pub fn sender(&self) -> UnboundedSender<AppEvent> {
        self.tx.clone()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
