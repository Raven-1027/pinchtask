//! TUI 模块入口 — 交互式终端界面。
//!
//! 基于 ratatui + crossterm 实现，通过 tokio mpsc channel 桥接异步事件循环与渲染。

pub mod app;
pub mod event;
pub mod ui;

use std::io;
use std::path::PathBuf;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::Terminal;
use ratatui::prelude::CrosstermBackend;

use app::App;
use event::EventBus;

// ── 终端恢复辅助 ──────────────────────────────────────────────────────────

/// 恢复终端到正常状态。
///
/// 在 panic 或正常退出时调用，确保用户终端不会停留在 raw mode / alternate screen。
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// 安装 panic hook，确保 panic 时终端状态能被恢复。
///
/// 使用 `std::panic::set_hook` 替换默认 hook，
/// 在输出 panic 信息之前先恢复终端。
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        // 尝试恢复终端——如果失败则忽略（已经在 panic 中）
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);

        // 输出 panic 信息到 stderr（原始 hook 的简化版本）
        eprintln!("\npinchtask TUI 发生 panic:\n{panic_info}");
    }));
}

// ── TUI 入口 ───────────────────────────────────────────────────────────────

/// TUI 入口函数。
///
/// 职责：
/// 1. 初始化终端（raw mode + alternate screen）
/// 2. 安装 panic hook 确保终端恢复
/// 3. 启动事件总线
/// 4. 运行主循环（渲染 → 事件 → 状态更新 → 重复）
/// 5. 恢复终端状态
pub async fn run(data_dir: Option<PathBuf>, project_id: Option<String>) -> Result<()> {
    // 在终端初始化之前安装 panic hook
    install_panic_hook();

    // ── 初始化终端 ─────────────────────────────────────────────────────
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // ── 创建应用状态与事件总线 ─────────────────────────────────────────
    let mut app = App::new(data_dir, project_id);
    let mut event_bus = EventBus::new();
    app.set_action_tx(event_bus.sender());

    // ── 启动时加载项目列表（会自动加载首个项目的任务） ─────────────────
    app.spawn_load_projects();

    // ── 主循环 ─────────────────────────────────────────────────────────
    loop {
        // 渲染当前帧
        terminal.draw(|f| ui::draw(f, &app))?;

        // 等待下一个事件
        let event = match event_bus.next().await {
            Some(event) => event,
            None => {
                // channel 已关闭，无更多事件
                break;
            }
        };

        // 处理事件，更新状态
        if let Err(e) = app.handle_event(event) {
            app.set_error_message(format!("事件处理错误: {e}"));
        }

        // 检查是否应该退出
        if app.should_quit() {
            break;
        }
    }

    // ── 恢复终端 ───────────────────────────────────────────────────────
    restore_terminal(&mut terminal)?;

    Ok(())
}
