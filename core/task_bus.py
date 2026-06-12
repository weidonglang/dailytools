from __future__ import annotations

import queue
import threading
import uuid
from dataclasses import dataclass, field
from datetime import datetime
from typing import Any, Callable


@dataclass
class BackgroundTask:
    name: str
    id: str = field(default_factory=lambda: uuid.uuid4().hex)
    status: str = "pending"
    progress: int = 0
    message: str = "等待中"
    started_at: datetime | None = None
    finished_at: datetime | None = None
    result: Any = None
    error: str | None = None


class TaskBus:
    def __init__(self) -> None:
        self.tasks: list[BackgroundTask] = []
        self.events: queue.Queue[BackgroundTask] = queue.Queue()

    def submit(self, name: str, func: Callable[[Callable[[int, str], None]], Any]) -> BackgroundTask:
        task = BackgroundTask(name=name)
        self.tasks.append(task)

        def update(progress: int, message: str) -> None:
            task.progress = max(0, min(100, int(progress)))
            task.message = message
            self.events.put(task)

        def worker() -> None:
            task.status = "running"
            task.started_at = datetime.now()
            update(0, "正在执行")
            try:
                task.result = func(update)
                task.status = "success"
                update(100, "已完成")
            except Exception as exc:
                task.status = "failed"
                task.error = str(exc)
                task.message = str(exc)
                self.events.put(task)
            finally:
                task.finished_at = datetime.now()

        threading.Thread(target=worker, name=f"task-{name}", daemon=True).start()
        return task
