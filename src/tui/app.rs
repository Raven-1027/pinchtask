//! 应用状态管理。
//!
//! `App` 持有 TUI 全局可变状态，是事件处理与渲染之间的桥梁。
//! 所有 UI 渲染与事件处理围绕此结构体展开。
//! 通过 `TaskStore` 复用 core 层纯函数，不绕过业务逻辑。

use std::path::PathBuf;

use tokio::sync::mpsc::UnboundedSender;

use crate::models::task::Task;
use crate::store::TaskStore;

use super::event::{Action, AppEvent};

// ── 视图枚举 ───────────────────────────────────────────────────────────────

/// TUI 当前激活的视图。
#[derive(Debug, Clone, PartialEq)]
pub enum View {
    /// 任务列表视图
    TaskList,
    /// 任务详情视图（显示选定任务的完整信息）
    TaskDetail,
    /// 任务创建/编辑表单
    TaskForm,
    /// 帮助面板
    Help,
}

// ── 排序模式 ───────────────────────────────────────────────────────────────

/// 任务列表排序方式。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SortMode {
    /// 按创建时间排序（默认）
    Created,
    /// 按优先级排序（high > medium > low > 无）
    Priority,
    /// 按更新时间排序（最新优先）
    Updated,
}

impl SortMode {
    /// 切换到下一种排序方式。
    pub fn next(self) -> Self {
        match self {
            Self::Created => Self::Priority,
            Self::Priority => Self::Updated,
            Self::Updated => Self::Created,
        }
    }

    /// 返回排序方式的中文标签。
    pub fn label(self) -> &'static str {
        match self {
            Self::Created => "创建时间",
            Self::Priority => "优先级",
            Self::Updated => "更新时间",
        }
    }
}

// ── 应用状态 ───────────────────────────────────────────────────────────────

/// TUI 应用状态。
///
/// 所有 UI 渲染与事件处理围绕此结构体展开。
/// 通过 `TaskStore` 复用 core 层纯函数，不绕过业务逻辑。
pub struct App {
    /// 数据存储目录（由 CLI -D 参数传入）
    data_dir: Option<PathBuf>,
    /// 当前激活视图
    view: View,
    /// 进入帮助视图前的上一个视图（用于 Esc 返回）
    previous_view: View,
    /// 任务存储实例（延迟初始化）
    store: Option<TaskStore>,

    // ── 任务列表状态 ────────────────────────────────────────────────────
    /// 任务列表缓存（原始顺序，按创建时间）
    tasks: Vec<Task>,
    /// 当前选中行索引（基于过滤后的列表）
    selected_index: usize,
    /// 当前查看详情的任务
    current_task: Option<Task>,
    /// 列表滚动偏移量
    scroll_offset: usize,

    // ── 排序与搜索 ────────────────────────────────────────────────────
    /// 当前排序方式
    sort_mode: SortMode,
    /// 搜索关键词
    search_query: Option<String>,
    /// 是否处于搜索输入模式
    search_mode: bool,

    // ── 模态对话框 ────────────────────────────────────────────────────
    /// 是否显示删除确认对话框
    confirm_delete: bool,

    // ── 生命周期与消息 ──────────────────────────────────────────────────
    /// 退出标志
    should_quit: bool,
    /// 底部消息提示
    message: Option<String>,
    /// 错误消息
    error_message: Option<String>,

    // ── 异步事件发送端 ─────────────────────────────────────────────────
    /// 事件总线发送端，用于从异步任务回传 Action
    action_tx: Option<UnboundedSender<AppEvent>>,
}

impl App {
    /// 创建新的应用状态。
    pub fn new(data_dir: Option<PathBuf>) -> Self {
        Self {
            data_dir,
            view: View::TaskList,
            previous_view: View::TaskList,
            store: None,
            tasks: Vec::new(),
            selected_index: 0,
            current_task: None,
            scroll_offset: 0,
            sort_mode: SortMode::Created,
            search_query: None,
            search_mode: false,
            confirm_delete: false,
            should_quit: false,
            message: None,
            error_message: None,
            action_tx: None,
        }
    }

    // ── 访问器 ────────────────────────────────────────────────────────────

    /// 获取当前视图。
    pub fn view(&self) -> &View {
        &self.view
    }

    /// 是否应该退出主循环。
    pub fn should_quit(&self) -> bool {
        self.should_quit
    }

    /// 获取任务列表引用（原始数据）。
    pub fn tasks(&self) -> &[Task] {
        &self.tasks
    }

    /// 获取当前选中索引。
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// 获取消息提示。
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// 获取错误消息。
    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    /// 获取当前查看详情的任务。
    pub fn current_task(&self) -> Option<&Task> {
        self.current_task.as_ref()
    }

    /// 获取排序方式。
    pub fn sort_mode(&self) -> SortMode {
        self.sort_mode
    }

    /// 获取搜索查询词。
    pub fn search_query(&self) -> Option<&str> {
        self.search_query.as_deref()
    }

    /// 是否处于搜索模式。
    pub fn search_mode(&self) -> bool {
        self.search_mode
    }

    /// 是否显示删除确认对话框。
    pub fn confirm_delete(&self) -> bool {
        self.confirm_delete
    }

    /// 获取滚动偏移量。
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    // ── 事件发送端 ────────────────────────────────────────────────────────

    /// 设置事件总线发送端（由 mod.rs 在创建 EventBus 后调用）。
    pub fn set_action_tx(&mut self, tx: UnboundedSender<AppEvent>) {
        self.action_tx = Some(tx);
    }

    /// 发送 Action 事件到主循环。
    fn send_action(&self, action: Action) {
        if let Some(tx) = &self.action_tx {
            let _ = tx.send(AppEvent::Action(action));
        }
    }

    // ── 视图切换 ──────────────────────────────────────────────────────────

    /// 切换到指定视图。
    pub fn set_view(&mut self, view: View) {
        self.view = view;
    }

    /// 切换到帮助视图（记录来源视图以便返回）。
    fn show_help(&mut self) {
        if self.view != View::Help {
            self.previous_view = self.view.clone();
            self.view = View::Help;
        } else {
            // 已经在帮助视图，返回上一个视图
            self.view = self.previous_view.clone();
        }
    }

    // ── 存储初始化 ────────────────────────────────────────────────────────

    /// 确保存储已初始化并返回不可变引用。
    pub async fn store(&mut self) -> anyhow::Result<&TaskStore> {
        if self.store.is_none() {
            self.store = Some(TaskStore::new(self.data_dir.clone()).await?);
        }
        Ok(self.store.as_ref().unwrap())
    }

    /// 获取 store 的克隆（用于 tokio::spawn 异步任务）。
    ///
    /// 需要 store 已初始化。若未初始化返回 None。
    fn store_cloned(&self) -> Option<PathBuf> {
        // TaskStore 内部持有 SqlitePool (Clone)，这里通过重新创建来实现
        // 由于 TaskStore 没有 Clone derive，我们用 data_dir 来创建新的实例
        // 但更好的方式是在 spawn 中用 Arc<TaskStore>
        // 简化方案：将 data_dir 传给 spawn，让异步任务自己创建 store
        self.data_dir.clone()
    }

    // ── 数据加载（异步） ────────────────────────────────────────────────

    /// 加载所有任务到本地缓存。
    pub async fn load_tasks(&mut self) -> anyhow::Result<()> {
        let tasks = self.store().await?.list_tasks().await?;
        self.tasks = tasks;
        self.clamp_selected_index();
        self.message = Some(format!("已加载 {} 个任务", self.tasks.len()));
        self.error_message = None;
        Ok(())
    }

    /// 异步加载任务列表（通过 spawn + Action 回传）。
    pub fn spawn_load_tasks(&mut self) {
        let data_dir = self.store_cloned();
        if let Some(tx) = self.action_tx.clone() {
            tokio::spawn(async move {
                let store = TaskStore::new(data_dir).await;
                match store {
                    Ok(s) => match s.list_tasks().await {
                        Ok(tasks) => {
                            let _ = tx.send(AppEvent::Action(Action::TasksLoaded(tasks)));
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Action(Action::Error(
                                format!("加载任务列表失败: {e}"),
                            )));
                        }
                    },
                    Err(e) => {
                        let _ = tx
                            .send(AppEvent::Action(Action::Error(format!("数据库连接失败: {e}"))));
                    }
                }
            });
            self.message = Some("正在刷新任务列表...".to_owned());
        }
    }

    /// 异步加载任务详情（通过 spawn + Action 回传）。
    pub fn spawn_load_task_detail(&mut self, task_id: String) {
        let data_dir = self.store_cloned();
        if let Some(tx) = self.action_tx.clone() {
            tokio::spawn(async move {
                let store = TaskStore::new(data_dir).await;
                match store {
                    Ok(s) => match s.get_task(&task_id).await {
                        Ok(task) => {
                            let _ = tx.send(AppEvent::Action(Action::TaskDetailLoaded(task)));
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Action(Action::Error(
                                format!("加载任务详情失败: {e}"),
                            )));
                        }
                    },
                    Err(e) => {
                        let _ = tx
                            .send(AppEvent::Action(Action::Error(format!("数据库连接失败: {e}"))));
                    }
                }
            });
        }
    }

    /// 异步删除任务（通过 spawn + Action 回传）。
    pub fn spawn_delete_task(&mut self, task_id: String) {
        let data_dir = self.store_cloned();
        if let Some(tx) = self.action_tx.clone() {
            tokio::spawn(async move {
                let store = TaskStore::new(data_dir).await;
                match store {
                    Ok(s) => match s.delete_task(&task_id).await {
                        Ok(()) => {
                            let _ = tx.send(AppEvent::Action(Action::TaskDeleted(task_id)));
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::Action(Action::Error(
                                format!("删除任务失败: {e}"),
                            )));
                        }
                    },
                    Err(e) => {
                        let _ = tx
                            .send(AppEvent::Action(Action::Error(format!("数据库连接失败: {e}"))));
                    }
                }
            });
        }
    }

    // ── 排序与过滤 ──────────────────────────────────────────────────────

    /// 获取过滤并排序后的任务列表。
    ///
    /// 先按 search_query 过滤，再按 sort_mode 排序。
    pub fn filtered_and_sorted_tasks(&self) -> Vec<&Task> {
        let mut result: Vec<&Task> = self.tasks.iter().collect();

        // 搜索过滤
        if let Some(query) = &self.search_query {
            if !query.is_empty() {
                let query_lower = query.to_lowercase();
                result.retain(|task| {
                    task.task_description.to_lowercase().contains(&query_lower)
                        || task.id.to_lowercase().contains(&query_lower)
                });
            }
        }

        // 排序
        match self.sort_mode {
            SortMode::Created => {
                // 默认顺序即为创建时间顺序（store.list_tasks 已按 created_at ASC 排序）
                // 翻转为最新在前
                result.reverse();
            }
            SortMode::Priority => {
                result.sort_by(|a, b| {
                    let pa = priority_rank(
                        a.metadata.as_ref().and_then(|m| m.priority.as_deref()),
                    );
                    let pb = priority_rank(
                        b.metadata.as_ref().and_then(|m| m.priority.as_deref()),
                    );
                    pb.cmp(&pa) // 高优先级在前
                });
            }
            SortMode::Updated => {
                result.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            }
        }

        result
    }

    // ── 辅助方法 ────────────────────────────────────────────────────────

    /// 确保 selected_index 在有效范围内。
    fn clamp_selected_index(&mut self) {
        let filtered_count = self.filtered_and_sorted_tasks().len();
        if filtered_count == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= filtered_count {
            self.selected_index = filtered_count - 1;
        }
    }

    /// 设置退出标志。
    fn quit(&mut self) {
        self.should_quit = true;
    }

    /// 设置错误消息。
    pub fn set_error_message(&mut self, msg: String) {
        self.error_message = Some(msg);
    }

    // ── 事件处理 ────────────────────────────────────────────────────────

    /// 处理一个应用事件，更新内部状态。
    pub fn handle_event(&mut self, event: AppEvent) -> anyhow::Result<()> {
        match event {
            AppEvent::Key(key) => self.handle_key(key),
            AppEvent::Resize(_w, _h) => Ok(()),
            AppEvent::Action(action) => self.handle_action(action),
        }
    }

    /// 键盘事件分发。
    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers};

        // Ctrl+C 全局退出
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit();
            return Ok(());
        }

        // 搜索模式下只处理搜索相关按键
        if self.search_mode {
            return self.handle_search_key(key);
        }

        match key.code {
            KeyCode::Char('q') => {
                if self.view == View::Help {
                    self.view = self.previous_view.clone();
                } else if self.confirm_delete {
                    self.confirm_delete = false;
                } else {
                    self.quit();
                }
            }
            KeyCode::Char('?') => {
                if !self.confirm_delete {
                    self.show_help();
                }
            }
            KeyCode::Esc => {
                if self.confirm_delete {
                    self.confirm_delete = false;
                } else {
                    match self.view {
                        View::Help => self.view = self.previous_view.clone(),
                        View::TaskDetail | View::TaskForm => self.view = View::TaskList,
                        _ => {}
                    }
                }
            }
            _ if self.view == View::TaskList => {
                // 删除确认对话框优先处理
                if self.confirm_delete {
                    self.handle_delete_confirm_key(key);
                } else {
                    self.handle_task_list_key(key)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// 搜索模式下的键盘处理。
    fn handle_search_key(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<()> {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Esc => {
                self.search_mode = false;
                self.search_query = None;
                self.clamp_selected_index();
            }
            KeyCode::Enter => {
                self.search_mode = false;
                self.clamp_selected_index();
            }
            KeyCode::Backspace => {
                if let Some(q) = &mut self.search_query {
                    q.pop();
                    self.clamp_selected_index();
                }
            }
            KeyCode::Char(c) => {
                if let Some(q) = &mut self.search_query {
                    q.push(c);
                } else {
                    self.search_query = Some(c.to_string());
                }
                self.clamp_selected_index();
            }
            _ => {}
        }

        Ok(())
    }

    /// 删除确认对话框的键盘处理。
    fn handle_delete_confirm_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                // 确认删除
                if let Some(task) = self.filtered_and_sorted_tasks().get(self.selected_index) {
                    let task_id = task.id.clone();
                    self.confirm_delete = false;
                    self.spawn_delete_task(task_id);
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.confirm_delete = false;
            }
            _ => {}
        }
    }

    /// 任务列表视图的键盘处理。
    fn handle_task_list_key(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<()> {
        use crossterm::event::KeyCode;

        let filtered_count = self.filtered_and_sorted_tasks().len();

        match key.code {
            // 上下移动
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    self.adjust_scroll();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if filtered_count > 0 && self.selected_index < filtered_count - 1 {
                    self.selected_index += 1;
                    self.adjust_scroll();
                }
            }
            // 查看详情
            KeyCode::Enter => {
                if let Some(task) = self.filtered_and_sorted_tasks().get(self.selected_index) {
                    let task_id = task.id.clone();
                    self.spawn_load_task_detail(task_id);
                    self.message = Some("正在加载任务详情...".to_owned());
                }
            }
            // 新建任务
            KeyCode::Char('n') => {
                self.view = View::TaskForm;
            }
            // 删除任务（首次按下显示确认）
            KeyCode::Char('d') => {
                if !self.filtered_and_sorted_tasks().is_empty() {
                    self.confirm_delete = true;
                }
            }
            // 刷新列表
            KeyCode::Char('r') => {
                self.spawn_load_tasks();
            }
            // 切换排序方式
            KeyCode::Tab => {
                self.sort_mode = self.sort_mode.next();
                self.selected_index = 0;
                self.scroll_offset = 0;
                self.message = Some(format!("排序: {}", self.sort_mode.label()));
            }
            // 进入搜索模式
            KeyCode::Char('/') => {
                self.search_mode = true;
                self.search_query = Some(String::new());
                self.message = Some("输入搜索关键词，Esc 取消".to_owned());
            }
            // Home/End 快速跳转
            KeyCode::Home => {
                self.selected_index = 0;
                self.scroll_offset = 0;
            }
            KeyCode::End => {
                if filtered_count > 0 {
                    self.selected_index = filtered_count - 1;
                    self.adjust_scroll();
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// 根据选中索引调整滚动偏移量。
    fn adjust_scroll(&mut self) {
        // 简单实现：如果选中项超出可视范围，调整偏移
        // 具体 visible_height 在渲染时确定，这里用保守值 20
        let visible_height = 20;
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected_index - visible_height + 1;
        }
    }

    /// 异步操作结果处理。
    fn handle_action(&mut self, action: Action) -> anyhow::Result<()> {
        match action {
            Action::TasksLoaded(tasks) => {
                self.tasks = tasks;
                self.clamp_selected_index();
                self.message = Some(format!("已加载 {} 个任务", self.tasks.len()));
                self.error_message = None;
            }
            Action::TaskDetailLoaded(task) => {
                self.current_task = Some(task);
                self.view = View::TaskDetail;
                self.error_message = None;
            }
            Action::TaskDeleted(task_id) => {
                self.tasks.retain(|t| t.id != task_id);
                self.clamp_selected_index();
                self.message = Some("任务已删除".to_owned());
                self.error_message = None;
            }
            Action::ItemToggled(task) => {
                self.current_task = Some(task);
                self.message = Some("条目状态已切换".to_owned());
            }
            Action::ItemAdded(task) => {
                self.current_task = Some(task);
                self.message = Some("条目已添加".to_owned());
            }
            Action::ItemRemoved(task) => {
                self.current_task = Some(task);
                self.message = Some("条目已移除".to_owned());
            }
            Action::ItemEdited(task) => {
                self.current_task = Some(task);
                self.message = Some("条目已编辑".to_owned());
            }
            Action::ItemReordered(task) => {
                self.current_task = Some(task);
                self.message = Some("条目已重排".to_owned());
            }
            Action::Error(err) => {
                self.error_message = Some(err);
            }
        }
        Ok(())
    }
}

// ── 辅助函数 ───────────────────────────────────────────────────────────────

/// 将优先级字符串转换为排序权重（数值越大优先级越高）。
fn priority_rank(priority: Option<&str>) -> u8 {
    match priority {
        Some("high") => 3,
        Some("medium") => 2,
        Some("low") => 1,
        _ => 0,
    }
}
