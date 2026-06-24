use super::app_usage::usage_item;
use super::model::AppUsageItem;
use std::env;
use std::path::PathBuf;

pub fn inspect_game_platforms() -> Vec<AppUsageItem> {
    let program_files = env::var_os("ProgramFiles")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"));
    let program_files_x86 = env::var_os("ProgramFiles(x86)")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files (x86)"));
    let mut result = vec![
        usage_item(
            "Steam 游戏库",
            vec![(
                "游戏内容",
                program_files_x86.join(r"Steam\steamapps\common"),
            )],
            vec!["使用 Steam 存储管理器迁移游戏，不直接移动或删除目录"],
        ),
        usage_item(
            "Epic Games",
            vec![("游戏内容", program_files.join("Epic Games"))],
            vec!["使用 Epic Games Launcher 验证或迁移游戏"],
        ),
        usage_item(
            "WeGame",
            vec![
                ("游戏内容", PathBuf::from(r"C:\Program Files\WeGameApps")),
                ("游戏内容", PathBuf::from(r"D:\Program Files\WeGameApps")),
            ],
            vec!["使用 WeGame 客户端迁移游戏，避免破坏安装清单"],
        ),
    ];
    result.retain(|item| item.detected);
    result
}
