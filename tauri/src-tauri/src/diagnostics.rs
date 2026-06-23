use serde::Serialize;
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_ITEMS_PER_ROOT: usize = 200;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTraceReport {
    pub generated_at: String,
    pub items: Vec<AgentTraceItem>,
    pub privacy_notice: String,
    pub limitations: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTraceItem {
    pub source: String,
    pub path: String,
    pub evidence: String,
    pub confidence: String,
    pub recommendation: String,
}

fn display(path: &Path) -> String {
    path.display().to_string()
}

fn push_existing(items: &mut Vec<AgentTraceItem>, source: &str, path: PathBuf, evidence: &str) {
    if !path.exists() {
        return;
    }
    items.push(AgentTraceItem {
        source: source.to_string(),
        path: display(&path),
        evidence: evidence.to_string(),
        confidence: "可验证路径".to_string(),
        recommendation: "仅展示线索；请使用对应工具确认来源和卸载方式，不要直接删除目录。"
            .to_string(),
    });
}

fn list_tool_entries(items: &mut Vec<AgentTraceItem>, source: &str, root: PathBuf) {
    let mut stack = vec![(root, 0_usize)];
    let mut visited = 0_usize;
    while let Some((path, depth)) = stack.pop() {
        if visited >= MAX_ITEMS_PER_ROOT {
            break;
        }
        visited += 1;
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            if depth < 3 {
                if let Ok(entries) = fs::read_dir(&path) {
                    stack.extend(entries.flatten().map(|entry| (entry.path(), depth + 1)));
                }
            }
            continue;
        }
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !["exe", "cmd", "bat", "ps1"].contains(&extension.as_str()) {
            continue;
        }
        items.push(AgentTraceItem {
            source: source.to_string(),
            path: display(&path),
            evidence: "语言生态全局工具入口".to_string(),
            confidence: "高".to_string(),
            recommendation: format!("建议通过 {source} 查询包来源后再决定是否卸载。"),
        });
    }
}

pub fn inspect_agent_traces(project_path: Option<&Path>) -> AgentTraceReport {
    let mut items = Vec::new();
    if let Some(home) = dirs::home_dir() {
        for (source, relative) in [
            ("Claude Code", ".claude"),
            ("Codex", ".codex"),
            ("Cursor", ".cursor"),
            ("Continue", ".continue"),
            ("Aider", ".aider.conf.yml"),
        ] {
            push_existing(
                &mut items,
                source,
                home.join(relative),
                "发现 Agent/CLI 本地配置或状态路径；未读取会话正文",
            );
        }
        list_tool_entries(&mut items, "Cargo", home.join(".cargo/bin"));
        list_tool_entries(&mut items, "Go", home.join("go/bin"));
        list_tool_entries(
            &mut items,
            "Python user scripts",
            home.join("AppData/Roaming/Python"),
        );
        list_tool_entries(&mut items, ".NET tools", home.join(".dotnet/tools"));
    }
    if let Some(app_data) = env::var_os("APPDATA") {
        list_tool_entries(
            &mut items,
            "npm/pnpm global",
            PathBuf::from(app_data).join("npm"),
        );
    }
    if let Some(project) = project_path.filter(|path| path.is_dir()) {
        for relative in [
            "package.json",
            "pyproject.toml",
            "requirements.txt",
            "pom.xml",
            "build.gradle",
            "build.gradle.kts",
            "global.json",
            ".tool-versions",
            ".node-version",
            ".python-version",
            ".vscode/tasks.json",
        ] {
            push_existing(
                &mut items,
                "项目配置",
                project.join(relative),
                "项目文件可能记录 CLI/Agent 采用的工具或版本；未读取敏感值",
            );
        }
    }
    let mut seen = BTreeSet::new();
    items.retain(|item| seen.insert(item.path.to_ascii_lowercase()));
    AgentTraceReport {
        generated_at: format!("{:?}", std::time::SystemTime::now()),
        items,
        privacy_notice: "本报告只读取本地路径和文件名，不上传数据，不读取 shell history、会话正文、token 或密钥。".to_string(),
        limitations: vec![
            "只能展示仍有可验证痕迹的工具，无法凭空还原已删除的命令和文件。".to_string(),
            "路径存在不等于一定由 AI 安装，来源判断需要结合对应包管理器确认。".to_string(),
            "未知来源项目默认不给出自动删除操作。".to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_project_does_not_break_agent_trace_scan() {
        let report = inspect_agent_traces(Some(Path::new(r"Z:\definitely-missing")));
        assert!(!report.privacy_notice.is_empty());
        assert!(report.limitations.len() >= 2);
    }
}
