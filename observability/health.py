from __future__ import annotations

from core.doctor import DiagnosticItem


def summarize_health(items: list[DiagnosticItem]) -> tuple[str, str]:
    if any(item.status == "ERROR" for item in items):
        return "异常", "#dc2626"
    if any(item.status == "WARNING" for item in items):
        return "需要处理", "#d97706"
    return "正常", "#16a34a"
