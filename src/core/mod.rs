//! 纯业务逻辑层（core）。
//!
//! CLI 和 MCP handler 共享的业务逻辑统一存放于此模块。
//! 所有函数签名统一为 `async fn xxx(store: &TaskStore, ...) -> Result<T, StoreError>`，
//! 不依赖任何传输层或协议类型。

pub mod item;
pub mod note;
pub mod project;
pub mod resource;
pub mod task;

// 便捷重导出：允许外部通过 `crate::core::initialize_task(...)` 直接调用。
pub use item::{
    add_checklist_item, mark_task_done, mark_task_undone, remove_checklist_item,
    reorder_checklist_item, update_checklist_item,
};
pub use note::{add_note, delete_note};
pub use project::{
    create_project, delete_project, delete_project_with_tasks, get_project,
    get_project_for_task, get_tasks_for_project, list_projects, set_task_project,
    update_project,
};
pub use resource::{add_resource, delete_resource};
pub use task::{
    clear_task, get_checklist_summary, initialize_task, list_tasks_summary, update_context,
    update_metadata, update_task_description,
};
