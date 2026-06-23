mod architecture;
mod disk;
mod model;
mod protect;
mod report;
mod scan;
mod utils;

#[allow(unused_imports)]
pub use architecture::{architecture, CleanupArchitecture, CleanupCategory};
pub use disk::inspect_disk_overview;
pub use model::{CleanupScanReport, DiskVolumeInfo, MaintenanceOverview};
#[allow(unused_imports)]
pub use protect::{
    classify_path_risk, is_inside_managed_runtime, is_inside_user_profile, is_protected_path,
    should_skip_path, CleanRisk,
};
pub use report::inspect_maintenance_overview;
pub use scan::scan_cleanup_targets;

/// Backwards-compatible entry point used by the CLI and older frontends.
pub fn scan(managed_root: &std::path::Path) -> Result<CleanupScanReport, String> {
    scan_cleanup_targets(managed_root)
}
