# MCP 协议文档

## 概述

mcp-pinchtask 实现了 [Model Context Protocol](https://modelcontextprotocol.io/) 服务器，通过 JSON-RPC 2.0 over Stdio 与客户端通信。

- **协议版本**：`2024-11-05`
- **传输方式**：Stdio（标准输入/输出）
- **服务端信息**：`name: "mcp-pinchtask"`, `version: <cargo version>`

## 生命周期

```
1. 客户端 → initialize          → 服务端返回 capabilities + serverInfo
2. 客户端 → notifications/initialized → （无响应，通知）
3. 客户端 → tools/list          → 服务端返回所有工具定义
4. 客户端 → tools/call          → 服务端执行工具并返回结果
5. 客户端 → ping                → 服务端返回空对象
```

## 错误码

| 错误码 | 含义 |
|--------|------|
| `-32601` | 方法未找到 / 工具未找到 |
| `-32602` | 参数无效或缺失 |

业务层错误通过 `CallToolResult.isError = true` 返回，包含在 `content[0].text` 中描述具体原因（如"任务不存在"、"清单条目索引越界"等）。

## 工具总览

共注册 **17 个工具**：

| 工具名 | 类别 | 说明 |
|--------|------|------|
| `initialize_task` | 任务 | 创建新任务 |
| `update_task` | 任务 | 统一更新任务字段 |
| `update_task_description` | 任务 | 更新任务描述 |
| `update_context` | 任务 | 更新共享上下文 |
| `update_metadata` | 任务 | 更新元数据 |
| `clear_task` | 任务 | 删除任务 |
| `get_checklist_summary` | 查询 | 获取清单概要 |
| `list_tasks` | 查询 | 列出所有任务 |
| `get_current_task_details` | 查询 | 获取当前子任务详情 |
| `add_checklist_item` | 清单 | 添加清单条目 |
| `update_checklist_item` | 清单 | 更新清单条目 |
| `mark_task_done` | 清单 | 标记完成 |
| `mark_task_undone` | 清单 | 标记未完成 |
| `reorder_checklist_item` | 清单 | 重排条目顺序 |
| `remove_checklist_item` | 清单 | 删除条目 |
| `add_note` | 笔记 | 添加笔记 |
| `add_resource` | 资源 | 添加资源引用 |

---

## 工具详细说明

### initialize_task

创建一个新任务。

**参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_description` | string | ✅ | 任务整体描述 |
| `context_for_all_tasks` | string | ❌ | 所有子任务共享的上下文信息 |
| `initial_checklist` | array | ❌ | 初始清单条目列表 |
| `initial_checklist[].task` | string | ✅* | 条目短名称 |
| `initial_checklist[].detailed_description` | string | ✅* | 条目详细描述 |
| `initial_checklist[].context_and_plan` | string | ❌ | 上下文与计划 |
| `initial_checklist[].done` | boolean | ❌ | 是否已完成（默认 false） |
| `notes` | string[] | ❌ | 初始笔记列表 |
| `resources` | array | ❌ | 初始资源列表 |
| `resources[].name` | string | ✅* | 资源名称 |
| `resources[].url` | string | ✅* | 资源 URL |
| `resources[].description` | string | ❌ | 资源描述 |
| `metadata` | object | ❌ | 元数据 |
| `metadata.tags` | string[] | ❌ | 标签 |
| `metadata.priority` | string | ❌ | 优先级 (`"high"` / `"medium"` / `"low"`) |
| `metadata.estimated_completion_time` | string | ❌ | 预计完成时间 |

**返回**：完整 Task 对象的 JSON 字符串。

---

### update_task

一次性更新任务的多个字段（描述、上下文、优先级、标签、预计时间）。仅修改指定字段。

**参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_id` | string | ✅ | 任务 ID |
| `task_description` | string | ❌ | 新的任务描述 |
| `context_for_all_tasks` | string | ❌ | 新的共享上下文 |
| `priority` | string | ❌ | 优先级 (`"high"` / `"medium"` / `"low"`) |
| `tags` | string | ❌ | 逗号分隔的标签 |
| `eta` | string | ❌ | 预计完成时间 |

> 至少需要指定一个可修改字段。

**返回**：更新后的完整 Task 对象。

---

### update_task_description

更新任务的描述。

**参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_id` | string | ✅ | 任务 ID |
| `task_description` | string | ✅ | 新的描述 |

**返回**：更新后的完整 Task 对象。

---

### update_context

更新所有子任务的共享上下文。

**参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_id` | string | ✅ | 任务 ID |
| `context_for_all_tasks` | string | ✅ | 新的上下文信息 |

**返回**：更新后的完整 Task 对象。

---

### add_checklist_item

向任务清单添加一个新条目。

**参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_id` | string | ✅ | 任务 ID |
| `task` | string | ✅ | 条目短名称 |
| `detailed_description` | string | ✅ | 详细描述 |
| `context_and_plan` | string | ❌ | 上下文与计划 |

**返回**：更新后的完整 Task 对象。

---

### update_checklist_item

更新已有的清单条目。仅修改指定字段。

**参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_id` | string | ✅ | 任务 ID |
| `index` | integer | ✅ | 条目索引（0-based, ≥ 0） |
| `task` | string | ❌ | 新短名称 |
| `detailed_description` | string | ❌ | 新详细描述 |
| `context_and_plan` | string | ❌ | 新上下文与计划（传 null 清空） |
| `done` | boolean | ❌ | 完成状态 |

> `context_and_plan` 不传与传 `null` 语义不同：不传 = 保持原值，传 `null` = 清空。

**返回**：更新后的完整 Task 对象。

---

### mark_task_done

将指定清单条目标记为已完成。

**参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_id` | string | ✅ | 任务 ID |
| `index` | integer | ✅ | 条目索引（0-based, ≥ 0） |

**返回**：更新后的完整 Task 对象。

---

### mark_task_undone

将指定清单条目标记为未完成。

**参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_id` | string | ✅ | 任务 ID |
| `index` | integer | ✅ | 条目索引（0-based, ≥ 0） |

**返回**：更新后的完整 Task 对象。

---

### reorder_checklist_item

移动清单条目到新位置。

**参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_id` | string | ✅ | 任务 ID |
| `from_index` | integer | ✅ | 当前索引（0-based, ≥ 0） |
| `to_index` | integer | ✅ | 目标索引（0-based, ≥ 0） |

**返回**：更新后的完整 Task 对象。

---

### remove_checklist_item

删除指定索引的清单条目。

**参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_id` | string | ✅ | 任务 ID |
| `index` | integer | ✅ | 条目索引（0-based, ≥ 0） |

**返回**：更新后的完整 Task 对象。

---

### add_note

向任务添加一条笔记。

**参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_id` | string | ✅ | 任务 ID |
| `content` | string | ✅ | 笔记内容 |

**返回**：更新后的完整 Task 对象。

---

### add_resource

向任务添加一个资源引用。

**参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_id` | string | ✅ | 任务 ID |
| `name` | string | ✅ | 资源名称 |
| `url` | string | ✅ | 资源 URL 或文件路径 |
| `description` | string | ❌ | 资源描述 |

**返回**：更新后的完整 Task 对象。

---

### update_metadata

更新任务元数据（标签、优先级、预计完成时间）。

**参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_id` | string | ✅ | 任务 ID |
| `metadata` | object | ✅ | 元数据对象 |
| `metadata.tags` | string[] | ❌ | 标签列表 |
| `metadata.priority` | string | ❌ | 优先级 (`"high"` / `"medium"` / `"low"`) |
| `metadata.estimated_completion_time` | string | ❌ | 预计完成时间 |

**返回**：更新后的完整 Task 对象。

---

### get_checklist_summary

获取任务清单概要（含完成状态统计）。

**参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_id` | string | ✅ | 任务 ID |
| `include_descriptions` | boolean | ❌ | 是否包含详细描述（默认 false） |

**返回**：文本格式的清单概要。

示例输出：
```
任务: 实现用户认证
进度: 2/5

✅ [0] 设计数据模型
✅ [1] 实现注册接口
⬜ [2] 实现登录接口
⬜ [3] JWT 中间件
⬜ [4] 编写测试
```

---

### clear_task

根据 ID 删除任务。

**参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `task_id` | string | ✅ | 要删除的任务 ID |

**返回**：文本确认消息。

---

### list_tasks

列出所有任务（按创建时间升序）。

**参数**：无。

**返回**：文本格式的任务列表摘要。

示例输出：
```
ID: a1b2c3d4-...
任务: 实现用户认证
进度: 2/5
创建时间: 2024-01-15T10:30:00+00:00

ID: e5f6g7h8-...
任务: 数据库迁移
进度: 0/3
创建时间: 2024-01-16T09:00:00+00:00
```

---

### get_current_task_details

获取第一个包含未完成子任务的任务详情。

**参数**：无。

**返回**：文本格式的当前任务详情，包含任务上下文和第一个未完成子任务的完整信息。

示例输出：
```
任务: 实现用户认证
共享上下文: 使用 JWT 认证方案

当前子任务 (索引 2):
  名称: 实现登录接口
  详细描述: 实现 POST /api/login 端点
  上下文与计划: 参考 OAuth2 规范
  状态: 进行中
```

---

## 通用返回格式

### 成功响应

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [
      { "type": "text", "text": "<Task JSON 或摘要文本>" }
    ],
    "isError": false
  }
}
```

### 错误响应

**JSON-RPC 层错误**（参数解析失败、工具未找到）：

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32602,
    "message": "Invalid params: Missing required parameter: task_id"
  }
}
```

**业务层错误**（任务不存在、索引越界）：

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [
      { "type": "text", "text": "任务不存在: abc-123" }
    ],
    "isError": true
  }
}
```

---

## 配置示例

### Claude Desktop

编辑 `~/Library/Application Support/Claude/claude_desktop_config.json`（macOS）或对应的配置文件：

```json
{
  "mcpServers": {
    "pinchtask": {
      "command": "/path/to/mcp-pinchtask",
      "args": ["mcp"]
    }
  }
}
```

### 自定义数据目录

通过环境变量设置（如需要指定非默认的存储路径，需修改代码中的 `default_data_dir()`）。默认数据目录为 `~/.mcp-pinchtask/`。
