//! 短 ID 前缀匹配。
//!
//! 允许用户输入 UUID 前 4 位以上的前缀，自动匹配唯一任务。
//! 0 匹配报错"未找到"，多匹配报错并列出候选。

use crate::models::task::Task;

/// 最少前缀长度。
const MIN_PREFIX_LEN: usize = 4;

/// 根据前缀匹配任务 ID。
///
/// - 前缀长度不足 4 位时报错提示。
/// - 唯一匹配时返回完整 UUID。
/// - 0 匹配报错"未找到"。
/// - 多匹配报错并列出候选列表。
pub fn resolve_task_id(prefix: &str, tasks: &[Task]) -> anyhow::Result<String> {
    if prefix.len() < MIN_PREFIX_LEN {
        anyhow::bail!(
            "ID 前缀至少需要 {MIN_PREFIX_LEN} 位，当前输入: \"{prefix}\"（{} 位）",
            prefix.len()
        );
    }

    let matches: Vec<&Task> = tasks.iter().filter(|t| t.id.starts_with(prefix)).collect();

    match matches.len() {
        0 => {
            anyhow::bail!("未找到匹配的任务: {prefix}")
        }
        1 => Ok(matches[0].id.clone()),
        n => {
            let candidates: Vec<String> = matches
                .iter()
                .map(|t| {
                    let short_id = &t.id[..8.min(t.id.len())];
                    let desc_len = 40.min(t.task_description.len());
                    let desc = &t.task_description[..desc_len];
                    format!("  {short_id}  {desc}")
                })
                .collect();
            anyhow::bail!(
                "前缀 \"{prefix}\" 匹配到 {n} 个任务，请多输入几位以消除歧义:\n{}",
                candidates.join("\n")
            )
        }
    }
}
