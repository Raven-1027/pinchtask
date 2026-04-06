//! TUI 渲染模块入口。
//!
//! 负责将 `App` 状态绘制到终端帧缓冲区。
//! 每个视图对应一个独立的渲染函数。
//!
//! ═══════════════════════════════════════════════════════════════════════
//! 项目管理功能 — 视图架构设计
//! ═══════════════════════════════════════════════════════════════════════
//!
//! 新增 View 变体:
//!   - ProjectList:  项目列表视图（表格 + 选中高亮）
//!   - ProjectDetail: 项目详情视图（名称/描述/时间 + 关联任务列表）
//!   - ProjectForm:  项目表单视图（名称 + 描述）
//!
//! 导航路径:
//!   TaskList ── p 键 ──▸ ProjectList
//!   ProjectList ◂── Esc ── TaskList
//!   ProjectList ── Enter ──▸ ProjectDetail
//!   ProjectList ── n ──▸ ProjectForm（新建）
//!   ProjectDetail ── E ──▸ ProjectForm（编辑）
//!   ProjectDetail ── Enter ──▸ TaskDetail（查看关联任务）
//!   ProjectDetail ◂── Esc ── ProjectList
//!   ProjectForm ◂── Esc ── ProjectDetail / ProjectList
//!
//! 项目详情视图内容:
//!   项目名、描述、创建时间、更新时间
//!   关联任务列表（描述、进度、优先级）
//!
//! 项目表单:
//!   名称（必填）+ 描述（可选）
//!   Tab 切换字段，Enter 提交，Esc 取消
//!
//! 快捷键映射:
//!   ProjectList: ↑↓/jk 移动，Enter 详情，n 新建，d 删除，r 刷新，Esc 返回
//!   ProjectDetail: ↑↓/jk 移动任务，Enter 查看任务，E 编辑项目，d 删除项目，Esc 返回
//!   ProjectForm: Tab 切换字段，Enter 提交，Esc 取消

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use super::app::{App, FormField, FormMode, InputMode, View};

pub mod project_detail;
pub mod project_form;
pub mod project_list;
pub mod task_detail;
pub mod task_list;
pub mod theme;

use theme::*;

// ── 公开渲染入口 ───────────────────────────────────────────────────────────

/// 顶层渲染入口：根据当前视图分发到对应的渲染函数。
pub fn draw(f: &mut Frame, app: &App) {
    let size = f.area();

    // 最小终端尺寸检测
    if size.width < MIN_WIDTH || size.height < MIN_HEIGHT {
        draw_too_small(f, size);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // 标题栏
            Constraint::Min(0),    // 主内容区
            Constraint::Length(1), // 状态栏
        ])
        .split(size);

    draw_title_bar(f, chunks[0], app);

    match app.view() {
        View::TaskList => draw_task_list(f, chunks[1], app),
        View::TaskDetail => draw_task_detail(f, chunks[1], app),
        View::TaskForm => draw_task_form(f, chunks[1], app),
        View::Help => draw_help(f, chunks[1], app),
        View::ProjectList => draw_project_list(f, chunks[1], app),
        View::ProjectDetail => draw_project_detail(f, chunks[1], app),
        View::ProjectForm => draw_project_form(f, chunks[1], app),
    }

    // 删除确认对话框（覆盖层）
    if app.confirm_delete() {
        draw_delete_confirm(f, size, app);
    }

    // 笔记删除确认对话框（覆盖层）
    if app.confirm_delete_note() {
        draw_delete_note_confirm(f, size, app);
    }

    // 项目删除确认对话框（覆盖层）
    if app.confirm_delete_project() {
        draw_delete_project_confirm(f, size, app);
    }

    draw_status_bar(f, chunks[2], app);
}

// ── 最小终端尺寸提示 ───────────────────────────────────────────────────────

fn draw_too_small(f: &mut Frame, area: Rect) {
    let msg = format!(
        " 终端尺寸过小（当前 {}×{}），请至少调整为 {}×{} ",
        area.width, area.height, MIN_WIDTH, MIN_HEIGHT
    );
    let paragraph = Paragraph::new(Line::from(Span::styled(
        msg,
        Style::default()
            .fg(ACCENT)
            .add_modifier(Modifier::BOLD),
    )))
    .style(Style::default().bg(TITLE_BG))
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(paragraph, area);
}

// ── 标题栏 ─────────────────────────────────────────────────────────────────

fn draw_title_bar(f: &mut Frame, area: Rect, app: &App) {
    let view_name = match app.view() {
        View::TaskList => "任务列表",
        View::TaskDetail => "任务详情",
        View::TaskForm => "新建/编辑任务",
        View::Help => "帮助",
        View::ProjectList => "项目列表",
        View::ProjectDetail => "项目详情",
        View::ProjectForm => "新建/编辑项目",
    };

    let mut spans = vec![
        Span::styled(
            " pinchtask TUI ",
            Style::default()
                .fg(ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("[{view_name}]"),
            Style::default().fg(HIGHLIGHT_FG),
        ),
    ];

    // 搜索模式下显示搜索框
    if app.search_mode() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled("搜索: ", Style::default().fg(Color::Green)));
        spans.push(Span::styled(
            format!("{}│", app.search_query().unwrap_or("")),
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
        ));
    } else if let Some(query) = app.search_query() {
        if !query.is_empty() {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(
                format!("过滤: {query}"),
                Style::default().fg(Color::Green),
            ));
        }
    }

    // 排序方式
    if app.view() == &View::TaskList && !app.search_mode() {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            format!("排序: {}", app.sort_mode().label()),
            Style::default().fg(MUTED),
        ));
    }

    let paragraph = Paragraph::new(Line::from(spans)).style(Style::default().bg(TITLE_BG));
    f.render_widget(paragraph, area);
}

// ── 状态栏 ─────────────────────────────────────────────────────────────────

fn draw_status_bar(f: &mut Frame, area: Rect, app: &App) {
    let text = if let Some(err) = app.error_message() {
        Line::from(vec![
            Span::styled(
                format!(" {} ", ICON_ERROR),
                Style::default().fg(TEXT).bg(Color::Red),
            ),
            Span::styled(err.to_owned(), Style::default().fg(Color::Red)),
        ])
    } else if let Some(msg) = app.message() {
        Line::from(vec![
            Span::styled(
                format!(" {} ", ICON_INFO),
                Style::default().fg(Color::Black).bg(ACCENT),
            ),
            Span::styled(msg.to_owned(), Style::default().fg(ACCENT)),
        ])
    } else {
        let hints = match app.view() {
            View::TaskList => {
                " ↑↓/jk 移动  Enter 详情  n 新建  d 删除  r/Ctrl+R 刷新  / 搜索  Tab 排序  p 项目  ? 帮助  q 退出"
            }
            View::TaskDetail => {
                if app.input_mode() != &InputMode::Normal {
                    " Enter 确认  Esc 取消  Backspace 删除"
                } else {
                    " ↑↓/jk 移动  Space 完成  a 添加  e 编辑  E 编辑任务  d 删除条目  N 添加笔记  D 删除笔记  L 添加资源  Ctrl+D 删除任务  Ctrl+J/K 移动顺序  Esc 返回"
                }
            }
            View::TaskForm => " Tab 切换字段  Enter 提交  Esc 取消",
            View::Help => " Esc/? 返回  q 退出",
            View::ProjectList => {
                " ↑↓/jk 移动  Enter 详情  n 新建  d 删除  r 刷新  Esc 返回任务列表"
            }
            View::ProjectDetail => {
                " ↑↓/jk 移动  Enter 查看任务  E 编辑项目  d 删除项目  Esc 返回"
            }
            View::ProjectForm => " Tab 切换字段  Enter 提交  Esc 取消",
        };
        Line::from(Span::styled(hints, Style::default().fg(MUTED)))
    };

    let paragraph = Paragraph::new(text).style(Style::default().bg(TITLE_BG));
    f.render_widget(paragraph, area);
}

// ── 任务列表视图 ───────────────────────────────────────────────────────────

fn draw_task_list(f: &mut Frame, area: Rect, app: &App) {
    use ratatui::style::Color;

    let tasks = app.filtered_and_sorted_tasks();
    let selected = app.selected_index();

    if tasks.is_empty() {
        let empty_msg = if app.search_query().is_some() {
            vec![
                Line::from(""),
                Line::styled(
                    "  没有匹配的任务",
                    Style::default()
                        .fg(MUTED)
                        .add_modifier(Modifier::ITALIC),
                ),
                Line::from(""),
                Line::styled(
                    "  按 Esc 清除搜索过滤",
                    Style::default().fg(MUTED),
                ),
            ]
        } else {
            vec![
                Line::from(""),
                Line::styled(
                    "  暂无任务",
                    Style::default()
                        .fg(MUTED)
                        .add_modifier(Modifier::ITALIC),
                ),
                Line::from(""),
                Line::styled(
                    "  按 n 创建新任务，按 ? 查看帮助",
                    Style::default().fg(MUTED),
                ),
            ]
        };
        f.render_widget(Paragraph::new(empty_msg), area);
        return;
    }

    // 计算可用行数（表头 2 行 + 底部信息 1 行）
    let available_rows = area.height.saturating_sub(3) as usize;

    // 调整滚动偏移
    let visible_start = app.scroll_offset();
    let visible_end = (visible_start + available_rows).min(tasks.len());

    let mut lines: Vec<Line> = Vec::new();

    // 表头
    lines.push(Line::from(vec![
        Span::styled("  #   ", header_style()),
        Span::styled("ID        ", header_style()),
        Span::styled("描述", header_style()),
        Span::styled("                    ", header_style()),
        Span::styled("进度    ", header_style()),
        Span::styled("优先级  ", header_style()),
    ]));
    lines.push(Line::from(separator(area.width.saturating_sub(2) as usize)));

    // 任务行（只渲染可见范围）
    let desc_width = 20usize;
    for i in visible_start..visible_end {
        let task = tasks[i];
        let is_selected = i == selected;

        let marker = if is_selected { ICON_SELECTED } else { ICON_UNSELECTED };
        let id_short = &task.id[..task.id.len().min(8)];

        // 进度
        let done_count = task.checklist.iter().filter(|item| item.done).count();
        let total = task.checklist.len();
        let progress = format!("{done_count}/{total}");
        let progress_pct = if total > 0 {
            done_count * 100 / total
        } else {
            0
        };

        // 优先级
        let priority_str = task
            .metadata
            .as_ref()
            .and_then(|m| m.priority.as_deref())
            .unwrap_or("-");
        let all_done = total > 0 && done_count == total;

        // 行样式
        let row_style = if is_selected {
            selected_style()
        } else if all_done {
            completed_style()
        } else {
            normal_style()
        };

        let priority_style = if is_selected {
            Style::default()
                .fg(priority_color(priority_str))
                .bg(Color::Indexed(236))
        } else if all_done {
            completed_priority_style(priority_str)
        } else {
            Style::default().fg(priority_color(priority_str))
        };

        lines.push(Line::from(vec![
            Span::styled(format!(" {marker}   "), row_style),
            Span::styled(format!("{id_short:<8} "), row_style),
            Span::styled(truncate_str(&task.task_description, desc_width), row_style),
            Span::raw(" "),
            Span::styled(format!("{progress:<5}"), row_style),
            Span::raw(" "),
            // 彩色进度条
            Span::styled(" ", row_style),
        ]));

        // 附加进度条 spans（需要单独添加以保留颜色）
        let bar_spans = progress_bar_spans(progress_pct, 6);
        for span in bar_spans {
        let styled = if is_selected {
        let fg = span.style.fg.unwrap_or(Color::Green);
        Span::styled(span.content, Style::default().fg(fg).bg(Color::Indexed(236)))
        } else {
        span
        };
        lines.last_mut().unwrap().spans.push(styled);
        }

        lines.last_mut().unwrap().spans.push(Span::raw(" "));
        lines.last_mut().unwrap().spans.push(Span::styled(
            format!("{priority_str:<6}"),
            priority_style,
        ));
    }

    // 底部信息
    lines.push(Line::from(""));
    let mut info_spans = vec![Span::styled(
        format!("  共 {} 个任务", tasks.len()),
        Style::default().fg(MUTED),
    )];
    if visible_start > 0 || visible_end < tasks.len() {
        info_spans.push(Span::styled(
            format!(
                "  (显示 {}/{}  ↑↓ 滚动)",
                visible_end - visible_start,
                tasks.len()
            ),
            Style::default().fg(MUTED),
        ));
    }
    lines.push(Line::from(info_spans));

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);
}

// ── 任务详情视图 ───────────────────────────────────────────────────────────

fn draw_task_detail(f: &mut Frame, area: Rect, app: &App) {
    if let Some(task) = app.current_task() {
        let widget = task_detail::TaskDetail::new(task, app.selected_item_index());
        f.render_widget(widget, area);
    } else {
        let content = vec![
            Line::from(""),
            Line::styled("  正在加载任务详情...", Style::default().fg(MUTED)),
        ];
        let paragraph = Paragraph::new(content).wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
    }

    // 输入模式覆盖层
    if app.input_mode() != &InputMode::Normal {
        draw_input_overlay(f, area, app);
    }
}

/// 行内输入覆盖层。
fn draw_input_overlay(f: &mut Frame, area: Rect, app: &App) {
    let label = match app.input_mode() {
        InputMode::AddingItem => "添加条目",
        InputMode::EditingItemName => "编辑名称",
        InputMode::EditingItemDesc => "编辑描述",
        InputMode::AddingNote => "添加笔记",
        InputMode::AddingResourceName => "资源名称",
        InputMode::AddingResourceUrl => "资源 URL",
        InputMode::Normal => return,
    };

    let width = 60.min(area.width.saturating_sub(4));
    let height = 5;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + area.height.saturating_sub(height).saturating_sub(2);
    let dialog_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, dialog_area);

    let lines = vec![
        Line::from(vec![
            Span::styled(format!("  {label}: "), Style::default().fg(ACCENT)),
            Span::styled(
                format!("{}│", app.input_buffer()),
                Style::default().fg(TEXT).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  Enter ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("确认  "),
            Span::styled(
                "Esc ",
                Style::default()
                    .fg(HIGHLIGHT_FG)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("取消"),
        ]),
    ];

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_FOCUSED))
            .title(Span::styled(
                format!(" {} {label} ", ICON_EDIT,),
                Style::default().fg(ACCENT),
            )),
    );
    f.render_widget(paragraph, dialog_area);
}

// ── 任务表单视图 ───────────────────────────────────────────────────────────

fn draw_task_form(f: &mut Frame, area: Rect, app: &App) {
    let Some(form) = app.form_state() else {
        let content = vec![
            Line::from(""),
            Line::styled(
                "  表单状态异常",
                Style::default().fg(Color::Red),
            ),
        ];
        let paragraph = Paragraph::new(content);
        f.render_widget(paragraph, area);
        return;
    };

    let is_edit = form.mode != FormMode::Create;
    let title_icon = if is_edit { ICON_EDIT } else { ICON_NEW };
    let title = if is_edit {
        " 编辑任务 "
    } else {
        " 新建任务 "
    };

    let total_width = area.width as usize;
    let label_width = 10;
    let field_width = total_width.saturating_sub(label_width).saturating_sub(6).max(20);

    let fields = [
        (
            FormField::Description,
            &form.description,
            form.focused_field == FormField::Description,
        ),
        (
            FormField::Context,
            &form.context,
            form.focused_field == FormField::Context,
        ),
        (
            FormField::Priority,
            &form.priority,
            form.focused_field == FormField::Priority,
        ),
        (
            FormField::Tags,
            &form.tags,
            form.focused_field == FormField::Tags,
        ),
        (
            FormField::Eta,
            &form.eta,
            form.focused_field == FormField::Eta,
        ),
    ];

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    for (field, value, focused) in &fields {
        let label = field.label();
        let placeholder = field.placeholder();

        // 标签样式
        let label_style = if *focused {
            Style::default()
                .fg(ACCENT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(MUTED)
        };

        // 值样式
        let value_display: String = if value.is_empty() && !focused {
            placeholder.to_owned()
        } else {
            let mut display = (*value).clone();
            if *focused {
                display.push('│');
            }
            display
        };
        let value_style = if value.is_empty() && !focused {
            Style::default().fg(MUTED)
        } else if *focused {
            Style::default().fg(TEXT).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(TEXT)
        };

        let marker = if *focused { ICON_SELECTED } else { ICON_UNSELECTED };

        lines.push(Line::from(vec![
            Span::styled(format!("  {marker} "), label_style),
            Span::styled(format!("{label}: "), label_style),
            Span::styled(truncate_str(&value_display, field_width), value_style),
        ]));

        // 空行分隔
        lines.push(Line::from(""));
    }

    // 校验错误
    if let Some(ref err) = form.error {
        lines.push(Line::from(vec![
            Span::styled(format!(" {} ", ICON_ERROR), Style::default().fg(Color::Red)),
            Span::styled(err.clone(), Style::default().fg(Color::Red)),
        ]));
    }

    // 提示
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled(
            "  Tab ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("切换字段  "),
        Span::styled(
            "Enter ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(if is_edit { "保存" } else { "创建" }),
        Span::raw("  "),
        Span::styled(
            "Esc ",
            Style::default()
                .fg(HIGHLIGHT_FG)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("取消"),
    ]));

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_FOCUSED))
            .title(Span::styled(
                format!(" {title_icon} {title}"),
                Style::default().fg(ACCENT),
            )),
    );
    f.render_widget(paragraph, area);
}

// ── 帮助视图 ───────────────────────────────────────────────────────────────

fn draw_help(f: &mut Frame, area: Rect, _app: &App) {
    let help_text = vec![
        Line::from(""),
        Line::styled(
            "  全局快捷键:",
            section_title_style(),
        ),
        Line::from(vec![
            Span::styled("    q / Ctrl+C", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    退出 TUI"),
        ]),
        Line::from(vec![
            Span::styled("    ?        ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    显示/关闭帮助"),
        ]),
        Line::from(""),
        Line::styled(
            "  任务列表视图:",
            section_title_style(),
        ),
        Line::from(vec![
            Span::styled("    ↑/j/k/↓  ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    上下移动选中"),
        ]),
        Line::from(vec![
            Span::styled("    Enter    ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    查看任务详情"),
        ]),
        Line::from(vec![
            Span::styled("    n        ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    新建任务"),
        ]),
        Line::from(vec![
            Span::styled("    d        ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    删除选中任务"),
        ]),
        Line::from(vec![
            Span::styled("    r/Ctrl+R ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    刷新任务列表"),
        ]),
        Line::from(vec![
            Span::styled("    /        ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    搜索过滤"),
        ]),
        Line::from(vec![
            Span::styled("    Tab      ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    切换排序方式"),
        ]),
        Line::from(vec![
            Span::styled("    Home/End ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    跳转到首/末"),
        ]),
        Line::from(""),
        Line::styled(
            "  项目列表视图:",
            section_title_style(),
        ),
        Line::from(vec![
            Span::styled("    p        ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    从任务列表进入项目列表"),
        ]),
        Line::from(vec![
            Span::styled("    ↑/j/k/↓  ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    上下移动选中"),
        ]),
        Line::from(vec![
            Span::styled("    Enter    ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    查看项目详情"),
        ]),
        Line::from(vec![
            Span::styled("    n        ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    新建项目"),
        ]),
        Line::from(vec![
            Span::styled("    d        ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    删除选中项目"),
        ]),
        Line::from(vec![
            Span::styled("    Esc      ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    返回任务列表"),
        ]),
        Line::from(""),
        Line::styled(
            "  项目详情视图:",
            section_title_style(),
        ),
        Line::from(vec![
            Span::styled("    ↑/j/k/↓  ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    上下移动关联任务"),
        ]),
        Line::from(vec![
            Span::styled("    Enter    ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    查看关联任务详情"),
        ]),
        Line::from(vec![
            Span::styled("    E        ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    编辑项目"),
        ]),
        Line::from(vec![
            Span::styled("    d        ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    删除项目"),
        ]),
        Line::from(vec![
            Span::styled("    Esc      ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    返回项目列表"),
        ]),
        Line::from(""),
        Line::styled(
            "  任务详情视图:",
            section_title_style(),
        ),
        Line::from(vec![
            Span::styled("    ↑/j/k/↓  ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    上下移动条目"),
        ]),
        Line::from(vec![
            Span::styled("    Space/x  ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    切换条目完成状态"),
        ]),
        Line::from(vec![
            Span::styled("    a        ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    添加清单条目"),
        ]),
        Line::from(vec![
            Span::styled("    N        ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    添加笔记"),
        ]),
        Line::from(vec![
            Span::styled("    D        ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    删除笔记（需确认）"),
        ]),
        Line::from(vec![
            Span::styled("    L        ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    添加资源链接"),
        ]),
        Line::from(vec![
            Span::styled("    Ctrl+D   ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    删除当前任务（需确认）"),
        ]),
        Line::from(vec![
            Span::styled("    Esc/←    ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    返回任务列表"),
        ]),
        Line::from(""),
        Line::styled(
            "  任务表单视图:",
            section_title_style(),
        ),
        Line::from(vec![
            Span::styled("    Tab      ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    切换到下一个字段"),
        ]),
        Line::from(vec![
            Span::styled("    Shift+Tab", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("切换到上一个字段"),
        ]),
        Line::from(vec![
            Span::styled("    Enter    ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    提交表单（新建/保存）"),
        ]),
        Line::from(vec![
            Span::styled("    Esc      ", Style::default().fg(HIGHLIGHT_FG)),
            Span::raw("    取消并返回"),
        ]),
        Line::from(""),
        Line::styled(
            "  按 Esc 或 ? 返回上一视图",
            Style::default()
                .fg(MUTED)
                .add_modifier(Modifier::ITALIC),
        ),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(BORDER))
                .title(Span::styled(" 帮助 ", Style::default().fg(ACCENT))),
        )
        .wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}

// ── 删除确认对话框 ─────────────────────────────────────────────────────────

fn draw_delete_confirm(f: &mut Frame, area: Rect, app: &App) {
    let width = 50.min(area.width.saturating_sub(4));
    let height = 6;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, dialog_area);

    let task_name = app
        .current_task()
        .map(|t| truncate_str(&t.task_description, 30))
        .or_else(|| {
            app.filtered_and_sorted_tasks()
                .get(app.selected_index())
                .map(|t| truncate_str(&t.task_description, 30))
        })
        .unwrap_or_default();

    let lines = vec![
        Line::from(""),
        Line::styled(
            "  确认删除此任务？",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ),
        Line::from(format!("  {task_name}")),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  y",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" 确认  "),
            Span::styled(
                "n/Esc",
                Style::default()
                    .fg(HIGHLIGHT_FG)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" 取消"),
        ]),
    ];

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_DANGER))
            .title(Span::styled(
                format!(" {} 删除确认 ", ICON_WARN),
                Style::default().fg(Color::Red),
            )),
    );

    f.render_widget(paragraph, dialog_area);
}

// ── 笔记删除确认对话框 ─────────────────────────────────────────────────────

fn draw_delete_note_confirm(f: &mut Frame, area: Rect, app: &App) {
    let width = 50.min(area.width.saturating_sub(4));
    let height = 6;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, dialog_area);

    let note_preview = app
        .current_task()
        .and_then(|t| t.notes.get(app.selected_note_index()))
        .map(|n| truncate_str(n, 30))
        .unwrap_or_default();

    let lines = vec![
        Line::from(""),
        Line::styled(
            "  确认删除此笔记？",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ),
        Line::from(format!("  {note_preview}")),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  y",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" 确认  "),
            Span::styled(
                "n/Esc",
                Style::default()
                    .fg(HIGHLIGHT_FG)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" 取消"),
        ]),
    ];

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_DANGER))
            .title(Span::styled(
                format!(" {} 删除笔记 ", ICON_WARN),
                Style::default().fg(Color::Red),
            )),
    );

    f.render_widget(paragraph, dialog_area);
}

// ── 项目列表视图 ───────────────────────────────────────────────────────────

fn draw_project_list(f: &mut Frame, area: Rect, app: &App) {
    let widget = project_list::ProjectList::new(app.projects(), app.project_selected_index());
    f.render_widget(widget, area);
}

// ── 项目详情视图 ─────────────────────────────────────────────────────────────

fn draw_project_detail(f: &mut Frame, area: Rect, app: &App) {
    if let Some(project) = app.current_project() {
        let widget = project_detail::ProjectDetail::new(
            project,
            app.project_tasks(),
            app.project_task_selected_index(),
        );
        f.render_widget(widget, area);
    } else {
        let content = vec![
            Line::from(""),
            Line::styled("  正在加载项目详情...", Style::default().fg(MUTED)),
        ];
        let paragraph = Paragraph::new(content).wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
    }
}

// ── 项目表单视图 ─────────────────────────────────────────────────────────────

fn draw_project_form(f: &mut Frame, area: Rect, app: &App) {
    let Some(form) = app.project_form_state() else {
        let content = vec![
            Line::from(""),
            Line::styled(
                "  表单状态异常",
                Style::default().fg(Color::Red),
            ),
        ];
        let paragraph = Paragraph::new(content);
        f.render_widget(paragraph, area);
        return;
    };

    let widget = project_form::ProjectForm::new(form);
    f.render_widget(widget, area);
}

// ── 项目删除确认对话框 ───────────────────────────────────────────────────────

fn draw_delete_project_confirm(f: &mut Frame, area: Rect, app: &App) {
    let width = 50.min(area.width.saturating_sub(4));
    let height = 6;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    let dialog_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, dialog_area);

    let project_name = app
        .current_project()
        .map(|p| truncate_str(&p.name, 30))
        .or_else(|| {
            app.projects()
                .get(app.project_selected_index())
                .map(|p| truncate_str(&p.name, 30))
        })
        .unwrap_or_default();

    let lines = vec![
        Line::from(""),
        Line::styled(
            "  确认删除此项目？",
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        ),
        Line::from(format!("  {project_name}")),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  y",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" 确认  "),
            Span::styled(
                "n/Esc",
                Style::default()
                    .fg(HIGHLIGHT_FG)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" 取消"),
        ]),
    ];

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER_DANGER))
            .title(Span::styled(
                format!(" {} 删除确认 ", ICON_WARN),
                Style::default().fg(Color::Red),
            )),
    );

    f.render_widget(paragraph, dialog_area);
}

// ── 辅助函数 ───────────────────────────────────────────────────────────────

/// 将字符串截断到指定字符宽度，不足则右填充空格。
fn truncate_str(s: &str, max_len: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_len {
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
