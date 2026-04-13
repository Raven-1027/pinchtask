[English](../en/mcp.md) | 中文

# MCP 协议文档

pinchtask 实现了 Model Context Protocol (MCP) 服务器，允许 AI Agent 通过标准化接口管理任务。

## 概述

pinchtask MCP 服务器通过 stdio 传输层与 AI 客户端通信，提供 9 个工具用于任务的完整生命周期管理：创建、查看、编辑、删除任务及其子资源（清单条目、笔记、资源引用），以及项目管理。

- **协议版本**: `2024-11-05`
- **服务器名称**: `pinchtask`
- **传输方式**: stdio（标准输入/输出）

## 启动方式

```bash
pinchtask serve
```

不带任何子命令直接运行 `pinchtask` 也会进入服务器模式：

```bash
pinchtask
```

可通过环境变量或 `-D` 参数指定数据目录：

```bash
PINCHTASK_DATA_DIR=/path/to/data pinchtask serve
pinchtask serve -D /path/to/data
```

## 传输协议

服务器通过 stdio 与客户端交换 JSON-RPC 2.0 消息，支持两种输入格式：

1. **换行分隔 JSON** — 每条消息占一行，以 `\n` 结尾
2. **Content-Length 头格式** — 每条消息前附带 `Content-Length: <字节数>\r\n\r\n` 头

服务器自动检测输入格式并统一以换行分隔 JSON 输出。

## 配置示例

### Claude Desktop

在 Claude Desktop 配置文件中添加：

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

### Cursor

在 Cursor 的 MCP 设置中添加：

```json
{
  "mcpServers": {
    "pinchtask": {
      "command": "pinchtask",
      "args": ["serve"]
    }
  }
}
```

### 通用配置

任何支持 MCP stdio 传输的客户端均可使用以下配置：

```json
{
  "command": "pinchtask",
  "args": ["serve"],
  "env": {
    "PINCHTASK_DATA_DIR": "~/.pinchtask"
  }
}
```

## 短 ID 前缀匹配

所有接受 `task_id` 或 `project_id` 参数的工具都支持 UUID 短前缀匹配。你可以只输入 UUID 的前 4 位以上字符，系统会自动匹配唯一的结果。

- 前缀长度不足 4 位时会报错。
- 唯一匹配时自动解析为完整 UUID。
- 无匹配时报错"未找到"。
- 多个匹配时返回前 10 个候选任务/项目列表，提示输入更多字符以消除歧义。

## 工作区项目关联（.pinchproject）

在项目根目录放置 `.pinchproject` 文件（内容为项目 UUID）可实现工作区自动关联。

**MCP 层**：`new_task` 和 `list_tasks` 不再自动注入 workspace project_id，`project_id` 必须显式传入。

**CLI 层**：`task new` 和 `task ls` 在未指定 `--project` 时仍支持自动使用 `.pinchproject` 中的项目 ID。详见 CLI 文档。

## 工具列表

### new_task

创建新任务，支持同时设置初始清单、笔记、资源和元数据。可通过 `project_id` 参数将任务关联到已有项目。

**参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_description` | string | 是 | 任务描述 |
| `context_for_all_tasks` | string | 否 | 所有清单条目共享的上下文信息（如技术栈、约束条件） |
| `initial_checklist` | array | 否 | 初始清单条目列表 |
| `notes` | array of string | 否 | 初始笔记列表 |
| `resources` | array | 否 | 初始资源引用列表 |
| `metadata` | object | 否 | 任务元数据 |
| `project_id` | string | 是 | 关联的项目 ID（支持短 ID 前缀匹配） |

**`initial_checklist` 条目结构：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task` | string | 是 | 条目简短名称 |
| `detailed_description` | string | 是 | 详细描述 |
| `context_and_plan` | string | 否 | 上下文与计划 |
| `done` | boolean | 否 | 是否已完成（默认 `false`） |
| `id` | string | 否 | 预设 ID（省略则自动生成 UUID） |

**`resources` 条目结构：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `name` | string | 是 | 资源名称 |
| `url` | string | 是 | 资源 URL 或文件路径 |
| `description` | string | 否 | 资源描述 |

**`metadata` 结构：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `tags` | array of string | 否 | 标签列表 |
| `priority` | string | 否 | 优先级：`high` / `medium` / `low` |
| `estimated_completion_time` | string | 否 | 预计完成时间（ISO 时间戳或时长描述） |

**示例：**

```json
{
  "task_description": "实现用户登录功能",
  "context_for_all_tasks": "使用 Rust + Axum 框架，JWT 认证方案",
  "initial_checklist": [
    {
      "task": "设计数据库表结构",
      "detailed_description": "创建 users 和 sessions 表",
      "context_and_plan": "参考 ER 图进行设计"
    },
    {
      "task": "实现登录接口",
      "detailed_description": "POST /api/login",
      "done": false
    }
  ],
  "metadata": {
    "priority": "high",
    "tags": ["后端", "认证"]
  },
  "project_id": "a1b2c3d4-..."
}
```

---

### update_task

更新任务的描述、上下文、元数据或项目关联字段。仅修改指定的字段，未指定的字段保持不变。至少需要指定一个可修改字段。

**参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_id` | string | 是 | 任务 ID（支持短 ID 前缀匹配） |
| `task_description` | string | 否 | 新的任务描述 |
| `context_for_all_tasks` | string | 否 | 新的共享上下文 |
| `priority` | string | 否 | 优先级：`high` / `medium` / `low` |
| `tags` | string | 否 | 逗号分隔的标签 |
| `eta` | string | 否 | 预计完成时间（ISO 时间戳或时长描述） |
| `project_id` | string or null | 否 | 项目 ID（支持短 ID 前缀匹配）。传入项目 UUID 将任务关联到该项目，传入 `null` 解除关联，不传则不变 |

**示例：**

```json
{
  "task_id": "a1b2c3d4-...",
  "priority": "high",
  "tags": "后端,认证,紧急"
}
```

**关联项目示例：**

```json
{
  "task_id": "a1b2c3d4-...",
  "project_id": "e5f6a7b8-..."
}
```

**解除项目关联示例：**

```json
{
  "task_id": "a1b2c3d4-...",
  "project_id": null
}
```

---

### manage_checklist_item

统一管理清单条目，支持添加、更新、重排序和删除操作。所有索引从 0 开始。

**参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `action` | string | 是 | 操作类型：`add` / `update` / `reorder` / `remove` / `batch_update` |
| `task_id` | string | 是 | 任务 ID（支持短 ID 前缀匹配） |
| `task` | string | `add` 时必填 | 条目简短名称 |
| `detailed_description` | string | `add` 时必填 | 详细描述 |
| `index` | number | `update` / `remove` 时必填 | 条目索引（0-based） |
| `from_index` | number | `reorder` 时必填 | 源索引（0-based） |
| `to_index` | number | `reorder` 时必填 | 目标索引（0-based） |
| `context_and_plan` | string or null | 否 | 上下文与计划。不传则不修改；传 `null` 则清空；传字符串则更新 |
| `done` | boolean | 否 | 是否完成（仅 `update` 时有效） |
| `updates` | array | `batch_update` 时必填 | 要批量更新的条目列表 |

**各操作说明：**

- **`add`** — 在清单末尾追加新条目，需提供 `task` 和 `detailed_description`
- **`update`** — 修改已有条目，需提供 `index`，仅修改指定字段
- **`reorder`** — 移动条目位置，需提供 `from_index` 和 `to_index`。重排序后索引会变化，后续操作前需刷新任务数据
- **`remove`** — 删除条目，需提供 `index`。删除后后续条目索引前移 1 位
- **`batch_update`** — 在单次请求中更新多个条目。需提供 `updates` 数组，每个元素指定 `index` 和要修改的字段。条目按顺序依次更新。适用于批量标记完成等场景

**添加条目示例：**

```json
{
  "action": "add",
  "task_id": "a1b2c3d4-...",
  "task": "编写单元测试",
  "detailed_description": "覆盖登录接口的正常和异常场景",
  "context_and_plan": "使用 mock 测试"
}
```

**标记完成示例：**

```json
{
  "action": "update",
  "task_id": "a1b2c3d4-...",
  "index": 0,
  "done": true
}
```

**重排序示例：**

```json
{
  "action": "reorder",
  "task_id": "a1b2c3d4-...",
  "from_index": 2,
  "to_index": 0
}
```

**批量更新示例：**

```json
{
  "action": "batch_update",
  "task_id": "a1b2c3d4-...",
  "updates": [
    {"index": 0, "done": true},
    {"index": 1, "done": true},
    {"index": 2, "done": true}
  ]
}
```

**`updates` 条目结构：**

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `index` | number | 是 | 条目索引（0-based） |
| `task` | string | 否 | 新标题 |
| `detailed_description` | string | 否 | 新描述 |
| `context_and_plan` | string or null | 否 | 新上下文与计划（`null` 清空） |
| `done` | boolean | 否 | 是否完成 |

---

### add_note

向任务追加一条笔记。笔记仅支持追加，不可编辑或删除。

**参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_id` | string | 是 | 任务 ID（支持短 ID 前缀匹配） |
| `content` | string | 是 | 笔记内容 |

**示例：**

```json
{
  "task_id": "a1b2c3d4-...",
  "content": "经过讨论决定使用 bcrypt 替代 argon2 进行密码哈希"
}
```

---

### add_resource

向任务追加一条资源引用。资源仅支持追加，不可编辑或删除。

**参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_id` | string | 是 | 任务 ID（支持短 ID 前缀匹配） |
| `name` | string | 是 | 资源名称 |
| `url` | string | 是 | 资源 URL 或文件路径 |
| `description` | string | 否 | 资源描述 |

**示例：**

```json
{
  "task_id": "a1b2c3d4-...",
  "name": "JWT 规范文档",
  "url": "https://jwt.io/introduction",
  "description": "JSON Web Token 官方介绍"
}
```

---

### get_checklist_summary

获取任务清单的进度摘要。

**参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_id` | string | 是 | 任务 ID（支持短 ID 前缀匹配） |
| `include_descriptions` | boolean | 否 | 是否包含详细描述（默认 `false`） |

**示例：**

```json
{
  "task_id": "a1b2c3d4-...",
  "include_descriptions": true
}
```

---

### clear_task

删除指定任务。此操作不可逆。

**参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_id` | string | 是 | 要删除的任务 ID（支持短 ID 前缀匹配） |

**示例：**

```json
{
  "task_id": "a1b2c3d4-..."
}
```

---

### list_tasks

列出任务并按状态分组返回。返回格式为：进行中 → 未开始 → 已完成，组内按优先级（high > medium > low）排序。当任务总数超过 10 时，未开始和已完成组自动截断，仅展示前 3 条并附统计摘要。可通过 `status_filter` 过滤指定状态组，或通过 `include_all` 强制展示全部任务。

**参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `project_id` | string | 是 | 项目 ID（支持短 ID 前缀匹配），返回该项目的任务。传入 `"*"` 表示跨所有项目查询 |
| `status_filter` | string | 否 | 按任务状态过滤：`"in_progress"` / `"not_started"` / `"completed"`。不传则显示全部状态 |
| `include_all` | boolean | 否 | 为 `true` 时跳过截断逻辑（默认总数 >10 时截断未开始/已完成组），强制展示全部任务 |

**示例 — 查询指定项目：**

```json
{
  "project_id": "e5f6a7b8-..."
}
```

**示例 — 跨所有项目查询：**

```json
{
  "project_id": "*"
}
```

---

### manage_project

统一管理项目，支持创建、查看、更新、删除和列表操作。每个任务最多属于一个项目。

典型工作流：先使用 `list` 查看现有项目，然后通过 `new_task`（指定 `project_id`）在新项目下创建任务，或通过 `update_task`（指定 `project_id`）将已有任务分配到项目。

**参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `action` | string | 是 | 操作类型：`create` / `get` / `update` / `delete` / `list` |
| `project_id` | string | `get` / `update` / `delete` 时必填 | 项目 ID（支持短 ID 前缀匹配） |
| `name` | string | `create` 时必填，`update` 时可选 | 项目名称 |
| `description` | string | 否 | 项目描述 |
| `delete_tasks` | boolean | 否 | 删除项目时是否同时删除关联任务（仅 `delete` 时有效，默认 `false`） |

**各操作说明：**

- **`create`** — 创建新项目，需提供 `name`，`description` 可选
- **`get`** — 获取项目详情，需提供 `project_id`
- **`update`** — 更新项目名称和/或描述，需提供 `project_id`
- **`delete`** — 删除项目。`delete_tasks` 为 `true` 时同时删除所有关联任务；为 `false` 或不传时保留关联任务（自动解除关联）
- **`list`** — 列出所有项目，无需额外参数

**创建项目示例：**

```json
{
  "action": "create",
  "name": "用户系统重构",
  "description": "重构认证和授权模块"
}
```

**删除项目示例（保留任务）：**

```json
{
  "action": "delete",
  "project_id": "e5f6a7b8-..."
}
```

**删除项目示例（同时删除任务）：**

```json
{
  "action": "delete",
  "project_id": "e5f6a7b8-...",
  "delete_tasks": true
}
```
