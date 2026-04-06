//! 项目表单视图组件。
//!
//! 实现 ratatui `Widget` trait，渲染项目创建/编辑表单。
//! 支持名称（必填）+ 描述（可选）两个字段，Tab 切换聚焦。

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::tui::app::{ProjectFormField, ProjectFormMode, ProjectFormState};

use super::theme::*;

// ── 组件 ───────────────────────────────────────────────────────────────────

/// 项目表单组件。
///
/// 持有渲染所需的不可变数据引用，通过 `Widget::render` 绘制到指定区域。
pub struct ProjectForm<'a> {
    /// 表单状态。
    state: &'a ProjectFormState,
}

impl<'a> ProjectForm<'a> {
    /// 创建新的项目表单组件。
    pub fn new(state: &'a ProjectFormState) -> Self {
        Self { state }
    }
}

impl<'a> Widget for ProjectForm<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let is_edit = self.state.mode != ProjectFormMode::Create;
        let title_icon = if is_edit { ICON_EDIT } else { ICON_NEW };
        let title = if is_edit {
            " 编辑项目 "
        } else {
            " 新建项目 "
        };

        let total_width = area.width as usize;
        let label_width = 8;
        let field_width = total_width
            .saturating_sub(label_width)
            .saturating_sub(6)
            .max(20);

        let fields = [
            (
                ProjectFormField::Name,
                &self.state.name,
                self.state.focused_field == ProjectFormField::Name,
            ),
            (
                ProjectFormField::Description,
                &self.state.description,
                self.state.focused_field == ProjectFormField::Description,
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

            let marker = if *focused {
                ICON_SELECTED
            } else {
                ICON_UNSELECTED
            };

            lines.push(Line::from(vec![
                Span::styled(format!("  {marker} "), label_style),
                Span::styled(format!("{label}: "), label_style),
                Span::styled(truncate_str(&value_display, field_width), value_style),
            ]));

            // 空行分隔
            lines.push(Line::from(""));
        }

        // 校验错误
        if let Some(ref err) = self.state.error {
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {} ", ICON_ERROR),
                    Style::default().fg(ratatui::style::Color::Red),
                ),
                Span::styled(err.clone(), Style::default().fg(ratatui::style::Color::Red)),
            ]));
        }

        // 提示
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(
                "  Tab ",
                Style::default()
                    .fg(ratatui::style::Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("切换字段  "),
            Span::styled(
                "Enter ",
                Style::default()
                    .fg(ratatui::style::Color::Green)
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

        let paragraph = ratatui::widgets::Paragraph::new(lines).block(
            ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .border_style(Style::default().fg(BORDER_FOCUSED))
                .title(Span::styled(
                    format!(" {title_icon} {title}"),
                    Style::default().fg(ACCENT),
                )),
        );
        paragraph.render(area, buf);
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
    fn truncate_str_short() {
        assert_eq!(truncate_str("abc", 5), "abc");
    }

    #[test]
    fn truncate_str_long() {
        assert_eq!(truncate_str("abcdef", 5), "abcd…");
    }
}
