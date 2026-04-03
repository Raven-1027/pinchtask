# mcp-pinchtask

基于 Model Context Protocol (MCP) 的任务管理工具，为 AI Agent 提供结构化的任务管理能力。

支持 **CLI 本地操作** 和 **MCP 服务器** 两种使用模式，共享同一套核心业务逻辑与数据存储。

## 功能特性

- **任务管理** — 创建、查看、编辑、删除任务，支持描述与共享上下文
- **清单系统** — 每个任务包含有序清单条目，支持增删改查、标记完成/未完成、重排序
- **笔记** — 向任务追加自由格式的文本笔记
- **资源引用** — 关联外部文档、URL 或文件路径
- **元数据** — 标签、优先级（high/medium/low）、预计完成时间
- **短 ID 匹配** — CLI 支持输入 UUID 前 4+ 位即可定位任务
- **双模式运行** — CLI 直接操作 / MCP stdio 服务器供 AI Agent 调用
- **JSON 输出** — 所有查询类命令支持 `--json` 标志输出机器可读格式

## 安装

### 从源码构建

```bash
# 需要 Rust 1.70+ 和 Cargo
git clone https://github.com/ravenxrq/mcp-pinchtask.git
cd mcp-pinchtask
cargo build --release
# 二进制文件位于 target/release/mcp-pinchtask
```

### 使用 Cargo 安装

```bash
cargo install --git https://github.com/ravenxrq/mcp-pinchtask
```

## 快速开始

### 创建并管理一个任务

```bash
# 创建任务
mcp-pinchtask new "实现用户登录功能" -c "使用 JWT 认证方案"

# 查看任务列表
mcp-pinchtask ls

# 添加清单条目
mcp-pinchtask add <TASK_ID> "设计数据库表结构" -d "users 表和 sessions 表"

# 标记条目完成
mcp-pinchtask check <TASK_ID> 0

# 查看进度摘要
mcp-pinchtask summary <TASK_ID>

# 查看任务详情
mcp-pinchtask show <TASK_ID>
```

### 启动 MCP 服务器

```bash
# 直接运行（无子命令时自动进入服务器模式）
mcp-pinchtask

# 或显式指定
mcp-pinchtask serve
```

## CLI 用法

```
mcp-pinchtask [OPTIONS] [COMMAND]

全局选项:
  -D, --data-dir <DIR>      数据存储目录（默认: ~/.mcp-pinchtask）
      --log-level <LEVEL>   日志级别 (trace, debug, info, warn, error)
  -v, --verbose             详细输出（等价于 --log-level debug）
  -q, --quiet               安静模式（等价于 --log-level error）
      --json                以 JSON 格式输出

命令:
  new        创建新任务
  ls         列出任务
  show       查看任务详情
  edit       编辑任务或清单条目
  rm         删除任务或清单条目
  add        添加清单条目
  check      切换清单条目完成/未完成状态
  mv         移动清单条目顺序
  summary    查看清单进度摘要
  note       添加笔记
  tag        设置标签和元数据
  link       添加资源引用
  serve      启动 MCP 服务器
  completion 生成 shell 补全脚本
```

> 完整命令参数说明请参考 [CLI 参考文档](docs/cli-reference.md)。

## MCP 配置

在支持 MCP 的 AI 客户端（如 Claude Desktop、Cursor 等）中配置：

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

或直接使用 `mcp-pinchtask`（无子命令时默认启动服务器模式）：

```json
{
  "mcpServers": {
    "pinchtask": {
      "command": "mcp-pinchtask"
    }
  }
}
```

服务器通过 **stdio** 传输协议与客户端通信，支持：

- 换行分隔 JSON 格式
- `Content-Length` 头格式

协议版本：`2024-11-05`

## 数据存储

任务数据存储在 SQLite 数据库文件中：

```
~/.mcp-pinchtask/
└── tasks.db
```

数据库采用混合范式化设计：`tasks` 主表存储任务基本信息，`checklist_items`、`notes`、`resources` 为独立关联表，`metadata` 以 JSON 列存储。可通过 `-D` 参数或 `PINCHTASK_DATA_DIR` 环境变量自定义数据目录。

## 开发指南

### 项目结构

```
src/
├── main.rs           # CLI 入口（错误处理、退出码）
├── lib.rs            # 库入口
├── server.rs         # MCP 服务器（请求分发、工具注册）
├── store.rs          # SQLite 持久化层（sqlx 异步驱动）
├── core/             # 纯业务逻辑层（CLI 和 MCP 共享）
│   ├── mod.rs        #   模块入口与公共接口
│   ├── task.rs       #   任务级操作
│   ├── item.rs       #   清单条目操作
│   ├── note.rs       #   笔记操作
│   └── resource.rs   #   资源操作
├── models/           # 数据模型
│   └── task.rs       #   Task, ChecklistItem, Resource, TaskMetadata
├── protocol/         # MCP JSON-RPC 协议类型
│   └── types.rs      #   Request/Response/ToolDefinition 等
├── transport/        # stdio 传输层
│   └── mod.rs
├── tools/            # MCP 工具适配层
│   └── task.rs       #   参数解析 → core 调用 → 结果封装
└── cli/              # CLI 命令处理
    ├── mod.rs        #   顶层参数解析与子命令分发
    ├── task.rs       #   new / ls / show
    ├── item.rs       #   add / check / mv / summary
    ├── note.rs       #   note
    ├── meta.rs       #   tag
    ├── resource.rs   #   link
    ├── output.rs     #   统一输出格式化
    ├── resolve.rs    #   短 ID 前缀匹配
    ├── logging.rs    #   日志初始化
    └── server.rs     #   serve 子命令
```

### 构建 & 测试

```bash
cargo build
cargo test                # 运行全部单元测试
cargo test -- --nocapture # 显示 println! 输出
```

### 架构概览

```
用户 → CLI (clap) ──┐
                     ├──→ core (纯逻辑) ──→ store (SQLite + sqlx)
AI Agent → MCP ──────┘
```

所有业务逻辑集中在 `core/` 层，CLI 和 MCP 工具分别作为适配层调用 core 函数，保证行为一致性。

### 退出码

| 退出码 | 含义                       |
| ------ | -------------------------- |
| 0      | 成功                       |
| 1      | 一般错误                   |
| 2      | 任务未找到                 |
| 3      | IO / 数据库错误 / 配置错误 |

## 许可证

MIT License
