use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanRisk {
    Low,
    Medium,
    High,
    Critical,
}

impl CleanRisk {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

fn normalized(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

fn is_same_or_inside(path: &Path, root: &Path) -> bool {
    let path = normalized(path);
    let root = normalized(root);
    !root.is_empty() && (path == root || path.starts_with(&format!("{root}\\")))
}

fn user_home() -> Option<PathBuf> {
    dirs::home_dir()
}

#[allow(dead_code)]
pub fn is_inside_user_profile(path: &Path) -> bool {
    user_home().is_some_and(|home| is_same_or_inside(path, &home))
}

pub fn is_inside_managed_runtime(path: &Path) -> bool {
    let mut roots = Vec::new();
    if let Some(root) = env::var_os("DEVENV_HOME") {
        roots.push(PathBuf::from(root));
    }
    if let Some(home) = user_home() {
        roots.push(home.join("DevEnvManager"));
    }
    roots.push(PathBuf::from(r"D:\DevEnvManager"));
    roots.into_iter().any(|root| {
        [
            "current", "envs", "jdks", "pythons", "nodes", "mavens", "gradles", "gos", "tools",
        ]
        .iter()
        .any(|part| is_same_or_inside(path, &root.join(part)))
    })
}

pub fn is_protected_path(path: &Path) -> bool {
    let value = normalized(path);
    if value.is_empty() || value.len() <= 3 {
        return true;
    }
    let system_roots = [
        r"c:\windows",
        r"c:\program files",
        r"c:\program files (x86)",
        r"c:\programdata\microsoft",
        r"c:\$recycle.bin",
        r"c:\windows.old",
        r"c:\system volume information",
    ];
    if system_roots
        .iter()
        .any(|root| value == *root || value.starts_with(&format!("{root}\\")))
    {
        return true;
    }
    if let Some(home) = user_home() {
        if [
            "Documents",
            "Downloads",
            "Desktop",
            "Pictures",
            "Videos",
            "Music",
        ]
        .iter()
        .any(|name| is_same_or_inside(path, &home.join(name)))
        {
            return true;
        }
    }
    if [r"c:\hiberfil.sys", r"c:\pagefile.sys", r"c:\swapfile.sys"].contains(&value.as_str()) {
        return true;
    }
    if let Ok(project) = env::current_dir() {
        if is_same_or_inside(path, &project) {
            return true;
        }
    }
    is_inside_managed_runtime(path)
}

pub fn classify_path_risk(path: &Path) -> CleanRisk {
    let value = normalized(path);
    if value.contains("cookie")
        || value.contains("login data")
        || value.contains("password")
        || value.contains("\\wechat files\\")
        || value.contains("\\tencent files\\")
        || value.ends_with("\\msg")
    {
        CleanRisk::Critical
    } else if is_protected_path(path) || value.contains("$recycle.bin") {
        CleanRisk::High
    } else if value.contains("cache") || value.contains("temp") {
        CleanRisk::Medium
    } else {
        CleanRisk::Low
    }
}

pub(crate) fn is_sensitive_account_data(path: &Path) -> bool {
    let value = normalized(path);
    value.contains("cookie")
        || value.contains("login data")
        || value.contains("password")
        || value.contains("\\wechat files\\")
        || value.contains("\\tencent files\\")
        || value.contains("\\wechat\\")
        || value.contains("\\qq\\")
        || value.contains("\\google\\chrome\\")
        || value.contains("\\microsoft\\edge\\")
        || value.contains("\\mozilla\\firefox\\")
}

pub fn should_skip_path(path: &Path) -> Option<String> {
    let value = normalized(path);
    if value.contains("cookie") || value.contains("login data") || value.contains("password") {
        return Some("浏览器身份凭据受保护".to_string());
    }
    if value.contains("\\wechat files\\")
        || value.contains("\\tencent files\\")
        || value.contains("\\wechat\\")
        || value.contains("\\qq\\")
    {
        return Some("微信/QQ 用户数据库受保护".to_string());
    }
    if value.contains("\\google\\chrome\\")
        || value.contains("\\microsoft\\edge\\")
        || value.contains("\\mozilla\\firefox\\")
    {
        return Some("浏览器用户目录受保护".to_string());
    }
    if is_inside_managed_runtime(path) {
        return Some("DevEnv Manager 受管运行时受保护".to_string());
    }
    if is_protected_path(path) {
        return Some("系统、项目或用户关键目录受保护".to_string());
    }
    None
}

pub(crate) fn is_inside_root(path: &Path, root: &Path) -> bool {
    is_same_or_inside(path, root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_protected_windows_paths() {
        assert!(is_protected_path(Path::new(r"C:\Windows\System32")));
        assert!(is_protected_path(Path::new(r"C:\Program Files\Example")));
        assert!(is_protected_path(Path::new(r"C:\$Recycle.Bin")));
        assert!(!is_protected_path(Path::new(
            r"C:\Users\someone\AppData\Local\Temp\old.tmp"
        )));
    }

    #[test]
    fn managed_runtime_directories_are_excluded() {
        assert!(is_inside_managed_runtime(Path::new(
            r"D:\DevEnvManager\current\jdk\bin"
        )));
        assert!(should_skip_path(Path::new(r"D:\DevEnvManager\envs\python")).is_some());
        assert!(!is_inside_managed_runtime(Path::new(
            r"D:\DevEnvManager\downloads\jdk.zip"
        )));
    }

    #[test]
    fn user_sensitive_directories_and_credentials_are_protected() {
        if let Some(home) = dirs::home_dir() {
            assert!(is_protected_path(&home.join("Downloads")));
            assert!(is_protected_path(&home.join("Desktop")));
        }
        assert!(should_skip_path(Path::new(
            r"C:\Users\test\AppData\Local\Google\Chrome\User Data\Default\Cookies"
        ))
        .is_some());
        assert!(should_skip_path(Path::new(
            r"C:\Users\test\Documents\WeChat Files\wxid\Msg\MicroMsg.db"
        ))
        .is_some());
        assert!(should_skip_path(Path::new(
            r"C:\Users\test\AppData\Local\Microsoft\Edge\User Data\Default\Login Data"
        ))
        .is_some());
    }
}
