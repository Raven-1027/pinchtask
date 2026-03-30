# mcp-pinchtask

一个基于 [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) 的任务管理服务器，灵感来源于 mcp-shrimp-task-manager。

为 AI 代理提供结构化的任务管理能力：创建任务、分解清单、追踪进度、记录笔记与资源，全部通过 MCP 协议暴露为可调用的工具。

## 功能列表

提供 **16 个 MCP 工具**，覆盖任务管理的完整生命周期：

| # | 工具名称 | 说明 |
|---|---------|------|
| 1 | `initialize_task` | 创建新任务（含描述、清单、笔记、资源、元数据） |
| 2 | `update_task_description` | 更新任务整体描述 |
| 3 | `update_context` | 更新所有子任务的共享上下文 |
| 4 | `add_checklist_item` | 向清单添加条目 |
| 5 | `update_checklist_item` | 更新清单条目内容 |
| 6 | `mark_task_done` | 标记清单条目为已完成 |
| 7 | `mark_task_undone` | 标记清单条目为未完成 |
| 8 | `reorder_checklist_item` | 移动清单条目位置 |
| 9 | `remove_checklist_item` | 删除清单条目 |
| 10 | `add_note` | 添加笔记 |
| 11 | `add_resource` | 添加资源引用 |
| 12 | `update_metadata` | 更新元数据（标签、优先级、预计完成时间） |
| 13 | `get_checklist_summary` | 获取清单概要（含完成进度） |
| 14 | `clear_task` | 删除任务 |
| 15 | `list_tasks` | 列出所有任务 |
| 16 | `get_current_task_details` | 获取第一个未完成子任务的详细信息 |

## 安装

### 从源码构建

```bash
git clone <repo-url>
cd mcp-pinchtask
cargo build --release
```

编译产物位于 `target/release/mcp-pinchtask`。

### 前置条件

- Rust 1.75+（推荐使用 `rustup` 安装最新稳定版）

## 使用方法

### 命令行参数

```
mcp-pinchtask [OPTIONS]

选项:
      --data-dir <PATH>     数据存储目录路径（默认: ~/.mcp-pinchtask）
      --log-level <LEVEL>   日志级别 [默认: warn] [可能值: trace, debug, info, warn, error]
  -h, --help                显示帮助信息
  -V, --version             显示版本号
```

环境变量：
- `PINCHTASK_DATA_DIR` — 数据存储目录路径
- `PINCHTASK_LOG_LEVEL` — 日志级别

### 配置到 MCP 客户端

在 MCP 客户端的配置文件中添加：

```json
{
  "mcpServers": {
    "pinchtask": {
      "command": "/path/to/mcp-pinchtask",
      "args": []
    }
  }
}
```

如果需要指定数据目录：

```json
{
  "mcpServers": {
    "pinchtask": {
      "command": "/path/to/mcp-pinchtask",
      "args": ["--data-dir", "/custom/data/path"]
    }
  }
}
```

### 传输协议

服务器通过 **stdio** 与客户端通信，支持两种 MCP 标准传输格式：
- **换行分隔 JSON**（默认）：每行一个 JSON-RPC 消息
- **Content-Length 头**：`Content-Length: N\r\n\r\n` + N 字节 JSON

## 工具参数说明

### initialize_task

创建新任务。

```json
{
  "task_description": "实现用户认证模块",
  "context_for_all_tasks": "所有子任务需遵循 OAuth 2.0 规范",
  "initial_checklist": [
    {
      "task": "设计数据库模型",
      "detailed_description": "设计用户表、Token 表和权限表",
      "context_and_plan": "参考第 3 章数据库设计文档"
    }
  ],
  "notes": ["需要在上线前完成安全审计"],
  "resources": [
    { "name": "OAuth 2.0 RFC", "url": "https://tools.ietf.org/html/rfc6749" }
  ],
  "metadata": {
    "tags": ["auth", "backend"],
    "priority": "high",
    "estimated_completion_time": "2024-12-01T00:00:00Z"
  }
}
```

**必填参数：** `task_description`
**可选参数：** `context_for_all_tasks`, `initial_checklist`, `notes`, `resources`, `metadata`

### add_checklist_item

添加清单条目。

```json
{
  "task_id": "uuid-of-task",
  "task": "步骤名称",
  "detailed_description": "详细描述",
  "context_and_plan": "上下文和执行计划（可选）"
}
```

**必填参数：** `task_id`, `task`, `detailed_description`

### update_checklist_item

更新清单条目。只更新传入的字段，未指定的字段保持原值。

```json
{
  "task_id": "uuid-of-task",
  "index": 0,
  "task": "新名称（可选）",
  "detailed_description": "新描述（可选）",
  "context_and_plan": "新计划（传 null 清空）",
  "done": true
}
```

**必填参数：** `task_id`, `index`

### reorder_checklist_item

移动清单条目位置。

```json
{
  "task_id": "uuid-of-task",
  "from_index": 2,
  "to_index": 0
}
```

### update_metadata

更新任务元数据。

```json
{
  "task_id": "uuid-of-task",
  "metadata": {
    "tags": ["urgent", "frontend"],
    "priority": "high",
    "estimated_completion_time": "P3D"
  }
}
```

`priority` 可选值：`high`, `medium`, `low`

### get_checklist_summary / get_current_task_details

```json
{ "task_id": "uuid-of-task" }
```

`get_current_task_details` 无需参数，自动查找第一个含未完成子任务的任务。

## 数据存储

任务数据以 JSON 文件形式存储在数据目录中（默认 `~/.mcp-pinchtask/`），每个任务一个文件：

```
~/.mcp-pinchtask/
├── 550e8400-e29b-41d4-a716-446655440000.json
├── 6ba7b810-9dad-11d1-80b4-00c04fd430c8.json
└── ...
```

## License

MIT
