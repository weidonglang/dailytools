use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MemorySummary {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub used_percent: f64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiskVolumeInfo {
    pub drive: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
    pub used_percent: f64,
    pub file_system: Option<String>,
    pub risk: String,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MaintenanceOverview {
    pub c_drive: DiskVolumeInfo,
    pub volumes: Vec<DiskVolumeInfo>,
    pub safe_clean_estimate: u64,
    pub move_estimate: u64,
    pub dev_cache_estimate: u64,
    pub large_file_count: usize,
    pub startup_count: usize,
    pub memory_summary: Option<MemorySummary>,
    pub risk_level: String,
    pub summary: String,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CleanupScanReport {
    pub generated_at: String,
    pub total_bytes: u64,
    pub total_items: usize,
    pub categories: Vec<CleanupCategoryScan>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CleanupCategoryScan {
    pub id: String,
    pub name: String,
    pub description: String,
    pub risk: String,
    pub scan_only: bool,
    pub cleanable: bool,
    pub enabled_by_default: bool,
    pub total_bytes: u64,
    pub item_count: usize,
    pub items: Vec<CleanupItem>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CleanupItem {
    pub id: String,
    pub path: String,
    pub size: u64,
    pub modified_at: Option<String>,
    pub source: String,
    pub reason: String,
    pub risk: String,
    pub cleanable: bool,
    pub selected_by_default: bool,
    pub skipped_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPlan {
    pub plan_id: String,
    pub created_at: String,
    pub selected_items: Vec<CleanupPlanItem>,
    pub estimated_bytes: u64,
    pub risk_summary: Vec<String>,
    pub requires_admin: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPlanItem {
    pub item_id: String,
    pub path: String,
    pub size: u64,
    pub category_id: String,
    pub risk: String,
    pub action: String,
    pub reversible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CleanupResult {
    pub plan_id: String,
    pub started_at: String,
    pub finished_at: String,
    pub success: bool,
    pub cleaned_bytes: u64,
    pub cleaned_items: usize,
    pub skipped_items: usize,
    pub failed_items: usize,
    pub failures: Vec<CleanupFailure>,
    pub report_markdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CleanupFailure {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LargeFileItem {
    pub path: String,
    pub size: u64,
    pub modified_at: Option<String>,
    pub file_type: String,
    pub suggestion: String,
    pub risk: String,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub size: u64,
    pub hash: String,
    pub files: Vec<DuplicateFileItem>,
    pub reclaimable_estimate: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateFileItem {
    pub path: String,
    pub modified_at: Option<String>,
    pub keep_suggestion: String,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FolderUsageItem {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub category: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FolderUsageReport {
    pub name: String,
    pub path: String,
    pub total_bytes: u64,
    pub categories: Vec<FolderUsageItem>,
    pub suggestions: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppUsageReport {
    pub wechat: Option<AppUsageItem>,
    pub qq: Option<AppUsageItem>,
    pub browsers: Vec<AppUsageItem>,
    pub net_disks: Vec<AppUsageItem>,
    pub video_editors: Vec<AppUsageItem>,
    pub game_platforms: Vec<AppUsageItem>,
    pub installed_software: Vec<InstalledSoftwareUsage>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppUsageItem {
    pub name: String,
    pub detected: bool,
    pub path: String,
    pub size: u64,
    pub categories: Vec<FolderUsageItem>,
    pub safe_actions: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InstalledSoftwareUsage {
    pub name: String,
    pub publisher: String,
    pub install_location: String,
    pub estimated_size: u64,
    pub uninstall_command_exists: bool,
    pub suggestion: String,
}
