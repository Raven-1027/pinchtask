//! 应用状态管理。
//!
//! `App` 持有 TUI 全局可变状态，是事件处理与渲染之间的桥梁。
//! 所有 UI 渲染与事件处理围绕此结构体展开。
//! 通过 `TaskStore` 复用 core 层纯函数，不绕过业务逻辑。

use std::path::PathBuf;

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
    /// 任务列表缓存
    tasks: Vec<Task>,
    /// 当前选中行索引
    selected_index: usize,
    /// 当前查看详情的任务
    current_task: Option<Task>,

    // ── 任务详情状态 ────────────────────────────────────────────────────
    /// 清单条目焦点索引（TaskDetail 视图使用）
    detail_item_index: usize,

    // ── 生命周期与消息 ──────────────────────────────────────────────────
    /// 退出标志
    should_quit: bool,
    /// 底部消息提示
    message: Option<String>,
    /// 错误消息
    error_message: Option<String>,
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
            detail_item_index: 0,
            should_quit: false,
            message: None,
            error_message: None,
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

    /// 获取任务列表引用。
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

    /// 获取详情视图清单焦点索引。
    pub fn detail_item_index(&self) -> usize {
        self.detail_item_index
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
    ///
    /// 首次调用时创建数据库连接，后续调用复用现有连接。
    pub async fn store(&mut self) -> anyhow::Result<&TaskStore> {
        if self.store.is_none() {
            self.store = Some(TaskStore::new(self.data_dir.clone()).await?);
        }
        Ok(self.store.as_ref().unwrap())
    }

    // ── 数据加载 ──────────────────────────────────────────────────────────

    /// 加载所有任务到本地缓存。
    ///
    /// 直接调用 `store.list_tasks()` 获取完整任务列表，
    /// 复用 core 层逻辑（core::list_tasks_summary 底层也调用此方法）。
    pub async fn load_tasks(&mut self) -> anyhow::Result<()> {
        let tasks = self.store().await?.list_tasks().await?;
        self.tasks = tasks;
        // 确保选中索引不越界
        if !self.tasks.is_empty() {
            self.selected_index = self.selected_index.min(self.tasks.len() - 1);
        } else {
            self.selected_index = 0;
        }
        self.message = Some(format!("已加载 {} 个任务", self.tasks.len()));
        self.error_message = None;
        Ok(())
    }

    /// 加载指定任务的完整详情。
    ///
    /// 通过 `store.get_task()` 获取任务，然后切换到 TaskDetail 视图。
    pub async fn load_task_detail(&mut self, task_id: String) -> anyhow::Result<()> {
        let task = self.store().await?.get_task(&task_id).await?;
        self.current_task = Some(task);
        self.view = View::TaskDetail;
        self.error_message = None;
        Ok(())
    }

    // ── 退出 ──────────────────────────────────────────────────────────────

    /// 设置错误消息（供外部模块如 mod.rs 调用）。
    pub fn set_error_message(&mut self, msg: String) {
        self.error_message = Some(msg);
    }

    /// 设置退出标志，主循环将在下一轮迭代退出。
    fn quit(&mut self) {
        self.should_quit = true;
    }

    // ── 事件处理 ──────────────────────────────────────────────────────────

    /// 处理一个应用事件，更新内部状态。
    ///
    /// 事件分发逻辑：
    /// - AppEvent::Key → 键盘事件处理（导航/视图切换/退出）
    /// - AppEvent::Resize → 自动由 ratatui 处理（无需额外逻辑）
    /// - AppEvent::Action → 异步操作结果更新
    pub fn handle_event(&mut self, event: AppEvent) -> anyhow::Result<()> {
        match event {
            AppEvent::Key(key) => self.handle_key(key),
            AppEvent::Resize(_w, _h) => {
                // ratatui 在 draw 时自动适配新尺寸，无需额外处理
                Ok(())
            }
            AppEvent::Action(action) => self.handle_action(action),
        }
    }

    /// 键盘事件分发：根据当前视图和按键组合更新状态。
    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers};

        // ── 全局快捷键 ─────────────────────────────────────────────────
        // Ctrl+C 在任何视图下都退出
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && key.code == KeyCode::Char('c')
        {
            self.quit();
            return Ok(());
        }

        match key.code {
            // q 键：在 Help 视图中返回上一视图，否则退出
            KeyCode::Char('q') => {
                if self.view == View::Help {
                    self.view = self.previous_view.clone();
                } else {
                    self.quit();
                }
            }
            // ? 键：切换帮助视图
            KeyCode::Char('?') => {
                self.show_help();
            }
            // Esc 键：帮助视图返回，详情视图返回列表
            KeyCode::Esc => {
                match self.view {
                    View::Help => self.view = self.previous_view.clone(),
                    View::TaskDetail | View::TaskForm => self.view = View::TaskList,
                    _ => {}
                }
            }
            // ── TaskList 视图按键 ──────────────────────────────────────
            _ if self.view == View::TaskList => {
                self.handle_task_list_key(key)?;
            }
            // ── TaskDetail 视图按键 ───────────────────────────────────
            _ if self.view == View::TaskDetail => {
                self.handle_task_detail_key(key)?;
            }
            // ── 其他视图按键（后续子任务实现）─────────────────────────
            _ => {}
        }

        Ok(())
    }

    /// 任务列表视图的键盘处理。
    fn handle_task_list_key(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> anyhow::Result<()> {
        use crossterm::event::KeyCode;

        match key.code {
            // 上下移动选中
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.tasks.is_empty() && self.selected_index < self.tasks.len() - 1 {
                    self.selected_index += 1;
                }
            }
            // 查看任务详情：从缓存中复制选中任务，切换到 TaskDetail 视图
            KeyCode::Enter => {
                if let Some(task) = self.tasks.get(self.selected_index).cloned() {
                    self.current_task = Some(task);
                    self.detail_item_index = 0;
                    self.view = View::TaskDetail;
                    self.error_message = None;
                }
            }
            // 新建任务：切换到 TaskForm 视图
            KeyCode::Char('n') => {
                self.view = View::TaskForm;
                self.message = None;
                self.error_message = None;
            }
            // 删除选中任务：设置确认提示（实际删除需异步操作，后续子任务实现）
            KeyCode::Char('d') => {
                if let Some(task) = self.tasks.get(self.selected_index) {
                    let id_short = &task.id[..task.id.len().min(8)];
                    self.message = Some(format!(
                        "删除功能待实现: 选中任务 {id_short} ({})",
                        task.task_description
                    ));
                }
            }
            // 刷新列表（标记为需要重新加载）
            KeyCode::Char('r') => {
                self.message = Some("按 r 刷新列表（异步加载中...）".to_owned());
            }
            _ => {}
        }

        Ok(())
    }

    /// 任务详情视图的键盘处理。
    ///
    /// 支持的操作：
    /// - ↑/k：清单条目上移焦点
    /// - ↓/j：清单条目下移焦点
    /// - Space/x：切换条目完成状态（本地缓存更新）
    /// - Esc/←/Backspace：返回任务列表
    /// - d：删除当前焦点条目（消息提示，异步持久化待集成）
    /// - a：添加清单条目（消息提示，表单待实现）
    fn handle_task_detail_key(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> anyhow::Result<()> {
        use crossterm::event::KeyCode;

        match key.code {
            // 上下移动清单条目焦点
            KeyCode::Up | KeyCode::Char('k') => {
                if self.detail_item_index > 0 {
                    self.detail_item_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref task) = self.current_task {
                    if !task.checklist.is_empty()
                        && self.detail_item_index < task.checklist.len() - 1
                    {
                        self.detail_item_index += 1;
                    }
                }
            }
            // Space/x 切换条目完成状态
            KeyCode::Char(' ') | KeyCode::Char('x') => {
                if let Some(ref mut task) = self.current_task {
                    if let Some(item) = task.checklist.get_mut(self.detail_item_index) {
                        item.done = !item.done;
                        let status = if item.done { "完成" } else { "撤销" };
                        self.message = Some(format!(
                            "条目 '{}': {}",
                            task.checklist[self.detail_item_index].task,
                            status
                        ));
                        // 注意：此处仅更新本地缓存，异步持久化需在主循环集成
                    }
                }
            }
            // d 删除当前焦点条目
            KeyCode::Char('d') => {
                if let Some(ref task) = self.current_task {
                    if let Some(item) = task.checklist.get(self.detail_item_index) {
                        self.message = Some(format!(
                            "删除功能待实现: 选中条目 '{}'（索引 {}）",
                            item.task, self.detail_item_index
                        ));
                    }
                }
            }
            // a 添加清单条目
            KeyCode::Char('a') => {
                self.message = Some("添加清单条目表单待实现".to_owned());
            }
            // Esc/←/Backspace 返回任务列表
            KeyCode::Esc | KeyCode::Left | KeyCode::Backspace => {
                self.view = View::TaskList;
                self.message = None;
                self.error_message = None;
            }
            _ => {}
        }

        Ok(())
    }

    /// 异步操作结果处理。
    fn handle_action(&mut self, action: Action) -> anyhow::Result<()> {
        match action {
            Action::TasksLoaded(tasks) => {
                self.tasks = tasks;
                if !self.tasks.is_empty() {
                    self.selected_index = self.selected_index.min(self.tasks.len() - 1);
                }
                self.message = Some(format!("已加载 {} 个任务", self.tasks.len()));
                self.error_message = None;
            }
            Action::TaskDetailLoaded(task) => {
                self.current_task = Some(task);
                self.detail_item_index = 0;
                self.view = View::TaskDetail;
                self.error_message = None;
            }
            Action::ItemToggled => {
                self.message = Some("条目状态已切换".to_owned());
            }
            Action::ItemAdded => {
                self.message = Some("条目已添加".to_owned());
            }
            Action::ItemRemoved => {
                self.message = Some("条目已移除".to_owned());
            }
            Action::ItemEdited => {
                self.message = Some("条目已编辑".to_owned());
            }
            Action::Error(err) => {
                self.error_message = Some(err);
            }
        }
        Ok(())
    }
}
