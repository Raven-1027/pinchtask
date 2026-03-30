# mcp-pinchtask

> MCP Task Manager — 基于 Rust 的 [Model Context Protocol](https://modelcontextprotocol.io/) 任务管理服务器。

受 [mcp-shrimp-task-manager](https://github.com/yym68686/mcp-shrimp-task-manager) 启发，提供一套完整的任务 CRUD、清单管理、笔记与资源追踪工具，专为 AI Agent 工作流设计。

## ✨ 功能特性

通过 16 个 MCP 工具管理任务的全生命周期：

| 工具 | 说明 |
|------|------|
| `initialize_task` | 创建新任务（含可选清单、笔记、资源、元数据） |
| `update_task_description` | 更新任务描述 |
| `update_context` | 更新所有子任务的共享上下文 |
| `add_checklist_item` | 添加清单条目 |
| `update_checklist_item` | 更新清单条目 |
| `mark_task_done` | 标记清单条目为已完成 |
| `mark_task_undone` | 标记清单条目为未完成 |
| `reorder_checklist_item` | 重排清单条目顺序 |
| `remove_checklist_item` | 删除清单条目 |
| `add_note` | 添加笔记 |
| `add_resource` | 添加资源引用 |
| `update_metadata` | 更新元数据（标签、优先级、预估时间） |
| `get_checklist_summary` | 获取清单完成进度摘要 |
| `clear_task` | 删除任务 |
| `list_tasks` | 列出所有任务 |
| `get_current_task_details` | 获取当前未完成子任务的详细信息 |

## 📦 安装

### 从源码构建

```bash
git clone https://github.com/your-username/mcp-pinchtask.git
cd mcp-pinchtask
cargo build --release
```

二进制文件位于 `target/release/mcp-pinchtask`。

### 环境要求

- Rust 1.75+（edition 2021）
- 无外部运行时依赖

## ⚙️ 配置

### 数据目录

任务数据默认存储在 `~/.mcp-pinchtask/` 目录下，每个任务保存为独立的 JSON 文件。

若需自定义数据目录，可通过代码层面传入路径（暂不支持命令行参数配置）。

### 日志级别

通过环境变量控制日志级别：

```bash
export PINCHTASK_LOG_LEVEL=debug   # trace | debug | info | warn | error
# 或使用 RUST_LOG 环境变量（优先级更高）
export RUST_LOG=mcp_pinchtask=debug
```

## 🔌 客户端配置

### Claude Desktop

编辑 `~/Library/Application Support/Claude/claude_desktop_config.json`（macOS）或对应的配置文件：

```json
{
  "mcpServers": {
    "mcp-pinchtask": {
      "command": "/path/to/mcp-pinchtask",
      "args": []
    }
  }
}
```

### Cursor

在 Cursor 的 MCP 设置中添加：

```json
{
  "mcpServers": {
    "mcp-pinchtask": {
      "command": "/path/to/mcp-pinchtask",
      "args": []
    }
  }
}
```

### 通用配置

mcp-pinchtask 使用标准 stdio 传输协议，兼容所有支持 MCP 的客户端。只需将 `command` 指向编译好的二进制文件即可。

## 🛠️ 开发

```bash
# 运行测试
cargo test

# 运行 Clippy 检查
cargo clippy --all-targets --all-features -- -D warnings

# 构建发布版本
cargo build --release
```

### 项目结构

```
src/
├── main.rs          # 入口：CLI 参数解析、日志初始化、启动服务器
├── lib.rs           # 库根：模块声明
├── server.rs        # MCP 服务器：工具注册、请求分发
├── store.rs         # 持久化层：文件系统读写
├── models/
│   ├── mod.rs
│   └── task.rs      # 数据模型：Task, ChecklistItem, Resource, TaskMetadata
├── protocol/
│   ├── mod.rs
│   └── types.rs     # MCP JSON-RPC 协议类型
├── tools/
│   ├── mod.rs
│   └── task.rs      # 工具处理器：参数解析、业务逻辑
└── transport/
    └── mod.rs       # Stdio 传输层
```

## 📄 协议

mcp-pinchtask 遵循 [MCP 协议规范 2024-11-05](https://spec.modelcontextprotocol.io/specification/2024-11-05/)，支持：

- **initialize** — 握手与能力协商
- **tools/list** — 枚举可用工具
- **tools/call** — 调用指定工具
- **ping** — 心跳检测
- 换行分隔 JSON 与 Content-Length 两种传输格式

## 📝 License

MIT
