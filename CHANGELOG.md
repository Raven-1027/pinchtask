# Changelog

本项目的所有重要变更都将记录在此文件中。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

## [0.2.0] - 2026-04-06

### Added

- CLI 嵌套子命令结构：`task`、`item`、`note`、`link`、`project` 五组子命令，替代原有的扁平命令
- 项目管理功能：项目 CRUD、任务关联，CLI 子命令（`project new/ls/show/edit/rm/add-task/rm-task`）
- `manage_project` MCP 工具（create/get/update/delete/list），MCP 工具总数增至 10 个
- TUI 左右分栏布局：左栏项目列表（30%），右栏任务列表/详情/表单（70%）
- TUI 面板边框：未选中白色，选中青色，危险操作红色
- TUI 优先级枚举选择器：聚焦时 `Space`/`←`/`→` 循环切换 `—`/`low`/`medium`/`high`，不再需要手动输入
- TUI 作为可选 feature，默认禁用（`cargo run --features tui -- tui`）
- 数据库迁移脚本：`20260406100000_add_projects.sql`（新增 projects 表）、`20260406110000_refactor_project_relation.sql`（多对多→一对多）
- `Task` 模型新增 `project_id: Option<String>` 字段
- `Project` 数据模型

### Changed

- CLI 命令从扁平结构（`new`、`ls`、`show`…）改为嵌套子命令（`task new`、`task ls`、`project ls`…）
- 任务与项目关系从多对多（`task_projects` 关联表）改为一对多（`tasks.project_id` 外键，`ON DELETE SET NULL`）
- TUI 从单栏顺序导航（TaskList → TaskDetail → TaskForm）改为左右分栏焦点切换（`→`/`←` 切换面板）
- TUI 新建任务自动关联当前选中的项目
- 核心层 API：`add_task_to_project`/`remove_task_from_project` 统一为 `set_task_project(store, task_id, Option<&str>)`
- 核心层 API：`get_projects_for_task` 改为 `get_project_for_task`（返回 `Option<Project>`）
- 核心层 API：`get_tasks_for_project` 改为直接查询 `tasks.project_id`
- `new_task` CLI 参数 `--project` 从 `Vec<String>` 改为 `Option<String>`（单个项目）

### Fixed

- 修复状态栏键位提示被 `message` 完全覆盖的问题：消息作为前缀显示，键位提示始终可见
- 修复状态栏文字不可见：前景色 `MUTED`(DarkGray) 与背景色 `TITLE_BG`(DarkGray) 同色，改为 White on Black
- 修复 TUI 面板边框不显示：`Block.inner(area)` 只计算内部区域未渲染 Block 本身，改为先 `render_widget(&block, area)` 再取 `inner(area)`
- 修复 MCP Schema 中 `Option<T>` 可空类型对客户端不兼容的问题
- 修复删除确认弹窗无背景色导致与下层内容混在一起：添加 `bg(Color::Black)` 实心背景
- 修复三个删除确认对话框重复代码：提取 `draw_confirm_dialog` 通用函数

### Removed

- 删除 `task_projects` 关联表（由 `tasks.project_id` 外键替代）
- 删除 TUI `project_detail.rs`（功能合并到左栏项目列表）
- 删除独立的 ProjectList/ProjectDetail/ProjectForm 视图（由分栏布局替代）

[0.2.0]: https://github.com/Raven-1027/pinchtask/releases/tag/v0.2.0

## [0.1.0] - 2026-04-04

### Added

- 项目初始化，包含数据模型、文件存储和基础 MCP 工具处理器
- 实现 MCP 协议层（JSON-RPC 2.0）和 stdio 传输层，支持换行分隔 JSON 和 Content-Length 双格式
- CLI 命令行工具：`new`、`ls`、`show`、`edit`、`rm`、`add`、`check`、`mv`、`summary`、`note`、`tag`、`link`
- 短 ID 前缀匹配，支持用 ID 前缀替代完整 UUID 操作任务
- Shell 补全脚本生成（`completion` 子命令）
- `-D` 短参数指定数据目录，`PINCHTASK_DATA_DIR` 环境变量支持
- `--json` 输出模式
- 迁移至 SQLite 持久化（sqlx 异步驱动），解耦核心业务逻辑层（`core/`）
- 迁移至 rmcp 框架（`#[tool]` 宏、ServerHandler、ToolRouter），移除自实现的 protocol/transport 层
- 为所有 MCP 工具的 description 追加使用建议
- 完整的交互式 TUI 界面（ratatui + crossterm）：
  - 任务列表视图（排序、筛选、滚动）
  - 任务详情视图（清单、笔记、资源、元数据）
  - 任务创建与编辑表单
  - 笔记与资源操作
  - 任务删除
  - 统一视觉主题（配色方案、图标、进度条）
  - 最小终端尺寸检测
- TUI 模块 55+ 个单元测试（FormField、SortMode、TaskFormState、主题函数）
- `tools/params` 模块包含 6 个 schema 内联验证测试
- 20 个 MCP 协议集成测试
- 项目重命名：`mcp-pinchtask` → `pinchtask`

### Changed

- CLI 命令结构扁平化
- 存储层从 JSON 文件迁移到 SQLite（`~/.pinchtask/tasks.db`）
- MCP 工具数量从 17 个精简至 9 个：
  - 移除 5 个冗余工具
  - 合并 4 个清单操作工具（`add_checklist_item`、`update_checklist_item`、`reorder_checklist_item`、`remove_checklist_item`）为统一的 `manage_checklist_item`
- `initialize_task` 重命名为 `new_task`
- 扁平化 `ManageChecklistItemParams` schema，所有操作共享单一 struct 通过 `action` 字段区分

### Fixed

- 修复 stdio 传输层生命周期问题和未使用变量警告
- 修复集成测试编译错误和 `initial_checklist` 默认值解析
- 修复 `show` 子命令的清单格式
- 修复进度排序逻辑和 `ls -l` 标签显示
- 修复 TUI 多处编译错误（derive 宏拼写、Action 模式匹配、类型标注、生命周期）
- 修复 EventBus sender 未连接到 App 导致异步 Action 回传失败
- 修复 TUI theme 测试中 `.len()` 对多字节 UTF-8 字符（`█`、`░`、`─`）返回字节长度而非字符数

### Security

- MCP 工具 inputSchema 禁止 `$ref` 引用：通过 `json_schema_for::<T>()` 使用 `inline_subschemas = true` 生成完全内联的 schema，确保不支持 JSON Schema 引用解析的 MCP 客户端也能正确解析工具参数

[0.1.0]: https://github.com/Raven-1027/pinchtask/releases/tag/v0.1.0
