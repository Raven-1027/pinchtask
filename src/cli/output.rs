//! 输出格式化工具。

use anyhow::Result;
use serde::Serialize;

use crate::models::task::Task;

/// 根据输出模式输出结果。
pub fn output<T: Serialize>(data: &T, json: bool) -> Result<()> {
    if json {
        let json_str = serde_json::to_string_pretty(data)?;
        println!("{json_str}");
    }
    Ok(())
}

/// 格式化任务摘要（人类可读）。
pub fn format_task_summary(task: &Task) -> String {
    let total = task.checklist.len();
    let done = task.checklist.iter().filter(|i| i.done).count();

    let mut s = String::new();
    s.push_str(&format!("ID: {}\n", task.id));
    s.push_str(&format!("任务: {}\n", task.task_description));
    if let Some(ref ctx) = task.context_for_all_tasks {
        s.push_str(&format!("上下文: {ctx}\n"));
    }
    s.push_str(&format!("进度: {done}/{total}\n"));
    s.push_str(&format!("创建时间: {}\n", task.created_at));
    s.push_str(&format!("更新时间: {}\n", task.updated_at));
    if let Some(ref meta) = task.metadata {
        if let Some(ref tags) = meta.tags {
            if !tags.is_empty() {
                s.push_str(&format!("标签: {}\n", tags.join(", ")));
            }
        }
        if let Some(ref priority) = meta.priority {
            s.push_str(&format!("优先级: {priority}\n"));
        }
    }
    s
}

/// 格式化任务详情（人类可读，含清单、笔记、资源）。
pub fn format_task_detail(task: &Task) -> String {
    let mut s = format_task_summary(task);
    s.push('\n');

    // 清单
    if !task.checklist.is_empty() {
        s.push_str("清单:\n");
        for (i, item) in task.checklist.iter().enumerate() {
            let status = if item.done { "✅" } else { "⬜" };
            s.push_str(&format!("  {status} [{i}] {}\n", item.task));
            if !item.detailed_description.is_empty() {
                s.push_str(&format!("       {}\n", item.detailed_description));
            }
            if let Some(ref plan) = item.context_and_plan {
                s.push_str(&format!("       计划: {plan}\n"));
            }
        }
        s.push('\n');
    }

    // 笔记
    if !task.notes.is_empty() {
        s.push_str("笔记:\n");
        for (i, note) in task.notes.iter().enumerate() {
            s.push_str(&format!("  {}. {note}\n", i + 1));
        }
        s.push('\n');
    }

    // 资源
    if !task.resources.is_empty() {
        s.push_str("资源:\n");
        for res in &task.resources {
            s.push_str(&format!("  - {} ({})", res.name, res.url));
            if let Some(ref desc) = res.description {
                s.push_str(&format!(": {desc}"));
            }
            s.push('\n');
        }
        s.push('\n');
    }

    s
}

/// 格式化清单进度摘要。
pub fn format_checklist_summary(task: &Task) -> String {
    let total = task.checklist.len();
    let done = task.checklist.iter().filter(|i| i.done).count();
    let mut s = format!("任务: {}\n进度: {done}/{total}\n\n", task.task_description);
    for (i, item) in task.checklist.iter().enumerate() {
        let status = if item.done { "✅" } else { "⬜" };
        s.push_str(&format!("{status} [{i}] {}\n", item.task));
    }
    s
}
