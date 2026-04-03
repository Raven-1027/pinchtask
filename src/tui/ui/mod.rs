//! TUI 渲染模块入口。
//!
//! 负责将 `App` 状态绘制到终端帧缓冲区。
//! 每个视图对应一个独立的渲染函数。

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use super::app::{App, View};

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
fn draw_task_list(f: &mut Frame, area: Rect, app: &App) {
    let tasks = app.tasks();
    let selected = app.selected_index();

    if tasks.is_empty() {
        let empty_msg = Paragraph::new(vec![
            Line::from(""),
            Line::styled(
                "  暂无任务",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            ),
            Line::from(""),
            Line::styled(
                "  按 n 创建新任务，按 ? 查看帮助",
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        f.render_widget(empty_msg, area);
        return;
    }

    // 构建任务行列表
    let mut lines: Vec<Line> = Vec::new();

    // 表头
    lines.push(Line::from(vec![
        Span::styled("  # ", Style::default().fg(Color::DarkGray)),
        Span::styled("ID       ", Style::default().fg(Color::DarkGray)),
        Span::styled("描述                ", Style::default().fg(Color::DarkGray)),
        Span::styled("进度   ", Style::default().fg(Color::DarkGray)),
        Span::styled("优先级", Style::default().fg(Color::DarkGray)),
    ]));
    lines.push(Line::from(Span::styled(
        "─".repeat(area.width.saturating_sub(2) as usize),
        Style::default().fg(Color::DarkGray),
    )));

    // 任务行
    for (i, task) in tasks.iter().enumerate() {
        let is_selected = i == selected;

        // 选中行标记
        let marker = if is_selected { "▸" } else { " " };

        // ID 前 8 位
        let id_short = &task.id[..task.id.len().min(8)];

        // 进度：已完成/总数
        let done_count = task.checklist.iter().filter(|item| item.done).count();
        let total = task.checklist.len();
        let progress = format!("{done_count}/{total}");

        // 优先级
        let priority = task
            .metadata
            .as_ref()
            .and_then(|m| m.priority.as_deref())
            .unwrap_or("-");

        let style = if is_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        lines.push(Line::from(vec![
            Span::styled(format!(" {marker} "), style),
            Span::styled(format!("{id_short:<8} "), style),
            // 描述截断到 20 字符
            Span::styled(
                truncate_str(&task.task_description, 20),
                style,
            ),
            Span::raw(" "),
            Span::styled(format!("{progress:<5} "), style),
            Span::styled(priority.to_owned(), style),
        ]));
    }

    // 底部信息行
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("  共 {} 个任务", tasks.len()),
        Style::default().fg(Color::DarkGray),
    )));

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);
}

/// 渲染任务详情视图（占位）。
fn draw_task_detail(f: &mut Frame, area: Rect, app: &App) {
    let content = if let Some(task) = app.current_task() {
        let done = task.checklist.iter().filter(|i| i.done).count();
        let total = task.checklist.len();
        vec![
            Line::from(""),
            Line::styled(
                format!("  任务: {}", task.task_description),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
            Line::from(format!(
                "  ID: {}",
                &task.id[..task.id.len().min(8)]
            )),
            Line::from(format!("  进度: {done}/{total}")),
            Line::from(format!("  创建: {}", task.created_at)),
            Line::from(""),
            Line::styled(
                format!(
                    "  共 {} 个清单条目，{} 个笔记，{} 个资源",
                    task.checklist.len(),
                    task.notes.len(),
                    task.resources.len()
                ),
                Style::default().fg(Color::DarkGray),
            ),
            Line::from(""),
            Line::styled(
                "  ↑↓/jk 移动条目  Space 完成/撤销  ←/Esc 返回列表",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]
    } else {
        vec![
            Line::from(""),
            Line::styled(
                "  未加载任务详情",
                Style::default().fg(Color::DarkGray),
            ),
        ]
    };

    let paragraph = Paragraph::new(content);
    f.render_widget(paragraph, area);
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

// ── 辅助函数 ───────────────────────────────────────────────────────────────

/// 将字符串截断到指定字符宽度。
///
/// 如果字符数超过 `max_len`，截断并追加 `…`；
/// 不足则用空格填充到 `max_len`。
fn truncate_str(s: &str, max_len: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_len {
        // 用空格右填充到 max_len
        let mut result = s.to_owned();
        while result.chars().count() < max_len {
            result.push(' ');
        }
        result
    } else {
        let truncated: String = chars[..max_len.saturating_sub(1)].iter().collect();
        format!("{truncated}…")
    }
}
