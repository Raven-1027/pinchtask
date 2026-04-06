//! 项目列表组件（左栏窄列渲染）。
//!
//! 实现 ratatui `Widget` trait，将项目列表渲染为紧凑的窄列列表。
//! 仅显示项目名称，选中项目用 ▸ 标记。支持自动滚动。

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::models::project::Project;

use super::theme::*;

// ── 常量 ───────────────────────────────────────────────────────────────────

/// 项目行固定前缀宽度：marker(3) = 3 字符。
const PREFIX_WIDTH: usize = 3;

/// 底部信息行数。
const FOOTER_ROWS: usize = 1;

// ── 组件 ───────────────────────────────────────────────────────────────────

/// 项目列表组件（左栏窄列渲染）。
///
/// 持有渲染所需的不可变数据引用，通过 `Widget::render` 绘制到指定区域。
/// 适配窄列布局，仅显示项目名称，选中用 ▸ 标记。
pub struct ProjectList<'a> {
    /// 项目列表数据。
    projects: &'a [Project],
    /// 当前选中行索引。
    selected_index: usize,
    /// 是否处于焦点状态（影响样式）。
    is_focused: bool,
}

impl<'a> ProjectList<'a> {
    /// 创建新的项目列表组件。
    pub fn new(projects: &'a [Project], selected_index: usize, is_focused: bool) -> Self {
        Self {
            projects,
            selected_index,
            is_focused,
        }
    }

    /// 根据可视区域高度和选中索引计算滚动偏移。
    fn scroll_offset(&self, visible_height: usize) -> usize {
        if visible_height == 0 || self.projects.is_empty() {
            return 0;
        }
        let usable = visible_height.saturating_sub(FOOTER_ROWS);
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
                    Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
                ),
                Line::from(""),
                Line::styled("  按 n 创建新项目", Style::default().fg(MUTED)),
            ];
            let widget = ratatui::widgets::Paragraph::new(empty_msg);
            widget.render(area, buf);
            return;
        }

        let visible_height = area.height as usize;
        let scroll = self.scroll_offset(visible_height);
        let usable_rows = visible_height.saturating_sub(FOOTER_ROWS);

        let total_width = area.width as usize;
        let name_width = total_width.saturating_sub(PREFIX_WIDTH).max(4);

        let mut lines: Vec<Line> = Vec::with_capacity(visible_height);

        // ── 项目行（紧凑列表，无表头） ────────────────────────────────
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

            // 行样式：选中时高亮，非选中时根据焦点状态调整
            let row_style = if is_selected {
                selected_style()
            } else if self.is_focused {
                normal_style()
            } else {
                Style::default().fg(MUTED)
            };

            lines.push(Line::from(vec![
                Span::styled(format!(" {marker} "), row_style),
                Span::styled(truncate_str(&project.name, name_width), row_style),
            ]));
        }

        // ── 底部信息 ───────────────────────────────────────────────────
        let footer = format!("  {} 个项目", self.projects.len());
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

// ── 单元测试 ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_offset_zero_when_all_fit() {
        let pl = ProjectList::new(&[], 0, true);
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
        let pl = ProjectList::new(&projects, 5, true);
        assert_eq!(pl.scroll_offset(6), 1);
    }

    #[test]
    fn truncate_str_short() {
        assert_eq!(truncate_str("abc", 5), "abc");
    }

    #[test]
    fn truncate_str_long() {
        assert_eq!(truncate_str("abcdef", 5), "abcd…");
    }
}
