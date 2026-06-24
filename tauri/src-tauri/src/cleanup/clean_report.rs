use super::model::CleanupResult;
use std::sync::{Mutex, OnceLock};

static LAST_RESULT: OnceLock<Mutex<Option<CleanupResult>>> = OnceLock::new();

fn result_store() -> &'static Mutex<Option<CleanupResult>> {
    LAST_RESULT.get_or_init(|| Mutex::new(None))
}

pub(crate) fn render_markdown(result: &CleanupResult) -> String {
    let mut report = format!(
        "# DevEnv Manager 清理报告\n\n- 计划：`{}`\n- 开始：{}\n- 完成：{}\n- 状态：{}\n- 释放空间：{} bytes\n- 已清理：{} 项\n- 已跳过：{} 项\n- 失败：{} 项\n",
        result.plan_id,
        result.started_at,
        result.finished_at,
        if result.success { "完成" } else { "部分完成" },
        result.cleaned_bytes,
        result.cleaned_items,
        result.skipped_items,
        result.failed_items,
    );
    if !result.failures.is_empty() {
        report.push_str("\n## 失败项目\n\n");
        for failure in &result.failures {
            report.push_str(&format!("- `{}`：{}\n", failure.path, failure.reason));
        }
    }
    report.push_str(
        "\n> 清理项在执行前经过重新扫描和保护规则校验；普通文件优先移入 Windows 回收站。\n",
    );
    report
}

pub(crate) fn store_result(mut result: CleanupResult) -> CleanupResult {
    result.report_markdown = render_markdown(&result);
    if let Ok(mut slot) = result_store().lock() {
        *slot = Some(result.clone());
    }
    result
}

pub fn export_cleanup_report(format: &str) -> Result<String, String> {
    let result = result_store()
        .lock()
        .map_err(|_| "清理报告暂时不可用".to_string())?
        .clone()
        .ok_or_else(|| "还没有可导出的清理报告".to_string())?;
    match format.trim().to_ascii_lowercase().as_str() {
        "markdown" | "md" => Ok(result.report_markdown),
        "json" => serde_json::to_string_pretty(&result)
            .map_err(|error| format!("生成 JSON 报告失败：{error}")),
        _ => Err("仅支持导出 Markdown 或 JSON".to_string()),
    }
}
