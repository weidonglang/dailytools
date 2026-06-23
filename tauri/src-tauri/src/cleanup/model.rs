use serde::Serialize;

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
