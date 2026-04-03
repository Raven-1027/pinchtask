# CLI 参考文档

`mcp-pinchtask` 命令行工具的完整命令与参数说明。

---

## 全局选项

以下选项可用于所有子命令：

| 选项 | 环境变量 | 说明 |
|------|----------|------|
| `-D, --data-dir <DIR>` | `PINCHTASK_DATA_DIR` | 数据存储目录路径，默认 `~/.mcp-pinchtask` |
| `--log-level <LEVEL>` | `PINCHTASK_LOG_LEVEL` | 日志级别：`trace`、`debug`、`info`、`warn`、`error` |
| `-v, --verbose` | — | 详细输出，等价于 `--log-level debug`，与 `-q` 互斥 |
| `-q, --quiet` | — | 安静模式，等价于 `--log-level error`，与 `-v` 互斥 |
| `--json` | — | 以 JSON 格式输出（适用于查询类子命令） |

不带子命令运行时，自动进入 MCP 服务器模式（等同于 `serve`）。

---

## 短 ID 匹配

所有接受 `<TASK_ID>` 参数的命令均支持**短前缀匹配**：只需输入 UUID 的前若干位（建议 ≥ 4 位），系统会自动定位唯一匹配的任务。若前缀匹配到多个任务，将报错提示。

```
# 假设完整 ID 为 550e8400-e29b-41d4-a716-446655440000
mcp-pinchtask show 550e
```

---

## 命令列表

### `new` — 创建新任务

```
mcp-pinchtask new <DESCRIPTION> [-c <CONTEXT>]
```

| 参数 | 说明 |
|------|------|
| `<DESCRIPTION>` | 任务描述（必填，位置参数） |
| `-c, --context <TEXT>` | 共享上下文信息，所有子任务可读取 |

**示例：**

```bash
mcp-pinchtask new "实现用户登录功能" -c "使用 JWT 认证方案"
```

---

### `ls` — 列出任务

```
mcp-pinchtask ls [-a] [-d] [-l] [-n <N>] [--sort <FIELD>]
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
mcp-pinchtask ls --all --long --limit 20 --sort priority
```

---

### `show` — 查看任务详情

```
mcp-pinchtask show <TASK_ID>
```

| 参数 | 说明 |
|------|------|
| `<TASK_ID>` | 任务 ID（支持短前缀） |

输出完整的任务信息，包括描述、共享上下文、所有清单条目及其状态、笔记、资源和元数据。

**示例：**

```bash
mcp-pinchtask show 550e
```

---

### `edit` — 编辑任务或清单条目

此命令根据是否传入 `--index` 参数，在**任务级编辑**和**条目级编辑**之间分流。

#### 任务级编辑

```
mcp-pinchtask edit <TASK_ID> [-d <TEXT>] [-c <TEXT>] [--priority <P>] [--tags <TAGS>] [--eta <TIME>]
```

| 选项 | 说明 |
|------|------|
| `<TASK_ID>` | 任务 ID（支持短前缀） |
| `-d, --description <TEXT>` | 新的任务描述 |
| `-c, --context <TEXT>` | 新的共享上下文 |
| `--priority <P>` | 优先级：`high` / `medium` / `low` |
| `--tags <TAGS>` | 标签，逗号分隔（如 `"bug,urgent"`） |
| `--eta <TIME>` | 预计完成时间，ISO 8601 格式 |

至少需要指定一个可修改字段，否则报错。

#### 条目级编辑

```
mcp-pinchtask edit <TASK_ID> <INDEX> [-t <TITLE>] [-d <TEXT>] [-p <PLAN>] [--done] [--undone]
```

| 选项 | 说明 |
|------|------|
| `<TASK_ID>` | 任务 ID（支持短前缀） |
| `<INDEX>` | 清单条目索引（0 起始） |
| `-t, --title <TITLE>` | 新标题 |
| `-d, --description <TEXT>` | 新描述 |
| `-p, --plan <PLAN>` | 新计划 |
| `--done` | 标记为已完成 |
| `--undone` | 标记为未完成 |

> `--done` 与 `--undone` 互斥。`--priority` / `--tags` / `--eta` 不能与 `<INDEX>` 同时使用。

**示例：**

```bash
# 修改任务描述和优先级
mcp-pinchtask edit 550e -d "更新后的描述" --priority high

# 修改第 2 个清单条目的标题并标记完成
mcp-pinchtask edit 550e 1 -t "新标题" --done
```

---

### `rm` — 删除任务或清单条目

#### 删除任务

```
mcp-pinchtask rm <TASK_ID>
```

#### 删除清单条目

```
mcp-pinchtask rm <TASK_ID> <INDEX>
```

| 参数 | 说明 |
|------|------|
| `<TASK_ID>` | 任务 ID（支持短前缀） |
| `<INDEX>` | 清单条目索引（0 起始），不传则删除整个任务 |

**示例：**

```bash
# 删除整个任务
mcp-pinchtask rm 550e

# 删除任务中的第 1 个清单条目
mcp-pinchtask rm 550e 0
```

---

### `add` — 添加清单条目

```
mcp-pinchtask add <TASK_ID> <TITLE> [-d <DESC>] [-p <PLAN>]
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
mcp-pinchtask add 550e "设计数据库表结构" -d "users 表和 sessions 表" -p "先完成 ER 图"
```

---

### `check` — 切换清单条目完成状态

```
mcp-pinchtask check <TASK_ID> <INDEX>
```

| 参数 | 说明 |
|------|------|
| `<TASK_ID>` | 任务 ID（支持短前缀） |
| `<INDEX>` | 清单条目索引（0 起始） |

此命令为**切换操作**：已完成 → 未完成，未完成 → 已完成。

**示例：**

```bash
# 将第 1 个条目标记为已完成
mcp-pinchtask check 550e 0

# 再次执行将标记回未完成
mcp-pinchtask check 550e 0
```

---

### `mv` — 移动清单条目顺序

```
mcp-pinchtask mv <TASK_ID> <FROM> <TO>
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
mcp-pinchtask mv 550e 2 0
```

---

### `summary` — 查看清单进度摘要

```
mcp-pinchtask summary <TASK_ID>
```

| 参数 | 说明 |
|------|------|
| `<TASK_ID>` | 任务 ID（支持短前缀） |

输出清单条目的总数、已完成数和完成百分比。

**示例：**

```bash
mcp-pinchtask summary 550e
```

---

### `note` — 添加笔记

```
mcp-pinchtask note <TASK_ID> <CONTENT>
```

| 参数 | 说明 |
|------|------|
| `<TASK_ID>` | 任务 ID（支持短前缀） |
| `<CONTENT>` | 笔记内容（自由格式文本） |

笔记追加到任务的笔记列表中，不影响已有笔记。

**示例：**

```bash
mcp-pinchtask note 550e "发现 JWT 库版本与项目不兼容，需要升级"
```

---

### `tag` — 设置标签和元数据

```
mcp-pinchtask tag <TASK_ID> [TAGS] [--priority <P>] [--eta <TIME>]
```

| 参数 | 说明 |
|------|------|
| `<TASK_ID>` | 任务 ID（支持短前缀） |
| `[TAGS]` | 标签，逗号分隔（可选，不传则保留现有标签） |
| `--priority <P>` | 优先级：`high` / `medium` / `low` |
| `--eta <TIME>` | 预计完成时间，ISO 8601 格式 |

所有选项均为可选，只更新传入的字段。

**示例：**

```bash
# 设置标签和优先级
mcp-pinchtask tag 550e "bug,urgent" --priority high

# 仅更新预计完成时间
mcp-pinchtask tag 550e --eta "2025-01-15T18:00:00"
```

---

### `link` — 添加资源引用

```
mcp-pinchtask link <TASK_ID> --name <NAME> --url <URL> [-d <DESC>]
```

| 参数 | 说明 |
|------|------|
| `<TASK_ID>` | 任务 ID（支持短前缀） |
| `--name <NAME>` | 资源名称（必填） |
| `--url <URL>` | 资源 URL 或文件路径（必填） |
| `-d, --description <DESC>` | 资源描述（可选） |

**示例：**

```bash
mcp-pinchtask link 550e --name "API 文档" --url "https://example.com/api" -d "REST API 参考文档"
```

---

### `serve` — 启动 MCP 服务器

```
mcp-pinchtask serve
```

启动基于 stdio 传输的 MCP 服务器，供 AI 客户端连接。不带子命令运行 `mcp-pinchtask` 时也会自动进入此模式。

服务器支持的环境变量：

| 环境变量 | 说明 |
|----------|------|
| `PINCHTASK_DATA_DIR` | 数据存储目录路径 |

---

### `completion` — 生成 Shell 补全脚本

```
mcp-pinchtask completion <SHELL>
```

| 参数 | 说明 |
|------|------|
| `<SHELL>` | 目标 shell：`bash`、`zsh`、`fish`、`powershell`、`elvish` |

生成的脚本输出到 stdout，需手动重定向到对应位置。

**示例：**

```bash
# Bash
mcp-pinchtask completion bash > /etc/bash_completion.d/mcp-pinchtask

# Zsh
mcp-pinchtask completion zsh > ~/.zfunc/_mcp-pinchtask

# Fish
mcp-pinchtask completion fish > ~/.config/fish/completions/mcp-pinchtask.fish
```

---

## 退出码

| 退出码 | 含义 |
|--------|------|
| `0` | 成功 |
| `1` | 一般错误（未被分类的错误） |
| `2` | 任务未找到（`NotFound`） |
| `3` | IO 错误 / 数据损坏 / 配置错误 |
