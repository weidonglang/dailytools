use super::disk::inspect_disk_overview;
use super::model::{MaintenanceOverview, MemorySummary};
use super::scan::scan_cleanup_targets;
use std::fs;
use std::path::Path;
use sysinfo::System;

fn is_c_drive(drive: &str) -> bool {
    let value = drive.replace('/', "\\").to_ascii_lowercase();
    value == "c:" || value.starts_with("c:\\")
}

fn count_startup_items() -> usize {
    let mut roots = Vec::new();
    if let Some(roaming) = std::env::var_os("APPDATA") {
        roots.push(
            std::path::PathBuf::from(roaming)
                .join(r"Microsoft\Windows\Start Menu\Programs\Startup"),
        );
    }
    if let Some(program_data) = std::env::var_os("ProgramData") {
        roots.push(
            std::path::PathBuf::from(program_data)
                .join(r"Microsoft\Windows\Start Menu\Programs\StartUp"),
        );
    }
    roots
        .into_iter()
        .map(|root| {
            fs::read_dir(root)
                .map(|items| items.flatten().count())
                .unwrap_or(0)
        })
        .sum()
}

fn memory_summary() -> Option<MemorySummary> {
    let mut system = System::new();
    system.refresh_memory();
    let total = system.total_memory();
    if total == 0 {
        return None;
    }
    let used = system.used_memory().min(total);
    Some(MemorySummary {
        total_bytes: total,
        used_bytes: used,
        available_bytes: total.saturating_sub(used),
        used_percent: used as f64 / total as f64 * 100.0,
    })
}

pub fn inspect_maintenance_overview(managed_root: &Path) -> Result<MaintenanceOverview, String> {
    let volumes = inspect_disk_overview()?;
    let c_drive = volumes
        .iter()
        .find(|volume| is_c_drive(&volume.drive))
        .cloned()
        .unwrap_or_default();
    let report = scan_cleanup_targets(managed_root)?;
    let category_bytes = |id: &str| {
        report
            .categories
            .iter()
            .find(|item| item.id == id)
            .map(|item| item.total_bytes)
            .unwrap_or(0)
    };
    let safe_clean_estimate = report
        .categories
        .iter()
        .flat_map(|category| &category.items)
        .filter(|item| item.cleanable)
        .map(|item| item.size)
        .sum();
    let dev_cache_estimate = category_bytes("developer-caches");
    let move_estimate = 0;
    let large_file_count = report
        .categories
        .iter()
        .flat_map(|category| &category.items)
        .filter(|item| item.size >= 1024 * 1024 * 1024)
        .count();
    let risk_level = if c_drive.risk.is_empty() {
        "unknown".to_string()
    } else {
        c_drive.risk.clone()
    };
    let summary = match risk_level.as_str() {
        "critical" => "C 盘空间严重不足，建议尽快检查临时文件、开发缓存和可搬家目录。",
        "high" => "C 盘空间偏紧，建议优先查看安全清理估算与开发缓存。",
        "medium" => "C 盘空间需要关注，可以安排清理缓存或迁移大目录。",
        "low" => "C 盘空间目前健康，仍可定期扫描并按计划执行保守清理。",
        _ => "暂未识别 C 盘容量，请确认磁盘卷可被 Windows 正常读取。",
    }
    .to_string();
    let mut suggestions = vec![
        "先扫描并预览清理计划；Phase 2 只处理用户明确选择且再次校验通过的项目。".to_string(),
        "出于安全边界，默认扫描不会进入桌面、下载、文档或其他个人资料目录。".to_string(),
    ];
    if dev_cache_estimate > 1024 * 1024 * 1024 {
        suggestions.push("开发缓存超过 1 GB，清理前请确认近期构建不依赖离线缓存。".to_string());
    }
    Ok(MaintenanceOverview {
        c_drive,
        volumes,
        safe_clean_estimate,
        move_estimate,
        dev_cache_estimate,
        large_file_count,
        startup_count: count_startup_items(),
        memory_summary: memory_summary(),
        risk_level,
        summary,
        suggestions,
    })
}
