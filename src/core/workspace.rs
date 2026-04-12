//! 工作区发现：从当前目录向上搜索 `.pinchproject` 文件并解析项目 ID。

use std::path::{Path, PathBuf};

/// `.pinchproject` 文件名常量。
const PINPROJECT_FILE: &str = ".pinchproject";

/// 从当前工作目录向上搜索 `.pinchproject` 文件，返回找到的项目 ID (UUID)。
///
/// 搜索策略：
/// - 从 CWD 开始，逐级向上（parent）
/// - 在每个目录中查找 `.pinchproject` 文件（必须是文件，不能是目录）
/// - 找到后解析内容，返回第一个有效 UUID
/// - 到达文件系统根目录（parent == self）后停止
/// - 如果设置了 `HOME` 环境变量，也在 HOME 目录处停止
/// - 返回最近（最靠近 CWD）的那个
pub fn discover_project_id() -> Option<String> {
    let path = discover_pinproject_path()?;
    parse_pinproject_file(&path)
}

/// 返回找到的 `.pinchproject` 文件的完整路径。
///
/// 搜索逻辑与 [`discover_project_id`] 相同，但返回路径而非解析后的 UUID。
pub fn discover_pinproject_path() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let home = std::env::var("HOME").ok().map(PathBuf::from);

    let mut dir = cwd.as_path();
    loop {
        let candidate = dir.join(PINPROJECT_FILE);
        if candidate.is_file() {
            return Some(candidate);
        }

        let parent = dir.parent();
        // 到达文件系统根目录：parent 为 None 或 parent == self
        if parent.is_none() || parent == Some(dir) {
            break;
        }

        // 到达 HOME 目录后停止，不再继续向上搜索
        if let Some(ref home_path) = home
            && parent == Some(home_path.as_path())
        {
            // 检查 HOME 目录本身
            let home_candidate = home_path.join(PINPROJECT_FILE);
            if home_candidate.is_file() {
                return Some(home_candidate);
            }
            break;
        }

        dir = parent.unwrap();
    }

    None
}

/// 解析 `.pinchproject` 文件内容，提取第一个有效 UUID。
///
/// 规则：
/// - `#` 开头为注释行
/// - 空行忽略
/// - 每行 trim 后尝试匹配 UUID 格式（8-4-4-4-12 十六进制）
/// - 取第一个匹配的 UUID
fn parse_pinproject_file(path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        // 跳过空行和注释
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if is_valid_uuid(trimmed) {
            return Some(trimmed.to_owned());
        }
    }
    tracing::warn!(
        path = %path.display(),
        "found .pinchproject file but no valid UUID in it"
    );
    None
}

/// 验证字符串是否为有效的 UUID 格式（宽松验证，只检查格式，不检查版本位）。
///
/// 接受 `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` 格式：
/// - 5 段，用 `-` 分隔
/// - 段长度分别为 8, 4, 4, 4, 12
/// - 每段只包含十六进制字符（0-9, a-f, A-F）
fn is_valid_uuid(s: &str) -> bool {
    let segments: Vec<&str> = s.split('-').collect();
    if segments.len() != 5 {
        return false;
    }
    let expected_lengths = [8, 4, 4, 4, 12];
    for (segment, &expected_len) in segments.iter().zip(expected_lengths.iter()) {
        if segment.len() != expected_len {
            return false;
        }
        if !segment.chars().all(|c| c.is_ascii_hexdigit()) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_is_valid_uuid() {
        assert!(is_valid_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(is_valid_uuid("00000000-0000-0000-0000-000000000000"));
        assert!(is_valid_uuid("FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF"));
        assert!(!is_valid_uuid("not-a-uuid"));
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716"));
        assert!(!is_valid_uuid(""));
        assert!(!is_valid_uuid("550e8400-e29b-41d4-a716-44665544000g")); // 包含非hex字符
        assert!(!is_valid_uuid("550e8400_e29b_41d4_a716_446655440000")); // 下划线
    }

    #[test]
    fn test_parse_pinproject_file_simple() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(PINPROJECT_FILE);
        fs::write(&path, "550e8400-e29b-41d4-a716-446655440000\n").unwrap();
        assert_eq!(
            parse_pinproject_file(&path),
            Some("550e8400-e29b-41d4-a716-446655440000".to_string())
        );
    }

    #[test]
    fn test_parse_pinproject_file_with_comments() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(PINPROJECT_FILE);
        fs::write(
            &path,
            "# This is a comment\n550e8400-e29b-41d4-a716-446655440000\n# Another comment\n",
        )
        .unwrap();
        assert_eq!(
            parse_pinproject_file(&path),
            Some("550e8400-e29b-41d4-a716-446655440000".to_string())
        );
    }

    #[test]
    fn test_parse_pinproject_file_empty() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(PINPROJECT_FILE);
        fs::write(&path, "").unwrap();
        assert_eq!(parse_pinproject_file(&path), None);
    }

    #[test]
    fn test_parse_pinproject_file_only_comments() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(PINPROJECT_FILE);
        fs::write(&path, "# comment 1\n# comment 2\n").unwrap();
        assert_eq!(parse_pinproject_file(&path), None);
    }

    #[test]
    fn test_parse_pinproject_file_invalid_uuid() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(PINPROJECT_FILE);
        fs::write(&path, "not-a-uuid\n").unwrap();
        assert_eq!(parse_pinproject_file(&path), None);
    }

    #[test]
    fn test_parse_pinproject_file_multiple_uuids() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(PINPROJECT_FILE);
        fs::write(
            &path,
            "11111111-1111-1111-1111-111111111111\n22222222-2222-2222-2222-222222222222\n",
        )
        .unwrap();
        // 应返回第一个
        assert_eq!(
            parse_pinproject_file(&path),
            Some("11111111-1111-1111-1111-111111111111".to_string())
        );
    }

    #[test]
    fn test_parse_pinproject_file_with_whitespace() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(PINPROJECT_FILE);
        fs::write(&path, "  550e8400-e29b-41d4-a716-446655440000  \n").unwrap();
        assert_eq!(
            parse_pinproject_file(&path),
            Some("550e8400-e29b-41d4-a716-446655440000".to_string())
        );
    }

    #[test]
    fn test_parse_nonexistent_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(PINPROJECT_FILE);
        assert_eq!(parse_pinproject_file(&path), None);
    }

    #[test]
    fn test_pinproject_is_directory() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(PINPROJECT_FILE);
        fs::create_dir(&path).unwrap();
        // parse_pinproject_file 应该对目录返回 None
        assert_eq!(parse_pinproject_file(&path), None);
    }
}
