//! 项目详情视图组件。
//!
//! 实现 ratatui `Widget` trait，将单个项目的完整信息渲染到终端。
//! 布局分为：项目头部（名称/描述/时间）、关联任务列表（带进度/优先级）。

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::models::project::Project;
use crate::models::task::Task;

use super::theme::*;

// ── 组件 ───────────────────────────────────────────────────────────────────

/// 项目详情组件。
///
/// 持有渲染所需的不可变数据引用，通过 `Widget::render` 绘制到指定区域。
pub struct ProjectDetail<'a> {
    /// 当前查看的项目。
    project: &'a Project,
    /// 关联任务列表。
    tasks: &'a [Task],
    /// 关联任务当前焦点索引。
    selected_task_index: usize,
}

impl<'a> ProjectDetail<'a> {
    /// 创建新的项目详情组件。
    pub fn new(
        project: &'a Project,
        tasks: &'a [Task],
        selected_task_index: usize,
    ) -> Self {
        Self {
            project,
            tasks,
            selected_task_index,
        }
    }
}

impl<'a> Widget for ProjectDetail<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let total_width = area.width as usize;
        let mut lines: Vec<Line> = Vec::new();

        // ── 1. 项目头部 ────────────────────────────────────────────────
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(" 📦 ", Style::default().fg(ACCENT)),
            Span::styled(
                &self.project.name,
                Style::default()
                    .fg(ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        // 描述
        if let Some(ref desc) = self.project.description {
            let desc_lines = wrap_text(desc, total_width.saturating_sub(10));
            if let Some(first) = desc_lines.first() {
                lines.push(Line::from(vec![
                    Span::styled(" 描述:   ", label_style()),
                    Span::styled(first.clone(), Style::default().fg(TEXT)),
                ]));
                for extra_line in desc_lines.iter().skip(1) {
                    lines.push(Line::from(format!("         {extra_line}")));
                }
            }
        } else {
            lines.push(Line::from(vec![
                Span::styled(" 描述:   ", label_style()),
                Span::styled("（无描述）", Style::default().fg(MUTED)),
            ]));
        }

        // ID
        let id_short = &self.project.id[..self.project.id.len().min(8)];
        lines.push(Line::from(vec![
            Span::styled(" ID:     ", label_style()),
            Span::styled(id_short, Style::default().fg(MUTED)),
        ]));

        // 时间
        lines.push(Line::from(vec![
            Span::styled(" 创建:   ", label_style()),
            Span::styled(&self.project.created_at, Style::default().fg(MUTED)),
        ]));
        lines.push(Line::from(vec![
            Span::styled(" 更新:   ", label_style()),
            Span::styled(&self.project.updated_at, Style::default().fg(MUTED)),
        ]));

        // ── 2. 关联任务区 ─────────────────────────────────────────────
        lines.push(Line::from(""));

        let task_count = self.tasks.len();
        let mut title_spans = vec![
            Span::styled(
                format!(" ☑ 关联任务 ({task_count})"),
                section_title_style(),
            ),
        ];
        lines.push(Line::from(title_spans));
        lines.push(Line::from(separator(total_width.saturating_sub(2))));

        if self.tasks.is_empty() {
            lines.push(Line::styled(
                "   （暂无关联任务）",
                Style::default()
                    .fg(MUTED)
                    .add_modifier(Modifier::ITALIC),
            ));
        } else {
            let task_name_width = total_width.saturating_sub(20).max(10);
            for (i, task) in self.tasks.iter().enumerate() {
                let is_selected = i == self.selected_task_index;
                let marker = if is_selected {
                    ICON_SELECTED
                } else {
                    ICON_UNSELECTED
                };

                let done_count = task.checklist.iter().filter(|item| item.done).count();
                let total_items = task.checklist.len();
                let progress = format!("{done_count}/{total_items}");
                let all_done = total_items > 0 && done_count == total_items;

                let priority_text = task
                    .metadata
                    .as_ref()
                    .and_then(|m| m.priority.as_deref())
                    .unwrap_or("-");

                // 行样式
                let row_style = if is_selected {
                    selected_style()
                } else if all_done {
                    completed_style()
                } else {
                    normal_style()
                };

                let p_style = if is_selected {
                    Style::default().fg(priority_color(priority_text))
                } else if all_done {
                    completed_priority_style(priority_text)
                } else {
                    Style::default().fg(priority_color(priority_text))
                };

                lines.push(Line::from(vec![
                    Span::styled(format!(" {marker} "), row_style),
                    Span::styled(
                        format!(
                            "{:<w$}",
                            truncate_str(&task.task_description, task_name_width),
                            w = task_name_width
                        ),
                        row_style,
                    ),
                    Span::styled(format!("{progress:<6}"), row_style),
                    Span::styled(format!("{priority_text:<6}"), p_style),
                ]));
            }
        }

        // ── 3. 快捷键提示 ─────────────────────────────────────────────
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                " Enter ",
                Style::default()
                    .fg(ratatui::style::Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("查看任务  "),
            Span::styled(
                "E ",
                Style::default()
                    .fg(ratatui::style::Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("编辑项目  "),
            Span::styled(
                "d ",
                Style::default()
                    .fg(ratatui::style::Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("删除项目  "),
            Span::styled(
                "Esc ",
                Style::default()
                    .fg(HIGHLIGHT_FG)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("返回"),
        ]));

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

/// 将文本按指定宽度自动换行。
fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return vec![text.to_owned()];
    }
    let mut result = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch == '\n' {
            result.push(current.clone());
            current.clear();
        } else if current.chars().count() >= max_width {
            result.push(current.clone());
            current.clear();
            current.push(ch);
        } else {
            current.push(ch);
        }
    }
    if !current.is_empty() || result.is_empty() {
        result.push(current);
    }
    result
}

// ── 单元测试 ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_project() -> Project {
        Project {
            id: "0123456789abcdef".to_owned(),
            name: "测试项目".to_owned(),
            description: Some("项目描述信息".to_owned()),
            created_at: "2026-04-03T18:30:00Z".to_owned(),
            updated_at: "2026-04-04T10:00:00Z".to_owned(),
        }
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
    fn wrap_text_short() {
        assert_eq!(wrap_text("abc", 10), vec!["abc"]);
    }

    #[test]
    fn wrap_text_wraps() {
        assert_eq!(wrap_text("abcdefg", 4), vec!["abcd", "efg"]);
    }

    #[test]
    fn wrap_text_newline() {
        assert_eq!(wrap_text("ab\ncd", 10), vec!["ab", "cd"]);
    }
}
