# pinchtask

[English](README.md.en) | 中文

> [!WARNING]  
> 几乎所有代码都由 GLM-5-Turbo 生成, 本程序按原样提供 不做任何保障.

为 AI Agent 与人类的协同工作提供结构化的任务管理能力.

支持 **CLI 本地操作**, **TUI** 和 **MCP 服务器** 三种使用模式, 共享同一套核心业务逻辑与数据存储.

## 功能特性

- **任务管理** — 创建、查看、编辑、删除任务，支持描述与共享上下文
- **清单系统** — 每个任务包含有序清单条目，支持增删改查、标记完成/未完成、重排序
- **笔记** — 向任务追加自由格式的文本笔记
- **资源引用** — 关联外部文档、URL 或文件路径
- **项目管理** — 创建、查看、编辑、删除项目，支持任务关联（一对多）
- **元数据** — 标签、优先级（high/medium/low）、预计完成时间
- **短 ID 匹配** — CLI 支持输入 UUID 前 4+ 位即可定位任务
- **双模式运行** — CLI 直接操作 / MCP stdio 服务器供 AI Agent 调用
- **JSON 输出** — 所有查询类命令支持 `--json` 标志输出机器可读格式
- **可选的交互式 TUI** — 基于 ratatui 的终端界面, 支持键盘导航, 实时搜索, 任务管理
- **工作区关联** — 通过 `.pinchproject` 文件自动关联项目，CLI/TUI/MCP 三端支持

## 安装

### 从源码构建

```bash
# 需要 Rust 1.88+ 和 Cargo
git clone https://github.com/Raven-1027/pinchtask.git
cd pinchtask
cargo build --release
# 二进制文件位于 target/release/pinchtask
```

### 使用 Cargo 安装

```bash
cargo install --git https://github.com/Raven-1027/pinchtask
```

## 快速开始

### 创建并管理一个任务

```bash
# 创建任务
pinchtask task new "实现用户登录功能" -c "使用 JWT 认证方案"

# 查看任务列表
pinchtask task ls

# 添加清单条目
pinchtask item add <TASK_ID> "设计数据库表结构" -d "users 表和 sessions 表"

# 标记条目完成
pinchtask item check <TASK_ID> 0

# 查看进度摘要
pinchtask item summary <TASK_ID>

# 查看任务详情
pinchtask task show <TASK_ID>
```

### 启动 MCP 服务器

```bash
# 直接运行（无子命令时自动进入服务器模式）
pinchtask

# 或显式指定
pinchtask serve
```

### 工作区项目关联

使用 `project init` 命令在当前目录创建 `.pinchproject` 文件：

```bash
# 先创建项目
pinchtask project new "我的项目"

# 在项目根目录初始化工作区关联（project_id 支持短前缀匹配）
pinchtask project init <项目ID>

# 如果文件已存在，使用 --force 覆盖
pinchtask project init <项目ID> --force
```

此后在该目录及子目录中执行 `task new`、`task ls` 等命令时，会自动关联到该项目。显式指定 `--project` 始终优先。

## CLI 用法

```
pinchtask [OPTIONS] [COMMAND]

全局选项:
  -D, --data-dir <DIR>      数据存储目录（默认: ~/.pinchtask）
      --log-level <LEVEL>   日志级别 (trace, debug, info, warn, error)
  -v, --verbose             详细输出（等价于 --log-level debug）
  -q, --quiet               安静模式（等价于 --log-level error）
      --json                以 JSON 格式输出

命令:
  task        任务管理 (new, ls, show, edit, rm)
  item        清单条目管理 (add, check, mv, summary, edit, rm)
  note        笔记管理 (add, rm)
  link        资源引用管理 (add, rm)
  project     项目管理 (new, ls, show, edit, rm, add-task, rm-task, init)
  serve       启动 MCP 服务器
  completion  生成 shell 补全脚本
```

> 完整命令参数说明请参考 [CLI 参考文档](docs/zh/cli.md).

## MCP 配置

在支持 MCP 的 AI 客户端 (如 Claude Desktop、Cursor 等) 中配置:

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

> 完整 MCP 工具说明请参考 [MCP 协议文档](docs/zh/mcp.md).

## TUI 交互式界面

通过 `pinchtask tui` 启动基于终端的交互式界面, 使用键盘快捷键管理任务.

### 启动

```bash
pinchtask tui
# 或指定数据目录
pinchtask tui -D /path/to/data
```

如果当前目录或上级目录存在 `.pinchproject` 文件，TUI 启动时会自动选中对应项目。

> 完整快捷键与视图说明请参考 [TUI 使用文档](docs/zh/tui.md).

## 数据存储

任务数据存储在 SQLite 数据库文件中:

```
~/.pinchtask/
└── tasks.db
```

数据库采用混合范式化设计: `tasks` 主表存储任务基本信息, `checklist_items`、`notes`、`resources` 为独立关联表, `metadata` 以 JSON 列存储. 可通过 `-D` 参数或 `PINCHTASK_DATA_DIR` 环境变量自定义数据目录. 任务与项目为一对多关系, 每个任务最多属于一个项目 (通过 `tasks.project_id` 外键关联), 删除项目时任务自动解除关联.

## 开发指南

### 构建 & 测试

```bash
cargo build
cargo test                # 运行全部单元测试
cargo test -- --nocapture # 显示 println! 输出
```

所有业务逻辑集中在 `core/` 层，CLI, TUI 和 MCP 工具分别作为适配层调用 core 函数, 保证行为一致性.

## 许可证

MIT License
