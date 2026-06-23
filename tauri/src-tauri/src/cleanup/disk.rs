use super::model::DiskVolumeInfo;
use sysinfo::Disks;

pub fn risk_for_space(total_bytes: u64, free_bytes: u64) -> String {
    if total_bytes == 0 {
        return "unknown".to_string();
    }
    let free_percent = free_bytes as f64 / total_bytes as f64 * 100.0;
    if free_bytes < 5 * 1024 * 1024 * 1024 || free_percent < 5.0 {
        "critical"
    } else if free_bytes < 15 * 1024 * 1024 * 1024 || free_percent < 10.0 {
        "high"
    } else if free_bytes < 30 * 1024 * 1024 * 1024 || free_percent < 20.0 {
        "medium"
    } else {
        "low"
    }
    .to_string()
}

pub fn inspect_disk_overview() -> Result<Vec<DiskVolumeInfo>, String> {
    let disks = Disks::new_with_refreshed_list();
    let mut volumes = disks
        .list()
        .iter()
        .filter_map(|disk| {
            let mount = disk.mount_point().to_string_lossy().to_string();
            let total = disk.total_space();
            if mount.is_empty() || total == 0 {
                return None;
            }
            let free = disk.available_space().min(total);
            let used = total.saturating_sub(free);
            Some(DiskVolumeInfo {
                drive: mount,
                total_bytes: total,
                free_bytes: free,
                used_bytes: used,
                used_percent: used as f64 / total as f64 * 100.0,
                file_system: disk.file_system().to_str().map(str::to_string),
                risk: risk_for_space(total, free),
            })
        })
        .collect::<Vec<_>>();
    volumes.sort_by(|a, b| {
        a.drive
            .to_ascii_lowercase()
            .cmp(&b.drive.to_ascii_lowercase())
    });
    Ok(volumes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_drive_risk_levels_follow_free_space_thresholds() {
        let gib = 1024_u64.pow(3);
        assert_eq!(risk_for_space(100 * gib, 4 * gib), "critical");
        assert_eq!(risk_for_space(100 * gib, 9 * gib), "high");
        assert_eq!(risk_for_space(100 * gib, 18 * gib), "medium");
        assert_eq!(risk_for_space(100 * gib, 40 * gib), "low");
    }
}
