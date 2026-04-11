[English](../en/cli.md) | 中文

# CLI 使用指南

pinchtask 提供完整的命令行界面，支持任务的创建、查询、编辑、删除，以及清单条目、笔记、资源引用和项目的管理。

## 全局选项

以下选项可用于所有子命令：

| 选项 | 说明 |
|------|------|
| `-D, --data-dir <DIR>` | 数据存储目录（默认: `~/.pinchtask`），也可通过 `PINCHTASK_DATA_DIR` 环境变量设置 |
| `--log-level <LEVEL>` | 日志级别：`trace`、`debug`、`info`、`warn`、`error`，也可通过 `PINCHTASK_LOG_LEVEL` 环境变量设置 |
| `-v, --verbose` | 详细输出（等价于 `--log-level debug`），与 `--quiet` 互斥 |
| `-q, --quiet` | 安静模式（等价于 `--log-level error`），与 `--verbose` 互斥 |
| `--json` | 以 JSON 格式输出（适用于查询类命令） |

## 短 ID 匹配

所有需要任务 ID 或项目 ID 的命令都支持短前缀匹配：输入 UUID 的前 4 位及以上即可定位目标。当前缀匹配到多个结果时，会提示歧义并列出候选项。

```bash
# 假设任务完整 ID 为 a1b2c3d4-e5f6-7890-abcd-ef1234567890
pinchtask task show a1b2c3d4
```

## 命令一览

```
pinchtask [OPTIONS] [COMMAND]

命令:
  task        任务管理
  item        清单条目管理
  note        笔记管理
  link        资源引用管理
  project     项目管理
  serve       启动 MCP 服务器
  tui         启动交互式 TUI 界面（需 tui feature）
  completion  生成 shell 补全脚本
```

---

## task — 任务管理

### `task new` — 创建新任务

```bash
pinchtask task new <描述> [OPTIONS]
```

| 参数 | 说明 |
|------|------|
| `<描述>` | 任务描述（必填，位置参数） |
| `-c, --context <文本>` | 共享上下文信息 |
| `-p, --project <项目ID>` | 关联到指定项目 |

示例：

```bash
# 创建简单任务
pinchtask task new "实现用户登录功能"

# 创建带上下文和项目关联的任务
pinchtask task new "实现用户登录功能" -c "使用 JWT 认证方案" -p a1b2c3d4
```

### `task ls` — 列出任务

```bash
pinchtask task ls [OPTIONS]
```

| 参数 | 说明 |
|------|------|
| `--all` | 显示全部任务（active + done），与 `--done` 互斥 |
| `--done` | 只显示已完成任务（所有清单条目均已完成），与 `--all` 互斥 |
| `-l, --long` | 详细模式（显示更多列） |
| `-n, --limit <数量>` | 限制显示数量（默认: 10） |
| `--sort <字段>` | 排序字段：`time`（默认）、`priority`、`progress` |
| `-p, --project <项目ID>` | 按项目筛选，只显示该项目下的任务（支持短前缀） |

默认只显示未完成任务（无清单条目，或存在未完成条目的任务）。

示例：

```bash
# 列出最近 10 个未完成任务
pinchtask task ls

# 列出所有任务，详细模式
pinchtask task ls --all --long

# 按优先级排序
pinchtask task ls --sort priority

# 按进度排序
pinchtask task ls --sort progress

# 只显示已完成任务
pinchtask task ls --done

# 按项目筛选
pinchtask task ls -p b1c2d3e4

# 按项目筛选并显示全部
pinchtask task ls --all -p b1c2d3e4
```

### `task show` — 查看任务详情

```bash
pinchtask task show <ID>
```

| 参数 | 说明 |
|------|------|
| `<ID>` | 任务 ID（支持短前缀） |

示例：

```bash
pinchtask task show a1b2c3d4
```

### `task edit` — 编辑任务

```bash
pinchtask task edit <ID> [OPTIONS]
```

| 参数 | 说明 |
|------|------|
| `<ID>` | 任务 ID（支持短前缀） |
| `-d, --description <文本>` | 新的任务描述 |
| `-c, --context <文本>` | 新的共享上下文 |
| `--priority <级别>` | 优先级：`high`、`medium`、`low` |
| `--tags <标签>` | 标签，逗号分隔 |
| `--eta <时间>` | 预计完成时间，ISO 8601 格式 |

至少需要指定一个可修改的字段。编辑后输出更新后的完整任务信息。

示例：

```bash
# 修改描述
pinchtask task edit a1b2c3d4 -d "实现 OAuth2 登录流程"

# 设置优先级和标签
pinchtask task edit a1b2c3d4 --priority high --tags "前端,认证"

# 设置预计完成时间
pinchtask task edit a1b2c3d4 --eta "2025-02-01T18:00:00+08:00"
```

### `task rm` — 删除任务

```bash
pinchtask task rm <ID>
```

| 参数 | 说明 |
|------|------|
| `<ID>` | 任务 ID（支持短前缀） |

此操作不可撤销。

示例：

```bash
pinchtask task rm a1b2c3d4
```

---

## item — 清单条目管理

清单条目使用 **0-based 索引**（第一条目索引为 0）。

### `item new` — 添加清单条目

```bash
pinchtask item new <任务ID> <标题> [OPTIONS]
```

| 参数 | 说明 |
|------|------|
| `<任务ID>` | 任务 ID（支持短前缀） |
| `<标题>` | 条目标题 |
| `-d, --description <文本>` | 详细描述（默认: 空字符串） |
| `-p, --plan <文本>` | 上下文与计划 |

示例：

```bash
pinchtask item new a1b2c3d4 "设计数据库表结构" -d "users 表和 sessions 表" -p "参考 ER 图"
```

### `item edit` — 编辑清单条目

```bash
pinchtask item edit <任务ID> <索引> [OPTIONS]
```

| 参数 | 说明 |
|------|------|
| `<任务ID>` | 任务 ID（支持短前缀） |
| `<索引>` | 条目索引（0-based） |
| `-t, --title <文本>` | 新标题 |
| `-d, --description <文本>` | 新描述 |
| `-p, --plan <文本>` | 新计划 |
| `--done` | 标记为已完成（与 `--undone` 互斥） |
| `--undone` | 标记为未完成（与 `--done` 互斥） |

示例：

```bash
# 修改标题
pinchtask item edit a1b2c3d4 0 -t "设计用户表和会话表"

# 标记完成
pinchtask item edit a1b2c3d4 0 --done
```

### `item check` — 切换条目完成状态

```bash
pinchtask item check <任务ID> <索引>
```

| 参数 | 说明 |
|------|------|
| `<任务ID>` | 任务 ID（支持短前缀） |
| `<索引>` | 条目索引（0-based） |

切换条目的完成/未完成状态。如果当前已完成则变为未完成，反之亦然。

示例：

```bash
pinchtask item check a1b2c3d4 0
```

### `item mv` — 移动条目顺序

```bash
pinchtask item mv <任务ID> <源索引> <目标索引>
```

| 参数 | 说明 |
|------|------|
| `<任务ID>` | 任务 ID（支持短前缀） |
| `<源索引>` | 当前位置（0-based） |
| `<目标索引>` | 目标位置（0-based） |

移动后索引会发生变化，后续操作前请先刷新任务数据。

示例：

```bash
# 将第 3 条移到第 1 条的位置
pinchtask item mv a1b2c3d4 2 0
```

### `item rm` — 删除清单条目

```bash
pinchtask item rm <任务ID> <索引>
```

| 参数 | 说明 |
|------|------|
| `<任务ID>` | 任务 ID（支持短前缀） |
| `<索引>` | 条目索引（0-based） |

删除后后续条目的索引会前移 1 位。

示例：

```bash
pinchtask item rm a1b2c3d4 1
```

### `item summary` — 查看清单进度摘要

```bash
pinchtask item summary <任务ID>
```

| 参数 | 说明 |
|------|------|
| `<任务ID>` | 任务 ID（支持短前缀） |

示例：

```bash
pinchtask item summary a1b2c3d4
```

---

## note — 笔记管理

### `note new` — 添加笔记

```bash
pinchtask note new <任务ID> <内容>
```

| 参数 | 说明 |
|------|------|
| `<任务ID>` | 任务 ID（支持短前缀） |
| `<内容>` | 笔记内容 |

笔记为追加式，不可修改或删除。

示例：

```bash
pinchtask note new a1b2c3d4 "决定使用 bcrypt 而非 scrypt 进行密码哈希"
```

---

## link — 资源引用管理

### `link new` — 添加资源引用

```bash
pinchtask link new <任务ID> --name <名称> --url <URL> [OPTIONS]
```

| 参数 | 说明 |
|------|------|
| `<任务ID>` | 任务 ID（支持短前缀） |
| `--name <名称>` | 资源名称（必填） |
| `--url <URL>` | 资源 URL 或文件路径（必填） |
| `-d, --description <文本>` | 资源描述 |

资源引用为追加式，不可修改或删除。

示例：

```bash
# 关联文档链接
pinchtask link new a1b2c3d4 --name "JWT 规范" --url "https://jwt.io/introduction" -d "JSON Web Token 官方介绍"

# 关联本地文件
pinchtask link new a1b2c3d4 --name "数据库设计稿" --url "/path/to/schema.sql"
```

---

## project — 项目管理

项目与任务为一对多关系：每个任务最多属于一个项目，删除项目时任务自动解除关联（除非使用 `--with-tasks`）。

### `project new` — 创建项目

```bash
pinchtask project new <名称> [OPTIONS]
```

| 参数 | 说明 |
|------|------|
| `<名称>` | 项目名称 |
| `-d, --description <文本>` | 项目描述 |

示例：

```bash
pinchtask project new "用户系统重构" -d "将认证模块从 session 迁移到 JWT"
```

### `project ls` — 列出所有项目

```bash
pinchtask project ls
```

示例：

```bash
pinchtask project ls
```

### `project show` — 查看项目详情

```bash
pinchtask project show <ID>
```

| 参数 | 说明 |
|------|------|
| `<ID>` | 项目 ID（支持短前缀） |

显示项目信息及其关联的任务列表。

示例：

```bash
pinchtask project show b1c2d3e4
```

### `project rm` — 删除项目

```bash
pinchtask project rm <ID> [OPTIONS]
```

| 参数 | 说明 |
|------|------|
| `<ID>` | 项目 ID（支持短前缀） |
| `--with-tasks` | 同时删除项目关联的所有任务 |

默认只删除项目本身，关联任务保留但解除项目关联。

示例：

```bash
# 只删除项目，保留任务
pinchtask project rm b1c2d3e4

# 删除项目及其所有关联任务
pinchtask project rm b1c2d3e4 --with-tasks
```

### `project add-task` — 将任务添加到项目

```bash
pinchtask project add-task <项目ID> <任务ID>
```

| 参数 | 说明 |
|------|------|
| `<项目ID>` | 项目 ID（支持短前缀） |
| `<任务ID>` | 任务 ID（支持短前缀） |

如果任务已属于其他项目，会自动转移。

示例：

```bash
pinchtask project add-task b1c2d3e4 a1b2c3d4
```

### `project rm-task` — 将任务从项目中移除

```bash
pinchtask project rm-task <项目ID> <任务ID>
```

| 参数 | 说明 |
|------|------|
| `<项目ID>` | 项目 ID（支持短前缀） |
| `<任务ID>` | 任务 ID（支持短前缀） |

移除后任务仍存在，只是不再属于任何项目。

示例：

```bash
pinchtask project rm-task b1c2d3e4 a1b2c3d4
```

---

## serve — 启动 MCP 服务器

```bash
pinchtask serve
```

通过 stdio 启动 MCP 服务器，供 AI Agent 调用。不带子命令直接运行 `pinchtask` 也会进入服务器模式。

详细配置说明请参考 [MCP 协议文档](mcp.md)。

---

## tui — 启动交互式 TUI

```bash
pinchtask tui
```

启动基于终端的交互式界面。需要编译时启用 `tui` feature：

```bash
cargo run --features tui -- tui
```

详细使用说明请参考 [TUI 使用文档](tui.md)。

---

## completion — 生成 Shell 补全脚本

```bash
pinchtask completion <SHELL>
```

| 参数 | 说明 |
|------|------|
| `<SHELL>` | 目标 shell：`bash`、`zsh`、`fish`、`powershell`、`elvish` |

示例：

```bash
# Bash
pinchtask completion bash > /etc/bash_completion.d/pinchtask

# Zsh
pinchtask completion zsh > ~/.zsh/completions/_pinchtask

# Fish
pinchtask completion fish > ~/.config/fish/completions/pinchtask.fish
```

---

## JSON 输出

所有查询类命令都支持 `--json` 全局选项，输出机器可读的 JSON 格式：

```bash
pinchtask task ls --json
pinchtask task show a1b2c3d4 --json
pinchtask item summary a1b2c3d4 --json
pinchtask project ls --json
pinchtask project show b1c2d3e4 --json
```

---

## 退出码

| 退出码 | 含义 |
|--------|------|
| 0 | 成功 |
| 1 | 一般错误（参数错误、校验失败等） |
| 2 | 任务或项目未找到 |
| 3 | 数据库或配置错误 |
