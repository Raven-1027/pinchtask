//! 项目列表视图组件。
//!
//! 实现 ratatui `Widget` trait，将项目列表渲染为带表头、选中高亮、
//! 描述截断、时间列的表格视图。支持自动滚动。

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::models::project::Project;

use super::theme::*;

// ── 常量 ───────────────────────────────────────────────────────────────────

/// 项目行固定前缀宽度：marker(3) = 3 字符。
const PREFIX_WIDTH: usize = 3;

/// 默认名称列宽度。
const DEFAULT_NAME_WIDTH: usize = 20;

/// 描述列固定宽度。
const DESC_WIDTH: usize = 30;

/// 时间列固定宽度："MM-DD"。
const TIME_WIDTH: usize = 6;

/// 表头 + 分隔线行数。
const HEADER_ROWS: usize = 2;

/// 底部信息行数。
const FOOTER_ROWS: usize = 2;

// ── 组件 ───────────────────────────────────────────────────────────────────

/// 项目列表组件。
///
/// 持有渲染所需的不可变数据引用，通过 `Widget::render` 绘制到指定区域。
pub struct ProjectList<'a> {
    /// 项目列表数据。
    projects: &'a [Project],
    /// 当前选中行索引。
    selected_index: usize,
}

impl<'a> ProjectList<'a> {
    /// 创建新的项目列表组件。
    pub fn new(projects: &'a [Project], selected_index: usize) -> Self {
        Self {
            projects,
            selected_index,
        }
    }

    /// 根据可视区域高度和选中索引计算滚动偏移。
    fn scroll_offset(&self, visible_height: usize) -> usize {
        if visible_height == 0 || self.projects.is_empty() {
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

impl<'a> Widget for ProjectList<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        // 空列表：显示占位提示
        if self.projects.is_empty() {
            let empty_msg = vec![
                Line::from(""),
                Line::styled(
                    "  暂无项目",
                    Style::default()
                        .fg(MUTED)
                        .add_modifier(Modifier::ITALIC),
                ),
                Line::from(""),
                Line::styled(
                    "  按 n 创建新项目，按 Esc 返回任务列表",
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
        let name_width = total_width
            .saturating_sub(PREFIX_WIDTH)
            .saturating_sub(DESC_WIDTH)
            .saturating_sub(TIME_WIDTH)
            .saturating_sub(4);
        let name_width = name_width.max(8).min(DEFAULT_NAME_WIDTH * 2);

        let mut lines: Vec<Line> = Vec::with_capacity(visible_height);

        // ── 表头 ───────────────────────────────────────────────────────
        lines.push(Line::from(vec![
            Span::styled("  # ", header_style()),
            Span::styled(format!("{:<w$}", "名称", w = name_width), header_style()),
            Span::styled(format!("{:<w$}", "描述", w = DESC_WIDTH), header_style()),
            Span::styled(format!("{:<w$}", "时间", w = TIME_WIDTH), header_style()),
        ]));

        lines.push(Line::from(separator(total_width.saturating_sub(2))));

        // ── 项目行 ─────────────────────────────────────────────────────
        let visible_projects = self
            .projects
            .iter()
            .enumerate()
            .skip(scroll)
            .take(usable_rows);

        for (i, project) in visible_projects {
            let is_selected = i == self.selected_index;

            let marker = if is_selected {
                ICON_SELECTED
            } else {
                ICON_UNSELECTED
            };
            let time_short = format_created_at(&project.created_at);
            let desc_display = project
                .description
                .as_deref()
                .unwrap_or("-");

            // 行样式
            let row_style = if is_selected {
                selected_style()
            } else {
                normal_style()
            };

            lines.push(Line::from(vec![
                Span::styled(format!(" {marker} "), row_style),
                Span::styled(
                    format!(
                        "{:<w$}",
                        truncate_str(&project.name, name_width),
                        w = name_width
                    ),
                    row_style,
                ),
                Span::styled(
                    format!("{:<w$}", truncate_str(desc_display, DESC_WIDTH), w = DESC_WIDTH),
                    row_style,
                ),
                Span::styled(time_short, row_style),
            ]));
        }

        // ── 底部信息 ───────────────────────────────────────────────────
        lines.push(Line::from(""));
        let footer = format!("  共 {} 个项目", self.projects.len());
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
        let pl = ProjectList::new(&[], 0);
        assert_eq!(pl.scroll_offset(20), 0);
    }

    #[test]
    fn scroll_offset_adjusts_when_selected_below_visible() {
        let projects: Vec<Project> = (0..10)
            .map(|i| Project {
                id: format!("{i:032}"),
                name: format!("项目{i}"),
                description: None,
                created_at: format!("2026-04-{i:02}T00:00:00Z"),
                updated_at: format!("2026-04-{i:02}T00:00:00Z"),
            })
            .collect();
        let pl = ProjectList::new(&projects, 5);
        assert_eq!(pl.scroll_offset(7), 3);
    }

    #[test]
    fn truncate_str_short() {
        assert_eq!(truncate_str("abc", 5), "abc");
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
