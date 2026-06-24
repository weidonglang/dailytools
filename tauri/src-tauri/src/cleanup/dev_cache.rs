use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn command_spec(tool: &str) -> Option<(&'static str, &'static [&'static str])> {
    match tool {
        "npm" => Some(("npm", &["cache", "clean", "--force"])),
        "pnpm" => Some(("pnpm", &["store", "prune"])),
        "yarn" => Some(("yarn", &["cache", "clean"])),
        "pip" => Some(("python", &["-m", "pip", "cache", "purge"])),
        "uv" => Some(("uv", &["cache", "clean"])),
        "poetry" => Some(("poetry", &["cache", "clear", "pypi", "--all"])),
        "go-cache" => Some(("go", &["clean", "-cache"])),
        "go-modcache" => Some(("go", &["clean", "-modcache"])),
        "dotnet" => Some(("dotnet", &["nuget", "locals", "all", "--clear"])),
        _ => None,
    }
}

fn find_command(executable: &str, managed_root: &Path) -> Option<PathBuf> {
    let mut roots = vec![
        managed_root.join("current/node"),
        managed_root.join("current/python"),
        managed_root.join("current/python/Scripts"),
        managed_root.join("current/go/bin"),
    ];
    roots.extend(
        std::env::var_os("PATH")
            .map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
            .unwrap_or_default(),
    );
    for root in roots {
        for suffix in ["", ".exe", ".cmd", ".bat", ".com"] {
            let candidate = root.join(format!("{executable}{suffix}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn run_command(executable: &str, args: &[&str], managed_root: &Path) -> Result<String, String> {
    let resolved = find_command(executable, managed_root)
        .ok_or_else(|| format!("未找到 {executable}，请先安装并确保它位于 PATH 或受管运行时中"))?;
    let output = Command::new(&resolved)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                format!("未找到 {executable}，请先安装并确保它位于 PATH 中")
            } else {
                format!("启动 {executable} 失败：{error}")
            }
        })?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() {
        return Err(format!(
            "{} 清理失败：{}",
            executable,
            if stderr.is_empty() { stdout } else { stderr }
        ));
    }
    Ok(if stdout.is_empty() {
        format!("{executable} 官方缓存清理命令执行完成")
    } else {
        stdout
    })
}

pub fn clean_dev_cache(tool: &str, managed_root: &Path) -> Result<String, String> {
    let normalized = tool.trim().to_ascii_lowercase();
    let (executable, args) = command_spec(&normalized)
        .ok_or_else(|| "不支持该开发缓存；Maven、Gradle、Cargo 仅允许扫描".to_string())?;
    run_command(executable, args, managed_root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_command_missing_has_friendly_error() {
        let root = tempfile::tempdir().unwrap();
        let error = run_command(
            "devenv-manager-command-that-does-not-exist",
            &[],
            root.path(),
        )
        .unwrap_err();
        assert!(error.contains("未找到"));
        assert!(error.contains("PATH"));
    }

    #[test]
    fn direct_delete_tools_are_not_supported() {
        assert!(command_spec("maven").is_none());
        assert!(command_spec("gradle").is_none());
        assert!(command_spec("cargo").is_none());
    }
}
