use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use tauri::Emitter;
use tempfile::Builder as TempBuilder;
use zip::ZipArchive;

#[cfg(windows)]
use winreg::{enums::*, RegKey};

const APP_NAME: &str = "DevEnvManager";
const MANAGED_PATHS: [&str; 7] = [
    r"%DEVENV_HOME%\current\jdk\bin",
    r"%DEVENV_HOME%\current\python",
    r"%DEVENV_HOME%\current\python\Scripts",
    r"%DEVENV_HOME%\current\node",
    r"%DEVENV_HOME%\current\maven\bin",
    r"%DEVENV_HOME%\current\gradle\bin",
    r"%DEVENV_HOME%\tools\npm-global",
];
const BLOCKED_PIDS: [u32; 2] = [0, 4];
const BLOCKED_NAMES: [&str; 9] = [
    "system",
    "idle",
    "registry",
    "smss.exe",
    "csrss.exe",
    "wininit.exe",
    "winlogon.exe",
    "services.exe",
    "lsass.exe",
];
const CAUTION_NAMES: [&str; 1] = ["svchost.exe"];
const ALLOWED_DOWNLOAD_HOSTS: [&str; 11] = [
    "api.adoptium.net",
    "github.com",
    "objects.githubusercontent.com",
    "release-assets.githubusercontent.com",
    "nodejs.org",
    "www.python.org",
    "python.org",
    "downloads.apache.org",
    "archive.apache.org",
    "services.gradle.org",
    "downloads.gradle.org",
];

#[derive(Debug, Clone)]
struct AppPaths {
    root: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Settings {
    root_dir: String,
    auto_check_update: bool,
    download_timeout_seconds: u64,
    theme: String,
    last_page: String,
    update_manifest_url: String,
    port_process_exclusions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct InstalledData {
    jdks: Vec<Value>,
    pythons: Vec<Value>,
    nodes: Vec<Value>,
    mavens: Vec<Value>,
    gradles: Vec<Value>,
    current: CurrentVersions,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct CurrentVersions {
    jdk: Option<String>,
    python: Option<String>,
    node: Option<String>,
    maven: Option<String>,
    gradle: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PathSummary {
    root: String,
    envs: String,
    jdks: String,
    pythons: String,
    nodes: String,
    mavens: String,
    gradles: String,
    current: String,
    downloads: String,
    config: String,
    logs: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigView {
    settings: Settings,
    installed: InstalledData,
    paths: PathSummary,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OperationResult {
    success: bool,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct KillResult {
    success: bool,
    message: String,
    needs_force: bool,
    blocked: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppSnapshot {
    default_root: String,
    config_dir: String,
    os: String,
    arch: String,
    username: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvSnapshot {
    path_entries: Vec<String>,
    java_home: Option<String>,
    devenv_home: Option<String>,
    path_warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeInfo {
    kind: String,
    version: String,
    executable: String,
    source: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PortRecord {
    protocol: String,
    local_address: String,
    local_port: u16,
    remote_address: String,
    state: String,
    pid: u32,
    process_name: String,
    risk: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectHealth {
    root: String,
    project_types: Vec<String>,
    signals: Vec<String>,
    suggestions: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NetworkCheck {
    name: String,
    url: String,
    success: bool,
    status: String,
    elapsed_ms: u128,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NetworkDiagnostics {
    checks: Vec<NetworkCheck>,
    proxy: Vec<(String, String)>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CacheEntry {
    name: String,
    path: String,
    size: u64,
    sha256: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CommandRunResult {
    success: bool,
    return_code: i32,
    output: String,
    elapsed_ms: u128,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct TaskProgress {
    task: String,
    percent: u8,
    message: String,
}

#[derive(Debug)]
struct RuntimeMeta {
    kind: &'static str,
    collection: &'static str,
    link_name: &'static str,
    exe_key: &'static str,
}

#[tauri::command]
fn app_snapshot() -> AppSnapshot {
    let paths = load_paths().unwrap_or_else(|_| AppPaths::new(default_root_dir()));
    AppSnapshot {
        default_root: display_path(&paths.root),
        config_dir: display_path(app_config_dir()),
        os: env::consts::OS.to_string(),
        arch: env::consts::ARCH.to_string(),
        username: env::var("USERNAME")
            .or_else(|_| env::var("USER"))
            .unwrap_or_else(|_| "unknown".to_string()),
    }
}

#[tauri::command]
fn load_config() -> Result<ConfigView, String> {
    let settings = load_settings()?;
    let paths = AppPaths::new(PathBuf::from(&settings.root_dir));
    paths.ensure().map_err(|err| err.to_string())?;
    let installed = load_installed(&paths)?;
    Ok(ConfigView {
        settings,
        installed,
        paths: paths.summary(),
    })
}

#[tauri::command]
fn set_root_dir(root: String) -> Result<ConfigView, String> {
    let root = PathBuf::from(root.trim());
    if root.as_os_str().is_empty() {
        return Err("根目录不能为空".to_string());
    }
    let root = root
        .canonicalize()
        .unwrap_or_else(|_| root.expand_home().unwrap_or(root));
    let mut settings = load_settings()?;
    settings.root_dir = display_path(&root);
    save_json(&settings_file(), &settings)?;
    let paths = AppPaths::new(root);
    paths.ensure().map_err(|err| err.to_string())?;
    ensure_installed(&paths)?;
    load_config()
}

#[tauri::command]
fn env_snapshot() -> EnvSnapshot {
    let paths = load_paths().unwrap_or_else(|_| AppPaths::new(default_root_dir()));
    let user_env = user_environment().unwrap_or_default();
    let path_value = user_env
        .get("Path")
        .or_else(|| user_env.get("PATH"))
        .cloned()
        .unwrap_or_else(|| env::var("PATH").unwrap_or_default());
    let path_entries: Vec<String> = path_value
        .split(';')
        .filter(|item| !item.trim().is_empty())
        .map(|item| item.trim().to_string())
        .collect();
    let path_warnings = inspect_path_entries(&path_entries, &paths);

    EnvSnapshot {
        path_entries,
        java_home: user_env.get("JAVA_HOME").cloned().or_else(|| env::var("JAVA_HOME").ok()),
        devenv_home: user_env.get("DEVENV_HOME").cloned().or_else(|| env::var("DEVENV_HOME").ok()),
        path_warnings,
    }
}

#[tauri::command]
fn configure_user_environment() -> Result<OperationResult, String> {
    let paths = load_paths()?;
    paths.ensure().map_err(|err| err.to_string())?;
    let environment = user_environment()?;
    let old_path = environment
        .get("Path")
        .or_else(|| environment.get("PATH"))
        .cloned()
        .unwrap_or_default();
    let backup = json!({
        "created_at": current_timestamp(),
        "DEVENV_HOME": environment.get("DEVENV_HOME"),
        "JAVA_HOME": environment.get("JAVA_HOME"),
        "Path": old_path,
    });
    save_json(&paths.env_backup_file(), &backup)?;
    let selected_java_home = select_java_home(&paths, &environment);
    set_user_environment_values(&paths, selected_java_home.as_deref(), &merge_path(&old_path))?;
    broadcast_environment_change();
    Ok(OperationResult {
        success: true,
        message: selected_java_home
            .map(|value| format!("已配置用户环境变量，JAVA_HOME = {value}"))
            .unwrap_or_else(|| "已配置用户环境变量，未发现可用 JAVA_HOME".to_string()),
    })
}

#[tauri::command]
fn restore_user_environment() -> Result<OperationResult, String> {
    let paths = load_paths()?;
    let backup: Value = read_json(&paths.env_backup_file())?;
    let path = backup.get("Path").and_then(Value::as_str).unwrap_or("");
    let devenv_home = backup.get("DEVENV_HOME").and_then(Value::as_str);
    let java_home = backup.get("JAVA_HOME").and_then(Value::as_str);
    restore_environment_values(devenv_home, java_home, path)?;
    broadcast_environment_change();
    Ok(OperationResult {
        success: true,
        message: "已恢复上一次备份的用户环境变量".to_string(),
    })
}

#[tauri::command]
fn discover_runtimes() -> Vec<RuntimeInfo> {
    let mut runtimes = Vec::new();
    for (kind, exe, args) in [
        ("Java", "java", vec!["-version"]),
        ("Python", "python", vec!["--version"]),
        ("Python Launcher", "py", vec!["--version"]),
        ("Node.js", "node", vec!["--version"]),
        ("npm", "npm", vec!["--version"]),
        ("Maven", "mvn", vec!["--version"]),
        ("Gradle", "gradle", vec!["--version"]),
    ] {
        if let Some(info) = detect_runtime(kind, exe, &args) {
            runtimes.push(info);
        }
    }
    runtimes
}

#[tauri::command]
fn install_jdk(app: tauri::AppHandle, version: String) -> Result<OperationResult, String> {
    let version = version.trim();
    let task = format!("JDK {version}");
    emit_task_progress(&app, &task, 2, "正在准备安装");
    if !["8", "11", "17", "21", "25"].contains(&version) {
        return Err(format!("暂不支持 JDK {version}"));
    }
    let paths = load_paths()?;
    paths.ensure().map_err(|err| err.to_string())?;
    emit_task_progress(&app, &task, 8, "正在查询 Adoptium");
    let release = resolve_jdk_release(version)?;
    let archive = paths.downloads().join(&release.name);
    let target = paths.jdks().join(format!("temurin-{version}"));
    paths.assert_inside_root(&target)?;
    if target.exists() {
        return Err(format!("JDK {version} 已安装：{}", display_path(&target)));
    }
    emit_task_progress(&app, &task, 18, "正在下载 JDK");
    download_file(&release.url, &archive, release.sha256.as_deref())?;
    emit_task_progress(&app, &task, 70, "正在解压 JDK");
    install_zip_payload(&archive, &target, &["bin/java.exe", "bin/javac.exe"])?;
    emit_task_progress(&app, &task, 88, "正在验证 JDK");
    let output = run_command_output(target.join("bin/java.exe"), &["-version"], 30)?;
    run_command_output(target.join("bin/javac.exe"), &["-version"], 30)?;
    record_install(
        &paths,
        runtime_meta("jdk")?,
        version,
        &target,
        &target.join("bin/java.exe"),
        json!({
            "distribution": "temurin",
            "detail": output.lines().next().unwrap_or(""),
        }),
    )?;
    switch_runtime("jdk".to_string(), version.to_string())?;
    refresh_user_java_home(&paths)?;
    emit_task_progress(&app, &task, 100, "安装完成");
    Ok(OperationResult {
        success: true,
        message: format!("安装成功 JDK {version}"),
    })
}

#[tauri::command]
fn install_node(app: tauri::AppHandle, version: String) -> Result<OperationResult, String> {
    let version = version.trim();
    let task = format!("Node.js {version}");
    emit_task_progress(&app, &task, 2, "正在准备安装");
    if !["16", "18", "20", "22", "24"].contains(&version) {
        return Err(format!("暂不支持 Node.js {version}"));
    }
    let paths = load_paths()?;
    paths.ensure().map_err(|err| err.to_string())?;
    emit_task_progress(&app, &task, 8, "正在查询 Node.js 官方版本");
    let release = resolve_node_release(version)?;
    let checksum = resolve_node_checksum(&release)?;
    let archive = paths.downloads().join(&release.name);
    let target = paths.nodes().join(format!("node-{version}"));
    paths.assert_inside_root(&target)?;
    if target.exists() {
        return Err(format!("Node.js {version} 已安装：{}", display_path(&target)));
    }
    emit_task_progress(&app, &task, 18, "正在下载 Node.js");
    download_file(&release.url, &archive, checksum.as_deref())?;
    emit_task_progress(&app, &task, 70, "正在解压 Node.js");
    install_zip_payload(&archive, &target, &["node.exe", "npm.cmd", "npx.cmd"])?;
    emit_task_progress(&app, &task, 88, "正在验证 Node.js");
    let output = run_command_output(target.join("node.exe"), &["-v"], 30)?;
    run_command_output(target.join("npm.cmd"), &["-v"], 30)?;
    record_install(
        &paths,
        runtime_meta("node")?,
        version,
        &target,
        &target.join("node.exe"),
        json!({
            "detail": output.lines().next().unwrap_or(&release.tag),
            "tag": release.tag,
        }),
    )?;
    switch_runtime("node".to_string(), version.to_string())?;
    emit_task_progress(&app, &task, 100, "安装完成");
    Ok(OperationResult {
        success: true,
        message: format!("安装成功 Node.js {version}"),
    })
}

#[tauri::command]
fn install_python(app: tauri::AppHandle, version: String) -> Result<OperationResult, String> {
    let version = version.trim();
    let task = format!("Python {version}");
    emit_task_progress(&app, &task, 2, "正在准备安装");
    if !["3.9", "3.10", "3.11", "3.12", "3.13", "3.14"].contains(&version) {
        return Err(format!("暂不支持 Python {version}"));
    }
    let paths = load_paths()?;
    paths.ensure().map_err(|err| err.to_string())?;
    emit_task_progress(&app, &task, 8, "正在查询 Python 官方版本");
    let release = resolve_python_release(version)?;
    let installer = paths.downloads().join(&release.name);
    let target = paths.pythons().join(format!("python-{version}"));
    paths.assert_inside_root(&target)?;
    if target.exists() {
        return Err(format!("Python {version} 已安装：{}", display_path(&target)));
    }
    emit_task_progress(&app, &task, 20, "正在下载 Python 安装器");
    download_file(&release.url, &installer, None)?;
    emit_task_progress(&app, &task, 62, "正在静默安装 Python");
    let output = Command::new(&installer)
        .args([
            "/quiet",
            "InstallAllUsers=0",
            &format!("TargetDir={}", display_path(&target)),
            "PrependPath=0",
            "AppendPath=0",
            "Include_launcher=0",
            "Include_pip=1",
            "Include_test=0",
            "Include_doc=0",
        ])
        .output()
        .map_err(|err| format!("Python 安装器执行失败：{err}"))?;
    if !output.status.success() {
        return Err(format!(
            "Python 安装失败，退出码 {:?}\n{}",
            output.status.code(),
            command_text(&output.stdout, &output.stderr)
        ));
    }
    let python_exe = target.join("python.exe");
    emit_task_progress(&app, &task, 88, "正在验证 Python 和 pip");
    let verify = run_command_output(python_exe.clone(), &["--version"], 30)?;
    run_command_output(python_exe.clone(), &["-m", "pip", "--version"], 30)?;
    record_install(
        &paths,
        runtime_meta("python")?,
        version,
        &target,
        &python_exe,
        json!({
            "detail": verify.lines().next().unwrap_or(&release.tag),
            "install_mode": "installer",
            "installer": display_path(&installer),
        }),
    )?;
    switch_runtime("python".to_string(), version.to_string())?;
    emit_task_progress(&app, &task, 100, "安装完成");
    Ok(OperationResult {
        success: true,
        message: format!("安装成功 Python {version}"),
    })
}

#[tauri::command]
fn install_maven_latest(app: tauri::AppHandle) -> Result<OperationResult, String> {
    let task = "Maven".to_string();
    emit_task_progress(&app, &task, 3, "正在查询 Maven 版本");
    let paths = load_paths()?;
    paths.ensure().map_err(|err| err.to_string())?;
    let release = resolve_maven_release()?;
    let archive = paths.downloads().join(&release.name);
    let target = paths.mavens().join(format!("maven-{}", release.tag));
    paths.assert_inside_root(&target)?;
    if target.exists() {
        return Err(format!("Maven {} 已安装：{}", release.tag, display_path(&target)));
    }
    emit_task_progress(&app, &task, 18, "正在下载 Maven");
    download_file(&release.url, &archive, None)?;
    emit_task_progress(&app, &task, 72, "正在解压 Maven");
    install_zip_payload(&archive, &target, &["bin/mvn.cmd"])?;
    emit_task_progress(&app, &task, 88, "正在验证 Maven");
    let output = run_command_output(target.join("bin/mvn.cmd"), &["-v"], 60)?;
    record_install(
        &paths,
        runtime_meta("maven")?,
        &release.tag,
        &target,
        &target.join("bin/mvn.cmd"),
        json!({ "detail": output.lines().next().unwrap_or("") }),
    )?;
    switch_runtime("maven".to_string(), release.tag.clone())?;
    emit_task_progress(&app, &task, 100, "安装完成");
    Ok(OperationResult {
        success: true,
        message: format!("安装成功 Maven {}", release.tag),
    })
}

#[tauri::command]
fn install_gradle_latest(app: tauri::AppHandle) -> Result<OperationResult, String> {
    let task = "Gradle".to_string();
    emit_task_progress(&app, &task, 3, "正在查询 Gradle 版本");
    let paths = load_paths()?;
    paths.ensure().map_err(|err| err.to_string())?;
    let release = resolve_gradle_release()?;
    let archive = paths.downloads().join(&release.name);
    let target = paths.gradles().join(format!("gradle-{}", release.tag));
    paths.assert_inside_root(&target)?;
    if target.exists() {
        return Err(format!("Gradle {} 已安装：{}", release.tag, display_path(&target)));
    }
    emit_task_progress(&app, &task, 18, "正在下载 Gradle");
    download_file(&release.url, &archive, release.sha256.as_deref())?;
    emit_task_progress(&app, &task, 72, "正在解压 Gradle");
    install_zip_payload(&archive, &target, &["bin/gradle.bat"])?;
    emit_task_progress(&app, &task, 88, "正在验证 Gradle");
    let output = run_command_output(target.join("bin/gradle.bat"), &["-v"], 120)?;
    record_install(
        &paths,
        runtime_meta("gradle")?,
        &release.tag,
        &target,
        &target.join("bin/gradle.bat"),
        json!({ "detail": output.lines().next().unwrap_or("") }),
    )?;
    switch_runtime("gradle".to_string(), release.tag.clone())?;
    emit_task_progress(&app, &task, 100, "安装完成");
    Ok(OperationResult {
        success: true,
        message: format!("安装成功 Gradle {}", release.tag),
    })
}

#[tauri::command]
fn switch_runtime(kind: String, version: String) -> Result<OperationResult, String> {
    let paths = load_paths()?;
    let meta = runtime_meta(&kind)?;
    let mut installed = load_installed(&paths)?;
    let record = collection(&installed, meta.collection)
        .iter()
        .find(|item| item.get("version").and_then(Value::as_str) == Some(version.as_str()))
        .cloned()
        .ok_or_else(|| format!("尚未安装 {} {}", meta.kind, version))?;
    let target = PathBuf::from(record.get("path").and_then(Value::as_str).unwrap_or(""));
    if !target.exists() {
        return Err(format!("版本目录不存在：{}", display_path(&target)));
    }
    switch_junction(&paths.current().join(meta.link_name), &target, &paths.root)?;
    set_current(&mut installed, meta.kind, Some(version.clone()));
    save_json(&paths.installed_file(), &installed)?;
    if meta.kind == "jdk" {
        refresh_user_java_home(&paths)?;
    }
    Ok(OperationResult {
        success: true,
        message: format!("已切换当前 {} 到 {}", meta.kind, version),
    })
}

#[tauri::command]
fn uninstall_runtime(kind: String, version: String) -> Result<OperationResult, String> {
    let paths = load_paths()?;
    let meta = runtime_meta(&kind)?;
    let mut installed = load_installed(&paths)?;
    let records = collection_mut(&mut installed, meta.collection);
    let index = records
        .iter()
        .position(|item| item.get("version").and_then(Value::as_str) == Some(version.as_str()))
        .ok_or_else(|| format!("未找到 DevEnv 管理的 {} {}", meta.kind, version))?;
    let record = records[index].clone();
    let target = PathBuf::from(record.get("path").and_then(Value::as_str).unwrap_or(""));
    let expected_parent = runtime_parent(&paths, meta.collection)?;
    if target.parent() != Some(expected_parent.as_path()) {
        return Err(format!("拒绝删除非标准受管目录：{}", display_path(&target)));
    }
    if current_version(&installed, meta.kind).as_deref() == Some(version.as_str()) {
        remove_junction(&paths.current().join(meta.link_name))?;
        set_current(&mut installed, meta.kind, None);
    }
    if target.exists() {
        fs::remove_dir_all(&target).map_err(|err| format!("删除版本目录失败：{err}"))?;
    }
    collection_mut(&mut installed, meta.collection).remove(index);
    save_json(&paths.installed_file(), &installed)?;
    if meta.kind == "jdk" {
        refresh_user_java_home(&paths)?;
    }
    Ok(OperationResult {
        success: true,
        message: format!("已卸载 {} {}", meta.kind, version),
    })
}

#[tauri::command]
fn kill_process(pid: u32, force: bool, allow_caution: bool) -> KillResult {
    if BLOCKED_PIDS.contains(&pid) {
        return KillResult {
            success: false,
            message: format!("PID {pid} 是受保护的系统进程"),
            needs_force: false,
            blocked: true,
        };
    }
    let system = sysinfo::System::new_all();
    let name = process_name(&system, pid);
    let lower = name.to_ascii_lowercase();
    if BLOCKED_NAMES.contains(&lower.as_str()) {
        return KillResult {
            success: false,
            message: format!("{name} 是受保护的关键系统进程"),
            needs_force: false,
            blocked: true,
        };
    }
    if CAUTION_NAMES.contains(&lower.as_str()) && !allow_caution {
        return KillResult {
            success: false,
            message: format!("{name} 需要额外确认"),
            needs_force: false,
            blocked: true,
        };
    }

    let mut args = vec!["/PID".to_string(), pid.to_string(), "/T".to_string()];
    if force {
        args.push("/F".to_string());
    }
    let output = Command::new("taskkill").args(&args).output();
    match output {
        Ok(done) if done.status.success() => KillResult {
            success: true,
            message: if force {
                format!("已强制结束 PID {pid} / {name}")
            } else {
                format!("已结束 PID {pid} / {name}")
            },
            needs_force: false,
            blocked: false,
        },
        Ok(done) => {
            let text = command_text(&done.stdout, &done.stderr);
            KillResult {
                success: false,
                message: if force {
                    format!("结束进程失败：{text}")
                } else {
                    format!("PID {pid} 未正常退出，可尝试强制结束：{text}")
                },
                needs_force: !force,
                blocked: false,
            }
        }
        Err(err) => KillResult {
            success: false,
            message: format!("结束进程失败：{err}"),
            needs_force: false,
            blocked: false,
        },
    }
}

#[tauri::command]
fn scan_ports() -> Result<Vec<PortRecord>, String> {
    let output = Command::new("netstat")
        .args(["-ano"])
        .output()
        .map_err(|err| format!("无法执行 netstat: {err}"))?;

    let text = String::from_utf8_lossy(&output.stdout);
    let system = sysinfo::System::new_all();
    let mut records = Vec::new();

    for line in text.lines() {
        let columns: Vec<&str> = line.split_whitespace().collect();
        if columns.len() < 4 {
            continue;
        }
        let protocol = columns[0].to_ascii_uppercase();
        if protocol != "TCP" && protocol != "UDP" {
            continue;
        }

        let (local, remote, state, pid_text) = if protocol == "TCP" && columns.len() >= 5 {
            (columns[1], columns[2], columns[3].to_string(), columns[4])
        } else if protocol == "UDP" && columns.len() >= 4 {
            (columns[1], columns[2], "LISTENING".to_string(), columns[3])
        } else {
            continue;
        };

        let Some((local_address, local_port)) = parse_socket(local) else {
            continue;
        };
        let pid = pid_text.parse::<u32>().unwrap_or(0);
        let process_name = process_name(&system, pid);
        let risk = classify_port(local_port, pid, &process_name);

        records.push(PortRecord {
            protocol,
            local_address,
            local_port,
            remote_address: remote.to_string(),
            state,
            pid,
            process_name,
            risk,
        });
    }

    records.sort_by(|a, b| {
        a.local_port
            .cmp(&b.local_port)
            .then(a.protocol.cmp(&b.protocol))
            .then(a.pid.cmp(&b.pid))
    });
    Ok(records)
}

#[tauri::command]
fn project_health(path: String) -> Result<ProjectHealth, String> {
    let root = PathBuf::from(path.trim());
    if !root.exists() {
        return Err("项目目录不存在".to_string());
    }
    if !root.is_dir() {
        return Err("请选择目录而不是文件".to_string());
    }

    let mut project_types = Vec::new();
    let mut signals = Vec::new();
    let mut suggestions = Vec::new();

    let checks = [
        ("package.json", "Node.js", "运行 npm install 安装依赖"),
        ("pyproject.toml", "Python", "创建虚拟环境并安装项目依赖"),
        ("requirements.txt", "Python", "运行 pip install -r requirements.txt"),
        ("pom.xml", "Maven", "运行 mvn test 验证项目"),
        ("build.gradle", "Gradle", "运行 gradle test 验证项目"),
        ("Cargo.toml", "Rust", "运行 cargo test 验证项目"),
        ("src-tauri/tauri.conf.json", "Tauri", "运行 npm run tauri:dev 启动桌面应用"),
    ];

    for (file, kind, suggestion) in checks {
        if root.join(file).exists() {
            if !project_types.iter().any(|item| item == kind) {
                project_types.push(kind.to_string());
            }
            signals.push(file.to_string());
            suggestions.push(suggestion.to_string());
        }
    }

    if root.join(".venv").exists() {
        signals.push(".venv".to_string());
    }
    if root.join("node_modules").exists() {
        signals.push("node_modules".to_string());
    }

    project_types.sort();
    suggestions.sort();
    suggestions.dedup();

    Ok(ProjectHealth {
        root: display_path(root),
        project_types,
        signals,
        suggestions,
    })
}

#[tauri::command]
fn network_diagnostics() -> NetworkDiagnostics {
    let endpoints = [
        ("GitHub", "https://github.com"),
        ("Python 官网", "https://www.python.org"),
        ("Node.js 官网", "https://nodejs.org/dist/index.json"),
        (
            "Adoptium API",
            "https://api.adoptium.net/v3/info/available_releases",
        ),
        ("Apache Maven", "https://downloads.apache.org/maven/maven-3/"),
        ("Gradle", "https://services.gradle.org/versions/current"),
    ];
    let client = reqwest::blocking::Client::builder()
        .user_agent("DevEnvManager/2.0")
        .timeout(std::time::Duration::from_secs(15))
        .build();
    let checks = endpoints
        .into_iter()
        .map(|(name, url)| {
            let started = Instant::now();
            match &client {
                Ok(client) => match client.get(url).send() {
                    Ok(response) => NetworkCheck {
                        name: name.to_string(),
                        url: url.to_string(),
                        success: response.status().is_success(),
                        status: response.status().as_u16().to_string(),
                        elapsed_ms: started.elapsed().as_millis(),
                    },
                    Err(err) => NetworkCheck {
                        name: name.to_string(),
                        url: url.to_string(),
                        success: false,
                        status: network_error(&err),
                        elapsed_ms: started.elapsed().as_millis(),
                    },
                },
                Err(err) => NetworkCheck {
                    name: name.to_string(),
                    url: url.to_string(),
                    success: false,
                    status: err.to_string(),
                    elapsed_ms: started.elapsed().as_millis(),
                },
            }
        })
        .collect();
    NetworkDiagnostics {
        checks,
        proxy: proxy_state(),
    }
}

#[tauri::command]
fn cache_entries(calculate_hash: bool) -> Result<Vec<CacheEntry>, String> {
    let paths = load_paths()?;
    fs::create_dir_all(paths.downloads()).map_err(|err| format!("创建缓存目录失败：{err}"))?;
    let mut entries = Vec::new();
    for item in fs::read_dir(paths.downloads()).map_err(|err| format!("读取缓存目录失败：{err}"))? {
        let path = item.map_err(|err| err.to_string())?.path();
        if !path.is_file() {
            continue;
        }
        entries.push(CacheEntry {
            name: path
                .file_name()
                .and_then(OsStr::to_str)
                .unwrap_or("")
                .to_string(),
            size: path.metadata().map(|meta| meta.len()).unwrap_or(0),
            sha256: if calculate_hash {
                file_sha256(&path).ok()
            } else {
                None
            },
            path: display_path(&path),
        });
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

#[tauri::command]
fn clear_download_cache() -> Result<OperationResult, String> {
    let paths = load_paths()?;
    let downloads = paths.downloads();
    fs::create_dir_all(&downloads).map_err(|err| format!("创建缓存目录失败：{err}"))?;
    let mut count = 0_u64;
    let mut size = 0_u64;
    for item in fs::read_dir(&downloads).map_err(|err| format!("读取缓存目录失败：{err}"))? {
        let path = item.map_err(|err| err.to_string())?.path();
        if !path.is_file() {
            continue;
        }
        paths.assert_inside_root(&path)?;
        if path.parent() != Some(downloads.as_path()) {
            continue;
        }
        size += path.metadata().map(|meta| meta.len()).unwrap_or(0);
        fs::remove_file(path).map_err(|err| format!("删除缓存失败：{err}"))?;
        count += 1;
    }
    Ok(OperationResult {
        success: true,
        message: format!("已清理 {count} 个缓存文件，释放 {}", format_size(size)),
    })
}

#[tauri::command]
fn run_tool_command(command: String, cwd: Option<String>) -> Result<CommandRunResult, String> {
    let parts = parse_command_line(&command)?;
    let executable = parts.first().ok_or_else(|| "命令不能为空".to_string())?;
    let started = Instant::now();
    let mut cmd = Command::new(executable);
    cmd.args(parts.iter().skip(1));
    if let Some(cwd) = cwd.filter(|item| !item.trim().is_empty()) {
        cmd.current_dir(cwd);
    }
    let output = cmd.output().map_err(|err| format!("执行命令失败：{err}"))?;
    Ok(CommandRunResult {
        success: output.status.success(),
        return_code: output.status.code().unwrap_or(-1),
        output: command_text(&output.stdout, &output.stderr),
        elapsed_ms: started.elapsed().as_millis(),
    })
}

#[tauri::command]
fn generate_vscode_config(project_path: String) -> Result<OperationResult, String> {
    let root = PathBuf::from(project_path.trim());
    if !root.is_dir() {
        return Err("项目目录不存在".to_string());
    }
    let vscode = root.join(".vscode");
    fs::create_dir_all(&vscode).map_err(|err| format!("创建 .vscode 失败：{err}"))?;
    let settings = json!({
        "terminal.integrated.defaultProfile.windows": "PowerShell",
        "python.defaultInterpreterPath": "${workspaceFolder}\\.venv\\Scripts\\python.exe",
        "java.configuration.updateBuildConfiguration": "interactive",
        "npm.packageManager": "npm"
    });
    let tasks = json!({
        "version": "2.0.0",
        "tasks": [
            {
                "label": "Python: pytest",
                "type": "shell",
                "command": "python -m pytest -q",
                "group": "test",
                "problemMatcher": []
            },
            {
                "label": "Node: test",
                "type": "shell",
                "command": "npm test",
                "group": "test",
                "problemMatcher": []
            }
        ]
    });
    save_json(&vscode.join("settings.json"), &settings)?;
    save_json(&vscode.join("tasks.json"), &tasks)?;
    Ok(OperationResult {
        success: true,
        message: format!("已生成 VS Code 配置：{}", display_path(vscode)),
    })
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            app_snapshot,
            load_config,
            set_root_dir,
            env_snapshot,
            configure_user_environment,
            restore_user_environment,
            discover_runtimes,
            install_jdk,
            install_node,
            install_python,
            install_maven_latest,
            install_gradle_latest,
            switch_runtime,
            uninstall_runtime,
            kill_process,
            scan_ports,
            project_health,
            network_diagnostics,
            cache_entries,
            clear_download_cache,
            run_tool_command,
            generate_vscode_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running DevEnv Manager");
}

impl AppPaths {
    fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn envs(&self) -> PathBuf {
        self.root.join("envs")
    }
    fn jdks(&self) -> PathBuf {
        self.envs().join("jdks")
    }
    fn pythons(&self) -> PathBuf {
        self.envs().join("pythons")
    }
    fn nodes(&self) -> PathBuf {
        self.envs().join("nodes")
    }
    fn mavens(&self) -> PathBuf {
        self.envs().join("mavens")
    }
    fn gradles(&self) -> PathBuf {
        self.envs().join("gradles")
    }
    fn current(&self) -> PathBuf {
        self.root.join("current")
    }
    fn downloads(&self) -> PathBuf {
        self.root.join("downloads")
    }
    fn config(&self) -> PathBuf {
        self.root.join("config")
    }
    fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }
    fn installed_file(&self) -> PathBuf {
        self.config().join("installed.json")
    }
    fn env_backup_file(&self) -> PathBuf {
        self.config().join("env_backup.json")
    }

    fn ensure(&self) -> io::Result<()> {
        for path in [
            self.root.clone(),
            self.jdks(),
            self.pythons(),
            self.nodes(),
            self.mavens(),
            self.gradles(),
            self.current(),
            self.downloads(),
            self.config(),
            self.logs(),
        ] {
            fs::create_dir_all(path)?;
        }
        Ok(())
    }

    fn assert_inside_root(&self, path: &Path) -> Result<(), String> {
        let root = self.root.canonicalize().unwrap_or_else(|_| self.root.clone());
        let candidate = path
            .parent()
            .and_then(|parent| parent.canonicalize().ok())
            .map(|parent| parent.join(path.file_name().unwrap_or_else(|| OsStr::new(""))))
            .unwrap_or_else(|| path.to_path_buf());
        if candidate != root && !candidate.starts_with(&root) {
            return Err(format!("目标路径不在安装根目录内：{}", display_path(candidate)));
        }
        Ok(())
    }

    fn summary(&self) -> PathSummary {
        PathSummary {
            root: display_path(&self.root),
            envs: display_path(self.envs()),
            jdks: display_path(self.jdks()),
            pythons: display_path(self.pythons()),
            nodes: display_path(self.nodes()),
            mavens: display_path(self.mavens()),
            gradles: display_path(self.gradles()),
            current: display_path(self.current()),
            downloads: display_path(self.downloads()),
            config: display_path(self.config()),
            logs: display_path(self.logs()),
        }
    }
}

trait ExpandHome {
    fn expand_home(&self) -> Option<PathBuf>;
}

impl ExpandHome for Path {
    fn expand_home(&self) -> Option<PathBuf> {
        let text = self.to_string_lossy();
        if text == "~" {
            return dirs::home_dir();
        }
        if let Some(rest) = text.strip_prefix("~/").or_else(|| text.strip_prefix("~\\")) {
            return dirs::home_dir().map(|home| home.join(rest));
        }
        Some(self.to_path_buf())
    }
}

fn default_root_dir() -> PathBuf {
    if cfg!(windows) && Path::new("D:\\").exists() {
        PathBuf::from("D:\\DevEnvManager")
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(APP_NAME)
    }
}

fn app_config_dir() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_NAME)
}

fn settings_file() -> PathBuf {
    app_config_dir().join("settings.json")
}

fn default_settings() -> Settings {
    Settings {
        root_dir: display_path(default_root_dir()),
        auto_check_update: false,
        download_timeout_seconds: 60,
        theme: "system".to_string(),
        last_page: "home".to_string(),
        update_manifest_url: String::new(),
        port_process_exclusions: Vec::new(),
    }
}

fn default_installed() -> InstalledData {
    InstalledData {
        jdks: Vec::new(),
        pythons: Vec::new(),
        nodes: Vec::new(),
        mavens: Vec::new(),
        gradles: Vec::new(),
        current: CurrentVersions::default(),
    }
}

fn load_settings() -> Result<Settings, String> {
    load_json_with_default(&settings_file(), default_settings())
}

fn load_paths() -> Result<AppPaths, String> {
    Ok(AppPaths::new(PathBuf::from(load_settings()?.root_dir)))
}

fn ensure_installed(paths: &AppPaths) -> Result<InstalledData, String> {
    load_json_with_default(&paths.installed_file(), default_installed())
}

fn load_installed(paths: &AppPaths) -> Result<InstalledData, String> {
    load_json_with_default(&paths.installed_file(), default_installed())
}

fn load_json_with_default<T>(path: &Path, default: T) -> Result<T, String>
where
    T: Serialize + for<'de> Deserialize<'de> + Clone,
{
    if !path.exists() {
        save_json(path, &default)?;
        return Ok(default);
    }
    match read_json(path) {
        Ok(value) => Ok(value),
        Err(_) => {
            let backup = path.with_extension(format!(
                "{}.broken",
                path.extension().and_then(OsStr::to_str).unwrap_or("json")
            ));
            let _ = fs::rename(path, backup);
            save_json(path, &default)?;
            Ok(default)
        }
    }
}

fn read_json<T>(path: &Path) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let text = fs::read_to_string(path).map_err(|err| format!("读取配置失败：{err}"))?;
    serde_json::from_str(&text).map_err(|err| format!("解析配置失败：{err}"))
}

fn save_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("创建目录失败：{err}"))?;
    }
    let temp = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(OsStr::to_str).unwrap_or("json")
    ));
    let text = serde_json::to_string_pretty(value).map_err(|err| err.to_string())?;
    fs::write(&temp, text).map_err(|err| format!("写入配置失败：{err}"))?;
    fs::rename(temp, path).map_err(|err| format!("保存配置失败：{err}"))
}

fn user_environment() -> Result<std::collections::HashMap<String, String>, String> {
    #[cfg(windows)]
    {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let env_key = hkcu
            .create_subkey("Environment")
            .map_err(|err| format!("打开用户环境变量失败：{err}"))?
            .0;
        let mut result = std::collections::HashMap::new();
        for item in env_key.enum_values() {
            let (name, value) = item.map_err(|err| format!("读取用户环境变量失败：{err}"))?;
            result.insert(name, value.to_string());
        }
        Ok(result)
    }
    #[cfg(not(windows))]
    {
        Ok(env::vars().collect())
    }
}

fn merge_path(existing: &str) -> String {
    fn key(value: &str) -> String {
        value
            .trim()
            .trim_matches('"')
            .trim_end_matches(['\\', '/'])
            .to_ascii_lowercase()
    }
    let managed_keys: BTreeSet<String> = MANAGED_PATHS.iter().map(|item| key(item)).collect();
    let mut retained = Vec::new();
    let mut seen = BTreeSet::new();
    for item in existing.split(';') {
        let item = item.trim();
        let item_key = key(item);
        if item.is_empty() || managed_keys.contains(&item_key) || seen.contains(&item_key) {
            continue;
        }
        seen.insert(item_key);
        retained.push(item.to_string());
    }
    MANAGED_PATHS
        .iter()
        .map(|item| item.to_string())
        .chain(retained)
        .collect::<Vec<String>>()
        .join(";")
}

fn set_user_environment_values(
    paths: &AppPaths,
    java_home: Option<&str>,
    path: &str,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (env_key, _) = hkcu
            .create_subkey("Environment")
            .map_err(|err| format!("打开用户环境变量失败：{err}"))?;
        env_key
            .set_value("DEVENV_HOME", &display_path(&paths.root))
            .map_err(|err| format!("写入 DEVENV_HOME 失败：{err}"))?;
        match java_home {
            Some(value) => env_key
                .set_value("JAVA_HOME", &value)
                .map_err(|err| format!("写入 JAVA_HOME 失败：{err}"))?,
            None => {
                let _ = env_key.delete_value("JAVA_HOME");
            }
        }
        env_key
            .set_value("Path", &path)
            .map_err(|err| format!("写入 Path 失败：{err}"))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = (paths, java_home, path);
        Err("环境变量配置仅支持 Windows".to_string())
    }
}

fn restore_environment_values(
    devenv_home: Option<&str>,
    java_home: Option<&str>,
    path: &str,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (env_key, _) = hkcu
            .create_subkey("Environment")
            .map_err(|err| format!("打开用户环境变量失败：{err}"))?;
        match devenv_home {
            Some(value) => env_key
                .set_value("DEVENV_HOME", &value)
                .map_err(|err| format!("恢复 DEVENV_HOME 失败：{err}"))?,
            None => {
                let _ = env_key.delete_value("DEVENV_HOME");
            }
        }
        match java_home {
            Some(value) => env_key
                .set_value("JAVA_HOME", &value)
                .map_err(|err| format!("恢复 JAVA_HOME 失败：{err}"))?,
            None => {
                let _ = env_key.delete_value("JAVA_HOME");
            }
        }
        env_key
            .set_value("Path", &path)
            .map_err(|err| format!("恢复 Path 失败：{err}"))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = (devenv_home, java_home, path);
        Err("环境变量恢复仅支持 Windows".to_string())
    }
}

fn broadcast_environment_change() {
    #[cfg(windows)]
    {
        let _ = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                r#"
Add-Type -Namespace Win32 -Name Native -MemberDefinition '[DllImport("user32.dll", SetLastError=true, CharSet=CharSet.Auto)] public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint Msg, UIntPtr wParam, string lParam, uint fuFlags, uint uTimeout, out UIntPtr lpdwResult);';
$result = [UIntPtr]::Zero
[Win32.Native]::SendMessageTimeout([IntPtr]0xffff, 0x1a, [UIntPtr]::Zero, 'Environment', 0x2, 5000, [ref]$result) | Out-Null
"#,
            ])
            .output();
    }
}

fn refresh_user_java_home(paths: &AppPaths) -> Result<(), String> {
    let environment = user_environment()?;
    let selected = select_java_home(paths, &environment);
    let current_path = environment
        .get("Path")
        .or_else(|| environment.get("PATH"))
        .cloned()
        .unwrap_or_default();
    set_user_environment_values(paths, selected.as_deref(), &current_path)?;
    broadcast_environment_change();
    Ok(())
}

fn select_java_home(
    paths: &AppPaths,
    user_environment: &std::collections::HashMap<String, String>,
) -> Option<String> {
    let managed = paths.current().join("jdk");
    if managed.join("bin/java.exe").is_file() && managed.join("bin/javac.exe").is_file() {
        return Some(r"%DEVENV_HOME%\current\jdk".to_string());
    }
    if let Some(value) = user_environment.get("JAVA_HOME") {
        if is_valid_java_home(value, paths) {
            return Some(expand_environment_path(value, paths));
        }
    }
    if let Some(java) = find_on_path("java") {
        let candidate = PathBuf::from(java).parent()?.parent()?.to_path_buf();
        if is_valid_java_home(&display_path(&candidate), paths) {
            return Some(display_path(candidate));
        }
    }
    None
}

fn is_valid_java_home(value: &str, paths: &AppPaths) -> bool {
    let home = PathBuf::from(expand_environment_path(value, paths));
    home.join("bin/java.exe").is_file() && home.join("bin/javac.exe").is_file()
}

fn expand_environment_path(value: &str, paths: &AppPaths) -> String {
    let replaced = value
        .trim()
        .trim_matches('"')
        .replace("%DEVENV_HOME%", &display_path(&paths.root))
        .replace("%devenv_home%", &display_path(&paths.root));
    shellexpand_env(&replaced)
}

fn shellexpand_env(value: &str) -> String {
    let mut result = value.to_string();
    for (key, val) in env::vars() {
        result = result.replace(&format!("%{key}%"), &val);
    }
    result
}

fn inspect_path_entries(entries: &[String], paths: &AppPaths) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut warnings = Vec::new();
    for entry in entries {
        let normalized = entry
            .trim()
            .trim_matches('"')
            .trim_end_matches(['\\', '/'])
            .to_ascii_lowercase();
        if !seen.insert(normalized) {
            warnings.push(format!("重复 PATH: {entry}"));
        }
        let expanded = expand_environment_path(entry, paths);
        if !entry.contains('%') && !Path::new(&expanded).exists() {
            warnings.push(format!("失效 PATH: {entry}"));
        }
    }
    warnings
}

#[derive(Debug)]
struct ReleaseInfo {
    name: String,
    url: String,
    sha256: Option<String>,
    tag: String,
}

fn resolve_jdk_release(version: &str) -> Result<ReleaseInfo, String> {
    let url = format!(
        "https://api.adoptium.net/v3/assets/latest/{version}/hotspot?architecture=x64&image_type=jdk&os=windows&vendor=eclipse"
    );
    let assets: Value = reqwest::blocking::get(&url)
        .map_err(|err| format!("查询 Adoptium 失败：{err}"))?
        .error_for_status()
        .map_err(|err| format!("查询 Adoptium 失败：{err}"))?
        .json()
        .map_err(|err| format!("解析 Adoptium 响应失败：{err}"))?;
    let package = assets
        .as_array()
        .and_then(|items| items.first())
        .and_then(|item| item.pointer("/binary/package"))
        .ok_or_else(|| format!("未找到 JDK {version} 的 Windows x64 版本"))?;
    let link = package
        .get("link")
        .and_then(Value::as_str)
        .ok_or_else(|| "Adoptium 响应缺少下载地址".to_string())?;
    let name = package
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "Adoptium 响应缺少文件名".to_string())?;
    Ok(ReleaseInfo {
        name: name.to_string(),
        url: link.to_string(),
        sha256: package
            .get("checksum")
            .and_then(Value::as_str)
            .map(str::to_string),
        tag: version.to_string(),
    })
}

fn resolve_node_release(version: &str) -> Result<ReleaseInfo, String> {
    let items: Value = reqwest::blocking::get("https://nodejs.org/dist/index.json")
        .map_err(|err| format!("查询 Node.js 失败：{err}"))?
        .error_for_status()
        .map_err(|err| format!("查询 Node.js 失败：{err}"))?
        .json()
        .map_err(|err| format!("解析 Node.js 响应失败：{err}"))?;
    let latest = items
        .as_array()
        .ok_or_else(|| "Node.js 版本索引格式异常".to_string())?
        .iter()
        .filter(|item| {
            item.get("version")
                .and_then(Value::as_str)
                .map(|tag| tag.trim_start_matches('v').split('.').next() == Some(version))
                .unwrap_or(false)
                && item
                    .get("files")
                    .and_then(Value::as_array)
                    .map(|files| files.iter().any(|file| file.as_str() == Some("win-x64-zip")))
                    .unwrap_or(false)
        })
        .max_by_key(|item| {
            item.get("version")
                .and_then(Value::as_str)
                .map(version_key)
                .unwrap_or_default()
        })
        .ok_or_else(|| format!("未找到 Node.js {version} 的 Windows x64 ZIP"))?;
    let tag = latest
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(|| "Node.js 版本缺少 tag".to_string())?;
    let name = format!("node-{tag}-win-x64.zip");
    Ok(ReleaseInfo {
        name: name.clone(),
        url: format!("https://nodejs.org/dist/{tag}/{name}"),
        sha256: None,
        tag: tag.to_string(),
    })
}

fn resolve_node_checksum(release: &ReleaseInfo) -> Result<Option<String>, String> {
    let url = format!("https://nodejs.org/dist/{}/SHASUMS256.txt", release.tag);
    let text = reqwest::blocking::get(&url)
        .map_err(|err| format!("查询 Node.js 校验文件失败：{err}"))?
        .error_for_status()
        .map_err(|err| format!("查询 Node.js 校验文件失败：{err}"))?
        .text()
        .map_err(|err| format!("读取 Node.js 校验文件失败：{err}"))?;
    Ok(text.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let sha = parts.next()?;
        let name = parts.next()?;
        if name == release.name && sha.len() == 64 {
            Some(sha.to_string())
        } else {
            None
        }
    }))
}

fn resolve_python_release(version: &str) -> Result<ReleaseInfo, String> {
    let text = reqwest::blocking::get("https://www.python.org/ftp/python/")
        .map_err(|err| format!("查询 Python 失败：{err}"))?
        .error_for_status()
        .map_err(|err| format!("查询 Python 失败：{err}"))?
        .text()
        .map_err(|err| format!("读取 Python 版本失败：{err}"))?;
    let client = reqwest::blocking::Client::new();
    let mut versions: Vec<String> = text
        .split("href=\"")
        .filter_map(|part| part.split('"').next())
        .filter_map(|href| href.strip_suffix('/'))
        .filter(|value| value.starts_with(&format!("{version}.")))
        .filter(|value| value.chars().all(|ch| ch.is_ascii_digit() || ch == '.'))
        .map(str::to_string)
        .collect();
    versions.sort_by_key(|value| version_key(value));
    versions.reverse();
    for full_version in versions {
        let name = format!("python-{full_version}-amd64.exe");
        let url = format!("https://www.python.org/ftp/python/{full_version}/{name}");
        let available = client
            .head(&url)
            .send()
            .map(|response| response.status().is_success())
            .unwrap_or(false);
        if available {
            return Ok(ReleaseInfo {
                name,
                url,
                sha256: None,
                tag: full_version,
            });
        }
    }
    Err(format!("Python {version} 没有可用的 Windows x64 安装器"))
}

fn resolve_maven_release() -> Result<ReleaseInfo, String> {
    let text = reqwest::blocking::get("https://downloads.apache.org/maven/maven-3/")
        .map_err(|err| format!("查询 Maven 失败：{err}"))?
        .error_for_status()
        .map_err(|err| format!("查询 Maven 失败：{err}"))?
        .text()
        .map_err(|err| format!("读取 Maven 版本失败：{err}"))?;
    let version = text
        .split("href=\"")
        .filter_map(|part| part.split('"').next())
        .filter_map(|href| href.strip_suffix('/'))
        .filter(|value| value.chars().all(|ch| ch.is_ascii_digit() || ch == '.'))
        .max_by_key(|value| version_key(value))
        .ok_or_else(|| "无法从 Apache 获取 Maven 版本".to_string())?;
    let name = format!("apache-maven-{version}-bin.zip");
    Ok(ReleaseInfo {
        name: name.clone(),
        url: format!("https://downloads.apache.org/maven/maven-3/{version}/binaries/{name}"),
        sha256: None,
        tag: version.to_string(),
    })
}

fn resolve_gradle_release() -> Result<ReleaseInfo, String> {
    let items: Value = reqwest::blocking::get("https://services.gradle.org/versions/all")
        .map_err(|err| format!("查询 Gradle 失败：{err}"))?
        .error_for_status()
        .map_err(|err| format!("查询 Gradle 失败：{err}"))?
        .json()
        .map_err(|err| format!("解析 Gradle 响应失败：{err}"))?;
    let item = items
        .as_array()
        .ok_or_else(|| "Gradle 版本索引格式异常".to_string())?
        .iter()
        .filter(|item| {
            !item.get("snapshot").and_then(Value::as_bool).unwrap_or(false)
                && !item.get("nightly").and_then(Value::as_bool).unwrap_or(false)
                && !item
                    .get("releaseNightly")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                && item
                    .get("version")
                    .and_then(Value::as_str)
                    .map(|version| !version.contains('-') && !version.contains('+'))
                    .unwrap_or(false)
        })
        .max_by_key(|item| {
            item.get("version")
                .and_then(Value::as_str)
                .map(version_key)
                .unwrap_or_default()
        })
        .ok_or_else(|| "无法从 Gradle 获取稳定版本".to_string())?;
    let version = item
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(|| "Gradle 响应缺少版本".to_string())?;
    let url = item
        .get("downloadUrl")
        .and_then(Value::as_str)
        .ok_or_else(|| "Gradle 响应缺少下载地址".to_string())?;
    Ok(ReleaseInfo {
        name: format!("gradle-{version}-bin.zip"),
        url: url.to_string(),
        sha256: item
            .get("checksum")
            .and_then(Value::as_str)
            .map(str::to_string),
        tag: version.to_string(),
    })
}

fn version_key(tag: &str) -> (u64, u64, u64) {
    let mut parts = tag.trim_start_matches('v').split('.');
    (
        parts.next().and_then(|item| item.parse().ok()).unwrap_or(0),
        parts.next().and_then(|item| item.parse().ok()).unwrap_or(0),
        parts.next().and_then(|item| item.parse().ok()).unwrap_or(0),
    )
}

fn validate_download_url(url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url).map_err(|err| format!("下载地址无效：{err}"))?;
    let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();
    if parsed.scheme() != "https" || !ALLOWED_DOWNLOAD_HOSTS.contains(&host.as_str()) {
        return Err(format!("下载地址不在安全白名单中：{url}"));
    }
    Ok(())
}

fn download_file(url: &str, target_path: &Path, expected_sha256: Option<&str>) -> Result<(), String> {
    validate_download_url(url)?;
    if target_path.exists() && target_path.metadata().map(|item| item.len()).unwrap_or(0) > 0 {
        if expected_sha256
            .map(|expected| file_sha256(target_path).ok().as_deref() == Some(&expected.to_ascii_lowercase()))
            .unwrap_or(true)
        {
            return Ok(());
        }
    }
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("创建下载目录失败：{err}"))?;
    }
    let temp_path = target_path.with_extension(format!(
        "{}.part",
        target_path.extension().and_then(OsStr::to_str).unwrap_or("download")
    ));
    let client = reqwest::blocking::Client::builder()
        .user_agent("DevEnvManager/2.0")
        .build()
        .map_err(|err| format!("创建下载客户端失败：{err}"))?;
    let mut response = client
        .get(url)
        .send()
        .map_err(|err| format!("下载失败：{err}"))?
        .error_for_status()
        .map_err(|err| format!("下载失败：{err}"))?;
    validate_download_url(response.url().as_str())?;
    let mut file = fs::File::create(&temp_path).map_err(|err| format!("写入下载缓存失败：{err}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 128];
    let mut downloaded = 0_u64;
    loop {
        let read = response
            .read(&mut buffer)
            .map_err(|err| format!("读取下载数据失败：{err}"))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .map_err(|err| format!("写入下载缓存失败：{err}"))?;
        hasher.update(&buffer[..read]);
        downloaded += read as u64;
    }
    if downloaded == 0 {
        return Err("服务器返回了空文件".to_string());
    }
    if let Some(expected) = expected_sha256 {
        let actual = format!("{:x}", hasher.finalize());
        if actual.to_ascii_lowercase() != expected.to_ascii_lowercase() {
            let _ = fs::remove_file(&temp_path);
            return Err("SHA-256 校验失败，文件可能不完整".to_string());
        }
    }
    fs::rename(temp_path, target_path).map_err(|err| format!("保存下载文件失败：{err}"))
}

fn file_sha256(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|err| format!("读取文件失败：{err}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 128];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|err| format!("读取文件失败：{err}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn install_zip_payload(archive: &Path, target: &Path, required_files: &[&str]) -> Result<(), String> {
    if target.exists() {
        return Err(format!("目标版本已经存在：{}", display_path(target)));
    }
    let parent = target
        .parent()
        .ok_or_else(|| "目标路径缺少父目录".to_string())?;
    fs::create_dir_all(parent).map_err(|err| format!("创建安装目录失败：{err}"))?;
    let temp = TempBuilder::new()
        .prefix("devenv-")
        .tempdir_in(parent)
        .map_err(|err| format!("创建临时目录失败：{err}"))?;
    safe_extract_zip(archive, temp.path())?;
    let mut candidates = vec![temp.path().to_path_buf()];
    for item in fs::read_dir(temp.path()).map_err(|err| format!("读取解压目录失败：{err}"))? {
        let item = item.map_err(|err| err.to_string())?.path();
        if item.is_dir() {
            candidates.push(item);
        }
    }
    let payload = candidates
        .into_iter()
        .find(|candidate| required_files.iter().all(|name| candidate.join(name).exists()))
        .ok_or_else(|| "无法识别压缩包中的运行时根目录".to_string())?;
    fs::rename(&payload, target).map_err(|err| format!("移动运行时目录失败：{err}"))
}

fn safe_extract_zip(archive: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|err| format!("创建解压目录失败：{err}"))?;
    let destination = destination
        .canonicalize()
        .map_err(|err| format!("解析解压目录失败：{err}"))?;
    let file = fs::File::open(archive).map_err(|err| format!("打开压缩包失败：{err}"))?;
    let mut zip = ZipArchive::new(file).map_err(|err| format!("解压失败：{err}"))?;
    for index in 0..zip.len() {
        let mut member = zip.by_index(index).map_err(|err| format!("读取压缩包失败：{err}"))?;
        let enclosed = member
            .enclosed_name()
            .ok_or_else(|| format!("压缩包包含危险路径：{}", member.name()))?
            .to_path_buf();
        let target = destination.join(enclosed);
        if !target.starts_with(&destination) {
            return Err(format!("压缩包包含危险路径：{}", member.name()));
        }
        if member.is_dir() {
            fs::create_dir_all(&target).map_err(|err| format!("创建目录失败：{err}"))?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|err| format!("创建目录失败：{err}"))?;
            }
            let mut out = fs::File::create(&target).map_err(|err| format!("写入解压文件失败：{err}"))?;
            io::copy(&mut member, &mut out).map_err(|err| format!("写入解压文件失败：{err}"))?;
        }
    }
    Ok(())
}

fn record_install(
    paths: &AppPaths,
    meta: RuntimeMeta,
    version: &str,
    path: &Path,
    executable_path: &Path,
    extra: Value,
) -> Result<(), String> {
    let mut installed = load_installed(paths)?;
    let records = collection_mut(&mut installed, meta.collection);
    records.retain(|item| item.get("version").and_then(Value::as_str) != Some(version));
    let mut record = json!({
        "version": version,
        "path": display_path(path),
        meta.exe_key: display_path(executable_path),
        "installed_at": current_timestamp(),
    });
    if let (Some(target), Some(source)) = (record.as_object_mut(), extra.as_object()) {
        for (key, value) in source {
            target.insert(key.clone(), value.clone());
        }
    }
    records.push(record);
    save_json(&paths.installed_file(), &installed)
}

fn runtime_meta(kind: &str) -> Result<RuntimeMeta, String> {
    match kind {
        "jdk" => Ok(RuntimeMeta {
            kind: "jdk",
            collection: "jdks",
            link_name: "jdk",
            exe_key: "java_exe",
        }),
        "python" => Ok(RuntimeMeta {
            kind: "python",
            collection: "pythons",
            link_name: "python",
            exe_key: "python_exe",
        }),
        "node" => Ok(RuntimeMeta {
            kind: "node",
            collection: "nodes",
            link_name: "node",
            exe_key: "node_exe",
        }),
        "maven" => Ok(RuntimeMeta {
            kind: "maven",
            collection: "mavens",
            link_name: "maven",
            exe_key: "mvn_exe",
        }),
        "gradle" => Ok(RuntimeMeta {
            kind: "gradle",
            collection: "gradles",
            link_name: "gradle",
            exe_key: "gradle_exe",
        }),
        _ => Err(format!("未知运行时类型：{kind}")),
    }
}

fn collection<'a>(installed: &'a InstalledData, collection: &str) -> &'a Vec<Value> {
    match collection {
        "jdks" => &installed.jdks,
        "pythons" => &installed.pythons,
        "nodes" => &installed.nodes,
        "mavens" => &installed.mavens,
        "gradles" => &installed.gradles,
        _ => &installed.jdks,
    }
}

fn collection_mut<'a>(installed: &'a mut InstalledData, collection: &str) -> &'a mut Vec<Value> {
    match collection {
        "jdks" => &mut installed.jdks,
        "pythons" => &mut installed.pythons,
        "nodes" => &mut installed.nodes,
        "mavens" => &mut installed.mavens,
        "gradles" => &mut installed.gradles,
        _ => &mut installed.jdks,
    }
}

fn runtime_parent(paths: &AppPaths, collection: &str) -> Result<PathBuf, String> {
    match collection {
        "jdks" => Ok(paths.jdks()),
        "pythons" => Ok(paths.pythons()),
        "nodes" => Ok(paths.nodes()),
        "mavens" => Ok(paths.mavens()),
        "gradles" => Ok(paths.gradles()),
        _ => Err(format!("未知运行时集合：{collection}")),
    }
}

fn current_version(installed: &InstalledData, kind: &str) -> Option<String> {
    match kind {
        "jdk" => installed.current.jdk.clone(),
        "python" => installed.current.python.clone(),
        "node" => installed.current.node.clone(),
        "maven" => installed.current.maven.clone(),
        "gradle" => installed.current.gradle.clone(),
        _ => None,
    }
}

fn set_current(installed: &mut InstalledData, kind: &str, version: Option<String>) {
    match kind {
        "jdk" => installed.current.jdk = version,
        "python" => installed.current.python = version,
        "node" => installed.current.node = version,
        "maven" => installed.current.maven = version,
        "gradle" => installed.current.gradle = version,
        _ => {}
    }
}

fn switch_junction(link: &Path, target: &Path, root: &Path) -> Result<(), String> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let target_resolved = target
        .canonicalize()
        .map_err(|err| format!("解析版本目录失败：{err}"))?;
    if !target_resolved.starts_with(&root) {
        return Err("版本目录不在安装根目录内".to_string());
    }
    if let Some(parent) = link.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("创建 current 目录失败：{err}"))?;
    }
    if link.exists() {
        if !is_junction(link) {
            return Err(format!("拒绝删除非链接目录：{}", display_path(link)));
        }
        remove_junction(link)?;
    }
    let output = Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .map_err(|err| format!("创建版本指针失败：{err}"))?;
    if !output.status.success() {
        return Err(command_text(&output.stdout, &output.stderr));
    }
    Ok(())
}

fn remove_junction(link: &Path) -> Result<(), String> {
    if !link.exists() {
        return Ok(());
    }
    if !is_junction(link) {
        return Err(format!("拒绝删除非链接目录：{}", display_path(link)));
    }
    let output = Command::new("cmd")
        .args(["/c", "rmdir"])
        .arg(link)
        .output()
        .map_err(|err| format!("删除版本指针失败：{err}"))?;
    if !output.status.success() {
        return Err(command_text(&output.stdout, &output.stderr));
    }
    Ok(())
}

fn is_junction(path: &Path) -> bool {
    Command::new("cmd")
        .args(["/c", "fsutil", "reparsepoint", "query"])
        .arg(path)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn detect_runtime(kind: &str, executable: &str, args: &[&str]) -> Option<RuntimeInfo> {
    let output = Command::new(executable).args(args).output().ok()?;
    let mut text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        text = String::from_utf8_lossy(&output.stderr).trim().to_string();
    }
    let version = text.lines().next().unwrap_or("unknown").to_string();
    let path = find_on_path(executable).unwrap_or_else(|| executable.to_string());

    Some(RuntimeInfo {
        kind: kind.to_string(),
        version,
        executable: path.clone(),
        source: classify_source(&path),
    })
}

fn find_on_path(executable: &str) -> Option<String> {
    let path_value = env::var_os("PATH")?;
    let extensions = if cfg!(windows) {
        vec![".exe", ".cmd", ".bat", ""]
    } else {
        vec![""]
    };

    for dir in env::split_paths(&path_value) {
        for ext in &extensions {
            let candidate = dir.join(format!("{executable}{ext}"));
            if candidate.is_file() {
                return Some(display_path(candidate));
            }
        }
    }
    None
}

fn parse_socket(value: &str) -> Option<(String, u16)> {
    let trimmed = value.trim();
    if trimmed.starts_with('[') {
        let end = trimmed.rfind("]:")?;
        let addr = trimmed[1..end].to_string();
        let port = trimmed[end + 2..].parse().ok()?;
        return Some((addr, port));
    }

    let (addr, port_text) = trimmed.rsplit_once(':')?;
    let normalized_addr = if addr == "*" {
        IpAddr::V4(Ipv4Addr::UNSPECIFIED).to_string()
    } else {
        addr.to_string()
    };
    let port = port_text.parse().ok()?;
    Some((normalized_addr, port))
}

fn process_name(system: &sysinfo::System, pid: u32) -> String {
    system
        .process(sysinfo::Pid::from_u32(pid))
        .map(|process| process.name().to_string_lossy().to_string())
        .unwrap_or_else(|| {
            if pid == 0 {
                "Idle".to_string()
            } else if pid == 4 {
                "System".to_string()
            } else {
                "unknown".to_string()
            }
        })
}

fn classify_port(port: u16, pid: u32, process_name: &str) -> String {
    let lower = process_name.to_ascii_lowercase();
    if pid == 0 || pid == 4 || lower == "system" {
        "系统保留".to_string()
    } else if matches!(
        port,
        20 | 21 | 22 | 23 | 25 | 53 | 80 | 110 | 135 | 139 | 143 | 443 | 445 | 3389
    ) {
        "敏感端口".to_string()
    } else {
        "普通".to_string()
    }
}

fn classify_source(path: &str) -> String {
    let lower = path.to_ascii_lowercase();
    if lower.contains("\\devenvmanager\\") {
        "DevEnv".to_string()
    } else if lower.contains("\\windowsapps\\") {
        "Microsoft Store".to_string()
    } else if lower.contains("\\scoop\\") {
        "Scoop".to_string()
    } else if lower.contains("\\chocolatey\\") {
        "Chocolatey".to_string()
    } else if lower.contains("\\program files\\") || lower.contains("\\program files (x86)\\") {
        "System".to_string()
    } else {
        "PATH".to_string()
    }
}

fn emit_task_progress(app: &tauri::AppHandle, task: &str, percent: u8, message: &str) {
    let _ = app.emit(
        "task-progress",
        TaskProgress {
            task: task.to_string(),
            percent,
            message: message.to_string(),
        },
    );
}

fn proxy_state() -> Vec<(String, String)> {
    let mut result = vec![
        ("HTTP_PROXY".to_string(), env::var("HTTP_PROXY").unwrap_or_default()),
        (
            "HTTPS_PROXY".to_string(),
            env::var("HTTPS_PROXY").unwrap_or_default(),
        ),
        ("NO_PROXY".to_string(), env::var("NO_PROXY").unwrap_or_default()),
    ];
    #[cfg(windows)]
    {
        let proxy = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Internet Settings")
            .ok()
            .and_then(|key| {
                let enabled: Result<u32, _> = key.get_value("ProxyEnable");
                if enabled.ok()? == 0 {
                    return Some("未启用".to_string());
                }
                key.get_value::<String, _>("ProxyServer").ok()
            })
            .unwrap_or_else(|| "未启用".to_string());
        result.push(("系统代理".to_string(), proxy));
    }
    result
}

fn network_error(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        "timeout".to_string()
    } else if error.is_connect() {
        "connect failed".to_string()
    } else {
        error.to_string()
    }
}

fn format_size(size: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let size = size as f64;
    if size >= GB {
        format!("{:.2} GB", size / GB)
    } else if size >= MB {
        format!("{:.2} MB", size / MB)
    } else if size >= KB {
        format!("{:.2} KB", size / KB)
    } else {
        format!("{size:.0} B")
    }
}

fn parse_command_line(command: &str) -> Result<Vec<String>, String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escape = false;

    for ch in command.chars() {
        if escape {
            current.push(ch);
            escape = false;
            continue;
        }
        if ch == '\\' {
            escape = true;
            continue;
        }
        if let Some(active) = quote {
            if ch == active {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        match ch {
            '"' | '\'' => quote = Some(ch),
            ch if ch.is_whitespace() => {
                if !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if escape {
        current.push('\\');
    }
    if quote.is_some() {
        return Err("命令包含未闭合的引号".to_string());
    }
    if !current.is_empty() {
        parts.push(current);
    }
    Ok(parts)
}

fn run_command_output(executable: PathBuf, args: &[&str], timeout_seconds: u64) -> Result<String, String> {
    let output = Command::new(executable)
        .args(args)
        .output()
        .map_err(|err| format!("执行命令失败：{err}"))?;
    let _ = timeout_seconds;
    if !output.status.success() {
        return Err(command_text(&output.stdout, &output.stderr));
    }
    Ok(command_text(&output.stdout, &output.stderr))
}

fn command_text(stdout: &[u8], stderr: &[u8]) -> String {
    let stdout = String::from_utf8_lossy(stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(stderr).trim().to_string();
    [stdout, stderr]
        .into_iter()
        .filter(|item| !item.is_empty())
        .collect::<Vec<String>>()
        .join("\n")
}

fn current_timestamp() -> String {
    // Keep dependencies lean; second precision is enough for audit records.
    format!("{:?}", std::time::SystemTime::now())
}

fn display_path(path: impl AsRef<Path>) -> String {
    path.as_ref().display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_path_adds_managed_entries_once() {
        let merged = merge_path(r"C:\Tools;%DEVENV_HOME%\current\node;C:\Tools;C:\Windows");
        let parts: Vec<&str> = merged.split(';').collect();
        assert_eq!(parts[0], r"%DEVENV_HOME%\current\jdk\bin");
        assert_eq!(
            parts
                .iter()
                .filter(|item| **item == r"%DEVENV_HOME%\current\node")
                .count(),
            1
        );
        assert_eq!(parts.iter().filter(|item| **item == r"C:\Tools").count(), 1);
    }

    #[test]
    fn parse_socket_handles_ipv4_and_ipv6() {
        assert_eq!(
            parse_socket("127.0.0.1:8080"),
            Some(("127.0.0.1".to_string(), 8080))
        );
        assert_eq!(
            parse_socket("[::1]:5173"),
            Some(("::1".to_string(), 5173))
        );
    }

    #[test]
    fn version_key_sorts_semver_like_tags() {
        assert!(version_key("v22.11.0") > version_key("v20.18.3"));
        assert!(version_key("3.12.10") > version_key("3.12.9"));
    }

    #[test]
    fn command_parser_preserves_quoted_arguments() {
        assert_eq!(
            parse_command_line(r#"python -m pytest "tests/test path.py""#).unwrap(),
            vec!["python", "-m", "pytest", "tests/test path.py"]
        );
        assert!(parse_command_line(r#"node "unterminated"#).is_err());
    }
}
