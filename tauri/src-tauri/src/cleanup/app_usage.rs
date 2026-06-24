use super::game_usage::inspect_game_platforms;
use super::model::{AppUsageItem, AppUsageReport, FolderUsageItem};
use super::software::inspect_installed_software_usage;
use super::utils::directory_size;
use std::collections::HashSet;
use std::env;
use std::path::PathBuf;

pub(crate) fn usage_item(
    name: &str,
    roots: Vec<(&str, PathBuf)>,
    actions: Vec<&str>,
) -> AppUsageItem {
    let mut categories = Vec::new();
    let mut seen = HashSet::new();
    for (category, path) in roots {
        let key = path
            .to_string_lossy()
            .replace('/', "\\")
            .to_ascii_lowercase();
        if !seen.insert(key) || !path.exists() {
            continue;
        }
        let size = if path.is_file() {
            path.metadata().map(|value| value.len()).unwrap_or(0)
        } else {
            directory_size(&path).0
        };
        categories.push(FolderUsageItem {
            name: category.to_string(),
            path: path.to_string_lossy().to_string(),
            size,
            category: category.to_string(),
            suggestion: "只展示占用；请通过应用内置设置备份、迁移或清理".to_string(),
        });
    }
    let size = categories.iter().map(|item| item.size).sum();
    AppUsageItem {
        name: name.to_string(),
        detected: !categories.is_empty(),
        path: categories
            .first()
            .map(|item| item.path.clone())
            .unwrap_or_default(),
        size,
        categories,
        safe_actions: actions.into_iter().map(str::to_string).collect(),
        warnings: vec![
            "仅统计路径和文件大小元数据，不读取账号、聊天正文或文件内容".to_string(),
            "本页面不会删除应用数据、登录数据或安装目录".to_string(),
        ],
    }
}

fn env_path(name: &str) -> PathBuf {
    env::var_os(name).map(PathBuf::from).unwrap_or_default()
}

pub fn inspect_app_usage() -> AppUsageReport {
    let home = dirs::home_dir().unwrap_or_default();
    let documents = dirs::document_dir().unwrap_or_else(|| home.join("Documents"));
    let downloads = dirs::download_dir().unwrap_or_else(|| home.join("Downloads"));
    let local = env_path("LOCALAPPDATA");
    let roaming = env_path("APPDATA");
    let wechat = usage_item(
        "微信",
        vec![
            ("用户文件总占用", documents.join("WeChat Files")),
            ("客户端数据总占用", roaming.join(r"Tencent\WeChat")),
            ("新版客户端数据", documents.join("xwechat_files")),
        ],
        vec!["打开目录", "提醒先在微信内备份", "建议使用微信迁移功能"],
    );
    let qq = usage_item(
        "QQ",
        vec![
            ("用户文件总占用", documents.join("Tencent Files")),
            ("客户端数据总占用", roaming.join(r"Tencent\QQ")),
        ],
        vec!["打开目录", "提醒备份聊天与接收文件", "建议通过 QQ 设置迁移"],
    );
    let mut firefox_cache_roots = Vec::new();
    if let Ok(profiles) = std::fs::read_dir(local.join(r"Mozilla\Firefox\Profiles")) {
        for profile in profiles.flatten().take(50) {
            firefox_cache_roots.push(("cache2", profile.path().join("cache2")));
            firefox_cache_roots.push(("startupCache", profile.path().join("startupCache")));
        }
    }
    let mut browsers = vec![
        usage_item(
            "Google Chrome 缓存",
            vec![
                (
                    "Cache",
                    local.join(r"Google\Chrome\User Data\Default\Cache"),
                ),
                (
                    "Code Cache",
                    local.join(r"Google\Chrome\User Data\Default\Code Cache"),
                ),
                (
                    "GPU Cache",
                    local.join(r"Google\Chrome\User Data\Default\GPUCache"),
                ),
            ],
            vec!["打开缓存目录", "建议使用浏览器内置清理入口"],
        ),
        usage_item(
            "Microsoft Edge 缓存",
            vec![
                (
                    "Cache",
                    local.join(r"Microsoft\Edge\User Data\Default\Cache"),
                ),
                (
                    "Code Cache",
                    local.join(r"Microsoft\Edge\User Data\Default\Code Cache"),
                ),
                (
                    "GPU Cache",
                    local.join(r"Microsoft\Edge\User Data\Default\GPUCache"),
                ),
            ],
            vec!["打开缓存目录", "建议使用浏览器内置清理入口"],
        ),
        usage_item(
            "Firefox 缓存",
            firefox_cache_roots,
            vec!["打开缓存目录", "建议使用 Firefox 设置清理缓存"],
        ),
    ];
    browsers.retain(|item| item.detected);

    let mut net_disks = vec![
        usage_item(
            "百度网盘",
            vec![
                ("下载目录", home.join("BaiduNetdiskDownload")),
                ("下载目录候选", downloads.join("BaiduNetdisk")),
            ],
            vec!["打开目录", "建议在百度网盘设置中迁移下载位置"],
        ),
        usage_item(
            "夸克网盘",
            vec![("下载目录", downloads.join("QuarkDownloads"))],
            vec!["打开目录", "建议在夸克设置中迁移下载位置"],
        ),
        usage_item(
            "迅雷",
            vec![
                ("下载目录", downloads.join("Thunder")),
                ("默认下载目录", home.join("Thunder Network")),
            ],
            vec!["打开目录", "建议在迅雷设置中修改下载目录"],
        ),
    ];
    net_disks.retain(|item| item.detected);

    let mut video_editors = vec![
        usage_item(
            "剪映专业版缓存",
            vec![
                ("Cache", local.join(r"JianyingPro\User Data\Cache")),
                ("Cache", local.join(r"CapCut\User Data\Cache")),
            ],
            vec!["打开缓存目录", "建议使用剪映设置中的缓存清理与迁移"],
        ),
        usage_item(
            "Adobe 媒体缓存",
            vec![
                (
                    "Common Media Cache",
                    roaming.join(r"Adobe\Common\Media Cache Files"),
                ),
                (
                    "Common Media Cache",
                    local.join(r"Adobe\Common\Media Cache Files"),
                ),
            ],
            vec!["打开缓存目录", "建议在 Premiere/After Effects 首选项中清理"],
        ),
        usage_item(
            "Adobe Photoshop 缓存",
            vec![("Temp", local.join("Temp").join("Photoshop Temp"))],
            vec!["打开目录", "关闭 Photoshop 后使用应用建议方式处理"],
        ),
    ];
    video_editors.retain(|item| item.detected);

    AppUsageReport {
        wechat: wechat.detected.then_some(wechat),
        qq: qq.detected.then_some(qq),
        browsers,
        net_disks,
        video_editors,
        game_platforms: inspect_game_platforms(),
        installed_software: inspect_installed_software_usage(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_analysis_never_targets_credentials() {
        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("Default").join("Cache");
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(cache.join("data.bin"), b"cache").unwrap();
        let browser = usage_item("Browser", vec![("Cache", cache)], vec!["打开目录"]);
        for category in browser.categories {
            let lower = category.path.to_ascii_lowercase();
            assert!(!lower.contains("cookie"));
            assert!(!lower.contains("login data"));
            assert!(!lower.contains("password"));
        }
    }

    #[test]
    fn chat_app_report_does_not_list_database_files() {
        let root = tempfile::tempdir().unwrap();
        let files = root.path().join("WeChat Files");
        std::fs::create_dir(&files).unwrap();
        std::fs::write(files.join("MicroMsg.db"), b"not-read-by-analysis").unwrap();
        let app = usage_item("微信", vec![("用户文件总占用", files)], vec!["打开目录"]);
        assert_eq!(app.categories.len(), 1);
        for category in app.categories {
            assert!(!category.path.to_ascii_lowercase().ends_with(".db"));
        }
    }
}
