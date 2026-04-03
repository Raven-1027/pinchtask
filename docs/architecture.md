# 架构文档

## 概述

pinchtask 是一个基于 MCP (Model Context Protocol) 的任务管理工具，同时提供 CLI 本地操作与 MCP 服务器模式。项目使用 Rust 编写，通过 SQLite + sqlx 实现异步持久化。

## 分层设计

```
┌─────────────────────────────────────────────┐
│              Frontend 层                      │
│  ┌──────────────┐    ┌────────────────────┐  │
│  │  CLI (cli/)  │    │  MCP Server        │  │
│  │  命令行入口   │    │  (server.rs)       │  │
│  └──────┬───────┘    └────────┬───────────┘  │
├─────────┼─────────────────────┼──────────────┤
│         │      Adapter 层     │              │
│  ┌──────▼───────┐    ┌───────▼────────────┐  │
│  │ CLI handlers │    │  Tool handlers     │  │
│  │ (cli/*.rs)   │    │  (tools/task.rs)   │  │
│  └──────┬───────┘    └───────┬────────────┘  │
├─────────┼─────────────────────┼──────────────┤
│         ▼    Core 业务层      ▼              │
│         ┌──────────────────────┐             │
│         │  core/               │             │
│         │  task.rs  item.rs    │             │
│         │  note.rs  resource.rs│             │
│         └──────────┬───────────┘             │
├────────────────────┼─────────────────────────┤
│       Store 持久化层│                         │
│  ┌─────────────────▼──────────────┐          │
│  │  store.rs (TaskStore)          │          │
│  │  SQLite 异步读写, 数据库迁移     │          │
│  └────────────────────────────────┘          │
├──────────────────────────────────────────────┤
│  models/       │  protocol/  │ transport/    │
│  数据结构定义    │  MCP 类型   │  Stdio 传输   │
└──────────────────────────────────────────────┘
```

## 模块职责

| 模块           | 路径             | 职责                                                        |
| -------------- | ---------------- | ----------------------------------------------------------- |
| **CLI**        | `src/cli/`       | 命令行参数解析与命令分发                                    |
| **MCP Server** | `src/server.rs`  | MCP 协议生命周期管理，工具注册与请求路由                    |
| **Tools**      | `src/tools/`     | MCP tool 适配层：将 JSON-RPC 参数解析后调用 core 层         |
| **Core**       | `src/core/`      | 纯业务逻辑层，CLI 和 MCP handler 共享，不依赖任何传输层类型 |
| **Store**      | `src/store.rs`   | SQLite 持久化层，使用 sqlx 异步驱动，支持数据库迁移         |
| **Models**     | `src/models/`    | 数据结构定义（Task, ChecklistItem, Resource, TaskMetadata） |
| **Protocol**   | `src/protocol/`  | MCP JSON-RPC 协议类型定义                                   |
| **Transport**  | `src/transport/` | Stdio 传输层读写                                            |

### Core 层详解

Core 层是项目的核心，按职责拆分为四个子模块：

| 子模块        | 函数                       | 说明               |
| ------------- | -------------------------- | ------------------ |
| `task.rs`     | `initialize_task`          | 创建新任务         |
|               | `update_task_description`  | 更新任务描述       |
|               | `update_context`           | 更新共享上下文     |
|               | `update_metadata`          | 更新元数据         |
|               | `clear_task`               | 删除任务           |
|               | `get_checklist_summary`    | 获取清单概要       |
|               | `get_current_task_details` | 获取当前子任务详情 |
|               | `list_tasks_summary`       | 列出所有任务摘要   |
| `item.rs`     | `add_checklist_item`       | 添加清单条目       |
|               | `update_checklist_item`    | 更新清单条目       |
|               | `mark_task_done`           | 标记完成           |
|               | `mark_task_undone`         | 标记未完成         |
|               | `remove_checklist_item`    | 删除条目           |
|               | `reorder_checklist_item`   | 重排条目           |
| `note.rs`     | `add_note`                 | 添加笔记           |
| `resource.rs` | `add_resource`             | 添加资源引用       |

所有 core 函数签名统一为：

```rust
fn xxx(store: &TaskStore, ...) -> Result<Task|String|(), StoreError>
```

### Store 层详解

`TaskStore` 基于 SQLite 实现异步持久化，使用 sqlx 作为数据库驱动：

- **默认路径**：`~/.pinchtask/tasks.db`
- **Schema**：采用混合范式化设计，主表 `tasks` + 关联表 `checklist_items`/`notes`/`resources`，`metadata` 为 JSON 列
- **迁移**：启动时自动执行 `migrations/` 目录下的 SQL 迁移脚本
- **级联删除**：删除任务时自动清理关联的清单条目、笔记和资源
- **自动管理**：`create_task` 自动生成 UUID v4 和时间戳；`update_task` 自动刷新 `updated_at`

错误类型 `StoreError` 覆盖三种场景：

- `Io` — 数据库文件/目录操作失败
- `Database` — SQL 执行失败
- `NotFound` — 任务或条目不存在

## 数据模型

```
Task
├── id: String (UUID v4)
├── task_description: String
├── context_for_all_tasks: Option<String>
├── checklist: Vec<ChecklistItem>
│   ├── id: String (UUID v4)
│   ├── task: String
│   ├── detailed_description: String
│   ├── context_and_plan: Option<String>
│   └── done: bool
├── notes: Vec<String>
├── resources: Vec<Resource>
│   ├── name: String
│   ├── url: String
│   └── description: Option<String>
├── metadata: Option<TaskMetadata>
│   ├── tags: Option<Vec<String>>
│   ├── priority: Option<String>  ("high" | "medium" | "low")
│   └── estimated_completion_time: Option<String>
├── created_at: String (ISO 8601)
└── updated_at: String (ISO 8601)
```

## 调用链示例

### MCP 模式：标记子任务完成

```
客户端 → StdioTransport → McpServer::handle_request()
       → McpServer::handle_tools_call()
       → task_tools::mark_task_done_handler(store, args)
       → core::mark_task_done(store, task_id, index)
       → store.get_task(task_id).await
       → task.checklist[index].done = true
       → store.update_task(&mut task).await
       → sqlx::query UPDATE checklist_items SET done = ?
       → 返回 CallToolResult
```

### CLI 模式：添加清单条目

```
cli::run() → cli::item::add_item_command()
           → core::add_checklist_item(store, task_id, name, desc, plan)
           → store.get_task().await → store.update_task().await
```

## 依赖关系图

```
main.rs ──→ cli/mod.rs ──→ cli/{task,item,note,resource,meta}.rs
                              │
                              ▼
                           core/{task,item,note,resource}.rs
                              │
                              ▼
                           store.rs ──→ models/task.rs

server.rs ──→ tools/task.rs ──→ core/*
    │              │
    ▼              ▼
protocol/types.rs  store.rs
transport/StdioTransport
```

## 测试策略

项目包含 **75 个测试**，按模块分布：

| 模块          | 测试文件                      | 覆盖范围                                           |
| ------------- | ----------------------------- | -------------------------------------------------- |
| Store         | `src/store.rs::tests`         | CRUD 操作、时间戳刷新、数据往返、排序、级联删除    |
| Core/Task     | `src/core/task.rs::tests`     | 初始化、更新、删除、摘要、当前任务详情             |
| Core/Item     | `src/core/item.rs::tests`     | 添加/更新/删除/重排条目、标记完成/未完成、边界检查 |
| Core/Note     | `src/core/note.rs::tests`     | 添加笔记、持久化验证、空内容处理                   |
| Core/Resource | `src/core/resource.rs::tests` | 添加资源（有/无描述）、持久化、多资源              |
| Tools         | `src/tools/task.rs::tests`    | MCP handler 集成测试                               |

所有测试使用 `tempfile::tempdir()` 创建临时目录，互不干扰，可并行运行。
