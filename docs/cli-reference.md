# CLI 参考文档

`pinchtask` 命令行工具的完整命令与参数说明。

---

## 全局选项

以下选项可用于所有子命令：

| 选项 | 环境变量 | 说明 |
|------|----------|------|
| `-D, --data-dir <DIR>` | `PINCHTASK_DATA_DIR` | 数据存储目录路径，默认 `~/.pinchtask` |
| `--log-level <LEVEL>` | `PINCHTASK_LOG_LEVEL` | 日志级别：`trace`、`debug`、`info`、`warn`、`error` |
| `-v, --verbose` | — | 详细输出，等价于 `--log-level debug`，与 `-q` 互斥 |
| `-q, --quiet` | — | 安静模式，等价于 `--log-level error`，与 `-v` 互斥 |
| `--json` | — | 以 JSON 格式输出（适用于查询类子命令） |

不带子命令运行时，自动进入 MCP 服务器模式（等同于 `serve`）。

---

## 短 ID 匹配

所有接受 `<ID>`、`<TASK_ID>` 或 `<PROJECT_ID>` 参数的命令均支持**短前缀匹配**：只需输入 UUID 的前若干位（建议 ≥ 4 位），系统会自动定位唯一匹配的对象。若前缀匹配到多个对象，将报错提示。

```bash
# 假设完整 ID 为 550e8400-e29b-41d4-a716-446655440000
pinchtask task show 550e
pinchtask project show 550e
```

---

## 命令列表

### `task` — 任务管理

任务级操作命令组。

#### `task new` — 创建新任务

```
pinchtask task new <DESCRIPTION> [-c <CONTEXT>] [-p <PROJECT>]
```

| 参数 | 说明 |
|------|------|
| `<DESCRIPTION>` | 任务描述（必填，位置参数） |
| `-c, --context <TEXT>` | 共享上下文信息 |
| `-p, --project <ID>` | 关联到指定项目（支持短前缀） |

**示例：**

```bash
pinchtask task new "实现用户登录功能" -c "使用 JWT 认证方案"
pinchtask task new "编写单元测试" -p 550e
```

---

#### `task ls` — 列出任务

```
pinchtask task ls [-a] [-d] [-l] [-n <N>] [--sort <FIELD>]
```

| 选项 | 说明 |
|------|------|
| `-a, --all` | 显示全部任务（进行中 + 已完成） |
| `-d, --done` | 仅显示已完成任务（所有清单条目均完成） |
| `-l, --long` | 详细模式，显示更多列信息 |
| `-n, --limit <N>` | 限制显示数量，默认 `10` |
| `--sort <FIELD>` | 排序字段：`time`（默认，按创建时间）、`priority`（按优先级）、`progress`（按进度） |

> `-a` 与 `-d` 互斥。若两者均不传，默认只显示进行中的任务（清单为空或存在未完成条目）。

**示例：**

```bash
pinchtask task ls --all --long --limit 20 --sort priority
```

---

#### `task show` — 查看任务详情

```
pinchtask task show <ID>
```

| 参数 | 说明 |
|------|------|
| `<ID>` | 任务 ID（支持短前缀） |

输出完整的任务信息，包括描述、共享上下文、所有清单条目及其状态、笔记、资源和元数据。

**示例：**

```bash
pinchtask task show 550e
```

---

#### `task edit` — 编辑任务

```
pinchtask task edit <ID> [-d <TEXT>] [-c <TEXT>] [--priority <P>] [--tags <TAGS>] [--eta <TIME>]
```

| 参数 | 说明 |
|------|------|
| `<ID>` | 任务 ID（支持短前缀） |
| `-d, --description <TEXT>` | 新的任务描述 |
| `-c, --context <TEXT>` | 新的共享上下文 |
| `--priority <P>` | 优先级：`high` / `medium` / `low` |
| `--tags <TAGS>` | 标签，逗号分隔（如 `"bug,urgent"`） |
| `--eta <TIME>` | 预计完成时间，ISO 8601 格式 |

至少需要指定一个可修改字段，否则报错。

**示例：**

```bash
pinchtask task edit 550e -d "更新后的描述" --priority high
pinchtask task edit 550e --tags "bug,urgent" --eta "2025-01-15T18:00:00"
```

---

#### `task rm` — 删除任务

```
pinchtask task rm <ID>
```

| 参数 | 说明 |
|------|------|
| `<ID>` | 任务 ID（支持短前缀） |

删除整个任务及其所有关联数据（清单条目、笔记、资源）。如需仅删除清单条目，请使用 `item rm`。

**示例：**

```bash
pinchtask task rm 550e
```

---

### `item` — 清单条目管理

清单条目级操作命令组。

#### `item new` — 添加清单条目

```
pinchtask item new <TASK_ID> <TITLE> [-d <DESC>] [-p <PLAN>]
```

| 参数 | 说明 |
|------|------|
| `<TASK_ID>` | 任务 ID（支持短前缀） |
| `<TITLE>` | 条目标题（必填） |
| `-d, --description <DESC>` | 详细描述，默认为空 |
| `-p, --plan <PLAN>` | 上下文与计划 |

条目默认追加到清单末尾，状态为未完成。

**示例：**

```bash
pinchtask item new 550e "设计数据库表结构" -d "users 表和 sessions 表" -p "先完成 ER 图"
```

---

#### `item edit` — 编辑清单条目

```
pinchtask item edit <TASK_ID> <INDEX> [-t <TITLE>] [-d <DESC>] [-p <PLAN>] [--done] [--undone]
```

| 参数 | 说明 |
|------|------|
| `<TASK_ID>` | 任务 ID（支持短前缀） |
| `<INDEX>` | 清单条目索引（0 起始） |
| `-t, --title <TITLE>` | 新标题 |
| `-d, --description <DESC>` | 新描述 |
| `-p, --plan <PLAN>` | 新计划 |
| `--done` | 标记为已完成 |
| `--undone` | 标记为未完成 |

> `--done` 与 `--undone` 互斥。

**示例：**

```bash
pinchtask item edit 550e 1 -t "新标题" --done
```

---

#### `item check` — 切换清单条目完成状态

```
pinchtask item check <TASK_ID> <INDEX>
```

| 参数 | 说明 |
|------|------|
| `<TASK_ID>` | 任务 ID（支持短前缀） |
| `<INDEX>` | 清单条目索引（0 起始） |

此命令为**切换操作**：已完成 → 未完成，未完成 → 已完成。

**示例：**

```bash
# 将第 1 个条目标记为已完成
pinchtask item check 550e 0

# 再次执行将标记回未完成
pinchtask item check 550e 0
```

---

#### `item mv` — 移动清单条目顺序

```
pinchtask item mv <TASK_ID> <FROM> <TO>
```

| 参数 | 说明 |
|------|------|
| `<TASK_ID>` | 任务 ID（支持短前缀） |
| `<FROM>` | 源索引（0 起始） |
| `<TO>` | 目标索引（0 起始） |

将索引 `FROM` 处的条目移动到索引 `TO` 的位置，其余条目自动调整顺序。

**示例：**

```bash
# 将第 3 个条目移到第 1 个位置
pinchtask item mv 550e 2 0
```

---

#### `item rm` — 删除清单条目

```
pinchtask item rm <TASK_ID> <INDEX>
```

| 参数 | 说明 |
|------|------|
| `<TASK_ID>` | 任务 ID（支持短前缀） |
| `<INDEX>` | 清单条目索引（0 起始） |

仅删除指定条目，不影响任务本身。

**示例：**

```bash
pinchtask item rm 550e 0
```

---

#### `item summary` — 查看清单进度摘要

```
pinchtask item summary <TASK_ID>
```

| 参数 | 说明 |
|------|------|
| `<TASK_ID>` | 任务 ID（支持短前缀） |

输出清单条目的总数、已完成数和完成百分比。

**示例：**

```bash
pinchtask item summary 550e
```

---

### `note` — 笔记管理

#### `note new` — 添加笔记

```
pinchtask note new <TASK_ID> <CONTENT>
```

| 参数 | 说明 |
|------|------|
| `<TASK_ID>` | 任务 ID（支持短前缀） |
| `<CONTENT>` | 笔记内容（自由格式文本） |

笔记追加到任务的笔记列表中，不影响已有笔记。

**示例：**

```bash
pinchtask note new 550e "发现 JWT 库版本与项目不兼容，需要升级"
```

---

### `link` — 资源引用管理

#### `link new` — 添加资源引用

```
pinchtask link new <TASK_ID> --name <NAME> --url <URL> [-d <DESC>]
```

| 参数 | 说明 |
|------|------|
| `<TASK_ID>` | 任务 ID（支持短前缀） |
| `--name <NAME>` | 资源名称（必填） |
| `--url <URL>` | 资源 URL 或文件路径（必填） |
| `-d, --description <DESC>` | 资源描述（可选） |

**示例：**

```bash
pinchtask link new 550e --name "API 文档" --url "https://example.com/api" -d "REST API 参考文档"
```

---

### `project` — 项目管理

项目级操作命令组。每个任务最多属于一个项目，删除项目时任务自动解除关联。

#### `project new` — 创建项目

```
pinchtask project new <NAME> [-d <DESC>]
```

| 参数 | 说明 |
|------|------|
| `<NAME>` | 项目名称（必填） |
| `-d, --description <DESC>` | 项目描述 |

**示例：**

```bash
pinchtask project new "后端重构" -d "Q2 季度后端架构升级项目"
```

---

#### `project ls` — 列出项目

```
pinchtask project ls
```

无参数。显示所有项目及其基本信息。

**示例：**

```bash
pinchtask project ls
```

---

#### `project show` — 查看项目详情

```
pinchtask project show <ID>
```

| 参数 | 说明 |
|------|------|
| `<ID>` | 项目 ID（支持短前缀） |

显示项目详情及关联的任务列表。

**示例：**

```bash
pinchtask project show 550e
```

---

#### `project rm` — 删除项目

```
pinchtask project rm <ID> [--with-tasks]
```

| 参数 | 说明 |
|------|------|
| `<ID>` | 项目 ID（支持短前缀） |
| `--with-tasks` | 同时删除关联的所有任务 |

默认情况下仅删除项目，关联的任务保留并自动解除关联。使用 `--with-tasks` 可同时删除所有关联任务。

**示例：**

```bash
# 仅删除项目，任务保留
pinchtask project rm 550e

# 删除项目及其所有任务
pinchtask project rm 550e --with-tasks
```

---

#### `project add-task` — 将任务添加到项目

```
pinchtask project add-task <PROJECT_ID> <TASK_ID>
```

| 参数 | 说明 |
|------|------|
| `<PROJECT_ID>` | 项目 ID（支持短前缀） |
| `<TASK_ID>` | 任务 ID（支持短前缀） |

将任务关联到指定项目。若任务已属于其他项目，将自动从原项目移除。

**示例：**

```bash
pinchtask project add-task 550e 660f
```

---

#### `project rm-task` — 将任务从项目移除

```
pinchtask project rm-task <PROJECT_ID> <TASK_ID>
```

| 参数 | 说明 |
|------|------|
| `<PROJECT_ID>` | 项目 ID（支持短前缀） |
| `<TASK_ID>` | 任务 ID（支持短前缀） |

将任务从项目中移除，任务本身不删除。

**示例：**

```bash
pinchtask project rm-task 550e 660f
```

---

### `serve` — 启动 MCP 服务器

```
pinchtask serve
```

启动基于 stdio 传输的 MCP 服务器，供 AI 客户端连接。不带子命令运行 `pinchtask` 时也会自动进入此模式。

服务器支持的环境变量：

| 环境变量 | 说明 |
|----------|------|
| `PINCHTASK_DATA_DIR` | 数据存储目录路径 |

---

### `tui` — 启动交互式 TUI 界面

```
pinchtask tui [-D <DIR>]
```

启动基于终端的交互式界面，使用键盘快捷键管理任务。

| 选项 | 说明 |
|------|------|
| `-D, --data-dir <DIR>` | 数据存储目录，默认 `~/.pinchtask` |

终端最小尺寸要求：80 × 24 字符。

#### 视图

| 视图 | 说明 |
|------|------|
| 任务列表 | 显示所有任务，支持搜索和排序 |
| 任务详情 | 查看清单条目、笔记、资源等完整信息 |
| 任务表单 | 创建/编辑任务（Tab 切换字段，Enter 提交） |
| 帮助面板 | 显示所有快捷键 |

#### 快捷键

**任务列表：**

| 快捷键 | 功能 |
|--------|------|
| `j` / `↓` | 下移 |
| `k` / `↑` | 上移 |
| `Enter` | 查看任务详情 |
| `n` | 新建任务 |
| `d` | 删除任务（需确认） |
| `Tab` | 切换排序方式 |
| `/` | 进入搜索模式 |
| `r` / `Ctrl+R` | 刷新列表 |
| `Home` / `End` | 跳到列表首/末 |
| `?` | 显示帮助 |
| `q` | 退出 |

**任务详情：**

| 快捷键 | 功能 |
|--------|------|
| `j` / `k` / `↑` / `↓` | 移动焦点 |
| `Space` / `x` | 切换条目完成状态 |
| `a` | 添加清单条目 |
| `e` | 编辑条目名称 |
| `d` | 删除当前条目 |
| `Ctrl+J` / `Ctrl+K` | 下移/上移条目顺序 |
| `N` (Shift+n) | 添加笔记 |
| `D` (Shift+d) | 删除笔记 |
| `L` (Shift+l) | 添加资源链接 |
| `E` (Shift+e) | 编辑任务 |
| `Esc` | 返回列表 |

**全局：**

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+C` | 强制退出 |
| `Esc` | 取消/返回上一视图 |

**示例：**

```bash
# 启动 TUI
pinchtask tui

# 指定数据目录
pinchtask tui -D /path/to/data
```

---

### `completion` — 生成 Shell 补全脚本

```
pinchtask completion <SHELL>
```

| 参数 | 说明 |
|------|------|
| `<SHELL>` | 目标 shell：`bash`、`zsh`、`fish`、`powershell`、`elvish` |

生成的脚本输出到 stdout，需手动重定向到对应位置。

**示例：**

```bash
# Bash
pinchtask completion bash > /etc/bash_completion.d/pinchtask

# Zsh
pinchtask completion zsh > ~/.zfunc/_pinchtask

# Fish
pinchtask completion fish > ~/.config/fish/completions/pinchtask.fish
```

---

## 退出码

| 退出码 | 含义 |
|--------|------|
| `0` | 成功 |
| `1` | 一般错误（未被分类的错误） |
| `2` | 任务/项目未找到 |
| `3` | IO 错误 / 数据库错误 / 配置错误 |
