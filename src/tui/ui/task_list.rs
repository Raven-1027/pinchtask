//! 任务列表视图组件。
//!
//! 实现 ratatui `Widget` trait，将任务列表渲染为带表头、选中高亮、
//! 进度/优先级/时间列的表格视图。支持自动滚动，当选中行超出可视区域时
//! 自动调整滚动偏移。

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::models::task::Task;

use super::theme::*;

// ── 常量 ───────────────────────────────────────────────────────────────────

/// 任务行固定前缀宽度：marker(3) + ID(9) = 12 字符。
const PREFIX_WIDTH: usize = 12;

/// 默认描述列宽度。
const DEFAULT_DESC_WIDTH: usize = 24;

/// 进度列固定宽度：" 99/99 "。
const PROGRESS_WIDTH: usize = 7;

/// 优先级列固定宽度。
const PRIORITY_WIDTH: usize = 7;

/// 时间列固定宽度："MM-DD"。
const TIME_WIDTH: usize = 6;

/// 表头 + 分隔线行数。
const HEADER_ROWS: usize = 2;

/// 底部信息行数。
const FOOTER_ROWS: usize = 2;

// ── 组件 ───────────────────────────────────────────────────────────────────

/// 任务列表组件。
///
/// 持有渲染所需的不可变数据引用，通过 `Widget::render` 绘制到指定区域。
/// 不持有任何可变状态，滚动偏移根据 `selected_index` 和区域高度自动计算。
pub struct TaskList<'a> {
    /// 任务列表数据。
    tasks: &'a [Task],
    /// 当前选中行索引。
    selected_index: usize,
}

impl<'a> TaskList<'a> {
    /// 创建新的任务列表组件。
    pub fn new(tasks: &'a [Task], selected_index: usize) -> Self {
        Self {
            tasks,
            selected_index,
        }
    }

    /// 根据可视区域高度和选中索引计算滚动偏移。
    fn scroll_offset(&self, visible_height: usize) -> usize {
        if visible_height == 0 || self.tasks.is_empty() {
            return 0;
        }
        let usable = visible_height.saturating_sub(HEADER_ROWS + FOOTER_ROWS);
        if usable == 0 {
            return 0;
        }
        if self.selected_index >= usable {
            self.selected_index - usable + 1
        } else {
            0
        }
    }
}

impl<'a> Widget for TaskList<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        // 空列表：显示占位提示
        if self.tasks.is_empty() {
            let empty_msg = vec![
                Line::from(""),
                Line::styled(
                    "  暂无任务",
                    Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
                ),
                Line::from(""),
                Line::styled(
                    "  按 n 创建新任务，按 ? 查看帮助",
                    Style::default().fg(MUTED),
                ),
            ];
            let widget = ratatui::widgets::Paragraph::new(empty_msg);
            widget.render(area, buf);
            return;
        }

        let visible_height = area.height as usize;
        let scroll = self.scroll_offset(visible_height);
        let usable_rows = visible_height.saturating_sub(HEADER_ROWS + FOOTER_ROWS);

        let total_width = area.width as usize;
        let desc_width = total_width
            .saturating_sub(PREFIX_WIDTH)
            .saturating_sub(PROGRESS_WIDTH)
            .saturating_sub(PRIORITY_WIDTH)
            .saturating_sub(TIME_WIDTH)
            .saturating_sub(4);
        let desc_width = desc_width.max(8).min(DEFAULT_DESC_WIDTH * 2);

        let mut lines: Vec<Line> = Vec::with_capacity(visible_height);

        // ── 表头 ───────────────────────────────────────────────────────
        lines.push(Line::from(vec![
            Span::styled("  # ", header_style()),
            Span::styled(format!("{:<9}", "ID"), header_style()),
            Span::styled(format!("{:<w$}", "描述", w = desc_width), header_style()),
            Span::styled(
                format!("{:<w$}", "进度", w = PROGRESS_WIDTH),
                header_style(),
            ),
            Span::styled(
                format!("{:<w$}", "优先级", w = PRIORITY_WIDTH),
                header_style(),
            ),
            Span::styled(format!("{:<w$}", "时间", w = TIME_WIDTH), header_style()),
        ]));

        lines.push(Line::from(separator(total_width.saturating_sub(2))));

        // ── 任务行 ─────────────────────────────────────────────────────
        let visible_tasks = self.tasks.iter().enumerate().skip(scroll).take(usable_rows);

        for (i, task) in visible_tasks {
            let is_selected = i == self.selected_index;

            let marker = if is_selected {
                ICON_SELECTED
            } else {
                ICON_UNSELECTED
            };
            let id_short = &task.id[..task.id.len().min(8)];

            let done_count = task.checklist.iter().filter(|item| item.done).count();
            let total = task.checklist.len();
            let all_done = total > 0 && done_count == total;

            let priority_text = task
                .metadata
                .as_ref()
                .and_then(|m| m.priority.as_deref())
                .unwrap_or("-");
            let time_short = format_created_at(&task.created_at);

            // 行样式
            let row_style = if is_selected {
                selected_style()
            } else if all_done {
                completed_style()
            } else {
                normal_style()
            };

            // 优先级样式
            let p_style = if is_selected {
                Style::default().fg(priority_color(priority_text))
            } else if all_done {
                completed_priority_style(priority_text)
            } else {
                Style::default().fg(priority_color(priority_text))
            };

            lines.push(Line::from(vec![
                Span::styled(format!(" {marker} "), row_style),
                Span::styled(format!("{id_short:<9}"), row_style),
                Span::styled(
                    format!(
                        "{:<w$}",
                        truncate_str(&task.task_description, desc_width),
                        w = desc_width
                    ),
                    row_style,
                ),
                Span::styled(
                    format!(
                        "{:<w$}",
                        format!("{done_count}/{total}"),
                        w = PROGRESS_WIDTH
                    ),
                    row_style,
                ),
                Span::styled(
                    format!("{:<w$}", priority_text, w = PRIORITY_WIDTH),
                    p_style,
                ),
                Span::styled(time_short, row_style),
            ]));
        }

        // ── 底部信息 ───────────────────────────────────────────────────
        lines.push(Line::from(""));
        let selected_id = self
            .tasks
            .get(self.selected_index)
            .map(|t| &t.id[..t.id.len().min(8)]);
        let footer = if let Some(id) = selected_id {
            format!("  共 {} 个任务，选中: {}", self.tasks.len(), id)
        } else {
            format!("  共 {} 个任务", self.tasks.len())
        };
        lines.push(Line::from(Span::styled(footer, Style::default().fg(MUTED))));

        let widget = ratatui::widgets::Paragraph::new(lines);
        widget.render(area, buf);
    }
}

// ── 文本处理辅助 ───────────────────────────────────────────────────────────

/// 将字符串截断到指定字符宽度。
fn truncate_str(s: &str, max_len: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_len {
        s.to_owned()
    } else {
        let truncated: String = chars[..max_len.saturating_sub(1)].iter().collect();
        format!("{truncated}…")
    }
}

/// 将 ISO 8601 日期时间字符串格式化为短日期（MM-DD）。
fn format_created_at(datetime: &str) -> String {
    if datetime.len() >= 10 {
        datetime[5..10].to_owned()
    } else {
        datetime.to_owned()
    }
}

// ── 单元测试 ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_offset_zero_when_all_fit() {
        let tl = TaskList::new(&[], 0);
        assert_eq!(tl.scroll_offset(20), 0);
    }

    #[test]
    fn scroll_offset_adjusts_when_selected_below_visible() {
        let tasks: Vec<Task> = (0..10)
            .map(|i| Task {
                id: format!("{i:032}"),
                task_description: format!("任务{i}"),
                context_for_all_tasks: None,
                checklist: vec![],
                notes: vec![],
                resources: vec![],
                metadata: None,
                project_id: None,
                created_at: format!("2026-04-{i:02}T00:00:00Z"),
                updated_at: format!("2026-04-{i:02}T00:00:00Z"),
            })
            .collect();
        let tl = TaskList::new(&tasks, 5);
        assert_eq!(tl.scroll_offset(7), 3);
    }

    #[test]
    fn scroll_offset_zero_when_selected_in_view() {
        let tasks: Vec<Task> = (0..5)
            .map(|i| Task {
                id: format!("{i:032}"),
                task_description: format!("任务{i}"),
                context_for_all_tasks: None,
                checklist: vec![],
                notes: vec![],
                resources: vec![],
                metadata: None,
                project_id: None,
                created_at: format!("2026-04-{i:02}T00:00:00Z"),
                updated_at: format!("2026-04-{i:02}T00:00:00Z"),
            })
            .collect();
        let tl = TaskList::new(&tasks, 2);
        assert_eq!(tl.scroll_offset(20), 0);
    }

    #[test]
    fn truncate_str_short() {
        assert_eq!(truncate_str("abc", 5), "abc");
    }

    #[test]
    fn truncate_str_exact() {
        assert_eq!(truncate_str("abcde", 5), "abcde");
    }

    #[test]
    fn truncate_str_long() {
        assert_eq!(truncate_str("abcdef", 5), "abcd…");
    }

    #[test]
    fn format_created_at_iso() {
        assert_eq!(format_created_at("2026-04-03T18:30:00Z"), "04-03");
    }

    #[test]
    fn format_created_at_short() {
        assert_eq!(format_created_at("abc"), "abc");
    }
}
