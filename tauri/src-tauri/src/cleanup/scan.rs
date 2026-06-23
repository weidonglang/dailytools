use super::model::{CleanupCategoryScan, CleanupItem, CleanupScanReport};
use super::protect::{
    classify_path_risk, is_inside_managed_runtime, is_inside_root, is_sensitive_account_data,
    should_skip_path,
};
use super::utils::{directory_size_filtered, generated_at, path_id, system_time_string};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const MAX_TEMP_ITEMS: usize = 500;

struct ScanContext {
    managed_root: PathBuf,
    warnings: Vec<String>,
    seen: HashSet<String>,
}

impl ScanContext {
    fn new(managed_root: &Path) -> Self {
        Self {
            managed_root: managed_root.to_path_buf(),
            warnings: Vec::new(),
            seen: HashSet::new(),
        }
    }

    fn add_warning(&mut self, warning: String) {
        if !self.warnings.contains(&warning) {
            self.warnings.push(warning);
        }
    }
}

fn category(
    id: &str,
    name: &str,
    description: &str,
    risk: &str,
    cleanable: bool,
) -> CleanupCategoryScan {
    CleanupCategoryScan {
        id: id.to_string(),
        name: name.to_string(),
        description: description.to_string(),
        risk: risk.to_string(),
        scan_only: true,
        cleanable,
        enabled_by_default: false,
        ..Default::default()
    }
}

fn add_path_summary(
    context: &mut ScanContext,
    category: &mut CleanupCategoryScan,
    path: &Path,
    source: &str,
    reason: &str,
    cleanable: bool,
    forced_skip: Option<&str>,
) {
    let key = path
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase();
    if !context.seen.insert(key) || !path.exists() {
        return;
    }
    if is_inside_root(path, &context.managed_root)
        && [
            "current", "envs", "jdks", "pythons", "nodes", "mavens", "gradles", "gos", "tools",
        ]
        .iter()
        .any(|name| is_inside_root(path, &context.managed_root.join(name)))
    {
        return;
    }
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            context.add_warning(format!("无法读取 {}：{error}", path.display()));
            return;
        }
    };
    if metadata.file_type().is_symlink() {
        return;
    }
    let current_project = env::current_dir().ok();
    if current_project
        .as_deref()
        .is_some_and(|project| is_inside_root(path, project))
    {
        return;
    }
    let (size, _, truncated) = if metadata.is_file() {
        (metadata.len(), 1, false)
    } else {
        directory_size_filtered(path, |child| {
            is_sensitive_account_data(child)
                || is_inside_managed_runtime(child)
                || current_project
                    .as_deref()
                    .is_some_and(|project| is_inside_root(child, project))
        })
    };
    if truncated {
        context.add_warning(format!("{} 条目过多，容量为上限内估算", path.display()));
    }
    let skipped_reason = forced_skip
        .map(str::to_string)
        .or_else(|| should_skip_path(path));
    let can_clean = cleanable && skipped_reason.is_none();
    category.items.push(CleanupItem {
        id: path_id(source, path),
        path: path.to_string_lossy().to_string(),
        size,
        modified_at: metadata.modified().ok().and_then(system_time_string),
        source: source.to_string(),
        reason: reason.to_string(),
        risk: classify_path_risk(path).as_str().to_string(),
        cleanable: can_clean,
        selected_by_default: false,
        skipped_reason,
    });
}

#[allow(clippy::too_many_arguments)]
fn add_matching_files(
    context: &mut ScanContext,
    category: &mut CleanupCategoryScan,
    root: &Path,
    source: &str,
    prefix: &str,
    extension: &str,
    reason: &str,
    forced_skip: &str,
) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten().take(MAX_TEMP_ITEMS) {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if name.starts_with(prefix) && name.ends_with(extension) {
            add_path_summary(
                context,
                category,
                &path,
                source,
                reason,
                false,
                Some(forced_skip),
            );
        }
    }
}

fn is_recent(modified: SystemTime, now: SystemTime) -> bool {
    now.duration_since(modified)
        .ok()
        .is_some_and(|age| age < Duration::from_secs(24 * 60 * 60))
}

fn scan_temp_root(
    context: &mut ScanContext,
    result: &mut CleanupCategoryScan,
    root: &Path,
    source: &str,
    system_only: bool,
) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.flatten().take(MAX_TEMP_ITEMS) {
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            continue;
        }
        let recent = metadata
            .modified()
            .ok()
            .is_some_and(|time| is_recent(time, now));
        let forced = if system_only {
            Some("Windows 系统目录仅统计，不允许清理")
        } else if recent {
            Some("24 小时内的临时项目受保护")
        } else {
            None
        };
        add_path_summary(
            context,
            result,
            &path,
            source,
            "临时文件占用",
            !system_only && !recent,
            forced,
        );
    }
}

fn developer_roots() -> Vec<(PathBuf, &'static str)> {
    let mut roots = Vec::new();
    if let Some(local) = env::var_os("LOCALAPPDATA") {
        let local = PathBuf::from(local);
        roots.extend([
            (local.join("npm-cache"), "npm cache"),
            (local.join("pnpm").join("store"), "pnpm store"),
            (local.join("Yarn").join("Cache"), "yarn cache"),
            (local.join("pip").join("Cache"), "pip cache"),
            (local.join("uv").join("cache"), "uv cache"),
            (local.join("pypoetry").join("Cache"), "Poetry cache"),
            (local.join("NuGet").join("v3-cache"), "NuGet cache"),
            (local.join("go-build"), "Go build cache"),
        ]);
    }
    if let Some(roaming) = env::var_os("APPDATA") {
        let roaming = PathBuf::from(roaming);
        roots.push((roaming.join("npm-cache"), "npm cache"));
        roots.push((roaming.join("pypoetry").join("cache"), "Poetry cache"));
    }
    if let Some(home) = dirs::home_dir() {
        roots.extend([
            (home.join(".pnpm-store"), "pnpm store"),
            (home.join(".cache").join("yarn"), "yarn cache"),
            (home.join(".cache").join("pip"), "pip cache"),
            (home.join(".cache").join("uv"), "uv cache"),
            (home.join(".m2").join("repository"), "Maven repository"),
            (home.join(".gradle").join("caches"), "Gradle caches"),
            (
                home.join(".cargo").join("registry").join("cache"),
                "Cargo registry cache",
            ),
            (
                home.join("go").join("pkg").join("mod").join("cache"),
                "Go module cache",
            ),
            (home.join(".nuget").join("packages"), "NuGet packages"),
        ]);
    }
    roots
}

fn add_common_cargo_targets(context: &mut ScanContext, category: &mut CleanupCategoryScan) {
    let Some(home) = dirs::home_dir() else { return };
    for parent in [
        "Projects",
        "projects",
        "source",
        "Source",
        "repos",
        "workspace",
    ] {
        let Ok(entries) = fs::read_dir(home.join(parent)) else {
            continue;
        };
        for project in entries.flatten().take(100) {
            let target = project.path().join("target");
            add_path_summary(
                context,
                category,
                &target,
                "Cargo target",
                "Rust 构建产物（仅常见项目目录）",
                true,
                None,
            );
        }
    }
}

fn finalize(mut category: CleanupCategoryScan) -> CleanupCategoryScan {
    category.total_bytes = category.items.iter().map(|item| item.size).sum();
    category.item_count = category.items.len();
    category
        .items
        .sort_by_key(|item| std::cmp::Reverse(item.size));
    category
}

pub fn scan_cleanup_targets(managed_root: &Path) -> Result<CleanupScanReport, String> {
    let mut context = ScanContext::new(managed_root);
    let mut temp = category(
        "windows-temp",
        "C 盘临时文件",
        "用户 Temp 可评估；Windows Temp 永远只统计",
        "medium",
        true,
    );
    let mut temp_roots = vec![(env::temp_dir(), "%TEMP%", false)];
    if let Some(local) = env::var_os("LOCALAPPDATA") {
        temp_roots.push((
            PathBuf::from(local).join("Temp"),
            "用户 AppData Temp",
            false,
        ));
    }
    let windows = env::var_os("WINDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    temp_roots.push((windows.join("Temp"), "Windows Temp", true));
    for (root, source, system_only) in temp_roots {
        scan_temp_root(&mut context, &mut temp, &root, source, system_only);
    }

    let mut system = category(
        "system-caches",
        "Windows 系统缓存",
        "错误报告、缩略图和 DirectX Shader Cache 仅统计",
        "high",
        false,
    );
    if let Some(local) = env::var_os("LOCALAPPDATA") {
        let local = PathBuf::from(local);
        add_path_summary(
            &mut context,
            &mut system,
            &local.join("Microsoft").join("Windows").join("WER"),
            "Windows Error Reporting",
            "Windows 错误报告",
            false,
            Some("系统诊断数据仅统计"),
        );
        add_matching_files(
            &mut context,
            &mut system,
            &local.join("Microsoft").join("Windows").join("Explorer"),
            "Thumbnail Cache",
            "thumbcache_",
            ".db",
            "包含缩略图缓存，Phase 1 仅统计",
            "缩略图缓存仅统计",
        );
        add_path_summary(
            &mut context,
            &mut system,
            &local.join("D3DSCache"),
            "DirectX Shader Cache",
            "DirectX 着色器缓存",
            false,
            Some("DirectX 缓存仅统计"),
        );
    }
    let program_data = env::var_os("ProgramData")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
    add_path_summary(
        &mut context,
        &mut system,
        &program_data.join("Microsoft").join("Windows").join("WER"),
        "Windows Error Reporting",
        "系统错误报告",
        false,
        Some("系统诊断数据仅统计"),
    );

    let mut recycle = category(
        "recycle-bin",
        "Windows 回收站",
        "仅估算回收站占用，不读取或清空内容",
        "high",
        false,
    );
    add_path_summary(
        &mut context,
        &mut recycle,
        Path::new(r"C:\$Recycle.Bin"),
        "Recycle Bin",
        "Windows 回收站占用",
        false,
        Some("回收站默认不清理"),
    );

    let mut devenv = category(
        "devenv-manager",
        "DevEnv Manager 自身",
        "下载和日志可评估，config 只统计",
        "medium",
        true,
    );
    add_path_summary(
        &mut context,
        &mut devenv,
        &managed_root.join("downloads"),
        "DevEnv downloads",
        "已下载的安装缓存",
        true,
        None,
    );
    add_path_summary(
        &mut context,
        &mut devenv,
        &managed_root.join("logs"),
        "DevEnv logs",
        "应用日志",
        true,
        None,
    );
    add_path_summary(
        &mut context,
        &mut devenv,
        &managed_root.join("config"),
        "DevEnv config",
        "应用配置",
        false,
        Some("配置目录只统计，不清理"),
    );
    if let Some(config_root) = dirs::data_local_dir() {
        add_path_summary(
            &mut context,
            &mut devenv,
            &config_root.join("DevEnvManager"),
            "DevEnv app data",
            "应用配置与状态数据",
            false,
            Some("配置目录只统计，不清理"),
        );
    }

    let mut developer = category(
        "developer-caches",
        "开发缓存",
        "包管理器和构建工具可再生成缓存",
        "medium",
        true,
    );
    for (root, source) in developer_roots() {
        add_path_summary(
            &mut context,
            &mut developer,
            &root,
            source,
            "开发工具可再生成缓存",
            true,
            None,
        );
    }
    add_common_cargo_targets(&mut context, &mut developer);

    let mut wps = category(
        "wps-cache",
        "WPS 临时文件与日志",
        "只检查明确命名的缓存、临时和日志目录，不进入文档、备份或云同步数据",
        "high",
        false,
    );
    let mut wps_roots = Vec::new();
    if let Some(local) = env::var_os("LOCALAPPDATA") {
        let local = PathBuf::from(local);
        wps_roots.push((local.join("Kingsoft/WPS Office/logs"), "WPS logs"));
        wps_roots.push((local.join("Kingsoft/WPS Office/cache"), "WPS cache"));
        wps_roots.push((local.join("Kingsoft/WPS Office/temp"), "WPS temp"));
    }
    if let Some(roaming) = env::var_os("APPDATA") {
        wps_roots.push((
            PathBuf::from(roaming).join("kingsoft/office6/log"),
            "WPS logs",
        ));
    }
    let temp_root = env::temp_dir();
    wps_roots.push((temp_root.join("Kingsoft"), "WPS temp"));
    wps_roots.push((temp_root.join("WPS"), "WPS temp"));
    for (path, source) in wps_roots {
        let lower = path.to_string_lossy().to_ascii_lowercase();
        if lower.contains("backup") || lower.contains("document") || lower.contains("cloud") {
            continue;
        }
        add_path_summary(
            &mut context,
            &mut wps,
            &path,
            source,
            "WPS 明确缓存/临时/日志路径",
            false,
            Some("WPS 类别在本版本仅预览，防止误伤备份或云文档"),
        );
    }

    let categories = vec![
        finalize(temp),
        finalize(system),
        finalize(recycle),
        finalize(devenv),
        finalize(developer),
        finalize(wps),
    ];
    let total_bytes = categories.iter().map(|item| item.total_bytes).sum();
    let total_items = categories.iter().map(|item| item.item_count).sum();
    context.add_warning("Phase 1 为只读扫描：没有任何删除、移动或清空行为".to_string());
    context.add_warning(
        "安全边界：默认扫描不会进入桌面、下载、文档、图片、视频或音乐目录".to_string(),
    );
    Ok(CleanupScanReport {
        generated_at: generated_at(),
        total_bytes,
        total_items,
        categories,
        warnings: context.warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn missing_directories_do_not_fail_scan() {
        let root = tempfile::tempdir().unwrap();
        let mut context = ScanContext::new(root.path());
        let mut result = category("test", "Test", "test", "low", false);
        add_path_summary(
            &mut context,
            &mut result,
            &root.path().join("does-not-exist"),
            "test",
            "test",
            false,
            None,
        );
        assert!(result.items.is_empty());
    }

    #[test]
    fn temp_scan_protects_recent_and_allows_old_items() {
        let root = tempfile::tempdir().unwrap();
        let mut file = fs::File::create(root.path().join("recent.tmp")).unwrap();
        file.write_all(b"temporary").unwrap();
        let mut context = ScanContext::new(Path::new(r"D:\DevEnvManager"));
        let mut result = category("windows-temp", "Temp", "test", "medium", true);
        scan_temp_root(&mut context, &mut result, root.path(), "test temp", false);
        assert_eq!(result.items.len(), 1);
        assert!(!result.items[0].cleanable);
        assert!(result.items[0]
            .skipped_reason
            .as_deref()
            .unwrap()
            .contains("24"));
        let now = SystemTime::now();
        assert!(is_recent(now - Duration::from_secs(60), now));
        assert!(!is_recent(now - Duration::from_secs(25 * 60 * 60), now));
    }
}
