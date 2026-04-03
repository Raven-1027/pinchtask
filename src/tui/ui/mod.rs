//! TUI 渲染模块入口。
//!
//! 负责将 `App` 状态绘制到终端帧缓冲区。
//! 每个视图对应一个独立的渲染函数。

mod task_detail;
mod task_list;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use super::app::{App, View};
use task_detail::TaskDetail;
use task_list::TaskList;

// ── 公开渲染入口 ───────────────────────────────────────────────────────────

/// 顶层渲染入口：根据当前视图分发到对应的渲染函数。
///
/// 布局：
/// - 顶部：标题栏（项目名 + 当前视图名）height=1
/// - 中间：主内容区（按 view 分发）自适应
/// - 底部：状态栏（快捷键提示 / 消息）height=1
pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();

    // 三段式布局：标题栏 / 主内容 / 状态栏
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),   // 标题栏
            Constraint::Min(0),      // 主内容区（自适应）
            Constraint::Length(1),   // 状态栏
        ])
        .split(size);

    draw_title_bar(f, chunks[0], app);

    // 主内容区：根据当前视图分发
    match app.view() {
        View::TaskList => draw_task_list(f, chunks[1], app),
        View::TaskDetail => draw_task_detail(f, chunks[1], app),
        View::TaskForm => draw_task_form(f, chunks[1], app),
        View::Help => draw_help(f, chunks[1], app),
    }

    draw_status_bar(f, chunks[2], app);
}

// ── 标题栏 ─────────────────────────────────────────────────────────────────

/// 渲染顶部标题栏。
fn draw_title_bar(f: &mut Frame, area: Rect, app: &App) {
    let view_name = match app.view() {
        View::TaskList => "任务列表",
        View::TaskDetail => "任务详情",
        View::TaskForm => "新建/编辑任务",
        View::Help => "帮助",
    };

    let title = Line::from(vec![
        Span::styled(
            " pinchtask TUI ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("[{view_name}]"),
            Style::default().fg(Color::Yellow),
        ),
    ]);

    let paragraph = Paragraph::new(title).style(Style::default().bg(Color::DarkGray));
    f.render_widget(paragraph, area);
}

// ── 状态栏 ─────────────────────────────────────────────────────────────────

/// 渲染底部状态栏（快捷键提示 + 消息）。
fn draw_status_bar(f: &mut Frame, area: Rect, app: &App) {
    // 优先显示错误消息，其次普通消息，最后显示快捷键提示
    let text = if let Some(err) = app.error_message() {
        Line::from(vec![
            Span::styled(
                " ✖ ",
                Style::default().fg(Color::White).bg(Color::Red),
            ),
            Span::styled(
                err.to_owned(),
                Style::default().fg(Color::Red),
            ),
        ])
    } else if let Some(msg) = app.message() {
        Line::from(vec![
            Span::styled(
                " ℹ ",
                Style::default().fg(Color::Black).bg(Color::Cyan),
            ),
            Span::styled(
                msg.to_owned(),
                Style::default().fg(Color::Cyan),
            ),
        ])
    } else {
        // 默认快捷键提示
        let hints = match app.view() {
            View::TaskList => {
                " ↑↓/jk 移动  Enter 详情  n 新建  d 删除  ? 帮助  q 退出"
            }
            View::TaskDetail => " ↑↓/jk 移动条目  Space 完成/撤销  a 添加  ←/Esc 返回  ? 帮助",
            View::TaskForm => " Tab 切换字段  Enter 提交  Esc 取消",
            View::Help => " Esc/? 返回  q 退出",
        };
        Line::from(Span::styled(
            hints,
            Style::default().fg(Color::DarkGray),
        ))
    };

    let paragraph = Paragraph::new(text).style(Style::default().bg(Color::DarkGray));
    f.render_widget(paragraph, area);
}

// ── 视图渲染函数 ───────────────────────────────────────────────────────────

/// 渲染任务列表视图。
///
/// 委托给 `TaskList` 组件的 `Widget` 实现，
/// 传入任务数据和选中索引即可完成渲染。
fn draw_task_list(f: &mut Frame, area: Rect, app: &App) {
    let widget = TaskList::new(app.tasks(), app.selected_index());
    f.render_widget(widget, area);
}

/// 渲染任务详情视图。
///
/// 委托给 `TaskDetail` 组件的 `Widget` 实现，
/// 传入当前任务和清单焦点索引即可完成渲染。
fn draw_task_detail(f: &mut Frame, area: Rect, app: &App) {
    if let Some(task) = app.current_task() {
        let widget = TaskDetail::new(task, app.detail_item_index());
        f.render_widget(widget, area);
    } else {
        let content = vec![
            Line::from(""),
            Line::styled(
                "  未加载任务详情，按 Esc 返回列表",
                Style::default().fg(Color::DarkGray),
            ),
        ];
        let paragraph = Paragraph::new(content);
        f.render_widget(paragraph, area);
    }
}

/// 渲染任务创建/编辑表单（占位）。
fn draw_task_form(f: &mut Frame, area: Rect, _app: &App) {
    let content = vec![
        Line::from(""),
        Line::styled(
            "  [任务表单 — 待实现]",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ),
        Line::from(""),
        Line::styled(
            "  Tab 切换字段  Enter 提交  Esc 取消",
            Style::default().fg(Color::DarkGray),
        ),
    ];

    let paragraph = Paragraph::new(content);
    f.render_widget(paragraph, area);
}

/// 渲染帮助面板。
fn draw_help(f: &mut Frame, area: Rect, _app: &App) {
    let help_text = vec![
        Line::from(""),
        Line::styled(
            "  全局快捷键:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Line::from(vec![
            Span::styled("    q / Ctrl+C", Style::default().fg(Color::Yellow)),
            Span::raw("    退出 TUI"),
        ]),
        Line::from(vec![
            Span::styled("    ?        ", Style::default().fg(Color::Yellow)),
            Span::raw("    显示/关闭帮助"),
        ]),
        Line::from(""),
        Line::styled(
            "  任务列表视图:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Line::from(vec![
            Span::styled("    ↑/j/k/↓  ", Style::default().fg(Color::Yellow)),
            Span::raw("    上下移动选中"),
        ]),
        Line::from(vec![
            Span::styled("    Enter    ", Style::default().fg(Color::Yellow)),
            Span::raw("    查看任务详情"),
        ]),
        Line::from(vec![
            Span::styled("    n        ", Style::default().fg(Color::Yellow)),
            Span::raw("    新建任务"),
        ]),
        Line::from(vec![
            Span::styled("    d        ", Style::default().fg(Color::Yellow)),
            Span::raw("    删除选中任务"),
        ]),
        Line::from(vec![
            Span::styled("    r        ", Style::default().fg(Color::Yellow)),
            Span::raw("    刷新任务列表"),
        ]),
        Line::from(""),
        Line::styled(
            "  任务详情视图:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Line::from(vec![
            Span::styled("    ↑/j/k/↓  ", Style::default().fg(Color::Yellow)),
            Span::raw("    上下移动条目"),
        ]),
        Line::from(vec![
            Span::styled("    Space/x  ", Style::default().fg(Color::Yellow)),
            Span::raw("    切换条目完成状态"),
        ]),
        Line::from(vec![
            Span::styled("    a        ", Style::default().fg(Color::Yellow)),
            Span::raw("    添加清单条目"),
        ]),
        Line::from(vec![
            Span::styled("    Esc/←    ", Style::default().fg(Color::Yellow)),
            Span::raw("    返回任务列表"),
        ]),
        Line::from(""),
        Line::styled(
            "  按 Esc 或 ? 返回上一视图",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::ITALIC),
        ),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(Span::styled(
                    " 帮助 ",
                    Style::default().fg(Color::Cyan),
                )),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}


