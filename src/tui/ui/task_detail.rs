//! 任务详情视图组件。
//!
//! 实现 ratatui `Widget` trait，将单个任务的完整信息渲染到终端。
//! 布局分为：任务头部（ID/描述/上下文）、元数据区、清单区（可滚动、带焦点）、
//! 笔记区、资源区。清单条目支持焦点高亮，选中行自动滚动到可视范围内。

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::models::task::Task;

use super::theme::*;

// ── 常量 ───────────────────────────────────────────────────────────────────

/// 清单条目前缀宽度：marker(3) + checkbox(3) = 6 字符（使用图标后宽度变化）。
const ITEM_PREFIX_WIDTH: usize = 7;

/// 清单条目名称默认最大宽度。
const DEFAULT_ITEM_NAME_WIDTH: usize = 40;

// ── 组件 ───────────────────────────────────────────────────────────────────

/// 任务详情组件。
///
/// 持有渲染所需的不可变数据引用，通过 `Widget::render` 绘制到指定区域。
/// 不持有可变状态，滚动偏移根据 `selected_item_index` 和可视高度自动计算。
pub struct TaskDetail<'a> {
    /// 当前查看的任务。
    task: &'a Task,
    /// 清单条目当前焦点索引。
    selected_item_index: usize,
}

impl<'a> TaskDetail<'a> {
    /// 创建新的任务详情组件。
    pub fn new(task: &'a Task, selected_item_index: usize) -> Self {
        Self {
            task,
            selected_item_index,
        }
    }

    /// 根据可视区域高度和选中索引计算清单区滚动偏移。
    fn scroll_offset(&self, visible_item_rows: usize) -> usize {
        if visible_item_rows == 0 || self.task.checklist.is_empty() {
            return 0;
        }
        if self.selected_item_index >= visible_item_rows {
            self.selected_item_index - visible_item_rows + 1
        } else {
            0
        }
    }
}

impl<'a> Widget for TaskDetail<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        let total_height = area.height as usize;
        let total_width = area.width as usize;
        let mut lines: Vec<Line> = Vec::with_capacity(total_height);

        // ── 1. 任务头部：描述 + ID + 上下文 ────────────────────────────
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(" 📋 ", Style::default().fg(ACCENT)),
            Span::styled(
                &self.task.task_description,
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            ),
        ]));
        let id_short = &self.task.id[..self.task.id.len().min(8)];
        lines.push(Line::from(vec![
            Span::styled(" ID:    ", label_style()),
            Span::styled(id_short, Style::default().fg(MUTED)),
        ]));

        // 上下文
        if let Some(ref ctx) = self.task.context_for_all_tasks {
            let ctx_lines = wrap_text(ctx, total_width.saturating_sub(10));
            if let Some(first) = ctx_lines.first() {
                lines.push(Line::from(vec![
                    Span::styled(" 上下文: ", label_style()),
                    Span::styled(first.clone(), Style::default().fg(TEXT)),
                ]));
                for extra_line in ctx_lines.iter().skip(1) {
                    lines.push(Line::from(format!("         {extra_line}")));
                }
            }
        }

        // ── 2. 元数据区 ───────────────────────────────────────────────
        if let Some(ref meta) = self.task.metadata {
            lines.push(Line::from(""));

            // 优先级（带图标）
            if let Some(ref priority) = meta.priority {
                lines.push(Line::from(vec![
                    Span::styled(" 优先级: ", label_style()),
                    priority_span(priority, 0),
                ]));
            }

            // 标签
            if let Some(ref tags) = meta.tags {
                if !tags.is_empty() {
                    let tags_str = tags.join(", ");
                    lines.push(Line::from(vec![
                        Span::styled(" 标签:   ", label_style()),
                        Span::styled(tags_str, Style::default().fg(TAG)),
                    ]));
                }
            }

            // 预计完成时间
            if let Some(ref eta) = meta.estimated_completion_time {
                lines.push(Line::from(vec![
                    Span::styled(" ETA:    ", label_style()),
                    Span::styled(eta, Style::default().fg(LINK)),
                ]));
            }
        }

        // ── 3. 清单区 ─────────────────────────────────────────────────
        lines.push(Line::from(""));
        let done_count = self.task.checklist.iter().filter(|i| i.done).count();
        let total_items = self.task.checklist.len();

        // 清单标题行（带进度条）
        let mut title_spans = vec![
            Span::styled(
                format!(" ☑ 清单 ({done_count}/{total_items})"),
                section_title_style(),
            ),
            Span::raw("  "),
        ];
        let pct = if total_items > 0 {
            done_count * 100 / total_items
        } else {
            0
        };
        title_spans.extend(progress_bar_spans(pct, 10));
        lines.push(Line::from(title_spans));

        // 分隔线
        lines.push(Line::from(separator(total_width.saturating_sub(2))));

        if self.task.checklist.is_empty() {
            lines.push(Line::styled(
                "   （无清单条目，按 a 添加）",
                Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
            ));
        } else {
            // 计算清单区可用行数
            let reserved_for_notes_resources = 6;
            let remaining = total_height.saturating_sub(lines.len() + reserved_for_notes_resources);
            let max_visible_items = remaining.max(3);

            let scroll = self.scroll_offset(max_visible_items);
            let visible_items = self
                .task
                .checklist
                .iter()
                .enumerate()
                .skip(scroll)
                .take(max_visible_items);

            // 动态条目名称宽度
            let item_name_width = total_width
                .saturating_sub(ITEM_PREFIX_WIDTH)
                .saturating_sub(4);
            let item_name_width = item_name_width.max(10).min(DEFAULT_ITEM_NAME_WIDTH * 2);

            for (i, item) in visible_items {
                let is_selected = i == self.selected_item_index;
                let marker = if is_selected {
                    ICON_SELECTED
                } else {
                    ICON_UNSELECTED
                };

                // 行样式
                let row_style = if is_selected {
                    selected_style()
                } else if item.done {
                    completed_style()
                } else {
                    normal_style()
                };

                // 复选框样式
                let checkbox_span = if item.done {
                    if is_selected {
                        checkbox_done_selected()
                    } else {
                        checkbox_done()
                    }
                } else {
                    if is_selected {
                        checkbox_pending_selected()
                    } else {
                        checkbox_pending()
                    }
                };

                let name = truncate_str(&item.task, item_name_width);

                lines.push(Line::from(vec![
                    Span::styled(format!(" {marker} "), row_style),
                    checkbox_span,
                    Span::styled(format!("{:<w$}", name, w = item_name_width), row_style),
                ]));

                // 选中时显示详细描述和上下文计划
                if is_selected {
                    if !item.detailed_description.is_empty() {
                        let desc_lines =
                            wrap_text(&item.detailed_description, total_width.saturating_sub(8));
                        for dl in desc_lines {
                            lines.push(Line::from(vec![
                                Span::styled("       ", Style::default()),
                                Span::styled(dl, Style::default().fg(TEXT)),
                            ]));
                        }
                    }
                    if let Some(ref plan) = item.context_and_plan {
                        let plan_lines = wrap_text(plan, total_width.saturating_sub(8));
                        for pl in plan_lines {
                            lines.push(Line::from(vec![
                                Span::styled("       ", Style::default()),
                                Span::styled(pl, Style::default().fg(LINK)),
                            ]));
                        }
                    }
                }
            }
        }

        // ── 4. 笔记区 ─────────────────────────────────────────────────
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            format!(" 📝 笔记 ({})", self.task.notes.len()),
            Style::default()
                .fg(HIGHLIGHT_FG)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(separator(total_width.saturating_sub(2))));

        if self.task.notes.is_empty() {
            lines.push(Line::styled(
                "   （暂无笔记）",
                Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
            ));
        } else {
            for (i, note) in self.task.notes.iter().enumerate() {
                let note_width = total_width.saturating_sub(8);
                let wrapped = wrap_text(note, note_width);
                for (li, line) in wrapped.into_iter().enumerate() {
                    if li == 0 {
                        lines.push(Line::from(vec![
                            Span::styled(format!(" {}. ", i + 1), Style::default().fg(MUTED)),
                            Span::raw(line),
                        ]));
                    } else {
                        lines.push(Line::from(format!("    {line}")));
                    }
                }
            }
        }

        // ── 5. 资源区 ─────────────────────────────────────────────────
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            format!(" 🔗 资源 ({})", self.task.resources.len()),
            Style::default().fg(LINK).add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(separator(total_width.saturating_sub(2))));

        if self.task.resources.is_empty() {
            lines.push(Line::styled(
                "   （暂无关联资源）",
                Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
            ));
        } else {
            let url_width = total_width.saturating_sub(6);
            for res in &self.task.resources {
                let desc_suffix = res
                    .description
                    .as_ref()
                    .map(|d| format!(" — {d}"))
                    .unwrap_or_default();
                let entry = format!("{}: {}{desc_suffix}", res.name, res.url);
                let truncated = truncate_str(&entry, url_width);
                lines.push(Line::from(vec![
                    Span::styled(format!(" {ICON_BULLET} "), Style::default().fg(LINK)),
                    Span::styled(truncated, Style::default()),
                ]));
            }
        }

        // ── 6. 时间信息 ───────────────────────────────────────────────
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            format!(
                " 创建: {}  更新: {}",
                self.task.created_at, self.task.updated_at
            ),
            Style::default().fg(MUTED),
        )]));

        // 使用 Paragraph 渲染所有行
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
    use crate::models::task::{ChecklistItem, Resource, Task, TaskMetadata};

    /// 创建测试用任务。
    fn test_task() -> Task {
        Task {
            id: "0123456789abcdef".to_owned(),
            task_description: "测试任务描述".to_owned(),
            context_for_all_tasks: Some("共享上下文信息".to_owned()),
            checklist: vec![
                ChecklistItem {
                    id: "item-1".to_owned(),
                    task: "条目一".to_owned(),
                    detailed_description: "详细描述一".to_owned(),
                    context_and_plan: Some("计划一".to_owned()),
                    done: false,
                },
                ChecklistItem {
                    id: "item-2".to_owned(),
                    task: "条目二".to_owned(),
                    detailed_description: "详细描述二".to_owned(),
                    context_and_plan: None,
                    done: true,
                },
                ChecklistItem {
                    id: "item-3".to_owned(),
                    task: "条目三".to_owned(),
                    detailed_description: String::new(),
                    context_and_plan: None,
                    done: false,
                },
            ],
            notes: vec!["笔记一".to_owned(), "笔记二".to_owned()],
            resources: vec![Resource {
                name: "文档".to_owned(),
                url: "https://example.com".to_owned(),
                description: Some("参考文档".to_owned()),
            }],
            metadata: Some(TaskMetadata {
                tags: Some(vec!["重要".to_owned(), "v2".to_owned()]),
                priority: Some("high".to_owned()),
                estimated_completion_time: Some("P3D".to_owned()),
            }),
            project_id: None,
            created_at: "2026-04-03T18:30:00Z".to_owned(),
            updated_at: "2026-04-04T10:00:00Z".to_owned(),
        }
    }

    #[test]
    fn scroll_offset_zero_when_all_fit() {
        let task = test_task();
        let td = TaskDetail::new(&task, 0);
        assert_eq!(td.scroll_offset(10), 0);
    }

    #[test]
    fn scroll_offset_adjusts_when_selected_below_visible() {
        let task = test_task();
        let td = TaskDetail::new(&task, 5);
        assert_eq!(td.scroll_offset(3), 3);
    }

    #[test]
    fn scroll_offset_zero_when_selected_in_view() {
        let task = test_task();
        let td = TaskDetail::new(&task, 2);
        assert_eq!(td.scroll_offset(5), 0);
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
    fn wrap_text_short() {
        assert_eq!(wrap_text("abc", 10), vec!["abc"]);
    }

    #[test]
    fn wrap_text_exact_width() {
        assert_eq!(wrap_text("abcde", 5), vec!["abcde"]);
    }

    #[test]
    fn wrap_text_wraps() {
        assert_eq!(wrap_text("abcdefg", 4), vec!["abcd", "efg"]);
    }

    #[test]
    fn wrap_text_newline() {
        assert_eq!(wrap_text("ab\ncd", 10), vec!["ab", "cd"]);
    }

    #[test]
    fn wrap_text_empty() {
        assert_eq!(wrap_text("", 10), vec![""]);
    }
}
