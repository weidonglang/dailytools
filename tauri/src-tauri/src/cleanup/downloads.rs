use super::model::{FolderUsageItem, FolderUsageReport};
use super::protect::is_sensitive_account_data;
use super::utils::system_time_string;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const MAX_FOLDER_ENTRIES: usize = 100_000;

pub(crate) fn classify_file_type(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match extension.as_str() {
        "exe" | "msi" | "msix" | "appx" | "appxbundle" => "安装包",
        "zip" | "7z" | "rar" | "tar" | "gz" | "bz2" | "xz" => "压缩包",
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" => "视频",
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "heic" | "svg" => "图片",
        "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "pdf" | "txt" | "md" | "rtf" => "文档",
        "iso" | "img" | "vhd" | "vhdx" => "ISO/磁盘镜像",
        "lnk" | "url" => "快捷方式",
        _ => "其他",
    }
}

fn collect_files(root: &Path) -> (Vec<(PathBuf, u64, Option<SystemTime>)>, bool) {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    let mut visited = 0_usize;
    let mut truncated = false;
    while let Some(path) = stack.pop() {
        if visited >= MAX_FOLDER_ENTRIES {
            truncated = true;
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
            files.push((path, metadata.len(), metadata.modified().ok()));
        } else if let Ok(entries) = fs::read_dir(&path) {
            stack.extend(entries.flatten().map(|entry| entry.path()));
        }
    }
    (files, truncated)
}

fn category_item(root: &Path, name: &str, size: u64, suggestion: &str) -> FolderUsageItem {
    FolderUsageItem {
        name: name.to_string(),
        path: root.to_string_lossy().to_string(),
        size,
        category: name.to_string(),
        suggestion: suggestion.to_string(),
    }
}

pub(crate) fn inspect_folder(root: &Path, desktop: bool) -> FolderUsageReport {
    let (files, truncated) = collect_files(root);
    let total_bytes = files.iter().map(|(_, size, _)| *size).sum();
    let now = SystemTime::now();
    let mut sizes: HashMap<&'static str, u64> = HashMap::new();
    let mut same_size: HashMap<u64, usize> = HashMap::new();
    for (path, size, modified) in &files {
        *sizes.entry(classify_file_type(path)).or_default() += *size;
        *same_size.entry(*size).or_default() += 1;
        if modified.is_some_and(|time| {
            now.duration_since(time).unwrap_or(Duration::ZERO)
                >= Duration::from_secs(30 * 24 * 60 * 60)
        }) {
            *sizes.entry("超过 30 天未修改").or_default() += *size;
        }
        if *size >= 1024 * 1024 * 1024 {
            *sizes.entry("超过 1GB").or_default() += *size;
        }
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if desktop
            && (name.contains("screenshot")
                || name.contains("screen shot")
                || name.contains("截图")
                || name.contains("截屏"))
        {
            *sizes.entry("截图").or_default() += *size;
        }
    }
    if desktop {
        let reclaimable = same_size
            .into_iter()
            .filter(|(size, count)| *size > 0 && *count > 1)
            .map(|(size, count)| size.saturating_mul((count - 1) as u64))
            .sum();
        sizes.insert("重复文件候选", reclaimable);
    }
    let order = if desktop {
        vec![
            "超过 1GB",
            "快捷方式",
            "安装包",
            "压缩包",
            "截图",
            "超过 30 天未修改",
            "重复文件候选",
            "视频",
            "图片",
            "文档",
            "其他",
        ]
    } else {
        vec![
            "安装包",
            "压缩包",
            "视频",
            "图片",
            "文档",
            "ISO/磁盘镜像",
            "超过 30 天未修改",
            "超过 1GB",
            "其他",
        ]
    };
    let categories = order
        .into_iter()
        .filter_map(|name| {
            let size = sizes.get(name).copied().unwrap_or(0);
            (size > 0).then(|| {
                category_item(
                    root,
                    name,
                    size,
                    if name == "安装包" || name.contains("30 天") {
                        "确认不再需要后加入未来归档计划；本阶段不移动、不删除"
                    } else if name == "重复文件候选" {
                        "仅按大小筛选候选，使用重复文件页计算 SHA256 后再判断"
                    } else {
                        "按类型查看并决定是否归档；本阶段只提供建议"
                    },
                )
            })
        })
        .collect();
    FolderUsageReport {
        name: if desktop {
            "桌面急救"
        } else {
            "下载目录"
        }
        .to_string(),
        path: root.to_string_lossy().to_string(),
        total_bytes,
        categories,
        suggestions: vec![
            "本阶段只生成整理建议，不删除或移动桌面/下载文件".to_string(),
            "旧安装包在归档前应确认对应软件已安装且安装包可重新获取".to_string(),
        ],
        warnings: truncated
            .then(|| "目录条目超过扫描上限，结果为上限内估算".to_string())
            .into_iter()
            .collect(),
    }
}

pub fn inspect_downloads() -> FolderUsageReport {
    let root = dirs::download_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join("Downloads"));
    inspect_folder(&root, false)
}

#[allow(dead_code)]
pub(crate) fn modified_string(value: Option<SystemTime>) -> Option<String> {
    value.and_then(system_time_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_file_types() {
        assert_eq!(classify_file_type(Path::new("setup.msi")), "安装包");
        assert_eq!(classify_file_type(Path::new("archive.7z")), "压缩包");
        assert_eq!(classify_file_type(Path::new("movie.mp4")), "视频");
        assert_eq!(classify_file_type(Path::new("disk.iso")), "ISO/磁盘镜像");
        assert_eq!(classify_file_type(Path::new("readme.unknown")), "其他");
    }
}
