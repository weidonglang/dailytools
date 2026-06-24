use super::model::InstalledSoftwareUsage;
use std::collections::HashSet;

#[cfg(windows)]
fn read_uninstall_root(
    hive: winreg::HKEY,
    flags: u32,
    result: &mut Vec<InstalledSoftwareUsage>,
    seen: &mut HashSet<String>,
) {
    use winreg::enums::KEY_READ;
    use winreg::RegKey;

    let root = RegKey::predef(hive);
    let Ok(uninstall) = root.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
        KEY_READ | flags,
    ) else {
        return;
    };
    for key_name in uninstall.enum_keys().flatten() {
        let Ok(key) = uninstall.open_subkey_with_flags(&key_name, KEY_READ | flags) else {
            continue;
        };
        let name: String = key.get_value("DisplayName").unwrap_or_default();
        if name.trim().is_empty() || key.get_value::<u32, _>("SystemComponent").unwrap_or(0) == 1 {
            continue;
        }
        let publisher: String = key.get_value("Publisher").unwrap_or_default();
        let install_location: String = key.get_value("InstallLocation").unwrap_or_default();
        let uninstall_command: String = key
            .get_value("QuietUninstallString")
            .or_else(|_| key.get_value("UninstallString"))
            .unwrap_or_default();
        let estimated_kb = key
            .get_value::<u32, _>("EstimatedSize")
            .map(u64::from)
            .or_else(|_| {
                key.get_value::<String, _>("EstimatedSize")
                    .map(|value| value.parse::<u64>().unwrap_or(0))
            })
            .unwrap_or(0);
        let identity = format!(
            "{}\0{}",
            name.trim().to_ascii_lowercase(),
            install_location.trim().to_ascii_lowercase()
        );
        if !seen.insert(identity) {
            continue;
        }
        result.push(InstalledSoftwareUsage {
            name: name.trim().to_string(),
            publisher,
            install_location,
            estimated_size: estimated_kb.saturating_mul(1024),
            uninstall_command_exists: !uninstall_command.trim().is_empty(),
            suggestion: if uninstall_command.trim().is_empty() {
                "未登记卸载入口，请勿直接删除安装目录"
            } else {
                "如不再使用，请通过 Windows 已安装的应用执行卸载"
            }
            .to_string(),
        });
    }
}

pub fn inspect_installed_software_usage() -> Vec<InstalledSoftwareUsage> {
    #[cfg(windows)]
    {
        use winreg::enums::{
            HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
        };
        let mut result = Vec::new();
        let mut seen = HashSet::new();
        for (hive, flag) in [
            (HKEY_LOCAL_MACHINE, KEY_WOW64_64KEY),
            (HKEY_LOCAL_MACHINE, KEY_WOW64_32KEY),
            (HKEY_CURRENT_USER, KEY_WOW64_64KEY),
            (HKEY_CURRENT_USER, KEY_WOW64_32KEY),
        ] {
            read_uninstall_root(hive, flag, &mut result, &mut seen);
        }
        result.sort_by_key(|item| std::cmp::Reverse(item.estimated_size));
        result
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}
