use super::downloads::classify_file_type;
use super::model::LargeFileItem;
use super::protect::{is_inside_managed_runtime, is_sensitive_account_data};
use super::utils::system_time_string;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_ANALYSIS_ENTRIES: usize = 250_000;

fn normalized(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

pub(crate) fn validate_analysis_root(root: &Path) -> Result<(), String> {
    if !root.is_dir() {
        return Err("扫描目录不存在".to_string());
    }
    let value = normalized(root);
    if value.len() <= 3
        || [
            r"c:\windows",
            r"c:\program files",
            r"c:\program files (x86)",
            r"c:\programdata\microsoft",
            r"c:\system volume information",
        ]
        .iter()
        .any(|blocked| value == *blocked || value.starts_with(&format!("{blocked}\\")))
        || is_inside_managed_runtime(root)
    {
        return Err("系统目录、盘符根目录和受管运行时不允许作为分析范围".to_string());
    }
    Ok(())
}

pub(crate) fn collect_large_files(
    root: &Path,
    min_bytes: u64,
    limit: usize,
) -> Result<Vec<LargeFileItem>, String> {
    validate_analysis_root(root)?;
    let mut result = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    let mut visited = 0_usize;
    while let Some(path) = stack.pop() {
        if visited >= MAX_ANALYSIS_ENTRIES {
            break;
        }
        visited += 1;
        if path != root && is_sensitive_account_data(&path) {
            continue;
        }
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_file() {
            if metadata.len() >= min_bytes {
                let file_type = classify_file_type(&path).to_string();
                result.push(LargeFileItem {
                    path: path.to_string_lossy().to_string(),
                    size: metadata.len(),
                    modified_at: metadata.modified().ok().and_then(system_time_string),
                    suggestion: match file_type.as_str() {
                        "安装包" | "压缩包" | "ISO/磁盘镜像" => {
                            "确认不再需要后可在 Phase 4 加入归档计划"
                        }
                        "视频" => "建议移动到空间充足的数据盘或媒体库",
                        _ => "先打开所在目录确认用途；本阶段不删除",
                    }
                    .to_string(),
                    risk: if metadata.len() >= 5 * 1024 * 1024 * 1024 {
                        "high"
                    } else {
                        "medium"
                    }
                    .to_string(),
                    file_type,
                });
            }
        } else if let Ok(entries) = fs::read_dir(&path) {
            stack.extend(entries.flatten().map(|entry| entry.path()));
        }
    }
    result.sort_by_key(|item| std::cmp::Reverse(item.size));
    result.truncate(limit.clamp(1, 100));
    Ok(result)
}

pub fn scan_large_files(
    root: String,
    min_size_mb: u64,
    limit: usize,
) -> Result<Vec<LargeFileItem>, String> {
    let root = if root.trim().is_empty() {
        dirs::home_dir().ok_or_else(|| "无法识别用户目录".to_string())?
    } else {
        PathBuf::from(root.trim())
    };
    let minimum = min_size_mb.max(1).saturating_mul(1024 * 1024);
    collect_large_files(&root, minimum, limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_file_scan_returns_top_n() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("one.bin"), vec![0_u8; 10]).unwrap();
        fs::write(root.path().join("two.bin"), vec![0_u8; 30]).unwrap();
        fs::write(root.path().join("three.bin"), vec![0_u8; 20]).unwrap();
        let result = collect_large_files(root.path(), 1, 2).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].size, 30);
        assert_eq!(result[1].size, 20);
    }

    #[test]
    fn system_root_is_rejected() {
        assert!(validate_analysis_root(Path::new(r"C:\Windows")).is_err());
    }
}
