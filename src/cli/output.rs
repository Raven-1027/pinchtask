//! 统一输出格式化。
//!
//! 所有 CLI 子命令的输出都通过 `Output` 枚举 + `print()` 函数统一处理，
//! 支持 `--json` 全局 flag 切换 JSON 序列化输出。

use serde::Serialize;

use crate::models::task::Task;

// ---------------------------------------------------------------------------
// Output 枚举
// ---------------------------------------------------------------------------

/// CLI 命令输出内容的统一封装。
#[allow(clippy::large_enum_variant)]
pub enum Output<'a> {
    /// 单个任务完整详情（new / show / edit 返回）。
    Task(&'a Task),
    /// 任务列表（ls 返回）。
    TaskList {
        tasks: Vec<TaskListEntry>,
        long: bool,
    },
    /// 清单进度摘要（summary 返回）。
    ChecklistSummary(String),
    /// 通用成功消息（check / note / tag / link 返回）。
    Success(String),
    /// 任务已删除（rm / rm-item 返回）。
    Deleted(String),
}

/// 任务列表中的一行条目。
#[derive(Serialize)]
pub struct TaskListEntry {
    /// 短 ID（前 8 位）。
    pub short_id: String,
    /// 优先级（-l 时使用）。
    pub priority: Option<String>,
    /// 任务描述。
    pub description: String,
    /// 进度条字符串。
    pub progress_bar: String,
    /// 完成数。
    pub done: usize,
    /// 总数。
    pub total: usize,
    /// 是否全部完成。
    pub is_done: bool,
    /// 标签（-l 时使用）。
    pub tags: Option<Vec<String>>,
    /// 创建日期（-l 时使用）。
    pub created_date: String,
}

// ---------------------------------------------------------------------------
// print 函数
// ---------------------------------------------------------------------------

/// 根据 Output 类型和 json flag 输出结果。
pub fn print(output: Output, json: bool) {
    if json {
        print_json(&output);
    } else {
        print_text(&output);
    }
}

/// JSON 格式输出。
///
/// 所有分支的序列化目标都是 `Serialize` 类型（`Task`、`Vec<TaskListEntry>`、
/// `serde_json::Value`），它们在序列化阶段不可能失败，因此使用 `expect()` 而非
/// `unwrap()` 以便在极端情况下给出可诊断的 panic 消息。
fn print_json(output: &Output) {
    let json_str = match output {
        Output::Task(task) => serde_json::to_string_pretty(task).expect("序列化 Task 不应失败"),
        Output::TaskList { tasks, .. } => {
            serde_json::to_string_pretty(tasks).expect("序列化 TaskList 不应失败")
        }
        Output::ChecklistSummary(s) => {
            let obj = serde_json::json!({ "summary": s });
            serde_json::to_string_pretty(&obj).expect("序列化 JSON value 不应失败")
        }
        Output::Success(msg) => {
            let obj = serde_json::json!({ "success": true, "message": msg });
            serde_json::to_string_pretty(&obj).expect("序列化 JSON value 不应失败")
        }
        Output::Deleted(msg) => {
            let obj = serde_json::json!({ "deleted": true, "message": msg });
            serde_json::to_string_pretty(&obj).expect("序列化 JSON value 不应失败")
        }
    };
    println!("{json_str}");
}

/// 文本格式输出。
fn print_text(output: &Output) {
    match output {
        Output::Task(task) => {
            print_task_detail(task);
        }
        Output::TaskList { tasks, long } => {
            if tasks.is_empty() {
                println!("当前没有任何任务");
                return;
            }
            for entry in tasks {
                print_task_list_entry(entry, *long);
            }
        }
        Output::ChecklistSummary(s) => {
            println!("{s}");
        }
        Output::Success(msg) => {
            println!("{msg}");
        }
        Output::Deleted(msg) => {
            println!("{msg}");
        }
    }
}

// ---------------------------------------------------------------------------
// 格式化辅助
// ---------------------------------------------------------------------------

/// 生成 10 字符宽的进度条。
///
/// 比例映射到 10 格: `█████░░░░░`
pub fn make_progress_bar(done: usize, total: usize) -> String {
    if total == 0 {
        return "░░░░░░░░░░".to_owned();
    }
    let filled = (done * 10 + total / 2) / total; // 四舍五入
    let filled = filled.min(10);
    let empty = 10 - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// 从 Task 构建列表条目。
pub fn task_to_list_entry(task: &Task) -> TaskListEntry {
    let total = task.checklist.len();
    let done = task.checklist.iter().filter(|i| i.done).count();
    let is_done = total > 0 && done == total;

    let short_id = task.id[..8.min(task.id.len())].to_owned();

    let priority = task.metadata.as_ref().and_then(|m| m.priority.clone());

    let tags = task
        .metadata
        .as_ref()
        .and_then(|m| m.tags.clone())
        .filter(|t| !t.is_empty());

    // 从 ISO 8601 时间戳中提取日期部分
    let created_date = task
        .created_at
        .get(..10)
        .unwrap_or(&task.created_at)
        .to_owned();

    TaskListEntry {
        short_id,
        priority,
        description: task.task_description.clone(),
        progress_bar: make_progress_bar(done, total),
        done,
        total,
        is_done,
        tags,
        created_date,
    }
}

/// 打印单个列表条目。
fn print_task_list_entry(entry: &TaskListEntry, long: bool) {
    // 对齐描述到 24 字符宽度（中文字符按 2 列宽计算）
    let desc_width = 24;
    let desc_padded = pad_display(&entry.description, desc_width);

    let bar = &entry.progress_bar;
    let fraction = format!("{}/{}", entry.done, entry.total);

    if long {
        // -l 模式: short_id  priority  description  bar  fraction  tags  date
        let prio = entry.priority.as_deref().unwrap_or("    ");
        let prio_padded = format!("{:<5}", prio);

        let tag_str = entry
            .tags
            .as_ref()
            .map(|t| t.join(","))
            .unwrap_or_else(|| "-".to_owned());

        println!(
            "{} {} {} {} {} {} {}",
            entry.short_id, prio_padded, desc_padded, bar, fraction, tag_str, entry.created_date
        );
    } else {
        // 默认模式: short_id  description  bar  fraction
        let done_marker = if entry.is_done { "[done] " } else { "" };
        println!(
            "{}  {}{} {} {}",
            entry.short_id, done_marker, desc_padded, bar, fraction
        );
    }
}

/// 打印任务详情（show / new / edit 返回）。
fn print_task_detail(task: &Task) {
    let total = task.checklist.len();
    let done = task.checklist.iter().filter(|i| i.done).count();

    println!("ID: {}", task.id);
    println!("任务: {}", task.task_description);
    if let Some(ref ctx) = task.context_for_all_tasks {
        println!("上下文: {ctx}");
    }
    println!("进度: {done}/{total}");

    // 元数据
    if let Some(ref meta) = task.metadata {
        if let Some(ref tags) = meta.tags {
            if !tags.is_empty() {
                println!("标签: {}", tags.join(", "));
            }
        }
        if let Some(ref priority) = meta.priority {
            println!("优先级: {priority}");
        }
        if let Some(ref eta) = meta.estimated_completion_time {
            println!("预计完成: {eta}");
        }
    }

    println!("创建时间: {}", task.created_at);
    println!("更新时间: {}", task.updated_at);

    // 清单
    if !task.checklist.is_empty() {
        println!("\n清单:");
        for (i, item) in task.checklist.iter().enumerate() {
            let status = if item.done { "✅" } else { "⬜" };
            println!("  {status} [{i}] {}", item.task);
            if !item.detailed_description.is_empty() {
                for line in item.detailed_description.lines() {
                    println!("       {}", line);
                }
            }
            if let Some(ref plan) = item.context_and_plan {
                println!("       计划: {plan}");
            }
        }
    }

    // 笔记
    if !task.notes.is_empty() {
        println!("\n笔记:");
        for (i, note) in task.notes.iter().enumerate() {
            println!("  {}. {note}", i + 1);
        }
    }

    // 资源
    if !task.resources.is_empty() {
        println!("\n资源:");
        for res in &task.resources {
            print!("  - {} ({})", res.name, res.url);
            if let Some(ref desc) = res.description {
                print!(": {desc}");
            }
            println!();
        }
    }
}

/// 将字符串填充到指定显示宽度（考虑中文字符双宽度）。
fn pad_display(s: &str, target_width: usize) -> String {
    let current_width = unicode_width(s);
    if current_width >= target_width {
        // 截断到目标宽度
        truncate_to_width(s, target_width)
    } else {
        format!("{}{}", s, " ".repeat(target_width - current_width))
    }
}

/// 计算字符串的 Unicode 显示宽度（CJK 字符算 2 列）。
fn unicode_width(s: &str) -> usize {
    s.chars()
        .map(|c| {
            if c.is_ascii() {
                1
            } else if c > '\u{2E80}' {
                // 粗略判断 CJK 字符为双宽度
                2
            } else {
                1
            }
        })
        .sum()
}

/// 按显示宽度截断字符串。
fn truncate_to_width(s: &str, max_width: usize) -> String {
    let mut width = 0;
    for c in s.chars() {
        let cw = if c.is_ascii() {
            1
        } else if c > '\u{2E80}' {
            2
        } else {
            1
        };
        if width + cw > max_width {
            break;
        }
        width += cw;
    }
    // width 字节位置
    let mut byte_pos = 0;
    let mut w = 0;
    for c in s.chars() {
        let cw = if c.is_ascii() {
            1
        } else if c > '\u{2E80}' {
            2
        } else {
            1
        };
        if w + cw > max_width {
            break;
        }
        w += cw;
        byte_pos += c.len_utf8();
    }
    s[..byte_pos].to_owned()
}
