//! 应用状态管理。
//!
//! `App` 持有 TUI 全局可变状态，是事件处理与渲染之间的桥梁。
//! 所有 UI 渲染与事件处理围绕此结构体展开。
//! 通过 `TaskStore` 复用 core 层纯函数，不绕过业务逻辑。

use std::path::PathBuf;

use tokio::sync::mpsc::UnboundedSender;

use crate::models::project::Project;
use crate::models::task::Task;
use crate::store::TaskStore;

use super::event::{Action, AppEvent};

// ── 输入模式 ───────────────────────────────────────────────────────────────

/// 清单条目输入模式。
#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    /// 正常模式（键盘导航）
    Normal,
    /// 添加新条目（行内输入）
    AddingItem,
    /// 编辑当前条目名称（行内输入）
    EditingItemName,
    /// 编辑当前条目描述（行内输入）
    EditingItemDesc,
    /// 添加笔记（行内输入）
    AddingNote,
    /// 添加资源 — 输入名称（第一步）
    AddingResourceName,
    /// 添加资源 — 输入 URL（第二步）
    AddingResourceUrl,
}

// ── 表单字段 ─────────────────────────────────────────────────────────────────

/// 任务表单中可聚焦的字段。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FormField {
    /// 任务描述（必填）
    Description,
    /// 共享上下文
    Context,
    /// 优先级 (high / medium / low / 空)
    Priority,
    /// 标签（逗号分隔）
    Tags,
    /// 预计完成时间
    Eta,
}

impl FormField {
    /// 所有字段按 Tab 顺序排列。
    const ORDER: [FormField; 5] = [
        FormField::Description,
        FormField::Context,
        FormField::Priority,
        FormField::Tags,
        FormField::Eta,
    ];

    /// 返回下一个字段（循环）。
    pub fn next(self) -> Self {
        let idx = self.index();
        Self::ORDER[(idx + 1) % Self::ORDER.len()]
    }

    /// 返回上一个字段（循环）。
    pub fn prev(self) -> Self {
        let idx = self.index();
        if idx == 0 {
            Self::ORDER[Self::ORDER.len() - 1]
        } else {
            Self::ORDER[idx - 1]
        }
    }

    /// 字段在 ORDER 中的索引。
    fn index(self) -> usize {
        Self::ORDER.iter().position(|&f| f == self).unwrap()
    }

    /// 返回字段的中文标签。
    pub fn label(self) -> &'static str {
        match self {
            FormField::Description => "描述",
            FormField::Context => "上下文",
            FormField::Priority => "优先级",
            FormField::Tags => "标签",
            FormField::Eta => "预计完成",
        }
    }

    /// 返回字段的占位提示。
    pub fn placeholder(self) -> &'static str {
        match self {
            FormField::Description => "输入任务描述...",
            FormField::Context => "输入共享上下文（可选）...",
            FormField::Priority => "Space/←/→ 切换: - / low / medium / high",
            FormField::Tags => "逗号分隔，如: 重要, v2",
            FormField::Eta => "如: 2h, P3D, 2025-12-31",
        }
    }
}

// ── 表单模式 ─────────────────────────────────────────────────────────────────

/// 表单操作模式。
#[derive(Debug, Clone, PartialEq)]
pub enum FormMode {
    /// 新建任务
    Create,
    /// 编辑已有任务
    Edit,
}

/// 任务表单状态。
#[derive(Debug, Clone)]
pub struct TaskFormState {
    /// 操作模式（新建 / 编辑）
    pub mode: FormMode,
    /// 编辑模式下的任务 ID
    pub editing_task_id: Option<String>,
    /// 描述字段内容
    pub description: String,
    /// 上下文字段内容
    pub context: String,
    /// 优先级字段内容
    pub priority: String,
    /// 标签字段内容（逗号分隔原始输入）
    pub tags: String,
    /// 预计完成时间字段内容
    pub eta: String,
    /// 当前聚焦字段
    pub focused_field: FormField,
    /// 表单校验错误
    pub error: Option<String>,
}

impl TaskFormState {
    /// 创建用于新建任务的空表单。
    pub fn new_create() -> Self {
        Self {
            mode: FormMode::Create,
            editing_task_id: None,
            description: String::new(),
            context: String::new(),
            priority: String::new(),
            tags: String::new(),
            eta: String::new(),
            focused_field: FormField::Description,
            error: None,
        }
    }

    /// 创建用于编辑已有任务的表单，预填充现有数据。
    pub fn new_edit(task: &Task) -> Self {
        let meta = task.metadata.as_ref();
        Self {
            mode: FormMode::Edit,
            editing_task_id: Some(task.id.clone()),
            description: task.task_description.clone(),
            context: task.context_for_all_tasks.clone().unwrap_or_default(),
            priority: meta.and_then(|m| m.priority.clone()).unwrap_or_default(),
            tags: meta
                .and_then(|m| m.tags.clone())
                .map(|t| t.join(", "))
                .unwrap_or_default(),
            eta: meta
                .and_then(|m| m.estimated_completion_time.clone())
                .unwrap_or_default(),
            focused_field: FormField::Description,
            error: None,
        }
    }

    /// 获取当前聚焦字段的可变引用。
    pub fn focused_value_mut(&mut self) -> &mut String {
        match self.focused_field {
            FormField::Description => &mut self.description,
            FormField::Context => &mut self.context,
            FormField::Priority => &mut self.priority,
            FormField::Tags => &mut self.tags,
            FormField::Eta => &mut self.eta,
        }
    }

    /// 获取当前聚焦字段的不可变引用。
    pub fn focused_value(&self) -> &str {
        match self.focused_field {
            FormField::Description => &self.description,
            FormField::Context => &self.context,
            FormField::Priority => &self.priority,
            FormField::Tags => &self.tags,
            FormField::Eta => &self.eta,
        }
    }

    /// 校验表单数据，返回错误信息。
    pub fn validate(&self) -> Result<(), String> {
        if self.description.trim().is_empty() {
            return Err("任务描述不能为空".to_owned());
        }
        if !self.priority.is_empty() {
            let p = self.priority.trim().to_lowercase();
            if !matches!(p.as_str(), "high" | "medium" | "low") {
                return Err("优先级必须是 high / medium / low".to_owned());
            }
        }
        Ok(())
    }
}

// ── 视图枚举 ───────────────────────────────────────────────────────────────

/// 焦点面板。
#[derive(Debug, Clone, PartialEq)]
pub enum FocusedPane {
    /// 左栏 - 项目列表
    Left,
    /// 右栏 - 任务列表/详情/表单/帮助
    Right,
}

/// 右栏子视图。
#[derive(Debug, Clone, PartialEq)]
pub enum RightPaneView {
    /// 任务列表
    TaskList,
    /// 任务详情
    TaskDetail,
    /// 任务表单（创建/编辑）
    TaskForm,
    /// 帮助
    Help,
}

/// 覆盖层（渲染在分栏之上）。
#[derive(Debug, Clone, PartialEq)]
pub enum Overlay {
    /// 无覆盖层
    None,
    /// 项目表单（创建/编辑）
    ProjectForm(ProjectFormMode),
    /// 删除项目确认
    DeleteProject,
    /// 删除任务确认
    DeleteTask,
    /// 删除笔记确认
    DeleteNote,
    /// 行内输入
    Input(InputMode),
}

// ── 项目表单字段 ───────────────────────────────────────────────────────────

/// 项目表单中可聚焦的字段。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProjectFormField {
    /// 项目名称（必填）
    Name,
    /// 项目描述（可选）
    Description,
}

impl ProjectFormField {
    /// 所有字段按 Tab 顺序排列。
    const ORDER: [ProjectFormField; 2] = [ProjectFormField::Name, ProjectFormField::Description];

    /// 返回下一个字段（循环）。
    pub fn next(self) -> Self {
        let idx = self.index();
        Self::ORDER[(idx + 1) % Self::ORDER.len()]
    }

    /// 返回上一个字段（循环）。
    pub fn prev(self) -> Self {
        let idx = self.index();
        if idx == 0 {
            Self::ORDER[Self::ORDER.len() - 1]
        } else {
            Self::ORDER[idx - 1]
        }
    }

    /// 字段在 ORDER 中的索引。
    fn index(self) -> usize {
        Self::ORDER.iter().position(|&f| f == self).unwrap()
    }

    /// 返回字段的中文标签。
    pub fn label(self) -> &'static str {
        match self {
            ProjectFormField::Name => "名称",
            ProjectFormField::Description => "描述",
        }
    }

    /// 返回字段的占位提示。
    pub fn placeholder(self) -> &'static str {
        match self {
            ProjectFormField::Name => "输入项目名称...",
            ProjectFormField::Description => "输入项目描述（可选）...",
        }
    }
}

// ── 项目表单模式 ───────────────────────────────────────────────────────────

/// 项目表单操作模式。
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectFormMode {
    /// 新建项目
    Create,
    /// 编辑已有项目
    Edit,
}

/// 项目表单状态。
#[derive(Debug, Clone)]
pub struct ProjectFormState {
    /// 操作模式（新建 / 编辑）
    pub mode: ProjectFormMode,
    /// 编辑模式下的项目 ID
    pub editing_project_id: Option<String>,
    /// 名称字段内容
    pub name: String,
    /// 描述字段内容
    pub description: String,
    /// 当前聚焦字段
    pub focused_field: ProjectFormField,
    /// 表单校验错误
    pub error: Option<String>,
}

impl ProjectFormState {
    /// 创建用于新建项目的空表单。
    pub fn new_create() -> Self {
        Self {
            mode: ProjectFormMode::Create,
            editing_project_id: None,
            name: String::new(),
            description: String::new(),
            focused_field: ProjectFormField::Name,
            error: None,
        }
    }

    /// 创建用于编辑已有项目的表单，预填充现有数据。
    pub fn new_edit(project: &Project) -> Self {
        Self {
            mode: ProjectFormMode::Edit,
            editing_project_id: Some(project.id.clone()),
            name: project.name.clone(),
            description: project.description.clone().unwrap_or_default(),
            focused_field: ProjectFormField::Name,
            error: None,
        }
    }

    /// 获取当前聚焦字段的可变引用。
    pub fn focused_value_mut(&mut self) -> &mut String {
        match self.focused_field {
            ProjectFormField::Name => &mut self.name,
            ProjectFormField::Description => &mut self.description,
        }
    }

    /// 获取当前聚焦字段的不可变引用。
    pub fn focused_value(&self) -> &str {
        match self.focused_field {
            ProjectFormField::Name => &self.name,
            ProjectFormField::Description => &self.description,
        }
    }

    /// 校验表单数据，返回错误信息。
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("项目名称不能为空".to_owned());
        }
        Ok(())
    }
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
    /// 焦点面板
    focused_pane: FocusedPane,
    /// 右栏子视图
    right_pane_view: RightPaneView,
    /// 进入帮助前的右栏视图（用于 Esc 返回）
    previous_right_pane_view: RightPaneView,
    /// 覆盖层状态
    overlay: Overlay,
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

    // ── 清单交互状态 ────────────────────────────────────────────────
    /// 清单条目当前焦点索引
    selected_item_index: usize,
    /// 行内输入缓冲区
    input_buffer: String,
    /// 待删除的笔记索引
    selected_note_index: usize,

    // ── 任务表单状态 ──────────────────────────────────────────────────
    /// 当前表单状态（进入 TaskForm 视图时创建）
    form_state: Option<TaskFormState>,

    // ── 项目列表状态 ──────────────────────────────────────────────────
    /// 项目列表缓存
    projects: Vec<Project>,
    /// 项目列表选中索引
    project_selected_index: usize,
    /// 项目列表滚动偏移
    project_scroll_offset: usize,
    /// 当前项目关联的任务列表
    project_tasks: Vec<Task>,
    /// 项目表单状态
    project_form_state: Option<ProjectFormState>,

    // ── 资源输入缓冲 ──────────────────────────────────────────────────
    /// 添加资源时暂存名称（AddingResourceName 完成后保存，AddingResourceUrl 完成后使用）
    resource_name_buffer: String,

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
            focused_pane: FocusedPane::Left,
            right_pane_view: RightPaneView::TaskList,
            previous_right_pane_view: RightPaneView::TaskList,
            overlay: Overlay::None,
            store: None,
            tasks: Vec::new(),
            selected_index: 0,
            current_task: None,
            scroll_offset: 0,
            sort_mode: SortMode::Created,
            search_query: None,
            search_mode: false,
            selected_note_index: 0,
            selected_item_index: 0,
            input_buffer: String::new(),
            form_state: None,
            projects: Vec::new(),
            project_selected_index: 0,
            project_scroll_offset: 0,
            project_tasks: Vec::new(),
            project_form_state: None,
            resource_name_buffer: String::new(),
            should_quit: false,
            message: None,
            error_message: None,
            action_tx: None,
        }
    }

    // ── 访问器 ────────────────────────────────────────────────────────────

    /// 获取焦点面板。
    pub fn focused_pane(&self) -> &FocusedPane {
        &self.focused_pane
    }

    /// 获取右栏子视图。
    pub fn right_pane_view(&self) -> &RightPaneView {
        &self.right_pane_view
    }

    /// 获取覆盖层状态。
    pub fn overlay(&self) -> &Overlay {
        &self.overlay
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

    /// 获取滚动偏移量。
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// 获取清单条目当前焦点索引。
    pub fn selected_item_index(&self) -> usize {
        self.selected_item_index
    }

    /// 获取输入缓冲区内容。
    pub fn input_buffer(&self) -> &str {
        &self.input_buffer
    }

    /// 获取表单状态引用。
    pub fn form_state(&self) -> Option<&TaskFormState> {
        self.form_state.as_ref()
    }

    /// 获取项目列表引用。
    pub fn projects(&self) -> &[Project] {
        &self.projects
    }

    /// 获取项目选中索引。
    pub fn project_selected_index(&self) -> usize {
        self.project_selected_index
    }

    /// 获取项目滚动偏移。
    pub fn project_scroll_offset(&self) -> usize {
        self.project_scroll_offset
    }

    /// 获取项目关联任务列表。
    pub fn project_tasks(&self) -> &[Task] {
        &self.project_tasks
    }

    /// 获取项目表单状态引用。
    pub fn project_form_state(&self) -> Option<&ProjectFormState> {
        self.project_form_state.as_ref()
    }

    /// 是否有活跃的覆盖层。
    pub fn is_overlay_active(&self) -> bool {
        self.overlay != Overlay::None
    }

    /// 获取待删除的笔记索引。
    pub fn selected_note_index(&self) -> usize {
        self.selected_note_index
    }

    // ── 事件发送端 ────────────────────────────────────────────────────────

    /// 设置事件总线发送端（由 mod.rs 在创建 EventBus 后调用）。
    pub fn set_action_tx(&mut self, tx: UnboundedSender<AppEvent>) {
        self.action_tx = Some(tx);
    }

    // ── 状态切换 ──────────────────────────────────────────────────────────

    /// 设置焦点面板。
    pub fn set_focused_pane(&mut self, pane: FocusedPane) {
        self.focused_pane = pane;
    }

    /// 设置右栏子视图。
    pub fn set_right_pane_view(&mut self, view: RightPaneView) {
        self.right_pane_view = view;
    }

    /// 切换到帮助视图（记录来源视图以便返回）。
    fn show_help(&mut self) {
        if self.right_pane_view != RightPaneView::Help {
            self.previous_right_pane_view = self.right_pane_view.clone();
            self.right_pane_view = RightPaneView::Help;
            self.focused_pane = FocusedPane::Right;
        } else {
            // 已经在帮助视图，返回上一个视图
            self.right_pane_view = self.previous_right_pane_view.clone();
        }
    }

    /// 关闭帮助视图（返回上一个右栏视图）。
    fn close_help(&mut self) {
        if self.right_pane_view == RightPaneView::Help {
            self.right_pane_view = self.previous_right_pane_view.clone();
        }
    }

    /// 设置覆盖层。
    #[allow(dead_code)]
    fn set_overlay(&mut self, overlay: Overlay) {
        self.overlay = overlay;
    }

    /// 关闭覆盖层。
    #[allow(dead_code)]
    fn close_overlay(&mut self) {
        self.overlay = Overlay::None;
    }

    /// 获取当前输入模式（从覆盖层中提取，若无则返回 Normal）。
    pub fn input_mode(&self) -> &InputMode {
        match &self.overlay {
            Overlay::Input(mode) => mode,
            _ => &InputMode::Normal,
        }
    }

    /// 是否显示删除任务确认对话框。
    pub fn confirm_delete(&self) -> bool {
        matches!(self.overlay, Overlay::DeleteTask)
    }

    /// 是否显示删除笔记确认对话框。
    pub fn confirm_delete_note(&self) -> bool {
        matches!(self.overlay, Overlay::DeleteNote)
    }

    /// 是否显示项目删除确认对话框。
    pub fn confirm_delete_project(&self) -> bool {
        matches!(self.overlay, Overlay::DeleteProject)
    }

    /// 获取当前选中的项目（从项目列表中获取）。
    pub fn current_project(&self) -> Option<&Project> {
        self.projects.get(self.project_selected_index)
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
                            let _ = tx.send(AppEvent::action(Action::TasksLoaded(tasks)));
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::action(Action::Error(format!(
                                "加载任务列表失败: {e}"
                            ))));
                        }
                    },
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "数据库连接失败: {e}"
                        ))));
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
                            let _ = tx.send(AppEvent::action(Action::TaskDetailLoaded(task)));
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::action(Action::Error(format!(
                                "加载任务详情失败: {e}"
                            ))));
                        }
                    },
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "数据库连接失败: {e}"
                        ))));
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
                            let _ = tx.send(AppEvent::action(Action::TaskDeleted(task_id)));
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::action(Action::Error(format!(
                                "删除任务失败: {e}"
                            ))));
                        }
                    },
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "数据库连接失败: {e}"
                        ))));
                    }
                }
            });
        }
    }

    // ── 排序与过滤 ──────────────────────────────────────────────────────

    /// 获取过滤并排序后的任务列表（右栏当前项目的任务）。
    ///
    /// 先按 search_query 过滤，再按 sort_mode 排序。
    /// 在分栏布局下，始终使用 `project_tasks`（当前选中项目的任务）。
    pub fn filtered_and_sorted_tasks(&self) -> Vec<&Task> {
        let mut result: Vec<&Task> = self.project_tasks.iter().collect();

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
                    let pa = priority_rank(a.metadata.as_ref().and_then(|m| m.priority.as_deref()));
                    let pb = priority_rank(b.metadata.as_ref().and_then(|m| m.priority.as_deref()));
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
            AppEvent::Action(action) => self.handle_action(*action),
        }
    }

    /// 键盘事件分发。
    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers};

        // 按键后清除临时消息，恢复键位提示
        self.message = None;

        // Ctrl+C 全局退出
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.quit();
            return Ok(());
        }

        // 搜索模式下只处理搜索相关按键
        if self.search_mode {
            return self.handle_search_key(key);
        }

        // 覆盖层优先处理
        match &self.overlay {
            Overlay::None => {}
            Overlay::ProjectForm(_) => {
                self.handle_project_form_key(key);
                return Ok(());
            }
            Overlay::DeleteProject => {
                self.handle_delete_project_confirm_key(key);
                return Ok(());
            }
            Overlay::DeleteTask => {
                self.handle_delete_confirm_key(key);
                return Ok(());
            }
            Overlay::DeleteNote => {
                self.handle_delete_note_confirm_key(key);
                return Ok(());
            }
            Overlay::Input(_) => {
                self.handle_input_key(key);
                return Ok(());
            }
        }

        match key.code {
            KeyCode::Char('q') => {
                // 任务表单中 q 键作为普通输入
                if self.right_pane_view == RightPaneView::TaskForm {
                    self.handle_task_form_key(key);
                    return Ok(());
                }
                if self.right_pane_view == RightPaneView::Help {
                    self.close_help();
                } else {
                    self.quit();
                }
            }
            KeyCode::Char('?') => {
                // 任务表单中 ? 键作为普通输入
                if self.right_pane_view == RightPaneView::TaskForm {
                    self.handle_task_form_key(key);
                    return Ok(());
                }
                self.show_help();
            }
            KeyCode::Esc => {
                // 任务表单中 Esc 由表单处理器处理
                if self.right_pane_view == RightPaneView::TaskForm {
                    self.handle_task_form_key(key);
                    return Ok(());
                }
                match self.right_pane_view {
                    RightPaneView::Help => self.close_help(),
                    RightPaneView::TaskDetail => self.right_pane_view = RightPaneView::TaskList,
                    _ => {}
                }
            }
            _ => {}
        }

        // 按焦点面板分派
        match self.focused_pane {
            FocusedPane::Left => self.handle_left_pane_key(key)?,
            FocusedPane::Right => self.handle_right_pane_key(key)?,
        }

        Ok(())
    }

    /// 左栏（项目列表）键盘处理。
    fn handle_left_pane_key(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<()> {
        use crossterm::event::KeyCode;

        let count = self.projects.len();

        match key.code {
            // 上下移动
            KeyCode::Up | KeyCode::Char('k') => {
                if self.project_selected_index > 0 {
                    self.project_selected_index -= 1;
                    self.adjust_project_scroll();
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if count > 0 && self.project_selected_index < count - 1 {
                    self.project_selected_index += 1;
                    self.adjust_project_scroll();
                }
            }
            // 切换到右栏，加载选中项目的任务
            KeyCode::Right | KeyCode::Enter => {
                self.focused_pane = FocusedPane::Right;
                self.spawn_load_project_tasks_for_selected_project();
            }
            // 新建项目
            KeyCode::Char('n') => {
                self.project_form_state = Some(ProjectFormState::new_create());
                self.overlay = Overlay::ProjectForm(ProjectFormMode::Create);
            }
            // 编辑项目
            KeyCode::Char('e') => {
                if let Some(project) = self.projects.get(self.project_selected_index) {
                    self.project_form_state = Some(ProjectFormState::new_edit(project));
                    self.overlay = Overlay::ProjectForm(ProjectFormMode::Edit);
                }
            }
            // 删除项目
            KeyCode::Char('d') => {
                if !self.projects.is_empty() {
                    self.overlay = Overlay::DeleteProject;
                }
            }
            // 刷新
            KeyCode::Char('r') => {
                self.spawn_load_projects();
            }
            // 帮助
            KeyCode::Char('?') => {
                self.show_help();
            }
            // Home/End 快速跳转
            KeyCode::Home => {
                self.project_selected_index = 0;
                self.project_scroll_offset = 0;
            }
            KeyCode::End => {
                if count > 0 {
                    self.project_selected_index = count - 1;
                    self.adjust_project_scroll();
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// 右栏键盘处理（按 right_pane_view 分派）。
    fn handle_right_pane_key(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<()> {
        use crossterm::event::KeyCode;

        match self.right_pane_view {
            RightPaneView::TaskList => {
                // ←/Esc 切回左栏
                if key.code == KeyCode::Left || key.code == KeyCode::Esc {
                    self.focused_pane = FocusedPane::Left;
                    return Ok(());
                }
                self.handle_right_task_list_key(key)?;
            }
            RightPaneView::TaskDetail => {
                // ←/Esc 返回右栏任务列表（不是切回左栏）
                if key.code == KeyCode::Left || key.code == KeyCode::Esc {
                    self.right_pane_view = RightPaneView::TaskList;
                    return Ok(());
                }
                self.handle_task_detail_key(key)?;
            }
            RightPaneView::TaskForm => {
                self.handle_task_form_key(key);
            }
            RightPaneView::Help => {
                // 已在 handle_key 中处理 Esc/? ，这里忽略其他按键
            }
        }

        Ok(())
    }

    /// 右栏任务列表的键盘处理。
    fn handle_right_task_list_key(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> anyhow::Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers};

        // Ctrl+R 刷新列表
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('r') {
            self.spawn_load_project_tasks_for_selected_project();
            return Ok(());
        }

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
            // 新建任务（自动关联当前选中项目）
            KeyCode::Char('n') => {
                self.form_state = Some(TaskFormState::new_create());
                self.right_pane_view = RightPaneView::TaskForm;
            }
            // 删除任务
            KeyCode::Char('d') => {
                if !self.filtered_and_sorted_tasks().is_empty() {
                    self.overlay = Overlay::DeleteTask;
                }
            }
            // 刷新列表
            KeyCode::Char('r') => {
                self.spawn_load_project_tasks_for_selected_project();
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
            // 帮助
            KeyCode::Char('?') => {
                self.show_help();
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

    /// 删除确认对话框的键盘处理（Overlay::DeleteTask）。
    fn handle_delete_confirm_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                // 确认删除：优先使用 current_task（详情视图），其次用列表选中项
                let task_id = if let Some(task) = &self.current_task {
                    task.id.clone()
                } else if let Some(task) = self.filtered_and_sorted_tasks().get(self.selected_index)
                {
                    task.id.clone()
                } else {
                    self.overlay = Overlay::None;
                    return;
                };
                self.overlay = Overlay::None;
                self.spawn_delete_task(task_id);
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.overlay = Overlay::None;
            }
            _ => {}
        }
    }

    /// 笔记删除确认对话框的键盘处理（Overlay::DeleteNote）。
    fn handle_delete_note_confirm_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(task) = &self.current_task {
                    let task_id = task.id.clone();
                    let idx = self.selected_note_index;
                    self.overlay = Overlay::None;
                    self.spawn_delete_note(task_id, idx);
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.overlay = Overlay::None;
            }
            _ => {}
        }
    }

    /// 异步切换清单条目完成状态。
    pub fn spawn_toggle_item(&mut self, task_id: String, item_index: usize, currently_done: bool) {
        let data_dir = self.store_cloned();
        if let Some(tx) = self.action_tx.clone() {
            tokio::spawn(async move {
                let store = match TaskStore::new(data_dir).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "数据库连接失败: {e}"
                        ))));
                        return;
                    }
                };
                let result = if currently_done {
                    crate::core::mark_task_undone(&store, &task_id, item_index).await
                } else {
                    crate::core::mark_task_done(&store, &task_id, item_index).await
                };
                match result {
                    Ok(task) => {
                        let _ = tx.send(AppEvent::action(Action::ItemToggled(task)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "切换条目状态失败: {e}"
                        ))));
                    }
                }
            });
        }
    }

    /// 异步添加清单条目。
    pub fn spawn_add_item(&mut self, task_id: String, item_name: String) {
        let data_dir = self.store_cloned();
        if let Some(tx) = self.action_tx.clone() {
            tokio::spawn(async move {
                let store = match TaskStore::new(data_dir).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "数据库连接失败: {e}"
                        ))));
                        return;
                    }
                };
                match crate::core::add_checklist_item(&store, &task_id, &item_name, "", None).await
                {
                    Ok(task) => {
                        let _ = tx.send(AppEvent::action(Action::ItemAdded(task)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "添加条目失败: {e}"
                        ))));
                    }
                }
            });
        }
    }

    /// 异步删除清单条目。
    pub fn spawn_remove_item(&mut self, task_id: String, item_index: usize) {
        let data_dir = self.store_cloned();
        if let Some(tx) = self.action_tx.clone() {
            tokio::spawn(async move {
                let store = match TaskStore::new(data_dir).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "数据库连接失败: {e}"
                        ))));
                        return;
                    }
                };
                match crate::core::remove_checklist_item(&store, &task_id, item_index).await {
                    Ok(task) => {
                        let _ = tx.send(AppEvent::action(Action::ItemRemoved(task)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "删除条目失败: {e}"
                        ))));
                    }
                }
            });
        }
    }

    /// 异步编辑清单条目名称。
    pub fn spawn_edit_item_name(&mut self, task_id: String, item_index: usize, new_name: String) {
        let data_dir = self.store_cloned();
        if let Some(tx) = self.action_tx.clone() {
            tokio::spawn(async move {
                let store = match TaskStore::new(data_dir).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "数据库连接失败: {e}"
                        ))));
                        return;
                    }
                };
                match crate::core::update_checklist_item(
                    &store,
                    &task_id,
                    item_index,
                    Some(&new_name),
                    None,
                    None,
                    None,
                )
                .await
                {
                    Ok(task) => {
                        let _ = tx.send(AppEvent::action(Action::ItemEdited(task)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "编辑条目失败: {e}"
                        ))));
                    }
                }
            });
        }
    }

    /// 异步编辑清单条目描述。
    pub fn spawn_edit_item_desc(&mut self, task_id: String, item_index: usize, new_desc: String) {
        let data_dir = self.store_cloned();
        if let Some(tx) = self.action_tx.clone() {
            tokio::spawn(async move {
                let store = match TaskStore::new(data_dir).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "数据库连接失败: {e}"
                        ))));
                        return;
                    }
                };
                match crate::core::update_checklist_item(
                    &store,
                    &task_id,
                    item_index,
                    None,
                    Some(&new_desc),
                    None,
                    None,
                )
                .await
                {
                    Ok(task) => {
                        let _ = tx.send(AppEvent::action(Action::ItemEdited(task)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "编辑条目失败: {e}"
                        ))));
                    }
                }
            });
        }
    }

    /// 异步重排清单条目。
    pub fn spawn_reorder_item(&mut self, task_id: String, from_index: usize, to_index: usize) {
        let data_dir = self.store_cloned();
        if let Some(tx) = self.action_tx.clone() {
            tokio::spawn(async move {
                let store = match TaskStore::new(data_dir).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "数据库连接失败: {e}"
                        ))));
                        return;
                    }
                };
                match crate::core::reorder_checklist_item(&store, &task_id, from_index, to_index)
                    .await
                {
                    Ok(task) => {
                        let _ = tx.send(AppEvent::action(Action::ItemReordered(task)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "移动条目失败: {e}"
                        ))));
                    }
                }
            });
        }
    }

    /// 异步添加笔记。
    pub fn spawn_add_note(&mut self, task_id: String, content: String) {
        let data_dir = self.store_cloned();
        if let Some(tx) = self.action_tx.clone() {
            tokio::spawn(async move {
                let store = match TaskStore::new(data_dir).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "数据库连接失败: {e}"
                        ))));
                        return;
                    }
                };
                match crate::core::add_note(&store, &task_id, &content).await {
                    Ok(task) => {
                        let _ = tx.send(AppEvent::action(Action::NoteAdded(task)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "添加笔记失败: {e}"
                        ))));
                    }
                }
            });
        }
    }

    /// 异步删除笔记。
    pub fn spawn_delete_note(&mut self, task_id: String, note_index: usize) {
        let data_dir = self.store_cloned();
        if let Some(tx) = self.action_tx.clone() {
            tokio::spawn(async move {
                let store = match TaskStore::new(data_dir).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "数据库连接失败: {e}"
                        ))));
                        return;
                    }
                };
                match crate::core::delete_note(&store, &task_id, note_index).await {
                    Ok(task) => {
                        let _ = tx.send(AppEvent::action(Action::NoteDeleted(task)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "删除笔记失败: {e}"
                        ))));
                    }
                }
            });
        }
    }

    /// 异步添加资源。
    pub fn spawn_add_resource(&mut self, task_id: String, name: String, url: String) {
        let data_dir = self.store_cloned();
        if let Some(tx) = self.action_tx.clone() {
            tokio::spawn(async move {
                let store = match TaskStore::new(data_dir).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "数据库连接失败: {e}"
                        ))));
                        return;
                    }
                };
                match crate::core::add_resource(&store, &task_id, &name, &url, None).await {
                    Ok(task) => {
                        let _ = tx.send(AppEvent::action(Action::ResourceAdded(task)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "添加资源失败: {e}"
                        ))));
                    }
                }
            });
        }
    }

    // ── 排序与过滤 ──────────────────────────────────────────────────────
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

    /// 任务详情视图的键盘处理（正常模式）。
    fn handle_task_detail_key(&mut self, key: crossterm::event::KeyEvent) -> anyhow::Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers};

        let item_count = self
            .current_task
            .as_ref()
            .map(|t| t.checklist.len())
            .unwrap_or(0);

        // Ctrl 组合键
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                // Ctrl+D 删除当前任务
                KeyCode::Char('d') => {
                    if self.current_task.is_some() {
                        self.overlay = Overlay::DeleteTask;
                    }
                    return Ok(());
                }
                // Ctrl+J / Ctrl+K 上下移动条目顺序
                KeyCode::Char('j') => {
                    if let Some(task) = &self.current_task {
                        let task_id = task.id.clone();
                        if self.selected_item_index < item_count.saturating_sub(1) {
                            let from = self.selected_item_index;
                            let to = from + 1;
                            self.spawn_reorder_item(task_id, from, to);
                            self.selected_item_index = to;
                        }
                    }
                    return Ok(());
                }
                KeyCode::Char('k') => {
                    if let Some(task) = &self.current_task {
                        let task_id = task.id.clone();
                        if self.selected_item_index > 0 {
                            let from = self.selected_item_index;
                            let to = from - 1;
                            self.spawn_reorder_item(task_id, from, to);
                            self.selected_item_index = to;
                        }
                    }
                    return Ok(());
                }
                _ => {}
            }
        }

        match key.code {
            // 上下移动焦点
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_item_index > 0 {
                    self.selected_item_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if item_count > 0 && self.selected_item_index < item_count - 1 {
                    self.selected_item_index += 1;
                }
            }
            // Space/x 切换完成状态
            KeyCode::Char(' ') | KeyCode::Char('x') => {
                if let Some(task) = &self.current_task {
                    if let Some(item) = task.checklist.get(self.selected_item_index) {
                        let task_id = task.id.clone();
                        let idx = self.selected_item_index;
                        let done = item.done;
                        self.spawn_toggle_item(task_id, idx, done);
                    }
                }
            }
            // a 键进入添加条目模式
            KeyCode::Char('a') => {
                self.overlay = Overlay::Input(InputMode::AddingItem);
                self.input_buffer.clear();
                self.message = Some("输入条目名称，Enter 确认，Esc 取消".to_owned());
            }
            // E 键（Shift+e）打开任务编辑表单
            KeyCode::Char('E') => {
                if let Some(task) = &self.current_task {
                    self.form_state = Some(TaskFormState::new_edit(task));
                    self.right_pane_view = RightPaneView::TaskForm;
                }
            }
            // e 键编辑当前条目名称
            KeyCode::Char('e') => {
                if let Some(task) = &self.current_task {
                    if let Some(item) = task.checklist.get(self.selected_item_index) {
                        self.overlay = Overlay::Input(InputMode::EditingItemName);
                        self.input_buffer = item.task.clone();
                        self.message = Some("编辑条目名称，Enter 确认，Esc 取消".to_owned());
                    }
                }
            }
            // d 键删除当前条目
            KeyCode::Char('d') => {
                if let Some(task) = &self.current_task {
                    if !task.checklist.is_empty() {
                        let task_id = task.id.clone();
                        let idx = self.selected_item_index;
                        let new_count = task.checklist.len().saturating_sub(1);
                        self.spawn_remove_item(task_id, idx);
                        // 调整索引
                        if new_count > 0 && self.selected_item_index >= new_count {
                            self.selected_item_index = new_count - 1;
                        } else if new_count == 0 {
                            self.selected_item_index = 0;
                        }
                    }
                }
            }
            // N 键（Shift+n）添加笔记
            KeyCode::Char('N') => {
                self.overlay = Overlay::Input(InputMode::AddingNote);
                self.input_buffer.clear();
                self.message = Some("输入笔记内容，Enter 确认，Esc 取消".to_owned());
            }
            // D 键（Shift+d）删除笔记（确认提示）
            KeyCode::Char('D') => {
                if let Some(task) = &self.current_task {
                    if !task.notes.is_empty() {
                        self.selected_note_index = 0;
                        self.overlay = Overlay::DeleteNote;
                    }
                }
            }
            // L 键添加资源链接（两步输入：名称 → URL）
            KeyCode::Char('L') => {
                self.overlay = Overlay::Input(InputMode::AddingResourceName);
                self.input_buffer.clear();
                self.resource_name_buffer.clear();
                self.message = Some("输入资源名称，Enter 下一步，Esc 取消".to_owned());
            }
            _ => {}
        }

        Ok(())
    }

    /// 输入模式下的键盘处理（覆盖层 Overlay::Input）。
    fn handle_input_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        // 获取当前输入模式的克隆（因为后续需要修改 overlay）
        let current_mode = match &self.overlay {
            Overlay::Input(mode) => mode.clone(),
            _ => return,
        };

        match key.code {
            KeyCode::Esc => {
                self.overlay = Overlay::None;
                self.input_buffer.clear();
                self.resource_name_buffer.clear();
                self.message = None;
            }
            KeyCode::Enter => {
                let value = self.input_buffer.trim().to_owned();
                if let Some(task) = &self.current_task {
                    let task_id = task.id.clone();
                    match &current_mode {
                        InputMode::AddingItem => {
                            if value.is_empty() {
                                self.overlay = Overlay::None;
                                self.input_buffer.clear();
                                self.message = Some("条目名称不能为空".to_owned());
                                return;
                            }
                            self.spawn_add_item(task_id, value);
                        }
                        InputMode::EditingItemName => {
                            if value.is_empty() {
                                self.overlay = Overlay::None;
                                self.input_buffer.clear();
                                self.message = Some("条目名称不能为空".to_owned());
                                return;
                            }
                            let idx = self.selected_item_index;
                            self.spawn_edit_item_name(task_id, idx, value);
                        }
                        InputMode::EditingItemDesc => {
                            let idx = self.selected_item_index;
                            self.spawn_edit_item_desc(task_id, idx, value);
                        }
                        InputMode::AddingNote => {
                            if value.is_empty() {
                                self.overlay = Overlay::None;
                                self.input_buffer.clear();
                                self.message = Some("笔记内容不能为空".to_owned());
                                return;
                            }
                            self.spawn_add_note(task_id, value);
                        }
                        InputMode::AddingResourceName => {
                            if value.is_empty() {
                                self.overlay = Overlay::None;
                                self.input_buffer.clear();
                                self.message = Some("资源名称不能为空".to_owned());
                                return;
                            }
                            // 保存名称，切换到 URL 输入步骤
                            self.resource_name_buffer = value;
                            self.input_buffer.clear();
                            self.overlay = Overlay::Input(InputMode::AddingResourceUrl);
                            self.message = Some("输入资源 URL，Enter 确认，Esc 取消".to_owned());
                            return; // 不清除覆盖层
                        }
                        InputMode::AddingResourceUrl => {
                            if value.is_empty() {
                                self.overlay = Overlay::None;
                                self.input_buffer.clear();
                                self.resource_name_buffer.clear();
                                self.message = Some("资源 URL 不能为空".to_owned());
                                return;
                            }
                            let name = std::mem::take(&mut self.resource_name_buffer);
                            self.spawn_add_resource(task_id, name, value);
                        }
                        InputMode::Normal => unreachable!(),
                    }
                }
                self.overlay = Overlay::None;
                self.input_buffer.clear();
                self.resource_name_buffer.clear();
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
            }
            _ => {}
        }
    }

    /// 项目删除确认对话框的键盘处理。
    fn handle_delete_project_confirm_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(project) = self.projects.get(self.project_selected_index) {
                    let project_id = project.id.clone();
                    self.overlay = Overlay::None;
                    self.spawn_delete_project(project_id);
                } else {
                    self.overlay = Overlay::None;
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.overlay = Overlay::None;
            }
            _ => {}
        }
    }

    /// 项目表单视图的键盘处理。
    fn handle_project_form_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        let Some(form) = self.project_form_state.as_mut() else {
            return;
        };

        match key.code {
            // Esc 取消表单
            KeyCode::Esc => {
                self.project_form_state = None;
                self.overlay = Overlay::None;
                self.message = Some("已取消".to_owned());
            }
            // Tab 切换到下一个字段
            KeyCode::Tab => {
                form.focused_field = form.focused_field.next();
                form.error = None;
            }
            // BackTab (Shift+Tab) 切换到上一个字段
            KeyCode::BackTab => {
                form.focused_field = form.focused_field.prev();
                form.error = None;
            }
            // Enter 提交表单
            KeyCode::Enter => {
                if let Err(e) = form.validate() {
                    form.error = Some(e);
                    return;
                }

                let name = form.name.trim().to_owned();
                let description = form.description.trim().to_owned();
                let mode = form.mode.clone();
                let editing_project_id = form.editing_project_id.clone();

                self.project_form_state = None;
                self.overlay = Overlay::None;

                match mode {
                    ProjectFormMode::Create => {
                        self.spawn_create_project(
                            name,
                            if description.is_empty() {
                                None
                            } else {
                                Some(description)
                            },
                        );
                    }
                    ProjectFormMode::Edit => {
                        if let Some(project_id) = editing_project_id {
                            self.spawn_update_project(
                                project_id,
                                name,
                                if description.is_empty() {
                                    None
                                } else {
                                    Some(description)
                                },
                            );
                        }
                    }
                }
            }
            // Backspace 删除末尾字符
            KeyCode::Backspace => {
                form.focused_value_mut().pop();
                form.error = None;
            }
            // 普通字符输入
            KeyCode::Char(c) => {
                form.focused_value_mut().push(c);
                form.error = None;
            }
            _ => {}
        }
    }

    /// 项目列表滚动调整。
    fn adjust_project_scroll(&mut self) {
        let visible_height = 20;
        if self.project_selected_index < self.project_scroll_offset {
            self.project_scroll_offset = self.project_selected_index;
        } else if self.project_selected_index >= self.project_scroll_offset + visible_height {
            self.project_scroll_offset = self.project_selected_index - visible_height + 1;
        }
    }

    // ── 任务表单键盘处理 ────────────────────────────────────────────────

    /// 任务表单视图的键盘处理。
    ///
    /// Tab/Backtab 切换字段，Enter 提交，Esc 取消，其他按键编辑当前字段。
    fn handle_task_form_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        // 表单状态必须存在
        let Some(form) = self.form_state.as_mut() else {
            return;
        };

        match key.code {
            // Esc 取消表单，返回右栏任务列表
            KeyCode::Esc => {
                self.form_state = None;
                self.right_pane_view = RightPaneView::TaskList;
                self.message = Some("已取消".to_owned());
            }
            // Tab 切换到下一个字段
            KeyCode::Tab => {
                form.focused_field = form.focused_field.next();
                form.error = None;
            }
            // BackTab (Shift+Tab) 切换到上一个字段
            KeyCode::BackTab => {
                form.focused_field = form.focused_field.prev();
                form.error = None;
            }
            // Enter 提交表单
            KeyCode::Enter => {
                // 校验
                if let Err(e) = form.validate() {
                    form.error = Some(e);
                    return;
                }

                // 提取表单数据
                let description = form.description.trim().to_owned();
                let context = form.context.trim().to_owned();
                let priority = form.priority.trim().to_lowercase();
                let tags_str = form.tags.trim().to_owned();
                let eta = form.eta.trim().to_owned();
                let mode = form.mode.clone();
                let editing_task_id = form.editing_task_id.clone();

                // 清除表单状态
                self.form_state = None;

                match mode {
                    FormMode::Create => {
                        // 获取当前选中项目的 ID，用于自动关联
                        let project_id = self
                            .projects
                            .get(self.project_selected_index)
                            .map(|p| p.id.clone());
                        self.spawn_create_task(
                            description,
                            if context.is_empty() {
                                None
                            } else {
                                Some(context)
                            },
                            if priority.is_empty() {
                                None
                            } else {
                                Some(priority)
                            },
                            if tags_str.is_empty() {
                                None
                            } else {
                                Some(tags_str)
                            },
                            if eta.is_empty() { None } else { Some(eta) },
                            project_id,
                        );
                    }
                    FormMode::Edit => {
                        if let Some(task_id) = editing_task_id {
                            self.spawn_update_task(
                                task_id,
                                description,
                                context,
                                priority,
                                tags_str,
                                eta,
                            );
                        }
                    }
                }
            }
            // 优先级字段：枚举选择模式（Space/→/← 循环切换）
            KeyCode::Char(' ') | KeyCode::Right | KeyCode::Left
                if form.focused_field == FormField::Priority =>
            {
                const OPTIONS: &[&str] = &["", "low", "medium", "high"];
                let current = form.priority.trim().to_lowercase();
                let idx = OPTIONS.iter().position(|&o| o == current).unwrap_or(0);
                let new_idx = if key.code == KeyCode::Left {
                    if idx == 0 {
                        OPTIONS.len() - 1
                    } else {
                        idx - 1
                    }
                } else {
                    (idx + 1) % OPTIONS.len()
                };
                form.priority = OPTIONS[new_idx].to_owned();
                form.error = None;
            }
            // Backspace 删除末尾字符（优先级字段除外）
            KeyCode::Backspace if form.focused_field != FormField::Priority => {
                form.focused_value_mut().pop();
                form.error = None;
            }
            // 普通字符输入（优先级字段除外）
            KeyCode::Char(c) if form.focused_field != FormField::Priority => {
                form.focused_value_mut().push(c);
                form.error = None;
            }
            _ => {}
        }
    }

    /// 异步创建任务（通过 spawn + Action 回传）。
    ///
    /// 如果指定了 `project_id`，创建后自动将任务关联到该项目。
    fn spawn_create_task(
        &mut self,
        description: String,
        context: Option<String>,
        priority: Option<String>,
        tags_str: Option<String>,
        eta: Option<String>,
        project_id: Option<String>,
    ) {
        let data_dir = self.store_cloned();
        if let Some(tx) = self.action_tx.clone() {
            self.message = Some("正在创建任务...".to_owned());
            tokio::spawn(async move {
                let store = match TaskStore::new(data_dir).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "数据库连接失败: {e}"
                        ))));
                        return;
                    }
                };

                // 构建元数据
                let metadata = if priority.is_some() || tags_str.is_some() || eta.is_some() {
                    Some(crate::models::task::TaskMetadata {
                        tags: tags_str.map(|s| {
                            s.split(',')
                                .map(|t| t.trim().to_owned())
                                .filter(|t| !t.is_empty())
                                .collect()
                        }),
                        priority,
                        estimated_completion_time: eta,
                    })
                } else {
                    None
                };

                match crate::core::initialize_task(
                    &store,
                    &description,
                    context.as_deref(),
                    vec![],
                    vec![],
                    vec![],
                    metadata,
                    None,
                )
                .await
                {
                    Ok(task) => {
                        // 如果指定了项目，自动关联
                        if let Some(pid) = &project_id {
                            match crate::core::project::set_task_project(
                                &store,
                                &task.id,
                                Some(pid),
                            )
                            .await
                            {
                                Ok(_) => {
                                    let _ = tx.send(AppEvent::action(Action::TaskCreated(task)));
                                }
                                Err(e) => {
                                    let _ = tx.send(AppEvent::action(Action::Error(format!(
                                        "任务已创建但关联项目失败: {e}"
                                    ))));
                                }
                            }
                        } else {
                            let _ = tx.send(AppEvent::action(Action::TaskCreated(task)));
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "创建任务失败: {e}"
                        ))));
                    }
                }
            });
        }
    }

    /// 异步更新任务（描述 + 上下文 + 元数据）（通过 spawn + Action 回传）。
    fn spawn_update_task(
        &mut self,
        task_id: String,
        description: String,
        context: String,
        priority: String,
        tags_str: String,
        eta: String,
    ) {
        let data_dir = self.store_cloned();
        if let Some(tx) = self.action_tx.clone() {
            self.message = Some("正在更新任务...".to_owned());
            tokio::spawn(async move {
                let store = match TaskStore::new(data_dir).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "数据库连接失败: {e}"
                        ))));
                        return;
                    }
                };

                // 1. 更新描述
                match crate::core::update_task_description(&store, &task_id, &description).await {
                    Ok(_) => {}
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "更新描述失败: {e}"
                        ))));
                        return;
                    }
                }

                // 2. 更新上下文
                if let Err(e) = crate::core::update_context(&store, &task_id, &context).await {
                    let _ = tx.send(AppEvent::action(Action::Error(format!(
                        "更新上下文失败: {e}"
                    ))));
                    return;
                }

                // 3. 更新元数据
                let metadata = crate::models::task::TaskMetadata {
                    tags: if tags_str.is_empty() {
                        None
                    } else {
                        Some(
                            tags_str
                                .split(',')
                                .map(|t| t.trim().to_owned())
                                .filter(|t| !t.is_empty())
                                .collect(),
                        )
                    },
                    priority: if priority.is_empty() {
                        None
                    } else {
                        Some(priority)
                    },
                    estimated_completion_time: if eta.is_empty() { None } else { Some(eta) },
                };
                if let Err(e) = crate::core::update_metadata(&store, &task_id, metadata).await {
                    let _ = tx.send(AppEvent::action(Action::Error(format!(
                        "更新元数据失败: {e}"
                    ))));
                    return;
                }

                // 4. 重新加载任务并回传
                match store.get_task(&task_id).await {
                    Ok(task) => {
                        let _ = tx.send(AppEvent::action(Action::TaskUpdated(task)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "加载更新后的任务失败: {e}"
                        ))));
                    }
                }
            });
        }
    }

    // ── 项目异步方法 ────────────────────────────────────────────────────

    /// 异步加载项目列表。
    pub fn spawn_load_projects(&mut self) {
        let data_dir = self.store_cloned();
        if let Some(tx) = self.action_tx.clone() {
            tokio::spawn(async move {
                let store = TaskStore::new(data_dir).await;
                match store {
                    Ok(s) => match crate::core::project::list_projects(&s).await {
                        Ok(projects) => {
                            let _ = tx.send(AppEvent::action(Action::ProjectsLoaded(projects)));
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::action(Action::Error(format!(
                                "加载项目列表失败: {e}"
                            ))));
                        }
                    },
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "数据库连接失败: {e}"
                        ))));
                    }
                }
            });
            self.message = Some("正在加载项目列表...".to_owned());
        }
    }

    /// 异步加载项目详情。
    pub fn spawn_load_project_detail(&mut self, project_id: String) {
        let data_dir = self.store_cloned();
        if let Some(tx) = self.action_tx.clone() {
            tokio::spawn(async move {
                let store = TaskStore::new(data_dir).await;
                match store {
                    Ok(s) => match crate::core::project::get_project(&s, &project_id).await {
                        Ok(project) => {
                            let _ = tx.send(AppEvent::action(Action::ProjectDetailLoaded(project)));
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::action(Action::Error(format!(
                                "加载项目详情失败: {e}"
                            ))));
                        }
                    },
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "数据库连接失败: {e}"
                        ))));
                    }
                }
            });
        }
    }

    /// 异步加载项目关联的任务列表。
    fn spawn_load_project_tasks(&mut self, project_id: String) {
        let data_dir = self.store_cloned();
        if let Some(tx) = self.action_tx.clone() {
            tokio::spawn(async move {
                let store = TaskStore::new(data_dir).await;
                match store {
                    Ok(s) => match crate::core::project::get_tasks_for_project(&s, &project_id)
                        .await
                    {
                        Ok(tasks) => {
                            let _ = tx.send(AppEvent::action(Action::ProjectTasksLoaded(tasks)));
                        }
                        Err(e) => {
                            let _ = tx.send(AppEvent::action(Action::Error(format!(
                                "加载项目任务失败: {e}"
                            ))));
                        }
                    },
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "数据库连接失败: {e}"
                        ))));
                    }
                }
            });
        }
    }

    /// 加载当前选中项目的任务列表（便捷方法）。
    ///
    /// 从 `projects[project_selected_index]` 获取项目 ID，
    /// 重置 `selected_index` 和 `scroll_offset`，然后调用 `spawn_load_project_tasks`。
    fn spawn_load_project_tasks_for_selected_project(&mut self) {
        if let Some(project) = self.projects.get(self.project_selected_index) {
            let project_id = project.id.clone();
            self.selected_index = 0;
            self.scroll_offset = 0;
            self.spawn_load_project_tasks(project_id);
        }
    }

    /// 异步创建项目。
    fn spawn_create_project(&mut self, name: String, description: Option<String>) {
        let data_dir = self.store_cloned();
        if let Some(tx) = self.action_tx.clone() {
            self.message = Some("正在创建项目...".to_owned());
            tokio::spawn(async move {
                let store = match TaskStore::new(data_dir).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "数据库连接失败: {e}"
                        ))));
                        return;
                    }
                };
                match crate::core::project::create_project(&store, &name, description.as_deref())
                    .await
                {
                    Ok(project) => {
                        let _ = tx.send(AppEvent::action(Action::ProjectCreated(project)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "创建项目失败: {e}"
                        ))));
                    }
                }
            });
        }
    }

    /// 异步更新项目。
    fn spawn_update_project(
        &mut self,
        project_id: String,
        name: String,
        description: Option<String>,
    ) {
        let data_dir = self.store_cloned();
        if let Some(tx) = self.action_tx.clone() {
            self.message = Some("正在更新项目...".to_owned());
            tokio::spawn(async move {
                let store = match TaskStore::new(data_dir).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "数据库连接失败: {e}"
                        ))));
                        return;
                    }
                };
                match crate::core::project::update_project(
                    &store,
                    &project_id,
                    Some(&name),
                    description.as_deref(),
                )
                .await
                {
                    Ok(project) => {
                        let _ = tx.send(AppEvent::action(Action::ProjectUpdated(project)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "更新项目失败: {e}"
                        ))));
                    }
                }
            });
        }
    }

    /// 异步删除项目。
    fn spawn_delete_project(&mut self, project_id: String) {
        let data_dir = self.store_cloned();
        if let Some(tx) = self.action_tx.clone() {
            tokio::spawn(async move {
                let store = match TaskStore::new(data_dir).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "数据库连接失败: {e}"
                        ))));
                        return;
                    }
                };
                match crate::core::project::delete_project(&store, &project_id).await {
                    Ok(()) => {
                        let _ = tx.send(AppEvent::action(Action::ProjectDeleted(project_id)));
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "删除项目失败: {e}"
                        ))));
                    }
                }
            });
        }
    }

    /// 异步添加任务到项目。
    #[allow(dead_code)]
    pub fn spawn_add_task_to_project(&mut self, task_id: String, project_id: String) {
        let data_dir = self.store_cloned();
        if let Some(tx) = self.action_tx.clone() {
            tokio::spawn(async move {
                let store = match TaskStore::new(data_dir).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "数据库连接失败: {e}"
                        ))));
                        return;
                    }
                };
                match crate::core::project::set_task_project(&store, &task_id, Some(&project_id))
                    .await
                {
                    Ok(_) => {
                        let _ = tx.send(AppEvent::action(Action::Error(
                            "任务已添加到项目".to_owned(),
                        )));
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "添加任务到项目失败: {e}"
                        ))));
                    }
                }
            });
        }
    }

    /// 异步从项目中移除任务。
    #[allow(dead_code)]
    pub fn spawn_remove_task_from_project(&mut self, task_id: String, _project_id: String) {
        let data_dir = self.store_cloned();
        if let Some(tx) = self.action_tx.clone() {
            tokio::spawn(async move {
                let store = match TaskStore::new(data_dir).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "数据库连接失败: {e}"
                        ))));
                        return;
                    }
                };
                match crate::core::project::set_task_project(&store, &task_id, None).await {
                    Ok(_) => {
                        let _ = tx.send(AppEvent::action(Action::Error(
                            "任务已从项目中移除".to_owned(),
                        )));
                    }
                    Err(e) => {
                        let _ = tx.send(AppEvent::action(Action::Error(format!(
                            "移除任务失败: {e}"
                        ))));
                    }
                }
            });
        }
    }

    /// 异步操作结果处理。
    fn handle_action(&mut self, action: Action) -> anyhow::Result<()> {
        match action {
            Action::TasksLoaded(tasks) => {
                // 保留用于 filtered_and_sorted_tasks 兼容（内部缓存）
                self.tasks = tasks;
                self.clamp_selected_index();
                self.error_message = None;
            }
            Action::TaskDetailLoaded(task) => {
                self.selected_item_index = 0;
                self.current_task = Some(task);
                self.right_pane_view = RightPaneView::TaskDetail;
                self.focused_pane = FocusedPane::Right;
                self.error_message = None;
            }
            Action::TaskDeleted(task_id) => {
                // 如果删除的是当前查看的任务，返回右栏任务列表
                if self.current_task.as_ref().is_some_and(|t| t.id == task_id) {
                    self.current_task = None;
                    self.right_pane_view = RightPaneView::TaskList;
                }
                self.project_tasks.retain(|t| t.id != task_id);
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
            Action::TaskCreated(task) => {
                self.message = Some("任务已创建".to_owned());
                self.error_message = None;
                // 刷新当前项目的任务列表
                self.spawn_load_project_tasks_for_selected_project();
                // 切换到右栏详情视图展示新建的任务
                self.current_task = Some(task);
                self.right_pane_view = RightPaneView::TaskDetail;
                self.focused_pane = FocusedPane::Right;
            }
            Action::TaskUpdated(task) => {
                // 更新缓存中的对应任务
                if let Some(t) = self.tasks.iter_mut().find(|t| t.id == task.id) {
                    *t = task.clone();
                }
                if let Some(t) = self.project_tasks.iter_mut().find(|t| t.id == task.id) {
                    *t = task.clone();
                }
                self.current_task = Some(task);
                self.right_pane_view = RightPaneView::TaskDetail;
                self.message = Some("任务已更新".to_owned());
                self.error_message = None;
            }
            Action::NoteAdded(task) => {
                self.current_task = Some(task);
                self.message = Some("笔记已添加".to_owned());
            }
            Action::NoteDeleted(task) => {
                self.current_task = Some(task);
                self.message = Some("笔记已删除".to_owned());
            }
            Action::ResourceAdded(task) => {
                self.current_task = Some(task);
                self.message = Some("资源已添加".to_owned());
            }
            Action::ResourceDeleted(task) => {
                self.current_task = Some(task);
                self.message = Some("资源已删除".to_owned());
            }
            Action::ProjectsLoaded(projects) => {
                let had_projects = !self.projects.is_empty();
                self.projects = projects;
                if self.project_selected_index >= self.projects.len() && !self.projects.is_empty() {
                    self.project_selected_index = self.projects.len() - 1;
                } else if self.projects.is_empty() {
                    self.project_selected_index = 0;
                }
                self.message = Some(format!("已加载 {} 个项目", self.projects.len()));
                self.error_message = None;
                // 首次加载项目时，自动选中第一个项目并加载其任务
                if !had_projects && !self.projects.is_empty() {
                    self.spawn_load_project_tasks_for_selected_project();
                }
            }
            Action::ProjectDetailLoaded(_project) => {
                // 不再使用独立的项目详情视图，忽略此 Action
                // 项目信息直接从 projects[project_selected_index] 获取
            }
            Action::ProjectCreated(project) => {
                self.message = Some("项目已创建".to_owned());
                self.error_message = None;
                // 关闭覆盖层
                self.overlay = Overlay::None;
                // 刷新项目列表
                self.spawn_load_projects();
                // 选中新建的项目
                if let Some(pos) = self.projects.iter().position(|p| p.id == project.id) {
                    self.project_selected_index = pos;
                }
            }
            Action::ProjectUpdated(project) => {
                // 更新列表缓存
                if let Some(p) = self.projects.iter_mut().find(|p| p.id == project.id) {
                    *p = project.clone();
                }
                // 关闭覆盖层
                self.overlay = Overlay::None;
                self.message = Some("项目已更新".to_owned());
                self.error_message = None;
            }
            Action::ProjectDeleted(project_id) => {
                // 关闭覆盖层
                self.overlay = Overlay::None;
                // 清理
                self.project_tasks
                    .retain(|t| t.project_id.as_ref() != Some(&project_id));
                self.projects.retain(|p| p.id != project_id);
                if self.project_selected_index >= self.projects.len() && !self.projects.is_empty() {
                    self.project_selected_index = self.projects.len() - 1;
                } else if self.projects.is_empty() {
                    self.project_selected_index = 0;
                    self.project_tasks.clear();
                }
                // 如果有剩余项目，加载其任务
                if !self.projects.is_empty() {
                    self.spawn_load_project_tasks_for_selected_project();
                }
                self.message = Some("项目已删除".to_owned());
                self.error_message = None;
            }
            Action::ProjectTasksLoaded(tasks) => {
                self.project_tasks = tasks;
                self.selected_index = 0;
                self.scroll_offset = 0;
                self.message = Some(format!("已加载 {} 个任务", self.project_tasks.len()));
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

// ── 单元测试 ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── FormField 测试 ────────────────────────────────────────────────────

    #[test]
    fn form_field_order_completeness() {
        // 确保 ORDER 覆盖所有变体（编译时也保证，但额外验证长度）
        assert_eq!(FormField::ORDER.len(), 5);
    }

    #[test]
    fn form_field_next_cycles() {
        // 从每个字段调用 next()，最后一个应循环回第一个
        let fields = FormField::ORDER;
        for (i, field) in fields.iter().enumerate() {
            let next = field.next();
            let expected = fields[(i + 1) % fields.len()];
            assert_eq!(next, expected, "FormField::next({i}) 循环错误");
        }
    }

    #[test]
    fn form_field_prev_cycles() {
        let fields = FormField::ORDER;
        for (i, field) in fields.iter().enumerate() {
            let prev = field.prev();
            let expected = if i == 0 {
                fields[fields.len() - 1]
            } else {
                fields[i - 1]
            };
            assert_eq!(prev, expected, "FormField::prev({i}) 循环错误");
        }
    }

    #[test]
    fn form_field_next_prev_inverse() {
        // 连续 next+prev 应回到原字段
        for field in FormField::ORDER {
            assert_eq!(field.next().prev(), field);
            assert_eq!(field.prev().next(), field);
        }
    }

    #[test]
    fn form_field_labels_non_empty() {
        for field in FormField::ORDER {
            assert!(!field.label().is_empty(), "label 不应为空");
        }
    }

    #[test]
    fn form_field_placeholders_non_empty() {
        for field in FormField::ORDER {
            assert!(!field.placeholder().is_empty(), "placeholder 不应为空");
        }
    }

    #[test]
    fn form_field_next_full_cycle() {
        // 连续调用 5 次 next() 回到起点
        let start = FormField::Description;
        let mut current = start;
        for _ in 0..5 {
            current = current.next();
        }
        assert_eq!(current, start);
    }

    #[test]
    fn form_field_prev_full_cycle() {
        let start = FormField::Description;
        let mut current = start;
        for _ in 0..5 {
            current = current.prev();
        }
        assert_eq!(current, start);
    }

    // ── SortMode 测试 ─────────────────────────────────────────────────────

    #[test]
    fn sort_mode_next_cycles() {
        assert_eq!(SortMode::Created.next(), SortMode::Priority);
        assert_eq!(SortMode::Priority.next(), SortMode::Updated);
        assert_eq!(SortMode::Updated.next(), SortMode::Created);
    }

    #[test]
    fn sort_mode_next_full_cycle() {
        let start = SortMode::Created;
        let mut current = start;
        for _ in 0..3 {
            current = current.next();
        }
        assert_eq!(current, start);
    }

    #[test]
    fn sort_mode_labels() {
        assert_eq!(SortMode::Created.label(), "创建时间");
        assert_eq!(SortMode::Priority.label(), "优先级");
        assert_eq!(SortMode::Updated.label(), "更新时间");
    }

    #[test]
    fn sort_mode_labels_non_empty() {
        let modes = [SortMode::Created, SortMode::Priority, SortMode::Updated];
        for mode in modes {
            assert!(!mode.label().is_empty());
        }
    }

    // ── priority_rank 测试 ────────────────────────────────────────────────

    #[test]
    fn priority_rank_values() {
        assert_eq!(priority_rank(Some("high")), 3);
        assert_eq!(priority_rank(Some("medium")), 2);
        assert_eq!(priority_rank(Some("low")), 1);
        assert_eq!(priority_rank(None), 0);
    }

    #[test]
    fn priority_rank_unknown_string_is_zero() {
        assert_eq!(priority_rank(Some("unknown")), 0);
        assert_eq!(priority_rank(Some("")), 0);
        assert_eq!(priority_rank(Some("HIGH")), 0); // 大小写敏感
    }

    #[test]
    fn priority_rank_ordering() {
        // high > medium > low > none
        assert!(priority_rank(Some("high")) > priority_rank(Some("medium")));
        assert!(priority_rank(Some("medium")) > priority_rank(Some("low")));
        assert!(priority_rank(Some("low")) > priority_rank(None));
    }

    // ── TaskFormState 测试 ────────────────────────────────────────────────

    #[test]
    fn form_state_new_create_defaults() {
        let form = TaskFormState::new_create();
        assert_eq!(form.mode, FormMode::Create);
        assert!(form.editing_task_id.is_none());
        assert!(form.description.is_empty());
        assert!(form.context.is_empty());
        assert!(form.priority.is_empty());
        assert!(form.tags.is_empty());
        assert!(form.eta.is_empty());
        assert_eq!(form.focused_field, FormField::Description);
        assert!(form.error.is_none());
    }

    #[test]
    fn form_state_validate_empty_description_fails() {
        let form = TaskFormState::new_create();
        let result = form.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "任务描述不能为空");
    }

    #[test]
    fn form_state_validate_whitespace_description_fails() {
        let mut form = TaskFormState::new_create();
        form.description = "   \t  ".to_owned();
        assert!(form.validate().is_err());
    }

    #[test]
    fn form_state_validate_invalid_priority_fails() {
        let mut form = TaskFormState::new_create();
        form.description = "有效描述".to_owned();
        form.priority = "urgent".to_owned();
        let result = form.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "优先级必须是 high / medium / low");
    }

    #[test]
    fn form_state_validate_valid_priorities() {
        for p in &["high", "medium", "low", "High", "MEDIUM", "LOW"] {
            let mut form = TaskFormState::new_create();
            form.description = "有效描述".to_owned();
            form.priority = p.to_string();
            assert!(form.validate().is_ok(), "优先级 '{p}' 应该通过校验");
        }
    }

    #[test]
    fn form_state_validate_empty_priority_ok() {
        let mut form = TaskFormState::new_create();
        form.description = "有效描述".to_owned();
        form.priority = String::new();
        assert!(form.validate().is_ok());
    }

    #[test]
    fn form_state_validate_complete_form_ok() {
        let mut form = TaskFormState::new_create();
        form.description = "测试任务".to_owned();
        form.context = "上下文信息".to_owned();
        form.priority = "high".to_owned();
        form.tags = "bug,urgent".to_owned();
        form.eta = "2025-12-31".to_owned();
        assert!(form.validate().is_ok());
    }

    #[test]
    fn form_state_focused_value_mut_and_get() {
        let mut form = TaskFormState::new_create();
        // 默认聚焦 Description
        assert_eq!(form.focused_value(), "");

        // 通过 focused_value_mut 写入
        form.focused_value_mut().push_str("hello");
        assert_eq!(form.focused_value(), "hello");
        assert_eq!(form.description, "hello");

        // 切换到 Context
        form.focused_field = FormField::Context;
        assert_eq!(form.focused_value(), "");
        form.focused_value_mut().push_str("ctx");
        assert_eq!(form.context, "ctx");

        // 切换到 Priority
        form.focused_field = FormField::Priority;
        form.focused_value_mut().push_str("high");
        assert_eq!(form.priority, "high");

        // 切换到 Tags
        form.focused_field = FormField::Tags;
        form.focused_value_mut().push_str("v1,bug");
        assert_eq!(form.tags, "v1,bug");

        // 切换到 Eta
        form.focused_field = FormField::Eta;
        form.focused_value_mut().push_str("2h");
        assert_eq!(form.eta, "2h");
    }

    // ── InputMode 测试 ────────────────────────────────────────────────────

    #[test]
    fn input_mode_variants_distinct() {
        // 确保所有变体互不相等
        let modes = [
            InputMode::Normal,
            InputMode::AddingItem,
            InputMode::EditingItemName,
            InputMode::EditingItemDesc,
            InputMode::AddingNote,
            InputMode::AddingResourceName,
            InputMode::AddingResourceUrl,
        ];
        for (i, a) in modes.iter().enumerate() {
            for (j, b) in modes.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "InputMode variants at {i} and {j} should differ");
                }
            }
        }
    }

    // ── FocusedPane / RightPaneView / Overlay 测试 ────────────────────────

    #[test]
    fn focused_pane_variants_distinct() {
        assert_ne!(FocusedPane::Left, FocusedPane::Right);
    }

    #[test]
    fn right_pane_view_variants_distinct() {
        let views = [
            RightPaneView::TaskList,
            RightPaneView::TaskDetail,
            RightPaneView::TaskForm,
            RightPaneView::Help,
        ];
        for (i, a) in views.iter().enumerate() {
            for (j, b) in views.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "RightPaneView variants at {i} and {j} should differ");
                }
            }
        }
    }

    #[test]
    fn overlay_variants_distinct() {
        let overlays = [
            Overlay::None,
            Overlay::ProjectForm(ProjectFormMode::Create),
            Overlay::ProjectForm(ProjectFormMode::Edit),
            Overlay::DeleteProject,
            Overlay::DeleteTask,
            Overlay::DeleteNote,
            Overlay::Input(InputMode::Normal),
            Overlay::Input(InputMode::AddingItem),
        ];
        for (i, a) in overlays.iter().enumerate() {
            for (j, b) in overlays.iter().enumerate() {
                if i != j {
                    assert_ne!(a, b, "Overlay variants at {i} and {j} should differ");
                }
            }
        }
    }

    // ── App 基础状态测试 ──────────────────────────────────────────────────

    #[test]
    fn app_new_defaults() {
        let app = App::new(None);
        assert_eq!(app.focused_pane(), &FocusedPane::Left);
        assert_eq!(app.right_pane_view(), &RightPaneView::TaskList);
        assert_eq!(app.overlay(), &Overlay::None);
        assert!(!app.should_quit());
        assert!(app.tasks().is_empty());
        assert_eq!(app.selected_index(), 0);
        assert!(app.message().is_none());
        assert!(app.error_message().is_none());
        assert!(app.current_task().is_none());
        assert_eq!(app.sort_mode(), SortMode::Created);
        assert!(app.search_query().is_none());
        assert!(!app.search_mode());
        assert!(!app.confirm_delete());
        assert!(!app.confirm_delete_note());
        assert!(!app.confirm_delete_project());
        assert!(!app.is_overlay_active());
        assert_eq!(app.scroll_offset(), 0);
        assert_eq!(app.selected_item_index(), 0);
        assert!(app.form_state().is_none());
        assert!(app.projects().is_empty());
        assert_eq!(app.project_selected_index(), 0);
    }

    #[test]
    fn app_filtered_and_sorted_tasks_empty() {
        let app = App::new(None);
        let result = app.filtered_and_sorted_tasks();
        assert!(result.is_empty());
    }

    #[test]
    fn app_set_focused_pane() {
        let mut app = App::new(None);
        assert_eq!(app.focused_pane(), &FocusedPane::Left);
        app.set_focused_pane(FocusedPane::Right);
        assert_eq!(app.focused_pane(), &FocusedPane::Right);
        app.set_focused_pane(FocusedPane::Left);
        assert_eq!(app.focused_pane(), &FocusedPane::Left);
    }

    #[test]
    fn app_set_right_pane_view() {
        let mut app = App::new(None);
        assert_eq!(app.right_pane_view(), &RightPaneView::TaskList);
        app.set_right_pane_view(RightPaneView::Help);
        assert_eq!(app.right_pane_view(), &RightPaneView::Help);
        app.set_right_pane_view(RightPaneView::TaskDetail);
        assert_eq!(app.right_pane_view(), &RightPaneView::TaskDetail);
    }
}
