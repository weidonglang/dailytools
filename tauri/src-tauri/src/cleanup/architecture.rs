use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupArchitecture {
    pub schema_version: u32,
    pub status: &'static str,
    pub categories: Vec<CleanupCategory>,
    pub safety_rules: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupCategory {
    pub id: &'static str,
    pub name: &'static str,
    pub risk: &'static str,
    pub scan_only: bool,
    pub cleanup_enabled: bool,
    pub protected_patterns: Vec<&'static str>,
}

pub fn architecture() -> CleanupArchitecture {
    CleanupArchitecture {
        schema_version: 3,
        status: "scan-only-phase-1",
        categories: vec![
            CleanupCategory {
                id: "windows-temp",
                name: "C 盘临时文件",
                risk: "medium",
                scan_only: true,
                cleanup_enabled: false,
                protected_patterns: vec!["24 小时内文件", "正在使用的文件", "Windows 系统目录"],
            },
            CleanupCategory {
                id: "system-caches",
                name: "Windows 系统缓存",
                risk: "high",
                scan_only: true,
                cleanup_enabled: false,
                protected_patterns: vec!["回收站", "错误报告", "缩略图", "Shader Cache"],
            },
            CleanupCategory {
                id: "developer-caches",
                name: "开发缓存",
                risk: "medium",
                scan_only: true,
                cleanup_enabled: false,
                protected_patterns: vec!["项目源码", "受管运行时", "工具配置"],
            },
            CleanupCategory {
                id: "devenv-manager",
                name: "DevEnv Manager",
                risk: "medium",
                scan_only: true,
                cleanup_enabled: false,
                protected_patterns: vec!["受管运行时", "config", "current", "envs"],
            },
            CleanupCategory {
                id: "wps-cache",
                name: "WPS 临时文件与日志",
                risk: "high",
                scan_only: true,
                cleanup_enabled: false,
                protected_patterns: vec!["文档", "云同步", "备份中心", "账号数据"],
            },
            CleanupCategory {
                id: "recycle-bin",
                name: "Windows 回收站",
                risk: "high",
                scan_only: true,
                cleanup_enabled: false,
                protected_patterns: vec!["本程序不清空回收站"],
            },
        ],
        safety_rules: vec![
            "Phase 1 仅扫描和统计，不删除、移动或修改任何文件",
            "系统目录、用户文档、当前项目和受管运行时始终受保护",
            "默认扫描不进入桌面、下载、文档、图片、视频或音乐目录",
            "回收站仅统计容量，本程序不会清空回收站",
            "浏览器 Cookie、登录数据和密码存储不会进入扫描结果",
            "微信、QQ 数据库和符号链接会被跳过",
            "权限不足或扫描上限触发时只记录警告，不尝试提权",
        ],
    }
}
