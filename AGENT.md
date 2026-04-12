# AGENT.md — pinchtask 仓库状态

## 项目概述

**pinchtask** 是一个基于 Model Context Protocol (MCP) 的任务管理工具，为 AI Agent 提供结构化的任务管理能力。支持 **CLI 本地操作**、**MCP 服务器** 和 **交互式 TUI** 三种使用模式，共享同一套核心业务逻辑与数据存储。

- **语言**: Rust (Edition 2024)
- **版本**: 0.3.1
- **许可证**: MIT
- **代码量**: ~12000 行 Rust 代码
- **构建状态**: 编译通过，114 个测试全部通过（96 单元测试 + 18 集成测试）

## 技术栈

| 依赖               | 版本     | 用途                                                    |
| ------------------ | -------- | ------------------------------------------------------- |
| tokio              | 1.x      | 异步运行时                                              |
| serde / serde_json | 1.x      | JSON 序列化                                             |
| clap               | 4.x      | CLI 参数解析（嵌套子命令）                              |
| anyhow             | 1.x      | 错误处理                                                |
| thiserror          | 2.x      | 自定义错误类型                                          |
| uuid               | 1.x (v4) | 任务/条目 ID 生成                                       |
| chrono             | 0.4.x    | 时间戳处理                                              |
| tracing            | 0.1.x    | 日志框架                                                |
| rmcp               | 1.3.0    | MCP 协议实现（`#[tool]` 宏、ServerHandler、ToolRouter） |
| schemars           | 1.2.1    | JSON Schema 自动生成（MCP 工具 inputSchema）            |
| sqlx               | 0.8      | SQLite 异步数据库驱动                                   |
| ratatui            | 0.30.0   | TUI 框架（终端 UI 渲染，可选 feature）                  |
| crossterm          | 0.29.0   | 终端控制（raw mode、事件、光标）                        |

## 项目结构

```
pinchtask/
├── src/
│   ├── main.rs              # CLI 入口，退出码处理
│   ├── lib.rs               # 库入口，模块导出
│   ├── server.rs            # MCP 服务器（工具注册、请求分发）
│   ├── store.rs             # SQLite 持久化层（sqlx）
│   ├── core/                # 纯业务逻辑层（CLI/MCP/TUI 共享）
│   │   ├── mod.rs           # 模块入口与公共接口重导出
│   │   ├── task.rs          # 任务级操作
│   │   ├── item.rs          # 清单条目操作
│   │   ├── note.rs          # 笔记操作
│   │   ├── project.rs       # 项目级操作（CRUD + 任务关联）
│   │   ├── resolve.rs       # 短 ID 前缀匹配（CLI/MCP 共享）
│   │   ├── resource.rs      # 资源操作
│   │   ├── workspace.rs     # 工作区发现（.pinchproject 搜索与解析）
│   ├── models/              # 数据模型
│   │   ├── mod.rs           # 模块入口
│   │   ├── task.rs          # Task, ChecklistItem, Resource, TaskMetadata
│   │   └── project.rs       # Project
│   ├── protocol/            # MCP JSON-RPC 协议类型
│   │   └── types.rs         # Request/Response/ToolDefinition 等
│   ├── transport/           # stdio 传输层
│   │   └── mod.rs           # 换行分隔 JSON + Content-Length 双格式支持
│   ├── tools/               # MCP 工具参数定义与测试
│   │   ├── mod.rs           # 模块入口
│   │   ├── params.rs        # 工具参数结构体 + json_schema_for() 内联 $ref
│   │   └── task.rs          # core 层单元测试
│   ├── tui/                 # 交互式终端界面（可选 feature）
│   │   ├── mod.rs           # TUI 入口（终端初始化、主循环）
│   │   ├── app.rs           # 应用状态管理、事件处理
│   │   ├── event.rs         # 事件定义与异步事件总线
│   │   └── ui/              # 渲染模块
│   │       ├── mod.rs       #   主渲染入口（左右分栏布局）
│   │       ├── task_detail.rs #   任务详情渲染
│   │       ├── task_list.rs   #   任务列表渲染
│   │       ├── project_form.rs   # 项目创建/编辑表单（覆盖层弹出）
│   │       ├── project_list.rs   # 项目列表渲染（左栏窄列）
│   │       └── theme.rs       #   视觉主题（配色、图标、进度条）
│   └── cli/                 # CLI 命令处理
│       ├── mod.rs           # 顶层参数解析与嵌套子命令分发
│       ├── task.rs          # task new / ls / show / edit / rm
│       ├── item.rs          # item add / check / mv / summary / edit / rm
│       ├── note.rs          # note add / rm
│       ├── link.rs          # link add / rm
│       ├── project.rs       # project new / ls / show / edit / rm / add-task / rm-task / init
│       ├── output.rs        # 统一输出格式化
│       ├── resolve.rs       # 短 ID 前缀匹配
│       ├── logging.rs       # 日志初始化
│       └── server.rs        # serve 子命令
├── migrations/
│   ├── 20250101000000_init.sql                  # 数据库初始 Schema
│   ├── 20260406100000_add_projects.sql          # 新增 projects 表（已废弃，被下一条替代）
│   └── 20260406110000_refactor_project_relation.sql # 项目关系重构：多对多→一对多
├── tests/
│   └── integration_test.rs  # MCP 协议集成测试
├── docs/
│   ├── architecture.md      # 架构文档
│   ├── cli-reference.md     # CLI 参考文档
│   ├── cli-redesign.md      # CLI 重设计记录
│   ├── mcp-protocol.md      # MCP 协议说明
│   └── tui-design.md        # TUI 设计文档
├── Cargo.toml
└── README.md
```

## 架构设计

```
┌─────────────────────────────────────────────┐
│              Frontend 层                      │
│  ┌──────────────┐ ┌────────┐ ┌────────────┐  │
│  │  CLI (cli/)  │ │  TUI   │ │ MCP Server │  │
│  └──────┬───────┘ └───┬────┘ └─────┬──────┘  │
├─────────┼─────────────┼────────────┼──────────┤
│         │      Adapter 层            │         │
│  ┌──────▼───────┐ ┌──▼──────┐ ┌────▼──────┐  │
│  │ CLI handlers │ │TUI state│ │Tool hand. │  │
│  └──────┬───────┘ └──┬──────┘ └────┬──────┘  │
├─────────┼────────────┼────────────┼──────────┤
│         ▼    Core 业务层            ▼          │
│         ┌──────────────────────────────┐      │
│         │  core/ (纯逻辑)              │      │
│         └──────────┬───────────────────┘      │
├────────────────────┼──────────────────────────┤
│       Store 持久化层│                          │
│  ┌─────────────────▼──────────────┐          │
│  │  store.rs (TaskStore)          │          │
│  │  SQLite + sqlx 异步持久化       │          │
│  └────────────────────────────────┘          │
└──────────────────────────────────────────────┘
```

**核心设计原则**: 所有业务逻辑集中在 `core/` 层，CLI、TUI 和 MCP 工具分别作为适配层调用 core 函数，保证行为一致性。

## 核心功能

### MCP 工具（9 个）

| 工具名                     | 功能                                                 |
| -------------------------- | ---------------------------------------------------- |
| `new_task`                 | 创建新任务（支持初始清单/笔记/资源/元数据/项目关联） |
| `update_task`              | 统一更新多个任务字段（含项目关联）。task_id 支持短 ID 前缀匹配 |
| `manage_checklist_item`    | 统一管理清单条目（add/update/reorder/remove）。task_id 支持短 ID 前缀匹配 |
| `add_note`                 | 添加笔记。task_id 支持短 ID 前缀匹配                 |
| `add_resource`             | 添加资源引用。task_id 支持短 ID 前缀匹配             |
| `get_checklist_summary`    | 获取清单进度摘要。task_id 支持短 ID 前缀匹配         |
| `clear_task`               | 删除任务。task_id 支持短 ID 前缀匹配                 |
| `list_tasks`               | 列出所有任务（支持按项目过滤）。project_id 支持短 ID 前缀匹配 |
| `manage_project`           | 统一管理项目（create/get/update/delete/list）。project_id 支持短 ID 前缀匹配 |

### 工作区自动关联

在项目根目录放置 `.pinchproject` 文件（内容为项目 UUID），CLI/TUI/MCP 操作时自动注入 `project_id`。

| 前端 | 自动注入行为 |
|------|------------|
| CLI | `task new` / `task ls` 未指定 `--project` 时自动使用 `.pinchproject` 中的项目 ID |
| TUI | 启动时自动选中 `.pinchproject` 指定的项目 |
| MCP | `new_task` / `list_tasks` 未指定 `project_id` 时自动使用服务器启动目录的 `.pinchproject` |

优先级：显式指定 > `.pinchproject` 文件 > 无项目（None）

### CLI 命令（嵌套子命令结构）

| 顶层命令     | 子命令                                                   |
| ------------ | -------------------------------------------------------- |
| `task`       | `new`, `ls`, `show`, `edit`, `rm`                        |
| `item`       | `add`, `check`, `mv`, `summary`, `edit`, `rm`            |
| `note`       | `add`, `rm`                                              |
| `link`       | `add`, `rm`                                              |
| `project`    | `new`, `ls`, `show`, `edit`, `rm`, `add-task`, `rm-task`, `init` |
| `serve`      | 启动 MCP 服务器                                          |
| `tui`        | 启动交互式 TUI（需 `tui` feature）                       |
| `completion` | 生成 shell 补全脚本                                      |

### 数据模型

```rust
Task {
    id: String,                      // UUID v4
    task_description: String,
    context_for_all_tasks: Option<String>,
    checklist: Vec<ChecklistItem>,
    notes: Vec<String>,
    resources: Vec<Resource>,
    metadata: Option<TaskMetadata>,   // tags, priority, estimated_completion_time
    project_id: Option<String>,      // 所属项目 ID（一对多外键）
    created_at: String,              // ISO 8601
    updated_at: String,
}

Project {
    id: String,                      // UUID v4
    name: String,
    description: Option<String>,
    created_at: String,              // ISO 8601
    updated_at: String,
}
```

**任务与项目关系**: 一对多。每个任务最多属于一个项目（通过 `tasks.project_id` 外键），删除项目时任务自动解除关联（`ON DELETE SET NULL`）。

## 数据存储

- **格式**: SQLite 数据库文件（tasks.db）
- **默认路径**: `~/.pinchtask/tasks.db`
- **可通过**: `-D` 参数或 `PINCHTASK_DATA_DIR` 环境变量配置
- **Schema**: 混合范式化设计
  - 主表: `tasks`（含 `project_id` 可空外键）、`projects`
  - 子表: `checklist_items`、`notes`、`resources`
  - `metadata` 为 JSON 列
  - 无关联表（任务-项目关系通过外键直接实现）

## 当前状态

### ✅ 已完成

- 完整的 MCP 服务器实现（stdio 传输，JSON-RPC 2.0）
- 完整的 CLI 命令行工具（嵌套子命令结构）
- 核心业务逻辑层解耦
- 9 个 MCP 工具全部注册
- 存储层从 JSON 文件迁移到 SQLite + sqlx（异步）
- 项目管理功能（CRUD、任务关联，一对多外键模型）
- 完整的交互式 TUI 界面（ratatui + crossterm），支持任务/项目列表、详情、创建/编辑表单
- TUI 作为可选 feature，默认禁用（`cargo run --features tui -- tui`）
- 104 个测试全部通过（86 单元测试 + 18 集成测试）
- tools/params 模块包含 schema 内联验证测试
- TUI 模块包含单元测试（FormField、SortMode、TaskFormState、主题函数）
- 双格式 stdio 传输（换行分隔 JSON + Content-Length 头）
- 短 ID 前缀匹配
- JSON 输出模式（`--json`）
- Shell 补全脚本生成
- `.pinchproject` 工作区自动关联（CLI/TUI/MCP 三端支持）

### ⚠️ 已知问题

无

## 开发注意事项

### MCP 工具 Schema：禁止 `$ref` 引用

rmcp 的 `#[tool]` 宏默认使用 `schema_for_type::<T>()` 生成 inputSchema，该函数内部硬编码 `SchemaSettings::draft2020_12()` 且 `inline_subschemas = false`，会导致嵌套的自定义类型（struct/enum）被放入 `$defs` 并通过 `$ref` 引用。**多数 MCP 客户端不支持 JSON Schema 引用解析**，看到 `{"$ref": "..."}` 后无法展开实际字段。

**规则**：如果工具参数类型中包含嵌套的自定义 struct 或 enum（如 `Option<Vec<ChecklistItemInput>>`、`Action` 枚举），**必须**在 `#[tool]` 宏中显式指定 `input_schema` 属性，使用 `tools::params::json_schema_for::<T>()` 生成完全内联的 schema。

```rust
// ❌ 错误：嵌套类型会产生 $ref
#[tool(name = "my_tool", description = "...")]
pub async fn my_tool(&self, Parameters(p): Parameters<MyParams>) -> ... { ... }

// ✅ 正确：显式指定内联 schema
#[tool(name = "my_tool", description = "...",
       input_schema = crate::tools::params::json_schema_for::<MyParams>())]
pub async fn my_tool(&self, Parameters(p): Parameters<MyParams>) -> ... { ... }
```

仅包含基础类型（`String`、`bool`、`u64`、`Option<String>` 等）的工具参数无需此处理，rmcp 默认生成的 schema 不会有 `$ref`。

当前需要显式指定 `input_schema` 的工具：`new_task`（含 `InitialChecklistItem`、`ResourceInput`、`TaskMetadataInput`）、`manage_checklist_item`（含 `Action` 枚举）、`manage_project`（含 `ProjectAction` 枚举）。

### 文档同步：变更代码时必须同步更新文档

代码变更往往导致文档过时。**提交涉及以下范围的代码变更时，必须同时检查并更新对应的文档文件，作为同一提交的一部分：**

| 变更范围 | 需同步检查的文档 |
|----------|-----------------|
| CLI 命令/参数增删改 | `docs/cli-reference.md`、`README.md`（CLI 用法、快速开始） |
| MCP 工具增删改 | `AGENT.md`（MCP 工具表）、`docs/mcp-protocol.md` |
| 数据模型变更 | `AGENT.md`（数据模型章节）、`docs/architecture.md` |
| 项目结构变更（新增/删除/重命名模块） | `AGENT.md`（项目结构树）、`README.md`（项目结构） |
| 新增/删除依赖 | `AGENT.md`（技术栈表） |
| 退出码/错误类型变更 | `AGENT.md`（退出码约定）、`README.md`（退出码） |
| TUI 功能变更 | `README.md`（TUI 快捷键表）、`docs/tui-design.md` |
| 构建方式/feature 变更 | `AGENT.md`（开发命令）、`README.md`（安装/构建） |
| 版本发布 | `Cargo.toml`（version）、`AGENT.md`（版本号、代码量）、`README.md.en`（如有）、`CHANGELOG.md` |

**执行方式**：在提交前，对照上表检查本次变更是否命中某个范围。如果命中，先读取对应文档确认是否需要更新，再一并提交。不要将代码变更和文档更新拆成两个提交。版本发布时，按以下顺序操作：更新 `Cargo.toml` 版本号 → 在 `CHANGELOG.md` 顶部添加本次版本条目 → 同步更新 `AGENT.md` 和 `README.md.en` 中的版本号与代码量等元数据 → 提交并打 tag。



### 🔧 待办/改进方向

- 考虑添加任务搜索/过滤功能
- 考虑添加任务导出/导入功能

## 开发命令

```bash
# 构建
cargo build --release

# 运行测试
cargo test
cargo test -- --nocapture  # 显示输出

# 运行 CLI
cargo run -- task new "测试任务"
cargo run -- task ls
cargo run -- project new "我的项目"
cargo run -- serve

# 运行 TUI（需要 tui feature）
cargo run --features tui -- tui

# 生成补全脚本
cargo run -- completion bash > /etc/bash_completion.d/pinchtask
```

## 退出码约定

| 退出码 | 含义              |
| ------ | ----------------- |
| 0      | 成功              |
| 1      | 一般错误          |
| 2      | 任务/项目未找到   |
| 3      | 数据库 / 配置错误 |

## MCP 配置示例

```json
{
  "mcpServers": {
    "pinchtask": {
      "command": "pinchtask",
      "args": ["serve"],
      "env": {
        "PINCHTASK_DATA_DIR": "/path/to/data"
      }
    }
  }
}
```

协议版本: `2024-11-05`
