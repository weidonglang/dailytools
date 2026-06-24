use super::clean_plan::take_valid_plan;
use super::clean_report::store_result;
use super::model::{CleanupFailure, CleanupPlan, CleanupPlanItem, CleanupResult};
use super::protect::{is_protected_path, should_skip_path};
use super::scan::scan_cleanup_targets;
use super::utils::{directory_size, generated_at};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

fn failure(path: &Path, reason: impl Into<String>) -> CleanupFailure {
    CleanupFailure {
        path: path.to_string_lossy().to_string(),
        reason: reason.into(),
    }
}

fn validate_candidate(path: &Path) -> Result<(), String> {
    if let Some(reason) = should_skip_path(path) {
        return Err(reason);
    }
    if is_protected_path(path) {
        return Err("保护路径不会被清理".to_string());
    }
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if metadata.file_type().is_symlink() {
        return Err("符号链接不会被清理".to_string());
    }
    Ok(())
}

fn execute_items<F>(items: &[CleanupPlanItem], mut cleaner: F) -> CleanupResult
where
    F: FnMut(&Path) -> Result<(), String>,
{
    let started_at = generated_at();
    let mut result = CleanupResult {
        started_at,
        ..Default::default()
    };
    for item in items {
        let path = PathBuf::from(&item.path);
        if !path.exists() {
            result.skipped_items += 1;
            continue;
        }
        if let Err(reason) = validate_candidate(&path) {
            result.failed_items += 1;
            result.failures.push(failure(&path, reason));
            continue;
        }
        match cleaner(&path) {
            Ok(()) if !path.exists() => {
                result.cleaned_items += 1;
                result.cleaned_bytes = result.cleaned_bytes.saturating_add(item.size);
            }
            Ok(()) => {
                result.failed_items += 1;
                result
                    .failures
                    .push(failure(&path, "清理后路径仍然存在，未计入释放空间"));
            }
            Err(reason) => {
                result.failed_items += 1;
                result.failures.push(failure(&path, reason));
            }
        }
    }
    result.finished_at = generated_at();
    result.success = result.failed_items == 0;
    result
}

pub fn clean_selected_targets(
    managed_root: &Path,
    submitted: CleanupPlan,
) -> Result<CleanupResult, String> {
    let plan = take_valid_plan(&submitted)?;
    let fresh = scan_cleanup_targets(managed_root)?;
    let current = fresh
        .categories
        .iter()
        .flat_map(|category| {
            category
                .items
                .iter()
                .filter(|item| item.cleanable)
                .map(move |item| (item.id.clone(), (category.id.clone(), item.path.clone())))
        })
        .collect::<HashMap<_, _>>();
    for item in &plan.selected_items {
        let Some((category, path)) = current.get(&item.item_id) else {
            return Err(format!("清理项已经失效，请重新扫描：{}", item.path));
        };
        if category != &item.category_id || path != &item.path {
            return Err("清理项路径或分类发生变化，请重新扫描".to_string());
        }
    }
    let mut result = execute_items(&plan.selected_items, |path| {
        trash::delete(path).map_err(|error| format!("移入回收站失败：{error}"))
    });
    result.plan_id = plan.plan_id;
    Ok(store_result(result))
}

fn download_cache_items(root: &Path) -> Vec<CleanupPlanItem> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).ok()?;
            if metadata.file_type().is_symlink() {
                return None;
            }
            let size = if metadata.is_file() {
                metadata.len()
            } else {
                directory_size(&path).0
            };
            Some(CleanupPlanItem {
                item_id: format!("download-{}", entry.file_name().to_string_lossy()),
                path: path.to_string_lossy().to_string(),
                size,
                category_id: "devenv-manager".to_string(),
                risk: "low".to_string(),
                action: "move_to_recycle_bin".to_string(),
                reversible: true,
            })
        })
        .collect()
}

fn clean_managed_download_cache_with<F>(managed_root: &Path, cleaner: F) -> CleanupResult
where
    F: FnMut(&Path) -> Result<(), String>,
{
    let root = managed_root.join("downloads");
    let items = download_cache_items(&root);
    let mut result = execute_items(&items, cleaner);
    result.plan_id = format!("managed-downloads-{}", generated_at());
    result
}

pub fn clean_managed_download_cache(managed_root: &Path) -> CleanupResult {
    let result = clean_managed_download_cache_with(managed_root, |path| {
        trash::delete(path).map_err(|error| format!("移入回收站失败：{error}"))
    });
    store_result(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn missing_path_is_skipped_without_failure() {
        let item = CleanupPlanItem {
            path: tempfile::tempdir()
                .unwrap()
                .path()
                .join("missing.tmp")
                .to_string_lossy()
                .to_string(),
            ..Default::default()
        };
        let result = execute_items(&[item], |_| Ok(()));
        assert_eq!(result.skipped_items, 1);
        assert_eq!(result.failed_items, 0);
    }

    #[test]
    fn cleanup_failure_is_recorded() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("old.tmp");
        fs::File::create(&path).unwrap().write_all(b"x").unwrap();
        let item = CleanupPlanItem {
            path: path.to_string_lossy().to_string(),
            size: 1,
            ..Default::default()
        };
        let result = execute_items(&[item], |_| Err("模拟失败".to_string()));
        assert_eq!(result.failed_items, 1);
        assert!(result.failures[0].reason.contains("模拟失败"));
    }

    #[test]
    fn managed_download_cache_enumerates_only_children() {
        let root = tempfile::tempdir().unwrap();
        let downloads = root.path().join("downloads");
        fs::create_dir(&downloads).unwrap();
        fs::write(downloads.join("archive.zip"), [1_u8; 7]).unwrap();
        let items = download_cache_items(&downloads);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].size, 7);
        assert_ne!(PathBuf::from(&items[0].path), downloads);
    }

    #[test]
    fn managed_download_cache_cleanup_removes_test_file() {
        let root = tempfile::tempdir().unwrap();
        let downloads = root.path().join("downloads");
        fs::create_dir(&downloads).unwrap();
        let file = downloads.join("archive.zip");
        fs::write(&file, [1_u8; 7]).unwrap();
        let result = clean_managed_download_cache_with(root.path(), |path| {
            fs::remove_file(path).map_err(|error| error.to_string())
        });
        assert!(result.success);
        assert_eq!(result.cleaned_items, 1);
        assert_eq!(result.cleaned_bytes, 7);
        assert!(!file.exists());
        assert!(downloads.exists());
    }

    #[test]
    fn protected_path_is_never_cleaned() {
        assert!(validate_candidate(Path::new(r"C:\Windows\Temp")).is_err());
    }
}
