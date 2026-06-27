use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;
const MAX_DISCOVERY_ENTRIES: usize = 20_000;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MySqlRepairReport {
    pub generated_at: String,
    pub candidates: Vec<MySqlCandidate>,
    pub warnings: Vec<String>,
    pub privacy_notice: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MySqlCandidate {
    pub id: String,
    pub status: String,
    pub version_hint: String,
    pub service_name: String,
    pub service_state: String,
    pub mysqld_path: String,
    pub my_ini_path: String,
    pub basedir: String,
    pub datadir: String,
    pub port: u16,
    pub port_occupied: bool,
    pub data_health: String,
    pub confidence: String,
    pub conclusion_level: String,
    pub evidence: Vec<String>,
    pub next_steps: Vec<String>,
    pub system_schema_missing: bool,
    pub business_databases: Vec<String>,
    pub last_error: String,
    pub suggestions: Vec<String>,
    pub registration_command: String,
    pub console_command: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MySqlRepairPlan {
    pub plan_id: String,
    pub created_at: String,
    pub candidate_id: String,
    pub action: String,
    pub title: String,
    pub steps: Vec<String>,
    pub commands: Vec<String>,
    pub warnings: Vec<String>,
    pub requires_admin: bool,
    pub requires_backup: bool,
    pub risk_level: String,
    pub plan_fingerprint: String,
}

#[derive(Clone)]
struct PendingPlan {
    public: MySqlRepairPlan,
    candidate: MySqlCandidate,
    created_at: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupReceipt {
    datadir: String,
    destination: String,
    created_at: u64,
    expires_at: u64,
    files: usize,
    bytes: u64,
    ibdata: bool,
    frm: bool,
    business_schema: bool,
    manifest_path: String,
    manifest_hash: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct BackupManifest {
    schema_version: u32,
    datadir: String,
    destination: String,
    created_at: u64,
    expires_at: u64,
    files: usize,
    bytes: u64,
    ibdata: bool,
    frm: bool,
    business_schema: bool,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MySqlPendingExecutionGuard {
    pub action_id: String,
    pub plan_id: String,
    pub risk_level: String,
    pub plan_fingerprint: String,
    pub backup_required: bool,
    pub backup_receipt: Option<String>,
}

static PLANS: OnceLock<Mutex<HashMap<String, PendingPlan>>> = OnceLock::new();
static BACKUPS: OnceLock<Mutex<Vec<BackupReceipt>>> = OnceLock::new();

fn plans() -> &'static Mutex<HashMap<String, PendingPlan>> {
    PLANS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn backups() -> &'static Mutex<Vec<BackupReceipt>> {
    BACKUPS.get_or_init(|| Mutex::new(Vec::new()))
}

fn app_config_dir() -> PathBuf {
    dirs::data_local_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("DevEnvManager")
}

fn backup_registry_file() -> PathBuf {
    app_config_dir().join("mysql-backups.json")
}

fn load_persisted_backups() -> Vec<BackupReceipt> {
    let path = backup_registry_file();
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str::<Vec<BackupReceipt>>(&text)
        .unwrap_or_default()
        .into_iter()
        .filter(|receipt| verify_backup_receipt(receipt).is_ok())
        .collect()
}

fn save_persisted_backups(receipts: &[BackupReceipt]) -> Result<(), String> {
    let path = backup_registry_file();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("创建 MySQL 备份登记目录失败：{err}"))?;
    }
    let text = serde_json::to_string_pretty(receipts)
        .map_err(|err| format!("生成 MySQL 备份登记失败：{err}"))?;
    fs::write(path, text).map_err(|err| format!("写入 MySQL 备份登记失败：{err}"))
}

fn backup_manifest_hash(manifest: &BackupManifest) -> Result<String, String> {
    let text = serde_json::to_string(manifest)
        .map_err(|err| format!("生成备份 manifest 校验失败：{err}"))?;
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

fn write_backup_manifest(manifest: &BackupManifest) -> Result<(String, String), String> {
    let destination = Path::new(&manifest.destination);
    fs::create_dir_all(destination).map_err(|err| format!("创建备份目录失败：{err}"))?;
    let target = destination.join("devenv-mysql-backup-manifest.json");
    let text = serde_json::to_string_pretty(manifest)
        .map_err(|err| format!("生成备份 manifest 失败：{err}"))?;
    fs::write(&target, text).map_err(|err| format!("写入备份 manifest 失败：{err}"))?;
    Ok((display(&target), backup_manifest_hash(manifest)?))
}

fn verify_backup_receipt(receipt: &BackupReceipt) -> Result<(), String> {
    if receipt.expires_at < now() {
        return Err("备份回执已超过有效期".to_string());
    }
    if !Path::new(&receipt.destination).is_dir() {
        return Err("备份目录不存在".to_string());
    }
    let manifest_path = Path::new(&receipt.manifest_path);
    let text = fs::read_to_string(manifest_path)
        .map_err(|_| "备份 manifest 不存在或不可读".to_string())?;
    let manifest: BackupManifest =
        serde_json::from_str(&text).map_err(|_| "备份 manifest 已损坏".to_string())?;
    if manifest.datadir != receipt.datadir
        || manifest.destination != receipt.destination
        || manifest.files != receipt.files
        || manifest.bytes != receipt.bytes
        || manifest.expires_at != receipt.expires_at
        || backup_manifest_hash(&manifest)? != receipt.manifest_hash
    {
        return Err("备份 manifest 与登记回执不一致".to_string());
    }
    Ok(())
}

fn remember_backup(receipt: BackupReceipt) -> Result<(), String> {
    let mut registry = load_persisted_backups();
    registry.retain(|item| item.datadir != receipt.datadir || item.expires_at >= now());
    registry.push(receipt.clone());
    save_persisted_backups(&registry)?;
    backups()
        .lock()
        .map_err(|_| "备份记录暂时不可用".to_string())?
        .push(receipt);
    Ok(())
}

fn recent_valid_backup(datadir: &str) -> Option<BackupReceipt> {
    let mut all = load_persisted_backups();
    if let Ok(memory) = backups().lock() {
        all.extend(memory.iter().cloned());
    }
    all.into_iter().find(|item| {
        item.datadir.eq_ignore_ascii_case(datadir) && verify_backup_receipt(item).is_ok()
    })
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn display(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn hidden_command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

fn collect_named(root: &Path, names: &[&str], depth: usize, output: &mut Vec<PathBuf>) {
    if depth == 0 || !root.is_dir() || output.len() >= MAX_DISCOVERY_ENTRIES {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        if output.len() >= MAX_DISCOVERY_ENTRIES {
            break;
        }
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_file()
            && path
                .file_name()
                .and_then(OsStr::to_str)
                .is_some_and(|name| names.iter().any(|item| name.eq_ignore_ascii_case(item)))
        {
            output.push(path);
        } else if metadata.is_dir() {
            collect_named(&path, names, depth - 1, output);
        }
    }
}

fn discovery_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for variable in ["ProgramFiles", "ProgramFiles(x86)", "ProgramData"] {
        if let Some(value) = std::env::var_os(variable) {
            let base = PathBuf::from(value);
            roots.push(base.join("MySQL"));
            roots.push(base.join("MariaDB"));
        }
    }
    roots.sort();
    roots.dedup();
    roots
}

fn service_inventory() -> Vec<(String, String, String)> {
    #[cfg(windows)]
    {
        let script = "[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new(); @(Get-CimInstance Win32_Service | Where-Object { $_.Name -match 'mysql|maria' -or $_.PathName -match 'mysqld' } | Select-Object Name,State,PathName) | ConvertTo-Json -Compress";
        let Ok(output) = hidden_command("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .output()
        else {
            return Vec::new();
        };
        let text = String::from_utf8_lossy(&output.stdout);
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
            return Vec::new();
        };
        let items = match value {
            serde_json::Value::Array(items) => items,
            serde_json::Value::Object(_) => vec![value],
            _ => Vec::new(),
        };
        items
            .into_iter()
            .filter_map(|item| {
                Some((
                    item.get("Name")?.as_str()?.to_string(),
                    item.get("State")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    item.get("PathName")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                ))
            })
            .collect()
    }
    #[cfg(not(windows))]
    Vec::new()
}

fn parse_ini(path: &Path) -> HashMap<String, String> {
    let Ok(text) = fs::read_to_string(path) else {
        return HashMap::new();
    };
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with(['#', ';', '[']) {
                return None;
            }
            let (key, value) = line.split_once('=')?;
            Some((
                key.trim().to_ascii_lowercase(),
                value.trim().trim_matches(['"', '\'']).to_string(),
            ))
        })
        .collect()
}

fn version_hint(path: &Path) -> String {
    let value = display(path).to_ascii_lowercase();
    for version in ["8.4", "8.0", "5.7", "5.6", "5.5"] {
        if value.contains(version) {
            return version.to_string();
        }
    }
    if value.contains("maria") {
        "MariaDB".to_string()
    } else {
        "未知".to_string()
    }
}

fn configured_path(value: &str, base: &Path) -> PathBuf {
    let mut expanded = value.to_string();
    for name in [
        "ProgramData",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "USERPROFILE",
    ] {
        if let Some(replacement) = std::env::var_os(name) {
            expanded = expanded.replace(&format!("%{name}%"), &replacement.to_string_lossy());
        }
    }
    let path = PathBuf::from(expanded);
    if path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}

fn inferred_service(version: &str) -> String {
    match version {
        "5.5" => "MySQL55",
        "5.6" => "MySQL56",
        "5.7" => "MySQL57",
        "8.0" => "MySQL80",
        "8.4" => "MySQL84",
        "MariaDB" => "MariaDB",
        _ => "MySQL",
    }
    .to_string()
}

fn read_tail(path: &Path, max_bytes: u64) -> String {
    let Ok(mut file) = fs::File::open(path) else {
        return String::new();
    };
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    let _ = file.seek(SeekFrom::Start(len.saturating_sub(max_bytes)));
    let mut bytes = Vec::new();
    let _ = file.read_to_end(&mut bytes);
    crate::decode_command_stream(&bytes)
        .lines()
        .rev()
        .take(80)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
}

fn redact_error_log(text: &str) -> String {
    text.lines()
        .map(|line| {
            let lower = line.to_ascii_lowercase();
            if ["identified by", "password=", "passwd=", "token=", "secret="]
                .iter()
                .any(|marker| lower.contains(marker))
            {
                "[包含凭据格式的日志行已隐藏]"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn database_directories(datadir: &Path) -> Vec<String> {
    let system = [
        "mysql",
        "performance_schema",
        "information_schema",
        "sys",
        "test",
    ];
    let Ok(entries) = fs::read_dir(datadir) else {
        return Vec::new();
    };
    let mut result = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
        .filter(|name| !system.iter().any(|item| name.eq_ignore_ascii_case(item)))
        .collect::<Vec<_>>();
    result.sort();
    result
}

fn candidate_id(mysqld: &Path, ini: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(display(mysqld).to_ascii_lowercase());
    hasher.update(b"\0");
    hasher.update(display(ini).to_ascii_lowercase());
    format!("{:x}", hasher.finalize())
}

fn candidate_from(
    mysqld: PathBuf,
    ini_candidates: &[PathBuf],
    services: &[(String, String, String)],
) -> MySqlCandidate {
    let root = mysqld
        .parent()
        .and_then(Path::parent)
        .unwrap_or_else(|| Path::new(""));
    let ini = ini_candidates
        .iter()
        .find(|path| path.starts_with(root))
        .cloned()
        .or_else(|| ini_candidates.first().cloned())
        .unwrap_or_else(|| root.join("my.ini"));
    let config = parse_ini(&ini);
    let basedir = config
        .get("basedir")
        .map(|value| configured_path(value, root))
        .unwrap_or_else(|| root.to_path_buf());
    let datadir = config
        .get("datadir")
        .map(|value| configured_path(value, &basedir))
        .unwrap_or_else(|| basedir.join("data"));
    let port = config
        .get("port")
        .and_then(|value| value.parse().ok())
        .unwrap_or(3306);
    let version = version_hint(&mysqld);
    let service = services.iter().find(|(_, _, path)| {
        path.to_ascii_lowercase()
            .contains(&display(&mysqld).to_ascii_lowercase())
    });
    let service_name = service
        .map(|item| item.0.clone())
        .unwrap_or_else(|| inferred_service(&version));
    let service_state = service
        .map(|item| item.1.clone())
        .unwrap_or_else(|| "NotInstalled".to_string());
    let mysql_schema = datadir.join("mysql");
    let legacy_required = ["host.frm", "user.frm", "db.frm", "plugin.frm"];
    let system_schema_missing = !mysql_schema.is_dir()
        || (version.starts_with("5.")
            && legacy_required
                .iter()
                .any(|name| !mysql_schema.join(name).is_file()));
    let business_databases = database_directories(&datadir);
    let mut error_files = Vec::new();
    collect_named(&datadir, &["mysql.err"], 2, &mut error_files);
    if let Ok(entries) = fs::read_dir(&datadir) {
        error_files.extend(entries.flatten().map(|e| e.path()).filter(|p| {
            p.extension()
                .and_then(OsStr::to_str)
                .is_some_and(|ext| ext.eq_ignore_ascii_case("err"))
        }));
    }
    error_files.sort_by_key(|path| fs::metadata(path).and_then(|m| m.modified()).ok());
    let raw_error = error_files
        .last()
        .map(|path| read_tail(path, 128 * 1024))
        .unwrap_or_default();
    let last_error = redact_error_log(&raw_error);
    let port_occupied = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)).is_err();
    let unsupported_layout = display(&mysqld).to_ascii_lowercase().contains("xampp")
        || display(&mysqld).to_ascii_lowercase().contains("phpstudy")
        || display(&mysqld).to_ascii_lowercase().contains("laragon")
        || version == "MariaDB";
    let conclusion_level = if unsupported_layout {
        "UnsupportedLayout"
    } else if service_state.eq_ignore_ascii_case("Running") && port_occupied {
        "Healthy"
    } else if system_schema_missing && !business_databases.is_empty() && !port_occupied {
        "LikelyBroken"
    } else if system_schema_missing && business_databases.is_empty() {
        "PotentialRisk"
    } else if service.is_none() || port_occupied {
        "UsableWithWarnings"
    } else {
        "PermissionUnknown"
    }
    .to_string();
    let status = if conclusion_level == "UnsupportedLayout" {
        "UnsupportedLayout"
    } else if service_state.eq_ignore_ascii_case("Running") {
        if raw_error.contains("1045") {
            "AuthFailed"
        } else {
            "Running"
        }
    } else if service.is_none() {
        "NotInstalled"
    } else if raw_error.contains("1067") || raw_error.to_ascii_lowercase().contains("fatal error") {
        "StartFailed"
    } else {
        "Stopped"
    }
    .to_string();
    let mut suggestions = Vec::new();
    if service.is_none() {
        suggestions
            .push("程序文件仍在但服务未注册；先预览注册计划，不要直接执行未知脚本".to_string());
    }
    if system_schema_missing {
        suggestions.push("业务库可能仍可恢复；必须完整备份 Data 后才能补回系统库".to_string());
    }
    if !business_databases.is_empty() {
        suggestions.push(format!(
            "检测到候选业务库：{}；服务恢复后应立即导出 SQL",
            business_databases.join("、")
        ));
    }
    let evidence = vec![
        format!("服务名：{service_name}"),
        format!("服务状态：{service_state}"),
        format!("mysqld：{}", display(&mysqld)),
        format!("my.ini：{}", display(&ini)),
        format!("basedir：{}", display(&basedir)),
        format!("datadir：{}", display(&datadir)),
        format!(
            "端口 {port} 监听：{}",
            if port_occupied { "是" } else { "否" }
        ),
        format!(
            "静态文件：mysql 系统库{}，业务库候选 {} 个",
            if system_schema_missing {
                "缺失或不完整"
            } else {
                "可读"
            },
            business_databases.len()
        ),
        "连接验证：未保存密码，默认不自动连接数据库".to_string(),
        format!(
            "结论可信度：{}",
            if unsupported_layout {
                "低"
            } else if service.is_some() {
                "中高"
            } else {
                "中"
            }
        ),
        format!(
            "判断原因：{}",
            if system_schema_missing {
                "静态系统库检查异常，需结合服务状态/端口/连接验证确认"
            } else {
                "服务、配置和 Data 静态检查未发现系统库缺失"
            }
        ),
    ];
    let confidence = if unsupported_layout {
        "低：非标准发行版/布局，仅提示，不自动修复"
    } else if service.is_some() && datadir.is_dir() {
        "中高：服务登记与配置/Data 路径均可验证"
    } else {
        "中：缺少服务登记或权限证据"
    }
    .to_string();
    let mut next_steps = suggestions.clone();
    if next_steps.is_empty() {
        next_steps
            .push("如仍无法启动，请先导出错误日志并使用只读向导排查账号或端口问题".to_string());
    }
    let quoted_mysqld = format!("\"{}\"", display(&mysqld));
    let quoted_ini = format!("\"{}\"", display(&ini));
    MySqlCandidate {
        id: candidate_id(&mysqld, &ini),
        status,
        version_hint: version,
        service_name: service_name.clone(),
        service_state,
        mysqld_path: display(&mysqld),
        my_ini_path: display(&ini),
        basedir: display(&basedir),
        datadir: display(&datadir),
        port,
        port_occupied,
        data_health: if system_schema_missing { "MySQL 系统库缺失或不完整" } else if datadir.is_dir() { "Data 目录可读" } else { "Data 目录不存在" }.to_string(),
        confidence,
        conclusion_level,
        evidence,
        next_steps,
        system_schema_missing,
        business_databases,
        last_error,
        suggestions,
        registration_command: format!("{quoted_mysqld} --install {service_name} --defaults-file={quoted_ini}\nsc config {service_name} start= auto"),
        console_command: format!("{quoted_mysqld} --defaults-file={quoted_ini} --console"),
    }
}

pub fn inspect() -> MySqlRepairReport {
    let mut mysqld = Vec::new();
    let mut ini = Vec::new();
    for root in discovery_roots() {
        collect_named(&root, &["mysqld.exe"], 6, &mut mysqld);
        collect_named(&root, &["my.ini", "my.cnf"], 6, &mut ini);
    }
    mysqld.sort();
    mysqld.dedup();
    ini.sort();
    ini.dedup();
    let services = service_inventory();
    let candidates = mysqld
        .into_iter()
        .map(|path| candidate_from(path, &ini, &services))
        .collect();
    MySqlRepairReport {
        generated_at: now().to_string(),
        candidates,
        warnings: vec![
            "诊断不会启动 mysqld --console，避免未确认时写入 Data；仅生成控制台命令并读取现有 .err 尾部".to_string(),
            "不会读取数据库表内容，也不会采集、保存或输出数据库密码".to_string(),
        ],
        privacy_notice: "报告只包含服务、配置路径、文件存在性、目录名和错误摘要；导出前仍应检查主机名等业务信息。".to_string(),
    }
}

fn current_candidate(id: &str) -> Result<MySqlCandidate, String> {
    inspect()
        .candidates
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| "MySQL 候选已变化，请重新诊断".to_string())
}

pub fn create_plan(candidate_id: String, action: String) -> Result<MySqlRepairPlan, String> {
    let candidate = current_candidate(&candidate_id)?;
    let (title, steps, commands, requires_admin, requires_backup) = match action.as_str() {
        "backup" => (
            "备份完整 Data 目录",
            vec![
                "选择 Data 目录之外的新备份目录",
                "复制普通文件且不跟随符号链接",
                "核对文件数、总大小、ibdata1、业务库和 .frm",
            ],
            vec![format!("备份 {} 到用户选择目录", candidate.datadir)],
            false,
            false,
        ),
        "register_service" => (
            "恢复 MySQL Windows 服务注册",
            vec![
                "重新核对 mysqld.exe 与 my.ini",
                "注册推断服务名",
                "设置自动启动；不立即修改 Data",
            ],
            candidate
                .registration_command
                .lines()
                .map(str::to_string)
                .collect(),
            true,
            false,
        ),
        "start_service" => (
            "启动 MySQL 服务",
            vec![
                "重新确认服务存在",
                "请求 Windows SCM 启动",
                "再次诊断服务与端口",
            ],
            vec![format!("sc start {}", candidate.service_name)],
            true,
            false,
        ),
        "repair_system_schema" => (
            "从安装目录补回缺失系统库",
            vec![
                "验证近 24 小时内完整 Data 备份",
                "只在目标 mysql 目录不存在时复制系统库",
                "不覆盖业务库、ibdata1 或 ib_logfile",
                "尝试启动并重新诊断",
            ],
            vec![format!(
                "复制 {}\\data\\mysql -> {}\\mysql",
                candidate.basedir, candidate.datadir
            )],
            true,
            true,
        ),
        "reset_root_guide" => (
            "生成 root 认证恢复向导",
            vec![
                "区分服务故障与 1045 认证失败",
                "停止服务并人工启动 skip-grant-tables",
                "在独立终端按版本执行账号 SQL",
                "全过程不把密码交给 DevEnv Manager",
            ],
            vec!["仅生成向导，不执行密码 SQL，不记录密码".to_string()],
            true,
            false,
        ),
        "dump_guide" => (
            "生成业务库导出建议",
            vec![
                "排除系统库",
                "逐个确认候选业务库",
                "使用 mysqldump -p 由终端安全读取密码",
            ],
            candidate
                .business_databases
                .iter()
                .map(|db| format!("mysqldump -u root -p {db} > <备份目录>\\{db}.sql"))
                .collect(),
            false,
            false,
        ),
        _ => return Err("不支持的 MySQL 修复动作".to_string()),
    };
    let created = now();
    let mut hasher = Sha256::new();
    hasher.update(candidate.id.as_bytes());
    hasher.update(action.as_bytes());
    hasher.update(created.to_le_bytes());
    let plan_id = format!("mysql-{:x}", hasher.finalize());
    let risk_level = mysql_action_risk(&action).to_string();
    let plan_fingerprint = plan_fingerprint(&candidate, &action);
    let public = MySqlRepairPlan {
        plan_id: plan_id.clone(),
        created_at: created.to_string(),
        candidate_id,
        action,
        title: title.to_string(),
        steps: steps.into_iter().map(str::to_string).collect(),
        commands,
        warnings: vec!["计划 30 分钟过期且只能执行一次；执行前会重新诊断路径与状态".to_string()],
        requires_admin,
        requires_backup,
        risk_level,
        plan_fingerprint,
    };
    let pending = PendingPlan {
        public: public.clone(),
        candidate,
        created_at: created,
    };
    let mut store = plans()
        .lock()
        .map_err(|_| "MySQL 计划存储暂时不可用".to_string())?;
    store.retain(|_, value| value.created_at.saturating_add(30 * 60) >= created);
    store.insert(plan_id, pending);
    Ok(public)
}

fn mysql_action_risk(action: &str) -> &'static str {
    match action {
        "repair_system_schema" => "critical",
        "register_service" | "start_service" | "backup" => "high",
        _ => "low",
    }
}

fn plan_fingerprint(candidate: &MySqlCandidate, action: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(candidate.id.as_bytes());
    hasher.update(b"\0");
    hasher.update(action.as_bytes());
    hasher.update(b"\0");
    hasher.update(candidate.mysqld_path.as_bytes());
    hasher.update(b"\0");
    hasher.update(candidate.my_ini_path.as_bytes());
    hasher.update(b"\0");
    hasher.update(candidate.datadir.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn pending_execution_guard(plan_id: &str) -> Result<MySqlPendingExecutionGuard, String> {
    let pending = plans()
        .lock()
        .map_err(|_| "MySQL 计划存储暂时不可用".to_string())?
        .get(plan_id)
        .cloned()
        .ok_or_else(|| "计划不存在、已执行或已过期".to_string())?;
    let backup = if pending.public.requires_backup {
        recent_valid_backup(&pending.candidate.datadir).map(|item| item.manifest_hash)
    } else {
        None
    };
    Ok(MySqlPendingExecutionGuard {
        action_id: format!("mysql_{}", pending.public.action),
        plan_id: pending.public.plan_id,
        risk_level: pending.public.risk_level,
        plan_fingerprint: pending.public.plan_fingerprint,
        backup_required: pending.public.requires_backup,
        backup_receipt: backup,
    })
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(u64, usize, bool, bool, bool), String> {
    if !source.is_dir() {
        return Err("Data 目录不存在或不可读".to_string());
    }
    let source_canonical = source
        .canonicalize()
        .map_err(|e| format!("解析 Data 目录失败：{e}"))?;
    let destination_name = destination
        .file_name()
        .filter(|name| *name != "." && *name != "..")
        .ok_or_else(|| "备份目标必须是明确的子目录名称".to_string())?;
    let destination_parent = destination
        .parent()
        .ok_or_else(|| "备份目标缺少父目录".to_string())?;
    fs::create_dir_all(destination_parent).map_err(|e| format!("创建备份父目录失败：{e}"))?;
    let destination = destination_parent
        .canonicalize()
        .map_err(|e| format!("解析备份父目录失败：{e}"))?
        .join(destination_name);
    if fs::symlink_metadata(&destination)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err("备份目标不能是符号链接或 junction".to_string());
    }
    if destination.exists()
        && fs::read_dir(&destination)
            .map(|mut i| i.next().is_some())
            .unwrap_or(true)
    {
        return Err("备份目标必须不存在或为空目录".to_string());
    }
    if destination.starts_with(&source_canonical) {
        return Err("备份目录不能位于 Data 目录内部".to_string());
    }
    fs::create_dir_all(&destination).map_err(|e| format!("创建备份目录失败：{e}"))?;
    let mut stack = vec![(source_canonical.clone(), destination)];
    let mut bytes = 0_u64;
    let mut files = 0_usize;
    let mut ibdata = false;
    let mut frm = false;
    let mut business = false;
    let system: HashSet<&str> = [
        "mysql",
        "performance_schema",
        "information_schema",
        "sys",
        "test",
    ]
    .into_iter()
    .collect();
    while let Some((src, dst)) = stack.pop() {
        for entry in fs::read_dir(&src)
            .map_err(|e| format!("读取备份源失败：{e}"))?
            .flatten()
        {
            let path = entry.path();
            let target = dst.join(entry.file_name());
            let meta = fs::symlink_metadata(&path).map_err(|e| format!("读取文件失败：{e}"))?;
            if meta.file_type().is_symlink() {
                continue;
            }
            if meta.is_dir() {
                if src == source_canonical
                    && !system.contains(
                        entry
                            .file_name()
                            .to_string_lossy()
                            .to_ascii_lowercase()
                            .as_str(),
                    )
                {
                    business = true;
                }
                fs::create_dir_all(&target).map_err(|e| format!("创建备份子目录失败：{e}"))?;
                stack.push((path, target));
            } else if meta.is_file() {
                fs::copy(&path, &target)
                    .map_err(|e| format!("复制 {} 失败：{e}", display(&path)))?;
                bytes = bytes.saturating_add(meta.len());
                files += 1;
                let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
                ibdata |= name == "ibdata1";
                frm |= name.ends_with(".frm");
            }
        }
    }
    Ok((bytes, files, ibdata, frm, business))
}

fn copy_missing_system_schema(source: &Path, destination: &Path) -> Result<(), String> {
    if destination.exists() {
        return Err("目标 mysql 系统库已经存在，已拒绝覆盖".to_string());
    }
    let _ = copy_tree(source, destination)?;
    Ok(())
}

pub fn execute(plan_id: String, backup_destination: Option<String>) -> Result<String, String> {
    let pending = plans()
        .lock()
        .map_err(|_| "MySQL 计划存储暂时不可用".to_string())?
        .remove(&plan_id)
        .ok_or_else(|| "计划不存在、已执行或已过期".to_string())?;
    if pending.created_at.saturating_add(30 * 60) < now() {
        return Err("计划已过期，请重新诊断和预览".to_string());
    }
    let current = current_candidate(&pending.public.candidate_id)?;
    if current.mysqld_path != pending.candidate.mysqld_path
        || current.datadir != pending.candidate.datadir
        || current.my_ini_path != pending.candidate.my_ini_path
    {
        return Err("MySQL 路径在预览后发生变化，已拒绝执行".to_string());
    }
    match pending.public.action.as_str() {
        "backup" => {
            let destination = backup_destination.filter(|v| !v.trim().is_empty()).ok_or_else(|| "请选择备份目标目录".to_string())?;
            let (bytes, files, ibdata, frm, business) = copy_tree(Path::new(&current.datadir), Path::new(&destination))?;
            let created_at = now();
            let expires_at = created_at + 24 * 60 * 60;
            let manifest = BackupManifest {
                schema_version: 1,
                datadir: current.datadir.clone(),
                destination: destination.clone(),
                created_at,
                expires_at,
                files,
                bytes,
                ibdata,
                frm,
                business_schema: business,
            };
            let (manifest_path, manifest_hash) = write_backup_manifest(&manifest)?;
            remember_backup(BackupReceipt {
                datadir: current.datadir,
                destination: destination.clone(),
                created_at,
                expires_at,
                files,
                bytes,
                ibdata,
                frm,
                business_schema: business,
                manifest_path: manifest_path.clone(),
                manifest_hash: manifest_hash.clone(),
            })?;
            Ok(format!("Data 备份完成：{files} 个文件，{bytes} 字节；ibdata1={}，业务库目录={}，.frm={}；目标：{destination}；manifest：{manifest_path}；回执：{manifest_hash}", ibdata, business, frm))
        }
        "register_service" => {
            if !Path::new(&current.mysqld_path).is_file() || !Path::new(&current.my_ini_path).is_file() { return Err("mysqld.exe 或 my.ini 已不存在".to_string()); }
            let output = hidden_command(&current.mysqld_path).args(["--install", &current.service_name, &format!("--defaults-file={}", current.my_ini_path)]).output().map_err(|e| format!("注册服务失败：{e}"))?;
            if !output.status.success() { return Err(format!("注册服务失败：{}", crate::command_text(&output.stdout, &output.stderr))); }
            let output = hidden_command("sc.exe").args(["config", &current.service_name, "start=", "auto"]).output().map_err(|e| format!("设置服务失败：{e}"))?;
            if !output.status.success() { return Err(format!("设置自动启动失败：{}", crate::command_text(&output.stdout, &output.stderr))); }
            Ok(format!("已注册服务 {}；请重新诊断后再启动", current.service_name))
        }
        "start_service" => {
            let output = hidden_command("sc.exe").args(["start", &current.service_name]).output().map_err(|e| format!("启动服务失败：{e}"))?;
            if !output.status.success() { return Err(format!("启动服务失败：{}", crate::command_text(&output.stdout, &output.stderr))); }
            Ok(format!("已请求启动服务 {}；请重新诊断确认状态和端口", current.service_name))
        }
        "repair_system_schema" => {
            let receipt = recent_valid_backup(&current.datadir).ok_or_else(|| "没有找到近 24 小时内由本程序完成且 manifest 校验通过的 Data 备份，禁止修复系统库".to_string())?;
            let source = Path::new(&current.basedir).join("data").join("mysql");
            let target = Path::new(&current.datadir).join("mysql");
            copy_missing_system_schema(&source, &target)?;
            Ok(format!("已从安装目录补回 mysql 系统库；未覆盖业务库、ibdata1 或日志。安全备份：{}。请重新诊断后手动启动服务", receipt.destination))
        }
        "reset_root_guide" => Ok(format!("认证恢复向导（不会执行或记录密码）：\n1. sc stop {}\n2. \"{}\" --defaults-file=\"{}\" --skip-grant-tables --console\n3. 另开管理员终端运行 mysql.exe -u root\n4. MySQL 5.5/5.7 使用 mysql.user 的 Password 兼容语法；MySQL 8.0 必须使用 ALTER USER。输入密码只在数据库终端完成。", current.service_name, current.mysqld_path, current.my_ini_path)),
        "dump_guide" => Ok(pending.public.commands.join("\n")),
        _ => Err("不支持的 MySQL 修复动作".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mysql_ini_without_reading_database_content() {
        let temp = tempfile::tempdir().unwrap();
        let ini = temp.path().join("my.ini");
        fs::write(
            &ini,
            "[mysqld]\nbasedir=\"C:\\MySQL\"\ndatadir=C:\\Data\nport=3307\n",
        )
        .unwrap();
        let parsed = parse_ini(&ini);
        assert_eq!(parsed.get("port").map(String::as_str), Some("3307"));
        assert_eq!(parsed.get("datadir").map(String::as_str), Some("C:\\Data"));
    }

    #[test]
    fn backup_copy_reports_critical_files_and_business_schema() {
        let source = tempfile::tempdir().unwrap();
        fs::write(source.path().join("ibdata1"), b"x").unwrap();
        fs::create_dir(source.path().join("zuoye")).unwrap();
        fs::write(source.path().join("zuoye").join("user.frm"), b"y").unwrap();
        let target_root = tempfile::tempdir().unwrap();
        let target = target_root.path().join("backup");
        let (_, files, ibdata, frm, business) = copy_tree(source.path(), &target).unwrap();
        assert_eq!(files, 2);
        assert!(ibdata && frm && business);
    }

    #[test]
    fn system_schema_copy_never_overwrites_existing_target() {
        let source = tempfile::tempdir().unwrap();
        fs::write(source.path().join("user.frm"), b"source").unwrap();
        let target_root = tempfile::tempdir().unwrap();
        let target = target_root.path().join("mysql");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("user.frm"), b"existing").unwrap();
        assert!(copy_missing_system_schema(source.path(), &target).is_err());
        assert_eq!(fs::read(target.join("user.frm")).unwrap(), b"existing");
    }

    #[test]
    fn backup_rejects_normalized_destination_inside_source() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("data");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("ibdata1"), b"x").unwrap();
        let disguised = root
            .path()
            .join("outside")
            .join("..")
            .join("data")
            .join("backup");
        assert!(copy_tree(&source, &disguised).is_err());
    }

    #[test]
    fn mysql_error_report_redacts_credential_shaped_lines() {
        let text = "normal error\npassword=hunter2\n1045 using password: YES";
        let redacted = redact_error_log(text);
        assert!(!redacted.contains("hunter2"));
        assert!(redacted.contains("1045 using password: YES"));
    }
}
