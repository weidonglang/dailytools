from __future__ import annotations

from core.config_store import ConfigService
from core.task_bus import TaskBus


def collect_metrics(config: ConfigService, tasks: TaskBus, port_count: int = 0) -> dict[str, int]:
    installed = config.installed()
    return {
        "JDK 数量": len(installed["jdks"]),
        "Python 数量": len(installed["pythons"]),
        "Node.js 数量": len(installed["nodes"]),
        "任务成功": sum(task.status == "success" for task in tasks.tasks),
        "任务失败": sum(task.status == "failed" for task in tasks.tasks),
        "端口记录": port_count,
    }
