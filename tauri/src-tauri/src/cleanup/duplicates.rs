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
const QUICK_HASH_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, Copy, Default)]
pub struct DuplicateScanProgress {
    pub stage: &'static str,
    pub visited_entries: usize,
    pub candidate_count: usize,
    pub quick_hashed: usize,
    pub full_hashed: usize,
    pub truncated: bool,
}

fn candidate_files_with_progress<F, C>(
    root: &Path,
    min_bytes: u64,
    mut progress: F,
    should_cancel: C,
) -> Result<Vec<(PathBuf, u64, Option<SystemTime>)>, String>
where
    F: FnMut(DuplicateScanProgress),
    C: Fn() -> bool,
{
    let mut result = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    let mut visited = 0_usize;
    let mut truncated = false;
    while let Some(path) = stack.pop() {
        if should_cancel() {
            return Err("扫描已取消".to_string());
        }
        if visited >= MAX_DUPLICATE_ENTRIES || result.len() >= MAX_HASH_CANDIDATES {
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
            if metadata.len() >= min_bytes {
                result.push((path, metadata.len(), metadata.modified().ok()));
            }
        } else if let Ok(entries) = fs::read_dir(&path) {
            stack.extend(entries.flatten().map(|entry| entry.path()));
        }
        if visited == 1 || visited.is_multiple_of(500) {
            progress(DuplicateScanProgress {
                stage: "collect",
                visited_entries: visited,
                candidate_count: result.len(),
                truncated,
                ..Default::default()
            });
        }
    }
    progress(DuplicateScanProgress {
        stage: "collect",
        visited_entries: visited,
        candidate_count: result.len(),
        truncated,
        ..Default::default()
    });
    Ok(result)
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

fn quick_sha256_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut remaining = QUICK_HASH_BYTES;
    let mut buffer = [0_u8; 64 * 1024];
    while remaining > 0 {
        let limit = buffer.len().min(remaining as usize);
        let read = file
            .read(&mut buffer[..limit])
            .map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        remaining = remaining.saturating_sub(read as u64);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
pub(crate) fn scan_duplicate_files(
    root: &Path,
    min_bytes: u64,
) -> Result<Vec<DuplicateGroup>, String> {
    scan_duplicate_files_with_progress(root, min_bytes, |_| {}, || false)
}

pub(crate) fn scan_duplicate_files_with_progress<F, C>(
    root: &Path,
    min_bytes: u64,
    mut progress: F,
    should_cancel: C,
) -> Result<Vec<DuplicateGroup>, String>
where
    F: FnMut(DuplicateScanProgress),
    C: Fn() -> bool,
{
    validate_analysis_root(root)?;
    let candidates = candidate_files_with_progress(root, min_bytes, &mut progress, &should_cancel)?;
    let candidate_count = candidates.len();
    let size_groups = group_by_size(candidates);
    let mut result = Vec::new();
    let mut quick_hashed = 0_usize;
    let mut full_hashed = 0_usize;
    for (size, candidates) in size_groups {
        let mut quick_hashes: HashMap<String, Vec<(PathBuf, Option<SystemTime>)>> = HashMap::new();
        for (path, modified) in candidates {
            if should_cancel() {
                return Err("扫描已取消".to_string());
            }
            if let Ok(hash) = quick_sha256_file(&path) {
                quick_hashes.entry(hash).or_default().push((path, modified));
            }
            quick_hashed += 1;
            if quick_hashed == 1 || quick_hashed.is_multiple_of(100) {
                progress(DuplicateScanProgress {
                    stage: "quick_hash",
                    candidate_count,
                    quick_hashed,
                    full_hashed,
                    ..Default::default()
                });
            }
        }
        for candidates in quick_hashes.into_values().filter(|items| items.len() > 1) {
            let mut hashes: HashMap<String, Vec<(PathBuf, Option<SystemTime>)>> = HashMap::new();
            for (path, modified) in candidates {
                if should_cancel() {
                    return Err("扫描已取消".to_string());
                }
                if let Ok(hash) = sha256_file(&path) {
                    hashes.entry(hash).or_default().push((path, modified));
                }
                full_hashed += 1;
                if full_hashed == 1 || full_hashed.is_multiple_of(25) {
                    progress(DuplicateScanProgress {
                        stage: "full_hash",
                        candidate_count,
                        quick_hashed,
                        full_hashed,
                        ..Default::default()
                    });
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
    }
    result.sort_by_key(|group| std::cmp::Reverse(group.reclaimable_estimate));
    result.truncate(200);
    progress(DuplicateScanProgress {
        stage: "done",
        candidate_count,
        quick_hashed,
        full_hashed,
        ..Default::default()
    });
    Ok(result)
}

pub fn scan_duplicate_large_files_with_progress<F, C>(
    root: String,
    min_size_mb: u64,
    progress: F,
    should_cancel: C,
) -> Result<Vec<DuplicateGroup>, String>
where
    F: FnMut(DuplicateScanProgress),
    C: Fn() -> bool,
{
    let root = if root.trim().is_empty() {
        dirs::home_dir().ok_or_else(|| "无法识别用户目录".to_string())?
    } else {
        PathBuf::from(root.trim())
    };
    scan_duplicate_files_with_progress(
        &root,
        min_size_mb.max(1).saturating_mul(1024 * 1024),
        progress,
        should_cancel,
    )
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

    #[test]
    fn quick_hash_is_only_prefilter_before_full_hash() {
        let root = tempfile::tempdir().unwrap();
        let mut first = vec![b'a'; QUICK_HASH_BYTES as usize];
        first.extend_from_slice(b"tail-one");
        let mut second = vec![b'a'; QUICK_HASH_BYTES as usize];
        second.extend_from_slice(b"tail-two");
        fs::write(root.path().join("a.bin"), first).unwrap();
        fs::write(root.path().join("b.bin"), second).unwrap();
        let result = scan_duplicate_files(root.path(), 1).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn duplicate_scan_reports_hash_progress_and_honors_cancel() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("a.bin"), b"same-content").unwrap();
        fs::write(root.path().join("b.bin"), b"same-content").unwrap();
        let mut stages = Vec::new();
        let result = scan_duplicate_files_with_progress(
            root.path(),
            1,
            |update| stages.push(update.stage),
            || false,
        )
        .unwrap();
        assert_eq!(result.len(), 1);
        assert!(stages.contains(&"collect"));
        assert!(stages.contains(&"quick_hash"));
        assert!(stages.contains(&"full_hash"));

        let error =
            scan_duplicate_files_with_progress(root.path(), 1, |_| {}, || true).unwrap_err();
        assert!(error.contains("取消"));
    }
}
