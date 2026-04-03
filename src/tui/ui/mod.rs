//! TUI 渲染模块入口。
//!
//! 负责将 `App` 状态绘制到终端帧缓冲区。
//! 每个视图对应一个独立的渲染函数。

use ratatui::Frame;

use super::app::App;

// ── 公开渲染入口 ───────────────────────────────────────────────────────────

/// 顶层渲染入口：根据当前视图分发到对应的渲染函数。
///
/// 布局：
/// - 顶部：标题栏（项目名 + 当前视图名）
/// - 中间：主内容区（按 view 分发）
/// - 底部：状态栏（快捷键提示）
pub fn draw(f: &mut Frame, app: &App) {
    // TODO: 实现布局分割（标题栏 / 主内容 / 状态栏）
    match app.view() {
        super::app::View::TaskList => draw_task_list(f, app),
        super::app::View::TaskDetail => draw_task_detail(f, app),
        super::app::View::TaskForm => draw_task_form(f, app),
        super::app::View::Help => draw_help(f, app),
    }
}

// ── 视图渲染函数 ───────────────────────────────────────────────────────────

/// 渲染任务列表视图。
///
/// TODO: 实现
/// - 任务列表（List widget）
/// - 筛选/排序指示器
/// - 选中项高亮
fn draw_task_list(_f: &mut Frame, _app: &App) {
    // TODO: 实现 TaskList 视图
}

/// 渲染任务详情视图。
///
/// TODO: 实现
/// - 任务描述 + 元数据
/// - 清单条目列表
/// - 笔记列表
/// - 资源链接列表
fn draw_task_detail(_f: &mut Frame, _app: &App) {
    // TODO: 实现 TaskDetail 视图
}

/// 渲染任务创建/编辑表单。
///
/// TODO: 实现
/// - 表单字段（标题、描述、标签等）
/// - 输入光标定位
/// - 提交/取消提示
fn draw_task_form(_f: &mut Frame, _app: &App) {
    // TODO: 实现 TaskForm 视图
}

/// 渲染帮助面板。
///
/// TODO: 实现
/// - 快捷键列表
/// - 视图特定操作说明
fn draw_help(_f: &mut Frame, _app: &App) {
    // TODO: 实现 Help 视图
}
