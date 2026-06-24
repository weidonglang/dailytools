use super::large_files::validate_analysis_root;
use super::model::{DuplicateFileItem, DuplicateGroup};
use super::protect::is_sensitive_account_data;
use super::utils::system_time_string;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const MAX_DUPLICATE_ENTRIES: usize = 250_000;
const MAX_HASH_CANDIDATES: usize = 50_000;

fn candidate_files(root: &Path, min_bytes: u64) -> Vec<(PathBuf, u64, Option<SystemTime>)> {
    let mut result = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    let mut visited = 0_usize;
    while let Some(path) = stack.pop() {
        if visited >= MAX_DUPLICATE_ENTRIES || result.len() >= MAX_HASH_CANDIDATES {
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
                result.push((path, metadata.len(), metadata.modified().ok()));
            }
        } else if let Ok(entries) = fs::read_dir(&path) {
            stack.extend(entries.flatten().map(|entry| entry.path()));
        }
    }
    result
}

fn group_by_size(
    files: Vec<(PathBuf, u64, Option<SystemTime>)>,
) -> HashMap<u64, Vec<(PathBuf, Option<SystemTime>)>> {
    let mut groups: HashMap<u64, Vec<(PathBuf, Option<SystemTime>)>> = HashMap::new();
    for (path, size, modified) in files {
        groups.entry(size).or_default().push((path, modified));
    }
    groups.retain(|_, items| items.len() > 1);
    groups
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub(crate) fn scan_duplicate_files(
    root: &Path,
    min_bytes: u64,
) -> Result<Vec<DuplicateGroup>, String> {
    validate_analysis_root(root)?;
    let size_groups = group_by_size(candidate_files(root, min_bytes));
    let mut result = Vec::new();
    for (size, candidates) in size_groups {
        let mut hashes: HashMap<String, Vec<(PathBuf, Option<SystemTime>)>> = HashMap::new();
        for (path, modified) in candidates {
            if let Ok(hash) = sha256_file(&path) {
                hashes.entry(hash).or_default().push((path, modified));
            }
        }
        for (hash, mut files) in hashes {
            if files.len() < 2 {
                continue;
            }
            files.sort_by_key(|item| std::cmp::Reverse(item.1));
            let file_count = files.len();
            let files = files
                .into_iter()
                .enumerate()
                .map(|(index, (path, modified))| DuplicateFileItem {
                    path: path.to_string_lossy().to_string(),
                    modified_at: modified.and_then(system_time_string),
                    keep_suggestion: if index == 0 {
                        "较新文件，建议优先保留；仍需人工确认内容用途"
                    } else {
                        "内容与组内文件完全一致，可作为未来归档候选"
                    }
                    .to_string(),
                })
                .collect();
            result.push(DuplicateGroup {
                size,
                hash,
                files,
                reclaimable_estimate: size.saturating_mul((file_count - 1) as u64),
            });
        }
    }
    result.sort_by_key(|group| std::cmp::Reverse(group.reclaimable_estimate));
    result.truncate(200);
    Ok(result)
}

pub fn scan_duplicate_large_files(
    root: String,
    min_size_mb: u64,
) -> Result<Vec<DuplicateGroup>, String> {
    let root = if root.trim().is_empty() {
        dirs::home_dir().ok_or_else(|| "无法识别用户目录".to_string())?
    } else {
        PathBuf::from(root.trim())
    };
    scan_duplicate_files(&root, min_size_mb.max(1).saturating_mul(1024 * 1024))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_candidates_are_grouped_by_size_first() {
        let files = vec![
            (PathBuf::from("a"), 10, None),
            (PathBuf::from("b"), 10, None),
            (PathBuf::from("c"), 20, None),
        ];
        let groups = group_by_size(files);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups.get(&10).unwrap().len(), 2);
    }

    #[test]
    fn sha256_confirms_real_duplicates() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("a.bin"), b"same-content").unwrap();
        fs::write(root.path().join("b.bin"), b"same-content").unwrap();
        fs::write(root.path().join("c.bin"), b"different---").unwrap();
        let result = scan_duplicate_files(root.path(), 1).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].files.len(), 2);
        assert_eq!(result[0].reclaimable_estimate, 12);
    }
}
