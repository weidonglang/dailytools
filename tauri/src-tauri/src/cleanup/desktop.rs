use super::downloads::inspect_folder;
use super::model::FolderUsageReport;

pub fn inspect_desktop() -> FolderUsageReport {
    let root =
        dirs::desktop_dir().unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join("Desktop"));
    inspect_folder(&root, true)
}
