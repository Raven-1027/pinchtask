//! TUI 模块入口 — 交互式终端界面。
//!
//! 基于 ratatui + crossterm 实现，通过 tokio mpsc channel 桥接异步事件循环与渲染。

pub mod app;
pub mod event;
pub mod ui;

use std::path::PathBuf;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;

use app::App;
use event::EventBus;

/// TUI 入口函数。
///
/// 职责：
/// 1. 初始化终端（raw mode + alternate screen）
/// 2. 启动事件总线
/// 3. 运行主循环（事件 → 状态更新 → 渲染）
/// 4. 恢复终端状态
pub async fn run(data_dir: Option<PathBuf>) -> Result<()> {
    // ── 初始化终端 ─────────────────────────────────────────────────────
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // ── 创建应用状态与事件总线 ─────────────────────────────────────────
    let mut app = App::new(data_dir);
    let mut event_bus = EventBus::new();

    // ── 主循环 ─────────────────────────────────────────────────────────
    // TODO: 实现主循环逻辑
    // loop {
    //     terminal.draw(|f| ui::draw(f, &app))?;
    //     match event_bus.next().await {
    //         AppEvent::Quit => break,
    //         event => app.handle_event(event)?,
    //     }
    // }

    // 骨架：直接渲染初始帧后退出
    terminal.draw(|f| ui::draw(f, &app))?;
    // TODO: 替换为真实主循环

    // ── 恢复终端 ───────────────────────────────────────────────────────
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
