use super::model::{CleanupPlan, CleanupPlanItem, CleanupScanReport};
use super::scan::scan_cleanup_targets;
use super::utils::generated_at;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Mutex, OnceLock};

static PLANS: OnceLock<Mutex<HashMap<String, CleanupPlan>>> = OnceLock::new();

fn plan_store() -> &'static Mutex<HashMap<String, CleanupPlan>> {
    PLANS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn plan_from_report(report: &CleanupScanReport, selected_item_ids: &[String]) -> CleanupPlan {
    let selected: HashSet<&str> = selected_item_ids.iter().map(String::as_str).collect();
    let mut selected_items = Vec::new();
    let mut warnings = Vec::new();
    let mut found = HashSet::new();
    for category in &report.categories {
        for item in &category.items {
            if !selected.contains(item.id.as_str()) {
                continue;
            }
            found.insert(item.id.as_str());
            if !item.cleanable || matches!(item.risk.as_str(), "high" | "critical") {
                warnings.push(format!("已排除不可自动清理项：{}", item.path));
                continue;
            }
            selected_items.push(CleanupPlanItem {
                item_id: item.id.clone(),
                path: item.path.clone(),
                size: item.size,
                category_id: category.id.clone(),
                risk: item.risk.clone(),
                action: "move_to_recycle_bin".to_string(),
                reversible: true,
            });
        }
    }
    let missing = selected.len().saturating_sub(found.len());
    if missing > 0 {
        warnings.push(format!("{missing} 个选择项已失效或不属于本轮扫描"));
    }
    selected_items.sort_by(|a, b| a.item_id.cmp(&b.item_id));
    let estimated_bytes = selected_items.iter().map(|item| item.size).sum();
    let mut risk_summary = selected_items
        .iter()
        .map(|item| item.risk.clone())
        .collect::<Vec<_>>();
    risk_summary.sort();
    risk_summary.dedup();
    let created_at = generated_at();
    let mut hasher = Sha256::new();
    hasher.update(created_at.as_bytes());
    hasher.update(std::process::id().to_le_bytes());
    for item in &selected_items {
        hasher.update(item.item_id.as_bytes());
    }
    let digest = format!("{:x}", hasher.finalize());
    CleanupPlan {
        plan_id: format!("cleanup-{}", &digest[..16]),
        created_at,
        selected_items,
        estimated_bytes,
        risk_summary,
        requires_admin: false,
        warnings,
    }
}

pub fn create_cleanup_plan(
    managed_root: &Path,
    selected_item_ids: Vec<String>,
) -> Result<CleanupPlan, String> {
    if selected_item_ids.is_empty() {
        return Err("请至少选择一个可清理项目".to_string());
    }
    let report = scan_cleanup_targets(managed_root)?;
    let plan = plan_from_report(&report, &selected_item_ids);
    if plan.selected_items.is_empty() {
        return Err("所选项目均受保护、风险过高或已经失效".to_string());
    }
    let mut store = plan_store()
        .lock()
        .map_err(|_| "清理计划存储暂时不可用".to_string())?;
    store.retain(|_, value| {
        value
            .created_at
            .parse::<u64>()
            .ok()
            .is_some_and(|created| created.saturating_add(30 * 60) >= current_epoch())
    });
    store.insert(plan.plan_id.clone(), plan.clone());
    Ok(plan)
}

fn current_epoch() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0)
}

pub(crate) fn take_valid_plan(plan: &CleanupPlan) -> Result<CleanupPlan, String> {
    let stored = plan_store()
        .lock()
        .map_err(|_| "清理计划存储暂时不可用".to_string())?
        .remove(&plan.plan_id)
        .ok_or_else(|| "清理计划不存在、已执行或已经过期，请重新扫描并预览".to_string())?;
    if &stored != plan {
        return Err("清理计划内容已发生变化，请重新扫描并预览".to_string());
    }
    let created = stored
        .created_at
        .parse::<u64>()
        .map_err(|_| "清理计划时间无效".to_string())?;
    if created.saturating_add(30 * 60) < current_epoch() {
        return Err("清理计划已超过 30 分钟，请重新扫描".to_string());
    }
    Ok(stored)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cleanup::model::{CleanupCategoryScan, CleanupItem};

    fn report() -> CleanupScanReport {
        CleanupScanReport {
            categories: vec![CleanupCategoryScan {
                id: "temp".to_string(),
                items: vec![
                    CleanupItem {
                        id: "safe".to_string(),
                        path: r"C:\Users\test\AppData\Local\Temp\old.tmp".to_string(),
                        size: 10,
                        risk: "medium".to_string(),
                        cleanable: true,
                        ..Default::default()
                    },
                    CleanupItem {
                        id: "high".to_string(),
                        path: r"C:\Windows\Temp\x.tmp".to_string(),
                        size: 20,
                        risk: "high".to_string(),
                        cleanable: false,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn plan_contains_only_user_selection() {
        let plan = plan_from_report(&report(), &["safe".to_string()]);
        assert_eq!(plan.selected_items.len(), 1);
        assert_eq!(plan.selected_items[0].item_id, "safe");
        assert_eq!(plan.estimated_bytes, 10);
    }

    #[test]
    fn high_risk_item_never_enters_plan() {
        let plan = plan_from_report(&report(), &["safe".to_string(), "high".to_string()]);
        assert_eq!(plan.selected_items.len(), 1);
        assert!(plan.warnings.iter().any(|item| item.contains("排除")));
    }
}
