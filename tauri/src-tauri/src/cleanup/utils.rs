use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAX_SCAN_ENTRIES: usize = 250_000;

#[allow(dead_code)]
pub fn directory_size(root: &Path) -> (u64, usize, bool) {
    directory_size_filtered(root, |_| false)
}

pub fn directory_size_filtered<F>(root: &Path, should_exclude: F) -> (u64, usize, bool)
where
    F: Fn(&Path) -> bool,
{
    let mut bytes = 0_u64;
    let mut files = 0_usize;
    let mut visited = 0_usize;
    let mut truncated = false;
    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        if visited >= MAX_SCAN_ENTRIES {
            truncated = true;
            break;
        }
        visited += 1;
        if path != root && should_exclude(&path) {
            continue;
        }
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_file() {
            bytes = bytes.saturating_add(metadata.len());
            files += 1;
        } else if let Ok(entries) = fs::read_dir(&path) {
            stack.extend(entries.flatten().map(|entry| entry.path()));
        }
    }
    (bytes, files, truncated)
}

pub fn path_id(source: &str, path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    hasher.update(b"\0");
    hasher.update(path.to_string_lossy().to_ascii_lowercase().as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn system_time_string(value: SystemTime) -> Option<String> {
    value
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs().to_string())
}

pub fn generated_at() -> String {
    system_time_string(SystemTime::now()).unwrap_or_else(|| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn counts_file_sizes_without_following_links() {
        let root = tempfile::tempdir().unwrap();
        let mut one = fs::File::create(root.path().join("one.bin")).unwrap();
        one.write_all(&[1_u8; 13]).unwrap();
        fs::create_dir(root.path().join("nested")).unwrap();
        let mut two = fs::File::create(root.path().join("nested").join("two.bin")).unwrap();
        two.write_all(&[2_u8; 29]).unwrap();
        let (size, files, truncated) = directory_size(root.path());
        assert_eq!(size, 42);
        assert_eq!(files, 2);
        assert!(!truncated);
    }
}
