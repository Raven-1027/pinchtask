# Changelog

本项目的所有重要变更都将记录在此文件中。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

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
