mod app_usage;
mod architecture;
mod clean_plan;
mod clean_report;
mod desktop;
mod dev_cache;
mod disk;
mod downloads;
mod duplicates;
mod game_usage;
mod large_files;
mod model;
mod protect;
mod report;
mod safe_clean;
mod scan;
mod software;
mod utils;

pub use app_usage::inspect_app_usage;
#[allow(unused_imports)]
pub use architecture::{architecture, CleanupArchitecture, CleanupCategory};
pub use clean_plan::create_cleanup_plan;
pub use clean_report::export_cleanup_report;
pub use desktop::inspect_desktop;
pub use dev_cache::clean_dev_cache;
pub use disk::inspect_disk_overview;
pub use downloads::inspect_downloads;
pub use duplicates::scan_duplicate_large_files;
pub use large_files::scan_large_files;
pub use model::{
    AppUsageReport, CleanupPlan, CleanupResult, CleanupScanReport, DiskVolumeInfo, DuplicateGroup,
    FolderUsageReport, InstalledSoftwareUsage, LargeFileItem, MaintenanceOverview,
};
#[allow(unused_imports)]
pub use protect::{
    classify_path_risk, is_inside_managed_runtime, is_inside_user_profile, is_protected_path,
    should_skip_path, CleanRisk,
};
pub use report::inspect_maintenance_overview;
pub use safe_clean::{clean_managed_download_cache, clean_selected_targets};
pub use scan::scan_cleanup_targets;
pub use software::inspect_installed_software_usage;

/// Backwards-compatible entry point used by the CLI and older frontends.
pub fn scan(managed_root: &std::path::Path) -> Result<CleanupScanReport, String> {
    scan_cleanup_targets(managed_root)
}
