# CLI 重新设计方案

> 审阅日期：2026-04-02

## 1. 当前设计问题诊断

| # | 问题 | 旧示例 | 影响 |
|---|------|--------|------|
| 1 | 三层嵌套 | `task checklist add <id> "标题"` | 输入冗长，违背 Unix 简洁原则 |
| 2 | task_id 每次必输 | 无当前任务概念，UUID 手抄痛苦 | 交互摩擦大 |
| 3 | 功能碎片化 | `checklist done` / `checklist undone` / `checklist update --done` 三种方式做同一件事 | 用户困惑，API 表面积过大 |
| 4 | update/context 分裂 | `task update -d` 和 `task context` 都是改任务属性 | 逻辑不统一 |
| 5 | 输出不统一 | 六个模块各自处理 JSON/文本输出 | 维护成本高，行为不一致 |
| 6 | 字段命名不一致 | checklist 用 `title` 但模型叫 `task`；metadata 用 `eta` 但模型叫 `estimated_completion_time` | 认知负担 |

## 2. 设计原则

1. **两层结构**：`task` 子命令管任务实体，顶层直接操作任务内部元素（清单、笔记等）
2. **动词优先**：子命令用动词（new/ls/show/edit/rm），不用名词
3. **清单是默认上下文**：最常用的清单操作不需要额外前缀
4. **短 ID 前缀匹配**：输入 UUID 前 6-8 位即可自动匹配，无需完整输入
5. **统一输出**：所有命令走 `output` 模块，`--json` 全局 flag
6. **字段名对齐**：CLI 参数名与内部模型字段名保持一致

## 3. 完整命令树

```
pinchtask
│
├── 任务实体 ──────────────────────────────────────
│   new "描述" [-c "上下文"]                  创建任务
│   ls [-a] [-l] [-d] [-n 10] [--sort time|priority]  列出任务
│   show <id>                                查看任务详情
│   edit <id> [index] [-d "描述"] [-c "上下文"] [-t "标题"] [-p "计划"] [--done|--undone]
│                                            编辑任务（无 index）或清单条目（有 index）
│   rm <id> [index]                          删除任务（无 index）或清单条目（有 index）
│
├── 清单（默认操作对象，提升为顶层）────────────────
│   add <id> "标题" [-d "描述"] [-p "计划"]    添加清单项
│   check <id> <index>                       toggle 完成/未完成
│   mv <id> <from> <to>                     移动清单项
│   summary <id>                            清单进度摘要
│
├── 其他维度 ─────────────────────────────────────
│   note <id> "内容"                         添加笔记
│   tag <id> "tag1,tag2" [--priority high|medium|low] [--eta "ISO时间"]  设置元数据
│   link <id> --name "名称" --url "URL" [-d "描述"]  添加资源引用
│
├── 服务 ──────────────────────────────────────────
│   serve                                    启动 MCP 服务器（原 server）
│   completion <shell>                       生成补全脚本
│
└── 全局选项
    --json          JSON 格式输出
    -D / --data-dir 数据目录路径
    -v / --verbose  详细日志
    -q / --quiet    安静模式
```

> `<id>` 接受 UUID 前缀（最少 4 位），自动匹配唯一任务。

## 5. ls 命令设计（对齐 Unix ls）

| 参数 | 长名 | 默认 | 含义 |
|------|------|------|------|
| *(无)* | *(无)* | active | 只显示活跃任务（有未完成清单项） |
| `-a` | `--all` | | 显示全部（active + done） |
| `-l` | `--long` | | 详细模式，显示更多列 |
| `-d` | `--done` | | 只显示已完成任务 |
| `-n <N>` | `--limit <N>` | 10 | 限制显示数量 |
| | `--sort <field>` | created | 排序：time / priority / progress |

### 输出格式

**默认模式（简洁）**
```
$ pinchtask ls
8c7b04  API 重构              ██████░░░░ 3/8
a1f3c9  文档更新              ████░░░░░░ 2/6
```

**-l 详细模式**
```
$ pinchtask ls -l
8c7b04  high  API 重构              ██████░░░░ 3/8  security,backend  2025-04-01
a1f3c9  low   文档更新              ████░░░░░░ 2/6  docs              2025-03-28
```

**-a 全部（已完成任务灰色显示）**
```
$ pinchtask ls -a
8c7b04  API 重构              ██████░░░░ 3/8
a1f3c9  文档更新              ████░░░░░░ 2/6
f2d8e1  [done] 登录模块开发     ██████████ 5/5
```

### 实现要点

```rust
pub struct ListFilter {
    pub show_all: bool,      // -a
    pub show_done_only: bool, // -d
    pub long_format: bool,    // -l
    pub limit: usize,         // -n
    pub sort_by: SortField,   // --sort
}

// -a 和 -d 互斥，clap 的 conflict_with 处理
// -a 优先级：-a > -d > 默认(active)
```

## 4. 短 ID 前缀匹配

### 设计理由

CLI 工具应保持无状态、可重现。`use <id>`（持久化当前任务）虽然减少输入，但牺牲了脚本可重现性和操作透明度。短 ID 前缀匹配在保持显式状态的同时，将输入量从 36 字符降到 6 字符。

### 匹配规则

```rust
pub fn resolve_task_id(prefix: &str, tasks: &[Task]) -> Result<String> {
    let matches: Vec<_> = tasks.iter()
        .filter(|t| t.id.starts_with(prefix))
        .collect();
    match matches.len() {
        0 => Err(format!("未找到匹配的任务: {}", prefix)),
        1 => Ok(matches[0].id.clone()),
        n => Err(format!("前缀 {} 匹配到 {} 个任务，请多输入几位以消除歧义:", prefix, n)),
    }
}
```

### 使用示例

```bash
# 列出任务后，用前缀操作
pinchtask ls
#  8c7b04a2  高优先级 - API 重构
#  a1f3c9d1  后备   - 文档更新

pinchtask show 8c7b04          # 唯一匹配 8c7b04a2
pinchtask add 8c7b04 "标题"
pinchtask check 8c7b04 0
pinchtask note 8c7b04 "备注"
```

## 6. 新旧对比

### 场景：操作一个任务的多个维度

```bash
# ── 旧设计（6次输入完整 UUID）──
pinchtask checklist add 8c7b04a2-xxxx-xxxx-xxxx-xxxxxxxxxxxx "实现认证模块" -d "JWT方案"
pinchtask checklist done 8c7b04a2-xxxx-xxxx-xxxx-xxxxxxxxxxxx 0
pinchtask checklist add 8c7b04a2-xxxx-xxxx-xxxx-xxxxxxxxxxxx "编写单元测试"
pinchtask note add 8c7b04a2-xxxx-xxxx-xxxx-xxxxxxxxxxxx "参考 OWASP 规范"
pinchtask metadata update 8c7b04a2-xxxx-xxxx-xxxx-xxxxxxxxxxxx --priority high --tags "security,backend"
pinchtask resource add 8c7b04a2-xxxx-xxxx-xxxx-xxxxxxxxxxxx --name "JWT文档" --url "https://jwt.io"

# ── 新设计（6次输入短前缀，状态显式）──
pinchtask add 8c7b04 "实现认证模块" -d "JWT方案"
pinchtask check 8c7b04 0
pinchtask add 8c7b04 "编写单元测试"
pinchtask note 8c7b04 "参考 OWASP 规范"
pinchtask tag 8c7b04 "security,backend" --priority high
pinchtask link 8c7b04 --name "JWT文档" --url "https://jwt.io"
```

### 命令名对照表

| 旧命令 | 新命令 | 变化说明 |
|--------|--------|----------|
| `task create -d "描述"` | `new "描述"` | 提升为顶层，去掉 create |
| `task list` | `ls` | 简写 |
| `task get <id>` | `show <id>` | 更直觉的动词 |
| `task update <id> -d "..."` | `edit <id> -d "..."` | update → edit |
| `task context <id> -c "..."` | `edit <id> -c "..."` | 合并到 edit |
| `task delete <id>` | `rm <id>` | 简写 |
| *(无)* | *(无)* | 去掉 `use`，改为短 ID 匹配 |
| `checklist add <id> "标题"` | `add [id] "标题"` | 去掉 checklist 前缀 |
| `checklist done <id> <i>` | `check [id] <i>` | done → check（toggle 语义） |
| `checklist undone <id> <i>` | `check [id] <i>` | 合并（toggle 自动处理） |
| `checklist update <id> <i> -t "..."` | `edit <id> <i> -t "..."` | 合并到 edit，index 可选 |
| `checklist remove <id> <i>` | `rm <id> <i>` | 合并到 rm，index 可选 |
| `checklist reorder <id> <from> <to>` | `mv [id] <from> <to>` | reorder → mv |
| `checklist summary <id>` | `summary [id]` | 去掉 checklist 前缀 |
| `note add <id> "内容"` | `note [id] "内容"` | 去掉 add |
| `metadata update <id> --tags --priority --eta` | `tag [id] "tags" [--priority] [--eta]` | 合并为 tag |
| `resource add <id> --name --url` | `link [id] --name --url` | resource → link |
| `server` | `serve` | 更简短 |

## 6. 代码结构

```
src/cli/
├── mod.rs          # Cli struct 定义 + run() 入口 + store 初始化 + 全局输出
├── task.rs         # new / ls / show / edit / rm / use
├── item.rs         # add / check / edit-item / mv / rm-item / summary
├── note.rs         # note
├── meta.rs         # tag（原 metadata）
├── resource.rs     # link（原 resource）
├── server.rs       # serve
└── output.rs       # 统一输出：文本表格 + JSON 序列化
├── resolve.rs      # 短 ID 前缀匹配 + 参数校验
```

### output.rs 统一策略

```rust
pub enum Output {
    Task(Task),
    TaskList(Vec<TaskSummary>),
    ChecklistSummary(ChecklistSummary),
    Success(&'static str),
    Deleted,
}

pub fn print(output: Output, json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        match output {
            Output::Task(t) => print_task_table(&t),
            Output::TaskList(list) => print_task_list_table(&list),
            // ...
        }
    }
}
```

### resolve.rs 短 ID 匹配

```rust
pub fn resolve_task_id(prefix: &str, tasks: &[Task]) -> Result<String> {
    let matches: Vec<_> = tasks.iter()
        .filter(|t| t.id.starts_with(prefix))
        .collect();
    match matches.len() {
        0 => Err(format!("未找到匹配的任务: {}", prefix)),
        1 => Ok(matches[0].id.clone()),
        n => Err(format!(
            "前缀 {} 匹配到 {} 个任务，请多输入几位:\n{}",
            prefix, n,
            matches.iter().map(|t| format!("  {} {}", &t.id[..8.min(t.id.len())], &t.description[..40.min(t.description.len())])).collect::<Vec<_>>().join("\n")
        )),
    }
}
```

## 7. 实施计划

1. **重构 Cli struct**：按新命令树重写 clap 定义
2. **实现 resolve.rs**：短 ID 前缀匹配函数
3. **重构 output.rs**：统一 Output enum + print 函数
4. **逐模块迁移**：按 item → note → meta → resource → task → server 顺序
5. **删除旧模块**：移除 checklist.rs、原 metadata.rs 中的碎片命令
6. **更新补全脚本**：重新生成
7. **更新测试**：按新命令树调整集成测试
