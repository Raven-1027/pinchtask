# AGENT.md — mcp-pinchtask 仓库状态

## 项目概述

**mcp-pinchtask** 是一个基于 Model Context Protocol (MCP) 的任务管理工具，为 AI Agent 提供结构化的任务管理能力。支持 CLI 本地操作和 MCP 服务器两种使用模式，共享同一套核心业务逻辑与数据存储。

- **语言**: Rust (Edition 2021)
- **版本**: 0.1.0
- **许可证**: MIT
- **代码量**: ~5000 行 Rust 代码
- **构建状态**: 编译通过，75 个测试全部通过

## 技术栈

| 依赖               | 版本     | 用途                                       |
| ------------------ | -------- | ------------------------------------------ |
| tokio              | 1.x      | 异步运行时                                 |
| serde / serde_json | 1.x      | JSON 序列化                                |
| clap               | 4.x      | CLI 参数解析                               |
| anyhow             | 1.x      | 错误处理                                   |
| thiserror          | 2.x      | 自定义错误类型                             |
| uuid               | 1.x (v4) | 任务/条目 ID 生成                          |
| chrono             | 0.4.x    | 时间戳处理                                 |
| tracing            | 0.1.x    | 日志框架                                   |
| rmcp               | 1.3.0    | MCP 协议参考（未直接使用，自行实现协议层） |
| sqlx               | 0.8      | SQLite 异步数据库驱动                      |

## 项目结构

```
mcp-pinchtask/
├── src/
│   ├── main.rs              # CLI 入口，退出码处理
│   ├── lib.rs               # 库入口，模块导出
│   ├── server.rs            # MCP 服务器（工具注册、请求分发）
│   ├── store.rs             # SQLite 持久化层（sqlx）
│   ├── core/                # 纯业务逻辑层（CLI/MCP 共享）
│   │   ├── mod.rs           # 模块入口与公共接口重导出
│   │   ├── task.rs          # 任务级操作
│   │   ├── item.rs          # 清单条目操作
│   │   ├── note.rs          # 笔记操作
│   │   └── resource.rs      # 资源操作
│   ├── models/              # 数据模型
│   │   └── task.rs          # Task, ChecklistItem, Resource, TaskMetadata
│   ├── protocol/            # MCP JSON-RPC 协议类型
│   │   └── types.rs         # Request/Response/ToolDefinition 等
│   ├── transport/           # stdio 传输层
│   │   └── mod.rs           # 换行分隔 JSON + Content-Length 双格式支持
│   ├── tools/               # MCP 工具适配层
│   │   └── task.rs          # 参数解析 → core 调用 → 结果封装
│   └── cli/                 # CLI 命令处理
│       ├── mod.rs           # 顶层参数解析与子命令分发
│       ├── task.rs          # new / ls / show / edit / rm
│       ├── item.rs          # add / check / mv / summary / edit / rm
│       ├── note.rs          # note
│       ├── meta.rs          # tag
│       ├── resource.rs      # link
│       ├── output.rs        # 统一输出格式化
│       ├── resolve.rs       # 短 ID 前缀匹配
│       ├── logging.rs       # 日志初始化
│       └── server.rs        # serve 子命令
├── migrations/
│   └── 20250101000000_init.sql  # 数据库 Schema 定义
├── tests/
│   └── integration_test.rs  # MCP 协议集成测试
├── docs/
│   ├── architecture.md      # 架构文档
│   ├── cli-reference.md     # CLI 参考文档
│   ├── cli-redesign.md      # CLI 重设计记录
│   └── mcp-protocol.md      # MCP 协议说明
├── Cargo.toml
└── README.md
```

## 架构设计

```
┌─────────────────────────────────────────────┐
│              Frontend 层                      │
│  ┌──────────────┐    ┌────────────────────┐  │
│  │  CLI (cli/)  │    │  MCP Server        │  │
│  └──────┬───────┘    └────────┬───────────┘  │
├─────────┼─────────────────────┼──────────────┤
│         │      Adapter 层     │              │
│  ┌──────▼───────┐    ┌───────▼────────────┐  │
│  │ CLI handlers │    │  Tool handlers     │  │
│  └──────┬───────┘    └───────┬────────────┘  │
├─────────┼─────────────────────┼──────────────┤
│         ▼    Core 业务层      ▼              │
│         ┌──────────────────────┐             │
│         │  core/ (纯逻辑)      │             │
│         └──────────┬───────────┘             │
├────────────────────┼─────────────────────────┤
│       Store 持久化层│                         │
│  ┌─────────────────▼──────────────┐          │
│  │  store.rs (TaskStore)          │          │
│  │  SQLite + sqlx 异步持久化       │          │
│  └────────────────────────────────┘          │
└──────────────────────────────────────────────┘
```

**核心设计原则**: 所有业务逻辑集中在 `core/` 层，CLI 和 MCP 工具分别作为适配层调用 core 函数，保证行为一致性。

## 核心功能

### MCP 工具（17 个）

| 工具名                     | 功能                                        |
| -------------------------- | ------------------------------------------- |
| `initialize_task`          | 创建新任务（支持初始清单/笔记/资源/元数据） |
| `update_task`              | 统一更新多个任务字段                        |
| `update_task_description`  | 更新任务描述                                |
| `update_context`           | 更新共享上下文                              |
| `add_checklist_item`       | 添加清单条目                                |
| `update_checklist_item`    | 更新清单条目                                |
| `mark_task_done`           | 标记条目完成                                |
| `mark_task_undone`         | 标记条目未完成                              |
| `reorder_checklist_item`   | 移动条目顺序                                |
| `remove_checklist_item`    | 删除清单条目                                |
| `add_note`                 | 添加笔记                                    |
| `add_resource`             | 添加资源引用                                |
| `update_metadata`          | 更新元数据（标签/优先级/预计时间）          |
| `get_checklist_summary`    | 获取清单进度摘要                            |
| `clear_task`               | 删除任务                                    |
| `list_tasks`               | 列出所有任务                                |
| `get_current_task_details` | 获取第一个未完成任务详情                    |

### CLI 命令

`new`, `ls`, `show`, `edit`, `rm`, `add`, `check`, `mv`, `summary`, `note`, `tag`, `link`, `serve`, `completion`

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
    created_at: String,              // ISO 8601
    updated_at: String,
}
```

## 数据存储

- **格式**: SQLite 数据库文件（tasks.db）
- **默认路径**: `~/.mcp-pinchtask/tasks.db`
- **可通过**: `-D` 参数或 `PINCHTASK_DATA_DIR` 环境变量配置
- **Schema**: 混合范式化设计，主表 tasks + checklist_items/notes/resources 独立表，metadata 为 JSON 列

## 当前状态

### ✅ 已完成

- 完整的 MCP 服务器实现（stdio 传输，JSON-RPC 2.0）
- 完整的 CLI 命令行工具
- 核心业务逻辑层解耦
- 17 个 MCP 工具全部注册
- 存储层从 JSON 文件迁移到 SQLite + sqlx（异步）
- 75 个测试全部通过（66 单元测试 + 9 集成测试）
- 双格式 stdio 传输（换行分隔 JSON + Content-Length 头）
- 短 ID 前缀匹配
- JSON 输出模式（`--json`）
- Shell 补全脚本生成

### ⚠️ 已知问题

无

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
cargo run -- new "测试任务"
cargo run -- ls
cargo run -- serve

# 生成补全脚本
cargo run -- completion bash > /etc/bash_completion.d/mcp-pinchtask
```

## 退出码约定

| 退出码 | 含义              |
| ------ | ----------------- |
| 0      | 成功              |
| 1      | 一般错误          |
| 2      | 任务未找到        |
| 3      | 数据库 / 配置错误 |

## MCP 配置示例

```json
{
  "mcpServers": {
    "pinchtask": {
      "command": "mcp-pinchtask",
      "args": ["serve"],
      "env": {
        "PINCHTASK_DATA_DIR": "/path/to/data"
      }
    }
  }
}
```

协议版本: `2024-11-05`
