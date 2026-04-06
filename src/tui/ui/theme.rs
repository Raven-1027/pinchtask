//! 统一视觉主题模块。
//!
//! 集中管理 TUI 所有视图的配色、图标与样式常量，
//! 确保视觉一致性并简化样式维护。

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

// ── 最小终端尺寸 ───────────────────────────────────────────────────────────

/// 最小终端宽度（字符列数）。
pub const MIN_WIDTH: u16 = 80;
/// 最小终端高度（字符行数）。
pub const MIN_HEIGHT: u16 = 24;

// ── 状态图标 ───────────────────────────────────────────────────────────────

/// 已完成（勾选）。
pub const ICON_DONE: &str = "✓";
/// 未完成（空心圆）。
pub const ICON_PENDING: &str = "○";
/// 选中行标记。
pub const ICON_SELECTED: &str = "▸";
/// 未选中行标记。
pub const ICON_UNSELECTED: &str = " ";
/// 列表圆点。
pub const ICON_BULLET: &str = "•";
/// 警告图标。
pub const ICON_WARN: &str = "⚠";
/// 编辑图标。
pub const ICON_EDIT: &str = "✏";
/// 新增图标。
pub const ICON_NEW: &str = "✚";
/// 错误图标。
pub const ICON_ERROR: &str = "✖";
/// 信息图标。
pub const ICON_INFO: &str = "ℹ";
/// 优先级标记：高。
pub const ICON_PRIORITY_HIGH: &str = "●";
/// 优先级标记：中。
pub const ICON_PRIORITY_MEDIUM: &str = "◑";
/// 优先级标记：低。
pub const ICON_PRIORITY_LOW: &str = "○";

// ── 配色常量 ───────────────────────────────────────────────────────────────

/// 标题栏背景色。
pub const TITLE_BG: Color = Color::DarkGray;

/// 默认边框色（非焦点）。
pub const BORDER: Color = Color::White;
/// 焦点边框色。
pub const BORDER_FOCUSED: Color = Color::Cyan;
/// 危险操作边框色。
pub const BORDER_DANGER: Color = Color::Red;

/// 标题/主文字强调色。
pub const ACCENT: Color = Color::Cyan;
/// 标签色（标签列表）。
pub const TAG: Color = Color::Magenta;
/// 链接色（资源 URL）。
pub const LINK: Color = Color::Blue;
/// 次要文本色。
pub const MUTED: Color = Color::DarkGray;
/// 正文色。
pub const TEXT: Color = Color::White;
/// 标题栏/状态栏选中高亮前景色。
pub const HIGHLIGHT_FG: Color = Color::Yellow;

// ── 优先级样式 ─────────────────────────────────────────────────────────────

/// 返回优先级对应的颜色。
pub fn priority_color(priority: &str) -> Color {
    match priority {
        "high" => Color::Red,
        "medium" => Color::Yellow,
        "low" => Color::Green,
        _ => MUTED,
    }
}

/// 返回优先级对应的图标。
pub fn priority_icon(priority: &str) -> &'static str {
    match priority {
        "high" => ICON_PRIORITY_HIGH,
        "medium" => ICON_PRIORITY_MEDIUM,
        "low" => ICON_PRIORITY_LOW,
        _ => " ",
    }
}

/// 返回优先级对应的 Span（带图标 + 文本 + 颜色）。
pub fn priority_span(priority: &str, width: usize) -> Span<'static> {
    let color = priority_color(priority);
    let icon = priority_icon(priority);
    let text = if priority == "-" || priority.is_empty() {
        format!("{:<w$}", "-", w = width)
    } else {
        format!(
            "{icon} {:<w$}",
            priority,
            w = width.saturating_sub(2).max(1)
        )
    };
    Span::styled(text, Style::default().fg(color))
}

// ── 行样式 ─────────────────────────────────────────────────────────────────

/// 选中行样式。
pub fn selected_style() -> Style {
    Style::default()
        .fg(HIGHLIGHT_FG)
        .bg(Color::Indexed(236)) // 深灰背景（比 DarkGray 更柔和）
        .add_modifier(Modifier::BOLD)
}

/// 已完成（全部条目完成）的行样式：灰色 + 删除线。
pub fn completed_style() -> Style {
    Style::default()
        .fg(Color::Indexed(244)) // 中灰，比 DarkGray 稍亮
        .add_modifier(Modifier::CROSSED_OUT)
}

/// 已完成行中优先级列的样式（保持颜色但加删除线）。
pub fn completed_priority_style(priority: &str) -> Style {
    Style::default()
        .fg(priority_color(priority))
        .add_modifier(Modifier::CROSSED_OUT)
}

/// 普通行样式。
pub fn normal_style() -> Style {
    Style::default()
}

// ── 区块标题样式 ───────────────────────────────────────────────────────────

/// 详情区区块标题样式。
pub fn section_title_style() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

/// 详情区标签样式（"ID:", "优先级:" 等前缀）。
pub fn label_style() -> Style {
    Style::default().fg(Color::Indexed(246)) // 浅灰
}

// ── 进度条 ─────────────────────────────────────────────────────────────────

/// 进度条填充字符。
const BAR_FILLED: char = '█';
/// 进度条空白字符。
const BAR_EMPTY: char = '░';

/// 生成带颜色的进度条 Span 列表。
///
/// 进度越高颜色越绿，越低越偏红/黄。宽度至少 2（两侧方括号）。
pub fn progress_bar_spans(pct: usize, width: usize) -> Vec<Span<'static>> {
    if width < 4 {
        // 极短时只显示百分比
        return vec![Span::styled(
            format!("{pct:>3}%"),
            Style::default().fg(bar_color(pct)),
        )];
    }

    let inner = width.saturating_sub(2); // 去掉 [ ]
    let filled = pct * inner / 100;
    let empty = inner.saturating_sub(filled);

    let filled_str = BAR_FILLED.to_string().repeat(filled);
    let empty_str = BAR_EMPTY.to_string().repeat(empty);

    vec![
        Span::styled("[", Style::default().fg(MUTED)),
        Span::styled(filled_str, Style::default().fg(bar_color(pct))),
        Span::styled(empty_str, Style::default().fg(Color::Indexed(236))),
        Span::styled("]", Style::default().fg(MUTED)),
    ]
}

/// 根据百分比返回进度条填充颜色。
///
/// - 0–25%: 红色
/// - 26–50%: 黄色
/// - 51–75%: 青色
/// - 76–100%: 绿色
fn bar_color(pct: usize) -> Color {
    match pct {
        0..=25 => Color::Red,
        26..=50 => Color::Yellow,
        51..=75 => Color::Cyan,
        _ => Color::Green,
    }
}

/// 生成纯文本进度条（不含颜色）。
pub fn progress_bar_plain(pct: usize, width: usize) -> String {
    if width < 2 {
        return String::new();
    }
    let filled = pct * (width - 2) / 100;
    let empty = (width - 2).saturating_sub(filled);
    format!(
        "[{}{}]",
        BAR_FILLED.to_string().repeat(filled),
        BAR_EMPTY.to_string().repeat(empty)
    )
}

// ── 表头样式 ───────────────────────────────────────────────────────────────

/// 列表表头样式。
pub fn header_style() -> Style {
    Style::default()
        .fg(Color::Indexed(248)) // 更亮的灰色
        .add_modifier(Modifier::BOLD)
}

// ── 分隔线 ─────────────────────────────────────────────────────────────────

/// 生成分隔线 Span。
pub fn separator(width: usize) -> Span<'static> {
    Span::styled(
        "─".repeat(width.max(1)),
        Style::default().fg(Color::Indexed(238)), // 略亮于 DarkGray 的灰
    )
}

// ── 复选框样式 ─────────────────────────────────────────────────────────────

/// 已勾选复选框文本（带绿色）。
pub fn checkbox_done() -> Span<'static> {
    Span::styled(
        format!(" {} ", ICON_DONE),
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
    )
}

/// 未勾选复选框文本（暗灰色）。
pub fn checkbox_pending() -> Span<'static> {
    Span::styled(
        format!(" {} ", ICON_PENDING),
        Style::default().fg(Color::Indexed(240)),
    )
}

/// 已勾选复选框文本 + 选中行高亮背景。
pub fn checkbox_done_selected() -> Span<'static> {
    Span::styled(
        format!(" {} ", ICON_DONE),
        Style::default()
            .fg(Color::Green)
            .bg(Color::Indexed(236))
            .add_modifier(Modifier::BOLD),
    )
}

/// 未勾选复选框文本 + 选中行高亮背景。
pub fn checkbox_pending_selected() -> Span<'static> {
    Span::styled(
        format!(" {} ", ICON_PENDING),
        Style::default()
            .fg(Color::Indexed(248))
            .bg(Color::Indexed(236)),
    )
}

// ── 单元测试 ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── priority_color 测试 ─────────────────────────────────────────────

    #[test]
    fn priority_color_known() {
        assert_eq!(priority_color("high"), Color::Red);
        assert_eq!(priority_color("medium"), Color::Yellow);
        assert_eq!(priority_color("low"), Color::Green);
    }

    #[test]
    fn priority_color_unknown_returns_muted() {
        assert_eq!(priority_color(""), MUTED);
        assert_eq!(priority_color("unknown"), MUTED);
        assert_eq!(priority_color("High"), MUTED); // 大小写敏感
    }

    // ── priority_icon 测试 ─────────────────────────────────────────────

    #[test]
    fn priority_icon_known() {
        assert_eq!(priority_icon("high"), ICON_PRIORITY_HIGH);
        assert_eq!(priority_icon("medium"), ICON_PRIORITY_MEDIUM);
        assert_eq!(priority_icon("low"), ICON_PRIORITY_LOW);
    }

    #[test]
    fn priority_icon_unknown_returns_space() {
        assert_eq!(priority_icon(""), " ");
        assert_eq!(priority_icon("urgent"), " ");
    }

    // ── priority_span 测试 ─────────────────────────────────────────────

    #[test]
    fn priority_span_contains_icon_and_text() {
        let span = priority_span("high", 10);
        assert!(span.content.contains(ICON_PRIORITY_HIGH));
        assert!(span.content.contains("high"));
    }

    #[test]
    fn priority_span_empty_shows_dash() {
        let span = priority_span("", 10);
        assert!(span.content.contains('-'));
    }

    #[test]
    fn priority_span_dash_shows_dash() {
        let span = priority_span("-", 10);
        assert!(span.content.contains('-'));
    }

    #[test]
    fn priority_span_has_correct_color() {
        let span = priority_span("high", 10);
        assert_eq!(span.style.fg, Some(Color::Red));

        let span = priority_span("medium", 10);
        assert_eq!(span.style.fg, Some(Color::Yellow));

        let span = priority_span("low", 10);
        assert_eq!(span.style.fg, Some(Color::Green));
    }

    // ── progress_bar_plain 测试 ─────────────────────────────────────────

    #[test]
    fn progress_bar_plain_zero_percent() {
        let bar = progress_bar_plain(0, 12);
        assert!(bar.starts_with('['));
        assert!(bar.ends_with(']'));
        assert!(!bar.contains(BAR_FILLED));
    }

    #[test]
    fn progress_bar_plain_full_percent() {
        let bar = progress_bar_plain(100, 12);
        assert!(bar.starts_with('['));
        assert!(bar.ends_with(']'));
        assert!(!bar.contains(BAR_EMPTY));
    }

    #[test]
    fn progress_bar_plain_fifty_percent() {
        let bar = progress_bar_plain(50, 12);
        assert!(bar.starts_with('['));
        assert!(bar.ends_with(']'));
        // 中间应混合填充和空白
        assert!(bar.contains(BAR_FILLED));
        assert!(bar.contains(BAR_EMPTY));
    }

    #[test]
    fn progress_bar_plain_length() {
        let bar = progress_bar_plain(50, 12);
        assert_eq!(bar.chars().count(), 12);
    }

    #[test]
    fn progress_bar_plain_too_narrow() {
        let bar = progress_bar_plain(50, 1);
        assert!(bar.is_empty());
    }

    #[test]
    fn progress_bar_plain_minimum_width() {
        let bar = progress_bar_plain(50, 2);
        assert_eq!(bar, "[]");
    }

    // ── progress_bar_spans 测试 ─────────────────────────────────────────

    #[test]
    fn progress_bar_spans_full_width() {
        let spans = progress_bar_spans(75, 20);
        // 应返回 4 个 span: [ filled empty ]
        assert_eq!(spans.len(), 4);
        assert_eq!(spans[0].content, "[");
        assert_eq!(spans[3].content, "]");
    }

    #[test]
    fn progress_bar_spans_narrow() {
        let spans = progress_bar_spans(50, 3);
        // 宽度 < 4 时只返回百分比文本
        assert_eq!(spans.len(), 1);
        assert!(spans[0].content.contains("50"));
    }

    #[test]
    fn progress_bar_spans_zero() {
        let spans = progress_bar_spans(0, 12);
        assert_eq!(spans.len(), 4);
        // 内部全为空白字符
        assert!(!spans[1].content.contains(BAR_FILLED));
    }

    #[test]
    fn progress_bar_spans_color_by_percent() {
        // 0% → Red
        let spans = progress_bar_spans(0, 12);
        assert_eq!(spans[1].style.fg, Some(Color::Red));

        // 50% → Yellow
        let spans = progress_bar_spans(50, 12);
        assert_eq!(spans[1].style.fg, Some(Color::Yellow));

        // 75% → Cyan
        let spans = progress_bar_spans(75, 12);
        assert_eq!(spans[1].style.fg, Some(Color::Cyan));

        // 100% → Green
        let spans = progress_bar_spans(100, 12);
        assert_eq!(spans[1].style.fg, Some(Color::Green));
    }

    // ── separator 测试 ──────────────────────────────────────────────────

    #[test]
    fn separator_length() {
        let span = separator(20);
        assert_eq!(span.content.chars().count(), 20);
    }

    #[test]
    fn separator_minimum_length() {
        // width.max(1) 确保至少 1 个字符
        let span = separator(0);
        assert_eq!(span.content.chars().count(), 1);
    }

    #[test]
    fn separator_content_is_dashes() {
        let span = separator(5);
        assert_eq!(span.content, "─────");
    }

    #[test]
    fn separator_has_color() {
        let span = separator(10);
        assert!(span.style.fg.is_some());
    }

    // ── bar_color 内部逻辑（通过公开函数间接测试） ────────────────────

    #[test]
    fn progress_bar_spans_red_for_low() {
        let spans = progress_bar_spans(10, 12);
        assert_eq!(spans[1].style.fg, Some(Color::Red));
    }

    #[test]
    fn progress_bar_spans_yellow_for_medium() {
        let spans = progress_bar_spans(30, 12);
        assert_eq!(spans[1].style.fg, Some(Color::Yellow));
    }

    // ── 常量一致性测试 ─────────────────────────────────────────────────

    #[test]
    fn icon_constants_non_empty() {
        assert!(!ICON_DONE.is_empty());
        assert!(!ICON_PENDING.is_empty());
        assert!(!ICON_SELECTED.is_empty());
        assert!(!ICON_BULLET.is_empty());
        assert!(!ICON_WARN.is_empty());
        assert!(!ICON_ERROR.is_empty());
        assert!(!ICON_INFO.is_empty());
    }

    #[test]
    fn min_size_constants_reasonable() {
        assert!(MIN_WIDTH >= 40);
        assert!(MIN_HEIGHT >= 10);
    }
}
