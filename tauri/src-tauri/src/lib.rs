mod cleanup;
mod diagnostics;

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
use std::process::{Command, Stdio};
use std::time::Instant;
use tauri::Emitter;
use tempfile::Builder as TempBuilder;
use zip::ZipArchive;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use winreg::{enums::*, RegKey};

const APP_NAME: &str = "DevEnvManager";
const MANAGED_PATHS: [&str; 8] = [
    r"%DEVENV_HOME%\current\jdk\bin",
    r"%DEVENV_HOME%\current\python",
    r"%DEVENV_HOME%\current\python\Scripts",
    r"%DEVENV_HOME%\current\node",
    r"%DEVENV_HOME%\current\maven\bin",
    r"%DEVENV_HOME%\current\gradle\bin",
    r"%DEVENV_HOME%\current\go\bin",
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
const ALLOWED_DOWNLOAD_HOSTS: [&str; 22] = [
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
    "go.dev",
    "raw.githubusercontent.com",
    "api.github.com",
    "api.azul.com",
    "cdn.azul.com",
    "api.bell-sw.com",
    "aka.ms",
    "download.visualstudio.microsoft.com",
    "bootstrap.pypa.io",
    "api.nuget.org",
    "globalcdn.nuget.org",
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
    #[serde(default)]
    gos: Vec<Value>,
    current: CurrentVersions,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct CurrentVersions {
    jdk: Option<String>,
    python: Option<String>,
    node: Option<String>,
    maven: Option<String>,
    gradle: Option<String>,
    go: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ConfigProfile {
    id: String,
    name: String,
    created_at: String,
    current: CurrentVersions,
    devenv_home: Option<String>,
    java_home: Option<String>,
    path: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigProfileBundle {
    schema_version: u32,
    exported_at: String,
    profiles: Vec<ConfigProfile>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigProfilePreviewItem {
    name: String,
    current: CurrentVersions,
    missing: Vec<String>,
    will_replace: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigProfileImportPreview {
    source: String,
    exported_at: String,
    profiles: Vec<ConfigProfilePreviewItem>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProfileRequirement {
    kind: String,
    version: String,
    installed: bool,
    auto_install_supported: bool,
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
    gos: String,
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
struct JavaEnvironmentReport {
    java_home: String,
    java_home_expanded: String,
    path_java: String,
    path_javac: String,
    java_version: String,
    javac_version: String,
    maven_runtime: String,
    gradle_runtime: String,
    effective_source: String,
    consistent: bool,
    warnings: Vec<String>,
    candidates: Vec<RuntimeInfo>,
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
    process_path: String,
    command_line: String,
    parent_pid: u32,
    parent_process_name: String,
    service_names: Vec<String>,
    common_usage: String,
    explanation: String,
    risk: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProjectPortConfig {
    id: String,
    kind: String,
    file: String,
    current_port: u16,
    line: usize,
    description: String,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PortHistoryEntry {
    port: u16,
    protocol: String,
    pid: u32,
    process_name: String,
    process_path: String,
    observed_at: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PortHistorySummary {
    port: u16,
    process_name: String,
    observations: usize,
    last_seen: u64,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CommandSafetyAssessment {
    allowed: bool,
    risk: String,
    reason: String,
    requires_confirmation: bool,
    elevated: bool,
    executable: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvHealthCheck {
    name: String,
    status: String,
    detail: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DoctorReport {
    score: u8,
    summary: String,
    checks: Vec<DoctorCheck>,
    suggestions: Vec<DoctorSuggestion>,
    generated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DoctorCheck {
    id: String,
    title: String,
    category: String,
    status: String,
    severity: String,
    detail: String,
    fix_action: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DoctorSuggestion {
    id: String,
    title: String,
    description: String,
    action: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DoctorRepairResult {
    before_score: u8,
    after_score: u8,
    applied: Vec<String>,
    remaining: Vec<String>,
    report: DoctorReport,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PythonToolState {
    path: String,
    version: String,
    status: String,
    detail: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PythonEntry {
    path: String,
    source: String,
    version: String,
    current: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PythonAnalysis {
    current_python: Option<PythonToolState>,
    current_pip: Option<PythonToolState>,
    launcher_output: String,
    discovered_pythons: Vec<PythonEntry>,
    discovered_pips: Vec<PythonEntry>,
    risks: Vec<String>,
    recommendations: Vec<String>,
    pip_repair_command: String,
    alias_settings_command: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectRuntimeRecommendation {
    name: String,
    requirement: String,
    status: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProjectAction {
    id: String,
    title: String,
    command: String,
    description: String,
    safe_to_run: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectAnalysis {
    root: String,
    project_types: Vec<String>,
    detected_files: Vec<String>,
    package_manager: Option<String>,
    recommended_runtime: Vec<ProjectRuntimeRecommendation>,
    actions: Vec<ProjectAction>,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ToolState {
    name: String,
    installed: bool,
    version: String,
    path: String,
    detail: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GitEnvironment {
    git: ToolState,
    git_bash_path: String,
    user_name: String,
    user_email: String,
    ssh: ToolState,
    ssh_key_exists: bool,
    public_key_path: String,
    public_key: String,
    github_ssh_status: String,
    github_https_status: String,
    git_lfs: ToolState,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NodeEcosystem {
    tools: Vec<ToolState>,
    npm_prefix: String,
    npm_registry: String,
    pnpm_store_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PythonEcosystem {
    tools: Vec<ToolState>,
    pip_config: String,
    pip_index_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolchainReport {
    tools: Vec<ToolDefinition>,
    git: GitEnvironment,
    node: NodeEcosystem,
    python: PythonEcosystem,
    generated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GoEnvironment {
    go: ToolState,
    goroot: String,
    gopath: String,
    goproxy: String,
    gomodcache: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RustEnvironment {
    tools: Vec<ToolState>,
    default_toolchain: String,
    installed_toolchains: Vec<String>,
    msvc_build_tools: String,
    cargo_config_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DotnetEnvironment {
    dotnet: ToolState,
    sdks: Vec<String>,
    runtimes: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MirrorCenter {
    npm_registry: String,
    pip_index_url: String,
    go_proxy: String,
    maven_settings_path: String,
    maven_settings_exists: bool,
    gradle_init_path: String,
    gradle_init_exists: bool,
    cargo_config_path: String,
    cargo_config_exists: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlatformReport {
    go: GoEnvironment,
    rust: RustEnvironment,
    dotnet: DotnetEnvironment,
    mirrors: MirrorCenter,
    generated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemPlatformReport {
    docker: ToolState,
    docker_info: String,
    docker_desktop_path: String,
    wsl: ToolState,
    wsl_status: String,
    wsl_distributions: Vec<String>,
    wsl_items: Vec<WslDistribution>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct WslDistribution {
    name: String,
    state: String,
    version: String,
    is_default: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalServiceStatus {
    id: String,
    name: String,
    port: u16,
    occupied: bool,
    pid: u32,
    process_name: String,
    process_path: String,
    service_names: Vec<String>,
    safe_to_stop: bool,
    connection_command: String,
    installed: bool,
    service_name: String,
    service_state: String,
    binary_path: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")]
struct WindowsServiceInfo {
    name: String,
    state: String,
    path_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct JdkDistribution {
    id: String,
    name: String,
    recommended: bool,
    supports_install: bool,
    description: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateManifest {
    version: String,
    date: String,
    notes: Vec<String>,
    download_url: String,
    #[serde(default)]
    sha256: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateCheckResult {
    current_version: String,
    latest_version: String,
    update_available: bool,
    date: String,
    notes: Vec<String>,
    download_url: String,
    sha256: String,
    checked_at: String,
}

#[derive(Debug, Clone)]
struct UninstallEntry {
    display_name: String,
    install_location: String,
    display_icon: String,
    uninstall_string: String,
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
    let root = normalize_root_dir(&root)?;
    let mut settings = load_settings()?;
    settings.root_dir = display_path(&root);
    save_json(&settings_file(), &settings)?;
    let paths = AppPaths::new(root);
    paths.ensure().map_err(|err| err.to_string())?;
    ensure_installed(&paths)?;
    load_config()
}

#[tauri::command]
fn set_auto_check_update(enabled: bool) -> Result<ConfigView, String> {
    let mut settings = load_settings()?;
    settings.auto_check_update = enabled;
    save_json(&settings_file(), &settings)?;
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
        java_home: user_env
            .get("JAVA_HOME")
            .cloned()
            .or_else(|| env::var("JAVA_HOME").ok()),
        devenv_home: user_env
            .get("DEVENV_HOME")
            .cloned()
            .or_else(|| env::var("DEVENV_HOME").ok()),
        path_warnings,
    }
}

#[tauri::command]
async fn configure_user_environment() -> Result<OperationResult, String> {
    run_blocking(configure_user_environment_blocking).await?
}

#[tauri::command]
fn storage_cleanup_architecture() -> cleanup::CleanupArchitecture {
    cleanup::architecture()
}

#[tauri::command]
async fn scan_storage_cleanup() -> Result<cleanup::CleanupScanReport, String> {
    run_blocking(|| {
        let paths = load_paths()?;
        cleanup::scan(&paths.root)
    })
    .await?
}

#[tauri::command]
async fn inspect_java_environment() -> Result<JavaEnvironmentReport, String> {
    run_blocking(inspect_java_environment_blocking).await?
}

#[tauri::command]
async fn inspect_agent_traces(
    project_path: Option<String>,
) -> Result<diagnostics::AgentTraceReport, String> {
    run_blocking(move || {
        let project = project_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
        diagnostics::inspect_agent_traces(project.as_deref())
    })
    .await
}

#[tauri::command]
async fn scan_cleanup_targets() -> Result<cleanup::CleanupScanReport, String> {
    run_blocking(|| {
        let paths = load_paths()?;
        cleanup::scan_cleanup_targets(&paths.root)
    })
    .await?
}

#[tauri::command]
async fn inspect_maintenance_overview() -> Result<cleanup::MaintenanceOverview, String> {
    run_blocking(|| {
        let paths = load_paths()?;
        cleanup::inspect_maintenance_overview(&paths.root)
    })
    .await?
}

#[tauri::command]
async fn inspect_disk_overview() -> Result<Vec<cleanup::DiskVolumeInfo>, String> {
    run_blocking(cleanup::inspect_disk_overview).await?
}

#[tauri::command]
fn jdk_distributions() -> Vec<JdkDistribution> {
    vec![
        JdkDistribution {
            id: "temurin".to_string(),
            name: "Eclipse Temurin".to_string(),
            recommended: true,
            supports_install: true,
            description: "默认推荐，支持官方 API 自动安装".to_string(),
        },
        JdkDistribution {
            id: "zulu".to_string(),
            name: "Azul Zulu".to_string(),
            recommended: false,
            supports_install: true,
            description: "通过 Azul 官方元数据 API 安装标准版 JDK".to_string(),
        },
        JdkDistribution {
            id: "liberica".to_string(),
            name: "BellSoft Liberica".to_string(),
            recommended: false,
            supports_install: true,
            description: "通过 BellSoft 官方 API 安装 Liberica Standard JDK".to_string(),
        },
        JdkDistribution {
            id: "microsoft".to_string(),
            name: "Microsoft OpenJDK".to_string(),
            recommended: false,
            supports_install: true,
            description: "通过 Microsoft 官方稳定下载地址安装 OpenJDK".to_string(),
        },
        JdkDistribution {
            id: "oracle".to_string(),
            name: "Oracle JDK".to_string(),
            recommended: false,
            supports_install: false,
            description: "仅检测或引导，不自动接受许可协议".to_string(),
        },
    ]
}

#[tauri::command]
async fn check_for_updates() -> Result<UpdateCheckResult, String> {
    run_blocking(check_for_updates_blocking).await?
}

fn check_for_updates_blocking() -> Result<UpdateCheckResult, String> {
    let settings = load_settings()?;
    let url = if settings.update_manifest_url.trim().is_empty() {
        "https://raw.githubusercontent.com/weidonglang/DevEnv-Manager/main/update-manifest.json"
            .to_string()
    } else {
        settings.update_manifest_url
    };
    validate_download_url(&url)?;
    let client = reqwest::blocking::Client::builder()
        .user_agent("DevEnvManager/2.0")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|err| format!("创建更新检查客户端失败：{err}"))?;
    let manifest: UpdateManifest = client
        .get(&url)
        .send()
        .map_err(|err| format!("检查更新失败：{err}"))?
        .error_for_status()
        .map_err(|err| format!("检查更新失败：{err}"))?
        .json()
        .map_err(|err| format!("更新清单格式错误：{err}"))?;
    validate_download_url(&manifest.download_url)?;
    let current = env!("CARGO_PKG_VERSION").to_string();
    Ok(UpdateCheckResult {
        update_available: version_key(&manifest.version) > version_key(&current),
        current_version: current,
        latest_version: manifest.version,
        date: manifest.date,
        notes: manifest.notes,
        download_url: manifest.download_url,
        sha256: manifest.sha256,
        checked_at: current_timestamp(),
    })
}

#[tauri::command]
async fn download_update(app: tauri::AppHandle) -> Result<OperationResult, String> {
    run_blocking(move || download_update_blocking(app)).await?
}

fn download_update_blocking(app: tauri::AppHandle) -> Result<OperationResult, String> {
    let update = check_for_updates_blocking()?;
    if !update.update_available {
        return Err("当前已经是最新版本".to_string());
    }
    validate_update_checksum(&update.sha256)?;
    let paths = load_paths()?;
    let target = update_installer_path(&paths, &update.latest_version);
    let task = format!("更新 {}", update.latest_version);
    emit_task_progress(&app, &task, 5, "正在准备下载安装包");
    download_file_with_progress(
        &update.download_url,
        &target,
        Some(&update.sha256),
        Some((&app, &task, 8, 95)),
    )?;
    emit_task_progress(&app, &task, 100, "更新安装包已通过 SHA256 校验");
    Ok(OperationResult {
        success: true,
        message: format!("更新安装包已就绪：{}", display_path(target)),
    })
}

#[tauri::command]
async fn launch_update_installer(app: tauri::AppHandle) -> Result<OperationResult, String> {
    let result = run_blocking(|| {
        let update = check_for_updates_blocking()?;
        if !update.update_available {
            return Err("当前已经是最新版本".to_string());
        }
        validate_update_checksum(&update.sha256)?;
        let paths = load_paths()?;
        let target = update_installer_path(&paths, &update.latest_version);
        if !target.is_file() {
            return Err("更新安装包尚未下载，请先点击下载更新".to_string());
        }
        let actual = file_sha256(&target)?;
        if !actual.eq_ignore_ascii_case(&update.sha256) {
            return Err("更新安装包 SHA256 校验失败，已拒绝启动".to_string());
        }
        hidden_command(&target)
            .spawn()
            .map_err(|err| format!("启动更新安装器失败：{err}"))?;
        Ok(OperationResult {
            success: true,
            message: "已启动更新安装器，当前程序即将退出".to_string(),
        })
    })
    .await??;
    app.exit(0);
    Ok(result)
}

fn validate_update_checksum(value: &str) -> Result<(), String> {
    if value.len() != 64 || !value.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err("更新清单缺少有效 SHA256，已拒绝下载".to_string());
    }
    Ok(())
}

fn update_installer_path(paths: &AppPaths, version: &str) -> PathBuf {
    paths
        .downloads()
        .join(format!("DevEnv-Manager-{version}-x64-setup.exe"))
}
#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
struct ToolDefinition {
    id: &'static str,
    name: &'static str,
    category: &'static str,
    exe_names: &'static [&'static str],
    env_vars: &'static [&'static str],
    managed_path_entries: &'static [&'static str],
    supports_install: bool,
    supports_switch: bool,
    supports_mirror: bool,
}

fn configure_user_environment_blocking() -> Result<OperationResult, String> {
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
    set_user_environment_values(
        &paths,
        selected_java_home.as_deref(),
        &merge_path(&old_path),
    )?;
    broadcast_environment_change();
    Ok(OperationResult {
        success: true,
        message: selected_java_home
            .map(|value| format!("已配置用户环境变量，JAVA_HOME = {value}"))
            .unwrap_or_else(|| "已配置用户环境变量，未发现可用 JAVA_HOME".to_string()),
    })
}

#[tauri::command]
async fn cleanup_path_entries() -> Result<OperationResult, String> {
    run_blocking(cleanup_path_entries_blocking).await?
}

fn cleanup_path_entries_blocking() -> Result<OperationResult, String> {
    let paths = load_paths()?;
    let environment = user_environment()?;
    let old_path = environment
        .get("Path")
        .or_else(|| environment.get("PATH"))
        .cloned()
        .unwrap_or_default();
    let mut seen = BTreeSet::new();
    let mut retained = Vec::new();
    let mut removed = 0_usize;

    for entry in old_path.split(';') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let key = path_key(entry);
        if !seen.insert(key) {
            removed += 1;
            continue;
        }
        let expanded = expand_environment_path(entry, &paths);
        if !Path::new(&expanded).exists() && !is_managed_pending_path(&expanded, &paths) {
            removed += 1;
            continue;
        }
        retained.push(entry.to_string());
    }

    let new_path = retained.join(";");
    let java_home = environment.get("JAVA_HOME").map(String::as_str);
    set_user_environment_values(&paths, java_home, &new_path)?;
    broadcast_environment_change();
    Ok(OperationResult {
        success: true,
        message: if removed == 0 {
            "PATH 没有需要清理的真实失效或重复项".to_string()
        } else {
            format!("已清理 {removed} 个真实失效或重复 PATH，托管待安装路径已保留")
        },
    })
}

#[tauri::command]
async fn restore_user_environment() -> Result<OperationResult, String> {
    run_blocking(restore_user_environment_blocking).await?
}

fn restore_user_environment_blocking() -> Result<OperationResult, String> {
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
async fn discover_runtimes() -> Vec<RuntimeInfo> {
    run_blocking(discover_runtimes_blocking)
        .await
        .unwrap_or_default()
}

fn discover_runtimes_blocking() -> Vec<RuntimeInfo> {
    let mut runtimes = Vec::new();

    for (kind, exe, args) in [
        ("Java", "java", vec!["-version"]),
        ("Python", "python", vec!["--version"]),
        ("Python Launcher", "py", vec!["--version"]),
        ("Node.js", "node", vec!["--version"]),
        ("npm", "npm", vec!["--version"]),
        ("Maven", "mvn", vec!["--version"]),
        ("Gradle", "gradle", vec!["--version"]),
        ("Go", "go", vec!["version"]),
        ("Rust", "rustc", vec!["--version"]),
        ("Cargo", "cargo", vec!["--version"]),
        (".NET SDK", "dotnet", vec!["--version"]),
    ] {
        for candidate in find_all_on_path(exe) {
            if let Some(info) = detect_runtime_at(kind, &candidate, &args, None) {
                push_runtime(&mut runtimes, info);
            }
        }
        if !runtimes.iter().any(|item| item.kind == kind) {
            if let Some(info) = detect_runtime(kind, exe, &args) {
                push_runtime(&mut runtimes, info);
            }
        }
    }

    if let Ok(paths) = load_paths() {
        add_managed_runtime_discoveries(&mut runtimes, &paths);
        if let Ok(environment) = user_environment() {
            if let Some(java_home) = environment.get("JAVA_HOME") {
                let executable =
                    PathBuf::from(expand_environment_path(java_home, &paths)).join("bin/java.exe");
                if let Some(info) = detect_runtime_at(
                    "Java",
                    &executable,
                    &["-version"],
                    Some("JAVA_HOME".to_string()),
                ) {
                    push_runtime(&mut runtimes, info);
                }
            }
        }
    }
    add_python_launcher_discoveries(&mut runtimes);
    add_python_registry_discoveries(&mut runtimes);

    runtimes.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then(
                version_key(&a.version)
                    .cmp(&version_key(&b.version))
                    .reverse(),
            )
            .then(a.executable.cmp(&b.executable))
    });
    runtimes
}

#[tauri::command]
async fn install_jdk(
    app: tauri::AppHandle,
    version: String,
    distribution: Option<String>,
) -> Result<OperationResult, String> {
    run_blocking(move || install_jdk_blocking(app, version, distribution)).await?
}

fn install_jdk_blocking(
    app: tauri::AppHandle,
    version: String,
    distribution: Option<String>,
) -> Result<OperationResult, String> {
    let version = version.trim();
    let distribution = distribution.as_deref().unwrap_or("temurin");
    if !["temurin", "zulu", "liberica", "microsoft"].contains(&distribution) {
        return Err("不支持该 JDK 发行版；Oracle JDK 不会自动接受许可协议".to_string());
    }
    let task = format!("JDK {version}");
    emit_task_progress(&app, &task, 2, "正在准备安装");
    if !["8", "11", "17", "21", "25"].contains(&version) {
        return Err(format!("暂不支持 JDK {version}"));
    }
    let paths = load_paths()?;
    paths.ensure().map_err(|err| err.to_string())?;
    emit_task_progress(
        &app,
        &task,
        8,
        &format!("正在查询 {} 官方版本", jdk_distribution_name(distribution)),
    );
    let release = resolve_jdk_release(distribution, version)?;
    let archive = paths.downloads().join(&release.name);
    let target = paths.jdks().join(format!("{distribution}-{version}"));
    let installed_version = format!("{version}-{distribution}");
    paths.assert_inside_root(&target)?;
    if target.exists() {
        return Err(format!("JDK {version} 已安装：{}", display_path(&target)));
    }
    emit_task_progress(&app, &task, 18, "正在下载 JDK");
    download_file_with_progress(
        &release.url,
        &archive,
        release.sha256.as_deref(),
        Some((&app, &task, 18, 68)),
    )?;
    emit_task_progress(&app, &task, 70, "正在解压 JDK");
    install_zip_payload(&archive, &target, &["bin/java.exe", "bin/javac.exe"])?;
    emit_task_progress(&app, &task, 88, "正在验证 JDK");
    let output = run_command_output(target.join("bin/java.exe"), &["-version"], 30)?;
    run_command_output(target.join("bin/javac.exe"), &["-version"], 30)?;
    record_install(
        &paths,
        runtime_meta("jdk")?,
        &installed_version,
        &target,
        &target.join("bin/java.exe"),
        json!({
            "distribution": distribution,
            "javaMajor": version,
            "detail": output.lines().next().unwrap_or(""),
        }),
    )?;
    switch_runtime_blocking(
        "jdk".to_string(),
        installed_version,
        Some(display_path(&target)),
    )?;
    refresh_user_java_home(&paths)?;
    emit_task_progress(&app, &task, 100, "安装完成");
    Ok(OperationResult {
        success: true,
        message: format!(
            "安装成功 {} JDK {version}",
            jdk_distribution_name(distribution)
        ),
    })
}

#[tauri::command]
async fn install_node(app: tauri::AppHandle, version: String) -> Result<OperationResult, String> {
    run_blocking(move || install_node_blocking(app, version)).await?
}

fn install_node_blocking(
    app: tauri::AppHandle,
    version: String,
) -> Result<OperationResult, String> {
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
        return Err(format!(
            "Node.js {version} 已安装：{}",
            display_path(&target)
        ));
    }
    emit_task_progress(&app, &task, 18, "正在下载 Node.js");
    download_file_with_progress(
        &release.url,
        &archive,
        checksum.as_deref(),
        Some((&app, &task, 18, 68)),
    )?;
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
    switch_runtime_blocking("node".to_string(), version.to_string(), None)?;
    emit_task_progress(&app, &task, 100, "安装完成");
    Ok(OperationResult {
        success: true,
        message: format!("安装成功 Node.js {version}"),
    })
}

#[tauri::command]
async fn install_go(app: tauri::AppHandle, version: String) -> Result<OperationResult, String> {
    run_blocking(move || install_go_blocking(app, version)).await?
}

fn install_go_blocking(app: tauri::AppHandle, version: String) -> Result<OperationResult, String> {
    let version = version.trim();
    let task = format!("Go {version}");
    emit_task_progress(&app, &task, 2, "正在准备安装");
    if !["1.22", "1.23", "1.24", "1.25", "1.26"].contains(&version) {
        return Err(format!("暂不支持 Go {version}"));
    }
    let paths = load_paths()?;
    paths.ensure().map_err(|err| err.to_string())?;
    emit_task_progress(&app, &task, 8, "正在查询 Go 官方版本");
    let release = resolve_go_release(version)?;
    let archive = paths.downloads().join(&release.name);
    let target = paths.gos().join(format!("go-{version}"));
    paths.assert_inside_root(&target)?;
    if target.exists() {
        return Err(format!("Go {version} 已安装：{}", display_path(&target)));
    }
    emit_task_progress(&app, &task, 18, "正在下载 Go");
    download_file_with_progress(
        &release.url,
        &archive,
        release.sha256.as_deref(),
        Some((&app, &task, 18, 70)),
    )?;
    emit_task_progress(&app, &task, 72, "正在解压 Go");
    install_zip_payload(&archive, &target, &["bin/go.exe"])?;
    emit_task_progress(&app, &task, 88, "正在验证 Go");
    let output = run_command_output(target.join("bin/go.exe"), &["version"], 30)?;
    record_install(
        &paths,
        runtime_meta("go")?,
        version,
        &target,
        &target.join("bin/go.exe"),
        json!({
            "detail": output.lines().next().unwrap_or(&release.tag),
            "tag": release.tag,
        }),
    )?;
    switch_runtime_blocking("go".to_string(), version.to_string(), None)?;
    emit_task_progress(&app, &task, 100, "安装完成");
    Ok(OperationResult {
        success: true,
        message: format!("安装成功 Go {version}"),
    })
}

#[tauri::command]
async fn install_python(app: tauri::AppHandle, version: String) -> Result<OperationResult, String> {
    run_blocking(move || install_python_blocking(app, version)).await?
}

fn install_python_blocking(
    app: tauri::AppHandle,
    version: String,
) -> Result<OperationResult, String> {
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
    let archive = paths.downloads().join(&release.name);
    let target = paths.pythons().join(format!("python-{version}"));
    paths.assert_inside_root(&target)?;
    if target.exists() {
        if locate_python_exe(&target).is_some() {
            return Err(format!(
                "Python {version} 已安装：{}",
                display_path(&target)
            ));
        }
        let failed = paths
            .pythons()
            .join(format!("python-{version}.failed-{}", filename_timestamp()));
        fs::rename(&target, &failed).map_err(|err| {
            format!(
                "发现上次安装留下的空目录，但无法保留为失败备份：{}：{err}",
                display_path(&target)
            )
        })?;
    }
    emit_task_progress(&app, &task, 20, "正在下载 Python 官方 NuGet 完整包");
    download_file_with_progress(&release.url, &archive, None, Some((&app, &task, 20, 62)))?;
    emit_task_progress(&app, &task, 64, "正在解压到受管目录");
    install_zip_payload(&archive, &target, &["python.exe"])?;
    let python_exe = locate_python_exe(&target).ok_or_else(|| {
        format!(
            "解压完成，但没有在目标目录找到 python.exe：{}",
            display_path(&target)
        )
    })?;
    let python_home = python_exe
        .parent()
        .ok_or_else(|| "无法识别 Python 安装目录".to_string())?
        .to_path_buf();
    emit_task_progress(&app, &task, 72, "正在启用 pip 与 venv");
    run_command_output(python_exe.clone(), &["-m", "ensurepip", "--upgrade"], 180)?;
    let bundled_pip = python_home.join("Lib").join("ensurepip").join("_bundled");
    if !bundled_pip.is_dir() {
        return Err(format!(
            "Python 完整包缺少内置 pip wheel：{}",
            display_path(&bundled_pip)
        ));
    }
    let bundled_pip_arg = display_path(&bundled_pip);
    run_command_output(
        python_exe.clone(),
        &[
            "-m",
            "pip",
            "install",
            "--no-index",
            "--force-reinstall",
            "--no-warn-script-location",
            "--find-links",
            &bundled_pip_arg,
            "pip",
        ],
        180,
    )?;
    run_command_output(python_exe.clone(), &["-m", "venv", "--help"], 60)?;
    emit_task_progress(&app, &task, 88, "正在验证 Python 和 pip");
    let verify = run_command_output(python_exe.clone(), &["--version"], 30)?;
    run_command_output(python_exe.clone(), &["-m", "pip", "--version"], 30)?;
    let pip_exe = python_home.join("Scripts").join("pip.exe");
    if !pip_exe.is_file() {
        return Err(format!(
            "pip 模块可用，但没有生成命令入口：{}",
            display_path(&pip_exe)
        ));
    }
    run_command_output(pip_exe, &["--version"], 30)?;
    record_install(
        &paths,
        runtime_meta("python")?,
        version,
        &python_home,
        &python_exe,
        json!({
            "detail": verify.lines().next().unwrap_or(&release.tag),
            "install_mode": "managed-nuget",
            "archive": display_path(&archive),
        }),
    )?;
    switch_runtime_blocking("python".to_string(), version.to_string(), None)?;
    emit_task_progress(&app, &task, 100, "安装完成");
    Ok(OperationResult {
        success: true,
        message: format!("安装成功 Python {version}"),
    })
}

#[tauri::command]
async fn install_maven_latest(app: tauri::AppHandle) -> Result<OperationResult, String> {
    run_blocking(move || install_maven_latest_blocking(app)).await?
}

fn install_maven_latest_blocking(app: tauri::AppHandle) -> Result<OperationResult, String> {
    let task = "Maven".to_string();
    emit_task_progress(&app, &task, 3, "正在查询 Maven 版本");
    let paths = load_paths()?;
    paths.ensure().map_err(|err| err.to_string())?;
    let release = resolve_maven_release()?;
    let archive = paths.downloads().join(&release.name);
    let target = paths.mavens().join(format!("maven-{}", release.tag));
    paths.assert_inside_root(&target)?;
    if target.exists() {
        return Err(format!(
            "Maven {} 已安装：{}",
            release.tag,
            display_path(&target)
        ));
    }
    emit_task_progress(&app, &task, 18, "正在下载 Maven");
    download_file_with_progress(&release.url, &archive, None, Some((&app, &task, 18, 70)))?;
    emit_task_progress(&app, &task, 72, "正在解压 Maven");
    install_zip_payload(&archive, &target, &["bin/mvn.cmd"])?;
    emit_task_progress(&app, &task, 88, "正在验证 Maven");
    let output = run_managed_command_output(&paths, target.join("bin/mvn.cmd"), &["-v"], 60)?;
    record_install(
        &paths,
        runtime_meta("maven")?,
        &release.tag,
        &target,
        &target.join("bin/mvn.cmd"),
        json!({ "detail": output.lines().next().unwrap_or("") }),
    )?;
    switch_runtime_blocking("maven".to_string(), release.tag.clone(), None)?;
    emit_task_progress(&app, &task, 100, "安装完成");
    Ok(OperationResult {
        success: true,
        message: format!("安装成功 Maven {}", release.tag),
    })
}

#[tauri::command]
async fn install_gradle_latest(app: tauri::AppHandle) -> Result<OperationResult, String> {
    run_blocking(move || install_gradle_latest_blocking(app)).await?
}

fn install_gradle_latest_blocking(app: tauri::AppHandle) -> Result<OperationResult, String> {
    let task = "Gradle".to_string();
    emit_task_progress(&app, &task, 3, "正在查询 Gradle 版本");
    let paths = load_paths()?;
    paths.ensure().map_err(|err| err.to_string())?;
    let release = resolve_gradle_release()?;
    let archive = paths.downloads().join(&release.name);
    let target = paths.gradles().join(format!("gradle-{}", release.tag));
    paths.assert_inside_root(&target)?;
    if target.exists() {
        return Err(format!(
            "Gradle {} 已安装：{}",
            release.tag,
            display_path(&target)
        ));
    }
    emit_task_progress(&app, &task, 18, "正在下载 Gradle");
    download_file_with_progress(
        &release.url,
        &archive,
        release.sha256.as_deref(),
        Some((&app, &task, 18, 70)),
    )?;
    emit_task_progress(&app, &task, 72, "正在解压 Gradle");
    install_zip_payload(&archive, &target, &["bin/gradle.bat"])?;
    emit_task_progress(&app, &task, 88, "正在验证 Gradle");
    let output = run_managed_command_output(&paths, target.join("bin/gradle.bat"), &["-v"], 120)?;
    record_install(
        &paths,
        runtime_meta("gradle")?,
        &release.tag,
        &target,
        &target.join("bin/gradle.bat"),
        json!({ "detail": output.lines().next().unwrap_or("") }),
    )?;
    switch_runtime_blocking("gradle".to_string(), release.tag.clone(), None)?;
    emit_task_progress(&app, &task, 100, "安装完成");
    Ok(OperationResult {
        success: true,
        message: format!("安装成功 Gradle {}", release.tag),
    })
}

#[tauri::command]
async fn switch_runtime(
    kind: String,
    version: String,
    path: Option<String>,
) -> Result<OperationResult, String> {
    run_blocking(move || switch_runtime_blocking(kind, version, path)).await?
}

fn switch_runtime_blocking(
    kind: String,
    version: String,
    path: Option<String>,
) -> Result<OperationResult, String> {
    let paths = load_paths()?;
    let meta = runtime_meta(&kind)?;
    let mut installed = load_installed(&paths)?;
    let requested_path = path.as_deref().map(path_key);
    let record = collection(&installed, meta.collection)
        .iter()
        .find(|item| {
            if let Some(requested) = requested_path.as_deref() {
                item.get("path")
                    .and_then(Value::as_str)
                    .map(path_key)
                    .as_deref()
                    == Some(requested)
            } else {
                item.get("version")
                    .and_then(Value::as_str)
                    .map(|installed_version| {
                        installed_version == version
                            || installed_version.starts_with(&format!("{version}-"))
                    })
                    .unwrap_or(false)
            }
        })
        .cloned()
        .ok_or_else(|| format!("尚未安装 {} {}", meta.kind, version))?;
    let selected_version = record
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or(version.as_str())
        .to_string();
    let target = PathBuf::from(record.get("path").and_then(Value::as_str).unwrap_or(""));
    if !target.exists() {
        return Err(format!("版本目录不存在：{}", display_path(&target)));
    }
    let previous_current = installed.current.clone();
    let previous_environment = (meta.kind == "jdk")
        .then(user_environment)
        .transpose()?
        .unwrap_or_default();
    if meta.kind == "jdk" {
        save_json(
            &paths.env_backup_file(),
            &json!({
                "created_at": current_timestamp(),
                "reason": format!("切换 JDK 到 {selected_version} 前的快照"),
                "DEVENV_HOME": previous_environment.get("DEVENV_HOME"),
                "JAVA_HOME": previous_environment.get("JAVA_HOME"),
                "Path": previous_environment.get("Path").or_else(|| previous_environment.get("PATH")),
            }),
        )?;
    }
    switch_junction(&paths.current().join(meta.link_name), &target, &paths.root)?;
    set_current(&mut installed, meta.kind, Some(selected_version.clone()));
    save_json(&paths.installed_file(), &installed)?;
    if meta.kind == "jdk" {
        if let Err(error) = refresh_user_java_home(&paths) {
            if let Some(previous_version) = previous_current.jdk.as_deref() {
                if let Some(previous_record) = installed.jdks.iter().find(|item| {
                    item.get("version").and_then(Value::as_str) == Some(previous_version)
                }) {
                    if let Some(previous_path) = previous_record.get("path").and_then(Value::as_str)
                    {
                        let _ = switch_junction(
                            &paths.current().join(meta.link_name),
                            Path::new(previous_path),
                            &paths.root,
                        );
                    }
                }
            } else {
                let _ = remove_junction(&paths.current().join(meta.link_name));
            }
            installed.current = previous_current;
            let _ = save_json(&paths.installed_file(), &installed);
            let previous_path = previous_environment
                .get("Path")
                .or_else(|| previous_environment.get("PATH"))
                .cloned()
                .unwrap_or_default();
            let _ = restore_environment_values(
                previous_environment.get("DEVENV_HOME").map(String::as_str),
                previous_environment.get("JAVA_HOME").map(String::as_str),
                &previous_path,
            );
            broadcast_environment_change();
            return Err(format!("JDK 切换验证失败，已恢复上一个环境：{error}"));
        }
    }
    Ok(OperationResult {
        success: true,
        message: format!("已切换当前 {} 到 {}", meta.kind, selected_version),
    })
}

#[tauri::command]
async fn uninstall_runtime(
    kind: String,
    version: String,
    path: Option<String>,
) -> Result<OperationResult, String> {
    run_blocking(move || uninstall_runtime_blocking(kind, version, path)).await?
}

fn uninstall_runtime_blocking(
    kind: String,
    version: String,
    path: Option<String>,
) -> Result<OperationResult, String> {
    let paths = load_paths()?;
    let meta = runtime_meta(&kind)?;
    let mut installed = load_installed(&paths)?;
    let requested_path = path.as_deref().map(path_key);
    let records = collection_mut(&mut installed, meta.collection);
    let index = records
        .iter()
        .position(|item| {
            if let Some(requested) = requested_path.as_deref() {
                item.get("path")
                    .and_then(Value::as_str)
                    .map(path_key)
                    .as_deref()
                    == Some(requested)
            } else {
                item.get("version").and_then(Value::as_str) == Some(version.as_str())
            }
        })
        .ok_or_else(|| format!("未找到 DevEnv 管理的 {} {}", meta.kind, version))?;
    let record = records[index].clone();
    let selected_version = record
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or(version.as_str())
        .to_string();
    let target = PathBuf::from(record.get("path").and_then(Value::as_str).unwrap_or(""));
    let expected_parent = runtime_parent(&paths, meta.collection)?;
    if target.parent() != Some(expected_parent.as_path()) {
        return Err(format!("拒绝删除非标准受管目录：{}", display_path(&target)));
    }
    if current_version(&installed, meta.kind).as_deref() == Some(selected_version.as_str()) {
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
        message: format!("已卸载 {} {}", meta.kind, selected_version),
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
    let output = hidden_command("taskkill").args(&args).output();
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
async fn scan_ports() -> Result<Vec<PortRecord>, String> {
    run_blocking(scan_ports_blocking).await?
}

fn scan_ports_blocking() -> Result<Vec<PortRecord>, String> {
    let output = hidden_command("netstat")
        .args(["-ano"])
        .output()
        .map_err(|err| format!("无法执行 netstat: {err}"))?;

    let text = String::from_utf8_lossy(&output.stdout);
    let system = sysinfo::System::new_all();
    let services = windows_service_map();
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
        let (process_path, command_line, parent_pid, parent_process_name) =
            process_details(&system, pid);
        let service_names = services.get(&pid).cloned().unwrap_or_default();
        let common_usage = port_common_usage(local_port, &process_name);
        let explanation = port_explanation(local_port, &process_name, &common_usage);
        let risk = classify_port(local_port, pid, &process_name);

        records.push(PortRecord {
            protocol,
            local_address,
            local_port,
            remote_address: remote.to_string(),
            state,
            pid,
            process_name,
            process_path,
            command_line,
            parent_pid,
            parent_process_name,
            service_names,
            common_usage,
            explanation,
            risk,
        });
    }

    records.sort_by(|a, b| {
        a.local_port
            .cmp(&b.local_port)
            .then(a.protocol.cmp(&b.protocol))
            .then(a.pid.cmp(&b.pid))
    });
    let _ = update_port_history(&records);
    Ok(records)
}

#[tauri::command]
fn port_history() -> Result<Vec<PortHistorySummary>, String> {
    let paths = load_paths()?;
    let entries: Vec<PortHistoryEntry> =
        load_json_with_default(&paths.port_history_file(), Vec::new())?;
    let mut grouped: std::collections::HashMap<(u16, String), PortHistorySummary> =
        std::collections::HashMap::new();
    for entry in entries {
        let key = (entry.port, entry.process_name.to_ascii_lowercase());
        let summary = grouped.entry(key).or_insert_with(|| PortHistorySummary {
            port: entry.port,
            process_name: entry.process_name.clone(),
            observations: 0,
            last_seen: 0,
        });
        summary.observations += 1;
        summary.last_seen = summary.last_seen.max(entry.observed_at);
    }
    let mut summaries = grouped.into_values().collect::<Vec<_>>();
    summaries.sort_by(|a, b| {
        b.observations
            .cmp(&a.observations)
            .then(b.last_seen.cmp(&a.last_seen))
    });
    summaries.truncate(50);
    Ok(summaries)
}

#[tauri::command]
fn open_process_location(pid: u32) -> Result<OperationResult, String> {
    if pid == 0 {
        return Err("PID 无效".to_string());
    }
    let mut system = sysinfo::System::new_all();
    system.refresh_all();
    let process = system
        .process(sysinfo::Pid::from_u32(pid))
        .ok_or_else(|| format!("PID {pid} 已不存在，请重新扫描端口"))?;
    let path = process
        .exe()
        .ok_or_else(|| "当前权限无法读取该进程路径".to_string())?;
    if !path.is_file() {
        return Err("进程文件已不存在".to_string());
    }
    #[cfg(windows)]
    {
        hidden_command("explorer.exe")
            .arg(format!("/select,{}", display_path(path)))
            .spawn()
            .map_err(|err| format!("打开进程位置失败：{err}"))?;
    }
    #[cfg(not(windows))]
    {
        return Err("打开进程位置目前仅支持 Windows".to_string());
    }
    Ok(OperationResult {
        success: true,
        message: format!("已打开 {}", display_path(path)),
    })
}

#[tauri::command]
async fn inspect_project_port_configs(path: String) -> Result<Vec<ProjectPortConfig>, String> {
    run_blocking(move || inspect_project_port_configs_blocking(Path::new(path.trim()))).await?
}

#[tauri::command]
async fn update_project_port(
    path: String,
    config_id: String,
    new_port: u16,
) -> Result<OperationResult, String> {
    run_blocking(move || update_project_port_blocking(Path::new(path.trim()), &config_id, new_port))
        .await?
}

fn inspect_project_port_configs_blocking(root: &Path) -> Result<Vec<ProjectPortConfig>, String> {
    if !root.is_dir() {
        return Err("项目目录不存在".to_string());
    }
    let root = root
        .canonicalize()
        .map_err(|err| format!("解析项目目录失败：{err}"))?;
    let mut configs = Vec::new();
    let files = [
        (
            "spring-properties",
            root.join("src/main/resources/application.properties"),
        ),
        (
            "spring-yaml",
            root.join("src/main/resources/application.yml"),
        ),
        (
            "spring-yaml",
            root.join("src/main/resources/application.yaml"),
        ),
        ("tomcat", root.join("conf/server.xml")),
        ("vite", root.join("vite.config.ts")),
        ("vite", root.join("vite.config.js")),
        ("vite", root.join("vite.config.mts")),
        ("vite", root.join("vite.config.mjs")),
        ("env", root.join(".env")),
        ("env", root.join(".env.local")),
    ];
    for (kind, file) in files {
        if !file.is_file()
            || file.metadata().map(|item| item.len()).unwrap_or(u64::MAX) > 2 * 1024 * 1024
        {
            continue;
        }
        let text = fs::read_to_string(&file)
            .map_err(|err| format!("读取端口配置失败 {}：{err}", display_path(&file)))?;
        match kind {
            "spring-properties" => {
                find_key_value_ports(
                    &root,
                    &file,
                    kind,
                    &text,
                    &["server.port"],
                    "Spring Boot server.port",
                    &mut configs,
                );
            }
            "env" => {
                find_key_value_ports(
                    &root,
                    &file,
                    kind,
                    &text,
                    &["PORT", "VITE_PORT"],
                    ".env 端口变量",
                    &mut configs,
                );
            }
            "spring-yaml" => find_spring_yaml_ports(&root, &file, &text, &mut configs),
            "tomcat" => find_inline_ports(
                &root,
                &file,
                kind,
                &text,
                "port=\"",
                "Tomcat Connector",
                &mut configs,
            ),
            "vite" => find_inline_ports(
                &root,
                &file,
                kind,
                &text,
                "port:",
                "Vite server.port",
                &mut configs,
            ),
            _ => {}
        }
    }
    if !configs.iter().any(|item| item.kind.starts_with("spring"))
        && project_uses_spring_boot(&root)
    {
        let file = root.join("src/main/resources/application.properties");
        configs.push(ProjectPortConfig {
            id: project_port_config_id(&file, "spring-properties-new", 0),
            kind: "spring-properties-new".to_string(),
            file: display_path(file),
            current_port: 8080,
            line: 0,
            description: "Spring Boot 默认端口（将创建 server.port）".to_string(),
        });
    }
    configs.sort_by(|left, right| left.file.cmp(&right.file).then(left.line.cmp(&right.line)));
    Ok(configs)
}

fn find_key_value_ports(
    root: &Path,
    file: &Path,
    kind: &str,
    text: &str,
    keys: &[&str],
    description: &str,
    output: &mut Vec<ProjectPortConfig>,
) {
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        for key in keys {
            let Some(rest) = trimmed.strip_prefix(key) else {
                continue;
            };
            let Some(value) = rest.trim_start().strip_prefix('=') else {
                continue;
            };
            if let Some(port) = leading_port(value.trim()) {
                push_project_port(root, file, kind, port, index + 1, description, output);
            }
        }
    }
}

fn find_spring_yaml_ports(
    root: &Path,
    file: &Path,
    text: &str,
    output: &mut Vec<ProjectPortConfig>,
) {
    let mut server_indent = None;
    for (index, line) in text.lines().enumerate() {
        let indent = line.len().saturating_sub(line.trim_start().len());
        let trimmed = line.trim();
        if trimmed == "server:" {
            server_indent = Some(indent);
            continue;
        }
        if let Some(parent_indent) = server_indent {
            if !trimmed.is_empty() && indent <= parent_indent {
                server_indent = None;
                continue;
            }
            if let Some(value) = trimmed.strip_prefix("port:") {
                if let Some(port) = leading_port(value.trim()) {
                    push_project_port(
                        root,
                        file,
                        "spring-yaml",
                        port,
                        index + 1,
                        "Spring Boot YAML server.port",
                        output,
                    );
                }
            }
        }
    }
}

fn find_inline_ports(
    root: &Path,
    file: &Path,
    kind: &str,
    text: &str,
    marker: &str,
    description: &str,
    output: &mut Vec<ProjectPortConfig>,
) {
    for (index, line) in text.lines().enumerate() {
        let Some(position) = line.find(marker) else {
            continue;
        };
        let rest = line[position + marker.len()..].trim_start_matches([' ', '\'', '"']);
        if let Some(port) = leading_port(rest) {
            push_project_port(root, file, kind, port, index + 1, description, output);
        }
    }
}

fn leading_port(value: &str) -> Option<u16> {
    let digits = value
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    digits.parse::<u16>().ok().filter(|port| *port > 0)
}

fn push_project_port(
    root: &Path,
    file: &Path,
    kind: &str,
    port: u16,
    line: usize,
    description: &str,
    output: &mut Vec<ProjectPortConfig>,
) {
    if file.starts_with(root) {
        output.push(ProjectPortConfig {
            id: project_port_config_id(file, kind, line),
            kind: kind.to_string(),
            file: display_path(file),
            current_port: port,
            line,
            description: description.to_string(),
        });
    }
}

fn project_port_config_id(file: &Path, kind: &str, line: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(file.to_string_lossy().to_ascii_lowercase().as_bytes());
    hasher.update(b"\0");
    hasher.update(kind.as_bytes());
    hasher.update(b"\0");
    hasher.update(line.to_string().as_bytes());
    format!("{:x}", hasher.finalize())
}

fn project_uses_spring_boot(root: &Path) -> bool {
    ["pom.xml", "build.gradle", "build.gradle.kts"]
        .iter()
        .filter_map(|name| fs::read_to_string(root.join(name)).ok())
        .any(|text| text.to_ascii_lowercase().contains("spring-boot"))
}

fn update_project_port_blocking(
    root: &Path,
    config_id: &str,
    new_port: u16,
) -> Result<OperationResult, String> {
    if !(1024..=65535).contains(&new_port) {
        return Err("项目端口必须在 1024 到 65535 之间".to_string());
    }
    let configs = inspect_project_port_configs_blocking(root)?;
    let config = configs
        .into_iter()
        .find(|item| item.id == config_id)
        .ok_or_else(|| "端口配置已变化，请重新分析项目".to_string())?;
    let file = PathBuf::from(&config.file);
    if config.kind == "spring-properties-new" {
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent).map_err(|err| format!("创建资源目录失败：{err}"))?;
        }
        fs::write(&file, format!("server.port={new_port}\n"))
            .map_err(|err| format!("创建 Spring Boot 端口配置失败：{err}"))?;
    } else {
        let text = fs::read_to_string(&file).map_err(|err| format!("读取端口配置失败：{err}"))?;
        let mut lines = text.lines().map(str::to_string).collect::<Vec<_>>();
        let index = config
            .line
            .checked_sub(1)
            .ok_or_else(|| "端口配置行无效".to_string())?;
        let line = lines
            .get_mut(index)
            .ok_or_else(|| "端口配置行已变化，请重新分析".to_string())?;
        let old = config.current_port.to_string();
        if !line.contains(&old) {
            return Err("端口配置内容已变化，请重新分析".to_string());
        }
        let backup = file.with_file_name(format!(
            "{}.devenv-backup-{}",
            file.file_name()
                .and_then(OsStr::to_str)
                .unwrap_or("port-config"),
            filename_timestamp()
        ));
        fs::copy(&file, &backup).map_err(|err| format!("备份端口配置失败：{err}"))?;
        *line = line.replacen(&old, &new_port.to_string(), 1);
        let trailing_newline = text.ends_with('\n');
        let mut updated = lines.join("\n");
        if trailing_newline {
            updated.push('\n');
        }
        fs::write(&file, updated).map_err(|err| format!("写入端口配置失败：{err}"))?;
    }
    let verified = inspect_project_port_configs_blocking(root)?
        .iter()
        .any(|item| item.file == config.file && item.current_port == new_port);
    if !verified {
        return Err("端口配置已写入，但重新分析未通过验证".to_string());
    }
    Ok(OperationResult {
        success: true,
        message: format!(
            "已将 {} 从 {} 修改为 {}",
            config.description, config.current_port, new_port
        ),
    })
}

#[tauri::command]
fn project_health(path: String) -> Result<ProjectHealth, String> {
    let analysis = analyze_project_blocking(&PathBuf::from(path.trim()))?;
    let suggestions = analysis
        .actions
        .iter()
        .map(|item| format!("{}：{}", item.title, item.command))
        .chain(analysis.warnings.iter().cloned())
        .collect::<Vec<_>>();

    Ok(ProjectHealth {
        root: analysis.root,
        project_types: analysis.project_types,
        signals: analysis.detected_files,
        suggestions,
    })
}

#[tauri::command]
async fn network_diagnostics() -> NetworkDiagnostics {
    run_blocking(network_diagnostics_blocking)
        .await
        .unwrap_or_else(|_| NetworkDiagnostics {
            checks: Vec::new(),
            proxy: Vec::new(),
        })
}

fn network_diagnostics_blocking() -> NetworkDiagnostics {
    let endpoints = [
        ("GitHub", "https://github.com"),
        ("Python 官网", "https://www.python.org"),
        ("Node.js 官网", "https://nodejs.org/dist/index.json"),
        (
            "Adoptium API",
            "https://api.adoptium.net/v3/info/available_releases",
        ),
        (
            "Apache Maven",
            "https://downloads.apache.org/maven/maven-3/",
        ),
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
    for item in fs::read_dir(paths.downloads()).map_err(|err| format!("读取缓存目录失败：{err}"))?
    {
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
    for item in fs::read_dir(&downloads).map_err(|err| format!("读取缓存目录失败：{err}"))?
    {
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
async fn run_tool_command(
    command: String,
    cwd: Option<String>,
    confirmed: Option<bool>,
) -> Result<CommandRunResult, String> {
    run_blocking(move || {
        let assessment = assess_command_safety(&command)?;
        if !assessment.allowed {
            return Err(format!("命令已被安全模式拦截：{}", assessment.reason));
        }
        if assessment.requires_confirmation && confirmed != Some(true) {
            return Err(format!("该命令需要确认：{}", assessment.reason));
        }
        run_tool_command_blocking(command, cwd)
    })
    .await?
}

#[tauri::command]
fn inspect_command_safety(command: String) -> Result<CommandSafetyAssessment, String> {
    assess_command_safety(&command)
}

fn is_process_elevated() -> bool {
    #[cfg(windows)]
    {
        hidden_command("whoami")
            .args(["/groups", "/fo", "csv", "/nh"])
            .output()
            .ok()
            .map(|output| command_text(&output.stdout, &output.stderr))
            .is_some_and(|text| {
                text.contains("S-1-16-12288")
                    || text.contains("S-1-16-16384")
                    || text.contains("S-1-5-32-544") && text.contains("Enabled group")
            })
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn assess_command_safety(command: &str) -> Result<CommandSafetyAssessment, String> {
    let parts = parse_command_line(command)?;
    let executable = parts
        .first()
        .ok_or_else(|| "命令不能为空".to_string())?
        .trim_matches('"')
        .to_ascii_lowercase();
    let name = Path::new(&executable)
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or(&executable)
        .trim_end_matches(".exe")
        .trim_end_matches(".cmd")
        .trim_end_matches(".bat")
        .to_string();
    let elevated = is_process_elevated();
    let blocked = [
        "cmd",
        "powershell",
        "pwsh",
        "reg",
        "regedit",
        "diskpart",
        "format",
        "bcdedit",
        "takeown",
        "icacls",
        "shutdown",
        "taskkill",
        "sc",
        "net",
        "netsh",
        "wmic",
        "cipher",
    ];
    if blocked.contains(&name.as_str()) {
        return Ok(CommandSafetyAssessment {
            allowed: false,
            risk: "blocked".to_string(),
            reason: "系统 Shell、磁盘、注册表、权限或服务管理命令不允许从命令面板运行".to_string(),
            requires_confirmation: false,
            elevated,
            executable: name,
        });
    }
    let allowed = [
        "node", "npm", "npx", "pnpm", "yarn", "corepack", "python", "py", "pip", "uv", "poetry",
        "java", "javac", "mvn", "mvnw", "gradle", "gradlew", "git", "go", "rustc", "cargo",
        "rustup", "dotnet", "devenv",
    ];
    if !allowed.contains(&name.as_str()) {
        return Ok(CommandSafetyAssessment {
            allowed: false,
            risk: "blocked".to_string(),
            reason: format!("{name} 不在开发工具白名单中"),
            requires_confirmation: false,
            elevated,
            executable: name,
        });
    }
    let lower = parts
        .iter()
        .skip(1)
        .map(|part| part.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let destructive_git = name == "git"
        && (lower.windows(2).any(|pair| pair == ["reset", "--hard"])
            || lower.iter().any(|arg| arg == "clean")
            || lower.windows(2).any(|pair| pair == ["branch", "-d"]));
    if destructive_git {
        return Ok(CommandSafetyAssessment {
            allowed: false,
            risk: "blocked".to_string(),
            reason: "破坏工作区或删除分支的 Git 命令已被拦截，请在受控终端中自行确认".to_string(),
            requires_confirmation: false,
            elevated,
            executable: name,
        });
    }
    let inline_code = matches!(name.as_str(), "python" | "py")
        && lower.iter().any(|arg| arg == "-c")
        || name == "node" && lower.iter().any(|arg| arg == "-e" || arg == "--eval");
    if inline_code {
        return Ok(CommandSafetyAssessment {
            allowed: false,
            risk: "blocked".to_string(),
            reason: "命令面板禁止解释器内联代码（python -c / node -e），避免把未知文本直接执行"
                .to_string(),
            requires_confirmation: false,
            elevated,
            executable: name,
        });
    }
    let changes_state = lower.iter().any(|arg| {
        matches!(
            arg.as_str(),
            "install" | "uninstall" | "update" | "upgrade" | "add" | "remove" | "publish" | "push"
        )
    });
    Ok(CommandSafetyAssessment {
        allowed: true,
        risk: if elevated || changes_state {
            "caution"
        } else {
            "low"
        }
        .to_string(),
        reason: if elevated {
            "程序当前可能处于管理员权限，命令影响范围更大".to_string()
        } else if changes_state {
            "命令可能安装、更新或发布内容，请确认来源和影响范围".to_string()
        } else {
            "命令属于常见开发工具白名单".to_string()
        },
        requires_confirmation: elevated || changes_state,
        elevated,
        executable: name,
    })
}

fn run_tool_command_blocking(
    command: String,
    cwd: Option<String>,
) -> Result<CommandRunResult, String> {
    let parts = parse_command_line(&command)?;
    let executable = parts.first().ok_or_else(|| "命令不能为空".to_string())?;
    let started = Instant::now();
    let mut cmd = hidden_command(executable);
    cmd.args(parts.iter().skip(1));
    if let Ok(paths) = load_paths() {
        apply_managed_environment(&paths, &mut cmd);
    }
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
async fn environment_health() -> Result<Vec<EnvHealthCheck>, String> {
    run_blocking(environment_health_blocking).await?
}

fn environment_health_blocking() -> Result<Vec<EnvHealthCheck>, String> {
    let paths = load_paths()?;
    let env = env_snapshot();
    let mut checks = Vec::new();

    checks.push(EnvHealthCheck {
        name: "DEVENV_HOME".to_string(),
        status: if env.devenv_home.as_deref().map(path_key)
            == Some(path_key(&display_path(&paths.root)))
        {
            "正常".to_string()
        } else {
            "需配置".to_string()
        },
        detail: env
            .devenv_home
            .clone()
            .unwrap_or_else(|| "未设置，点击“配置”写入受管根目录".to_string()),
    });

    checks.push(EnvHealthCheck {
        name: "PATH".to_string(),
        status: if env
            .path_warnings
            .iter()
            .any(|item| item.starts_with("失效 PATH") || item.starts_with("重复 PATH"))
        {
            "需清理".to_string()
        } else {
            "正常".to_string()
        },
        detail: if env.path_warnings.is_empty() {
            format!("{} 个条目，没有真实失效或重复项", env.path_entries.len())
        } else {
            env.path_warnings.join("；")
        },
    });

    if let Some(java_home) = env.java_home.as_deref() {
        checks.push(EnvHealthCheck {
            name: "JAVA_HOME".to_string(),
            status: if is_valid_java_home(java_home, &paths) {
                "正常".to_string()
            } else {
                "异常".to_string()
            },
            detail: java_home.to_string(),
        });
    } else {
        checks.push(EnvHealthCheck {
            name: "JAVA_HOME".to_string(),
            status: "未设置".to_string(),
            detail: "未发现 JAVA_HOME；安装或切换 JDK 后会自动配置".to_string(),
        });
    }

    if let Ok(java) = inspect_java_environment_blocking() {
        checks.push(EnvHealthCheck {
            name: "JDK 生效链".to_string(),
            status: if java.consistent { "正常" } else { "异常" }.to_string(),
            detail: if java.consistent {
                format!("{} · {}", java.java_version, java.effective_source)
            } else {
                java.warnings.join("；")
            },
        });
    }

    for (name, executable, args) in [
        (
            "JDK",
            paths.current().join("jdk/bin/java.exe"),
            vec!["-version"],
        ),
        (
            "Python",
            paths.current().join("python/python.exe"),
            vec!["--version"],
        ),
        ("Node.js", paths.current().join("node/node.exe"), vec!["-v"]),
    ] {
        checks.push(check_executable_health(name, &executable, &args));
    }
    checks.push(check_managed_executable_health(
        &paths,
        "Maven",
        &paths.current().join("maven/bin/mvn.cmd"),
        &["-v"],
    ));
    checks.push(check_managed_executable_health(
        &paths,
        "Gradle",
        &paths.current().join("gradle/bin/gradle.bat"),
        &["-v"],
    ));
    if load_installed(&paths)?.current.go.is_some() {
        checks.push(check_executable_health(
            "Go",
            &paths.current().join("go/bin/go.exe"),
            &["version"],
        ));
    }

    Ok(checks)
}

#[tauri::command]
async fn run_doctor() -> Result<DoctorReport, String> {
    run_blocking(run_doctor_blocking).await?
}

fn run_doctor_blocking() -> Result<DoctorReport, String> {
    let paths = load_paths()?;
    let env = env_snapshot();
    let health = environment_health_blocking().unwrap_or_default();
    let runtimes = discover_runtimes_blocking();
    let ports = scan_ports_blocking().unwrap_or_default();
    let network = network_diagnostics_blocking();
    let python = analyze_python_environment_blocking();
    let mut score = 100_i32;
    let mut checks = Vec::new();
    let mut suggestions = Vec::new();

    push_doctor_check(
        &mut checks,
        &mut score,
        DoctorCheck {
            id: "devenv-home".to_string(),
            title: "DEVENV_HOME".to_string(),
            category: "环境变量".to_string(),
            status: if env.devenv_home.as_deref().map(path_key)
                == Some(path_key(&display_path(&paths.root)))
            {
                "正常".to_string()
            } else {
                "需修复".to_string()
            },
            severity: if env.devenv_home.as_deref().map(path_key)
                == Some(path_key(&display_path(&paths.root)))
            {
                "info".to_string()
            } else {
                "warning".to_string()
            },
            detail: env
                .devenv_home
                .clone()
                .unwrap_or_else(|| "未设置 DEVENV_HOME".to_string()),
            fix_action: Some("configure_env".to_string()),
        },
    );

    let duplicate_count = env
        .path_warnings
        .iter()
        .filter(|item| item.starts_with("重复 PATH"))
        .count();
    let invalid_count = env
        .path_warnings
        .iter()
        .filter(|item| item.starts_with("失效 PATH"))
        .count();
    push_doctor_check(
        &mut checks,
        &mut score,
        DoctorCheck {
            id: "path-quality".to_string(),
            title: "PATH 重复和失效项".to_string(),
            category: "环境变量".to_string(),
            status: if duplicate_count == 0 && invalid_count == 0 {
                "正常".to_string()
            } else {
                "需清理".to_string()
            },
            severity: if invalid_count > 5 {
                "warning"
            } else if duplicate_count > 0 || invalid_count > 0 {
                "notice"
            } else {
                "info"
            }
            .to_string(),
            detail: format!(
                "PATH 共 {} 项；重复 {} 项；失效 {} 项",
                env.path_entries.len(),
                duplicate_count,
                invalid_count
            ),
            fix_action: Some("cleanup_path".to_string()),
        },
    );

    let managed_missing = MANAGED_PATHS
        .iter()
        .filter(|managed| {
            !env.path_entries
                .iter()
                .any(|entry| path_key(entry) == path_key(managed))
        })
        .count();
    push_doctor_check(
        &mut checks,
        &mut score,
        DoctorCheck {
            id: "managed-paths".to_string(),
            title: "受管 PATH".to_string(),
            category: "环境变量".to_string(),
            status: if managed_missing == 0 {
                "正常"
            } else {
                "缺失"
            }
            .to_string(),
            severity: if managed_missing == 0 {
                "info"
            } else {
                "warning"
            }
            .to_string(),
            detail: if managed_missing == 0 {
                "PATH 已包含 DevEnv Manager 受管目录".to_string()
            } else {
                format!("缺少 {managed_missing} 个受管 PATH 项，安装后可能不能立刻在终端使用")
            },
            fix_action: Some("configure_env".to_string()),
        },
    );

    for check in &health {
        let severe = matches!(
            check.status.as_str(),
            "异常" | "未安装" | "未设置" | "需清理" | "需配置"
        );
        push_doctor_check(
            &mut checks,
            &mut score,
            DoctorCheck {
                id: format!(
                    "runtime-{}",
                    check.name.to_ascii_lowercase().replace([' ', '.'], "-")
                ),
                title: check.name.clone(),
                category: "运行时".to_string(),
                status: check.status.clone(),
                severity: if severe { "warning" } else { "info" }.to_string(),
                detail: check.detail.clone(),
                fix_action: Some("discover_runtimes".to_string()),
            },
        );
    }

    let git_path = resolve_tool(&paths, "git");
    push_doctor_check(
        &mut checks,
        &mut score,
        tool_state_doctor_check(probe_tool("Git", git_path.clone(), &["--version"]), true),
    );
    let git_name = command_value(git_path.clone(), &["config", "--global", "user.name"]);
    let git_email = command_value(git_path, &["config", "--global", "user.email"]);
    let git_identity_ok = !git_name.is_empty() && !git_email.is_empty();
    push_doctor_check(
        &mut checks,
        &mut score,
        DoctorCheck {
            id: "git-identity".to_string(),
            title: "Git 用户身份".to_string(),
            category: "Git".to_string(),
            status: if git_identity_ok {
                "正常"
            } else {
                "未配置"
            }
            .to_string(),
            severity: if git_identity_ok { "info" } else { "notice" }.to_string(),
            detail: if git_identity_ok {
                format!("{git_name} <{git_email}>")
            } else {
                "尚未同时配置 user.name 和 user.email".to_string()
            },
            fix_action: Some("toolchains".to_string()),
        },
    );
    let ssh_key_exists = dirs::home_dir()
        .map(|home| {
            home.join(".ssh/id_ed25519.pub").is_file() || home.join(".ssh/id_rsa.pub").is_file()
        })
        .unwrap_or(false);
    push_doctor_check(
        &mut checks,
        &mut score,
        DoctorCheck {
            id: "git-ssh-key".to_string(),
            title: "GitHub SSH Key".to_string(),
            category: "Git".to_string(),
            status: if ssh_key_exists {
                "已发现"
            } else {
                "未配置"
            }
            .to_string(),
            severity: if ssh_key_exists { "info" } else { "notice" }.to_string(),
            detail: if ssh_key_exists {
                "已发现 ed25519 或 RSA 公钥；报告不会包含私钥".to_string()
            } else {
                "没有发现常用 SSH 公钥，可在工具链页面安全生成".to_string()
            },
            fix_action: Some("toolchains".to_string()),
        },
    );
    for (name, executable, args) in [
        ("npm", "npm", vec!["--version"]),
        ("pnpm", "pnpm", vec!["--version"]),
        ("Yarn", "yarn", vec!["--version"]),
        ("Corepack", "corepack", vec!["--version"]),
    ] {
        let state = probe_tool(name, resolve_tool(&paths, executable), &args);
        push_doctor_check(
            &mut checks,
            &mut score,
            tool_state_doctor_check(state, false),
        );
    }
    let python_executable = resolve_tool(&paths, "python");
    for (name, args) in [
        ("pip", vec!["-m", "pip", "--version"]),
        ("uv", vec!["-m", "uv", "--version"]),
        ("Poetry", vec!["-m", "poetry", "--version"]),
        ("virtualenv", vec!["-m", "virtualenv", "--version"]),
    ] {
        let state = probe_tool(name, python_executable.clone(), &args);
        push_doctor_check(
            &mut checks,
            &mut score,
            tool_state_doctor_check(state, name == "pip"),
        );
    }
    for (name, exe, args) in [
        ("Go", "go", vec!["version"]),
        ("Rust", "rustc", vec!["--version"]),
        (".NET", "dotnet", vec!["--version"]),
    ] {
        push_doctor_check(
            &mut checks,
            &mut score,
            optional_command_probe(name, exe, &args),
        );
    }

    let python_conflict_count = python.discovered_pythons.len();
    let python_store_risk = python
        .discovered_pythons
        .iter()
        .any(|item| item.source == "Microsoft Store");
    let pip_problem = python
        .current_pip
        .as_ref()
        .map(|item| item.status != "正常")
        .unwrap_or(true);
    push_doctor_check(
        &mut checks,
        &mut score,
        DoctorCheck {
            id: "python-conflicts".to_string(),
            title: "Python 多版本和 pip 匹配".to_string(),
            category: "Python".to_string(),
            status: if python_conflict_count <= 1 && !python_store_risk && !pip_problem {
                "正常".to_string()
            } else {
                "需关注".to_string()
            },
            severity: if python_store_risk || pip_problem {
                "warning"
            } else if python_conflict_count > 1 {
                "notice"
            } else {
                "info"
            }
            .to_string(),
            detail: format!(
                "发现 {} 个 Python、{} 个 pip；{}",
                python.discovered_pythons.len(),
                python.discovered_pips.len(),
                python.risks.join("；")
            ),
            fix_action: Some("python_analysis".to_string()),
        },
    );

    let java_count = runtimes.iter().filter(|item| item.kind == "Java").count();
    push_doctor_check(
        &mut checks,
        &mut score,
        DoctorCheck {
            id: "java-conflicts".to_string(),
            title: "JDK 多版本".to_string(),
            category: "Java".to_string(),
            status: if java_count <= 1 {
                "正常"
            } else {
                "多版本"
            }
            .to_string(),
            severity: if java_count <= 1 { "info" } else { "notice" }.to_string(),
            detail: format!("发现 {java_count} 个 JDK/Java 入口"),
            fix_action: Some("discover_runtimes".to_string()),
        },
    );

    let watched_ports = [
        80_u16, 443, 3000, 3306, 5432, 5173, 6379, 8000, 8080, 8081, 8888,
    ];
    for port in watched_ports {
        if let Some(record) = ports.iter().find(|item| item.local_port == port) {
            push_doctor_check(
                &mut checks,
                &mut score,
                DoctorCheck {
                    id: format!("port-{port}"),
                    title: format!("端口 {port} 占用"),
                    category: "端口".to_string(),
                    status: "占用".to_string(),
                    severity: if matches!(port, 80 | 443 | 3306 | 5432 | 6379 | 8080) {
                        "notice"
                    } else {
                        "info"
                    }
                    .to_string(),
                    detail: format!(
                        "{} / PID {} / {}",
                        record.process_name, record.pid, record.risk
                    ),
                    fix_action: Some("ports".to_string()),
                },
            );
        }
    }

    for item in &network.checks {
        push_doctor_check(
            &mut checks,
            &mut score,
            DoctorCheck {
                id: format!("network-{}", slug(&item.name)),
                title: item.name.clone(),
                category: "网络".to_string(),
                status: if item.success {
                    "正常"
                } else {
                    "不可访问"
                }
                .to_string(),
                severity: if item.success { "info" } else { "notice" }.to_string(),
                detail: format!("{} · {} ms · {}", item.status, item.elapsed_ms, item.url),
                fix_action: Some("network".to_string()),
            },
        );
    }

    let cache_size = dir_size(&paths.downloads()).unwrap_or(0);
    push_doctor_check(
        &mut checks,
        &mut score,
        DoctorCheck {
            id: "download-cache".to_string(),
            title: "下载缓存".to_string(),
            category: "缓存".to_string(),
            status: if cache_size > 2 * 1024 * 1024 * 1024 {
                "过大"
            } else {
                "正常"
            }
            .to_string(),
            severity: if cache_size > 2 * 1024 * 1024 * 1024 {
                "notice"
            } else {
                "info"
            }
            .to_string(),
            detail: format!("当前缓存大小 {}", format_size(cache_size)),
            fix_action: Some("cache".to_string()),
        },
    );

    if duplicate_count > 0 || invalid_count > 0 {
        suggestions.push(DoctorSuggestion {
            id: "cleanup-path".to_string(),
            title: "清理失效和重复 PATH".to_string(),
            description: "只清理真实不存在的路径和重复项，保留 DevEnv Manager 待安装的受管目录。"
                .to_string(),
            action: Some("cleanup_path".to_string()),
        });
    }
    if managed_missing > 0 {
        suggestions.push(DoctorSuggestion {
            id: "configure-env".to_string(),
            title: "配置受管环境变量".to_string(),
            description:
                "写入用户级 DEVENV_HOME、JAVA_HOME 和受管 PATH，安装后的工具可直接在新终端使用。"
                    .to_string(),
            action: Some("configure_env".to_string()),
        });
    }
    if python_store_risk || pip_problem || python_conflict_count > 1 {
        suggestions.push(DoctorSuggestion {
            id: "python-analysis".to_string(),
            title: "查看 Python 冲突分析".to_string(),
            description:
                "确认默认 python、pip、py launcher 和 Microsoft Store 执行别名是否互相抢占。"
                    .to_string(),
            action: Some("python_analysis".to_string()),
        });
    }
    suggestions.push(DoctorSuggestion {
        id: "export-report".to_string(),
        title: "导出诊断报告".to_string(),
        description: "生成可分享的 Markdown 报告，自动脱敏用户目录和敏感字段。".to_string(),
        action: Some("export_report".to_string()),
    });

    let final_score = score.clamp(0, 100) as u8;
    let problem_count = checks
        .iter()
        .filter(|item| doctor_check_needs_attention(item))
        .count();
    Ok(DoctorReport {
        score: final_score,
        summary: format!("环境评分 {final_score}/100，发现 {problem_count} 个需要关注的项目。"),
        checks,
        suggestions,
        generated_at: current_timestamp(),
    })
}

#[tauri::command]
async fn export_doctor_report(report: DoctorReport) -> Result<OperationResult, String> {
    run_blocking(move || export_doctor_report_blocking(report)).await?
}

fn export_doctor_report_blocking(report: DoctorReport) -> Result<OperationResult, String> {
    let paths = load_paths()?;
    fs::create_dir_all(paths.logs()).map_err(|err| format!("创建报告目录失败：{err}"))?;
    let filename = format!("doctor-report-{}.md", filename_timestamp());
    let target = paths.logs().join(filename);
    let text = redact_report_text(&doctor_report_markdown(&report));
    fs::write(&target, text).map_err(|err| format!("写入诊断报告失败：{err}"))?;
    Ok(OperationResult {
        success: true,
        message: format!("已导出诊断报告：{}", display_path(target)),
    })
}

#[tauri::command]
async fn export_doctor_report_json(report: DoctorReport) -> Result<OperationResult, String> {
    run_blocking(move || export_doctor_report_json_blocking(report)).await?
}

fn export_doctor_report_json_blocking(report: DoctorReport) -> Result<OperationResult, String> {
    let paths = load_paths()?;
    fs::create_dir_all(paths.logs()).map_err(|err| format!("创建报告目录失败：{err}"))?;
    let target = paths
        .logs()
        .join(format!("doctor-report-{}.json", filename_timestamp()));
    let text = serde_json::to_string_pretty(&report)
        .map_err(|err| format!("生成 JSON 报告失败：{err}"))?;
    fs::write(&target, redact_report_text(&text))
        .map_err(|err| format!("写入 JSON 报告失败：{err}"))?;
    Ok(OperationResult {
        success: true,
        message: format!("已导出 JSON 诊断报告：{}", display_path(target)),
    })
}

#[tauri::command]
fn doctor_report_text(report: DoctorReport, format: String) -> Result<String, String> {
    let text = match format.as_str() {
        "markdown" => doctor_report_markdown(&report),
        "json" => serde_json::to_string_pretty(&report)
            .map_err(|err| format!("生成 JSON 报告失败：{err}"))?,
        _ => return Err("不支持的报告格式".to_string()),
    };
    Ok(redact_report_text(&text))
}

#[tauri::command]
async fn analyze_python_environment() -> Result<PythonAnalysis, String> {
    run_blocking(|| Ok(analyze_python_environment_blocking())).await?
}

fn analyze_python_environment_blocking() -> PythonAnalysis {
    let current_python = detect_runtime("Python", "python", &["--version"]).map(|runtime| {
        let status = if runtime
            .executable
            .to_ascii_lowercase()
            .contains("\\windowsapps\\")
        {
            "风险".to_string()
        } else {
            "正常".to_string()
        };
        PythonToolState {
            path: runtime.executable,
            version: runtime.version,
            status,
            detail: runtime.source,
        }
    });

    let python_m_pip = current_python.as_ref().and_then(|python| {
        run_command_output(PathBuf::from(&python.path), &["-m", "pip", "--version"], 30).ok()
    });
    let pip_runtime = detect_runtime("pip", "pip", &["--version"]);
    let current_pip = pip_runtime.map(|runtime| {
        let pip_output = runtime.version.clone();
        let matches_python = python_m_pip
            .as_deref()
            .map(|expected| same_python_package_location(expected, &pip_output))
            .unwrap_or(false);
        PythonToolState {
            path: runtime.executable,
            version: pip_output.clone(),
            status: if matches_python {
                "正常"
            } else {
                "不匹配"
            }
            .to_string(),
            detail: if matches_python {
                "pip 与当前 python -m pip 指向一致".to_string()
            } else {
                python_m_pip
                    .as_ref()
                    .map(|expected| format!("python -m pip: {expected}"))
                    .unwrap_or_else(|| "当前 Python 无法运行 -m pip".to_string())
            },
        }
    });

    let current_python_key = current_python.as_ref().map(|item| path_key(&item.path));
    let discovered_pythons = python_candidates()
        .into_iter()
        .filter_map(|path| {
            detect_runtime_at("Python", &path, &["--version"], None).map(|runtime| PythonEntry {
                current: current_python_key.as_deref()
                    == Some(path_key(&runtime.executable).as_str()),
                source: runtime.source,
                path: runtime.executable,
                version: runtime.version,
            })
        })
        .collect::<Vec<_>>();
    let current_pip_key = current_pip.as_ref().map(|item| path_key(&item.path));
    let discovered_pips = find_all_on_path("pip")
        .into_iter()
        .filter_map(|path| {
            detect_runtime_at("pip", &path, &["--version"], None).map(|runtime| PythonEntry {
                current: current_pip_key.as_deref() == Some(path_key(&runtime.executable).as_str()),
                source: runtime.source,
                path: runtime.executable,
                version: runtime.version,
            })
        })
        .collect::<Vec<_>>();
    let launcher_output = hidden_command("py")
        .arg("-0p")
        .output()
        .map(|output| command_text(&output.stdout, &output.stderr))
        .unwrap_or_else(|_| "未发现 Python Launcher 或 py -0p 执行失败".to_string());

    let mut risks = Vec::new();
    if discovered_pythons.len() > 1 {
        risks.push(format!(
            "PATH/注册表中发现 {} 个 Python，pip 容易安装到错误版本",
            discovered_pythons.len()
        ));
    }
    if discovered_pythons
        .iter()
        .any(|item| item.source == "Microsoft Store")
    {
        risks.push("Microsoft Store Python 执行别名可能抢占 python 命令".to_string());
    }
    if current_pip.as_ref().map(|item| item.status.as_str()) != Some("正常") {
        risks.push("pip 与当前 python -m pip 不一致或当前 Python 缺少 pip".to_string());
    }
    if risks.is_empty() {
        risks.push("未发现明显 Python 冲突".to_string());
    }

    let mut recommendations = Vec::new();
    recommendations.push("优先使用 DevEnv Manager 受管 Python 或官网安装版 Python。".to_string());
    recommendations.push("安装包时尽量使用 python -m pip，而不是直接运行 pip。".to_string());
    if discovered_pythons
        .iter()
        .any(|item| item.source == "Microsoft Store")
    {
        recommendations.push(
            "如默认 python 指向 WindowsApps，请在 Windows“应用执行别名”中关闭 Python 别名。"
                .to_string(),
        );
    }

    PythonAnalysis {
        current_python,
        current_pip,
        launcher_output,
        discovered_pythons,
        discovered_pips,
        risks,
        recommendations,
        pip_repair_command: "python -m ensurepip --upgrade; python -m pip install --upgrade pip"
            .to_string(),
        alias_settings_command: "start ms-settings:appsfeatures-app".to_string(),
    }
}

#[tauri::command]
async fn inspect_toolchains() -> Result<ToolchainReport, String> {
    run_blocking(inspect_toolchains_blocking).await?
}

fn inspect_toolchains_blocking() -> Result<ToolchainReport, String> {
    let paths = load_paths()?;
    let git = probe_tool("Git", resolve_tool(&paths, "git"), &["--version"]);
    let ssh = probe_tool("OpenSSH", resolve_tool(&paths, "ssh"), &["-V"]);
    let git_lfs = probe_tool("Git LFS", resolve_tool(&paths, "git-lfs"), &["version"]);
    let git_bash_path = git_bash_from_git(&git.path).unwrap_or_default();
    let user_name = command_value(
        resolve_tool(&paths, "git"),
        &["config", "--global", "user.name"],
    );
    let user_email = command_value(
        resolve_tool(&paths, "git"),
        &["config", "--global", "user.email"],
    );
    let ssh_dir = dirs::home_dir().unwrap_or_default().join(".ssh");
    let public_key_path = ["id_ed25519.pub", "id_rsa.pub"]
        .iter()
        .map(|name| ssh_dir.join(name))
        .find(|path| path.is_file())
        .unwrap_or_else(|| ssh_dir.join("id_ed25519.pub"));
    let public_key = fs::read_to_string(&public_key_path)
        .map(|value| value.trim().to_string())
        .unwrap_or_default();
    let github_ssh_status = github_ssh_status(resolve_tool(&paths, "ssh"));
    let github_https_status = github_https_status();

    let node_tools = [
        ("Node.js", "node", vec!["--version"]),
        ("npm", "npm", vec!["--version"]),
        ("npx", "npx", vec!["--version"]),
        ("pnpm", "pnpm", vec!["--version"]),
        ("Yarn", "yarn", vec!["--version"]),
        ("Corepack", "corepack", vec!["--version"]),
    ]
    .into_iter()
    .map(|(name, executable, args)| probe_tool(name, resolve_tool(&paths, executable), &args))
    .collect();
    let npm = resolve_tool(&paths, "npm");
    let pnpm = resolve_tool(&paths, "pnpm");
    let npm_prefix = command_value(npm.clone(), &["config", "get", "prefix"]);
    let npm_registry = command_value(npm, &["config", "get", "registry"]);
    let pnpm_store_path = command_value(pnpm, &["store", "path"]);

    let python = resolve_tool(&paths, "python");
    let python_tools = [
        ("pip", vec!["-m", "pip", "--version"]),
        ("uv", vec!["-m", "uv", "--version"]),
        ("Poetry", vec!["-m", "poetry", "--version"]),
        ("virtualenv", vec!["-m", "virtualenv", "--version"]),
    ]
    .into_iter()
    .map(|(name, args)| probe_tool(name, python.clone(), &args))
    .collect();
    let pip_config = command_value(python.clone(), &["-m", "pip", "config", "list"]);
    let pip_index_url = pip_config_value(&pip_config, "global.index-url");

    Ok(ToolchainReport {
        tools: tool_registry(),
        git: GitEnvironment {
            git,
            git_bash_path,
            user_name,
            user_email,
            ssh,
            ssh_key_exists: public_key_path.is_file(),
            public_key_path: display_path(public_key_path),
            public_key,
            github_ssh_status,
            github_https_status,
            git_lfs,
        },
        node: NodeEcosystem {
            tools: node_tools,
            npm_prefix,
            npm_registry,
            pnpm_store_path,
        },
        python: PythonEcosystem {
            tools: python_tools,
            pip_config,
            pip_index_url,
        },
        generated_at: current_timestamp(),
    })
}

#[tauri::command]
async fn run_toolchain_action(
    app: tauri::AppHandle,
    action: String,
    value: Option<String>,
    secondary: Option<String>,
) -> Result<OperationResult, String> {
    let task = toolchain_action_title(&action).to_string();
    emit_task_progress(&app, &task, 5, "正在准备操作");
    let worker_action = action.clone();
    let result =
        run_blocking(move || run_toolchain_action_blocking(&worker_action, value, secondary))
            .await?;
    emit_task_progress(
        &app,
        &task,
        100,
        if result.is_ok() {
            "操作完成"
        } else {
            "操作失败"
        },
    );
    result
}

fn run_toolchain_action_blocking(
    action: &str,
    value: Option<String>,
    secondary: Option<String>,
) -> Result<OperationResult, String> {
    let paths = load_paths()?;
    let required = |name: &str| {
        resolve_tool(&paths, name)
            .ok_or_else(|| format!("没有找到 {name}，请先安装对应工具并刷新诊断"))
    };
    let message = match action {
        "git_identity" => {
            let name = validate_setting(value.as_deref(), "Git 用户名")?;
            let email = validate_setting(secondary.as_deref(), "Git 邮箱")?;
            let git = required("git")?;
            run_action_command(
                &paths,
                git.clone(),
                &["config", "--global", "user.name", &name],
            )?;
            run_action_command(&paths, git, &["config", "--global", "user.email", &email])?;
            "已更新当前用户的 Git 用户名和邮箱".to_string()
        }
        "git_generate_ssh" => {
            let email = validate_setting(value.as_deref(), "SSH Key 注释邮箱")?;
            let target = dirs::home_dir()
                .ok_or_else(|| "无法定位用户目录".to_string())?
                .join(".ssh/id_ed25519");
            if target.exists() || target.with_extension("pub").exists() {
                return Err("已存在 id_ed25519 密钥，为避免覆盖已取消生成".to_string());
            }
            let parent = target
                .parent()
                .ok_or_else(|| "SSH Key 路径无效".to_string())?;
            fs::create_dir_all(parent).map_err(|err| format!("创建 .ssh 目录失败：{err}"))?;
            let ssh_keygen = required("ssh-keygen")?;
            run_action_command(
                &paths,
                ssh_keygen,
                &[
                    "-t",
                    "ed25519",
                    "-C",
                    &email,
                    "-f",
                    &display_path(&target),
                    "-N",
                    "",
                ],
            )?;
            format!(
                "已生成 SSH Key，公钥位于 {}",
                display_path(target.with_extension("pub"))
            )
        }
        "git_test_ssh" => github_ssh_status(Some(required("ssh")?)),
        "corepack_enable" => {
            run_action_command(&paths, required("corepack")?, &["enable"])?;
            "Corepack 已启用".to_string()
        }
        "npm_install_pnpm" => {
            run_action_command(&paths, required("npm")?, &["install", "--global", "pnpm"])?;
            "pnpm 已安装，请刷新工具链状态".to_string()
        }
        "npm_install_yarn" => {
            run_action_command(&paths, required("npm")?, &["install", "--global", "yarn"])?;
            "Yarn 已安装，请刷新工具链状态".to_string()
        }
        "npm_registry" => {
            let registry = match value.as_deref() {
                Some("official") => "https://registry.npmjs.org/",
                Some("npmmirror") => "https://registry.npmmirror.com/",
                _ => return Err("不支持的 npm 镜像源".to_string()),
            };
            run_action_command(
                &paths,
                required("npm")?,
                &["config", "set", "registry", registry],
            )?;
            format!("npm registry 已切换为 {registry}")
        }
        "npm_managed_prefix" => {
            fs::create_dir_all(paths.npm_global())
                .map_err(|err| format!("创建 npm 全局目录失败：{err}"))?;
            run_action_command(
                &paths,
                required("npm")?,
                &["config", "set", "prefix", &display_path(paths.npm_global())],
            )?;
            "npm 全局目录已切换到 DevEnv Manager 受管目录".to_string()
        }
        "python_install_tool" => {
            let package = match value.as_deref() {
                Some("uv") => "uv",
                Some("poetry") => "poetry",
                Some("virtualenv") => "virtualenv",
                _ => return Err("不支持的 Python 工具".to_string()),
            };
            run_action_command(
                &paths,
                required("python")?,
                &["-m", "pip", "install", "--upgrade", package],
            )?;
            format!("{package} 已安装到当前 Python")
        }
        "pip_index" => {
            let index = match value.as_deref() {
                Some("official") => "https://pypi.org/simple",
                Some("tsinghua") => "https://pypi.tuna.tsinghua.edu.cn/simple",
                Some("aliyun") => "https://mirrors.aliyun.com/pypi/simple/",
                Some("ustc") => "https://pypi.mirrors.ustc.edu.cn/simple/",
                _ => return Err("不支持的 PyPI 镜像源".to_string()),
            };
            run_action_command(
                &paths,
                required("python")?,
                &["-m", "pip", "config", "set", "global.index-url", index],
            )?;
            format!("pip 镜像源已切换为 {index}")
        }
        _ => return Err("不支持的工具链操作".to_string()),
    };
    Ok(OperationResult {
        success: true,
        message,
    })
}

fn toolchain_action_title(action: &str) -> &'static str {
    match action {
        "git_identity" => "配置 Git 身份",
        "git_generate_ssh" => "生成 SSH Key",
        "git_test_ssh" => "测试 GitHub SSH",
        "corepack_enable" => "启用 Corepack",
        "npm_install_pnpm" => "安装 pnpm",
        "npm_install_yarn" => "安装 Yarn",
        "npm_registry" => "切换 npm 镜像",
        "npm_managed_prefix" => "配置 npm 全局目录",
        "python_install_tool" => "安装 Python 工具",
        "pip_index" => "切换 pip 镜像",
        _ => "工具链操作",
    }
}

#[tauri::command]
async fn inspect_platform_toolchains() -> Result<PlatformReport, String> {
    run_blocking(inspect_platform_toolchains_blocking).await?
}

fn inspect_platform_toolchains_blocking() -> Result<PlatformReport, String> {
    let paths = load_paths()?;
    let go_executable = resolve_tool(&paths, "go");
    let go = probe_tool("Go", go_executable.clone(), &["version"]);
    let go_value = |key: &str| command_value(go_executable.clone(), &["env", key]);
    let user_env = user_environment().unwrap_or_default();

    let rustup = resolve_tool(&paths, "rustup");
    let rustc = resolve_tool(&paths, "rustc");
    let cargo = resolve_tool(&paths, "cargo");
    let rust_tools = vec![
        probe_tool("rustup", rustup.clone(), &["--version"]),
        probe_tool("rustc", rustc, &["--version"]),
        probe_tool("Cargo", cargo, &["--version"]),
    ];
    let default_toolchain = command_value(rustup.clone(), &["show", "active-toolchain"]);
    let installed_toolchains = command_value(rustup, &["toolchain", "list"])
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    let cargo_config_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".cargo/config.toml");

    let dotnet_executable = resolve_tool(&paths, "dotnet");
    let dotnet = probe_tool(".NET SDK", dotnet_executable.clone(), &["--version"]);
    let sdks = command_value(dotnet_executable.clone(), &["--list-sdks"])
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    let runtimes = command_value(dotnet_executable, &["--list-runtimes"])
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();

    let npm_registry = command_value(resolve_tool(&paths, "npm"), &["config", "get", "registry"]);
    let python = resolve_tool(&paths, "python");
    let pip_config = command_value(python, &["-m", "pip", "config", "list"]);
    let home = dirs::home_dir().unwrap_or_default();
    let maven_settings_path = home.join(".m2/settings.xml");
    let gradle_init_path = home.join(".gradle/init.gradle");

    Ok(PlatformReport {
        go: GoEnvironment {
            go,
            goroot: go_value("GOROOT"),
            gopath: go_value("GOPATH"),
            goproxy: user_env
                .get("GOPROXY")
                .cloned()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| go_value("GOPROXY")),
            gomodcache: go_value("GOMODCACHE"),
        },
        rust: RustEnvironment {
            tools: rust_tools,
            default_toolchain,
            installed_toolchains,
            msvc_build_tools: detect_msvc_build_tools(),
            cargo_config_path: display_path(&cargo_config_path),
        },
        dotnet: DotnetEnvironment {
            dotnet,
            sdks,
            runtimes,
        },
        mirrors: MirrorCenter {
            npm_registry,
            pip_index_url: pip_config_value(&pip_config, "global.index-url"),
            go_proxy: user_env
                .get("GOPROXY")
                .cloned()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| go_value("GOPROXY")),
            maven_settings_path: display_path(&maven_settings_path),
            maven_settings_exists: maven_settings_path.is_file(),
            gradle_init_path: display_path(&gradle_init_path),
            gradle_init_exists: gradle_init_path.is_file(),
            cargo_config_path: display_path(&cargo_config_path),
            cargo_config_exists: cargo_config_path.is_file(),
        },
        generated_at: current_timestamp(),
    })
}

#[tauri::command]
async fn run_platform_action(
    app: tauri::AppHandle,
    action: String,
    value: Option<String>,
) -> Result<OperationResult, String> {
    let task = platform_action_title(&action).to_string();
    emit_task_progress(&app, &task, 5, "正在准备操作");
    let worker_action = action.clone();
    let result = run_blocking(move || run_platform_action_blocking(&worker_action, value)).await?;
    emit_task_progress(
        &app,
        &task,
        100,
        if result.is_ok() {
            "操作完成"
        } else {
            "操作失败"
        },
    );
    result
}

fn run_platform_action_blocking(
    action: &str,
    value: Option<String>,
) -> Result<OperationResult, String> {
    let paths = load_paths()?;
    let required = |name: &str| {
        resolve_tool(&paths, name).ok_or_else(|| format!("没有找到 {name}，请先安装并刷新平台诊断"))
    };
    let message = match action {
        "go_proxy" => {
            let proxy = match value.as_deref() {
                Some("official") => "https://proxy.golang.org,direct",
                Some("goproxy_cn") => "https://goproxy.cn,direct",
                Some("direct") => "direct",
                _ => return Err("不支持的 Go 代理".to_string()),
            };
            set_user_environment_variable("GOPROXY", Some(proxy))?;
            broadcast_environment_change();
            format!("当前用户 GOPROXY 已设置为 {proxy}")
        }
        "rust_default_stable" => {
            run_action_command(&paths, required("rustup")?, &["default", "stable"])?;
            "Rust 默认工具链已切换为 stable".to_string()
        }
        "rust_update" => {
            run_action_command(&paths, required("rustup")?, &["update"])?;
            "rustup 工具链更新完成".to_string()
        }
        "maven_mirror" => {
            let mirror = match value.as_deref() {
                Some("official") => None,
                Some("aliyun") => Some(("aliyun", "https://maven.aliyun.com/repository/public")),
                _ => return Err("不支持的 Maven 镜像".to_string()),
            };
            let target = dirs::home_dir()
                .ok_or_else(|| "无法定位用户目录".to_string())?
                .join(".m2/settings.xml");
            let backup = backup_before_write(&target)?;
            write_maven_settings(&target, mirror)?;
            config_write_message("Maven settings.xml", &target, backup.as_deref())
        }
        "gradle_mirror" => {
            let mirror = match value.as_deref() {
                Some("official") => None,
                Some("aliyun") => Some("https://maven.aliyun.com/repository/public"),
                _ => return Err("不支持的 Gradle 镜像".to_string()),
            };
            let target = dirs::home_dir()
                .ok_or_else(|| "无法定位用户目录".to_string())?
                .join(".gradle/init.gradle");
            let backup = backup_before_write(&target)?;
            write_gradle_init(&target, mirror)?;
            config_write_message("Gradle init.gradle", &target, backup.as_deref())
        }
        "restore_maven_config" => {
            let target = dirs::home_dir()
                .ok_or_else(|| "无法定位用户目录".to_string())?
                .join(".m2/settings.xml");
            restore_latest_backup(&target)?
        }
        "restore_gradle_config" => {
            let target = dirs::home_dir()
                .ok_or_else(|| "无法定位用户目录".to_string())?
                .join(".gradle/init.gradle");
            restore_latest_backup(&target)?
        }
        _ => return Err("不支持的平台工具链操作".to_string()),
    };
    Ok(OperationResult {
        success: true,
        message,
    })
}

fn platform_action_title(action: &str) -> &'static str {
    match action {
        "go_proxy" => "切换 Go 代理",
        "rust_default_stable" => "切换 Rust stable",
        "rust_update" => "更新 Rust 工具链",
        "maven_mirror" => "配置 Maven 镜像",
        "gradle_mirror" => "配置 Gradle 镜像",
        "restore_maven_config" => "恢复 Maven 配置",
        "restore_gradle_config" => "恢复 Gradle 配置",
        _ => "平台工具链操作",
    }
}

#[tauri::command]
async fn inspect_system_platforms() -> Result<SystemPlatformReport, String> {
    run_blocking(inspect_system_platforms_blocking).await?
}

fn inspect_system_platforms_blocking() -> Result<SystemPlatformReport, String> {
    let paths = load_paths()?;
    let docker_executable = resolve_tool(&paths, "docker");
    let docker = probe_tool("Docker", docker_executable.clone(), &["--version"]);
    let docker_info = command_value(
        docker_executable,
        &["info", "--format", "{{.ServerVersion}}"],
    );
    let docker_desktop_path = docker_desktop_path().map(display_path).unwrap_or_default();
    let wsl_executable = resolve_tool(&paths, "wsl");
    let wsl = probe_tool("WSL", wsl_executable.clone(), &["--version"]);
    let wsl_status = command_value(wsl_executable.clone(), &["--status"]);
    let wsl_list_output = command_value(wsl_executable, &["--list", "--verbose"]);
    let wsl_distributions = wsl_list_output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();
    let wsl_items = parse_wsl_distributions(&wsl_list_output);
    Ok(SystemPlatformReport {
        docker,
        docker_info: if docker_info.is_empty() {
            "Docker 服务未运行或无法连接".to_string()
        } else {
            format!("Docker Engine {docker_info}")
        },
        docker_desktop_path,
        wsl,
        wsl_status,
        wsl_distributions,
        wsl_items,
    })
}

fn parse_wsl_distributions(output: &str) -> Vec<WslDistribution> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.replace('\0', "");
            let trimmed = line.trim();
            if trimmed.is_empty()
                || (trimmed.to_ascii_uppercase().contains("NAME")
                    && trimmed.to_ascii_uppercase().contains("STATE"))
            {
                return None;
            }
            let is_default = trimmed.starts_with('*');
            let parts = trimmed
                .trim_start_matches('*')
                .split_whitespace()
                .collect::<Vec<_>>();
            if parts.len() < 3 {
                return None;
            }
            Some(WslDistribution {
                name: parts[0].to_string(),
                state: parts[1].to_string(),
                version: parts[2].to_string(),
                is_default,
            })
        })
        .collect()
}

#[tauri::command]
async fn manage_system_platform(
    app: tauri::AppHandle,
    action: String,
    value: Option<String>,
) -> Result<OperationResult, String> {
    run_blocking(move || manage_system_platform_blocking(app, action, value)).await?
}

fn manage_system_platform_blocking(
    app: tauri::AppHandle,
    action: String,
    value: Option<String>,
) -> Result<OperationResult, String> {
    emit_task_progress(&app, "平台管理", 5, "正在校验操作");
    let message = match action.as_str() {
        "docker_install" => {
            run_command_output(
                PathBuf::from("winget.exe"),
                &[
                    "install",
                    "--id",
                    "Docker.DockerDesktop",
                    "--exact",
                    "--accept-package-agreements",
                    "--accept-source-agreements",
                ],
                900,
            )?;
            "Docker Desktop 安装命令已完成".to_string()
        }
        "docker_update" => {
            run_command_output(
                PathBuf::from("winget.exe"),
                &[
                    "upgrade",
                    "--id",
                    "Docker.DockerDesktop",
                    "--exact",
                    "--accept-package-agreements",
                    "--accept-source-agreements",
                ],
                900,
            )?;
            "Docker Desktop 更新命令已完成".to_string()
        }
        "docker_shutdown" => {
            let desktop =
                docker_desktop_path().ok_or_else(|| "没有找到 Docker Desktop".to_string())?;
            let cli = desktop
                .parent()
                .map(|parent| parent.join("DockerCli.exe"))
                .filter(|path| path.is_file())
                .ok_or_else(|| "没有找到 DockerCli.exe".to_string())?;
            run_command_output(cli, &["-Shutdown"], 60)?;
            "已请求安全退出 Docker Desktop".to_string()
        }
        "wsl_install" => {
            launch_elevated_wsl(None, "install")?;
            "已打开 WSL 安装授权窗口；完成后可能需要重启 Windows".to_string()
        }
        "wsl_update" => {
            launch_elevated_wsl(None, "update")?;
            "已打开 WSL 更新授权窗口".to_string()
        }
        "wsl_install_distro" => {
            let distro = validate_online_wsl_distribution(value.as_deref().unwrap_or(""))?;
            launch_elevated_wsl(Some(&distro), "install-distro")?;
            format!("已打开 {distro} 安装授权窗口")
        }
        "wsl_start" | "wsl_terminate" | "wsl_set_default" => {
            let distro = validate_installed_wsl_distribution(value.as_deref().unwrap_or(""))?;
            let args = match action.as_str() {
                "wsl_start" => vec![
                    "--distribution",
                    distro.as_str(),
                    "--exec",
                    "echo",
                    "DevEnv Manager started WSL",
                ],
                "wsl_terminate" => vec!["--terminate", distro.as_str()],
                _ => vec!["--set-default", distro.as_str()],
            };
            run_command_output(PathBuf::from("wsl.exe"), &args, 120)?;
            match action.as_str() {
                "wsl_start" => format!("已启动 WSL 发行版 {distro}"),
                "wsl_terminate" => format!("已终止 WSL 发行版 {distro}"),
                _ => format!("已将 {distro} 设为默认 WSL 发行版"),
            }
        }
        _ => return Err("不支持的平台管理操作".to_string()),
    };
    emit_task_progress(&app, "平台管理", 100, "操作完成");
    Ok(OperationResult {
        success: true,
        message,
    })
}

fn validate_installed_wsl_distribution(value: &str) -> Result<String, String> {
    let value = value.trim();
    if !valid_wsl_distribution_name(value) {
        return Err("WSL 发行版名称无效".to_string());
    }
    let output = command_value(Some(PathBuf::from("wsl.exe")), &["--list", "--verbose"]);
    parse_wsl_distributions(&output)
        .into_iter()
        .find(|item| item.name.eq_ignore_ascii_case(value))
        .map(|item| item.name)
        .ok_or_else(|| "该 WSL 发行版不在已安装列表中".to_string())
}

fn validate_online_wsl_distribution(value: &str) -> Result<String, String> {
    let value = value.trim();
    if !valid_wsl_distribution_name(value) {
        return Err("WSL 发行版名称无效".to_string());
    }
    let output = command_value(Some(PathBuf::from("wsl.exe")), &["--list", "--online"]);
    output
        .replace('\0', "")
        .lines()
        .flat_map(str::split_whitespace)
        .find(|item| item.eq_ignore_ascii_case(value))
        .map(str::to_string)
        .ok_or_else(|| "该名称不在 WSL 官方在线发行版列表中".to_string())
}

fn valid_wsl_distribution_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}

fn launch_elevated_wsl(distro: Option<&str>, mode: &str) -> Result<(), String> {
    let script = match mode {
        "install" => "Start-Process -FilePath wsl.exe -ArgumentList @('--install','--no-distribution') -Verb RunAs",
        "update" => "Start-Process -FilePath wsl.exe -ArgumentList @('--update') -Verb RunAs",
        "install-distro" => {
            "Start-Process -FilePath wsl.exe -ArgumentList @('--install','--distribution',$env:DEVENV_WSL_DISTRO) -Verb RunAs"
        }
        _ => return Err("不支持的 WSL 授权操作".to_string()),
    };
    let mut command = hidden_command("powershell.exe");
    command.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    if let Some(distro) = distro {
        command.env("DEVENV_WSL_DISTRO", distro);
    }
    let output = command
        .output()
        .map_err(|err| format!("启动 WSL 授权操作失败：{err}"))?;
    if !output.status.success() {
        return Err(format!(
            "启动 WSL 授权操作失败：{}",
            command_text(&output.stdout, &output.stderr)
        ));
    }
    Ok(())
}

#[tauri::command]
async fn inspect_local_services() -> Result<Vec<LocalServiceStatus>, String> {
    run_blocking(inspect_local_services_blocking).await?
}

fn inspect_local_services_blocking() -> Result<Vec<LocalServiceStatus>, String> {
    let ports = scan_ports_blocking()?;
    let services = windows_service_inventory();
    Ok(database_service_definitions()
        .into_iter()
        .map(|(id, name, port, connection_command)| {
            let record = ports.iter().find(|record| {
                record.local_port == port && record.state.eq_ignore_ascii_case("LISTENING")
            });
            let service = record
                .and_then(|item| {
                    item.service_names.iter().find_map(|service_name| {
                        services
                            .iter()
                            .find(|service| service.name.eq_ignore_ascii_case(service_name))
                    })
                })
                .or_else(|| {
                    services
                        .iter()
                        .find(|service| service_matches_database(port, &service.name))
                });
            let service_names = record
                .map(|item| item.service_names.clone())
                .filter(|items| !items.is_empty())
                .or_else(|| service.map(|item| vec![item.name.clone()]))
                .unwrap_or_default();
            LocalServiceStatus {
                id: id.to_string(),
                name: name.to_string(),
                port,
                occupied: record.is_some(),
                pid: record.map(|item| item.pid).unwrap_or(0),
                process_name: record
                    .map(|item| item.process_name.clone())
                    .unwrap_or_default(),
                process_path: record
                    .map(|item| item.process_path.clone())
                    .unwrap_or_default(),
                service_names,
                safe_to_stop: record
                    .map(|item| {
                        !BLOCKED_PIDS.contains(&item.pid)
                            && !BLOCKED_NAMES
                                .contains(&item.process_name.to_ascii_lowercase().as_str())
                            && service.is_some()
                    })
                    .unwrap_or(false),
                connection_command: connection_command.to_string(),
                installed: service.is_some(),
                service_name: service.map(|item| item.name.clone()).unwrap_or_default(),
                service_state: service
                    .map(|item| item.state.clone())
                    .unwrap_or_else(|| "NotInstalled".to_string()),
                binary_path: service
                    .map(|item| item.path_name.clone())
                    .unwrap_or_default(),
            }
        })
        .collect())
}

#[tauri::command]
async fn stop_local_service(port: u16, service_name: String) -> Result<OperationResult, String> {
    run_blocking(move || stop_local_service_blocking(port, service_name)).await?
}

fn stop_local_service_blocking(port: u16, service_name: String) -> Result<OperationResult, String> {
    if !database_service_definitions()
        .iter()
        .any(|item| item.2 == port)
    {
        return Err("只允许停止已定义的数据库开发服务".to_string());
    }
    if !service_matches_database(port, &service_name) {
        return Err("服务名与数据库端口不匹配，已拒绝停止".to_string());
    }
    let records = scan_ports_blocking()?;
    let verified = records.iter().any(|record| {
        record.local_port == port
            && record.state.eq_ignore_ascii_case("LISTENING")
            && record
                .service_names
                .iter()
                .any(|service| service.eq_ignore_ascii_case(&service_name))
    });
    if !verified {
        return Err("重新扫描后没有确认该服务仍占用目标端口".to_string());
    }
    let output = hidden_command("sc.exe")
        .args(["stop", service_name.as_str()])
        .output()
        .map_err(|err| format!("停止 Windows 服务失败：{err}"))?;
    if !output.status.success() {
        return Err(format!(
            "停止 Windows 服务失败：{}",
            command_text(&output.stdout, &output.stderr)
        ));
    }
    Ok(OperationResult {
        success: true,
        message: format!("已请求停止 Windows 服务 {service_name}"),
    })
}
fn windows_service_inventory() -> Vec<WindowsServiceInfo> {
    #[cfg(windows)]
    {
        let script = "$ErrorActionPreference='Stop'; @(Get-CimInstance Win32_Service | Select-Object Name,State,PathName) | ConvertTo-Json -Compress";
        let Ok(output) = hidden_command("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .output()
        else {
            return Vec::new();
        };
        if !output.status.success() {
            return Vec::new();
        }
        let text = command_text(&output.stdout, &output.stderr);
        let Ok(value) = serde_json::from_str::<Value>(&text) else {
            return Vec::new();
        };
        match value {
            Value::Array(items) => items
                .into_iter()
                .filter_map(|item| serde_json::from_value(item).ok())
                .collect(),
            Value::Object(_) => serde_json::from_value(value).into_iter().collect(),
            _ => Vec::new(),
        }
    }
    #[cfg(not(windows))]
    Vec::new()
}

fn validated_database_service(name: &str) -> Result<(WindowsServiceInfo, u16), String> {
    let name = name.trim();
    if name.is_empty() || name.len() > 128 || name.chars().any(char::is_control) {
        return Err("Windows 服务名无效".to_string());
    }
    let service = windows_service_inventory()
        .into_iter()
        .find(|item| item.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| "重新检查后没有找到该 Windows 服务".to_string())?;
    let port = database_service_definitions()
        .iter()
        .find(|item| service_matches_database(item.2, &service.name))
        .map(|item| item.2)
        .ok_or_else(|| "该服务不属于允许管理的开发数据库".to_string())?;
    Ok((service, port))
}

#[tauri::command]
async fn manage_local_service(
    service_name: String,
    action: String,
) -> Result<OperationResult, String> {
    run_blocking(move || manage_local_service_blocking(service_name, action)).await?
}

fn manage_local_service_blocking(
    service_name: String,
    action: String,
) -> Result<OperationResult, String> {
    let (service, _) = validated_database_service(&service_name)?;
    if !matches!(action.as_str(), "start" | "stop" | "restart") {
        return Err("只允许启动、停止或重启数据库服务".to_string());
    }
    if action == "stop" || action == "restart" {
        let output = hidden_command("sc.exe")
            .args(["stop", service.name.as_str()])
            .output()
            .map_err(|err| format!("停止 Windows 服务失败：{err}"))?;
        let text = command_text(&output.stdout, &output.stderr);
        if !output.status.success() && !text.contains("1062") {
            return Err(format!("停止 Windows 服务失败：{text}"));
        }
        if action == "restart" {
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    }
    if action == "start" || action == "restart" {
        let output = hidden_command("sc.exe")
            .args(["start", service.name.as_str()])
            .output()
            .map_err(|err| format!("启动 Windows 服务失败：{err}"))?;
        let text = command_text(&output.stdout, &output.stderr);
        if !output.status.success() && !text.contains("1056") {
            return Err(format!("启动 Windows 服务失败：{text}"));
        }
    }
    Ok(OperationResult {
        success: true,
        message: format!(
            "已{} Windows 服务 {}",
            match action.as_str() {
                "start" => "请求启动",
                "stop" => "请求停止",
                _ => "请求重启",
            },
            service.name
        ),
    })
}

#[tauri::command]
async fn local_service_logs(service_name: String) -> Result<String, String> {
    run_blocking(move || {
        let (service, _) = validated_database_service(&service_name)?;
        let script = "$needle=$env:DEVENV_SERVICE_NAME; Get-WinEvent -FilterHashtable @{LogName='Application'; StartTime=(Get-Date).AddDays(-7)} -MaxEvents 500 -ErrorAction SilentlyContinue | Where-Object { $_.ProviderName -like ('*'+$needle+'*') -or $_.Message -like ('*'+$needle+'*') } | Select-Object -First 50 TimeCreated,LevelDisplayName,ProviderName,Message | Format-List | Out-String -Width 240";
        let output = hidden_command("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .env("DEVENV_SERVICE_NAME", &service.name)
            .output()
            .map_err(|err| format!("读取 Windows 事件日志失败：{err}"))?;
        if !output.status.success() {
            return Err(format!(
                "读取 Windows 事件日志失败：{}",
                command_text(&output.stdout, &output.stderr)
            ));
        }
        let text = command_text(&output.stdout, &output.stderr);
        Ok(if text.trim().is_empty() {
            format!("最近 7 天没有找到与 {} 匹配的应用程序事件", service.name)
        } else {
            text
        })
    })
    .await?
}

#[tauri::command]
fn open_local_service_directory(service_name: String) -> Result<OperationResult, String> {
    let (service, _) = validated_database_service(&service_name)?;
    let executable = service_executable_path(&service.path_name)
        .ok_or_else(|| "无法从服务配置识别程序路径".to_string())?;
    let directory = executable
        .parent()
        .filter(|path| path.is_dir())
        .ok_or_else(|| "数据库服务程序目录不存在".to_string())?;
    hidden_command("explorer.exe")
        .arg(directory)
        .spawn()
        .map_err(|err| format!("打开数据库服务目录失败：{err}"))?;
    Ok(OperationResult {
        success: true,
        message: format!("已打开 {}", display_path(directory)),
    })
}

fn service_executable_path(value: &str) -> Option<PathBuf> {
    let value = value.trim();
    let path = if let Some(rest) = value.strip_prefix('"') {
        rest.split('"').next()?
    } else {
        value.split_whitespace().next()?
    };
    let path = PathBuf::from(path);
    path.is_file().then_some(path)
}

#[tauri::command]
fn open_docker_desktop() -> Result<OperationResult, String> {
    let executable =
        docker_desktop_path().ok_or_else(|| "没有找到 Docker Desktop.exe".to_string())?;
    hidden_command(&executable)
        .spawn()
        .map_err(|err| format!("启动 Docker Desktop 失败：{err}"))?;
    Ok(OperationResult {
        success: true,
        message: format!("已启动 {}", display_path(executable)),
    })
}

fn database_service_definitions() -> Vec<(&'static str, &'static str, u16, &'static str)> {
    vec![
        (
            "mysql",
            "MySQL",
            3306,
            "mysql -h 127.0.0.1 -P 3306 -u root -p",
        ),
        (
            "postgres",
            "PostgreSQL",
            5432,
            "psql -h 127.0.0.1 -p 5432 -U postgres",
        ),
        ("redis", "Redis", 6379, "redis-cli -h 127.0.0.1 -p 6379"),
        (
            "mongo",
            "MongoDB",
            27017,
            "mongosh mongodb://127.0.0.1:27017",
        ),
        (
            "elasticsearch",
            "Elasticsearch",
            9200,
            "curl http://127.0.0.1:9200",
        ),
        (
            "sqlserver",
            "SQL Server",
            1433,
            "sqlcmd -S 127.0.0.1,1433 -E",
        ),
    ]
}

fn service_matches_database(port: u16, service: &str) -> bool {
    let service = service.to_ascii_lowercase();
    match port {
        3306 => service.contains("mysql") || service.contains("maria"),
        5432 => service.contains("postgres"),
        6379 => service.contains("redis"),
        27017 => service.contains("mongo"),
        9200 => service.contains("elastic"),
        1433 => service.contains("mssql") || service.contains("sqlserver"),
        _ => false,
    }
}

fn docker_desktop_path() -> Option<PathBuf> {
    let program_files =
        env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string());
    let local_app_data = env::var("LOCALAPPDATA").unwrap_or_default();
    [
        PathBuf::from(program_files).join("Docker/Docker/Docker Desktop.exe"),
        PathBuf::from(local_app_data).join("Docker/Docker Desktop.exe"),
    ]
    .into_iter()
    .find(|path| path.is_file())
}

#[tauri::command]
fn analyze_project(path: String) -> Result<ProjectAnalysis, String> {
    analyze_project_blocking(&PathBuf::from(path.trim()))
}

fn analyze_project_blocking(root: &Path) -> Result<ProjectAnalysis, String> {
    if !root.exists() {
        return Err("项目目录不存在".to_string());
    }
    if !root.is_dir() {
        return Err("请选择目录而不是文件".to_string());
    }
    let signals = project_signals(root);
    let mut project_types = Vec::new();
    let mut recommendations = Vec::new();
    let mut actions = Vec::new();
    let mut warnings = Vec::new();
    let has = |name: &str| signals.iter().any(|item| item == name);

    if has("package.json") {
        push_unique(&mut project_types, "Node.js");
        recommendations.push(runtime_recommendation(
            "Node.js",
            "建议 Node.js 20/22 LTS",
            "node",
        ));
        let manager = detect_package_manager(&signals);
        actions.push(project_action(
            "npm_install",
            "安装依赖",
            &format!("{manager} install"),
            "安装前端或 Node 项目依赖",
            true,
        ));
        actions.push(project_action(
            "npm_dev",
            "启动开发服务",
            &format!("{manager} run dev"),
            "启动 Vite/Next/Node 开发服务，后台运行",
            true,
        ));
        actions.push(project_action(
            "npm_test",
            "运行测试",
            &format!("{manager} test"),
            "运行 package.json 中的测试脚本",
            true,
        ));
    }
    if has("pyproject.toml")
        || has("requirements.txt")
        || has("poetry.lock")
        || has("uv.lock")
        || has(".venv")
    {
        push_unique(&mut project_types, "Python");
        recommendations.push(runtime_recommendation(
            "Python",
            "建议 Python 3.12/3.14，并使用 .venv",
            "python",
        ));
        actions.push(project_action(
            "python_pytest",
            "运行 pytest",
            "python -m pytest -q",
            "使用当前 Python 运行测试",
            true,
        ));
        if !has(".venv") {
            warnings.push("未发现 .venv，建议用当前 Python 创建项目虚拟环境".to_string());
        }
    }
    if has("pom.xml") {
        push_unique(&mut project_types, "Maven");
        recommendations.push(project_jdk_recommendation(
            root,
            "Maven 项目通常需要 JDK 8/11/17/21",
        ));
        recommendations.push(runtime_recommendation("Maven", "需要 mvn 可用", "mvn"));
        actions.push(project_action(
            "mvn_test",
            "Maven 测试",
            "mvn test",
            "运行 Maven 测试",
            true,
        ));
    }
    if has("build.gradle") || has("build.gradle.kts") || has("gradlew") {
        push_unique(&mut project_types, "Gradle");
        if !has("pom.xml") {
            recommendations.push(project_jdk_recommendation(
                root,
                "Gradle 项目通常需要 JDK 17/21",
            ));
        }
        recommendations.push(runtime_recommendation(
            "Gradle",
            "优先使用项目 gradlew；否则使用受管 Gradle",
            "gradle",
        ));
        actions.push(project_action(
            "gradle_test",
            "Gradle 测试",
            gradle_command(root, "test").as_str(),
            "运行 Gradle 测试",
            true,
        ));
    }
    if has("Cargo.toml") {
        push_unique(&mut project_types, "Rust");
        recommendations.push(runtime_recommendation(
            "Rust",
            "建议 rustup stable + MSVC Build Tools",
            "rustc",
        ));
        actions.push(project_action(
            "cargo_test",
            "Cargo 测试",
            "cargo test",
            "运行 Rust 测试",
            true,
        ));
        actions.push(project_action(
            "cargo_check",
            "Cargo 检查",
            "cargo check",
            "检查 Rust 项目但不生成最终产物",
            true,
        ));
    }
    if has("src-tauri/tauri.conf.json") {
        push_unique(&mut project_types, "Tauri");
        recommendations.push(runtime_recommendation(
            "Tauri",
            "需要 Node.js、Rust、MSVC Build Tools",
            "cargo",
        ));
        if has("package.json") {
            actions.push(project_action(
                "npm_tauri_dev",
                "启动 Tauri 开发",
                "npm run tauri:dev",
                "启动 Tauri 桌面开发服务，后台运行",
                true,
            ));
        }
    }
    if signals
        .iter()
        .any(|item| item.ends_with(".csproj") || item.ends_with(".sln"))
    {
        push_unique(&mut project_types, ".NET");
        if let Some(required) = dotnet_required_sdk(root) {
            let installed =
                command_value(find_on_path("dotnet").map(PathBuf::from), &["--list-sdks"]);
            recommendations.push(ProjectRuntimeRecommendation {
                name: ".NET SDK".to_string(),
                requirement: format!("global.json 要求 SDK {required}"),
                status: if installed.lines().any(|line| line.starts_with(&required)) {
                    "版本匹配".to_string()
                } else {
                    "缺少指定版本".to_string()
                },
            });
        } else {
            recommendations.push(runtime_recommendation(
                ".NET SDK",
                "需要 dotnet SDK",
                "dotnet",
            ));
        }
        actions.push(project_action(
            "dotnet_restore",
            ".NET 还原",
            "dotnet restore",
            "还原 NuGet 依赖",
            true,
        ));
        actions.push(project_action(
            "dotnet_build",
            ".NET 构建",
            "dotnet build",
            "构建 .NET 项目",
            true,
        ));
        actions.push(project_action(
            "dotnet_test",
            ".NET 测试",
            "dotnet test",
            "运行 .NET 测试",
            true,
        ));
    }
    if has("go.mod") {
        push_unique(&mut project_types, "Go");
        recommendations.push(runtime_recommendation("Go", "需要 go 命令可用", "go"));
        actions.push(project_action(
            "go_test",
            "Go 测试",
            "go test ./...",
            "运行 Go 测试",
            true,
        ));
    }
    if has("bin/startup.cmd") && has("conf/application.properties") {
        push_unique(&mut project_types, "Nacos");
        let java = inspect_java_environment_blocking().ok();
        recommendations.push(ProjectRuntimeRecommendation {
            name: "Nacos Java".to_string(),
            requirement: "需要完整 JDK 8 或更高版本，并保证 JAVA_HOME 与 PATH 一致".to_string(),
            status: java
                .as_ref()
                .map(|report| {
                    if report.consistent && !report.path_javac.is_empty() {
                        format!("已验证：{}", report.java_version)
                    } else {
                        "JAVA_HOME/PATH 需要修复".to_string()
                    }
                })
                .unwrap_or_else(|| "未能读取 Java 环境".to_string()),
        });
        actions.push(project_action(
            "nacos_start",
            "启动 Nacos 单机模式",
            "bin\\startup.cmd -m standalone",
            "使用 DevEnv Manager 已验证的 JAVA_HOME 启动 Nacos",
            true,
        ));
        if java.as_ref().is_some_and(|report| !report.consistent) {
            warnings.push("Nacos 启动前请先修复 JAVA_HOME 与 PATH 的 JDK 不一致问题".to_string());
        }
    }
    actions.push(project_action(
        "vscode",
        "生成 VS Code 配置",
        "generate-vscode-config",
        "写入 .vscode/settings.json 和 tasks.json",
        true,
    ));
    actions.push(project_action(
        "copy_commands",
        "复制推荐命令",
        "copy",
        "复制该项目的推荐命令清单",
        true,
    ));
    if project_types.is_empty() {
        warnings.push("还没有识别到常见项目文件，可检查是否选中了项目根目录。".to_string());
    }
    project_types.sort();
    project_types.dedup();
    Ok(ProjectAnalysis {
        root: display_path(root),
        project_types,
        detected_files: signals.clone(),
        package_manager: if has("package.json") {
            Some(detect_package_manager(&signals))
        } else {
            None
        },
        recommended_runtime: recommendations,
        actions,
        warnings,
    })
}

#[tauri::command]
async fn run_project_action(path: String, action: String) -> Result<CommandRunResult, String> {
    run_blocking(move || run_project_action_blocking(path, action)).await?
}

fn run_project_action_blocking(path: String, action: String) -> Result<CommandRunResult, String> {
    let root = PathBuf::from(path.trim());
    let analysis = analyze_project_blocking(&root)?;
    let selected = analysis
        .actions
        .iter()
        .find(|item| item.id == action)
        .cloned()
        .ok_or_else(|| "这个项目不支持所选操作".to_string())?;
    if action == "vscode" {
        let result = generate_vscode_config(display_path(&root))?;
        return Ok(CommandRunResult {
            success: result.success,
            return_code: 0,
            output: result.message,
            elapsed_ms: 0,
        });
    }
    if action == "copy_commands" {
        return Ok(CommandRunResult {
            success: true,
            return_code: 0,
            output: analysis
                .actions
                .iter()
                .filter(|item| item.command != "copy" && item.command != "generate-vscode-config")
                .map(|item| format!("{}: {}", item.title, item.command))
                .collect::<Vec<_>>()
                .join("\n"),
            elapsed_ms: 0,
        });
    }
    if action == "nacos_start" {
        let paths = load_paths()?;
        let started = Instant::now();
        let mut command = hidden_command("cmd");
        command
            .args(["/d", "/c", "bin\\startup.cmd", "-m", "standalone"])
            .current_dir(&root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        apply_managed_environment(&paths, &mut command);
        command
            .spawn()
            .map_err(|err| format!("启动 Nacos 失败：{err}"))?;
        return Ok(CommandRunResult {
            success: true,
            return_code: 0,
            output: "已使用经过校验的 JAVA_HOME 后台启动 Nacos 单机模式".to_string(),
            elapsed_ms: started.elapsed().as_millis(),
        });
    }
    if matches!(action.as_str(), "npm_dev" | "npm_tauri_dev") {
        let parts = parse_command_line(&selected.command)?;
        let executable = parts.first().ok_or_else(|| "命令为空".to_string())?;
        let started = Instant::now();
        let mut command = hidden_command(executable);
        command
            .args(parts.iter().skip(1))
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Ok(paths) = load_paths() {
            apply_managed_environment(&paths, &mut command);
        }
        command
            .spawn()
            .map_err(|err| format!("启动开发服务失败：{err}"))?;
        return Ok(CommandRunResult {
            success: true,
            return_code: 0,
            output: format!("已后台启动：{}", selected.command),
            elapsed_ms: started.elapsed().as_millis(),
        });
    }
    run_tool_command_blocking(selected.command, Some(display_path(root)))
}

#[tauri::command]
async fn list_config_profiles() -> Result<Vec<ConfigProfile>, String> {
    run_blocking(list_config_profiles_blocking).await?
}

fn list_config_profiles_blocking() -> Result<Vec<ConfigProfile>, String> {
    let paths = load_paths()?;
    let mut profiles = load_profiles(&paths)?;
    profiles.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(profiles)
}

#[tauri::command]
async fn repair_doctor_safe() -> Result<DoctorRepairResult, String> {
    run_blocking(repair_doctor_safe_blocking).await?
}

fn repair_doctor_safe_blocking() -> Result<DoctorRepairResult, String> {
    let before = run_doctor_blocking()?;
    let actions = before
        .checks
        .iter()
        .filter(|item| item.status != "正常")
        .filter_map(|item| item.fix_action.as_deref())
        .collect::<BTreeSet<_>>();
    let mut applied = Vec::new();
    if actions.contains("cleanup_path") {
        applied.push(cleanup_path_entries_blocking()?.message);
    }
    if actions.contains("configure_env") {
        applied.push(configure_user_environment_blocking()?.message);
    }
    let report = run_doctor_blocking()?;
    let remaining = report
        .checks
        .iter()
        .filter(|item| doctor_check_needs_attention(item))
        .map(|item| format!("{}：{}", item.title, item.detail))
        .collect();
    Ok(DoctorRepairResult {
        before_score: before.score,
        after_score: report.score,
        applied,
        remaining,
        report,
    })
}

#[tauri::command]
fn config_profile_requirements(id: String) -> Result<Vec<ProfileRequirement>, String> {
    let paths = load_paths()?;
    let profile = load_profiles(&paths)?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| "没有找到配置模板".to_string())?;
    profile_requirements(&profile, &load_installed(&paths)?)
}

#[tauri::command]
async fn install_profile_missing(
    app: tauri::AppHandle,
    id: String,
) -> Result<OperationResult, String> {
    run_blocking(move || install_profile_missing_blocking(app, id)).await?
}

fn install_profile_missing_blocking(
    app: tauri::AppHandle,
    id: String,
) -> Result<OperationResult, String> {
    let paths = load_paths()?;
    let profile = load_profiles(&paths)?
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| "没有找到配置模板".to_string())?;
    let before = load_installed(&paths)?.current;
    let requirements = profile_requirements(&profile, &load_installed(&paths)?)?;
    let missing = requirements
        .into_iter()
        .filter(|item| !item.installed)
        .collect::<Vec<_>>();
    if missing.iter().any(|item| !item.auto_install_supported) {
        return Err(format!(
            "以下版本暂时无法自动补装：{}",
            missing
                .iter()
                .filter(|item| !item.auto_install_supported)
                .map(|item| format!("{} {}", item.kind, item.version))
                .collect::<Vec<_>>()
                .join("、")
        ));
    }
    for requirement in &missing {
        let result = match requirement.kind.as_str() {
            "jdk" => {
                let mut parts = requirement.version.splitn(2, '-');
                let major = parts.next().unwrap_or(&requirement.version).to_string();
                let distribution = parts
                    .next()
                    .filter(|value| ["temurin", "zulu", "liberica", "microsoft"].contains(value))
                    .unwrap_or("temurin")
                    .to_string();
                install_jdk_blocking(app.clone(), major, Some(distribution))
            }
            "python" => install_python_blocking(app.clone(), requirement.version.clone()),
            "node" => install_node_blocking(app.clone(), requirement.version.clone()),
            "go" => install_go_blocking(app.clone(), requirement.version.clone()),
            "maven" => install_maven_latest_blocking(app.clone()),
            "gradle" => install_gradle_latest_blocking(app.clone()),
            _ => Err(format!("不支持自动安装 {}", requirement.kind)),
        };
        if let Err(error) = result {
            restore_current_versions(&before);
            return Err(format!(
                "补装 {} {} 失败：{error}",
                requirement.kind, requirement.version
            ));
        }
    }
    match apply_config_profile_blocking(id) {
        Ok(result) => Ok(OperationResult {
            success: true,
            message: if missing.is_empty() {
                result.message
            } else {
                format!("已补装 {} 个缺失运行时并应用模板", missing.len())
            },
        }),
        Err(error) => {
            restore_current_versions(&before);
            Err(format!("运行时已下载，但应用模板失败：{error}"))
        }
    }
}

fn restore_current_versions(current: &CurrentVersions) {
    for (kind, version) in [
        ("jdk", current.jdk.as_ref()),
        ("python", current.python.as_ref()),
        ("node", current.node.as_ref()),
        ("maven", current.maven.as_ref()),
        ("gradle", current.gradle.as_ref()),
        ("go", current.go.as_ref()),
    ] {
        if let Some(version) = version {
            let _ = switch_runtime_blocking(kind.to_string(), version.clone(), None);
        }
    }
}

fn profile_requirements(
    profile: &ConfigProfile,
    installed: &InstalledData,
) -> Result<Vec<ProfileRequirement>, String> {
    [
        ("jdk", profile.current.jdk.as_ref()),
        ("python", profile.current.python.as_ref()),
        ("node", profile.current.node.as_ref()),
        ("maven", profile.current.maven.as_ref()),
        ("gradle", profile.current.gradle.as_ref()),
        ("go", profile.current.go.as_ref()),
    ]
    .into_iter()
    .filter_map(|(kind, version)| version.map(|value| (kind, value)))
    .map(|(kind, version)| {
        let meta = runtime_meta(kind)?;
        Ok(ProfileRequirement {
            kind: kind.to_string(),
            version: version.clone(),
            installed: collection(installed, meta.collection)
                .iter()
                .any(|item| item.get("version").and_then(Value::as_str) == Some(version.as_str())),
            auto_install_supported: matches!(
                kind,
                "jdk" | "python" | "node" | "maven" | "gradle" | "go"
            ),
        })
    })
    .collect()
}

#[tauri::command]
async fn save_config_profile(name: String) -> Result<OperationResult, String> {
    run_blocking(move || save_config_profile_blocking(name)).await?
}

fn save_config_profile_blocking(name: String) -> Result<OperationResult, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("模板名称不能为空".to_string());
    }
    let paths = load_paths()?;
    let installed = load_installed(&paths)?;
    let environment = user_environment()?;
    let path = environment
        .get("Path")
        .or_else(|| environment.get("PATH"))
        .cloned()
        .unwrap_or_default();
    let id = format!(
        "profile-{}",
        current_timestamp().replace([' ', ':', '.', '{', '}', ','], "-")
    );
    let profile = ConfigProfile {
        id: id.clone(),
        name: name.to_string(),
        created_at: current_timestamp(),
        current: installed.current,
        devenv_home: environment.get("DEVENV_HOME").cloned(),
        java_home: environment.get("JAVA_HOME").cloned(),
        path,
    };
    let mut profiles = load_profiles(&paths)?;
    profiles.retain(|item| item.name != name);
    profiles.push(profile);
    save_json(&paths.profiles_file(), &profiles)?;
    Ok(OperationResult {
        success: true,
        message: format!("已保存配置模板：{name}"),
    })
}

#[tauri::command]
async fn apply_config_profile(id: String) -> Result<OperationResult, String> {
    run_blocking(move || apply_config_profile_blocking(id)).await?
}

fn apply_config_profile_blocking(id: String) -> Result<OperationResult, String> {
    let paths = load_paths()?;
    let profiles = load_profiles(&paths)?;
    let profile = profiles
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| "没有找到配置模板".to_string())?;
    let switches = [
        ("jdk", profile.current.jdk.clone()),
        ("python", profile.current.python.clone()),
        ("node", profile.current.node.clone()),
        ("maven", profile.current.maven.clone()),
        ("gradle", profile.current.gradle.clone()),
        ("go", profile.current.go.clone()),
    ];
    let installed = load_installed(&paths)?;
    let missing = switches
        .iter()
        .filter_map(|(kind, version)| {
            let version = version.as_ref()?;
            let meta = runtime_meta(kind).ok()?;
            (!collection(&installed, meta.collection)
                .iter()
                .any(|item| item.get("version").and_then(Value::as_str) == Some(version.as_str())))
            .then(|| format!("{kind} {version}"))
        })
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "模板所需版本尚未安装：{}。为避免只切换一部分，当前环境没有发生变化。",
            missing.join("、")
        ));
    }
    let mut applied = Vec::new();
    for (kind, version) in switches {
        if let Some(version) = version {
            switch_runtime_blocking(kind.to_string(), version.clone(), None)?;
            applied.push(format!("{kind} {version}"));
        }
    }
    restore_environment_values(
        profile.devenv_home.as_deref(),
        profile.java_home.as_deref(),
        &profile.path,
    )?;
    broadcast_environment_change();
    Ok(OperationResult {
        success: true,
        message: if applied.is_empty() {
            format!("已恢复环境变量模板：{}", profile.name)
        } else {
            format!("已应用模板 {}：{}", profile.name, applied.join("，"))
        },
    })
}

#[tauri::command]
async fn delete_config_profile(id: String) -> Result<OperationResult, String> {
    run_blocking(move || delete_config_profile_blocking(id)).await?
}

fn delete_config_profile_blocking(id: String) -> Result<OperationResult, String> {
    let paths = load_paths()?;
    let mut profiles = load_profiles(&paths)?;
    let before = profiles.len();
    profiles.retain(|item| item.id != id);
    if profiles.len() == before {
        return Err("没有找到配置模板".to_string());
    }
    save_json(&paths.profiles_file(), &profiles)?;
    Ok(OperationResult {
        success: true,
        message: "已删除配置模板".to_string(),
    })
}

#[tauri::command]
fn export_config_profiles() -> Result<OperationResult, String> {
    let paths = load_paths()?;
    let profiles = load_profiles(&paths)?;
    let bundle = ConfigProfileBundle {
        schema_version: 1,
        exported_at: current_timestamp(),
        profiles,
    };
    fs::create_dir_all(paths.logs()).map_err(|err| format!("创建导出目录失败：{err}"))?;
    let target = paths
        .logs()
        .join(format!("config-profiles-{}.json", filename_timestamp()));
    save_json(&target, &bundle)?;
    Ok(OperationResult {
        success: true,
        message: format!("已导出配置模板：{}", display_path(target)),
    })
}

#[tauri::command]
fn preview_config_profiles(path: String) -> Result<ConfigProfileImportPreview, String> {
    let source = PathBuf::from(path.trim().trim_matches('"'));
    let bundle = read_profile_bundle(&source)?;
    let paths = load_paths()?;
    let existing = load_profiles(&paths)?;
    let installed = load_installed(&paths)?;
    let previews = bundle
        .profiles
        .into_iter()
        .map(|profile| {
            let missing = profile_requirements(&profile, &installed)?
                .into_iter()
                .filter(|item| !item.installed)
                .map(|item| format!("{} {}", item.kind, item.version))
                .collect();
            Ok(ConfigProfilePreviewItem {
                will_replace: existing.iter().any(|item| item.name == profile.name),
                name: profile.name,
                current: profile.current,
                missing,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(ConfigProfileImportPreview {
        source: display_path(source),
        exported_at: bundle.exported_at,
        profiles: previews,
    })
}

#[tauri::command]
fn import_config_profiles(path: String) -> Result<OperationResult, String> {
    let source = PathBuf::from(path.trim().trim_matches('"'));
    let bundle = read_profile_bundle(&source)?;
    let paths = load_paths()?;
    let mut profiles = load_profiles(&paths)?;
    let mut imported = 0_usize;
    for (index, mut profile) in bundle.profiles.into_iter().enumerate() {
        profile.name = profile.name.trim().to_string();
        if profile.name.is_empty()
            || profile.name.len() > 100
            || profile.name.chars().any(char::is_control)
        {
            return Err(format!("第 {} 个模板名称无效", index + 1));
        }
        profile.id = format!("imported-{}-{index}", filename_timestamp());
        profile.created_at = current_timestamp();
        profiles.retain(|item| item.name != profile.name);
        profiles.push(profile);
        imported += 1;
    }
    save_json(&paths.profiles_file(), &profiles)?;
    Ok(OperationResult {
        success: true,
        message: format!("已导入 {imported} 个配置模板；应用前会检查所需版本"),
    })
}

fn read_profile_bundle(source: &Path) -> Result<ConfigProfileBundle, String> {
    if !source.is_file() {
        return Err("模板文件不存在，请输入有效的 JSON 文件路径".to_string());
    }
    let metadata = source
        .metadata()
        .map_err(|err| format!("读取模板文件失败：{err}"))?;
    if metadata.len() > 1024 * 1024 {
        return Err("模板文件超过 1 MB，已拒绝导入".to_string());
    }
    let text = fs::read_to_string(source).map_err(|err| format!("读取模板文件失败：{err}"))?;
    let bundle: ConfigProfileBundle =
        serde_json::from_str(&text).map_err(|err| format!("模板 JSON 格式不正确：{err}"))?;
    if bundle.schema_version != 1 {
        return Err(format!("不支持的模板版本：{}", bundle.schema_version));
    }
    if bundle.profiles.is_empty() || bundle.profiles.len() > 100 {
        return Err("模板数量必须在 1 到 100 之间".to_string());
    }
    for (index, profile) in bundle.profiles.iter().enumerate() {
        let name = profile.name.trim();
        if name.is_empty() || name.len() > 100 || name.chars().any(char::is_control) {
            return Err(format!("第 {} 个模板名称无效", index + 1));
        }
    }
    Ok(bundle)
}

#[tauri::command]
async fn uninstall_external_runtime(
    executable: String,
    kind: String,
) -> Result<OperationResult, String> {
    run_blocking(move || uninstall_external_runtime_blocking(executable, kind)).await?
}

fn uninstall_external_runtime_blocking(
    executable: String,
    kind: String,
) -> Result<OperationResult, String> {
    let executable_path = PathBuf::from(executable.trim());
    if !executable_path.exists() {
        return Err("运行时路径不存在，无法定位卸载器".to_string());
    }
    if find_uninstall_entry_for_path(&executable_path, &kind).is_none() {
        return uninstall_unregistered_runtime(&executable_path, &kind);
    }
    let entry = find_uninstall_entry_for_path(&executable_path, &kind)
        .ok_or_else(|| {
            format!(
                "没有在 Windows 卸载注册表中找到匹配的卸载入口。{} 可能是绿色版、IDE 内置运行时，或没有单独卸载器；可以先用“配置”切换到 DevEnv 管理的版本，再手动删除原软件目录。",
                display_path(&executable_path)
            )
        })?;
    launch_uninstall_string(&entry.uninstall_string)?;
    Ok(OperationResult {
        success: true,
        message: format!("已启动 {} 的系统卸载程序", entry.display_name),
    })
}

fn uninstall_unregistered_runtime(
    executable: &Path,
    kind: &str,
) -> Result<OperationResult, String> {
    let normalized = executable.to_string_lossy().to_ascii_lowercase();
    if normalized.contains("\\jetbrains\\") || normalized.contains("\\jbr\\") {
        return Err("这是 IDE 内置运行时。直接删除会破坏 IDE；请先切换到 DevEnv 管理版本，并从 IDE 设置中取消使用该 JBR。".to_string());
    }
    if let Some(app) = package_name_from_path(executable, "scoop", "apps") {
        let scoop = find_on_path("scoop.cmd")
            .or_else(|| find_on_path("scoop"))
            .ok_or_else(|| "路径属于 Scoop，但当前找不到 scoop 命令".to_string())?;
        return run_package_uninstall(
            scoop,
            &["uninstall", app.as_str()],
            &format!("Scoop 卸载 {app}"),
        );
    }
    if let Some(package) = package_name_from_path(executable, "chocolatey", "lib") {
        let choco = find_on_path("choco.exe")
            .ok_or_else(|| "路径属于 Chocolatey，但当前找不到 choco.exe".to_string())?;
        return run_package_uninstall(
            choco,
            &["uninstall", package.as_str(), "-y"],
            &format!("Chocolatey 卸载 {package}"),
        );
    }
    let root = portable_runtime_root(executable, kind)?;
    trash::delete(&root).map_err(|err| format!("将便携运行时移入回收站失败：{err}"))?;
    Ok(OperationResult {
        success: true,
        message: format!("已将便携运行时移入 Windows 回收站：{}", display_path(root)),
    })
}

fn run_package_uninstall(
    executable: String,
    args: &[&str],
    title: &str,
) -> Result<OperationResult, String> {
    let output = hidden_command(executable)
        .args(args)
        .output()
        .map_err(|err| format!("{title}失败：{err}"))?;
    if !output.status.success() {
        return Err(format!(
            "{title}失败：{}",
            command_text(&output.stdout, &output.stderr)
        ));
    }
    Ok(OperationResult {
        success: true,
        message: format!("已完成 {title}"),
    })
}

fn package_name_from_path(path: &Path, manager: &str, collection: &str) -> Option<String> {
    let parts = path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>();
    parts.windows(3).find_map(|window| {
        (window[0].eq_ignore_ascii_case(manager)
            && window[1].eq_ignore_ascii_case(collection)
            && valid_package_name(window[2]))
        .then(|| window[2].to_string())
    })
}

fn valid_package_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
}

fn portable_runtime_root(executable: &Path, kind: &str) -> Result<PathBuf, String> {
    let kind = kind.to_ascii_lowercase();
    let root = match kind.as_str() {
        "java" | "jdk" | "maven" | "gradle" | "go" => executable.parent().and_then(Path::parent),
        "python" | "node" | "node.js" => executable.parent(),
        _ => None,
    }
    .ok_or_else(|| "无法安全识别便携运行时根目录".to_string())?
    .to_path_buf();
    let name = root
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_ascii_lowercase();
    let expected: &[&str] = match kind.as_str() {
        "java" | "jdk" => &["java", "jdk", "openjdk"],
        "python" => &["python", "cpython", "py"],
        "node" | "node.js" => &["node", "nodejs"],
        "maven" => &["maven", "apache-maven"],
        "gradle" => &["gradle"],
        "go" => &["go", "golang"],
        _ => &[],
    };
    let in_windows = env::var_os("WINDIR")
        .map(PathBuf::from)
        .map(|windows| root.starts_with(windows))
        .unwrap_or(false);
    if !root.is_dir()
        || root.parent().and_then(Path::parent).is_none()
        || !expected.iter().any(|token| name.contains(token))
        || in_windows
        || root == dirs::home_dir().unwrap_or_default()
    {
        return Err(format!(
            "没有卸载入口，且目录不符合便携版安全规则，已拒绝删除：{}",
            display_path(root)
        ));
    }
    Ok(root)
}

#[tauri::command]
async fn self_uninstall(app: tauri::AppHandle) -> Result<OperationResult, String> {
    let result = run_blocking(self_uninstall_blocking).await??;
    app.exit(0);
    Ok(result)
}

fn self_uninstall_blocking() -> Result<OperationResult, String> {
    let entry = find_self_uninstall_entry()
        .ok_or_else(|| "没有找到 DevEnv Manager 的卸载入口，请从 Windows 设置中卸载".to_string())?;
    launch_uninstall_string(&entry.uninstall_string)?;
    Ok(OperationResult {
        success: true,
        message: "已启动 DevEnv Manager 卸载程序".to_string(),
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
            storage_cleanup_architecture,
            scan_storage_cleanup,
            scan_cleanup_targets,
            inspect_maintenance_overview,
            inspect_disk_overview,
            jdk_distributions,
            check_for_updates,
            download_update,
            launch_update_installer,
            load_config,
            set_root_dir,
            set_auto_check_update,
            env_snapshot,
            inspect_java_environment,
            inspect_agent_traces,
            configure_user_environment,
            cleanup_path_entries,
            restore_user_environment,
            discover_runtimes,
            install_jdk,
            install_node,
            install_go,
            install_python,
            install_maven_latest,
            install_gradle_latest,
            switch_runtime,
            uninstall_runtime,
            kill_process,
            scan_ports,
            port_history,
            open_process_location,
            run_doctor,
            repair_doctor_safe,
            export_doctor_report,
            export_doctor_report_json,
            doctor_report_text,
            analyze_python_environment,
            inspect_toolchains,
            run_toolchain_action,
            inspect_platform_toolchains,
            run_platform_action,
            inspect_system_platforms,
            manage_system_platform,
            inspect_local_services,
            manage_local_service,
            local_service_logs,
            open_local_service_directory,
            stop_local_service,
            open_docker_desktop,
            project_health,
            inspect_project_port_configs,
            update_project_port,
            analyze_project,
            run_project_action,
            network_diagnostics,
            cache_entries,
            clear_download_cache,
            inspect_command_safety,
            run_tool_command,
            environment_health,
            list_config_profiles,
            config_profile_requirements,
            install_profile_missing,
            save_config_profile,
            apply_config_profile,
            delete_config_profile,
            export_config_profiles,
            preview_config_profiles,
            import_config_profiles,
            uninstall_external_runtime,
            self_uninstall,
            generate_vscode_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running DevEnv Manager");
}

pub fn cli_main() -> i32 {
    match run_cli(std::env::args().skip(1).collect()) {
        Ok(output) => {
            println!("{output}");
            0
        }
        Err(error) => {
            eprintln!("错误：{error}");
            1
        }
    }
}

fn run_cli(args: Vec<String>) -> Result<String, String> {
    let Some(command) = args.first().map(String::as_str) else {
        return Ok(cli_help());
    };
    match command {
        "help" | "--help" | "-h" => Ok(cli_help()),
        "version" | "--version" | "-V" => Ok(format!("devenv {}", env!("CARGO_PKG_VERSION"))),
        "doctor" => {
            let report = run_doctor_blocking()?;
            if args.iter().any(|item| item == "--json") {
                serde_json::to_string_pretty(&report)
                    .map_err(|err| format!("生成 JSON 失败：{err}"))
            } else {
                Ok(redact_report_text(&doctor_report_markdown(&report)))
            }
        }
        "list" => {
            let runtimes = discover_runtimes_blocking();
            if args.iter().any(|item| item == "--json") {
                serde_json::to_string_pretty(&runtimes)
                    .map_err(|err| format!("生成 JSON 失败：{err}"))
            } else if runtimes.is_empty() {
                Ok("没有发现开发运行时".to_string())
            } else {
                Ok(runtimes
                    .into_iter()
                    .map(|item| {
                        format!("{:<10} {:<18} {}", item.kind, item.version, item.executable)
                    })
                    .collect::<Vec<_>>()
                    .join("\n"))
            }
        }
        "use" => {
            let kind = args.get(1).ok_or_else(|| {
                "用法：devenv use <jdk|python|node|maven|gradle|go> <version>".to_string()
            })?;
            let version = args.get(2).ok_or_else(|| "缺少版本号".to_string())?;
            Ok(switch_runtime_blocking(kind.clone(), version.clone(), None)?.message)
        }
        "project" if args.get(1).map(String::as_str) == Some("check") => {
            let root = args
                .get(2)
                .filter(|item| !item.starts_with("--"))
                .map(PathBuf::from)
                .unwrap_or(env::current_dir().map_err(|err| err.to_string())?);
            let analysis = analyze_project_blocking(&root)?;
            if args.iter().any(|item| item == "--json") {
                serde_json::to_string_pretty(&analysis)
                    .map_err(|err| format!("生成 JSON 失败：{err}"))
            } else {
                let recommendations = analysis
                    .recommended_runtime
                    .iter()
                    .map(|item| format!("- {}：{}（{}）", item.name, item.requirement, item.status))
                    .collect::<Vec<_>>()
                    .join("\n");
                Ok(format!(
                    "项目：{}\n类型：{}\n{}",
                    analysis.root,
                    analysis.project_types.join(" / "),
                    recommendations
                ))
            }
        }
        "cleanup" if args.get(1).map(String::as_str) == Some("scan") => {
            let paths = load_paths()?;
            let report = cleanup::scan(&paths.root)?;
            if args.iter().any(|item| item == "--json") {
                serde_json::to_string_pretty(&report)
                    .map_err(|err| format!("生成 JSON 失败：{err}"))
            } else {
                Ok(format!(
                    "扫描到 {} 个统计项，共 {}。\nPhase 1 仅扫描，不会删除、移动或修改文件。",
                    report.total_items,
                    format_byte_size(report.total_bytes)
                ))
            }
        }
        "profile" if args.get(1).map(String::as_str) == Some("list") => {
            let profiles = list_config_profiles_blocking()?;
            if profiles.is_empty() {
                Ok("没有配置模板".to_string())
            } else {
                Ok(profiles
                    .into_iter()
                    .map(|item| format!("{}\t{}\t{}", item.id, item.name, item.created_at))
                    .collect::<Vec<_>>()
                    .join("\n"))
            }
        }
        "profile" if args.get(1).map(String::as_str) == Some("apply") => {
            let id = args
                .get(2)
                .ok_or_else(|| "用法：devenv profile apply <id>".to_string())?;
            Ok(apply_config_profile_blocking(id.clone())?.message)
        }
        _ => Err(format!("未知命令：{command}\n\n{}", cli_help())),
    }
}

fn cli_help() -> String {
    format!(
        "DevEnv Manager CLI {}\n\n用法：\n  devenv doctor [--json]\n  devenv list [--json]\n  devenv use <kind> <version>\n  devenv project check [path] [--json]\n  devenv cleanup scan [--json]\n  devenv profile list\n  devenv profile apply <id>\n  devenv version",
        env!("CARGO_PKG_VERSION")
    )
}

fn format_byte_size(size: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let value = size as f64;
    if value >= GB {
        format!("{:.2} GB", value / GB)
    } else if value >= MB {
        format!("{:.2} MB", value / MB)
    } else if value >= KB {
        format!("{:.2} KB", value / KB)
    } else {
        format!("{size} B")
    }
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
    fn gos(&self) -> PathBuf {
        self.envs().join("gos")
    }
    fn tools(&self) -> PathBuf {
        self.root.join("tools")
    }
    fn npm_global(&self) -> PathBuf {
        self.tools().join("npm-global")
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
    fn profiles_file(&self) -> PathBuf {
        self.config().join("profiles.json")
    }
    fn port_history_file(&self) -> PathBuf {
        self.config().join("port_history.json")
    }

    fn ensure(&self) -> io::Result<()> {
        for path in [
            self.root.clone(),
            self.jdks(),
            self.pythons(),
            self.nodes(),
            self.mavens(),
            self.gradles(),
            self.gos(),
            self.tools(),
            self.npm_global(),
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
        let root = self
            .root
            .canonicalize()
            .unwrap_or_else(|_| self.root.clone());
        let candidate = path
            .parent()
            .and_then(|parent| parent.canonicalize().ok())
            .map(|parent| parent.join(path.file_name().unwrap_or_else(|| OsStr::new(""))))
            .unwrap_or_else(|| path.to_path_buf());
        if candidate != root && !candidate.starts_with(&root) {
            return Err(format!(
                "目标路径不在安装根目录内：{}",
                display_path(candidate)
            ));
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
            gos: display_path(self.gos()),
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

fn normalize_root_dir(input: &str) -> Result<PathBuf, String> {
    let raw = input.trim().trim_matches('"');
    if raw.is_empty() {
        return Err("根目录不能为空".to_string());
    }
    let expanded = PathBuf::from(raw)
        .expand_home()
        .unwrap_or_else(|| PathBuf::from(raw));
    let resolved = expanded.canonicalize().unwrap_or(expanded);
    if is_drive_root(&resolved) {
        Ok(resolved.join(APP_NAME))
    } else {
        Ok(resolved)
    }
}

fn is_drive_root(path: &Path) -> bool {
    let trimmed = display_path(path).trim_end_matches(['\\', '/']).to_string();
    cfg!(windows) && trimmed.len() == 2 && trimmed.as_bytes().get(1) == Some(&b':')
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
        update_manifest_url:
            "https://raw.githubusercontent.com/weidonglang/DevEnv-Manager/main/update-manifest.json"
                .to_string(),
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
        gos: Vec::new(),
        current: CurrentVersions::default(),
    }
}

fn default_profiles() -> Vec<ConfigProfile> {
    Vec::new()
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

fn load_profiles(paths: &AppPaths) -> Result<Vec<ConfigProfile>, String> {
    load_json_with_default(&paths.profiles_file(), default_profiles())
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
    let managed_keys: BTreeSet<String> = MANAGED_PATHS.iter().map(|item| path_key(item)).collect();
    let mut retained = Vec::new();
    let mut seen = BTreeSet::new();
    for item in existing.split(';') {
        let item = item.trim();
        let item_key = path_key(item);
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

fn path_key(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_end_matches(['\\', '/'])
        .to_ascii_lowercase()
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
        let saved_devenv = env_key
            .get_value::<String, _>("DEVENV_HOME")
            .map_err(|err| format!("校验 DEVENV_HOME 失败：{err}"))?;
        let saved_path = env_key
            .get_value::<String, _>("Path")
            .map_err(|err| format!("校验 Path 失败：{err}"))?;
        if path_key(&saved_devenv) != path_key(&display_path(&paths.root)) || saved_path != path {
            return Err("用户环境变量写入后校验不一致，已停止并建议重新打开程序后重试".to_string());
        }
        if let Some(expected) = java_home {
            let saved_java = env_key
                .get_value::<String, _>("JAVA_HOME")
                .map_err(|err| format!("校验 JAVA_HOME 失败：{err}"))?;
            if path_key(&saved_java) != path_key(expected) {
                return Err("JAVA_HOME 写入后校验不一致".to_string());
            }
        }
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
        let _ = hidden_command("powershell")
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
    set_user_environment_values(paths, selected.as_deref(), &merge_path(&current_path))?;
    broadcast_environment_change();
    Ok(())
}

fn select_java_home(
    paths: &AppPaths,
    user_environment: &std::collections::HashMap<String, String>,
) -> Option<String> {
    let managed = paths.current().join("jdk");
    let managed_is_selected = load_installed(paths)
        .ok()
        .and_then(|installed| installed.current.jdk)
        .is_some();
    if managed_is_selected
        && managed.join("bin/java.exe").is_file()
        && managed.join("bin/javac.exe").is_file()
    {
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

fn find_in_configured_path(
    executable: &str,
    path_value: &str,
    paths: &AppPaths,
) -> Option<PathBuf> {
    for entry in path_value
        .split(';')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        let directory = PathBuf::from(expand_environment_path(entry, paths));
        for suffix in [".exe", ".cmd", ".bat", ""] {
            let candidate = directory.join(format!("{executable}{suffix}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn java_root_from_executable(executable: &Path) -> Option<PathBuf> {
    executable.parent()?.parent().map(Path::to_path_buf)
}

fn first_output_line(executable: &Path, args: &[&str]) -> String {
    run_command_output(executable.to_path_buf(), args, 30)
        .unwrap_or_default()
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim()
        .to_string()
}

fn inspect_java_environment_blocking() -> Result<JavaEnvironmentReport, String> {
    let paths = load_paths()?;
    let user = user_environment()?;
    let java_home = user.get("JAVA_HOME").cloned().unwrap_or_default();
    let java_home_expanded = if java_home.is_empty() {
        String::new()
    } else {
        expand_environment_path(&java_home, &paths)
    };
    let path_value = user
        .get("Path")
        .or_else(|| user.get("PATH"))
        .cloned()
        .unwrap_or_default();
    let path_java = find_in_configured_path("java", &path_value, &paths);
    let path_javac = find_in_configured_path("javac", &path_value, &paths);
    let java_version = path_java
        .as_deref()
        .map(|path| first_output_line(path, &["-version"]))
        .unwrap_or_default();
    let javac_version = path_javac
        .as_deref()
        .map(|path| first_output_line(path, &["-version"]))
        .unwrap_or_default();
    let expected_root =
        (!java_home_expanded.is_empty()).then(|| PathBuf::from(&java_home_expanded));
    let java_root = path_java.as_deref().and_then(java_root_from_executable);
    let javac_root = path_javac.as_deref().and_then(java_root_from_executable);
    let home_matches_java = match (&expected_root, &java_root) {
        (Some(home), Some(root)) => path_key(&display_path(home)) == path_key(&display_path(root)),
        (None, None) => true,
        _ => false,
    };
    let java_matches_javac = match (&java_root, &javac_root) {
        (Some(java), Some(javac)) => {
            path_key(&display_path(java)) == path_key(&display_path(javac))
        }
        (None, None) => true,
        _ => false,
    };
    let mut warnings = Vec::new();
    if java_home.is_empty() {
        warnings.push("JAVA_HOME 未设置".to_string());
    } else if !is_valid_java_home(&java_home, &paths) {
        warnings.push("JAVA_HOME 不包含可用的 java.exe 与 javac.exe".to_string());
    }
    if !home_matches_java {
        warnings.push("JAVA_HOME 与用户 PATH 首个 java.exe 不一致".to_string());
    }
    if !java_matches_javac {
        warnings.push("用户 PATH 中 java.exe 与 javac.exe 来自不同 JDK".to_string());
    }
    let candidates = discover_runtimes_blocking()
        .into_iter()
        .filter(|item| item.kind == "Java")
        .collect::<Vec<_>>();
    let effective_source = path_java
        .as_deref()
        .map(|path| classify_source(&display_path(path)))
        .unwrap_or_else(|| "未发现".to_string());
    let maven_runtime = resolve_tool(&paths, "mvn")
        .map(|path| command_value(Some(path), &["-version"]))
        .unwrap_or_default()
        .lines()
        .take(3)
        .collect::<Vec<_>>()
        .join(" · ");
    let gradle_runtime = resolve_tool(&paths, "gradle")
        .map(|path| command_value(Some(path), &["-version"]))
        .unwrap_or_default()
        .lines()
        .filter(|line| line.contains("JVM") || line.contains("Java"))
        .take(2)
        .collect::<Vec<_>>()
        .join(" · ");
    Ok(JavaEnvironmentReport {
        java_home,
        java_home_expanded,
        path_java: path_java.as_deref().map(display_path).unwrap_or_default(),
        path_javac: path_javac.as_deref().map(display_path).unwrap_or_default(),
        java_version,
        javac_version,
        maven_runtime,
        gradle_runtime,
        effective_source,
        consistent: warnings.is_empty(),
        warnings,
        candidates,
    })
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
        let normalized = path_key(entry);
        if !seen.insert(normalized) {
            warnings.push(format!("重复 PATH: {entry}"));
        }
        let expanded = expand_environment_path(entry, paths);
        if !Path::new(&expanded).exists() {
            if is_managed_pending_path(&expanded, paths) {
                warnings.push(format!("托管 PATH 待安装: {entry}"));
            } else if !entry.contains('%') {
                warnings.push(format!("失效 PATH: {entry}"));
            }
        }
    }
    warnings
}

fn is_managed_pending_path(expanded: &str, paths: &AppPaths) -> bool {
    let path = PathBuf::from(expanded);
    path.starts_with(paths.current()) || path.starts_with(paths.tools())
}

#[derive(Debug)]
struct ReleaseInfo {
    name: String,
    url: String,
    sha256: Option<String>,
    tag: String,
}

fn resolve_jdk_release(distribution: &str, version: &str) -> Result<ReleaseInfo, String> {
    match distribution {
        "temurin" => resolve_temurin_release(version),
        "zulu" => resolve_zulu_release(version),
        "liberica" => resolve_liberica_release(version),
        "microsoft" => resolve_microsoft_jdk_release(version),
        _ => Err("不支持该 JDK 发行版".to_string()),
    }
}

fn jdk_distribution_name(distribution: &str) -> &'static str {
    match distribution {
        "temurin" => "Eclipse Temurin",
        "zulu" => "Azul Zulu",
        "liberica" => "BellSoft Liberica",
        "microsoft" => "Microsoft OpenJDK",
        _ => "OpenJDK",
    }
}

fn resolve_temurin_release(version: &str) -> Result<ReleaseInfo, String> {
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

fn resolve_zulu_release(version: &str) -> Result<ReleaseInfo, String> {
    let url = format!(
        "https://api.azul.com/metadata/v1/zulu/packages/?java_version={version}&os=windows&arch=x86_64&archive_type=zip&java_package_type=jdk&release_status=ga&latest=true&page_size=100"
    );
    let response: Value = reqwest::blocking::get(&url)
        .map_err(|err| format!("查询 Azul Zulu 失败：{err}"))?
        .error_for_status()
        .map_err(|err| format!("查询 Azul Zulu 失败：{err}"))?
        .json()
        .map_err(|err| format!("解析 Azul Zulu 响应失败：{err}"))?;
    let packages = response
        .as_array()
        .ok_or_else(|| "Azul Zulu 响应格式异常".to_string())?;
    let package = packages
        .iter()
        .filter(|item| {
            item.get("name")
                .and_then(Value::as_str)
                .map(|name| {
                    !name.contains("-fx-")
                        && !name.contains("-crac-")
                        && name.ends_with("win_x64.zip")
                })
                .unwrap_or(false)
        })
        .max_by_key(|item| {
            item.get("java_version")
                .and_then(Value::as_array)
                .map(|parts| {
                    parts
                        .iter()
                        .take(3)
                        .map(|part| part.as_u64().unwrap_or(0))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .ok_or_else(|| format!("未找到 Zulu JDK {version} 的标准 Windows x64 ZIP"))?;
    let name = package
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| "Azul 响应缺少文件名".to_string())?;
    let download_url = package
        .get("download_url")
        .and_then(Value::as_str)
        .ok_or_else(|| "Azul 响应缺少下载地址".to_string())?;
    Ok(ReleaseInfo {
        name: name.to_string(),
        url: download_url.to_string(),
        sha256: None,
        tag: version.to_string(),
    })
}

fn resolve_liberica_release(version: &str) -> Result<ReleaseInfo, String> {
    let url = format!(
        "https://api.bell-sw.com/v1/liberica/releases?version-feature={version}&version-modifier=latest&os=windows&arch=x86&bitness=64&package-type=zip&bundle-type=jdk"
    );
    let package: Value = reqwest::blocking::get(&url)
        .map_err(|err| format!("查询 BellSoft Liberica 失败：{err}"))?
        .error_for_status()
        .map_err(|err| format!("查询 BellSoft Liberica 失败：{err}"))?
        .json()
        .map_err(|err| format!("解析 BellSoft Liberica 响应失败：{err}"))?;
    let name = package
        .get("filename")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("未找到 Liberica JDK {version} 的 Windows x64 ZIP"))?;
    let download_url = package
        .get("downloadUrl")
        .and_then(Value::as_str)
        .ok_or_else(|| "BellSoft 响应缺少下载地址".to_string())?;
    Ok(ReleaseInfo {
        name: name.to_string(),
        url: download_url.to_string(),
        sha256: None,
        tag: package
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or(version)
            .to_string(),
    })
}

fn resolve_microsoft_jdk_release(version: &str) -> Result<ReleaseInfo, String> {
    let url = format!("https://aka.ms/download-jdk/microsoft-jdk-{version}-windows-x64.zip");
    let checksum_url = format!("{url}.sha256sum.txt");
    let client = reqwest::blocking::Client::builder()
        .user_agent("DevEnvManager/1.1")
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|err| format!("创建 Microsoft JDK 客户端失败：{err}"))?;
    let checksum_text = client
        .get(&checksum_url)
        .send()
        .map_err(|err| format!("读取 Microsoft JDK 校验和失败：{err}"))?
        .error_for_status()
        .map_err(|err| format!("读取 Microsoft JDK 校验和失败：{err}"))?
        .text()
        .map_err(|err| format!("解析 Microsoft JDK 校验和失败：{err}"))?;
    let sha256 = checksum_text
        .split_whitespace()
        .find(|item| item.len() == 64 && item.chars().all(|ch| ch.is_ascii_hexdigit()))
        .ok_or_else(|| "Microsoft JDK 校验和格式异常".to_string())?;
    Ok(ReleaseInfo {
        name: format!("microsoft-jdk-{version}-windows-x64.zip"),
        url,
        sha256: Some(sha256.to_string()),
        tag: version.to_string(),
    })
}

fn resolve_go_release(version: &str) -> Result<ReleaseInfo, String> {
    let items: Value = reqwest::blocking::get("https://go.dev/dl/?mode=json&include=all")
        .map_err(|err| format!("查询 Go 版本失败：{err}"))?
        .error_for_status()
        .map_err(|err| format!("查询 Go 版本失败：{err}"))?
        .json()
        .map_err(|err| format!("解析 Go 版本响应失败：{err}"))?;
    parse_go_release_index(&items, version)
}

fn parse_go_release_index(items: &Value, version: &str) -> Result<ReleaseInfo, String> {
    let release = items
        .as_array()
        .ok_or_else(|| "Go 版本索引格式异常".to_string())?
        .iter()
        .filter(|item| {
            item.get("stable").and_then(Value::as_bool).unwrap_or(false)
                && item
                    .get("version")
                    .and_then(Value::as_str)
                    .map(|tag| {
                        tag.trim_start_matches("go")
                            .starts_with(&format!("{version}."))
                    })
                    .unwrap_or(false)
        })
        .max_by_key(|item| {
            item.get("version")
                .and_then(Value::as_str)
                .map(version_key)
                .unwrap_or_default()
        })
        .ok_or_else(|| format!("Go 官方索引中没有可用的 {version} 稳定版"))?;
    let tag = release
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(|| "Go 版本响应缺少版本号".to_string())?;
    let file = release
        .get("files")
        .and_then(Value::as_array)
        .and_then(|files| {
            files.iter().find(|file| {
                file.get("os").and_then(Value::as_str) == Some("windows")
                    && file.get("arch").and_then(Value::as_str) == Some("amd64")
                    && file.get("kind").and_then(Value::as_str) == Some("archive")
                    && file
                        .get("filename")
                        .and_then(Value::as_str)
                        .map(|name| name.ends_with(".zip"))
                        .unwrap_or(false)
            })
        })
        .ok_or_else(|| format!("没有找到 {tag} 的 Windows x64 ZIP"))?;
    let name = file
        .get("filename")
        .and_then(Value::as_str)
        .ok_or_else(|| "Go 下载项缺少文件名".to_string())?;
    Ok(ReleaseInfo {
        name: name.to_string(),
        url: format!("https://go.dev/dl/{name}"),
        sha256: file
            .get("sha256")
            .and_then(Value::as_str)
            .map(str::to_string),
        tag: tag.to_string(),
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
                    .map(|files| {
                        files
                            .iter()
                            .any(|file| file.as_str() == Some("win-x64-zip"))
                    })
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
    let index: Value =
        reqwest::blocking::get("https://api.nuget.org/v3-flatcontainer/python/index.json")
            .map_err(|err| format!("查询 Python 失败：{err}"))?
            .error_for_status()
            .map_err(|err| format!("查询 Python 失败：{err}"))?
            .json()
            .map_err(|err| format!("解析 Python 版本索引失败：{err}"))?;
    let full_version = index
        .get("versions")
        .and_then(Value::as_array)
        .ok_or_else(|| "Python 版本索引格式异常".to_string())?
        .iter()
        .filter_map(Value::as_str)
        .filter(|value| value.starts_with(&format!("{version}.")))
        .filter(|value| !value.contains('-'))
        .max_by_key(|value| version_key(value))
        .ok_or_else(|| format!("Python {version} 没有可用的 Windows x64 完整包"))?;
    let name = format!("python.{full_version}.nupkg");
    Ok(ReleaseInfo {
        name: name.clone(),
        url: format!("https://api.nuget.org/v3-flatcontainer/python/{full_version}/{name}"),
        sha256: None,
        tag: full_version.to_string(),
    })
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
            !item
                .get("snapshot")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                && !item
                    .get("nightly")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
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

fn download_file_with_progress(
    url: &str,
    target_path: &Path,
    expected_sha256: Option<&str>,
    progress: Option<(&tauri::AppHandle, &str, u8, u8)>,
) -> Result<(), String> {
    validate_download_url(url)?;
    let cache_valid = target_path.exists()
        && target_path.metadata().map(|item| item.len()).unwrap_or(0) > 0
        && expected_sha256
            .map(|expected| {
                file_sha256(target_path)
                    .map(|actual| actual.eq_ignore_ascii_case(expected))
                    .unwrap_or(false)
            })
            .unwrap_or(true);
    if cache_valid {
        if let Some((app, task, _, end)) = progress {
            emit_task_progress(app, task, end, "使用已有下载缓存");
        }
        return Ok(());
    }
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("创建下载目录失败：{err}"))?;
    }
    let temp_path = target_path.with_extension(format!(
        "{}.part",
        target_path
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or("download")
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
    let total = response.content_length();
    let mut file =
        fs::File::create(&temp_path).map_err(|err| format!("写入下载缓存失败：{err}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 128];
    let mut downloaded = 0_u64;
    let mut last_percent = 0_u8;
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
        if let (Some(total), Some((app, task, start, end))) = (total, progress) {
            let span = end.saturating_sub(start) as u64;
            if let Some(completed) = downloaded.saturating_mul(span).checked_div(total) {
                let percent = start.saturating_add(completed as u8).min(end);
                if percent >= last_percent.saturating_add(3) || percent >= end {
                    last_percent = percent;
                    emit_task_progress(
                        app,
                        task,
                        percent,
                        &format!("正在下载 {}", format_size(downloaded)),
                    );
                }
            }
        }
    }
    if downloaded == 0 {
        return Err("服务器返回了空文件".to_string());
    }
    if let Some(expected) = expected_sha256 {
        let actual = format!("{:x}", hasher.finalize());
        if !actual.eq_ignore_ascii_case(expected) {
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

fn install_zip_payload(
    archive: &Path,
    target: &Path,
    required_files: &[&str],
) -> Result<(), String> {
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
    for item in fs::read_dir(temp.path()).map_err(|err| format!("读取解压目录失败：{err}"))?
    {
        let item = item.map_err(|err| err.to_string())?.path();
        if item.is_dir() {
            candidates.push(item);
        }
    }
    let payload = candidates
        .into_iter()
        .find(|candidate| {
            required_files
                .iter()
                .all(|name| candidate.join(name).exists())
        })
        .ok_or_else(|| "无法识别压缩包中的运行时根目录".to_string())?;
    fs::rename(&payload, target).map_err(|err| format!("移动运行时目录失败：{err}"))
}

fn locate_python_exe(target: &Path) -> Option<PathBuf> {
    let direct = target.join("python.exe");
    if direct.is_file() {
        return Some(direct);
    }
    find_file_limited(target, "python.exe", 4)
}

fn find_file_limited(root: &Path, file_name: &str, depth: usize) -> Option<PathBuf> {
    if depth == 0 || !root.is_dir() {
        return None;
    }
    for item in fs::read_dir(root).ok()?.flatten() {
        let path = item.path();
        if path.is_file()
            && path
                .file_name()
                .and_then(OsStr::to_str)
                .map(|name| name.eq_ignore_ascii_case(file_name))
                .unwrap_or(false)
        {
            return Some(path);
        }
        if path.is_dir() {
            if let Some(found) = find_file_limited(&path, file_name, depth - 1) {
                return Some(found);
            }
        }
    }
    None
}

fn safe_extract_zip(archive: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination).map_err(|err| format!("创建解压目录失败：{err}"))?;
    let destination = destination
        .canonicalize()
        .map_err(|err| format!("解析解压目录失败：{err}"))?;
    let file = fs::File::open(archive).map_err(|err| format!("打开压缩包失败：{err}"))?;
    let mut zip = ZipArchive::new(file).map_err(|err| format!("解压失败：{err}"))?;
    for index in 0..zip.len() {
        let mut member = zip
            .by_index(index)
            .map_err(|err| format!("读取压缩包失败：{err}"))?;
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
            let mut out =
                fs::File::create(&target).map_err(|err| format!("写入解压文件失败：{err}"))?;
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
        "go" => Ok(RuntimeMeta {
            kind: "go",
            collection: "gos",
            link_name: "go",
            exe_key: "go_exe",
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
        "gos" => &installed.gos,
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
        "gos" => &mut installed.gos,
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
        "gos" => Ok(paths.gos()),
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
        "go" => installed.current.go.clone(),
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
        "go" => installed.current.go = version,
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
    let output = hidden_command("cmd")
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
    let output = hidden_command("cmd")
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
    hidden_command("cmd")
        .args(["/c", "fsutil", "reparsepoint", "query"])
        .arg(path)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn detect_runtime(kind: &str, executable: &str, args: &[&str]) -> Option<RuntimeInfo> {
    let path = find_on_path(executable).unwrap_or_else(|| executable.to_string());
    detect_runtime_at(kind, Path::new(&path), args, None)
}

fn detect_runtime_at(
    kind: &str,
    executable: &Path,
    args: &[&str],
    source: Option<String>,
) -> Option<RuntimeInfo> {
    if executable.components().count() > 1 && !executable.is_file() {
        return None;
    }
    let output = hidden_command(executable).args(args).output().ok()?;
    let mut text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        text = String::from_utf8_lossy(&output.stderr).trim().to_string();
    }
    let version = text.lines().next().unwrap_or("unknown").to_string();
    let path = display_path(executable);

    Some(RuntimeInfo {
        kind: kind.to_string(),
        version,
        executable: path.clone(),
        source: source.unwrap_or_else(|| classify_source(&path)),
    })
}

fn find_on_path(executable: &str) -> Option<String> {
    find_all_on_path(executable)
        .into_iter()
        .next()
        .map(display_path)
}

fn find_all_on_path(executable: &str) -> Vec<PathBuf> {
    let Some(path_value) = env::var_os("PATH") else {
        return Vec::new();
    };
    let extensions = if cfg!(windows) {
        vec![".exe", ".cmd", ".bat", ""]
    } else {
        vec![""]
    };

    let mut result = Vec::new();
    let mut seen = BTreeSet::new();
    for dir in env::split_paths(&path_value) {
        for ext in &extensions {
            let candidate = dir.join(format!("{executable}{ext}"));
            if candidate.is_file() && seen.insert(path_key(&display_path(&candidate))) {
                result.push(candidate);
            }
        }
    }
    result
}

fn push_runtime(runtimes: &mut Vec<RuntimeInfo>, info: RuntimeInfo) {
    let key = format!(
        "{}|{}",
        info.kind.to_ascii_lowercase(),
        path_key(&info.executable)
    );
    if !runtimes.iter().any(|item| {
        format!(
            "{}|{}",
            item.kind.to_ascii_lowercase(),
            path_key(&item.executable)
        ) == key
    }) {
        runtimes.push(info);
    }
}

fn add_managed_runtime_discoveries(runtimes: &mut Vec<RuntimeInfo>, paths: &AppPaths) {
    let Ok(installed) = load_installed(paths) else {
        return;
    };
    for (label, meta) in [
        ("Java", runtime_meta("jdk")),
        ("Python", runtime_meta("python")),
        ("Node.js", runtime_meta("node")),
        ("Maven", runtime_meta("maven")),
        ("Gradle", runtime_meta("gradle")),
        ("Go", runtime_meta("go")),
    ] {
        let Ok(meta) = meta else {
            continue;
        };
        for item in collection(&installed, meta.collection) {
            let Some(executable) = item.get(meta.exe_key).and_then(Value::as_str) else {
                continue;
            };
            let version = item
                .get("detail")
                .and_then(Value::as_str)
                .or_else(|| item.get("version").and_then(Value::as_str))
                .unwrap_or("unknown")
                .to_string();
            let path = PathBuf::from(executable);
            if path.is_file() {
                push_runtime(
                    runtimes,
                    RuntimeInfo {
                        kind: label.to_string(),
                        version,
                        executable: display_path(path),
                        source: "DevEnv managed".to_string(),
                    },
                );
            }
        }
    }
}

fn add_python_launcher_discoveries(runtimes: &mut Vec<RuntimeInfo>) {
    let Ok(output) = hidden_command("py").arg("-0p").output() else {
        return;
    };
    let text = command_text(&output.stdout, &output.stderr);
    for line in text.lines() {
        let Some(path) = extract_windows_path(line) else {
            continue;
        };
        if let Some(info) = detect_runtime_at(
            "Python",
            Path::new(&path),
            &["--version"],
            Some("py launcher".to_string()),
        ) {
            push_runtime(runtimes, info);
        }
    }
}

#[cfg(windows)]
fn add_python_registry_discoveries(runtimes: &mut Vec<RuntimeInfo>) {
    for root in [
        RegKey::predef(HKEY_CURRENT_USER),
        RegKey::predef(HKEY_LOCAL_MACHINE),
    ] {
        let Ok(core) = root.open_subkey(r"Software\Python\PythonCore") else {
            continue;
        };
        for version in core.enum_keys().flatten() {
            let Ok(install) = core.open_subkey(format!(r"{version}\InstallPath")) else {
                continue;
            };
            let executable = install
                .get_value::<String, _>("ExecutablePath")
                .ok()
                .map(PathBuf::from)
                .or_else(|| {
                    install
                        .get_value::<String, _>("")
                        .ok()
                        .map(|path| PathBuf::from(path).join("python.exe"))
                });
            if let Some(executable) = executable {
                if let Some(info) = detect_runtime_at(
                    "Python",
                    &executable,
                    &["--version"],
                    Some("Python registry".to_string()),
                ) {
                    push_runtime(runtimes, info);
                }
            }
        }
    }
    add_java_registry_discoveries(runtimes);
    add_java_common_dir_discoveries(runtimes);
}

#[cfg(not(windows))]
fn add_python_registry_discoveries(_runtimes: &mut Vec<RuntimeInfo>) {}

#[cfg(windows)]
fn add_java_registry_discoveries(runtimes: &mut Vec<RuntimeInfo>) {
    for root in [
        RegKey::predef(HKEY_CURRENT_USER),
        RegKey::predef(HKEY_LOCAL_MACHINE),
    ] {
        let Ok(java_soft) = root.open_subkey(r"Software\JavaSoft\JDK") else {
            continue;
        };
        for version in java_soft.enum_keys().flatten() {
            let Ok(key) = java_soft.open_subkey(version) else {
                continue;
            };
            let Ok(java_home) = key.get_value::<String, _>("JavaHome") else {
                continue;
            };
            let executable = PathBuf::from(java_home).join("bin/java.exe");
            if let Some(info) = detect_runtime_at(
                "Java",
                &executable,
                &["-version"],
                Some("Java registry".to_string()),
            ) {
                push_runtime(runtimes, info);
            }
        }
    }
}

#[cfg(windows)]
fn add_java_common_dir_discoveries(runtimes: &mut Vec<RuntimeInfo>) {
    for base in [
        r"C:\Program Files\Java",
        r"C:\Program Files\Eclipse Adoptium",
        r"D:\Java",
        r"D:\Program Files\Java",
        r"D:\Program Files\Eclipse Adoptium",
    ] {
        let base = Path::new(base);
        let Ok(items) = fs::read_dir(base) else {
            continue;
        };
        for item in items.flatten() {
            let executable = item.path().join("bin/java.exe");
            if let Some(info) = detect_runtime_at(
                "Java",
                &executable,
                &["-version"],
                Some("common install dir".to_string()),
            ) {
                push_runtime(runtimes, info);
            }
        }
    }
}

fn extract_windows_path(line: &str) -> Option<String> {
    let start = line
        .find(":\\")
        .and_then(|index| line[..index].char_indices().last().map(|(pos, _)| pos))?;
    Some(line[start..].trim().trim_matches('"').to_string())
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
        (
            "HTTP_PROXY".to_string(),
            env::var("HTTP_PROXY").unwrap_or_default(),
        ),
        (
            "HTTPS_PROXY".to_string(),
            env::var("HTTPS_PROXY").unwrap_or_default(),
        ),
        (
            "NO_PROXY".to_string(),
            env::var("NO_PROXY").unwrap_or_default(),
        ),
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
    let mut chars = command.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if matches!(chars.peek(), Some('"') | Some('\'') | Some('\\')) {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            } else {
                current.push(ch);
            }
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

    if quote.is_some() {
        return Err("命令包含未闭合的引号".to_string());
    }
    if !current.is_empty() {
        parts.push(current);
    }
    Ok(parts)
}

fn run_command_output(
    executable: PathBuf,
    args: &[&str],
    timeout_seconds: u64,
) -> Result<String, String> {
    let output = hidden_command(executable)
        .args(args)
        .output()
        .map_err(|err| format!("执行命令失败：{err}"))?;
    let _ = timeout_seconds;
    if !output.status.success() {
        return Err(command_text(&output.stdout, &output.stderr));
    }
    Ok(command_text(&output.stdout, &output.stderr))
}

fn process_details(system: &sysinfo::System, pid: u32) -> (String, String, u32, String) {
    let Some(process) = system.process(sysinfo::Pid::from_u32(pid)) else {
        return (String::new(), String::new(), 0, String::new());
    };
    let process_path = process.exe().map(display_path).unwrap_or_default();
    let command_line = process
        .cmd()
        .iter()
        .map(|item| item.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let parent_pid = process.parent().map(|value| value.as_u32()).unwrap_or(0);
    let parent_process_name = system
        .process(sysinfo::Pid::from_u32(parent_pid))
        .map(|parent| parent.name().to_string_lossy().to_string())
        .unwrap_or_default();
    (process_path, command_line, parent_pid, parent_process_name)
}

fn windows_service_map() -> std::collections::HashMap<u32, Vec<String>> {
    let mut result = std::collections::HashMap::new();
    #[cfg(windows)]
    {
        let Ok(output) = hidden_command("tasklist")
            .args(["/svc", "/fo", "csv", "/nh"])
            .output()
        else {
            return result;
        };
        for line in decode_command_stream(&output.stdout).lines() {
            let columns = parse_csv_line(line);
            let Some(pid) = columns.get(1).and_then(|value| value.parse::<u32>().ok()) else {
                continue;
            };
            let services = columns
                .get(2)
                .map(|value| {
                    value
                        .split(',')
                        .map(str::trim)
                        .filter(|item| !item.is_empty() && !item.eq_ignore_ascii_case("N/A"))
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if !services.is_empty() {
                result.insert(pid, services);
            }
        }
    }
    result
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                current.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => values.push(std::mem::take(&mut current)),
            _ => current.push(ch),
        }
    }
    values.push(current);
    values
}

fn port_common_usage(port: u16, process_name: &str) -> String {
    let lower = process_name.to_ascii_lowercase();
    let known = match port {
        80 => "HTTP Web 服务",
        443 => "HTTPS Web 服务",
        1433 => "SQL Server",
        3000 | 4173 | 5173 | 5174 => "前端开发服务",
        3306 => "MySQL",
        5432 => "PostgreSQL",
        6379 => "Redis",
        8005 | 8009 | 8443 => "Tomcat",
        8000 => "常见 Web 开发服务",
        8080..=8082 => "Spring Boot / Tomcat / Web 服务",
        8761 => "Spring Cloud Eureka",
        8888 => "Jupyter / Spring Config",
        9200 => "Elasticsearch",
        27017 => "MongoDB",
        _ if lower.contains("java") => "Java 应用",
        _ if lower.contains("node") => "Node.js 应用",
        _ if lower.contains("python") => "Python 应用",
        _ => "未识别的本地服务",
    };
    known.to_string()
}

fn port_explanation(port: u16, process_name: &str, usage: &str) -> String {
    let lower = process_name.to_ascii_lowercase();
    if matches!(port, 8080..=8082) && lower.contains("java") {
        "可能是 Spring Boot、Tomcat，或 IDE 启动的 Java Web 服务。".to_string()
    } else if matches!(port, 3000 | 4173 | 5173 | 5174) && lower.contains("node") {
        "可能是 Vite、Next.js、React/Vue 开发服务器或其他 Node.js 服务。".to_string()
    } else {
        format!("该端口通常用于{usage}；结束前请确认对应项目不再需要。")
    }
}

fn update_port_history(records: &[PortRecord]) -> Result<(), String> {
    let paths = load_paths()?;
    let mut history: Vec<PortHistoryEntry> =
        load_json_with_default(&paths.port_history_file(), Vec::new())?;
    let now = unix_timestamp();
    let retention_start = now.saturating_sub(7 * 24 * 60 * 60);
    history.retain(|entry| entry.observed_at >= retention_start);
    for record in records
        .iter()
        .filter(|record| record.state.eq_ignore_ascii_case("LISTENING"))
    {
        let duplicate = history.iter().rev().take(200).any(|entry| {
            entry.port == record.local_port
                && entry.pid == record.pid
                && entry
                    .process_name
                    .eq_ignore_ascii_case(&record.process_name)
                && now.saturating_sub(entry.observed_at) < 5 * 60
        });
        if !duplicate {
            history.push(PortHistoryEntry {
                port: record.local_port,
                protocol: record.protocol.clone(),
                pid: record.pid,
                process_name: record.process_name.clone(),
                process_path: record.process_path.clone(),
                observed_at: now,
            });
        }
    }
    if history.len() > 2_000 {
        history.drain(0..history.len() - 2_000);
    }
    save_json(&paths.port_history_file(), &history)
}

fn run_managed_command_output(
    paths: &AppPaths,
    executable: PathBuf,
    args: &[&str],
    timeout_seconds: u64,
) -> Result<String, String> {
    let mut command = hidden_command(executable);
    command.args(args);
    apply_managed_environment(paths, &mut command);
    let output = command
        .output()
        .map_err(|err| format!("执行命令失败：{err}"))?;
    let _ = timeout_seconds;
    if !output.status.success() {
        return Err(command_text(&output.stdout, &output.stderr));
    }
    Ok(command_text(&output.stdout, &output.stderr))
}

fn apply_managed_environment(paths: &AppPaths, command: &mut Command) {
    command.env("DEVENV_HOME", display_path(&paths.root));
    let java_home = paths.current().join("jdk");
    if java_home.join("bin/java.exe").is_file() {
        command.env("JAVA_HOME", display_path(&java_home));
    }

    let current_path = env::var("PATH").unwrap_or_default();
    let mut entries = Vec::new();
    for item in [
        paths.current().join("jdk/bin"),
        paths.current().join("python"),
        paths.current().join("python/Scripts"),
        paths.current().join("node"),
        paths.current().join("maven/bin"),
        paths.current().join("gradle/bin"),
        paths.current().join("go/bin"),
        paths.npm_global(),
    ] {
        entries.push(display_path(item));
    }
    entries.extend(
        current_path
            .split(';')
            .map(|item| expand_environment_path(item, paths))
            .filter(|item| !item.trim().is_empty()),
    );
    command.env("PATH", entries.join(";"));
}

fn command_text(stdout: &[u8], stderr: &[u8]) -> String {
    let stdout = decode_command_stream(stdout).trim().to_string();
    let stderr = decode_command_stream(stderr).trim().to_string();
    [stdout, stderr]
        .into_iter()
        .filter(|item| !item.is_empty())
        .collect::<Vec<String>>()
        .join("\n")
}

fn decode_command_stream(bytes: &[u8]) -> String {
    let looks_utf16 = bytes.len() >= 4
        && bytes.len().is_multiple_of(2)
        && bytes
            .iter()
            .skip(1)
            .step_by(2)
            .filter(|byte| **byte == 0)
            .count()
            > bytes.len() / 6;
    if looks_utf16 {
        let words = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        String::from_utf16_lossy(&words)
            .trim_start_matches('\u{feff}')
            .to_string()
    } else {
        String::from_utf8_lossy(bytes).to_string()
    }
}

fn hidden_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    hide_command_window(&mut command);
    command
}

fn hide_command_window(command: &mut Command) {
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
}

fn check_executable_health(name: &str, executable: &Path, args: &[&str]) -> EnvHealthCheck {
    if !executable.is_file() {
        return EnvHealthCheck {
            name: name.to_string(),
            status: "未安装".to_string(),
            detail: format!("未发现 {}", display_path(executable)),
        };
    }
    match run_command_output(executable.to_path_buf(), args, 30) {
        Ok(output) => EnvHealthCheck {
            name: name.to_string(),
            status: "正常".to_string(),
            detail: output.lines().next().unwrap_or("验证通过").to_string(),
        },
        Err(err) => EnvHealthCheck {
            name: name.to_string(),
            status: "异常".to_string(),
            detail: err,
        },
    }
}

fn check_managed_executable_health(
    paths: &AppPaths,
    name: &str,
    executable: &Path,
    args: &[&str],
) -> EnvHealthCheck {
    if !executable.is_file() {
        return EnvHealthCheck {
            name: name.to_string(),
            status: "未安装".to_string(),
            detail: format!("未发现 {}", display_path(executable)),
        };
    }
    match run_managed_command_output(paths, executable.to_path_buf(), args, 30) {
        Ok(output) => EnvHealthCheck {
            name: name.to_string(),
            status: "正常".to_string(),
            detail: output.lines().next().unwrap_or("验证通过").to_string(),
        },
        Err(err) => EnvHealthCheck {
            name: name.to_string(),
            status: "异常".to_string(),
            detail: err,
        },
    }
}

#[cfg(windows)]
fn uninstall_entries() -> Vec<UninstallEntry> {
    let mut entries = Vec::new();
    for root in [
        RegKey::predef(HKEY_CURRENT_USER),
        RegKey::predef(HKEY_LOCAL_MACHINE),
    ] {
        for subkey in [
            r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
            r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        ] {
            let Ok(uninstall) = root.open_subkey(subkey) else {
                continue;
            };
            for name in uninstall.enum_keys().flatten() {
                let Ok(app) = uninstall.open_subkey(name) else {
                    continue;
                };
                let display_name = app
                    .get_value::<String, _>("DisplayName")
                    .unwrap_or_default();
                let uninstall_string = app
                    .get_value::<String, _>("UninstallString")
                    .unwrap_or_default();
                if display_name.trim().is_empty() || uninstall_string.trim().is_empty() {
                    continue;
                }
                entries.push(UninstallEntry {
                    display_name,
                    install_location: app
                        .get_value::<String, _>("InstallLocation")
                        .unwrap_or_default(),
                    display_icon: app
                        .get_value::<String, _>("DisplayIcon")
                        .unwrap_or_default(),
                    uninstall_string,
                });
            }
        }
    }
    entries
}

#[cfg(not(windows))]
fn uninstall_entries() -> Vec<UninstallEntry> {
    Vec::new()
}

fn find_uninstall_entry_for_path(executable: &Path, kind: &str) -> Option<UninstallEntry> {
    let executable_key = path_key(&display_path(executable));
    let executable_roots = executable_candidate_roots(executable);
    let kind_words = uninstall_kind_words(kind);
    uninstall_entries()
        .into_iter()
        .filter(|entry| {
            let name = entry.display_name.to_ascii_lowercase();
            kind_words.is_empty() || kind_words.iter().any(|word| name.contains(word))
        })
        .map(|entry| {
            let mut score = 0;
            let name = entry.display_name.to_ascii_lowercase();
            for word in &kind_words {
                if name.contains(word) {
                    score += 2;
                }
            }
            for candidate in [
                &entry.install_location,
                &entry.display_icon,
                &entry.uninstall_string,
            ] {
                if candidate.trim().is_empty() {
                    continue;
                }
                for candidate_part in path_like_parts(candidate) {
                    let candidate_key = path_key(&candidate_part);
                    if candidate_key.is_empty() {
                        continue;
                    }
                    if candidate_key == executable_key {
                        score += 30;
                    } else if executable_key.starts_with(&candidate_key)
                        || candidate_key.starts_with(&executable_key)
                    {
                        score += 18;
                    } else if executable_roots.iter().any(|root| {
                        candidate_key.starts_with(root) || root.starts_with(&candidate_key)
                    }) {
                        score += 12;
                    }
                }
            }
            (score, entry)
        })
        .filter(|(score, _)| *score >= 10)
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, entry)| entry)
}

fn executable_candidate_roots(executable: &Path) -> Vec<String> {
    let mut roots = Vec::new();
    let mut current = executable.parent();
    for _ in 0..5 {
        let Some(path) = current else {
            break;
        };
        roots.push(path_key(&display_path(path)));
        current = path.parent();
    }
    roots
}

fn path_like_parts(value: &str) -> Vec<String> {
    let cleaned = value.trim().trim_matches('"');
    let mut parts = vec![cleaned
        .split(',')
        .next()
        .unwrap_or(cleaned)
        .trim()
        .trim_matches('"')
        .to_string()];
    for token in parse_command_line(cleaned).unwrap_or_default() {
        if token.contains(":\\") || token.contains("\\\\") {
            parts.push(
                token
                    .split(',')
                    .next()
                    .unwrap_or(&token)
                    .trim_matches('"')
                    .to_string(),
            );
        }
    }
    parts
        .into_iter()
        .filter(|item| !item.trim().is_empty())
        .collect()
}

fn find_self_uninstall_entry() -> Option<UninstallEntry> {
    let current = env::current_exe()
        .ok()
        .map(|path| path_key(&display_path(path)));
    uninstall_entries().into_iter().find(|entry| {
        let name = entry.display_name.to_ascii_lowercase();
        if name == "devenv manager" || name.contains("devenv manager") {
            return true;
        }
        if let Some(current) = current.as_deref() {
            let install_key = path_key(&entry.install_location);
            return !install_key.is_empty() && current.starts_with(&install_key);
        }
        false
    })
}

fn uninstall_kind_words(kind: &str) -> Vec<&'static str> {
    match kind.to_ascii_lowercase().as_str() {
        "java" | "jdk" => vec!["java", "jdk", "temurin", "adoptium", "oracle"],
        "python" | "python launcher" => vec!["python"],
        "node.js" | "node" | "npm" => vec!["node", "node.js"],
        "maven" => vec!["maven"],
        "gradle" => vec!["gradle"],
        "go" => vec!["go programming language", "golang", "go1."],
        _ => vec![],
    }
}

fn launch_uninstall_string(uninstall_string: &str) -> Result<(), String> {
    let mut parts = parse_command_line(uninstall_string)?;
    let executable = parts
        .first()
        .cloned()
        .ok_or_else(|| "卸载命令为空".to_string())?;
    if executable.to_ascii_lowercase().contains("msiexec") {
        for part in parts.iter_mut().skip(1) {
            if part.eq_ignore_ascii_case("/i") || part.eq_ignore_ascii_case("/I") {
                *part = "/X".to_string();
                break;
            }
        }
    }
    hidden_command(&executable)
        .args(parts.iter().skip(1))
        .spawn()
        .map_err(|err| format!("启动卸载程序失败：{err}"))?;
    Ok(())
}

fn push_doctor_check(checks: &mut Vec<DoctorCheck>, score: &mut i32, check: DoctorCheck) {
    let penalty = match (check.severity.as_str(), check.status.as_str()) {
        ("warning", "异常" | "未安装" | "未设置" | "缺失" | "需修复") => 12,
        ("warning", _) => 8,
        ("notice", "占用" | "可选缺失") => 0,
        ("notice", "不可访问") => 3,
        ("notice", _) => 2,
        _ => 0,
    };
    *score -= penalty;
    checks.push(check);
}

fn doctor_check_needs_attention(check: &DoctorCheck) -> bool {
    check.severity == "warning"
        || (check.severity == "notice"
            && !matches!(
                check.status.as_str(),
                "正常" | "可选缺失" | "占用" | "已发现"
            ))
}

fn optional_command_probe(name: &str, executable: &str, args: &[&str]) -> DoctorCheck {
    let fix_action = if matches!(name, "Go" | "Rust" | ".NET") {
        "platforms"
    } else {
        "discover_runtimes"
    };
    match detect_runtime(name, executable, args) {
        Some(info) => DoctorCheck {
            id: format!("tool-{}", slug(name)),
            title: name.to_string(),
            category: "扩展工具".to_string(),
            status: "正常".to_string(),
            severity: "info".to_string(),
            detail: format!("{} · {}", info.version, info.executable),
            fix_action: Some(fix_action.to_string()),
        },
        None => DoctorCheck {
            id: format!("tool-{}", slug(name)),
            title: name.to_string(),
            category: "扩展工具".to_string(),
            status: "可选缺失".to_string(),
            severity: "info".to_string(),
            detail: format!("没有找到 {executable}；只有对应项目或生态功能需要它"),
            fix_action: Some(fix_action.to_string()),
        },
    }
}

fn tool_registry() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            id: "jdk",
            name: "JDK",
            category: "runtime",
            exe_names: &["java", "javac"],
            env_vars: &["JAVA_HOME"],
            managed_path_entries: &[r"%DEVENV_HOME%\current\jdk\bin"],
            supports_install: true,
            supports_switch: true,
            supports_mirror: false,
        },
        ToolDefinition {
            id: "python",
            name: "Python",
            category: "runtime",
            exe_names: &["python", "pip"],
            env_vars: &[],
            managed_path_entries: &[
                r"%DEVENV_HOME%\current\python",
                r"%DEVENV_HOME%\current\python\Scripts",
            ],
            supports_install: true,
            supports_switch: true,
            supports_mirror: true,
        },
        ToolDefinition {
            id: "node",
            name: "Node.js",
            category: "runtime",
            exe_names: &["node", "npm", "npx"],
            env_vars: &[],
            managed_path_entries: &[r"%DEVENV_HOME%\current\node"],
            supports_install: true,
            supports_switch: true,
            supports_mirror: true,
        },
        ToolDefinition {
            id: "maven",
            name: "Maven",
            category: "build",
            exe_names: &["mvn"],
            env_vars: &["MAVEN_HOME"],
            managed_path_entries: &[r"%DEVENV_HOME%\current\maven\bin"],
            supports_install: true,
            supports_switch: true,
            supports_mirror: true,
        },
        ToolDefinition {
            id: "gradle",
            name: "Gradle",
            category: "build",
            exe_names: &["gradle"],
            env_vars: &["GRADLE_HOME"],
            managed_path_entries: &[r"%DEVENV_HOME%\current\gradle\bin"],
            supports_install: true,
            supports_switch: true,
            supports_mirror: true,
        },
        ToolDefinition {
            id: "git",
            name: "Git",
            category: "scm",
            exe_names: &["git", "git-lfs", "ssh"],
            env_vars: &[],
            managed_path_entries: &[],
            supports_install: false,
            supports_switch: false,
            supports_mirror: false,
        },
        ToolDefinition {
            id: "go",
            name: "Go",
            category: "runtime",
            exe_names: &["go"],
            env_vars: &["GOROOT", "GOPATH", "GOPROXY"],
            managed_path_entries: &[r"%DEVENV_HOME%\current\go\bin"],
            supports_install: true,
            supports_switch: true,
            supports_mirror: true,
        },
        ToolDefinition {
            id: "rust",
            name: "Rust",
            category: "runtime",
            exe_names: &["rustup", "rustc", "cargo"],
            env_vars: &["RUSTUP_HOME", "CARGO_HOME"],
            managed_path_entries: &[],
            supports_install: false,
            supports_switch: false,
            supports_mirror: true,
        },
        ToolDefinition {
            id: "dotnet",
            name: ".NET SDK",
            category: "runtime",
            exe_names: &["dotnet"],
            env_vars: &["DOTNET_ROOT"],
            managed_path_entries: &[],
            supports_install: false,
            supports_switch: false,
            supports_mirror: false,
        },
        ToolDefinition {
            id: "pnpm",
            name: "pnpm",
            category: "node-ecosystem",
            exe_names: &["pnpm"],
            env_vars: &["PNPM_HOME"],
            managed_path_entries: &[],
            supports_install: true,
            supports_switch: false,
            supports_mirror: true,
        },
        ToolDefinition {
            id: "yarn",
            name: "Yarn",
            category: "node-ecosystem",
            exe_names: &["yarn"],
            env_vars: &[],
            managed_path_entries: &[],
            supports_install: true,
            supports_switch: false,
            supports_mirror: true,
        },
        ToolDefinition {
            id: "python-tools",
            name: "Python 工具",
            category: "python-ecosystem",
            exe_names: &["uv", "poetry", "virtualenv"],
            env_vars: &[],
            managed_path_entries: &[],
            supports_install: true,
            supports_switch: false,
            supports_mirror: true,
        },
    ]
}

fn set_user_environment_variable(name: &str, value: Option<&str>) -> Result<(), String> {
    if !["GOPROXY"].contains(&name) {
        return Err("拒绝写入未授权的用户环境变量".to_string());
    }
    #[cfg(windows)]
    {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (env_key, _) = hkcu
            .create_subkey("Environment")
            .map_err(|err| format!("打开用户环境变量失败：{err}"))?;
        if let Some(value) = value {
            env_key
                .set_value(name, &value)
                .map_err(|err| format!("写入 {name} 失败：{err}"))?;
        } else {
            let _ = env_key.delete_value(name);
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = value;
        Err("用户环境变量管理仅支持 Windows".to_string())
    }
}

fn backup_before_write(target: &Path) -> Result<Option<PathBuf>, String> {
    let home = dirs::home_dir().ok_or_else(|| "无法定位用户目录".to_string())?;
    if !target.starts_with(&home) {
        return Err("拒绝修改用户目录之外的配置文件".to_string());
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("创建配置目录失败：{err}"))?;
    }
    if !target.exists() {
        return Ok(None);
    }
    let name = target
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("config");
    let backup = target.with_file_name(format!("{name}.devenv-backup-{}", filename_timestamp()));
    fs::copy(target, &backup).map_err(|err| format!("备份配置失败：{err}"))?;
    Ok(Some(backup))
}

fn restore_latest_backup(target: &Path) -> Result<String, String> {
    let home = dirs::home_dir().ok_or_else(|| "无法定位用户目录".to_string())?;
    if !target.starts_with(&home) {
        return Err("拒绝恢复用户目录之外的配置文件".to_string());
    }
    let parent = target.parent().ok_or_else(|| "配置路径无效".to_string())?;
    let name = target
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("config");
    let prefix = format!("{name}.devenv-backup-");
    let backup = fs::read_dir(parent)
        .map_err(|err| format!("读取备份目录失败：{err}"))?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_name().to_string_lossy().starts_with(&prefix) && entry.path().is_file()
        })
        .max_by_key(|entry| entry.metadata().and_then(|meta| meta.modified()).ok())
        .map(|entry| entry.path())
        .ok_or_else(|| format!("没有找到 {name} 的 DevEnv Manager 备份"))?;
    let current_backup = backup_before_write(target)?;
    fs::copy(&backup, target).map_err(|err| format!("恢复配置失败：{err}"))?;
    Ok(match current_backup {
        Some(current) => format!(
            "已从 {} 恢复配置；恢复前状态已备份到 {}",
            display_path(backup),
            display_path(current)
        ),
        None => format!("已从 {} 恢复配置", display_path(backup)),
    })
}

fn write_maven_settings(target: &Path, mirror: Option<(&str, &str)>) -> Result<(), String> {
    fs::write(target, maven_settings_content(mirror))
        .map_err(|err| format!("写入 Maven 配置失败：{err}"))
}

fn maven_settings_content(mirror: Option<(&str, &str)>) -> String {
    let mirror_xml = mirror
        .map(|(id, url)| {
            format!(
                "    <mirror>\n      <id>{id}</id>\n      <name>DevEnv Manager mirror</name>\n      <url>{url}</url>\n      <mirrorOf>*</mirrorOf>\n    </mirror>\n"
            )
        })
        .unwrap_or_default();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<settings xmlns=\"http://maven.apache.org/SETTINGS/1.2.0\"\n          xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"\n          xsi:schemaLocation=\"http://maven.apache.org/SETTINGS/1.2.0 https://maven.apache.org/xsd/settings-1.2.0.xsd\">\n  <!-- Generated by DevEnv Manager. Previous file is backed up before replacement. -->\n  <mirrors>\n{mirror_xml}  </mirrors>\n</settings>\n"
    )
}

fn write_gradle_init(target: &Path, mirror: Option<&str>) -> Result<(), String> {
    fs::write(target, gradle_init_content(mirror))
        .map_err(|err| format!("写入 Gradle 配置失败：{err}"))
}

fn gradle_init_content(mirror: Option<&str>) -> String {
    mirror
        .map(|url| {
            format!(
                "// Generated by DevEnv Manager. Previous file is backed up before replacement.\nallprojects {{\n    repositories {{\n        maven {{ url '{url}' }}\n        mavenCentral()\n        gradlePluginPortal()\n    }}\n}}\n"
            )
        })
        .unwrap_or_else(|| "// Generated by DevEnv Manager. Using Gradle default repositories.\n".to_string())
}

fn config_write_message(label: &str, target: &Path, backup: Option<&Path>) -> String {
    match backup {
        Some(backup) => format!(
            "已更新 {label}：{}；原配置已备份到 {}",
            display_path(target),
            display_path(backup)
        ),
        None => format!("已创建 {label}：{}", display_path(target)),
    }
}

fn detect_msvc_build_tools() -> String {
    #[cfg(windows)]
    {
        let program_files_x86 =
            env::var("ProgramFiles(x86)").unwrap_or_else(|_| r"C:\Program Files (x86)".to_string());
        let vswhere =
            PathBuf::from(program_files_x86).join("Microsoft Visual Studio/Installer/vswhere.exe");
        if vswhere.is_file() {
            let result = command_value(
                Some(vswhere),
                &[
                    "-latest",
                    "-products",
                    "*",
                    "-requires",
                    "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                    "-property",
                    "installationPath",
                ],
            );
            if !result.trim().is_empty() {
                return result;
            }
        }
    }
    "未发现 MSVC Build Tools".to_string()
}

fn resolve_tool(paths: &AppPaths, executable: &str) -> Option<PathBuf> {
    let managed = match executable {
        "python" => Some(paths.current().join("python/python.exe")),
        "node" => Some(paths.current().join("node/node.exe")),
        "npm" => Some(paths.current().join("node/npm.cmd")),
        "npx" => Some(paths.current().join("node/npx.cmd")),
        "corepack" => Some(paths.current().join("node/corepack.cmd")),
        "pnpm" => Some(paths.current().join("node/pnpm.cmd")),
        "yarn" => Some(paths.current().join("node/yarn.cmd")),
        "go" => Some(paths.current().join("go/bin/go.exe")),
        _ => None,
    };
    managed
        .filter(|path| path.is_file())
        .or_else(|| find_all_on_path(executable).into_iter().next())
        .or_else(|| find_on_user_path(paths, executable))
}

fn find_on_user_path(paths: &AppPaths, executable: &str) -> Option<PathBuf> {
    let values = user_environment().ok()?;
    let path_value = values.get("Path").or_else(|| values.get("PATH"))?;
    let extensions = if cfg!(windows) {
        vec![".exe", ".cmd", ".bat", ""]
    } else {
        vec![""]
    };
    for raw_dir in path_value.split(';') {
        let dir = PathBuf::from(expand_environment_path(raw_dir, paths));
        for extension in &extensions {
            let candidate = dir.join(format!("{executable}{extension}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn probe_tool(name: &str, executable: Option<PathBuf>, args: &[&str]) -> ToolState {
    let Some(executable) = executable else {
        return ToolState {
            name: name.to_string(),
            installed: false,
            version: "未安装".to_string(),
            path: String::new(),
            detail: "没有在受管目录、当前 PATH 或用户 PATH 中找到".to_string(),
        };
    };
    let output = hidden_command(&executable).args(args).output();
    match output {
        Ok(output) => {
            let detail = command_text(&output.stdout, &output.stderr);
            ToolState {
                name: name.to_string(),
                installed: output.status.success(),
                version: detail.lines().next().unwrap_or("未返回版本").to_string(),
                path: display_path(&executable),
                detail: if output.status.success() {
                    classify_source(&display_path(&executable))
                } else {
                    detail
                },
            }
        }
        Err(err) => ToolState {
            name: name.to_string(),
            installed: false,
            version: "无法运行".to_string(),
            path: display_path(executable),
            detail: format!("执行失败：{err}"),
        },
    }
}

fn tool_state_doctor_check(state: ToolState, required: bool) -> DoctorCheck {
    DoctorCheck {
        id: format!("tool-{}", slug(&state.name)),
        title: state.name,
        category: "工具链".to_string(),
        status: if state.installed {
            "正常"
        } else if required {
            "未安装"
        } else {
            "可选缺失"
        }
        .to_string(),
        severity: if !state.installed && required {
            "warning"
        } else {
            "info"
        }
        .to_string(),
        detail: if state.installed {
            format!("{} · {}", state.version, state.path)
        } else {
            state.detail
        },
        fix_action: Some("toolchains".to_string()),
    }
}

fn command_value(executable: Option<PathBuf>, args: &[&str]) -> String {
    executable
        .and_then(|path| hidden_command(path).args(args).output().ok())
        .filter(|output| output.status.success())
        .map(|output| command_text(&output.stdout, &output.stderr))
        .unwrap_or_default()
}

fn run_action_command(
    paths: &AppPaths,
    executable: PathBuf,
    args: &[&str],
) -> Result<String, String> {
    let mut command = hidden_command(executable);
    command.args(args);
    apply_managed_environment(paths, &mut command);
    let output = command
        .output()
        .map_err(|err| format!("执行命令失败：{err}"))?;
    let text = command_text(&output.stdout, &output.stderr);
    if output.status.success() {
        Ok(text)
    } else if text.is_empty() {
        Err(format!(
            "命令执行失败，退出码 {}",
            output.status.code().unwrap_or(-1)
        ))
    } else {
        Err(text)
    }
}

fn validate_setting(value: Option<&str>, label: &str) -> Result<String, String> {
    let value = value.unwrap_or_default().trim();
    if value.is_empty() {
        return Err(format!("{label}不能为空"));
    }
    if value.len() > 200 || value.chars().any(char::is_control) {
        return Err(format!("{label}格式不正确"));
    }
    Ok(value.to_string())
}

fn git_bash_from_git(git_path: &str) -> Option<String> {
    let git = PathBuf::from(git_path);
    let root = git.parent()?.parent()?;
    [root.join("bin/bash.exe"), root.join("git-bash.exe")]
        .into_iter()
        .find(|path| path.is_file())
        .map(display_path)
}

fn github_ssh_status(ssh: Option<PathBuf>) -> String {
    let Some(ssh) = ssh else {
        return "未安装 OpenSSH".to_string();
    };
    match hidden_command(ssh)
        .args([
            "-T",
            "-o",
            "BatchMode=yes",
            "-o",
            "ConnectTimeout=5",
            "git@github.com",
        ])
        .output()
    {
        Ok(output) => {
            let text = command_text(&output.stdout, &output.stderr);
            if text
                .to_ascii_lowercase()
                .contains("successfully authenticated")
            {
                "认证成功".to_string()
            } else if text.is_empty() {
                format!("未通过，退出码 {}", output.status.code().unwrap_or(-1))
            } else {
                text.lines().next().unwrap_or("未通过").to_string()
            }
        }
        Err(err) => format!("测试失败：{err}"),
    }
}

fn github_https_status() -> String {
    let client = reqwest::blocking::Client::builder()
        .user_agent("DevEnvManager/2.0")
        .timeout(std::time::Duration::from_secs(8))
        .build();
    match client.and_then(|client| client.get("https://github.com").send()) {
        Ok(response) if response.status().is_success() => {
            format!("正常（HTTP {}）", response.status().as_u16())
        }
        Ok(response) => format!("异常（HTTP {}）", response.status().as_u16()),
        Err(err) => format!("不可访问：{err}"),
    }
}

fn pip_config_value(config: &str, key: &str) -> String {
    config
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once('=')?;
            (name.trim().trim_matches(['\'', '"']) == key)
                .then(|| value.trim().trim_matches(['\'', '"']).to_string())
        })
        .unwrap_or_else(|| "未配置（使用环境或官方默认源）".to_string())
}

fn doctor_report_markdown(report: &DoctorReport) -> String {
    let mut text = String::new();
    text.push_str("# DevEnv Manager 诊断报告\n\n");
    text.push_str(&format!("生成时间：{}\n\n", report.generated_at));
    text.push_str(&format!("环境评分：{} / 100\n\n", report.score));
    text.push_str(&format!("{}\n\n", report.summary));
    text.push_str("## 问题列表\n\n");
    for check in &report.checks {
        text.push_str(&format!(
            "- [{}] {} / {} / {}：{}\n",
            check.category, check.title, check.status, check.severity, check.detail
        ));
    }
    text.push_str("\n## 建议\n\n");
    for suggestion in &report.suggestions {
        text.push_str(&format!(
            "- {}：{}\n",
            suggestion.title, suggestion.description
        ));
    }
    text
}

fn redact_report_text(text: &str) -> String {
    let mut result = text.to_string();
    for key in ["USERPROFILE", "HOME"] {
        if let Ok(value) = env::var(key) {
            if !value.trim().is_empty() {
                result = result.replace(&value, "%USER_HOME%");
            }
        }
    }
    result
        .lines()
        .map(|line| {
            line.split(' ')
                .map(|part| {
                    let lower = part.to_ascii_lowercase();
                    for marker in ["token=", "password=", "secret=", "access_key="] {
                        if lower.starts_with(marker) {
                            return format!("{marker}<redacted>");
                        }
                    }
                    part.to_string()
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn filename_timestamp() -> String {
    format!("{:?}", std::time::SystemTime::now())
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect()
}

fn slug(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn dir_size(path: &Path) -> io::Result<u64> {
    let mut total = 0_u64;
    if !path.exists() {
        return Ok(0);
    }
    for item in fs::read_dir(path)? {
        let item = item?;
        let meta = item.metadata()?;
        if meta.is_dir() {
            total += dir_size(&item.path())?;
        } else {
            total += meta.len();
        }
    }
    Ok(total)
}

fn same_python_package_location(left: &str, right: &str) -> bool {
    let normalize = |value: &str| {
        value
            .split(" from ")
            .nth(1)
            .and_then(|tail| tail.split(" (python").next())
            .map(path_key)
            .unwrap_or_else(|| path_key(value))
    };
    normalize(left) == normalize(right)
}

fn python_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let mut seen = BTreeSet::new();
    for exe in ["python", "python3", "py"] {
        for path in find_all_on_path(exe) {
            if path
                .file_name()
                .and_then(OsStr::to_str)
                .map(|name| name.eq_ignore_ascii_case("py.exe"))
                .unwrap_or(false)
            {
                continue;
            }
            if seen.insert(path_key(&display_path(&path))) {
                candidates.push(path);
            }
        }
    }
    for runtime in discover_runtimes_blocking() {
        if runtime.kind == "Python" && seen.insert(path_key(&runtime.executable)) {
            candidates.push(PathBuf::from(runtime.executable));
        }
    }
    candidates
}

fn dotnet_required_sdk(root: &Path) -> Option<String> {
    let value: Value =
        serde_json::from_str(&fs::read_to_string(root.join("global.json")).ok()?).ok()?;
    value
        .pointer("/sdk/version")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn project_jdk_recommendation(root: &Path, fallback: &str) -> ProjectRuntimeRecommendation {
    let required = detect_project_jdk_requirement(root);
    let environment = inspect_java_environment_blocking().ok();
    let current = environment
        .as_ref()
        .map(|report| report.java_version.clone())
        .unwrap_or_default();
    let current_major = normalize_java_requirement(&current);
    match required {
        Some(required) => ProjectRuntimeRecommendation {
            name: "JDK".to_string(),
            requirement: format!("项目配置建议 JDK {required}"),
            status: if environment
                .as_ref()
                .is_some_and(|report| !report.consistent)
            {
                "JAVA_HOME 与 PATH 不一致，请先修复".to_string()
            } else if current_major.as_deref() == Some(required.as_str()) {
                "版本匹配".to_string()
            } else if current.is_empty() {
                "未发现 JDK".to_string()
            } else {
                format!(
                    "当前版本可能不匹配：{}",
                    current.lines().next().unwrap_or("未知")
                )
            },
        },
        None => runtime_recommendation("JDK", fallback, "java"),
    }
}

fn detect_project_jdk_requirement(root: &Path) -> Option<String> {
    let java_version = root.join(".java-version");
    if let Ok(value) = fs::read_to_string(java_version) {
        if let Some(version) = normalize_java_requirement(&value) {
            return Some(version);
        }
    }
    if let Ok(pom) = fs::read_to_string(root.join("pom.xml")) {
        for tag in [
            "maven.compiler.release",
            "maven.compiler.target",
            "maven.compiler.source",
            "release",
            "target",
            "source",
        ] {
            if let Some(value) = extract_xml_tag(&pom, tag).and_then(normalize_java_requirement) {
                return Some(value);
            }
        }
    }
    for name in ["build.gradle", "build.gradle.kts"] {
        if let Ok(gradle) = fs::read_to_string(root.join(name)) {
            for line in gradle.lines().filter(|line| {
                line.contains("sourceCompatibility")
                    || line.contains("targetCompatibility")
                    || line.contains("languageVersion")
                    || line.contains("jvmToolchain")
            }) {
                if let Some(value) = normalize_java_requirement(line) {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn extract_xml_tag<'a>(text: &'a str, tag: &str) -> Option<&'a str> {
    let start_tag = format!("<{tag}>");
    let end_tag = format!("</{tag}>");
    let start = text.find(&start_tag)? + start_tag.len();
    let end = text[start..].find(&end_tag)? + start;
    Some(text[start..end].trim())
}

fn normalize_java_requirement(value: &str) -> Option<String> {
    let numbers = value
        .split(|ch: char| !ch.is_ascii_digit() && ch != '.')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    for number in numbers {
        if let Some(rest) = number.strip_prefix("1.") {
            if matches!(rest, "8") {
                return Some(rest.to_string());
            }
        }
        if let Ok(version) = number.split('.').next()?.parse::<u32>() {
            if (8..=30).contains(&version) {
                return Some(version.to_string());
            }
        }
    }
    None
}

fn project_signals(root: &Path) -> Vec<String> {
    let mut signals = Vec::new();
    for file in [
        "package.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "package-lock.json",
        "vite.config.js",
        "vite.config.ts",
        "next.config.js",
        "next.config.mjs",
        "requirements.txt",
        "pyproject.toml",
        "poetry.lock",
        "uv.lock",
        ".venv",
        "pom.xml",
        "build.gradle",
        "build.gradle.kts",
        "gradlew",
        "Cargo.toml",
        "src-tauri/tauri.conf.json",
        "go.mod",
        "go.sum",
        "global.json",
        "bin/startup.cmd",
        "conf/application.properties",
    ] {
        if root.join(file).exists() {
            signals.push(file.to_string());
        }
    }
    if let Ok(items) = fs::read_dir(root) {
        for item in items.flatten() {
            let path = item.path();
            if let Some(name) = path.file_name().and_then(OsStr::to_str) {
                if name.ends_with(".csproj") || name.ends_with(".sln") {
                    signals.push(name.to_string());
                }
            }
        }
    }
    signals.sort();
    signals.dedup();
    signals
}

fn detect_package_manager(signals: &[String]) -> String {
    if signals.iter().any(|item| item == "pnpm-lock.yaml") {
        "pnpm".to_string()
    } else if signals.iter().any(|item| item == "yarn.lock") {
        "yarn".to_string()
    } else {
        "npm".to_string()
    }
}

fn runtime_recommendation(
    name: &str,
    requirement: &str,
    executable: &str,
) -> ProjectRuntimeRecommendation {
    ProjectRuntimeRecommendation {
        name: name.to_string(),
        requirement: requirement.to_string(),
        status: if find_on_path(executable).is_some() {
            "已发现".to_string()
        } else {
            "未发现".to_string()
        },
    }
}

fn project_action(
    id: &str,
    title: &str,
    command: &str,
    description: &str,
    safe_to_run: bool,
) -> ProjectAction {
    ProjectAction {
        id: id.to_string(),
        title: title.to_string(),
        command: command.to_string(),
        description: description.to_string(),
        safe_to_run,
    }
}

fn gradle_command(root: &Path, task: &str) -> String {
    if root.join("gradlew.bat").exists() || root.join("gradlew").exists() {
        format!(".\\gradlew {task}")
    } else {
        format!("gradle {task}")
    }
}

fn push_unique(items: &mut Vec<String>, value: &str) {
    if !items.iter().any(|item| item == value) {
        items.push(value.to_string());
    }
}

async fn run_blocking<F, R>(task: F) -> Result<R, String>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|err| format!("后台任务失败：{err}"))
}

fn current_timestamp() -> String {
    // Keep dependencies lean; second precision is enough for audit records.
    format!("{:?}", std::time::SystemTime::now())
}

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
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
        assert_eq!(parse_socket("[::1]:5173"), Some(("::1".to_string(), 5173)));
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
        assert_eq!(
            parse_command_line(r#""C:\Program Files\App\uninstall.exe" /S"#).unwrap(),
            vec![r"C:\Program Files\App\uninstall.exe", "/S"]
        );
        assert!(parse_command_line(r#"node "unterminated"#).is_err());
    }

    #[test]
    fn command_panel_blocks_shells_and_destructive_git() {
        let powershell = assess_command_safety("powershell -Command Get-ChildItem").unwrap();
        assert!(!powershell.allowed);
        let destructive = assess_command_safety("git reset --hard").unwrap();
        assert!(!destructive.allowed);
        let safe = assess_command_safety("node --version").unwrap();
        assert!(safe.allowed);
        assert!(!safe.requires_confirmation || safe.elevated);
    }

    #[test]
    fn package_install_requires_command_panel_confirmation() {
        let assessment = assess_command_safety("npm install vite").unwrap();
        assert!(assessment.allowed);
        assert!(assessment.requires_confirmation);
    }

    #[test]
    fn project_signals_recognize_nacos_layout() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("bin")).unwrap();
        fs::create_dir_all(root.path().join("conf")).unwrap();
        fs::write(root.path().join("bin/startup.cmd"), "@echo off").unwrap();
        fs::write(
            root.path().join("conf/application.properties"),
            "server.port=8848",
        )
        .unwrap();
        let signals = project_signals(root.path());
        assert!(signals.contains(&"bin/startup.cmd".to_string()));
        assert!(signals.contains(&"conf/application.properties".to_string()));
    }

    #[test]
    fn project_signals_detect_mixed_tauri_project() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("package.json"), "{}").unwrap();
        fs::write(temp.path().join("Cargo.toml"), "[package]\nname='demo'\n").unwrap();
        fs::create_dir_all(temp.path().join("src-tauri")).unwrap();
        fs::write(temp.path().join("src-tauri").join("tauri.conf.json"), "{}").unwrap();
        let analysis = analyze_project_blocking(temp.path()).unwrap();
        assert!(analysis.project_types.contains(&"Node.js".to_string()));
        assert!(analysis.project_types.contains(&"Rust".to_string()));
        assert!(analysis.project_types.contains(&"Tauri".to_string()));
        assert!(analysis
            .actions
            .iter()
            .any(|item| item.id == "npm_tauri_dev"));
    }

    #[test]
    fn python_pip_location_compare_uses_site_package_path() {
        let left = r"pip 25.0 from C:\Python312\Lib\site-packages\pip (python 3.12)";
        let right = r"pip 25.0 from C:\Python312\Lib\site-packages\pip (python 3.12)";
        let other = r"pip 24.0 from C:\Python311\Lib\site-packages\pip (python 3.11)";
        assert!(same_python_package_location(left, right));
        assert!(!same_python_package_location(left, other));
    }

    #[test]
    fn report_redaction_masks_home_and_sensitive_pairs() {
        let text = format!(
            "path={} token=abc123 password=hunter2",
            env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users\\demo".to_string())
        );
        let redacted = redact_report_text(&text);
        assert!(!redacted.contains("abc123"));
        assert!(!redacted.contains("hunter2"));
    }

    #[test]
    fn download_url_allowlist_rejects_unknown_hosts() {
        assert!(validate_download_url("https://nodejs.org/dist/index.json").is_ok());
        assert!(validate_download_url("https://example.com/file.zip").is_err());
    }

    #[test]
    fn tool_registry_has_unique_ids_and_core_ecosystems() {
        let tools = tool_registry();
        let ids = tools.iter().map(|item| item.id).collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), tools.len());
        assert!(ids.contains("git"));
        assert!(ids.contains("pnpm"));
        assert!(ids.contains("python-tools"));
    }

    #[test]
    fn pip_config_parser_reads_quoted_index_url() {
        let config = "global.index-url='https://pypi.tuna.tsinghua.edu.cn/simple'\n";
        assert_eq!(
            pip_config_value(config, "global.index-url"),
            "https://pypi.tuna.tsinghua.edu.cn/simple"
        );
    }

    #[test]
    fn setting_validation_rejects_empty_and_control_characters() {
        assert!(validate_setting(Some(""), "测试值").is_err());
        assert!(validate_setting(Some("line\nbreak"), "测试值").is_err());
        assert_eq!(
            validate_setting(Some("valid value"), "测试值").unwrap(),
            "valid value"
        );
    }

    #[test]
    fn go_release_parser_selects_latest_windows_archive() {
        let index = json!([
            {
                "version": "go1.25.2",
                "stable": true,
                "files": [{"filename": "go1.25.2.windows-amd64.zip", "os": "windows", "arch": "amd64", "kind": "archive", "sha256": "old"}]
            },
            {
                "version": "go1.25.10",
                "stable": true,
                "files": [{"filename": "go1.25.10.windows-amd64.zip", "os": "windows", "arch": "amd64", "kind": "archive", "sha256": "new"}]
            },
            {
                "version": "go1.26rc1",
                "stable": false,
                "files": [{"filename": "go1.26rc1.windows-amd64.zip", "os": "windows", "arch": "amd64", "kind": "archive", "sha256": "rc"}]
            }
        ]);
        let release = parse_go_release_index(&index, "1.25").unwrap();
        assert_eq!(release.tag, "go1.25.10");
        assert_eq!(release.sha256.as_deref(), Some("new"));
        assert!(release.url.ends_with("go1.25.10.windows-amd64.zip"));
    }

    #[test]
    fn mirror_templates_include_only_selected_source() {
        let maven = maven_settings_content(Some((
            "aliyun",
            "https://maven.aliyun.com/repository/public",
        )));
        assert!(maven.contains("<id>aliyun</id>"));
        assert!(maven.contains("<mirrorOf>*</mirrorOf>"));
        let gradle = gradle_init_content(Some("https://maven.aliyun.com/repository/public"));
        assert!(gradle.contains("mavenCentral()"));
        assert!(gradle.contains("maven.aliyun.com"));
    }

    #[test]
    fn old_installed_data_deserializes_without_go_fields() {
        let installed: InstalledData = serde_json::from_value(json!({
            "jdks": [], "pythons": [], "nodes": [], "mavens": [], "gradles": [], "current": {}
        }))
        .unwrap();
        assert!(installed.gos.is_empty());
        assert!(installed.current.go.is_none());
    }

    #[test]
    fn csv_parser_preserves_service_list_column() {
        let values = parse_csv_line(r#""mysqld.exe","1234","MySQL80, Helper""#);
        assert_eq!(values, vec!["mysqld.exe", "1234", "MySQL80, Helper"]);
    }

    #[test]
    fn database_service_guard_requires_matching_port_and_name() {
        assert!(service_matches_database(3306, "MySQL80"));
        assert!(service_matches_database(5432, "postgresql-x64-17"));
        assert!(!service_matches_database(3306, "Dhcp"));
        assert!(!service_matches_database(445, "MySQL80"));
    }

    #[test]
    fn project_jdk_requirement_reads_java_version_and_gradle() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join(".java-version"), "17\n").unwrap();
        assert_eq!(
            detect_project_jdk_requirement(temp.path()).as_deref(),
            Some("17")
        );
        fs::remove_file(temp.path().join(".java-version")).unwrap();
        fs::write(
            temp.path().join("build.gradle.kts"),
            "java { toolchain { languageVersion.set(JavaLanguageVersion.of(21)) } }",
        )
        .unwrap();
        assert_eq!(
            detect_project_jdk_requirement(temp.path()).as_deref(),
            Some("21")
        );
    }

    #[test]
    fn command_stream_decodes_utf16le() {
        let bytes = "WSL 正常"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        assert_eq!(decode_command_stream(&bytes), "WSL 正常");
    }

    #[test]
    fn cleanup_architecture_is_scan_only() {
        let architecture = cleanup::architecture();
        assert_eq!(architecture.status, "scan-only-phase-1");
        assert!(!architecture
            .categories
            .iter()
            .any(|category| category.cleanup_enabled));
        assert!(!architecture
            .categories
            .iter()
            .any(|category| category.id == "user-space"));
        assert!(architecture
            .categories
            .iter()
            .any(|category| category.id == "wps-cache"));
        let recycle_bin = architecture
            .categories
            .iter()
            .find(|category| category.id == "recycle-bin")
            .unwrap();
        assert!(recycle_bin.scan_only);
        assert!(!recycle_bin.cleanup_enabled);
    }
    #[test]
    fn wsl_distribution_parser_handles_default_marker() {
        let items = parse_wsl_distributions(
            "  NAME      STATE     VERSION\n* Ubuntu   Running   2\n  Debian   Stopped   2\n",
        );
        assert_eq!(items.len(), 2);
        assert!(items[0].is_default);
        assert_eq!(items[0].name, "Ubuntu");
        assert_eq!(items[1].state, "Stopped");
    }

    #[test]
    fn update_checksum_requires_exact_sha256() {
        assert!(validate_update_checksum(&"a".repeat(64)).is_ok());
        assert!(validate_update_checksum(&"g".repeat(64)).is_err());
        assert!(validate_update_checksum("abc").is_err());
    }

    #[test]
    fn project_port_update_creates_backup_and_verifies() {
        let temp = tempfile::tempdir().unwrap();
        let resources = temp.path().join("src").join("main").join("resources");
        fs::create_dir_all(&resources).unwrap();
        let file = resources.join("application.properties");
        fs::write(&file, "spring.application.name=demo\nserver.port=8080\n").unwrap();
        let configs = inspect_project_port_configs_blocking(temp.path()).unwrap();
        let config = configs
            .iter()
            .find(|item| item.current_port == 8080)
            .unwrap();
        update_project_port_blocking(temp.path(), &config.id, 9090).unwrap();
        assert!(fs::read_to_string(&file)
            .unwrap()
            .contains("server.port=9090"));
        let backups = fs::read_dir(&resources)
            .unwrap()
            .flatten()
            .filter(|item| {
                item.file_name()
                    .to_string_lossy()
                    .contains(".devenv-backup-")
            })
            .count();
        assert_eq!(backups, 1);
    }

    #[test]
    fn portable_runtime_root_requires_matching_directory_name() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("python-3.12");
        fs::create_dir_all(&root).unwrap();
        let executable = root.join("python.exe");
        fs::write(&executable, b"test").unwrap();
        assert_eq!(portable_runtime_root(&executable, "Python").unwrap(), root);
        let unrelated = temp.path().join("misc");
        fs::create_dir_all(&unrelated).unwrap();
        let unrelated_exe = unrelated.join("python.exe");
        fs::write(&unrelated_exe, b"test").unwrap();
        assert!(portable_runtime_root(&unrelated_exe, "Python").is_err());
    }

    #[test]
    fn service_executable_path_supports_quoted_paths() {
        let temp = tempfile::tempdir().unwrap();
        let folder = temp.path().join("Program Files").join("Database");
        fs::create_dir_all(&folder).unwrap();
        let executable = folder.join("server.exe");
        fs::write(&executable, b"test").unwrap();
        let command = format!("\"{}\" --service", executable.display());
        assert_eq!(service_executable_path(&command).unwrap(), executable);
    }
}
