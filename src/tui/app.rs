//! 应用状态管理。
//!
//! `App` 持有 TUI 全局可变状态，是事件处理与渲染之间的桥梁。

use std::path::PathBuf;

use crate::store::TaskStore;

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
    /// 任务存储实例
    /// TODO: 延迟初始化（首次访问数据库时创建）
    store: Option<TaskStore>,
}

impl App {
    /// 创建新的应用状态。
    pub fn new(data_dir: Option<PathBuf>) -> Self {
        Self {
            data_dir,
            view: View::TaskList,
            store: None,
        }
    }

    /// 获取当前视图。
    pub fn view(&self) -> &View {
        &self.view
    }

    /// 切换到指定视图。
    pub fn set_view(&mut self, view: View) {
        self.view = view;
    }

    /// 确保存储已初始化并返回不可变引用。
    ///
    /// 首次调用时创建数据库连接，后续调用复用现有连接。
    /// TODO: 实现 TaskStore 的延迟初始化
    pub async fn store(&mut self) -> anyhow::Result<&TaskStore> {
        if self.store.is_none() {
            self.store = Some(TaskStore::new(self.data_dir.clone()).await?);
        }
        Ok(self.store.as_ref().unwrap())
    }

    /// 处理一个应用事件，更新内部状态。
    ///
    /// TODO: 实现事件分发逻辑
    /// - 键盘事件 → 视图切换 / 列表导航 / 表单输入
    /// - 任务操作 → 调用 TaskStore 方法
    pub fn handle_event(&mut self, _event: super::event::AppEvent) -> anyhow::Result<()> {
        // TODO: 根据 event 更新 app 状态
        Ok(())
    }
}
