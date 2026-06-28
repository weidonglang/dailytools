mod cleanup;
mod diagnostics;
mod env_core;
mod mysql_repair;
mod safety;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Mutex, OnceLock,
};
use std::time::Instant;
use tauri::Emitter;
use tempfile::Builder as TempBuilder;
use zip::ZipArchive;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use winreg::{enums::*, RegKey};

const APP_NAME: &str = "DevEnvManager";
const SAFETY_DISCLAIMER_VERSION: u32 = 1;
static SAVE_JSON_COUNTER: AtomicU64 = AtomicU64::new(0);
static MAINTENANCE_SCAN_CANCELLED: AtomicBool = AtomicBool::new(false);
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
const ALLOWED_DOWNLOAD_HOSTS: [&str; 24] = [
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
    "dl.google.com",
    "raw.githubusercontent.com",
    "api.github.com",
    "api.azul.com",
    "cdn.azul.com",
    "api.bell-sw.com",
    "download.bell-sw.com",
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
    #[serde(default)]
    safety_disclaimer_accepted: bool,
    #[serde(default)]
    safety_disclaimer_version: u32,
    #[serde(default)]
    safety_disclaimer_accepted_at: Option<String>,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ConfirmationToken {
    token: String,
    action_id: String,
    plan_id: String,
    risk_level: String,
    plan_fingerprint: String,
    backup_receipt: Option<String>,
    triple_confirmed: bool,
    created_at: u64,
    expires_at: u64,
    used: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmationTokenView {
    token: String,
    action_id: String,
    plan_id: String,
    risk_level: String,
    expires_at: u64,
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

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct EnvironmentValueChange {
    name: String,
    current: String,
    proposed: String,
    impact: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct EnvironmentConfigPreview {
    preview_id: String,
    created_at: String,
    changes: Vec<EnvironmentValueChange>,
    path_added: Vec<String>,
    path_removed: Vec<String>,
    warnings: Vec<String>,
    backup_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EnvironmentBackupInfo {
    file_name: String,
    created_at: String,
    devenv_home: String,
    java_home: String,
    path_entries: usize,
}

#[derive(Clone)]
struct PendingEnvironmentConfig {
    preview: EnvironmentConfigPreview,
    java_home: Option<String>,
    path: String,
    baseline_fingerprint: String,
}

static ENVIRONMENT_PREVIEWS: OnceLock<Mutex<HashMap<String, PendingEnvironmentConfig>>> =
    OnceLock::new();

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
    identity: String,
    confidence: u8,
    evidence_count: usize,
    conflict_count: usize,
    risk_level: String,
    recommendation: String,
    evidence: Vec<String>,
    conflict_evidence: Vec<String>,
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
    identity: String,
    risk_level: String,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ArchivePlanItem {
    id: String,
    path: String,
    size: u64,
    source: String,
    added_at: String,
    suggestion: String,
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
    launcher_path: String,
    launcher_output: String,
    first_python_on_path: String,
    first_pip_on_path: String,
    python_m_pip_available: bool,
    managed_python_available: bool,
    discovered_pythons: Vec<PythonEntry>,
    discovered_pips: Vec<PythonEntry>,
    user_path_entry_count: usize,
    current_terminal_matches_user_path: bool,
    store_alias_risk: bool,
    repair_blockers: Vec<String>,
    recovery_actions: Vec<String>,
    diagnostic_report: String,
    risks: Vec<String>,
    recommendations: Vec<String>,
    pip_repair_command: String,
    alias_settings_command: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PythonRepairPlan {
    plan_id: String,
    created_at: String,
    python_path: String,
    actions: Vec<String>,
    commands: Vec<String>,
    path_added: Vec<String>,
    warnings: Vec<String>,
    backup_name: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ValidationCheck {
    id: String,
    title: String,
    success: bool,
    required: bool,
    detail: String,
    stage: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PythonIntegrityReport {
    python_path: String,
    python_home: String,
    managed: bool,
    fully_usable: bool,
    status: String,
    checks: Vec<ValidationCheck>,
    risks: Vec<String>,
    suggestions: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct RuntimeStrongStatus {
    kind: String,
    version: String,
    path: String,
    registered: bool,
    current: bool,
    environment_effective: bool,
    status: String,
    checks: Vec<ValidationCheck>,
    failure_stage: Option<String>,
    report: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeStrongVerificationReport {
    generated_at: String,
    items: Vec<RuntimeStrongStatus>,
    summary: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct IdeaProjectReport {
    root: String,
    detected: bool,
    read_files: Vec<String>,
    project_sdk: String,
    language_level: String,
    module_sdks: Vec<String>,
    module_count: usize,
    compiler_target: String,
    maven_importer_jdk: String,
    gradle_jvm: String,
    output_dir: String,
    current_java_home: String,
    current_java_version: String,
    jdk_match: String,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct JavaConsumerReport {
    consumer: String,
    root: String,
    startup_exists: bool,
    java_home_raw: Option<String>,
    java_home_expanded: Option<String>,
    java_exists: bool,
    javac_exists: bool,
    path_java: Option<String>,
    indirect_java_home_risk: bool,
    process_user_env_differs: bool,
    usable: bool,
    explanation: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DirectoryValidationResult {
    path: String,
    exists: bool,
    is_directory: bool,
    recognized_project: bool,
    signals: Vec<String>,
    message: String,
}

#[derive(Clone)]
struct PendingPythonRepair {
    public: PythonRepairPlan,
    baseline_fingerprint: String,
    proposed_path: String,
    repair_pip: bool,
    repair_path: bool,
}

static PYTHON_REPAIR_PREVIEWS: OnceLock<Mutex<HashMap<String, PendingPythonRepair>>> =
    OnceLock::new();

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

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProjectConfigFileDraft {
    relative_path: String,
    content: String,
    existed: bool,
    enabled: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectConfigPreview {
    project_path: String,
    detected_types: Vec<String>,
    files: Vec<ProjectConfigFileDraft>,
    current: CurrentVersions,
    warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProjectConfigApplyRequest {
    project_path: String,
    files: Vec<ProjectConfigFileDraft>,
    switches: CurrentVersions,
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
    chsrc: ToolState,
    chsrc_recovery: ChsrcRecovery,
    generated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChsrcRecovery {
    missing: bool,
    explanation: Vec<String>,
    scoop_command: String,
    winget_command: String,
    official_url: String,
    fallback_features: Vec<String>,
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
    #[serde(alias = "download_url")]
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

#[derive(Debug, Clone, Copy)]
struct RuntimeMeta {
    kind: &'static str,
    collection: &'static str,
    link_name: &'static str,
    exe_key: &'static str,
}

static CONFIRMATION_TOKENS: OnceLock<Mutex<HashMap<String, ConfirmationToken>>> = OnceLock::new();

fn confirmation_tokens() -> &'static Mutex<HashMap<String, ConfirmationToken>> {
    CONFIRMATION_TOKENS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[tauri::command]
fn create_confirmation_token(
    action_id: String,
    plan_id: String,
    risk_level: String,
    plan_fingerprint: String,
    triple_confirmed: bool,
    backup_receipt: Option<String>,
) -> Result<ConfirmationTokenView, String> {
    let action_id = validate_confirmation_field(action_id, "action_id")?;
    let plan_id = validate_confirmation_field(plan_id, "plan_id")?;
    let risk_level = validate_confirmation_field(risk_level, "risk_level")?;
    let plan_fingerprint = validate_confirmation_field(plan_fingerprint, "plan_fingerprint")?;
    if risk_level == "critical" && !triple_confirmed {
        return Err("极高风险操作必须完成三次确认".to_string());
    }
    let now = unix_timestamp();
    let expires_at = now + 5 * 60;
    let mut hasher = Sha256::new();
    hasher.update(action_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(plan_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(plan_fingerprint.as_bytes());
    hasher.update(b"\0");
    hasher.update(now.to_le_bytes());
    hasher.update(std::process::id().to_le_bytes());
    let token = format!("{:x}", hasher.finalize());
    let record = ConfirmationToken {
        token: token.clone(),
        action_id: action_id.clone(),
        plan_id: plan_id.clone(),
        risk_level: risk_level.clone(),
        plan_fingerprint,
        backup_receipt,
        triple_confirmed,
        created_at: now,
        expires_at,
        used: false,
    };
    let mut store = confirmation_tokens()
        .lock()
        .map_err(|_| "确认 token 存储不可用".to_string())?;
    store.retain(|_, item| !item.used && item.expires_at >= now);
    store.insert(token.clone(), record);
    Ok(ConfirmationTokenView {
        token,
        action_id,
        plan_id,
        risk_level,
        expires_at,
    })
}

fn validate_confirmation_field(value: String, label: &str) -> Result<String, String> {
    let value = value.trim().to_string();
    if value.is_empty() || value.len() > 512 || value.chars().any(char::is_control) {
        return Err(format!("{label} 无效"));
    }
    Ok(value)
}

fn require_confirmation_token(
    token: Option<String>,
    action_id: &str,
    plan_id: &str,
    risk_level: &str,
    plan_fingerprint: &str,
    backup_required: bool,
) -> Result<(), String> {
    let token = token
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "缺少后端 confirmation token，已拒绝执行高风险操作".to_string())?;
    let now = unix_timestamp();
    let mut store = confirmation_tokens()
        .lock()
        .map_err(|_| "确认 token 存储不可用".to_string())?;
    let record = store
        .get_mut(&token)
        .ok_or_else(|| "confirmation token 不存在或已失效".to_string())?;
    if record.used {
        return Err("confirmation token 已使用，已拒绝重复执行".to_string());
    }
    if record.expires_at < now {
        record.used = true;
        return Err("confirmation token 已过期，请重新确认".to_string());
    }
    if record.action_id != action_id
        || record.plan_id != plan_id
        || record.risk_level != risk_level
        || record.plan_fingerprint != plan_fingerprint
    {
        return Err("confirmation token 与当前计划不匹配".to_string());
    }
    if risk_level == "critical" && !record.triple_confirmed {
        return Err("极高风险操作未完成三次确认".to_string());
    }
    if backup_required
        && record
            .backup_receipt
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
    {
        return Err("该操作需要有效备份回执".to_string());
    }
    record.used = true;
    Ok(())
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
async fn create_cleanup_plan(
    selected_item_ids: Vec<String>,
) -> Result<cleanup::CleanupPlan, String> {
    run_blocking(move || {
        let paths = load_paths()?;
        cleanup::create_cleanup_plan(&paths.root, selected_item_ids)
    })
    .await?
}

#[tauri::command]
async fn clean_selected_targets(
    plan: cleanup::CleanupPlan,
) -> Result<cleanup::CleanupResult, String> {
    run_blocking(move || {
        let paths = load_paths()?;
        cleanup::clean_selected_targets(&paths.root, plan)
    })
    .await?
}

#[tauri::command]
async fn clean_managed_download_cache() -> Result<cleanup::CleanupResult, String> {
    run_blocking(move || {
        let paths = load_paths()?;
        Ok(cleanup::clean_managed_download_cache(&paths.root))
    })
    .await?
}

#[tauri::command]
async fn clean_dev_cache(tool: String) -> Result<OperationResult, String> {
    run_blocking(move || {
        let paths = load_paths()?;
        let message = cleanup::clean_dev_cache(&tool, &paths.root)?;
        Ok(OperationResult {
            success: true,
            message,
        })
    })
    .await?
}

#[tauri::command]
fn export_cleanup_report(format: String) -> Result<String, String> {
    let content = cleanup::export_cleanup_report(&format)?;
    let paths = load_paths()?;
    let reports = paths.root.join("reports");
    fs::create_dir_all(&reports).map_err(|error| format!("创建报告目录失败：{error}"))?;
    let extension = match format.trim().to_ascii_lowercase().as_str() {
        "markdown" | "md" => "md",
        "json" => "json",
        _ => return Err("仅支持导出 Markdown 或 JSON".to_string()),
    };
    let target = reports.join(format!(
        "cleanup-report-{}.{}",
        filename_timestamp(),
        extension
    ));
    fs::write(&target, content).map_err(|error| format!("写入清理报告失败：{error}"))?;
    Ok(display_path(target))
}

#[tauri::command]
async fn scan_large_files(
    app: tauri::AppHandle,
    root: String,
    min_size_mb: u64,
    limit: usize,
) -> Result<Vec<cleanup::LargeFileItem>, String> {
    MAINTENANCE_SCAN_CANCELLED.store(false, Ordering::SeqCst);
    let started = Instant::now();
    emit_task_progress(
        &app,
        "大文件扫描",
        2,
        "正在预估扫描范围，上限 250000 个条目",
    );
    let progress_app = app.clone();
    let result = run_blocking(move || {
        cleanup::scan_large_files_with_progress(
            root,
            min_size_mb,
            limit,
            move |update| {
                let percent =
                    8 + ((update.visited_entries.min(50_000) as u64 * 82 / 50_000) as u8).min(82);
                let truncated = if update.truncated {
                    "；已达到扫描上限"
                } else {
                    ""
                };
                emit_task_progress(
                    &progress_app,
                    "大文件扫描",
                    percent.min(95),
                    &format!(
                        "已访问 {} 项，候选 {} 个{}",
                        update.visited_entries, update.candidate_count, truncated
                    ),
                );
            },
            || MAINTENANCE_SCAN_CANCELLED.load(Ordering::SeqCst),
        )
    })
    .await?;
    match result {
        Ok(items) => {
            MAINTENANCE_SCAN_CANCELLED.store(false, Ordering::SeqCst);
            emit_task_progress(
                &app,
                "大文件扫描",
                100,
                &format!(
                    "完成：{} 个候选，耗时 {} 秒",
                    items.len(),
                    started.elapsed().as_secs()
                ),
            );
            Ok(items)
        }
        Err(err) => {
            let percent = if err.contains("取消") { 100 } else { 0 };
            emit_task_progress(&app, "大文件扫描", percent, &err);
            Err(err)
        }
    }
}

#[tauri::command]
async fn scan_duplicate_large_files(
    app: tauri::AppHandle,
    root: String,
    min_size_mb: u64,
) -> Result<Vec<cleanup::DuplicateGroup>, String> {
    MAINTENANCE_SCAN_CANCELLED.store(false, Ordering::SeqCst);
    let started = Instant::now();
    emit_task_progress(&app, "重复文件扫描", 2, "正在按大小收集候选文件");
    let progress_app = app.clone();
    let result = run_blocking(move || {
        cleanup::scan_duplicate_large_files_with_progress(
            root,
            min_size_mb,
            move |update| {
                let (base, span, label) = match update.stage {
                    "collect" => (5_u8, 30_u8, "收集候选"),
                    "quick_hash" => (36_u8, 29_u8, "quick hash"),
                    "full_hash" => (66_u8, 28_u8, "full hash"),
                    _ => (95_u8, 0_u8, "整理结果"),
                };
                let work_units = match update.stage {
                    "collect" => update.visited_entries.min(50_000),
                    "quick_hash" => update.quick_hashed.min(update.candidate_count.max(1)),
                    "full_hash" => update.full_hashed.min(update.candidate_count.max(1)),
                    _ => update.candidate_count,
                };
                let total = if update.stage == "collect" {
                    50_000
                } else {
                    update.candidate_count.max(1)
                };
                let percent = base + ((work_units as u64 * span as u64 / total as u64) as u8);
                let truncated = if update.truncated {
                    "；已达到扫描上限"
                } else {
                    ""
                };
                emit_task_progress(
                    &progress_app,
                    "重复文件扫描",
                    percent.min(96),
                    &format!(
                        "{}：访问 {} 项，候选 {} 个，quick {}，full {}{}",
                        label,
                        update.visited_entries,
                        update.candidate_count,
                        update.quick_hashed,
                        update.full_hashed,
                        truncated
                    ),
                );
            },
            || MAINTENANCE_SCAN_CANCELLED.load(Ordering::SeqCst),
        )
    })
    .await?;
    match result {
        Ok(groups) => {
            MAINTENANCE_SCAN_CANCELLED.store(false, Ordering::SeqCst);
            emit_task_progress(
                &app,
                "重复文件扫描",
                100,
                &format!(
                    "完成：{} 组重复文件，耗时 {} 秒",
                    groups.len(),
                    started.elapsed().as_secs()
                ),
            );
            Ok(groups)
        }
        Err(err) => {
            let percent = if err.contains("取消") { 100 } else { 0 };
            emit_task_progress(&app, "重复文件扫描", percent, &err);
            Err(err)
        }
    }
}

#[tauri::command]
fn cancel_maintenance_scan() -> Result<OperationResult, String> {
    MAINTENANCE_SCAN_CANCELLED.store(true, Ordering::SeqCst);
    Ok(OperationResult {
        success: true,
        message: "已请求取消当前扫描；正在等待后台任务安全退出。".to_string(),
    })
}

#[tauri::command]
async fn inspect_downloads() -> Result<cleanup::FolderUsageReport, String> {
    run_blocking(|| Ok(cleanup::inspect_downloads())).await?
}

#[tauri::command]
async fn inspect_desktop() -> Result<cleanup::FolderUsageReport, String> {
    run_blocking(|| Ok(cleanup::inspect_desktop())).await?
}

#[tauri::command]
async fn inspect_app_usage() -> Result<cleanup::AppUsageReport, String> {
    run_blocking(|| Ok(cleanup::inspect_app_usage())).await?
}

#[tauri::command]
async fn inspect_installed_software_usage() -> Result<Vec<cleanup::InstalledSoftwareUsage>, String>
{
    run_blocking(|| Ok(cleanup::inspect_installed_software_usage())).await?
}

#[tauri::command]
async fn inspect_env_reliability() -> Result<env_core::EnvReliabilitySnapshot, String> {
    run_blocking(move || {
        let paths = load_paths()?;
        Ok(env_core::inspect_env_reliability(&paths.root))
    })
    .await?
}

#[tauri::command]
async fn create_env_repair_plan(
    target: String,
    options: env_core::EnvRepairOptions,
) -> Result<env_core::EnvRepairPlan, String> {
    run_blocking(move || {
        let paths = load_paths()?;
        env_core::create_env_repair_plan(&paths.root, target, options)
    })
    .await?
}

#[tauri::command]
async fn apply_env_repair_plan(
    plan: env_core::EnvRepairPlan,
) -> Result<env_core::EnvRepairResult, String> {
    run_blocking(move || {
        let paths = load_paths()?;
        Ok(env_core::apply_env_repair_plan(&paths.root, plan))
    })
    .await?
}

#[tauri::command]
async fn verify_env_after_apply(
    plan_id: String,
) -> Result<env_core::EnvVerificationReport, String> {
    run_blocking(move || {
        let paths = load_paths()?;
        Ok(env_core::verify_env_after_apply(&paths.root, plan_id))
    })
    .await?
}

#[tauri::command]
async fn rollback_env_repair(backup_name: String) -> Result<env_core::EnvRepairResult, String> {
    run_blocking(move || env_core::restore_env_backup(backup_name)).await?
}

#[tauri::command]
async fn export_env_reliability_report(format: String) -> Result<String, String> {
    run_blocking(move || {
        let paths = load_paths()?;
        env_core::export_env_reliability_report(&paths.root, format)
    })
    .await?
}

#[tauri::command]
async fn create_java_stabilize_plan(jdk_path: String) -> Result<env_core::EnvRepairPlan, String> {
    run_blocking(move || {
        let paths = load_paths()?;
        env_core::create_java_stabilize_plan(&paths.root, jdk_path)
    })
    .await?
}

#[tauri::command]
async fn apply_java_stabilize_plan(
    plan: env_core::EnvRepairPlan,
) -> Result<env_core::EnvRepairResult, String> {
    apply_env_repair_plan(plan).await
}

#[tauri::command]
async fn verify_java_toolchain() -> Result<env_core::JavaVerificationReport, String> {
    run_blocking(move || {
        let paths = load_paths()?;
        Ok(env_core::verify_java_toolchain(&paths.root))
    })
    .await?
}

#[tauri::command]
async fn verify_nacos_java_environment(
    nacos_root: String,
) -> Result<env_core::NacosEnvReport, String> {
    run_blocking(move || {
        let paths = load_paths()?;
        Ok(env_core::verify_nacos_java_environment(
            &paths.root,
            nacos_root,
        ))
    })
    .await?
}

#[tauri::command]
async fn verify_maven_gradle_with_current_jdk() -> Result<env_core::MavenGradleReliability, String>
{
    run_blocking(move || {
        let paths = load_paths()?;
        let user = user_environment().unwrap_or_default();
        let java_home = user
            .get("JAVA_HOME")
            .map(|value| expand_environment_path(value, &paths));
        Ok(env_core::maven_gradle::inspect_maven_gradle_reliability(
            &paths.root,
            &user,
            java_home.as_deref(),
        ))
    })
    .await?
}

#[tauri::command]
async fn repair_maven_gradle_registration(
    kind: String,
    path: String,
) -> Result<OperationResult, String> {
    run_blocking(move || {
        let message = env_core::repair_maven_gradle_registration(kind, path)?;
        Ok(OperationResult {
            success: true,
            message,
        })
    })
    .await?
}

#[tauri::command]
async fn list_env_backups() -> Result<Vec<env_core::EnvBackupRecord>, String> {
    run_blocking(|| Ok(env_core::list_env_backups())).await?
}

#[tauri::command]
async fn inspect_env_backup(backup_name: String) -> Result<env_core::EnvBackupDiff, String> {
    run_blocking(move || env_core::inspect_env_backup(backup_name)).await?
}

#[tauri::command]
async fn restore_env_backup(backup_name: String) -> Result<env_core::EnvRepairResult, String> {
    run_blocking(move || env_core::restore_env_backup(backup_name)).await?
}

#[tauri::command]
fn safety_disclaimer() -> String {
    safety::disclaimer_text()
}

#[tauri::command]
fn feature_risk_registry() -> Vec<safety::FeatureRiskInfo> {
    safety::feature_risk_registry()
}

#[tauri::command]
fn get_feature_risk(feature_id: String) -> Option<safety::FeatureRiskInfo> {
    safety::get_feature_risk(feature_id)
}

#[tauri::command]
fn accept_safety_disclaimer() -> Result<OperationResult, String> {
    let mut settings = load_settings()?;
    settings.safety_disclaimer_accepted = true;
    settings.safety_disclaimer_version = SAFETY_DISCLAIMER_VERSION;
    settings.safety_disclaimer_accepted_at = Some(current_timestamp());
    save_json(&settings_file(), &settings)?;
    Ok(OperationResult {
        success: true,
        message: "已记录安全说明已读状态；不会上传任何数据。".to_string(),
    })
}

#[tauri::command]
fn reset_ui_config() -> Result<OperationResult, String> {
    let mut settings = load_settings()?;
    settings.theme = "system".to_string();
    settings.last_page = "home".to_string();
    settings.port_process_exclusions.clear();
    save_json(&settings_file(), &settings)?;
    Ok(OperationResult {
        success: true,
        message: "已重置 UI 状态；根目录、版本记录和安全声明状态已保留。".to_string(),
    })
}

#[tauri::command]
fn open_app_config_dir() -> Result<OperationResult, String> {
    let dir = app_config_dir();
    fs::create_dir_all(&dir).map_err(|err| format!("创建配置目录失败：{err}"))?;
    hidden_command("explorer.exe")
        .arg(&dir)
        .spawn()
        .map_err(|error| format!("打开配置目录失败：{error}"))?;
    Ok(OperationResult {
        success: true,
        message: format!("已打开配置目录：{}", display_path(dir)),
    })
}

#[tauri::command]
async fn create_move_plan(
    source: String,
    target_drive: String,
    mode: String,
) -> Result<cleanup::MovePlan, String> {
    run_blocking(move || cleanup::create_move_plan(source, target_drive, mode)).await?
}

#[tauri::command]
async fn execute_move_plan(plan: cleanup::MovePlan) -> Result<cleanup::MoveResult, String> {
    run_blocking(move || {
        let paths = load_paths()?;
        Ok(cleanup::execute_move_plan(&paths.root, plan))
    })
    .await?
}

#[tauri::command]
async fn list_rollback_records() -> Result<Vec<cleanup::RollbackRecord>, String> {
    run_blocking(move || {
        let paths = load_paths()?;
        Ok(cleanup::list_rollback_records(&paths.root))
    })
    .await?
}

#[tauri::command]
async fn rollback_move(rollback_id: String) -> Result<OperationResult, String> {
    run_blocking(move || {
        let paths = load_paths()?;
        let message = cleanup::rollback_move(&paths.root, rollback_id)?;
        Ok(OperationResult {
            success: true,
            message,
        })
    })
    .await?
}

#[tauri::command]
async fn create_junction_bridge(
    source: String,
    target: String,
) -> Result<cleanup::MoveResult, String> {
    run_blocking(move || {
        let paths = load_paths()?;
        cleanup::create_junction_bridge(&paths.root, source, target)
    })
    .await?
}

#[tauri::command]
async fn create_desktop_archive_plan(target_drive: String) -> Result<cleanup::MovePlan, String> {
    run_blocking(move || cleanup::create_desktop_archive_plan(target_drive)).await?
}

#[tauri::command]
async fn execute_desktop_archive_plan(
    plan: cleanup::MovePlan,
) -> Result<cleanup::MoveResult, String> {
    run_blocking(move || {
        let paths = load_paths()?;
        Ok(cleanup::execute_desktop_archive_plan(&paths.root, plan))
    })
    .await?
}

#[tauri::command]
async fn create_downloads_archive_plan(target_drive: String) -> Result<cleanup::MovePlan, String> {
    run_blocking(move || cleanup::create_downloads_archive_plan(target_drive)).await?
}

#[tauri::command]
async fn execute_downloads_archive_plan(
    plan: cleanup::MovePlan,
) -> Result<cleanup::MoveResult, String> {
    run_blocking(move || {
        let paths = load_paths()?;
        Ok(cleanup::execute_downloads_archive_plan(&paths.root, plan))
    })
    .await?
}

#[tauri::command]
async fn inspect_partition_layout() -> Result<cleanup::PartitionLayoutReport, String> {
    run_blocking(cleanup::inspect_partition_layout).await?
}

#[tauri::command]
async fn create_c_drive_expansion_plan() -> Result<cleanup::ExpansionPlan, String> {
    run_blocking(cleanup::create_c_drive_expansion_plan).await?
}

#[tauri::command]
async fn execute_c_drive_expansion(
    plan: cleanup::ExpansionPlan,
) -> Result<cleanup::ExpansionResult, String> {
    run_blocking(move || Ok(cleanup::execute_c_drive_expansion(plan))).await?
}

#[tauri::command]
fn open_analysis_path(path: String) -> Result<OperationResult, String> {
    let path = PathBuf::from(path.trim());
    if path.is_file() {
        hidden_command("explorer.exe")
            .arg("/select,")
            .arg(&path)
            .spawn()
            .map_err(|error| format!("打开并选中文件失败：{error}"))?;
        return Ok(OperationResult {
            success: true,
            message: format!("已打开并选中：{}", display_path(path)),
        });
    }
    if !path.exists() {
        return Err("路径不存在，可能已移动或删除；请重新扫描后再打开。".to_string());
    }
    let target = if path.is_dir() {
        path
    } else {
        path.parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| "无法识别所在目录".to_string())?
    };
    if !target.is_dir() {
        return Err("目录不存在，可能已移动或删除；请重新扫描后再打开。".to_string());
    }
    hidden_command("explorer.exe")
        .arg(&target)
        .spawn()
        .map_err(|error| format!("打开目录失败：{error}"))?;
    Ok(OperationResult {
        success: true,
        message: format!("已打开：{}", display_path(target)),
    })
}

#[tauri::command]
fn open_apps_features() -> Result<OperationResult, String> {
    hidden_command("explorer.exe")
        .arg("ms-settings:appsfeatures")
        .spawn()
        .map_err(|error| format!("打开 Windows 已安装的应用失败：{error}"))?;
    Ok(OperationResult {
        success: true,
        message: "已打开 Windows 已安装的应用；请通过系统卸载入口操作".to_string(),
    })
}

#[tauri::command]
fn open_python_alias_settings() -> Result<OperationResult, String> {
    hidden_command("explorer.exe")
        .arg("ms-settings:appsfeatures-app")
        .spawn()
        .map_err(|error| format!("打开应用执行别名设置失败：{error}"))?;
    Ok(OperationResult {
        success: true,
        message:
            "已打开 Windows 应用执行别名设置；请手动关闭 python.exe / python3.exe Store Alias。"
                .to_string(),
    })
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
    validate_update_manifest(&manifest)?;
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

fn validate_update_manifest(manifest: &UpdateManifest) -> Result<(), String> {
    if manifest.version.trim().is_empty() {
        return Err("更新清单缺少 version".to_string());
    }
    if manifest.download_url.trim().is_empty() {
        return Err("更新清单缺少 downloadUrl".to_string());
    }
    validate_download_url(&manifest.download_url)?;
    validate_update_checksum(&manifest.sha256)?;
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

fn environment_preview_store() -> &'static Mutex<HashMap<String, PendingEnvironmentConfig>> {
    ENVIRONMENT_PREVIEWS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn split_path_entries(value: &str) -> Vec<String> {
    value
        .split(';')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_string)
        .collect()
}

fn environment_fingerprint(environment: &HashMap<String, String>) -> String {
    let mut hasher = Sha256::new();
    for name in ["DEVENV_HOME", "JAVA_HOME"] {
        hasher.update(name.as_bytes());
        hasher.update([0]);
        hasher.update(
            environment
                .get(name)
                .map(String::as_str)
                .unwrap_or("")
                .as_bytes(),
        );
        hasher.update([0]);
    }
    hasher.update(b"Path\0");
    hasher.update(
        environment
            .get("Path")
            .or_else(|| environment.get("PATH"))
            .map(String::as_str)
            .unwrap_or("")
            .as_bytes(),
    );
    format!("{:x}", hasher.finalize())
}

fn create_environment_backup(
    paths: &AppPaths,
    environment: &HashMap<String, String>,
) -> Result<String, String> {
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
    let directory = paths.config().join("env_backups");
    fs::create_dir_all(&directory).map_err(|error| format!("创建环境备份目录失败：{error}"))?;
    let file_name = format!("env-backup-{}.json", filename_timestamp());
    save_json(&directory.join(&file_name), &backup)?;
    if let Ok(entries) = fs::read_dir(&directory) {
        let mut files = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        files.sort();
        let remove_count = files.len().saturating_sub(20);
        for old in files.into_iter().take(remove_count) {
            let _ = fs::remove_file(old);
        }
    }
    Ok(file_name)
}

#[tauri::command]
fn preview_user_environment_configuration() -> Result<EnvironmentConfigPreview, String> {
    let paths = load_paths()?;
    paths.ensure().map_err(|error| error.to_string())?;
    let environment = user_environment()?;
    let old_path = environment
        .get("Path")
        .or_else(|| environment.get("PATH"))
        .cloned()
        .unwrap_or_default();
    let proposed_path = merge_path(&old_path);
    let java_home = select_java_home(&paths, &environment);
    let current_devenv = environment.get("DEVENV_HOME").cloned().unwrap_or_default();
    let proposed_devenv = display_path(&paths.root);
    let current_java = environment.get("JAVA_HOME").cloned().unwrap_or_default();
    let old_entries = split_path_entries(&old_path);
    let new_entries = split_path_entries(&proposed_path);
    let normalized_old = old_entries
        .iter()
        .map(|entry| entry.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let normalized_new = new_entries
        .iter()
        .map(|entry| entry.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let path_added = new_entries
        .iter()
        .filter(|entry| !normalized_old.contains(&entry.to_ascii_lowercase()))
        .cloned()
        .collect::<Vec<_>>();
    let path_removed = old_entries
        .iter()
        .filter(|entry| !normalized_new.contains(&entry.to_ascii_lowercase()))
        .cloned()
        .collect::<Vec<_>>();
    let created_at = unix_timestamp().to_string();
    let mut hasher = Sha256::new();
    hasher.update(created_at.as_bytes());
    hasher.update(proposed_devenv.as_bytes());
    hasher.update(proposed_path.as_bytes());
    let preview_id = format!("env-{:x}", hasher.finalize());
    let backup_name = "env-backup-<应用时间>.json".to_string();
    let mut warnings = inspect_path_entries(&new_entries, &paths);
    warnings.push("只修改当前用户环境变量；已打开的终端和 IDE 不会自动刷新".to_string());
    let preview = EnvironmentConfigPreview {
        preview_id: preview_id.clone(),
        created_at,
        changes: vec![
            EnvironmentValueChange {
                name: "DEVENV_HOME".to_string(),
                current: current_devenv,
                proposed: proposed_devenv,
                impact: "DevEnv Manager 受管目录根路径".to_string(),
            },
            EnvironmentValueChange {
                name: "JAVA_HOME".to_string(),
                current: current_java,
                proposed: java_home.clone().unwrap_or_default(),
                impact: "仅在已验证 JDK 可用时设置".to_string(),
            },
            EnvironmentValueChange {
                name: "Path".to_string(),
                current: format!("{} 个条目", old_entries.len()),
                proposed: format!("{} 个条目", new_entries.len()),
                impact: "受管路径置前并去重；不删除外部工具路径".to_string(),
            },
        ],
        path_added,
        path_removed,
        warnings,
        backup_name,
    };
    let now = unix_timestamp();
    let mut previews = environment_preview_store()
        .lock()
        .map_err(|_| "环境配置预览暂时不可用".to_string())?;
    previews.retain(|_, pending| {
        pending
            .preview
            .created_at
            .parse::<u64>()
            .unwrap_or(0)
            .saturating_add(10 * 60)
            >= now
    });
    previews.insert(
        preview_id,
        PendingEnvironmentConfig {
            preview: preview.clone(),
            java_home,
            path: proposed_path,
            baseline_fingerprint: environment_fingerprint(&environment),
        },
    );
    Ok(preview)
}

#[tauri::command]
fn apply_user_environment_configuration(preview_id: String) -> Result<OperationResult, String> {
    let pending = environment_preview_store()
        .lock()
        .map_err(|_| "环境配置预览暂时不可用".to_string())?
        .remove(&preview_id)
        .ok_or_else(|| "环境配置预览不存在、已应用或已过期，请重新预览".to_string())?;
    let created = pending.preview.created_at.parse::<u64>().unwrap_or(0);
    if created.saturating_add(10 * 60) < unix_timestamp() {
        return Err("环境配置预览已超过 10 分钟，请重新预览".to_string());
    }
    let paths = load_paths()?;
    let environment = user_environment()?;
    if environment_fingerprint(&environment) != pending.baseline_fingerprint {
        return Err("用户环境变量在预览后发生变化，已拒绝写入；请重新预览".to_string());
    }
    let backup = create_environment_backup(&paths, &environment)?;
    set_user_environment_values(&paths, pending.java_home.as_deref(), &pending.path)?;
    broadcast_environment_change();
    Ok(OperationResult {
        success: true,
        message: format!("用户环境变量已写入并回读验证；备份：{backup}"),
    })
}

#[tauri::command]
fn list_environment_backups() -> Result<Vec<EnvironmentBackupInfo>, String> {
    let paths = load_paths()?;
    let directory = paths.config().join("env_backups");
    let mut result = Vec::new();
    let Ok(entries) = fs::read_dir(directory) else {
        return Ok(result);
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if !file_name.starts_with("env-backup-") || !file_name.ends_with(".json") {
            continue;
        }
        let Ok(value) = read_json::<Value>(&path) else {
            continue;
        };
        result.push(EnvironmentBackupInfo {
            file_name,
            created_at: value
                .get("created_at")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            devenv_home: value
                .get("DEVENV_HOME")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            java_home: value
                .get("JAVA_HOME")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            path_entries: value
                .get("Path")
                .and_then(Value::as_str)
                .map(|path| split_path_entries(path).len())
                .unwrap_or(0),
        });
    }
    result.sort_by(|a, b| b.file_name.cmp(&a.file_name));
    Ok(result)
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
    let backup_name = create_environment_backup(&paths, &environment)?;
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
            .map(|value| format!("已配置用户环境变量，JAVA_HOME = {value}；备份：{backup_name}"))
            .unwrap_or_else(|| {
                format!("已配置用户环境变量，未发现可用 JAVA_HOME；备份：{backup_name}")
            }),
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
    let backup_name = if removed > 0 {
        Some(create_environment_backup(&paths, &environment)?)
    } else {
        None
    };
    let java_home = environment.get("JAVA_HOME").map(String::as_str);
    set_user_environment_values(&paths, java_home, &new_path)?;
    broadcast_environment_change();
    Ok(OperationResult {
        success: true,
        message: if removed == 0 {
            "PATH 没有需要清理的真实失效或重复项".to_string()
        } else {
            format!(
                "已清理 {removed} 个真实失效或重复 PATH，托管待安装路径已保留；备份：{}",
                backup_name.unwrap_or_default()
            )
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
fn restore_environment_backup(file_name: String) -> Result<OperationResult, String> {
    if !file_name.starts_with("env-backup-")
        || !file_name.ends_with(".json")
        || file_name
            .chars()
            .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.')))
    {
        return Err("环境备份文件名无效".to_string());
    }
    let paths = load_paths()?;
    let source = paths.config().join("env_backups").join(&file_name);
    let backup: Value = read_json(&source)?;
    let current = user_environment()?;
    let safety_backup = create_environment_backup(&paths, &current)?;
    let path = backup.get("Path").and_then(Value::as_str).unwrap_or("");
    let devenv_home = backup.get("DEVENV_HOME").and_then(Value::as_str);
    let java_home = backup.get("JAVA_HOME").and_then(Value::as_str);
    restore_environment_values(devenv_home, java_home, path)?;
    broadcast_environment_change();
    Ok(OperationResult {
        success: true,
        message: format!("已恢复环境备份 {file_name}；恢复前状态另存为 {safety_backup}"),
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
fn inspect_runtime_strong_verification() -> Result<RuntimeStrongVerificationReport, String> {
    let paths = load_paths()?;
    let installed = load_installed(&paths)?;
    let mut items = Vec::new();
    for kind in ["jdk", "python", "node", "maven", "gradle", "go"] {
        let meta = runtime_meta(kind)?;
        for record in collection(&installed, meta.collection) {
            items.push(verify_registered_runtime(&paths, &installed, meta, record));
        }
    }
    let summary = vec![
        "目录存在只代表文件夹存在；版本命令通过才代表基本能运行。".to_string(),
        "组件检查通过才代表开发所需组件完整；环境生效还需要 current 指针和用户 PATH/JAVA_HOME 命中。"
            .to_string(),
        "组件缺失不会显示为完全可用。".to_string(),
    ];
    Ok(RuntimeStrongVerificationReport {
        generated_at: current_timestamp(),
        items,
        summary,
    })
}

fn current_version_for_kind<'a>(installed: &'a InstalledData, kind: &str) -> Option<&'a str> {
    match kind {
        "jdk" => installed.current.jdk.as_deref(),
        "python" => installed.current.python.as_deref(),
        "node" => installed.current.node.as_deref(),
        "maven" => installed.current.maven.as_deref(),
        "gradle" => installed.current.gradle.as_deref(),
        "go" => installed.current.go.as_deref(),
        _ => None,
    }
}

fn verify_registered_runtime(
    paths: &AppPaths,
    installed: &InstalledData,
    meta: RuntimeMeta,
    record: &Value,
) -> RuntimeStrongStatus {
    let version = record
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let root = record
        .get("path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_default();
    let executable = record
        .get(meta.exe_key)
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_default();
    let current = current_version_for_kind(installed, meta.kind) == Some(version.as_str());
    let mut checks = Vec::new();
    checks.push(ValidationCheck {
        id: "directory".to_string(),
        title: "目录存在".to_string(),
        success: root.is_dir(),
        required: true,
        detail: display_path(&root),
        stage: "DirectoryInvalid".to_string(),
    });
    checks.push(ValidationCheck {
        id: "executable".to_string(),
        title: "可执行文件存在".to_string(),
        success: executable.is_file(),
        required: true,
        detail: display_path(&executable),
        stage: "ExecutableMissing".to_string(),
    });
    match meta.kind {
        "jdk" => {
            checks.push(validation_check(
                "java",
                "java -version",
                true,
                "PostInstallVerify",
                run_command_output(root.join("bin/java.exe"), &["-version"], 30),
            ));
            checks.push(validation_check(
                "javac",
                "javac -version",
                true,
                "PostInstallVerify",
                run_command_output(root.join("bin/javac.exe"), &["-version"], 30),
            ));
            checks.push(validation_check(
                "jar",
                "jar --help",
                true,
                "ComponentMissing",
                run_command_output(root.join("bin/jar.exe"), &["--help"], 30),
            ));
        }
        "python" => {
            let report = python_integrity_for_path(&executable, paths);
            checks.extend(report.checks);
        }
        "node" => {
            checks.push(validation_check(
                "node",
                "node -v",
                true,
                "PostInstallVerify",
                run_command_output(root.join("node.exe"), &["-v"], 30),
            ));
            checks.push(validation_check(
                "npm",
                "npm -v",
                true,
                "ComponentMissing",
                run_command_output(root.join("npm.cmd"), &["-v"], 30),
            ));
            checks.push(validation_check(
                "npx",
                "npx -v",
                true,
                "ComponentMissing",
                run_command_output(root.join("npx.cmd"), &["-v"], 30),
            ));
            checks.push(validation_check(
                "corepack",
                "corepack --version",
                false,
                "OptionalComponentMissing",
                run_command_output(root.join("corepack.cmd"), &["--version"], 30),
            ));
        }
        "maven" => {
            checks.push(validation_check(
                "mvn",
                "mvn -version",
                true,
                "PostInstallVerify",
                run_command_output(root.join("bin/mvn.cmd"), &["-version"], 60),
            ));
        }
        "gradle" => {
            checks.push(validation_check(
                "gradle",
                "gradle -version",
                true,
                "PostInstallVerify",
                run_command_output(root.join("bin/gradle.bat"), &["--version"], 60),
            ));
        }
        "go" => {
            checks.push(validation_check(
                "go",
                "go version",
                true,
                "PostInstallVerify",
                run_command_output(root.join("bin/go.exe"), &["version"], 30),
            ));
            checks.push(validation_check(
                "goroot",
                "go env GOROOT",
                true,
                "PostInstallVerify",
                run_command_output(root.join("bin/go.exe"), &["env", "GOROOT"], 30),
            ));
            checks.push(validation_check(
                "gopath",
                "go env GOPATH",
                false,
                "PostInstallVerify",
                run_command_output(root.join("bin/go.exe"), &["env", "GOPATH"], 30),
            ));
            checks.push(validation_check(
                "goproxy",
                "go env GOPROXY",
                false,
                "PostInstallVerify",
                run_command_output(root.join("bin/go.exe"), &["env", "GOPROXY"], 30),
            ));
        }
        _ => {}
    }
    let user = user_environment().unwrap_or_default();
    let path_value = user
        .get("Path")
        .or_else(|| user.get("PATH"))
        .cloned()
        .unwrap_or_default();
    let environment_effective = match meta.kind {
        "jdk" => user
            .get("JAVA_HOME")
            .map(|value| {
                path_key(&expand_environment_path(value, paths)) == path_key(&display_path(&root))
            })
            .unwrap_or(false),
        "python" => find_in_configured_path("python", &path_value, paths)
            .map(|path| is_path_inside(&path, &root))
            .unwrap_or(false),
        "node" => find_in_configured_path("node", &path_value, paths)
            .map(|path| is_path_inside(&path, &root))
            .unwrap_or(false),
        "maven" => find_in_configured_path("mvn", &path_value, paths)
            .map(|path| is_path_inside(&path, &root))
            .unwrap_or(false),
        "gradle" => find_in_configured_path("gradle", &path_value, paths)
            .map(|path| is_path_inside(&path, &root))
            .unwrap_or(false),
        "go" => find_in_configured_path("go", &path_value, paths)
            .map(|path| is_path_inside(&path, &root))
            .unwrap_or(false),
        _ => false,
    };
    let required_ok = checks
        .iter()
        .filter(|item| item.required)
        .all(|item| item.success);
    let failure_stage = checks
        .iter()
        .find(|item| item.required && !item.success)
        .map(|item| item.stage.clone());
    let status = if !root.exists() {
        "已登记但目录不存在"
    } else if !executable.is_file() {
        "已登记但不可用"
    } else if !required_ok {
        "组件缺失"
    } else if current && environment_effective {
        "当前生效"
    } else if current {
        "已安装但环境未生效"
    } else {
        "可用"
    }
    .to_string();
    RuntimeStrongStatus {
        kind: meta.kind.to_string(),
        version,
        path: display_path(root),
        registered: true,
        current,
        environment_effective,
        status,
        checks,
        failure_stage,
        report: vec![
            "安装失败不会写入 installed.json；本报告只检查已登记记录。".to_string(),
            "current 指针和环境生效是独立状态，请重新打开终端/IDE 后验证。".to_string(),
        ],
    }
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
    install_zip_payload(
        &archive,
        &target,
        &["bin/java.exe", "bin/javac.exe", "bin/jar.exe"],
    )?;
    emit_task_progress(&app, &task, 88, "正在验证 JDK");
    let output = run_command_output(target.join("bin/java.exe"), &["-version"], 30)?;
    run_command_output(target.join("bin/javac.exe"), &["-version"], 30)?;
    run_command_output(target.join("bin/jar.exe"), &["--help"], 30)?;
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
    emit_task_progress(&app, &task, 88, "正在验证 Python 完整性");
    let integrity = python_integrity_for_path(&python_exe, &paths);
    if !integrity.fully_usable {
        let failed = integrity
            .checks
            .iter()
            .filter(|item| item.required && !item.success)
            .map(|item| format!("{}：{}", item.title, item.detail))
            .collect::<Vec<_>>()
            .join("；");
        return Err(format!(
            "Python 组件完整性检查失败，未写入 installed.json：{failed}"
        ));
    }
    let verify = run_command_output(python_exe.clone(), &["--version"], 30)?;
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
        emit_task_progress(
            &app,
            &task,
            18,
            "检测到 Maven 已安装，正在修复登记与 current 指针",
        );
    } else {
        emit_task_progress(&app, &task, 18, "正在下载 Maven");
        download_file_with_progress(&release.url, &archive, None, Some((&app, &task, 18, 70)))?;
        emit_task_progress(&app, &task, 72, "正在解压 Maven");
        install_zip_payload(&archive, &target, &["bin/mvn.cmd"])?;
    }
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
        message: format!("Maven {} 已就绪并已切换到 current", release.tag),
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
        emit_task_progress(
            &app,
            &task,
            18,
            "检测到 Gradle 已安装，正在修复登记与 current 指针",
        );
    } else {
        emit_task_progress(&app, &task, 18, "正在下载 Gradle");
        download_file_with_progress(
            &release.url,
            &archive,
            release.sha256.as_deref(),
            Some((&app, &task, 18, 70)),
        )?;
        emit_task_progress(&app, &task, 72, "正在解压 Gradle");
        install_zip_payload(&archive, &target, &["bin/gradle.bat"])?;
    }
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
        message: format!("Gradle {} 已就绪并已切换到 current", release.tag),
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
        create_environment_backup(&paths, &previous_environment)?;
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
fn kill_process(
    pid: u32,
    force: bool,
    allow_caution: bool,
    confirmation_token: Option<String>,
) -> KillResult {
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
    let risk_level = if force { "critical" } else { "high" };
    let plan_id = format!("pid-{pid}-force-{force}-allow-{allow_caution}");
    let fingerprint = process_action_fingerprint("kill_process", &plan_id, risk_level);
    if let Err(message) = require_confirmation_token(
        confirmation_token,
        "kill_process",
        &plan_id,
        risk_level,
        &fingerprint,
        false,
    ) {
        return KillResult {
            success: false,
            message,
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

fn process_action_fingerprint(action_id: &str, plan_id: &str, risk_level: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(action_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(plan_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(risk_level.as_bytes());
    format!("{:x}", hasher.finalize())
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
        let signature = analyze_port_signature(
            local_port,
            &state,
            &process_name,
            &process_path,
            &command_line,
            &service_names,
        );
        let command_line = redact_command_line(&command_line);
        let common_usage = signature.identity.clone();
        let explanation = signature.explanation.clone();
        let risk = signature.risk.clone();

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
            identity: signature.identity,
            confidence: signature.confidence,
            evidence_count: signature.evidence.len(),
            conflict_count: signature.conflict_evidence.len(),
            risk_level: signature.risk_level,
            recommendation: signature.recommendation,
            evidence: signature.evidence,
            conflict_evidence: signature.conflict_evidence,
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

fn archive_plan_file(paths: &AppPaths) -> PathBuf {
    paths.config().join("archive-plan.json")
}

fn load_archive_plan(paths: &AppPaths) -> Result<Vec<ArchivePlanItem>, String> {
    load_json_with_default(&archive_plan_file(paths), Vec::<ArchivePlanItem>::new())
}

fn archive_user_root_allowed(path: &Path) -> bool {
    dirs::home_dir().is_some_and(|home| {
        [home.join("Desktop"), home.join("Downloads")]
            .iter()
            .any(|root| path.starts_with(root))
    })
}

fn archive_path_is_sensitive(path: &Path) -> bool {
    let lower = display_path(path).replace('/', "\\").to_ascii_lowercase();
    lower.contains("\\wechat files\\")
        || lower.contains("\\tencent files\\")
        || lower.contains("\\google\\chrome\\")
        || lower.contains("\\microsoft\\edge\\")
        || lower.contains("\\mozilla\\firefox\\")
        || lower.contains("cookie")
        || lower.contains("login data")
        || lower.contains("password")
}

#[tauri::command]
fn add_archive_plan_item(path: String, source: String) -> Result<OperationResult, String> {
    let candidate = PathBuf::from(path.trim())
        .canonicalize()
        .map_err(|error| format!("解析归档候选失败：{error}"))?;
    let metadata =
        fs::symlink_metadata(&candidate).map_err(|error| format!("读取归档候选失败：{error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("归档计划当前只接受普通文件，不接受目录或符号链接".to_string());
    }
    if archive_path_is_sensitive(&candidate) {
        return Err("聊天数据、浏览器用户数据和凭据不能加入归档计划".to_string());
    }
    if cleanup::is_inside_managed_runtime(&candidate)
        || env::current_dir()
            .ok()
            .is_some_and(|root| candidate.starts_with(root))
        || (cleanup::should_skip_path(&candidate).is_some()
            && !archive_user_root_allowed(&candidate))
    {
        return Err("系统、当前项目或受管运行时不能加入归档计划".to_string());
    }
    let paths = load_paths()?;
    let mut plan = load_archive_plan(&paths)?;
    let mut hasher = Sha256::new();
    hasher.update(display_path(&candidate).to_ascii_lowercase().as_bytes());
    let id = format!("{:x}", hasher.finalize());
    if plan.iter().any(|item| item.id == id) {
        return Ok(OperationResult {
            success: true,
            message: "该文件已经在归档计划中；本阶段不会移动它".to_string(),
        });
    }
    plan.push(ArchivePlanItem {
        id,
        path: display_path(&candidate),
        size: metadata.len(),
        source: source.trim().chars().take(80).collect(),
        added_at: current_timestamp(),
        suggestion: "Phase 4 执行移动前将重新校验路径、目标空间并生成回滚计划".to_string(),
    });
    save_json(&archive_plan_file(&paths), &plan)?;
    Ok(OperationResult {
        success: true,
        message: format!(
            "已加入归档计划；当前共 {} 项，本阶段没有移动文件",
            plan.len()
        ),
    })
}

#[tauri::command]
fn list_archive_plan_items() -> Result<Vec<ArchivePlanItem>, String> {
    load_archive_plan(&load_paths()?)
}

#[tauri::command]
fn remove_archive_plan_item(id: String) -> Result<OperationResult, String> {
    let paths = load_paths()?;
    let mut plan = load_archive_plan(&paths)?;
    let before = plan.len();
    plan.retain(|item| item.id != id);
    save_json(&archive_plan_file(&paths), &plan)?;
    Ok(OperationResult {
        success: true,
        message: if plan.len() < before {
            "已从归档计划移除；原文件没有变化"
        } else {
            "归档计划中没有该项目"
        }
        .to_string(),
    })
}

#[tauri::command]
fn clear_download_cache() -> Result<OperationResult, String> {
    let paths = load_paths()?;
    let result = cleanup::clean_managed_download_cache(&paths.root);
    Ok(OperationResult {
        success: result.success,
        message: format!(
            "下载缓存已移入回收站：{} 项，释放 {}，跳过 {}，失败 {}",
            result.cleaned_items,
            format_size(result.cleaned_bytes),
            result.skipped_items,
            result.failed_items
        ),
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
    let mut value =
        serde_json::to_value(&report).map_err(|err| format!("生成 JSON 报告失败：{err}"))?;
    redact_json_value(&mut value);
    let text =
        serde_json::to_string_pretty(&value).map_err(|err| format!("生成 JSON 报告失败：{err}"))?;
    fs::write(&target, text).map_err(|err| format!("写入 JSON 报告失败：{err}"))?;
    Ok(OperationResult {
        success: true,
        message: format!("已导出 JSON 诊断报告：{}", display_path(target)),
    })
}

#[tauri::command]
fn doctor_report_text(report: DoctorReport, format: String) -> Result<String, String> {
    let text = match format.as_str() {
        "markdown" => doctor_report_markdown(&report),
        "json" => {
            let mut value = serde_json::to_value(&report)
                .map_err(|err| format!("生成 JSON 报告失败：{err}"))?;
            redact_json_value(&mut value);
            serde_json::to_string_pretty(&value)
                .map_err(|err| format!("生成 JSON 报告失败：{err}"))?
        }
        _ => return Err("不支持的报告格式".to_string()),
    };
    Ok(redact_report_text(&text))
}

#[tauri::command]
async fn analyze_python_environment() -> Result<PythonAnalysis, String> {
    run_blocking(|| Ok(analyze_python_environment_blocking())).await?
}

fn analyze_python_environment_blocking() -> PythonAnalysis {
    let first_python_on_path = find_on_path("python").unwrap_or_default();
    let first_pip_on_path = find_on_path("pip").unwrap_or_default();
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
    let python_m_pip_available = python_m_pip.is_some();
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
    let launcher_path = find_on_path("py").unwrap_or_default();
    let user = user_environment().unwrap_or_default();
    let user_path = user
        .get("Path")
        .or_else(|| user.get("PATH"))
        .cloned()
        .unwrap_or_default();
    let user_path_entry_count = split_path_entries(&user_path).len();
    let configured_python = load_paths()
        .ok()
        .and_then(|paths| find_in_configured_path("python", &user_path, &paths));
    let current_terminal_matches_user_path = match (&current_python, configured_python) {
        (Some(current), Some(configured)) => {
            path_key(&current.path) == path_key(&display_path(configured))
        }
        (None, None) => true,
        _ => false,
    };
    let store_alias_risk = discovered_pythons.iter().any(|item| {
        item.source == "Microsoft Store"
            || item.path.to_ascii_lowercase().contains("\\windowsapps\\")
    });
    let managed_python_available = load_paths()
        .ok()
        .and_then(|paths| load_installed(&paths).ok())
        .map(|installed| !installed.pythons.is_empty())
        .unwrap_or(false);

    let mut risks = Vec::new();
    if discovered_pythons.len() > 1 {
        risks.push(format!(
            "PATH/注册表中发现 {} 个 Python，pip 容易安装到错误版本",
            discovered_pythons.len()
        ));
    }
    if store_alias_risk {
        risks.push("Microsoft Store Python 执行别名可能抢占 python 命令".to_string());
    }
    if !current_terminal_matches_user_path {
        risks
            .push("当前进程 PATH 与最新用户 PATH 不一致；修复后需要重新打开终端和 IDE".to_string());
    }
    if current_pip.as_ref().map(|item| item.status.as_str()) != Some("正常") {
        risks.push("pip 与当前 python -m pip 不一致或当前 Python 缺少 pip".to_string());
    }
    let mut repair_blockers = Vec::new();
    if store_alias_risk {
        repair_blockers.push("命中 WindowsApps Store Alias 时，DevEnv Manager 不会自动关闭别名或删除 WindowsApps PATH。".to_string());
    }
    if current_python.is_none() {
        repair_blockers
            .push("当前 PATH 没有可执行 python，不能对未知 Python 执行 ensurepip。".to_string());
    }
    if !python_m_pip_available {
        repair_blockers.push(
            "当前 python -m pip 不可用；只有受管 Python 才允许生成 pip 修复计划。".to_string(),
        );
    }
    if !managed_python_available {
        repair_blockers.push("尚未安装受管 Python；无法直接切换到 DevEnv 管理版本。".to_string());
    }
    if risks.is_empty() {
        risks.push("未发现明显 Python 冲突".to_string());
    }

    let mut recommendations = Vec::new();
    recommendations.push("优先使用 DevEnv Manager 受管 Python 或官网安装版 Python。".to_string());
    recommendations.push("安装包时尽量使用 python -m pip，而不是直接运行 pip。".to_string());
    if store_alias_risk {
        recommendations.push(
            "如默认 python 指向 WindowsApps，请在 Windows“应用执行别名”中关闭 Python 别名。"
                .to_string(),
        );
    }
    let mut recovery_actions = vec![
        "打开 Windows 应用执行别名设置，人工关闭 python.exe / python3.exe Store Alias。"
            .to_string(),
        "重新检测 Python 环境，确认新终端和 IDE 已继承最新 PATH。".to_string(),
        "导出只读 Python 诊断报告，发给自己或 issue 复盘。".to_string(),
    ];
    if managed_python_available {
        recovery_actions
            .push("切换到已安装的受管 Python，并生成用户级 PATH 修复计划。".to_string());
    } else {
        recovery_actions
            .push("安装受管 Python，再用预览计划切换；不会删除系统 Python。".to_string());
    }
    let diagnostic_report = python_diagnostic_report(PythonDiagnosticInput {
        current_python: current_python.as_ref(),
        current_pip: current_pip.as_ref(),
        launcher_path: &launcher_path,
        launcher_output: &launcher_output,
        first_python_on_path: &first_python_on_path,
        first_pip_on_path: &first_pip_on_path,
        python_m_pip_available,
        store_alias_risk,
        managed_python_available,
        risks: &risks,
        repair_blockers: &repair_blockers,
        recovery_actions: &recovery_actions,
    });

    PythonAnalysis {
        current_python,
        current_pip,
        launcher_path,
        launcher_output,
        first_python_on_path,
        first_pip_on_path,
        python_m_pip_available,
        managed_python_available,
        discovered_pythons,
        discovered_pips,
        user_path_entry_count,
        current_terminal_matches_user_path,
        store_alias_risk,
        repair_blockers,
        recovery_actions,
        diagnostic_report,
        risks,
        recommendations,
        pip_repair_command: "python -m ensurepip --upgrade; python -m pip install --upgrade pip"
            .to_string(),
        alias_settings_command: "start ms-settings:appsfeatures-app".to_string(),
    }
}

struct PythonDiagnosticInput<'a> {
    current_python: Option<&'a PythonToolState>,
    current_pip: Option<&'a PythonToolState>,
    launcher_path: &'a str,
    launcher_output: &'a str,
    first_python_on_path: &'a str,
    first_pip_on_path: &'a str,
    python_m_pip_available: bool,
    store_alias_risk: bool,
    managed_python_available: bool,
    risks: &'a [String],
    repair_blockers: &'a [String],
    recovery_actions: &'a [String],
}

fn python_diagnostic_report(input: PythonDiagnosticInput<'_>) -> String {
    redact_report_text(&format!(
        "# Python diagnostic\n\n默认 python: {}\n默认 pip: {}\nPATH 首个 python: {}\nPATH 首个 pip: {}\npy launcher: {}\npython -m pip: {}\nWindowsApps Alias: {}\n受管 Python: {}\n\n## py -0p\n{}\n\n## 风险\n{}\n\n## 阻断原因\n{}\n\n## 下一步\n{}\n",
        input.current_python
            .map(|item| format!("{} · {} · {}", item.status, item.version, item.path))
            .unwrap_or_else(|| "未发现".to_string()),
        input.current_pip
            .map(|item| format!("{} · {} · {}", item.status, item.version, item.path))
            .unwrap_or_else(|| "未发现".to_string()),
        if input.first_python_on_path.is_empty() { "未发现" } else { input.first_python_on_path },
        if input.first_pip_on_path.is_empty() { "未发现" } else { input.first_pip_on_path },
        if input.launcher_path.is_empty() { "未发现" } else { input.launcher_path },
        if input.python_m_pip_available { "可用" } else { "不可用" },
        if input.store_alias_risk { "可能命中" } else { "未发现" },
        if input.managed_python_available { "存在" } else { "不存在" },
        input.launcher_output,
        input.risks.iter().map(|item| format!("- {item}")).collect::<Vec<_>>().join("\n"),
        input.repair_blockers.iter().map(|item| format!("- {item}")).collect::<Vec<_>>().join("\n"),
        input.recovery_actions.iter().map(|item| format!("- {item}")).collect::<Vec<_>>().join("\n"),
    ))
}

#[tauri::command]
fn export_python_diagnostic_report() -> Result<OperationResult, String> {
    let analysis = analyze_python_environment_blocking();
    let reports = app_config_dir().join("reports");
    fs::create_dir_all(&reports).map_err(|err| format!("创建报告目录失败：{err}"))?;
    let target = reports.join(format!("python-diagnostic-{}.md", filename_timestamp()));
    fs::write(&target, analysis.diagnostic_report)
        .map_err(|err| format!("写入 Python 诊断报告失败：{err}"))?;
    Ok(OperationResult {
        success: true,
        message: format!("已导出 Python 只读诊断报告：{}", display_path(target)),
    })
}

fn learning_command_allowed(parts: &[String]) -> bool {
    let Some(program) = parts.first() else {
        return false;
    };
    let program = program
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(program)
        .trim_end_matches(".exe")
        .trim_end_matches(".cmd")
        .to_ascii_lowercase();
    let args = parts.iter().skip(1).map(String::as_str).collect::<Vec<_>>();
    match program.as_str() {
        "java" | "javac" | "mvn" | "gradle" => args == ["-version"],
        "python" | "python3" => args == ["--version"] || args == ["-m", "pip", "--version"],
        "py" => args == ["--version"] || args == ["-0p"],
        "node" | "npm" | "rustc" | "cargo" | "uv" | "chsrc" | "scoop" => args == ["--version"],
        "go" => args == ["version"] || args == ["env", "GOROOT"] || args == ["env", "GOPATH"],
        "dotnet" => args == ["--info"] || args == ["--list-sdks"] || args == ["--list-runtimes"],
        "mise" => args == ["doctor"] || args == ["--version"],
        "vfox" => args == ["version"] || args == ["--version"],
        "where" => {
            args.len() == 1
                && [
                    "java", "javac", "python", "pip", "py", "node", "npm", "mvn", "gradle", "go",
                    "rustc", "cargo", "dotnet",
                ]
                .contains(&args[0].to_ascii_lowercase().as_str())
        }
        _ => false,
    }
}

#[tauri::command]
async fn run_learning_check(command: String) -> Result<CommandRunResult, String> {
    run_blocking(move || {
        let parts = parse_command_line(&command)?;
        if !learning_command_allowed(&parts) {
            return Err("学习中心只允许固定的版本、位置和环境只读检查命令".to_string());
        }
        let executable = parts.first().ok_or_else(|| "命令不能为空".to_string())?;
        let started = Instant::now();
        let output = hidden_command(executable)
            .args(parts.iter().skip(1))
            .output()
            .map_err(|error| format!("执行只读检查失败：{error}"))?;
        Ok(CommandRunResult {
            success: output.status.success(),
            return_code: output.status.code().unwrap_or(-1),
            output: command_text(&output.stdout, &output.stderr),
            elapsed_ms: started.elapsed().as_millis(),
        })
    })
    .await?
}

fn python_repair_store() -> &'static Mutex<HashMap<String, PendingPythonRepair>> {
    PYTHON_REPAIR_PREVIEWS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn validation_check(
    id: &str,
    title: &str,
    required: bool,
    stage: &str,
    result: Result<String, String>,
) -> ValidationCheck {
    match result {
        Ok(detail) => ValidationCheck {
            id: id.to_string(),
            title: title.to_string(),
            success: true,
            required,
            detail: first_meaningful_output_line(&detail).unwrap_or(detail),
            stage: stage.to_string(),
        },
        Err(error) => ValidationCheck {
            id: id.to_string(),
            title: title.to_string(),
            success: false,
            required,
            detail: error.trim().to_string(),
            stage: stage.to_string(),
        },
    }
}

fn python_integrity_for_path(python_exe: &Path, paths: &AppPaths) -> PythonIntegrityReport {
    let python_home = python_exe.parent().unwrap_or(python_exe);
    let managed = is_path_inside(python_exe, &paths.pythons())
        || is_path_inside(python_exe, &paths.current().join("python"));
    let checks = vec![
        validation_check(
            "python-version",
            "python --version",
            true,
            "PostInstallVerify",
            run_command_output(python_exe.to_path_buf(), &["--version"], 30),
        ),
        validation_check(
            "python-executable",
            "sys.executable",
            true,
            "PostInstallVerify",
            run_command_output(
                python_exe.to_path_buf(),
                &["-c", "import sys; print(sys.executable)"],
                30,
            ),
        ),
        validation_check(
            "pip-module",
            "python -m pip --version",
            true,
            "ComponentMissing",
            run_command_output(python_exe.to_path_buf(), &["-m", "pip", "--version"], 60),
        ),
        validation_check(
            "venv",
            "python -m venv --help",
            true,
            "ComponentMissing",
            run_command_output(python_exe.to_path_buf(), &["-m", "venv", "--help"], 60),
        ),
        validation_check(
            "ssl",
            "ssl",
            true,
            "ComponentMissing",
            run_command_output(
                python_exe.to_path_buf(),
                &["-c", "import ssl; print(ssl.OPENSSL_VERSION)"],
                30,
            ),
        ),
        validation_check(
            "sqlite3",
            "sqlite3",
            true,
            "ComponentMissing",
            run_command_output(
                python_exe.to_path_buf(),
                &["-c", "import sqlite3; print(sqlite3.sqlite_version)"],
                30,
            ),
        ),
        validation_check(
            "ctypes",
            "ctypes",
            true,
            "ComponentMissing",
            run_command_output(
                python_exe.to_path_buf(),
                &["-c", "import ctypes; print('ok')"],
                30,
            ),
        ),
        validation_check(
            "tkinter",
            "tkinter",
            false,
            "OptionalComponentMissing",
            run_command_output(
                python_exe.to_path_buf(),
                &["-c", "import tkinter; print('ok')"],
                30,
            ),
        ),
    ];
    let pip_exe = python_home.join("Scripts").join("pip.exe");
    let pip_exe_check = ValidationCheck {
        id: "pip-exe".to_string(),
        title: "Scripts\\pip.exe".to_string(),
        success: pip_exe.is_file(),
        required: true,
        detail: if pip_exe.is_file() {
            display_path(&pip_exe)
        } else {
            "Scripts\\pip.exe 不存在".to_string()
        },
        stage: "ExecutableMissing".to_string(),
    };
    let mut checks = checks;
    checks.push(pip_exe_check);
    let pip_module = checks
        .iter()
        .find(|item| item.id == "pip-module")
        .map(|item| item.detail.clone())
        .unwrap_or_default();
    let pip_exe_output = if pip_exe.is_file() {
        run_command_output(pip_exe.clone(), &["--version"], 30).unwrap_or_default()
    } else {
        String::new()
    };
    let mut risks = Vec::new();
    if !pip_exe_output.is_empty()
        && !pip_module.is_empty()
        && !same_python_package_location(&pip_module, &pip_exe_output)
    {
        risks.push("pip.exe 与 python -m pip 归属不一致，建议使用 python -m pip。".to_string());
    }
    if checks
        .iter()
        .any(|item| item.id == "tkinter" && !item.success)
    {
        risks.push("tkinter 不可用，Python GUI 相关库可能无法使用。".to_string());
    }
    if display_path(python_exe)
        .to_ascii_lowercase()
        .contains("\\windowsapps\\")
    {
        risks
            .push("当前 Python 可能是 Microsoft Store Alias；本程序不会自动关闭别名。".to_string());
    }
    let required_ok = checks
        .iter()
        .filter(|item| item.required)
        .all(|item| item.success);
    let mut suggestions =
        vec!["优先使用 python -m pip，避免 pip.exe 指向其他 Python。".to_string()];
    if managed
        && checks
            .iter()
            .any(|item| item.id == "pip-module" && !item.success)
    {
        suggestions
            .push("这是受管 Python，可生成 pip 修复计划执行 ensurepip 与 pip 升级。".to_string());
    } else if !managed
        && checks
            .iter()
            .any(|item| item.id == "pip-module" && !item.success)
    {
        suggestions.push(
            "非受管 Python 只提示问题，不自动修复；请使用其官方安装器或包管理器处理。".to_string(),
        );
    }
    PythonIntegrityReport {
        python_path: display_path(python_exe),
        python_home: display_path(python_home),
        managed,
        fully_usable: required_ok,
        status: if required_ok {
            if checks.iter().any(|item| !item.success && !item.required) {
                "可用，存在可选组件提示".to_string()
            } else {
                "可用".to_string()
            }
        } else {
            "组件缺失".to_string()
        },
        checks,
        risks,
        suggestions,
    }
}

#[tauri::command]
fn inspect_python_integrity(python_path: Option<String>) -> Result<PythonIntegrityReport, String> {
    let paths = load_paths()?;
    let python = python_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| resolve_tool(&paths, "python"))
        .ok_or_else(|| "没有找到 Python，请先安装或切换 Python".to_string())?;
    if !python.is_file() {
        return Err("Python 路径不存在或不是文件".to_string());
    }
    Ok(python_integrity_for_path(&python, &paths))
}

#[tauri::command]
fn create_managed_python_pip_repair_plan(python_path: String) -> Result<PythonRepairPlan, String> {
    let paths = load_paths()?;
    let python = PathBuf::from(python_path.trim());
    if !python.is_file() {
        return Err("Python 路径不存在或不是文件".to_string());
    }
    if !is_path_inside(&python, &paths.pythons())
        && !is_path_inside(&python, &paths.current().join("python"))
    {
        return Err("只允许为 DevEnv Manager 受管 Python 生成 pip 修复计划".to_string());
    }
    let current_python = resolve_tool(&paths, "python")
        .ok_or_else(|| "没有找到当前生效的 Python，请先切换到该受管 Python".to_string())?;
    if path_key(&display_path(&current_python)) != path_key(&display_path(&python)) {
        return Err(
            "pip 修复计划只会操作当前生效的 Python；请先切换到该受管 Python 后再生成计划"
                .to_string(),
        );
    }
    preview_python_repair(true, true)
}

#[tauri::command]
async fn apply_managed_python_pip_repair(plan_id: String) -> Result<OperationResult, String> {
    apply_python_repair(plan_id).await
}

fn prepend_path_entries(existing: &str, additions: &[String]) -> (String, Vec<String>) {
    let existing_entries = split_path_entries(existing);
    let existing_keys = existing_entries
        .iter()
        .map(|item| path_key(item))
        .collect::<BTreeSet<_>>();
    let mut result = Vec::new();
    let mut added = Vec::new();
    let mut seen = BTreeSet::new();
    for item in additions.iter().chain(existing_entries.iter()) {
        let key = path_key(item);
        if key.is_empty() || !seen.insert(key.clone()) {
            continue;
        }
        if additions.iter().any(|value| path_key(value) == key) && !existing_keys.contains(&key) {
            added.push(item.clone());
        }
        result.push(item.clone());
    }
    (result.join(";"), added)
}

#[tauri::command]
fn preview_python_repair(repair_pip: bool, repair_path: bool) -> Result<PythonRepairPlan, String> {
    if !repair_pip && !repair_path {
        return Err("请至少选择 pip 修复或 PATH 修复".to_string());
    }
    let analysis = analyze_python_environment_blocking();
    let python = analysis
        .current_python
        .as_ref()
        .ok_or_else(|| "没有找到可修复的当前 Python；请先安装或切换 Python".to_string())?;
    if python.path.to_ascii_lowercase().contains("\\windowsapps\\")
        || !Path::new(&python.path).is_file()
    {
        return Err(
            "当前 python 是 Microsoft Store 别名或路径无效；请先关闭应用执行别名或切换受管 Python"
                .to_string(),
        );
    }
    let python_home = Path::new(&python.path)
        .parent()
        .ok_or_else(|| "无法识别 Python 安装目录".to_string())?;
    let environment = user_environment()?;
    let old_path = environment
        .get("Path")
        .or_else(|| environment.get("PATH"))
        .cloned()
        .unwrap_or_default();
    let additions = vec![
        display_path(python_home),
        display_path(python_home.join("Scripts")),
    ];
    let (proposed_path, path_added) = if repair_path {
        prepend_path_entries(&old_path, &additions)
    } else {
        (old_path, Vec::new())
    };
    let created = unix_timestamp();
    let mut hasher = Sha256::new();
    hasher.update(python.path.as_bytes());
    hasher.update(created.to_le_bytes());
    hasher.update([repair_pip as u8, repair_path as u8]);
    let plan_id = format!("python-{:x}", hasher.finalize());
    let mut actions = Vec::new();
    let mut commands = Vec::new();
    if repair_pip {
        actions.push("使用当前 python 运行 ensurepip，再升级 pip，并回读 pip 归属".to_string());
        commands.push(format!("\"{}\" -m ensurepip --upgrade", python.path));
        commands.push(format!("\"{}\" -m pip install --upgrade pip", python.path));
    }
    if repair_path {
        actions.push(
            "把当前 Python 与 Scripts 置于用户 PATH 前部；不删除其他 Python 路径".to_string(),
        );
    }
    let public = PythonRepairPlan {
        plan_id: plan_id.clone(),
        created_at: created.to_string(),
        python_path: python.path.clone(),
        actions,
        commands,
        path_added,
        warnings: vec![
            "计划有效 10 分钟且只能应用一次；用户环境在预览后变化会拒绝写入".to_string(),
            "pip 升级需要联网；不会卸载其他 Python，也不会自动关闭 Microsoft Store 别名"
                .to_string(),
            "PATH 修改后必须重新打开终端和 IDE".to_string(),
        ],
        backup_name: "env-backup-<应用时间>.json".to_string(),
    };
    let pending = PendingPythonRepair {
        public: public.clone(),
        baseline_fingerprint: environment_fingerprint(&environment),
        proposed_path,
        repair_pip,
        repair_path,
    };
    let mut store = python_repair_store()
        .lock()
        .map_err(|_| "Python 修复预览暂时不可用".to_string())?;
    store.retain(|_, item| {
        item.public
            .created_at
            .parse::<u64>()
            .unwrap_or(0)
            .saturating_add(10 * 60)
            >= created
    });
    store.insert(plan_id, pending);
    Ok(public)
}

#[tauri::command]
async fn apply_python_repair(plan_id: String) -> Result<OperationResult, String> {
    run_blocking(move || {
        let pending = python_repair_store()
            .lock()
            .map_err(|_| "Python 修复预览暂时不可用".to_string())?
            .remove(&plan_id)
            .ok_or_else(|| "Python 修复计划不存在、已应用或已过期".to_string())?;
        let created = pending.public.created_at.parse::<u64>().unwrap_or(0);
        if created.saturating_add(10 * 60) < unix_timestamp() {
            return Err("Python 修复计划已过期，请重新分析和预览".to_string());
        }
        let environment = user_environment()?;
        if environment_fingerprint(&environment) != pending.baseline_fingerprint {
            return Err("用户环境在预览后发生变化，已拒绝覆盖；请重新分析".to_string());
        }
        if !Path::new(&pending.public.python_path).is_file() {
            return Err("预览中的 Python 已不存在".to_string());
        }
        let paths = load_paths()?;
        let backup = create_environment_backup(&paths, &environment)?;
        if pending.repair_pip {
            run_command_output(
                PathBuf::from(&pending.public.python_path),
                &["-m", "ensurepip", "--upgrade"],
                180,
            )?;
            run_command_output(
                PathBuf::from(&pending.public.python_path),
                &["-m", "pip", "install", "--upgrade", "pip"],
                300,
            )?;
        }
        if pending.repair_path {
            restore_environment_values(
                environment.get("DEVENV_HOME").map(String::as_str),
                environment.get("JAVA_HOME").map(String::as_str),
                &pending.proposed_path,
            )?;
            broadcast_environment_change();
        }
        let verified = run_command_output(
            PathBuf::from(&pending.public.python_path),
            &["-m", "pip", "--version"],
            60,
        )?;
        Ok(OperationResult {
            success: true,
            message: format!(
                "Python 修复完成并回读验证：{}；环境备份：{}",
                verified.lines().next().unwrap_or("pip 可用"),
                backup
            ),
        })
    })
    .await?
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
    let chsrc = probe_tool("chsrc", resolve_tool(&paths, "chsrc"), &["--version"]);
    let chsrc_recovery = chsrc_recovery(!chsrc.installed);

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
        chsrc,
        chsrc_recovery,
        generated_at: current_timestamp(),
    })
}

fn chsrc_recovery(missing: bool) -> ChsrcRecovery {
    ChsrcRecovery {
        missing,
        explanation: if missing {
            vec![
                "chsrc 是 RubyMetric 提供的多生态换源工具，用于查看、测速和切换软件源。".to_string(),
                "当前未检测到 chsrc，因此统一换源按钮不可用；DevEnv Manager 不会静默安装第三方工具。".to_string(),
                "不安装 chsrc 时，仍可使用 npm、pip、GOPROXY、Maven、Gradle、Cargo 的单项检测和配置。".to_string(),
            ]
        } else {
            vec!["chsrc 已安装，统一换源操作仍会经过固定目标和源 ID 白名单。".to_string()]
        },
        scoop_command: "scoop install chsrc".to_string(),
        winget_command: "winget install RubyMetric.chsrc".to_string(),
        official_url: "https://github.com/RubyMetric/chsrc".to_string(),
        fallback_features: vec![
            "npm registry".to_string(),
            "pip index-url".to_string(),
            "GOPROXY".to_string(),
            "Maven settings.xml".to_string(),
            "Gradle init.gradle".to_string(),
            "Cargo config.toml".to_string(),
        ],
    }
}

#[tauri::command]
async fn run_chsrc_action(
    action: String,
    target: String,
    source: Option<String>,
) -> Result<OperationResult, String> {
    run_blocking(move || run_chsrc_action_blocking(&action, &target, source.as_deref())).await?
}

fn run_chsrc_action_blocking(
    action: &str,
    target: &str,
    source: Option<&str>,
) -> Result<OperationResult, String> {
    const TARGETS: [&str; 9] = [
        "node", "python", "go", "rust", "cargo", "maven", "gradle", "nuget", "dotnet",
    ];
    let target = target.trim().to_ascii_lowercase();
    if !TARGETS.contains(&target.as_str()) {
        return Err("该换源目标未进入 DevEnv Manager 安全白名单".to_string());
    }
    let paths = load_paths()?;
    let executable = resolve_tool(&paths, "chsrc").ok_or_else(|| {
        "未找到 chsrc。请先通过 Scoop 或 WinGet 安装官方 RubyMetric/chsrc".to_string()
    })?;
    let output = match action {
        "get" => run_action_command(&paths, executable, &["get", &target])?,
        "list" => run_action_command(&paths, executable, &["list", &target])?,
        "measure" => run_action_command(&paths, executable, &["measure", &target])?,
        "auto" => run_action_command(&paths, executable, &["set", &target])?,
        "reset" => run_action_command(&paths, executable, &["reset", &target])?,
        "set" => {
            let source = source.unwrap_or_default().trim().to_ascii_lowercase();
            if source.is_empty()
                || source.len() > 40
                || !source
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
            {
                return Err("镜像源只能填写 chsrc 列出的源 ID；不接受自定义 URL".to_string());
            }
            run_action_command(&paths, executable, &["set", &target, &source])?
        }
        _ => return Err("不支持的 chsrc 操作".to_string()),
    };
    Ok(OperationResult {
        success: true,
        message: if output.trim().is_empty() {
            format!("chsrc {action} {target} 执行完成")
        } else {
            output
        },
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

#[tauri::command]
async fn inspect_mysql_repair() -> Result<mysql_repair::MySqlRepairReport, String> {
    run_blocking(|| Ok(mysql_repair::inspect())).await?
}

#[tauri::command]
async fn create_mysql_repair_plan(
    candidate_id: String,
    action: String,
) -> Result<mysql_repair::MySqlRepairPlan, String> {
    run_blocking(move || mysql_repair::create_plan(candidate_id, action)).await?
}

#[tauri::command]
fn mysql_pending_execution_guard(
    plan_id: String,
) -> Result<mysql_repair::MySqlPendingExecutionGuard, String> {
    mysql_repair::pending_execution_guard(&plan_id)
}

#[tauri::command]
async fn execute_mysql_repair_plan(
    plan_id: String,
    backup_destination: Option<String>,
    confirmation_token: Option<String>,
) -> Result<OperationResult, String> {
    run_blocking(move || {
        let guard = mysql_repair::pending_execution_guard(&plan_id)?;
        if guard.risk_level != "low" {
            require_confirmation_token(
                confirmation_token,
                &guard.action_id,
                &guard.plan_id,
                &guard.risk_level,
                &guard.plan_fingerprint,
                guard.backup_required,
            )?;
        }
        mysql_repair::execute(plan_id, backup_destination).map(|message| OperationResult {
            success: true,
            message,
        })
    })
    .await?
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

#[tauri::command]
fn validate_directory_path(path: String) -> Result<DirectoryValidationResult, String> {
    let root = PathBuf::from(path.trim());
    if !root.exists() {
        return Ok(DirectoryValidationResult {
            path: display_path(root),
            exists: false,
            is_directory: false,
            recognized_project: false,
            signals: Vec::new(),
            message: "该路径不存在，请重新选择项目文件夹。".to_string(),
        });
    }
    if !root.is_dir() {
        return Ok(DirectoryValidationResult {
            path: display_path(root),
            exists: true,
            is_directory: false,
            recognized_project: false,
            signals: Vec::new(),
            message: "请选择项目根目录，而不是单个文件。".to_string(),
        });
    }
    let signals = project_signals(&root);
    let recognized = !signals.is_empty();
    Ok(DirectoryValidationResult {
        path: display_path(root),
        exists: true,
        is_directory: true,
        recognized_project: recognized,
        message: if recognized {
            "已识别常见项目文件，可继续分析。".to_string()
        } else {
            "没有识别到常见项目文件。你仍可以查看目录，但项目启动建议可能为空。".to_string()
        },
        signals,
    })
}

fn analyze_project_blocking(root: &Path) -> Result<ProjectAnalysis, String> {
    if !root.exists() {
        return Err("该路径不存在，请重新选择项目文件夹。".to_string());
    }
    if !root.is_dir() {
        return Err("请选择项目根目录，而不是单个文件。".to_string());
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
    let idea = inspect_idea_project_blocking(root);
    if let Ok(report) = &idea {
        if report.detected {
            push_unique(&mut project_types, "IntelliJ IDEA");
            recommendations.push(ProjectRuntimeRecommendation {
                name: "IDEA 项目 JDK".to_string(),
                requirement: if report.project_sdk.is_empty() {
                    "未显式读取到 Project SDK".to_string()
                } else {
                    format!("IDEA Project SDK：{}", report.project_sdk)
                },
                status: report.jdk_match.clone(),
            });
            warnings.extend(report.warnings.iter().cloned());
        }
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

fn read_text_file_limited(path: &Path, max_bytes: u64) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if metadata.len() > max_bytes {
        return None;
    }
    fs::read_to_string(path).ok()
}

fn extract_attr(text: &str, attr: &str) -> String {
    let pattern = format!("{attr}=\"");
    text.find(&pattern)
        .and_then(|start| {
            let rest = &text[start + pattern.len()..];
            rest.find('"').map(|end| rest[..end].to_string())
        })
        .unwrap_or_default()
}

fn extract_idea_sdk(text: &str) -> String {
    for attr in [
        "project-jdk-name",
        "jdkName",
        "inheritedJdk",
        "LANGUAGE_LEVEL",
    ] {
        let value = extract_attr(text, attr);
        if !value.is_empty() {
            return value;
        }
    }
    String::new()
}

#[tauri::command]
fn inspect_idea_project(path: String) -> Result<IdeaProjectReport, String> {
    inspect_idea_project_blocking(&PathBuf::from(path.trim()))
}

fn inspect_idea_project_blocking(root: &Path) -> Result<IdeaProjectReport, String> {
    if !root.exists() {
        return Err("该路径不存在，请重新选择项目文件夹。".to_string());
    }
    if !root.is_dir() {
        return Err("请选择项目根目录，而不是单个文件。".to_string());
    }
    let idea_dir = root.join(".idea");
    let mut read_files = Vec::new();
    let mut project_sdk = String::new();
    let mut language_level = String::new();
    let mut compiler_target = String::new();
    let mut maven_importer_jdk = String::new();
    let mut gradle_jvm = String::new();
    let mut output_dir = String::new();
    let mut module_sdks = Vec::new();
    let mut module_count = 0_usize;
    let mut warnings = Vec::new();

    if let Some(misc) = read_text_file_limited(&idea_dir.join("misc.xml"), 256 * 1024) {
        read_files.push(".idea/misc.xml".to_string());
        project_sdk = extract_attr(&misc, "project-jdk-name");
        language_level = extract_attr(&misc, "languageLevel");
        let maven = extract_attr(&misc, "jdkNameForImporter");
        if !maven.is_empty() {
            maven_importer_jdk = maven;
        }
        let gradle = extract_attr(&misc, "gradleJvm");
        if !gradle.is_empty() {
            gradle_jvm = gradle;
        }
    }
    if let Some(compiler) = read_text_file_limited(&idea_dir.join("compiler.xml"), 256 * 1024) {
        read_files.push(".idea/compiler.xml".to_string());
        compiler_target = extract_attr(&compiler, "target");
        output_dir = extract_attr(&compiler, "url");
    }
    if let Some(modules) = read_text_file_limited(&idea_dir.join("modules.xml"), 256 * 1024) {
        read_files.push(".idea/modules.xml".to_string());
        module_count = modules
            .matches("fileurl=")
            .count()
            .max(modules.matches(".iml").count());
    }
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .and_then(OsStr::to_str)
                .is_some_and(|ext| ext.eq_ignore_ascii_case("iml"))
            {
                if let Some(text) = read_text_file_limited(&path, 256 * 1024) {
                    read_files.push(
                        path.file_name()
                            .and_then(OsStr::to_str)
                            .unwrap_or("*.iml")
                            .to_string(),
                    );
                    let sdk = extract_idea_sdk(&text);
                    if !sdk.is_empty() {
                        module_sdks.push(sdk);
                    }
                }
            }
        }
    }
    if idea_dir.join("workspace.xml").is_file() {
        warnings.push(
            "检测到 workspace.xml；为保护隐私，本版本不全量读取最近文件、历史路径或敏感配置。"
                .to_string(),
        );
    }
    module_sdks.sort();
    module_sdks.dedup();
    let java = inspect_java_environment_blocking().ok();
    let current_java_home = java
        .as_ref()
        .map(|report| report.java_home_expanded.clone())
        .unwrap_or_default();
    let current_java_version = java
        .as_ref()
        .map(|report| report.java_version.clone())
        .unwrap_or_default();
    let jdk_match = if project_sdk.is_empty() {
        "未读取到 IDEA Project SDK，仅做只读提示。".to_string()
    } else if current_java_version.contains(&project_sdk)
        || current_java_home.contains(&project_sdk)
    {
        "IDEA 项目 JDK 与当前 JAVA_HOME 大致匹配。".to_string()
    } else {
        format!(
            "IDEA 项目要求 {}；当前 JAVA_HOME 为 {}。建议切换 JDK 或检查 IDEA Project SDK。",
            project_sdk,
            if current_java_home.is_empty() {
                "未设置".to_string()
            } else {
                current_java_home.clone()
            }
        )
    };
    Ok(IdeaProjectReport {
        root: display_path(root),
        detected: idea_dir.is_dir() || !read_files.is_empty(),
        read_files,
        project_sdk,
        language_level,
        module_sdks,
        module_count,
        compiler_target,
        maven_importer_jdk,
        gradle_jvm,
        output_dir,
        current_java_home,
        current_java_version,
        jdk_match,
        warnings,
    })
}

#[tauri::command]
fn verify_java_consumer_environment(
    consumer: String,
    root: String,
) -> Result<JavaConsumerReport, String> {
    verify_java_consumer_environment_blocking(&consumer, &PathBuf::from(root.trim()))
}

#[tauri::command]
fn verify_nexus_java_environment(root: String) -> Result<JavaConsumerReport, String> {
    verify_java_consumer_environment_blocking("Nexus", &PathBuf::from(root.trim()))
}

fn verify_java_consumer_environment_blocking(
    consumer: &str,
    root: &Path,
) -> Result<JavaConsumerReport, String> {
    if !root.exists() {
        return Err("该路径不存在，请重新选择项目文件夹。".to_string());
    }
    if !root.is_dir() {
        return Err("请选择项目根目录，而不是单个文件。".to_string());
    }
    let paths = load_paths()?;
    let user = user_environment().unwrap_or_default();
    let process = env::vars().collect::<HashMap<_, _>>();
    let raw = user.get("JAVA_HOME").cloned();
    let expanded = raw
        .as_deref()
        .map(|value| expand_environment_path(value, &paths));
    let java = expanded
        .as_deref()
        .map(|home| PathBuf::from(home).join("bin/java.exe"));
    let javac = expanded
        .as_deref()
        .map(|home| PathBuf::from(home).join("bin/javac.exe"));
    let path_value = user
        .get("Path")
        .or_else(|| user.get("PATH"))
        .cloned()
        .unwrap_or_default();
    let path_java = find_in_configured_path("java", &path_value, &paths).map(display_path);
    let consumer_lower = consumer.to_ascii_lowercase();
    let startup_exists = if consumer_lower.contains("nacos") {
        root.join("bin").join("startup.cmd").is_file()
    } else if consumer_lower.contains("nexus") {
        root.join("bin").join("nexus.exe").is_file()
            || root.join("bin").join("nexus.bat").is_file()
            || root.join("bin").join("nexus").is_file()
    } else if consumer_lower.contains("maven") {
        root.join("pom.xml").is_file()
    } else if consumer_lower.contains("gradle") {
        root.join("build.gradle").is_file()
            || root.join("build.gradle.kts").is_file()
            || root.join("gradlew").is_file()
    } else {
        root.join("bin").join("startup.cmd").is_file()
            || fs::read_dir(root)
                .ok()
                .into_iter()
                .flatten()
                .flatten()
                .any(|entry| {
                    entry
                        .path()
                        .extension()
                        .and_then(OsStr::to_str)
                        .is_some_and(|ext| {
                            ext.eq_ignore_ascii_case("bat") || ext.eq_ignore_ascii_case("cmd")
                        })
                })
    };
    let indirect = raw.as_deref().is_some_and(|value| value.contains('%'));
    let process_differs = process.get("JAVA_HOME") != user.get("JAVA_HOME")
        || process.get("Path").or_else(|| process.get("PATH"))
            != user.get("Path").or_else(|| user.get("PATH"));
    let java_exists = java.as_deref().is_some_and(Path::is_file);
    let javac_exists = javac.as_deref().is_some_and(Path::is_file);
    let usable = startup_exists && java_exists && javac_exists && !indirect;
    let mut explanation = vec![
        format!("{consumer} 读取不到 Java 不一定是 JDK 没装。"),
        "常见原因包括 JAVA_HOME 间接引用、进程环境未刷新、服务仍使用旧环境、PATH 首个 java.exe 与 JAVA_HOME 不一致，或 JDK 缺少 javac.exe。".to_string(),
    ];
    if indirect {
        explanation.push("当前 JAVA_HOME 是间接引用，建议写入真实绝对路径。".to_string());
    }
    if process_differs {
        explanation
            .push("当前进程环境与最新用户环境不同，请重启终端、IDE 或相关服务。".to_string());
    }
    if !javac_exists {
        explanation.push("JAVA_HOME 缺少 javac.exe，可能不是完整 JDK。".to_string());
    }
    Ok(JavaConsumerReport {
        consumer: consumer.to_string(),
        root: display_path(root),
        startup_exists,
        java_home_raw: raw,
        java_home_expanded: expanded,
        java_exists,
        javac_exists,
        path_java,
        indirect_java_home_risk: indirect,
        process_user_env_differs: process_differs,
        usable,
        explanation,
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
        let user = user_environment()?;
        let java_home = select_java_home(&paths, &user)
            .map(|value| expand_environment_path(&value, &paths))
            .ok_or_else(|| {
                "Nacos 启动前没有找到同时包含 java.exe 与 javac.exe 的有效 JDK".to_string()
            })?;
        let java_exe = Path::new(&java_home).join("bin").join("java.exe");
        let java_version = first_output_line(&java_exe, &["-version"]);
        if java_version.is_empty() {
            return Err("Nacos 启动前 JDK 回读验证失败；请在环境页重新预览配置".to_string());
        }
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
            output: format!(
                "已使用最新用户环境中的 JAVA_HOME 后台启动 Nacos 单机模式：{} · {}",
                java_home, java_version
            ),
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
    Err(external_runtime_manual_action_message(
        &executable_path,
        &kind,
    ))
}

fn external_runtime_manual_action_message(executable: &Path, kind: &str) -> String {
    let normalized = executable.to_string_lossy().to_ascii_lowercase();
    if normalized.contains("\\jetbrains\\") || normalized.contains("\\jbr\\") {
        return "这是 IDE 内置运行时。DevEnv Manager 不会卸载或删除它；请先切换到 DevEnv 管理版本，并从 IDE 设置中取消使用该 JBR。".to_string();
    }
    if let Some(app) = package_name_from_path(executable, "scoop", "apps") {
        return format!(
            "路径属于 Scoop 应用 {app}。DevEnv Manager 不会调用包管理器卸载；请在终端自行运行 scoop uninstall {app}，或打开系统卸载入口。"
        );
    }
    if let Some(package) = package_name_from_path(executable, "chocolatey", "lib") {
        return format!(
            "路径属于 Chocolatey 包 {package}。DevEnv Manager 不会调用包管理器卸载；请在管理员终端自行运行 choco uninstall {package}，或打开系统卸载入口。"
        );
    }
    if let Some(entry) = find_uninstall_entry_for_path(executable, kind) {
        return format!(
            "已找到 Windows 卸载入口：{}。DevEnv Manager 不会直接启动卸载器；请点击“系统卸载入口”后由用户手动卸载。",
            entry.display_name
        );
    }
    if let Ok(root) = portable_runtime_root(executable, kind) {
        return format!(
            "疑似便携运行时：{}。DevEnv Manager 不会删除外部目录；请确认不再使用后由用户手动处理。",
            display_path(root)
        );
    }
    format!(
        "未识别到可由 DevEnv Manager 安全管理的 {} 卸载方式。请先切换到受管版本，再通过系统设置、原安装器或包管理器手动处理：{}",
        kind,
        display_path(executable)
    )
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
fn preview_project_configuration(project_path: String) -> Result<ProjectConfigPreview, String> {
    let root = PathBuf::from(project_path.trim());
    let analysis = analyze_project_blocking(&root)?;
    let installed = load_installed(&load_paths()?)?;
    let package_manager = analysis.package_manager.as_deref().unwrap_or("npm");
    let settings = json!({
        "terminal.integrated.defaultProfile.windows": "PowerShell",
        "python.defaultInterpreterPath": "${workspaceFolder}\\.venv\\Scripts\\python.exe",
        "java.configuration.updateBuildConfiguration": "interactive",
        "npm.packageManager": package_manager
    });
    let tasks = json!({
        "version": "2.0.0",
        "tasks": analysis.actions.iter()
            .filter(|action| action.safe_to_run && !matches!(action.id.as_str(), "vscode" | "copy_commands" | "nacos_start"))
            .map(|action| json!({
                "label": action.title,
                "type": "shell",
                "command": action.command,
                "problemMatcher": []
            }))
            .collect::<Vec<_>>()
    });
    let jdk = installed.current.jdk.as_deref().unwrap_or("17");
    let safe_jdk = jdk
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
        .collect::<String>();
    let proposals = [
        (
            ".vscode/settings.json",
            serde_json::to_string_pretty(&settings)
                .map_err(|error| format!("生成 VS Code 设置失败：{error}"))?,
        ),
        (
            ".vscode/tasks.json",
            serde_json::to_string_pretty(&tasks)
                .map_err(|error| format!("生成 VS Code 任务失败：{error}"))?,
        ),
        (
            ".idea/misc.xml",
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<project version=\"4\">\n  <component name=\"ProjectRootManager\" version=\"2\" languageLevel=\"JDK_{safe_jdk}\" project-jdk-name=\"{safe_jdk}\" project-jdk-type=\"JavaSDK\" />\n</project>\n"
            ),
        ),
        (
            ".idea/compiler.xml",
            format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<project version=\"4\">\n  <component name=\"CompilerConfiguration\">\n    <bytecodeTargetLevel target=\"{safe_jdk}\" />\n  </component>\n</project>\n"
            ),
        ),
    ];
    let files = proposals
        .into_iter()
        .map(|(relative_path, content)| ProjectConfigFileDraft {
            existed: root.join(relative_path).exists(),
            relative_path: relative_path.to_string(),
            content,
            enabled: true,
        })
        .collect();
    Ok(ProjectConfigPreview {
        project_path: display_path(&root),
        detected_types: analysis.project_types,
        files,
        current: installed.current,
        warnings: vec![
            "应用前可逐项编辑；仅允许写入固定的 .vscode/.idea 配置文件".to_string(),
            "已有文件会备份到项目内 .devenv-manager/backups/时间戳 目录".to_string(),
            "切换运行时前会自动保存一份时间命名的环境变量模板".to_string(),
        ],
    })
}

fn allowed_project_config(relative: &str) -> bool {
    matches!(
        relative.replace('\\', "/").as_str(),
        ".vscode/settings.json" | ".vscode/tasks.json" | ".idea/misc.xml" | ".idea/compiler.xml"
    )
}

fn restore_project_files(changes: &[(PathBuf, Option<PathBuf>)]) {
    for (target, backup) in changes.iter().rev() {
        if let Some(backup) = backup {
            let _ = fs::copy(backup, target);
        } else {
            let _ = fs::remove_file(target);
        }
    }
}

#[tauri::command]
fn apply_project_configuration(
    request: ProjectConfigApplyRequest,
) -> Result<OperationResult, String> {
    let root = PathBuf::from(request.project_path.trim());
    analyze_project_blocking(&root)?;
    if request.files.len() > 4 {
        return Err("项目配置文件数量超出安全限制".to_string());
    }
    let backup_stamp = filename_timestamp();
    let backup_root = root
        .join(".devenv-manager")
        .join("backups")
        .join(&backup_stamp);
    let enabled = request
        .files
        .iter()
        .filter(|file| file.enabled)
        .collect::<Vec<_>>();
    for file in &enabled {
        if !allowed_project_config(&file.relative_path) {
            return Err(format!("不允许写入该项目配置：{}", file.relative_path));
        }
        if file.content.len() > 64 * 1024 || file.content.chars().any(|ch| ch == '\0') {
            return Err(format!("项目配置内容无效或过大：{}", file.relative_path));
        }
        let target = root.join(file.relative_path.replace('/', "\\"));
        if fs::symlink_metadata(&target)
            .ok()
            .is_some_and(|metadata| metadata.file_type().is_symlink())
        {
            return Err(format!("拒绝写入符号链接：{}", file.relative_path));
        }
        if target
            .parent()
            .and_then(|parent| fs::symlink_metadata(parent).ok())
            .is_some_and(|metadata| metadata.file_type().is_symlink())
        {
            return Err(format!("拒绝写入符号链接目录：{}", file.relative_path));
        }
    }
    let has_switches = [
        &request.switches.jdk,
        &request.switches.python,
        &request.switches.node,
        &request.switches.maven,
        &request.switches.gradle,
        &request.switches.go,
    ]
    .iter()
    .any(|value| value.is_some());
    let env_backup = if has_switches {
        let name = format!("自动备份 项目切换 {backup_stamp}");
        save_config_profile_blocking(name.clone())?;
        load_profiles(&load_paths()?)?
            .into_iter()
            .find(|profile| profile.name == name)
    } else {
        None
    };
    let mut changes = Vec::new();
    for file in enabled {
        let relative = file.relative_path.replace('\\', "/");
        let target = root.join(&relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("创建项目配置目录失败：{error}"))?;
        }
        let backup = if target.exists() {
            let backup = backup_root.join(&relative);
            if let Some(parent) = backup.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("创建项目配置备份目录失败：{error}"))?;
            }
            fs::copy(&target, &backup)
                .map_err(|error| format!("备份 {} 失败：{error}", file.relative_path))?;
            Some(backup)
        } else {
            None
        };
        if let Err(error) = fs::write(&target, &file.content) {
            changes.push((target, backup));
            restore_project_files(&changes);
            return Err(format!("写入 {} 失败：{error}", file.relative_path));
        }
        changes.push((target, backup));
    }
    let switches = [
        ("jdk", request.switches.jdk),
        ("python", request.switches.python),
        ("node", request.switches.node),
        ("maven", request.switches.maven),
        ("gradle", request.switches.gradle),
        ("go", request.switches.go),
    ];
    for (kind, version) in switches {
        if let Some(version) = version {
            if let Err(error) = switch_runtime_blocking(kind.to_string(), version, None) {
                restore_project_files(&changes);
                if let Some(profile) = &env_backup {
                    let _ = apply_config_profile_blocking(profile.id.clone());
                }
                return Err(format!("项目运行时切换失败，已尝试恢复：{error}"));
            }
        }
    }
    Ok(OperationResult {
        success: true,
        message: format!(
            "项目配置已应用；文件备份：{}{}",
            display_path(backup_root),
            env_backup
                .map(|profile| format!("；环境模板：{}", profile.name))
                .unwrap_or_default()
        ),
    })
}

#[tauri::command]
fn generate_vscode_config(project_path: String) -> Result<OperationResult, String> {
    let preview = preview_project_configuration(project_path.clone())?;
    apply_project_configuration(ProjectConfigApplyRequest {
        project_path,
        files: preview
            .files
            .into_iter()
            .filter(|file| file.relative_path.starts_with(".vscode/"))
            .collect(),
        switches: CurrentVersions::default(),
    })
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            app_snapshot,
            storage_cleanup_architecture,
            scan_storage_cleanup,
            scan_cleanup_targets,
            inspect_maintenance_overview,
            inspect_disk_overview,
            create_cleanup_plan,
            clean_selected_targets,
            clean_managed_download_cache,
            clean_dev_cache,
            export_cleanup_report,
            scan_large_files,
            scan_duplicate_large_files,
            cancel_maintenance_scan,
            inspect_downloads,
            inspect_desktop,
            inspect_app_usage,
            inspect_installed_software_usage,
            inspect_env_reliability,
            create_env_repair_plan,
            apply_env_repair_plan,
            verify_env_after_apply,
            rollback_env_repair,
            export_env_reliability_report,
            create_java_stabilize_plan,
            apply_java_stabilize_plan,
            verify_java_toolchain,
            verify_nacos_java_environment,
            verify_nexus_java_environment,
            verify_java_consumer_environment,
            verify_maven_gradle_with_current_jdk,
            inspect_python_integrity,
            create_managed_python_pip_repair_plan,
            apply_managed_python_pip_repair,
            inspect_runtime_strong_verification,
            validate_directory_path,
            inspect_idea_project,
            repair_maven_gradle_registration,
            list_env_backups,
            inspect_env_backup,
            restore_env_backup,
            safety_disclaimer,
            feature_risk_registry,
            get_feature_risk,
            create_confirmation_token,
            accept_safety_disclaimer,
            reset_ui_config,
            open_app_config_dir,
            create_move_plan,
            execute_move_plan,
            list_rollback_records,
            rollback_move,
            create_junction_bridge,
            create_desktop_archive_plan,
            execute_desktop_archive_plan,
            create_downloads_archive_plan,
            execute_downloads_archive_plan,
            inspect_partition_layout,
            create_c_drive_expansion_plan,
            execute_c_drive_expansion,
            open_analysis_path,
            open_apps_features,
            open_python_alias_settings,
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
            preview_user_environment_configuration,
            apply_user_environment_configuration,
            list_environment_backups,
            restore_environment_backup,
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
            export_python_diagnostic_report,
            preview_python_repair,
            apply_python_repair,
            inspect_toolchains,
            run_toolchain_action,
            inspect_platform_toolchains,
            run_chsrc_action,
            run_platform_action,
            inspect_system_platforms,
            manage_system_platform,
            inspect_local_services,
            inspect_mysql_repair,
            create_mysql_repair_plan,
            mysql_pending_execution_guard,
            execute_mysql_repair_plan,
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
            add_archive_plan_item,
            list_archive_plan_items,
            remove_archive_plan_item,
            clear_download_cache,
            inspect_command_safety,
            run_tool_command,
            run_learning_check,
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
            preview_project_configuration,
            apply_project_configuration,
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
                    "扫描到 {} 个统计项，共 {}。\nCLI cleanup scan 始终只读；清理计划必须在 GUI 中预览并确认。",
                    report.total_items,
                    format_byte_size(report.total_bytes)
                ))
            }
        }
        "env" if args.get(1).map(String::as_str) == Some("inspect") => {
            let paths = load_paths()?;
            let snapshot = env_core::inspect_env_reliability(&paths.root);
            if args.iter().any(|item| item == "--json") {
                serde_json::to_string_pretty(&snapshot)
                    .map_err(|err| format!("生成 JSON 失败：{err}"))
            } else {
                Ok(format!(
                    "环境可靠性：Java={}，PATH {} 项，问题 {} 个\n提示：当前进程环境和新终端用户环境可能不同，修改后请重启终端/IDE。",
                    snapshot.java.consistency,
                    snapshot.path_analysis.total_entries,
                    snapshot.issues.len()
                ))
            }
        }
        "env"
            if args.get(1).map(String::as_str) == Some("plan")
                && args.get(2).map(String::as_str) == Some("java") =>
        {
            let jdk = args
                .iter()
                .position(|item| item == "--jdk")
                .and_then(|index| args.get(index + 1))
                .ok_or_else(|| "用法：devenv env plan java --jdk <JDK根目录>".to_string())?;
            let paths = load_paths()?;
            let plan = env_core::create_java_stabilize_plan(&paths.root, jdk.clone())?;
            serde_json::to_string_pretty(&plan).map_err(|err| format!("生成计划失败：{err}"))
        }
        "env" if args.get(1).map(String::as_str) == Some("apply") => {
            if !args.iter().any(|item| item == "--confirm-risk") {
                return Err(
                    "执行环境修复计划需要 --confirm-risk。请先确认 diff、备份和风险说明。"
                        .to_string(),
                );
            }
            let plan_id = args
                .get(2)
                .ok_or_else(|| "用法：devenv env apply <plan-id> --confirm-risk".to_string())?;
            let paths = load_paths()?;
            let plan = env_core::plan::load_plan(plan_id)?;
            let result = env_core::apply_env_repair_plan(&paths.root, plan);
            serde_json::to_string_pretty(&result).map_err(|err| format!("生成结果失败：{err}"))
        }
        "env" if args.get(1).map(String::as_str) == Some("verify") => {
            let paths = load_paths()?;
            let report = env_core::verify_env_after_apply(&paths.root, "cli".to_string());
            serde_json::to_string_pretty(&report).map_err(|err| format!("生成验证报告失败：{err}"))
        }
        "env" if args.get(1).map(String::as_str) == Some("backups") => {
            serde_json::to_string_pretty(&env_core::list_env_backups())
                .map_err(|err| format!("生成备份列表失败：{err}"))
        }
        "env" if args.get(1).map(String::as_str) == Some("restore") => {
            if !args.iter().any(|item| item == "--confirm-risk") {
                return Err(
                    "恢复环境备份需要 --confirm-risk。恢复前会创建当前状态备份。".to_string(),
                );
            }
            let backup = args.get(2).ok_or_else(|| {
                "用法：devenv env restore <backup-name> --confirm-risk".to_string()
            })?;
            let result = env_core::restore_env_backup(backup.clone())?;
            serde_json::to_string_pretty(&result).map_err(|err| format!("生成恢复结果失败：{err}"))
        }
        "java" if args.get(1).map(String::as_str) == Some("verify") => {
            let paths = load_paths()?;
            serde_json::to_string_pretty(&env_core::verify_java_toolchain(&paths.root))
                .map_err(|err| format!("生成 Java 验证失败：{err}"))
        }
        "python" if args.get(1).map(String::as_str) == Some("verify") => {
            let paths = load_paths()?;
            let snapshot = env_core::inspect_env_reliability(&paths.root);
            let integrity = resolve_tool(&paths, "python")
                .map(|python| python_integrity_for_path(&python, &paths));
            serde_json::to_string_pretty(&json!({
                "environment": snapshot.python,
                "integrity": integrity,
            }))
            .map_err(|err| format!("生成 Python 验证失败：{err}"))
        }
        "nacos" if args.get(1).map(String::as_str) == Some("verify") => {
            let root = args
                .get(2)
                .ok_or_else(|| "用法：devenv nacos verify <nacos-root>".to_string())?;
            let paths = load_paths()?;
            serde_json::to_string_pretty(&env_core::verify_nacos_java_environment(
                &paths.root,
                root.clone(),
            ))
            .map_err(|err| format!("生成 Nacos 验证失败：{err}"))
        }
        "safety" if args.get(1).map(String::as_str) == Some("disclaimer") => {
            Ok(safety::disclaimer_text())
        }
        "safety" if args.get(1).map(String::as_str) == Some("risks") => {
            serde_json::to_string_pretty(&safety::feature_risk_registry())
                .map_err(|err| format!("生成风险表失败：{err}"))
        }
        "db" if args.get(1).map(String::as_str) == Some("doctor")
            && args.get(2).map(String::as_str) == Some("mysql") =>
        {
            serde_json::to_string_pretty(&mysql_repair::inspect())
                .map_err(|error| format!("生成 MySQL 诊断 JSON 失败：{error}"))
        }
        "db" if args.get(1).map(String::as_str) == Some("repair-plan")
            && args.get(2).map(String::as_str) == Some("mysql") =>
        {
            let candidate = args.get(3).ok_or_else(|| {
                "用法：devenv db repair-plan mysql <candidate-id> <action>".to_string()
            })?;
            let action = args.get(4).ok_or_else(|| "缺少修复动作".to_string())?;
            let plan = mysql_repair::create_plan(candidate.clone(), action.clone())?;
            serde_json::to_string_pretty(&plan)
                .map_err(|error| format!("生成 MySQL 修复计划失败：{error}"))
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
        "DevEnv Manager CLI {}\n\n用法：\n  devenv doctor [--json]\n  devenv env inspect [--json]\n  devenv env plan java --jdk <JDK根目录>\n  devenv env apply <plan-id> --confirm-risk\n  devenv env verify\n  devenv env backups\n  devenv env restore <backup-name> --confirm-risk\n  devenv java verify\n  devenv python verify\n  devenv nacos verify <nacos-root>\n  devenv safety disclaimer\n  devenv safety risks\n  devenv list [--json]\n  devenv use <kind> <version>\n  devenv project check [path] [--json]\n  devenv cleanup scan [--json]\n  devenv db doctor mysql --json\n  devenv db repair-plan mysql <candidate-id> <action>\n  devenv profile list\n  devenv profile apply <id>\n  devenv version",
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
    if raw.contains('\0') || raw.chars().any(char::is_control) {
        return Err("根目录不能包含控制字符".to_string());
    }
    let expanded = PathBuf::from(raw)
        .expand_home()
        .unwrap_or_else(|| PathBuf::from(raw));
    let resolved = expanded.canonicalize().unwrap_or(expanded);
    validate_root_dir_choice(&resolved)?;
    if is_drive_root(&resolved) {
        Ok(resolved.join(APP_NAME))
    } else {
        Ok(resolved)
    }
}

fn validate_root_dir_choice(root: &Path) -> Result<(), String> {
    let value = path_key(&display_path(root));
    if value.is_empty() {
        return Err("根目录无效".to_string());
    }
    let blocked = [
        r"c:\windows",
        r"c:\program files",
        r"c:\program files (x86)",
        r"c:\programdata",
        r"c:\users",
        r"c:\users\public",
        r"c:\system volume information",
    ];
    if blocked
        .iter()
        .any(|item| value == *item || value.starts_with(&format!("{item}\\")))
    {
        return Err(
            "根目录不能放在 Windows、Program Files、ProgramData 或用户根目录等敏感位置".to_string(),
        );
    }
    if let Some(home) = dirs::home_dir() {
        for child in [
            "Desktop",
            "Downloads",
            "Documents",
            "Pictures",
            "Videos",
            "Music",
        ] {
            let sensitive = path_key(&display_path(home.join(child)));
            if !sensitive.is_empty()
                && (value == sensitive || value.starts_with(&format!("{sensitive}\\")))
            {
                return Err(
                    "根目录不能放在桌面、下载、文档、图片、视频或音乐等个人数据目录".to_string(),
                );
            }
        }
    }
    Ok(())
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
        safety_disclaimer_accepted: false,
        safety_disclaimer_version: 0,
        safety_disclaimer_accepted_at: None,
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
    let file_name = path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("config.json");
    let temp = path.with_file_name(format!(
        "{file_name}.{}.{}.tmp",
        std::process::id(),
        SAVE_JSON_COUNTER.fetch_add(1, Ordering::Relaxed)
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
        return Some(display_path(managed));
    }
    if let Some(value) = user_environment.get("JAVA_HOME") {
        if is_valid_java_home(value, paths) {
            return Some(expand_environment_path(value, paths));
        }
    }
    if let Ok(value) = env::var("JAVA_HOME") {
        if is_valid_java_home(&value, paths) {
            return Some(expand_environment_path(&value, paths));
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

fn normalized_absolute(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn is_path_inside(path: &Path, root: &Path) -> bool {
    let candidate_key = path_key(&display_path(normalized_absolute(path)));
    let root_key = path_key(&display_path(normalized_absolute(root)));
    candidate_key == root_key || candidate_key.starts_with(&format!("{root_key}\\"))
}

fn first_output_line(executable: &Path, args: &[&str]) -> String {
    first_meaningful_output_line(
        &run_command_output(executable.to_path_buf(), args, 30).unwrap_or_default(),
    )
    .unwrap_or_default()
}

fn first_meaningful_output_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| {
            !line.is_empty()
                && !line
                    .chars()
                    .all(|ch| ch == '-' || ch == '=' || ch.is_whitespace())
        })
        .map(str::to_string)
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
        .map(|mut item| {
            item.source = classify_jdk_candidate_source(&item.executable);
            item
        })
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
    let gradle_output = resolve_tool(&paths, "gradle")
        .map(|path| command_value(Some(path), &["-version"]))
        .unwrap_or_default();
    let gradle_runtime = gradle_output
        .lines()
        .filter(|line| line.contains("JVM") || line.contains("Java"))
        .take(2)
        .collect::<Vec<_>>()
        .join(" · ");
    let gradle_runtime = if gradle_runtime.trim().is_empty() {
        first_meaningful_output_line(&gradle_output).unwrap_or_default()
    } else {
        gradle_runtime
    };
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

fn classify_jdk_candidate_source(path: &str) -> String {
    let lower = path.to_ascii_lowercase();
    if lower.contains("\\devenvmanager\\") {
        "Managed".to_string()
    } else if lower.contains("\\jetbrains\\") || lower.contains("\\jbr\\") {
        "IdeBundled".to_string()
    } else if lower.contains("\\scoop\\") {
        "Scoop".to_string()
    } else if lower.contains("\\chocolatey\\") {
        "Chocolatey".to_string()
    } else if lower.contains("\\mise\\") {
        "Mise".to_string()
    } else if lower.contains("\\asdf\\") {
        "Asdf".to_string()
    } else if lower.contains("\\program files\\") || lower.contains("\\program files (x86)\\") {
        "SystemInstaller".to_string()
    } else if lower.contains("\\java\\") || lower.contains("\\jdk") {
        "External".to_string()
    } else {
        "Unknown".to_string()
    }
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
    let response: Value = reqwest::blocking::get(&url)
        .map_err(|err| format!("查询 BellSoft Liberica 失败：{err}"))?
        .error_for_status()
        .map_err(|err| format!("查询 BellSoft Liberica 失败：{err}"))?
        .json()
        .map_err(|err| format!("解析 BellSoft Liberica 响应失败：{err}"))?;
    let package = response
        .as_array()
        .and_then(|items| items.first())
        .or_else(|| {
            response
                .get("releases")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
        })
        .unwrap_or(&response);
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
        .user_agent("DevEnvManager/1.3")
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
    parse_sha256_for_file(&text, &release.name)
        .map(Some)
        .ok_or_else(|| "Node.js 校验文件没有目标文件的有效 SHA-256".to_string())
}

fn parse_sha256_for_file(text: &str, expected_name: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let sha = parts.next()?;
        let name = parts.next()?.trim_start_matches('*');
        if name == expected_name
            && sha.len() == 64
            && sha.chars().all(|character| character.is_ascii_hexdigit())
        {
            Some(sha.to_ascii_lowercase())
        } else {
            None
        }
    })
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
    validate_download_url_value(&parsed)
}

fn validate_download_url_value(parsed: &reqwest::Url) -> Result<(), String> {
    let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();
    if parsed.scheme() != "https" || !ALLOWED_DOWNLOAD_HOSTS.contains(&host.as_str()) {
        return Err(format!("下载地址不在安全白名单中：{parsed}"));
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
    if let Some(expected) = expected_sha256 {
        validate_update_checksum(expected)
            .map_err(|_| "下载元数据中的 SHA-256 格式无效，已拒绝下载".to_string())?;
    }
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
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 10 {
                return attempt.error("下载重定向次数过多");
            }
            if validate_download_url_value(attempt.url()).is_ok() {
                attempt.follow()
            } else {
                attempt.error("下载重定向地址不在安全白名单中")
            }
        }))
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
    let version = first_meaningful_output_line(&text).unwrap_or_else(|| "unknown".to_string());
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

#[derive(Debug, Clone)]
struct PortSignature {
    identity: String,
    confidence: u8,
    evidence: Vec<String>,
    conflict_evidence: Vec<String>,
    risk: String,
    risk_level: String,
    recommendation: String,
    explanation: String,
}

fn analyze_port_signature(
    port: u16,
    state: &str,
    process_name: &str,
    process_path: &str,
    command_line: &str,
    service_names: &[String],
) -> PortSignature {
    let lower_name = process_name.to_ascii_lowercase();
    let haystack = format!(
        "{} {} {} {}",
        lower_name,
        process_path.to_ascii_lowercase(),
        command_line.to_ascii_lowercase(),
        service_names.join(" ").to_ascii_lowercase()
    );
    let mut evidence = Vec::new();
    let mut conflict = Vec::new();
    let mut identity = "未识别的本地服务".to_string();
    let mut score = 0_i32;

    let signatures: &[(&str, &[&str], &str)] = &[
        (
            "Spring Boot",
            &[
                "spring-boot",
                "org.springframework.boot",
                "bootrun",
                "springapplication",
            ],
            "Java / JVM",
        ),
        (
            "Tomcat",
            &["tomcat", "catalina", "org.apache.catalina"],
            "Java / JVM",
        ),
        ("Jetty", &["jetty", "org.eclipse.jetty"], "Java / JVM"),
        ("Undertow", &["undertow", "io.undertow"], "Java / JVM"),
        ("Nacos", &["nacos", "com.alibaba.nacos"], "Java / JVM"),
        ("Sentinel", &["sentinel", "csp.sentinel"], "Java / JVM"),
        ("Seata", &["seata"], "Java / JVM"),
        ("Eureka", &["eureka"], "Java / JVM"),
        ("Jenkins", &["jenkins"], "Java / JVM"),
        ("Nexus", &["nexus", "sonatype"], "Java / JVM"),
        ("SonarQube", &["sonarqube", "sonar"], "Java / JVM"),
        (
            "Java / JVM",
            &["java.exe", "\\jdk", "\\jre", "java -jar"],
            "Java / JVM",
        ),
        ("Maven", &["mvn.cmd", "maven"], "Java / JVM"),
        ("Gradle", &["gradle", "gradlew"], "Java / JVM"),
        (
            "Node.js",
            &["node.exe", "\\nodejs\\", "npm", "pnpm", "yarn", "bun"],
            "Node / 前端",
        ),
        ("Vite", &["vite", "vite.config"], "Node / 前端"),
        (
            "Webpack Dev Server",
            &["webpack-dev-server", "webpack serve"],
            "Node / 前端",
        ),
        ("Next.js", &["next dev", "next-server"], "Node / 前端"),
        ("Nuxt", &["nuxt", "nuxi"], "Node / 前端"),
        ("React Scripts", &["react-scripts"], "Node / 前端"),
        ("Vue CLI", &["vue-cli-service"], "Node / 前端"),
        ("Angular", &["ng serve", "@angular/cli"], "Node / 前端"),
        ("Storybook", &["storybook"], "Node / 前端"),
        ("NestJS", &["nestjs", "@nestjs"], "Node / 前端"),
        ("Electron Dev", &["electron", "electron.exe"], "Node / 前端"),
        ("Tauri Dev", &["tauri dev", "tauri-cli"], "Node / 前端"),
        (
            "Python",
            &[
                "python.exe",
                "\\python",
                "uvicorn",
                "gunicorn",
                "flask",
                "django",
            ],
            "Python / AI",
        ),
        ("FastAPI / Uvicorn", &["fastapi", "uvicorn"], "Python / AI"),
        (
            "Jupyter",
            &["jupyter", "ipykernel", "notebook"],
            "Python / AI",
        ),
        ("Streamlit", &["streamlit"], "Python / AI"),
        ("Gradio", &["gradio"], "Python / AI"),
        ("ComfyUI", &["comfyui"], "Python / AI"),
        (
            "Stable Diffusion WebUI",
            &["stable-diffusion-webui", "webui-user"],
            "Python / AI",
        ),
        ("Ollama", &["ollama"], "Python / AI"),
        ("LM Studio", &["lm studio", "lmstudio"], "Python / AI"),
        ("vLLM", &["vllm"], "Python / AI"),
        ("Go", &["go.exe", "\\go\\bin", ".go"], "Go / Rust / .NET"),
        (
            "Rust",
            &["cargo.exe", "target\\debug", "target\\release"],
            "Go / Rust / .NET",
        ),
        (
            ".NET",
            &["dotnet.exe", "iisexpress", "kestrel"],
            "Go / Rust / .NET",
        ),
        ("PHP", &["php.exe", "php-cgi", "phpstudy"], "PHP / Ruby"),
        ("Ruby", &["ruby.exe", "rails", "puma"], "PHP / Ruby"),
        ("MySQL / MariaDB", &["mysqld", "mysql", "mariadb"], "数据库"),
        ("PostgreSQL", &["postgres", "postmaster"], "数据库"),
        ("Redis", &["redis-server"], "数据库"),
        ("MongoDB", &["mongod"], "数据库"),
        ("Elasticsearch", &["elasticsearch"], "数据库"),
        ("OpenSearch", &["opensearch"], "数据库"),
        ("SQL Server", &["sqlservr", "mssql"], "数据库"),
        ("Oracle", &["oracle", "tnslsnr"], "数据库"),
        ("Nginx", &["nginx.exe", "\\nginx\\"], "Web 服务器"),
        ("Apache HTTPD", &["httpd.exe", "apache"], "Web 服务器"),
        ("RabbitMQ", &["rabbitmq", "beam.smp"], "中间件"),
        ("Kafka", &["kafka"], "中间件"),
        ("ZooKeeper", &["zookeeper"], "中间件"),
        ("MinIO", &["minio"], "中间件"),
        ("Prometheus", &["prometheus"], "中间件"),
        ("Grafana", &["grafana"], "中间件"),
        (
            "Docker / Container",
            &["docker", "com.docker", "com.docker.backend", "containerd"],
            "Docker / WSL",
        ),
        (
            "WSL",
            &["wsl", "wslhost", "vmmem", "\\wsl$"],
            "Docker / WSL",
        ),
        (
            "本地代理",
            &[
                "clash", "mihomo", "v2ray", "xray", "sing-box", "privoxy", "fiddler", "charles",
            ],
            "本地代理",
        ),
        (
            "Node Inspector",
            &["--inspect", "inspector"],
            "IDE / 调试器",
        ),
        ("Java JDWP", &["jdwp", "address=*:"], "IDE / 调试器"),
        ("Python debugpy", &["debugpy"], "IDE / 调试器"),
        (
            "IDE / 调试器",
            &[
                "idea64",
                "pycharm",
                "webstorm",
                "code.exe",
                "cursor.exe",
                "trae.exe",
                "debug",
            ],
            "IDE / 调试器",
        ),
        (
            "桌面应用",
            &[
                "steam.exe",
                "steamwebhelper.exe",
                "qq.exe",
                "wechat",
                "weixin",
                "wxwork",
                "chrome.exe",
                "msedge.exe",
                "firefox.exe",
                "discord.exe",
                "telegram.exe",
                "onedrive.exe",
                "baidunetdisk",
                "baiduyunguanjia",
                "cursor.exe",
                "trae.exe",
                "code.exe",
                "wechat.exe",
                "webview",
            ],
            "桌面应用",
        ),
    ];
    let is_generic_signature = |label: &str| {
        matches!(
            label,
            "Java / JVM" | "Node.js" | "Python" | "Go" | "Rust" | ".NET"
        )
    };
    for (label, markers, group) in signatures {
        let hits = markers
            .iter()
            .filter(|marker| haystack.contains(&marker.to_ascii_lowercase()))
            .count();
        if hits > 0 {
            score += (hits as i32) * 25;
            if identity == "未识别的本地服务"
                || (!is_generic_signature(label) && is_generic_signature(&identity))
                || matches!(*label, "桌面应用" | "IDE / 调试器")
            {
                identity = (*label).to_string();
            }
            evidence.push(format!(
                "{group} 强证据：{label} 命中 {hits} 个进程/路径/命令行标记"
            ));
        }
    }

    if !state.eq_ignore_ascii_case("LISTENING") {
        conflict.push(format!("{state} 不是本地监听状态，不能当作正在提供服务"));
        score -= 25;
    }
    if matches!(
        lower_name.as_str(),
        "chrome.exe" | "msedge.exe" | "firefox.exe"
    ) && (port == 9222 || haystack.contains("remote-debugging-port"))
    {
        let browser = match lower_name.as_str() {
            "msedge.exe" => "Edge 调试端口",
            "firefox.exe" => "Firefox 调试端口",
            _ => "Chrome 调试端口",
        };
        identity = browser.to_string();
        score += 60;
        evidence.push(format!("{browser} 强证据：浏览器进程使用远程调试端口"));
    }

    if [
        "steam.exe",
        "steamwebhelper.exe",
        "qq.exe",
        "wechat.exe",
        "weixin.exe",
        "wxwork.exe",
        "chrome.exe",
        "msedge.exe",
        "firefox.exe",
        "code.exe",
        "cursor.exe",
        "trae.exe",
    ]
    .iter()
    .any(|name| lower_name == *name)
    {
        conflict
            .push("桌面/浏览器/IDE 进程只按实际进程识别，不按端口号猜测为 Web 框架".to_string());
        score -= 20;
    }
    let port_hint = match port {
        80 => Some("HTTP Web 服务"),
        443 => Some("HTTPS Web 服务"),
        1433 => Some("SQL Server"),
        3000 | 4173 | 5173 | 5174 => Some("前端开发服务"),
        3306 => Some("MySQL"),
        5432 => Some("PostgreSQL"),
        6379 => Some("Redis"),
        8005 | 8009 | 8443 => Some("Tomcat"),
        8000 => Some("常见 Web 开发服务"),
        8080..=8082 => Some("Spring Boot / Tomcat / Web 服务"),
        8761 => Some("Spring Cloud Eureka"),
        8888 => Some("Jupyter / Spring Config"),
        9200 => Some("Elasticsearch"),
        27017 => Some("MongoDB"),
        _ => None,
    };
    if let Some(hint) = port_hint {
        evidence.push(format!("弱证据：端口 {port} 常见于 {hint}"));
        let ambiguous_web_port = matches!(port, 80 | 443 | 8000 | 8080..=8082 | 8888);
        if identity == "未识别的本地服务"
            && state.eq_ignore_ascii_case("LISTENING")
            && !ambiguous_web_port
        {
            identity = format!("{hint}（仅端口弱证据）");
        }
        score += 8;
    }

    let confidence = score.clamp(0, 100) as u8;
    let unknown_or_weak = identity == "未识别的本地服务" || identity.contains("仅端口弱证据");
    let risk_level = if !state.eq_ignore_ascii_case("LISTENING") {
        "low"
    } else if matches!(port, 3306 | 5432 | 6379 | 27017 | 9200 | 1433) {
        "high"
    } else if unknown_or_weak && matches!(port, 80 | 443 | 8000 | 8080..=8082 | 8888) {
        "low"
    } else if matches!(port, 80 | 443 | 8080..=8082 | 8000 | 8888) {
        "medium"
    } else {
        "low"
    }
    .to_string();
    let risk = match risk_level.as_str() {
        "high" => "敏感服务",
        "medium" => "需确认",
        _ => "普通",
    }
    .to_string();
    let recommendation = if !state.eq_ignore_ascii_case("LISTENING") {
        "这是已有连接记录，优先确认远端地址，不建议结束进程。"
    } else if confidence < 40 {
        "识别证据不足，先打开进程位置或查看详情再操作。"
    } else if risk_level == "high" {
        "疑似数据库/中间件等敏感服务，结束前先确认项目、备份和连接用户。"
    } else {
        "如确认对应项目已停止使用，可从详情中执行安全结束。"
    }
    .to_string();
    let explanation = format!(
        "{identity}；置信度 {confidence}%；强/弱证据 {} 条，冲突证据 {} 条。",
        evidence.len(),
        conflict.len()
    );
    PortSignature {
        identity,
        confidence,
        evidence,
        conflict_evidence: conflict,
        risk,
        risk_level,
        recommendation,
        explanation,
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
                process_path: redact_report_text(&record.process_path),
                identity: record.identity.clone(),
                risk_level: record.risk_level.clone(),
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
    let user = user_environment().unwrap_or_default();
    let selected_java = select_java_home(paths, &user)
        .map(|value| expand_environment_path(&value, paths))
        .filter(|value| {
            Path::new(value).join("bin/java.exe").is_file()
                && Path::new(value).join("bin/javac.exe").is_file()
        });
    if let Some(java_home) = &selected_java {
        command.env("JAVA_HOME", java_home);
    }

    let latest_user_path = user
        .get("Path")
        .or_else(|| user.get("PATH"))
        .cloned()
        .unwrap_or_default();
    let inherited_path = env::var("PATH").unwrap_or_default();
    let current_path = [latest_user_path, inherited_path]
        .into_iter()
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>()
        .join(";");
    let mut entries = Vec::new();
    if let Some(java_home) = &selected_java {
        entries.push(display_path(Path::new(java_home).join("bin")));
    }
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
        let value = display_path(item);
        if !entries
            .iter()
            .any(|existing| path_key(existing) == path_key(&value))
        {
            entries.push(value);
        }
    }
    for item in current_path
        .split(';')
        .map(|item| expand_environment_path(item, paths))
        .filter(|item| !item.trim().is_empty())
    {
        if !entries
            .iter()
            .any(|existing| path_key(existing) == path_key(&item))
        {
            entries.push(item);
        }
    }
    command.env("PATH", entries.join(";"));
}

pub(crate) fn command_text(stdout: &[u8], stderr: &[u8]) -> String {
    let stdout = decode_command_stream(stdout).trim().to_string();
    let stderr = decode_command_stream(stderr).trim().to_string();
    [stdout, stderr]
        .into_iter()
        .filter(|item| !item.is_empty())
        .collect::<Vec<String>>()
        .join("\n")
}

pub(crate) fn decode_command_stream(bytes: &[u8]) -> String {
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
    } else if let Ok(value) = std::str::from_utf8(bytes) {
        value.to_string()
    } else {
        decode_windows_ansi(bytes).unwrap_or_else(|| String::from_utf8_lossy(bytes).to_string())
    }
}

#[cfg(windows)]
fn decode_windows_ansi(bytes: &[u8]) -> Option<String> {
    #[link(name = "kernel32")]
    extern "system" {
        fn MultiByteToWideChar(
            code_page: u32,
            flags: u32,
            source: *const u8,
            source_len: i32,
            target: *mut u16,
            target_len: i32,
        ) -> i32;
    }
    if bytes.is_empty() || bytes.len() > i32::MAX as usize {
        return Some(String::new());
    }
    let required = unsafe {
        MultiByteToWideChar(
            0,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            std::ptr::null_mut(),
            0,
        )
    };
    if required <= 0 {
        return None;
    }
    let mut words = vec![0_u16; required as usize];
    let written = unsafe {
        MultiByteToWideChar(
            0,
            0,
            bytes.as_ptr(),
            bytes.len() as i32,
            words.as_mut_ptr(),
            required,
        )
    };
    (written > 0).then(|| String::from_utf16_lossy(&words[..written as usize]))
}

#[cfg(not(windows))]
fn decode_windows_ansi(_bytes: &[u8]) -> Option<String> {
    None
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
    let detected = load_paths()
        .ok()
        .and_then(|paths| resolve_tool(&paths, executable))
        .and_then(|path| {
            let source = Some(classify_source(&display_path(&path)));
            detect_runtime_at(name, &path, args, source)
        })
        .or_else(|| detect_runtime(name, executable, args));
    match detected {
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
            let version =
                first_meaningful_output_line(&detail).unwrap_or_else(|| "未返回版本".to_string());
            ToolState {
                name: name.to_string(),
                installed: output.status.success(),
                version,
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
    redact_sensitive_text(text)
}

fn redact_sensitive_text(text: &str) -> String {
    let mut result = text.to_string();
    for key in ["USERPROFILE", "HOME"] {
        if let Ok(value) = env::var(key) {
            if !value.trim().is_empty() {
                result = result.replace(&value, "%USERPROFILE%");
            }
        }
    }
    result = redact_path(&result);
    result
        .lines()
        .map(|line| {
            let line = redact_bearer_token(line);
            line.split(' ')
                .map(|part| {
                    let lower = part.to_ascii_lowercase();
                    if lower == "bearer" {
                        return "Bearer".to_string();
                    }
                    if lower.starts_with("bearer ") || lower.starts_with("authorization:bearer") {
                        return "Bearer <redacted>".to_string();
                    }
                    for marker in [
                        "token=",
                        "password=",
                        "passwd=",
                        "pwd=",
                        "secret=",
                        "apikey=",
                        "api_key=",
                        "access_key=",
                        "private_key=",
                        "--token=",
                        "--password=",
                    ] {
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

fn redact_bearer_token(line: &str) -> String {
    let lower = line.to_ascii_lowercase();
    let Some(index) = lower.find("bearer ") else {
        return line.to_string();
    };
    let token_start = index + "bearer ".len();
    let token_tail = &line[token_start..];
    let token_end = token_tail
        .find([' ', '\t', '\r', '\n', '"', '\''])
        .unwrap_or(token_tail.len());
    format!(
        "{}Bearer <redacted>{}",
        &line[..index],
        &token_tail[token_end..]
    )
}

fn redact_path(path: &str) -> String {
    redact_windows_user_paths(path)
}

fn redact_command_line(command_line: &str) -> String {
    redact_sensitive_text(command_line)
}

fn redact_json_value(value: &mut Value) {
    match value {
        Value::String(text) => *text = redact_sensitive_text(text),
        Value::Array(items) => items.iter_mut().for_each(redact_json_value),
        Value::Object(map) => {
            for (key, value) in map.iter_mut() {
                let lower = key.to_ascii_lowercase();
                if [
                    "password",
                    "passwd",
                    "pwd",
                    "token",
                    "secret",
                    "apikey",
                    "api_key",
                    "access_key",
                    "private_key",
                    "authorization",
                ]
                .iter()
                .any(|marker| lower.contains(marker))
                {
                    *value = Value::String("<redacted>".to_string());
                } else {
                    redact_json_value(value);
                }
            }
        }
        _ => {}
    }
}

fn redact_windows_user_paths(text: &str) -> String {
    let mut result = String::new();
    let mut rest = text;
    loop {
        let lower = rest.to_ascii_lowercase();
        let Some(index) = lower.find("c:\\users\\") else {
            result.push_str(rest);
            break;
        };
        result.push_str(&rest[..index]);
        let after_prefix = index + "c:\\users\\".len();
        let tail = &rest[after_prefix..];
        let end = tail
            .find(['\\', '/', '\n', '\r', ' ', '\t', '"', '\''])
            .unwrap_or(tail.len());
        result.push_str("%USERPROFILE%");
        rest = &tail[end..];
    }
    result
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
    path.as_ref()
        .display()
        .to_string()
        .trim_start_matches("\\\\?\\")
        .to_string()
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
    fn environment_backup_keeps_latest_and_history() {
        let root = tempfile::tempdir().unwrap();
        let paths = AppPaths::new(root.path().join("DevEnvManager"));
        paths.ensure().unwrap();
        let environment = HashMap::from([
            ("DEVENV_HOME".to_string(), r"D:\OldDevEnv".to_string()),
            ("JAVA_HOME".to_string(), r"D:\Java\jdk-21".to_string()),
            ("Path".to_string(), r"C:\Windows;D:\Tools".to_string()),
        ]);
        let file_name = create_environment_backup(&paths, &environment).unwrap();
        assert!(paths.env_backup_file().is_file());
        assert!(paths.config().join("env_backups").join(file_name).is_file());
        assert_eq!(split_path_entries(&environment["Path"]).len(), 2);
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
    fn directory_validation_reports_file_and_unknown_project() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("single.txt");
        fs::write(&file, "x").unwrap();
        let file_result = validate_directory_path(display_path(&file)).unwrap();
        assert!(file_result.exists);
        assert!(!file_result.is_directory);
        assert!(file_result.message.contains("项目根目录"));

        let dir_result = validate_directory_path(display_path(root.path())).unwrap();
        assert!(dir_result.is_directory);
        assert!(!dir_result.recognized_project);
        assert!(dir_result.message.contains("没有识别到常见项目文件"));
    }

    #[test]
    fn idea_project_analysis_reads_safe_files_only() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join(".idea")).unwrap();
        fs::write(
            root.path().join(".idea").join("misc.xml"),
            r#"<project><component name="ProjectRootManager" languageLevel="JDK_17" project-jdk-name="17" /></project>"#,
        )
        .unwrap();
        fs::write(
            root.path().join(".idea").join("compiler.xml"),
            r#"<project><bytecodeTargetLevel target="17" /></project>"#,
        )
        .unwrap();
        fs::write(
            root.path().join("demo.iml"),
            r#"<module><orderEntry type="jdk" jdkName="17" /></module>"#,
        )
        .unwrap();
        fs::write(
            root.path().join(".idea").join("workspace.xml"),
            "recentFiles=\"secret\" token=\"hidden\"",
        )
        .unwrap();
        let report = inspect_idea_project_blocking(root.path()).unwrap();
        assert!(report.detected);
        assert_eq!(report.project_sdk, "17");
        assert!(report
            .read_files
            .iter()
            .any(|item| item == ".idea/misc.xml"));
        assert!(!report
            .read_files
            .iter()
            .any(|item| item.contains("workspace.xml")));
        assert!(report
            .warnings
            .iter()
            .any(|item| item.contains("workspace.xml")));
    }

    #[test]
    fn java_consumer_report_explains_missing_javac() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("bin")).unwrap();
        fs::write(root.path().join("bin").join("nexus.bat"), "@echo off").unwrap();
        let report = verify_java_consumer_environment_blocking("Nexus", root.path()).unwrap();
        assert_eq!(report.consumer, "Nexus");
        assert!(report.startup_exists);
        assert!(report
            .explanation
            .iter()
            .any(|item| item.contains("读取不到 Java")));
    }

    #[test]
    fn project_config_paths_are_strictly_limited() {
        assert!(allowed_project_config(".vscode/settings.json"));
        assert!(allowed_project_config(r".idea\misc.xml"));
        assert!(!allowed_project_config("pom.xml"));
        assert!(!allowed_project_config("../outside.json"));
    }

    #[test]
    fn chsrc_rejects_unknown_target_before_execution() {
        let error = run_chsrc_action_blocking("get", "unknown-target", None).unwrap_err();
        assert!(error.contains("白名单"));
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
    fn python_diagnostic_report_explains_store_alias_without_dangerous_fix() {
        let python = PythonToolState {
            path: r"C:\Users\Alice\AppData\Local\Microsoft\WindowsApps\python.exe".to_string(),
            version: "Python 3.12".to_string(),
            status: "风险".to_string(),
            detail: "Microsoft Store".to_string(),
        };
        let risks = ["Microsoft Store Python 执行别名可能抢占 python 命令".to_string()];
        let repair_blockers =
            ["命中 WindowsApps Store Alias 时，DevEnv Manager 不会自动关闭别名或删除 WindowsApps PATH。".to_string()];
        let recovery_actions = [
            "打开 Windows 应用执行别名设置，人工关闭 python.exe / python3.exe Store Alias。"
                .to_string(),
        ];
        let report = python_diagnostic_report(PythonDiagnosticInput {
            current_python: Some(&python),
            current_pip: None,
            launcher_path: r"C:\Windows\py.exe",
            launcher_output: r"-V:3.12 C:\Python312\python.exe",
            first_python_on_path: &python.path,
            first_pip_on_path: "",
            python_m_pip_available: false,
            store_alias_risk: true,
            managed_python_available: false,
            risks: &risks,
            repair_blockers: &repair_blockers,
            recovery_actions: &recovery_actions,
        });
        assert!(report.contains("WindowsApps"));
        assert!(report.contains("不会自动关闭别名"));
        assert!(report.contains("%USERPROFILE%"));
        assert!(!report.contains(r"C:\Users\Alice"));
    }

    #[test]
    fn jdk_candidate_source_distinguishes_managers_and_ide_bundled() {
        assert_eq!(
            classify_jdk_candidate_source(r"D:\DevEnvManager\current\jdk\bin\java.exe"),
            "Managed"
        );
        assert_eq!(
            classify_jdk_candidate_source(
                r"C:\Users\Alice\scoop\apps\openjdk\current\bin\java.exe"
            ),
            "Scoop"
        );
        assert_eq!(
            classify_jdk_candidate_source(
                r"C:\Program Files\JetBrains\IntelliJ IDEA\jbr\bin\java.exe"
            ),
            "IdeBundled"
        );
        assert_eq!(
            classify_jdk_candidate_source(r"C:\Program Files\Eclipse Adoptium\jdk-21\bin\java.exe"),
            "SystemInstaller"
        );
    }

    #[test]
    fn open_analysis_path_reports_missing_path_as_rescan_needed() {
        let missing = tempfile::tempdir()
            .unwrap()
            .path()
            .join("missing-file.zip")
            .to_string_lossy()
            .to_string();
        let error = open_analysis_path(missing).unwrap_err();
        assert!(error.contains("重新扫描"));
    }

    #[test]
    fn chsrc_missing_keeps_single_mirror_fallbacks() {
        let recovery = chsrc_recovery(true);
        assert!(recovery.missing);
        assert!(recovery
            .explanation
            .iter()
            .any(|item| item.contains("不会静默安装")));
        assert!(recovery
            .fallback_features
            .contains(&"pip index-url".to_string()));
        assert!(recovery.fallback_features.contains(&"GOPROXY".to_string()));
        assert_eq!(recovery.scoop_command, "scoop install chsrc");
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
    fn redaction_masks_bearer_cli_flags_paths_and_json() {
        let command = r#"server.exe --token=abc --password=hunter2 Authorization: Bearer secret C:\Users\Alice\project"#;
        let redacted = redact_command_line(command);
        assert!(!redacted.contains("abc"));
        assert!(!redacted.contains("hunter2"));
        assert!(!redacted.contains("secret"));
        assert!(!redacted.contains(r"C:\Users\Alice"));
        assert!(redacted.contains("%USERPROFILE%"));
        assert_eq!(
            redact_path(r"C:\Users\Alice\Desktop"),
            r"%USERPROFILE%\Desktop"
        );

        let mut value = json!({
            "nested": {
                "api_key": "abc123",
                "path": "C:\\Users\\Alice\\Downloads\\tool.zip",
                "mysql": "ERROR 1045 (28000)"
            }
        });
        redact_json_value(&mut value);
        let text = serde_json::to_string(&value).unwrap();
        assert!(!text.contains("abc123"));
        assert!(!text.contains(r"C:\\Users\\Alice"));
        assert!(text.contains("ERROR 1045"));
    }

    #[test]
    fn download_url_allowlist_rejects_unknown_hosts() {
        assert!(validate_download_url("https://nodejs.org/dist/index.json").is_ok());
        assert!(validate_download_url("https://dl.google.com/go/go1.25.windows-amd64.zip").is_ok());
        assert!(validate_download_url("https://download.bell-sw.com/java/21/file.zip").is_ok());
        assert!(validate_download_url("https://example.com/file.zip").is_err());
        assert!(validate_download_url("http://go.dev/dl/file.zip").is_err());
    }

    #[test]
    fn update_manifest_accepts_camel_and_snake_download_url() {
        let camel: UpdateManifest = serde_json::from_value(json!({
            "version": "1.5.2",
            "date": "2026-06-27",
            "notes": ["patch"],
            "downloadUrl": "https://github.com/weidonglang/DevEnv-Manager/releases/download/v1.5.2/DevEnv.Manager_1.5.2_x64-setup.exe",
            "sha256": "a".repeat(64)
        }))
        .unwrap();
        validate_update_manifest(&camel).unwrap();
        let snake: UpdateManifest = serde_json::from_value(json!({
            "version": "1.5.2",
            "date": "2026-06-27",
            "notes": ["patch"],
            "download_url": "https://github.com/weidonglang/DevEnv-Manager/releases/download/v1.5.2/DevEnv.Manager_1.5.2_x64-setup.exe",
            "sha256": "b".repeat(64)
        }))
        .unwrap();
        assert_eq!(snake.download_url, camel.download_url);
        validate_update_manifest(&snake).unwrap();
    }

    #[test]
    fn confirmation_token_is_bound_and_single_use() {
        let plan_id = format!("test-plan-{}", unix_timestamp());
        let fingerprint = process_action_fingerprint("kill_process", &plan_id, "high");
        let token = create_confirmation_token(
            "kill_process".to_string(),
            plan_id.clone(),
            "high".to_string(),
            fingerprint.clone(),
            false,
            None,
        )
        .unwrap();
        assert!(require_confirmation_token(
            Some(token.token.clone()),
            "kill_process",
            &plan_id,
            "high",
            &fingerprint,
            false,
        )
        .is_ok());
        assert!(require_confirmation_token(
            Some(token.token),
            "kill_process",
            &plan_id,
            "high",
            &fingerprint,
            false,
        )
        .is_err());
    }

    #[test]
    fn desktop_process_not_classified_as_spring_by_port_only() {
        let signature = analyze_port_signature(
            8080,
            "LISTENING",
            "steamwebhelper.exe",
            r"C:\Program Files (x86)\Steam\bin\cef\steamwebhelper.exe",
            r#""C:\Program Files (x86)\Steam\bin\cef\steamwebhelper.exe""#,
            &[],
        );
        assert_eq!(signature.identity, "桌面应用");
        assert!(signature
            .conflict_evidence
            .iter()
            .any(|item| item.contains("桌面/浏览器/IDE")));
        assert!(!signature.identity.contains("Spring"));
    }

    #[test]
    fn qq_on_8082_is_not_spring() {
        let signature = analyze_port_signature(
            8082,
            "LISTENING",
            "QQ.exe",
            r"C:\Program Files\Tencent\QQNT\QQ.exe",
            r#""C:\Program Files\Tencent\QQNT\QQ.exe""#,
            &[],
        );
        assert_eq!(signature.identity, "桌面应用");
        assert!(!signature.identity.contains("Spring"));
        assert!(!signature.identity.contains("Tomcat"));
    }

    #[test]
    fn chrome_debug_port_is_specific_not_generic_web() {
        let signature = analyze_port_signature(
            9222,
            "LISTENING",
            "chrome.exe",
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r#""C:\Program Files\Google\Chrome\Application\chrome.exe" --remote-debugging-port=9222"#,
            &[],
        );
        assert_eq!(signature.identity, "Chrome 调试端口");
        assert!(!signature.identity.contains("Web 服务"));
    }

    #[test]
    fn code_process_is_not_user_project_service_by_port_only() {
        let signature = analyze_port_signature(
            5173,
            "LISTENING",
            "Code.exe",
            r"C:\Users\Alice\AppData\Local\Programs\Microsoft VS Code\Code.exe",
            r#""C:\Users\Alice\AppData\Local\Programs\Microsoft VS Code\Code.exe""#,
            &[],
        );
        assert!(matches!(
            signature.identity.as_str(),
            "IDE / 调试器" | "桌面应用"
        ));
        assert!(!signature.identity.contains("Vite"));
    }

    #[test]
    fn unknown_8080_stays_low_confidence_unknown() {
        let signature = analyze_port_signature(
            8080,
            "LISTENING",
            "unknown.exe",
            r"C:\Tools\unknown.exe",
            r#""C:\Tools\unknown.exe""#,
            &[],
        );
        assert_eq!(signature.identity, "未识别的本地服务");
        assert!(signature.confidence < 40);
        assert_eq!(signature.risk_level, "low");
    }

    #[test]
    fn established_connection_is_not_local_listening_service() {
        let signature = analyze_port_signature(
            8080,
            "ESTABLISHED",
            "java.exe",
            r"C:\Program Files\Java\jdk-21\bin\java.exe",
            r#""C:\Program Files\Java\jdk-21\bin\java.exe" -jar app.jar"#,
            &[],
        );
        assert_eq!(signature.risk_level, "low");
        assert!(signature
            .conflict_evidence
            .iter()
            .any(|item| item.contains("不是本地监听状态")));
    }

    #[test]
    fn vite_and_spring_have_strong_identity() {
        let vite = analyze_port_signature(
            5173,
            "LISTENING",
            "node.exe",
            r"C:\Program Files\nodejs\node.exe",
            r#""node.exe" "C:\app\node_modules\.bin\vite" --host 127.0.0.1"#,
            &[],
        );
        assert_eq!(vite.identity, "Vite");
        assert!(vite.confidence >= 40);

        let spring = analyze_port_signature(
            8080,
            "LISTENING",
            "java.exe",
            r"C:\Program Files\Java\jdk-21\bin\java.exe",
            r#""java.exe" -jar demo-spring-boot.jar"#,
            &[],
        );
        assert_eq!(spring.identity, "Spring Boot");
        assert!(spring.confidence >= 40);
    }

    #[test]
    fn liberica_parser_accepts_array_response() {
        let response = json!([{
            "filename": "bellsoft-jdk21.zip",
            "downloadUrl": "https://download.bell-sw.com/java/21/bellsoft-jdk21.zip",
            "version": "21.0.8"
        }]);
        let package = response
            .as_array()
            .and_then(|items| items.first())
            .or_else(|| {
                response
                    .get("releases")
                    .and_then(Value::as_array)
                    .and_then(|items| items.first())
            })
            .unwrap_or(&response);
        assert_eq!(
            package.get("filename").and_then(Value::as_str),
            Some("bellsoft-jdk21.zip")
        );
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
    fn cleanup_architecture_enables_only_phase2_safe_categories() {
        let architecture = cleanup::architecture();
        assert_eq!(architecture.status, "safe-clean-and-analysis-phase-3");
        assert!(architecture
            .categories
            .iter()
            .any(|category| { category.id == "windows-temp" && category.cleanup_enabled }));
        assert!(architecture
            .categories
            .iter()
            .any(|category| { category.id == "devenv-manager" && category.cleanup_enabled }));
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
    fn checksum_text_parser_rejects_non_hash_text() {
        let valid = format!("{}  python.zip\n", "a".repeat(64));
        assert_eq!(
            parse_sha256_for_file(&valid, "python.zip"),
            Some("a".repeat(64))
        );
        assert!(parse_sha256_for_file("SHA256 (python.zip) = downloading", "python.zip").is_none());
        assert!(
            parse_sha256_for_file(&format!("{}  other.zip", "b".repeat(64)), "python.zip")
                .is_none()
        );
    }

    #[test]
    fn python_path_preview_only_prepends_and_deduplicates() {
        let additions = vec![
            r"C:\Python312".to_string(),
            r"C:\Python312\Scripts".to_string(),
        ];
        let (path, added) = prepend_path_entries(r"C:\Tools;C:\Python312", &additions);
        assert_eq!(added, vec![r"C:\Python312\Scripts"]);
        assert!(path.starts_with(r"C:\Python312;C:\Python312\Scripts;C:\Tools"));
    }

    #[test]
    fn learning_center_rejects_state_changing_commands() {
        assert!(learning_command_allowed(&[
            "python".to_string(),
            "--version".to_string()
        ]));
        assert!(learning_command_allowed(&[
            "python".to_string(),
            "-m".to_string(),
            "pip".to_string(),
            "--version".to_string()
        ]));
        assert!(!learning_command_allowed(&[
            "python".to_string(),
            "-m".to_string(),
            "pip".to_string(),
            "install".to_string(),
            "requests".to_string()
        ]));
        assert!(!learning_command_allowed(&[
            "cmd".to_string(),
            "/c".to_string(),
            "set".to_string()
        ]));
    }

    #[test]
    fn archive_plan_rejects_chat_and_browser_credentials() {
        assert!(archive_path_is_sensitive(Path::new(
            r"C:\Users\test\Documents\WeChat Files\wxid\Msg\MicroMsg.db"
        )));
        assert!(archive_path_is_sensitive(Path::new(
            r"C:\Users\test\AppData\Local\Google\Chrome\User Data\Default\Login Data"
        )));
        assert!(!archive_path_is_sensitive(Path::new(
            r"C:\Users\test\Downloads\archive.zip"
        )));
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
    fn external_runtime_manual_action_never_deletes_portable_go() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("go1.22");
        let bin = root.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let executable = bin.join("go.exe");
        fs::write(&executable, b"test").unwrap();
        let message = external_runtime_manual_action_message(&executable, "Go");
        assert!(message.contains("不会删除外部目录"));
        assert!(root.exists());
    }

    #[test]
    fn external_runtime_package_manager_paths_are_manual_only() {
        let scoop_python = PathBuf::from(r"C:\Users\Alice\scoop\apps\python\current\python.exe");
        let message = external_runtime_manual_action_message(&scoop_python, "Python");
        assert!(message.contains("不会调用包管理器卸载"));
        assert!(message.contains("scoop uninstall python"));
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
