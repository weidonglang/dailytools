mod app_usage;
mod architecture;
mod clean_plan;
mod clean_report;
mod desktop;
mod dev_cache;
mod disk;
mod downloads;
mod duplicates;
mod expansion;
mod game_usage;
mod junction;
mod large_files;
mod migration;
mod model;
mod move_plan;
mod partition;
mod protect;
mod report;
mod rollback;
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
pub use duplicates::scan_duplicate_large_files_with_progress;
pub use expansion::{create_c_drive_expansion_plan, execute_c_drive_expansion};
pub use large_files::scan_large_files_with_progress;
pub use migration::execute_move_plan;
pub use model::{
    AppUsageReport, CleanupPlan, CleanupResult, CleanupScanReport, DiskVolumeInfo, DuplicateGroup,
    ExpansionPlan, ExpansionResult, FolderUsageReport, InstalledSoftwareUsage, LargeFileItem,
    MaintenanceOverview, MovePlan, MoveResult, PartitionLayoutReport, RollbackRecord,
};
pub use move_plan::{
    create_desktop_archive_plan, create_downloads_archive_plan, create_junction_bridge,
    create_move_plan, execute_desktop_archive_plan, execute_downloads_archive_plan,
};
pub use partition::inspect_partition_layout;
#[allow(unused_imports)]
pub use protect::{
    classify_path_risk, is_inside_managed_runtime, is_inside_user_profile, is_protected_path,
    should_skip_path, CleanRisk,
};
pub use report::inspect_maintenance_overview;
pub use rollback::{list_rollback_records, rollback_move};
pub use safe_clean::{clean_managed_download_cache, clean_selected_targets};
pub use scan::scan_cleanup_targets;
pub use software::inspect_installed_software_usage;

/// Backwards-compatible entry point used by the CLI and older frontends.
pub fn scan(managed_root: &std::path::Path) -> Result<CleanupScanReport, String> {
    scan_cleanup_targets(managed_root)
}
