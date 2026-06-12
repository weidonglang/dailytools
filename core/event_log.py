from __future__ import annotations

import logging
import threading
from collections import deque
from datetime import datetime
from pathlib import Path
from typing import Callable


class EventLog:
    def __init__(self, log_file: Path, max_items: int = 1000) -> None:
        self.log_file = log_file
        self.items: deque[str] = deque(maxlen=max_items)
        self._subscribers: list[Callable[[str], None]] = []
        self._lock = threading.RLock()
        log_file.parent.mkdir(parents=True, exist_ok=True)
        self._logger = logging.getLogger(f"devenv.{id(self)}")
        self._logger.setLevel(logging.INFO)
        self._logger.propagate = False
        handler = logging.FileHandler(log_file, encoding="utf-8")
        handler.setFormatter(logging.Formatter("%(asctime)s [%(levelname)s] %(message)s"))
        self._logger.addHandler(handler)
        self._load_recent()

    def _load_recent(self) -> None:
        try:
            lines = self.log_file.read_text(encoding="utf-8").splitlines()
            self.items.extend(lines[-self.items.maxlen :])
        except OSError:
            pass

    def subscribe(self, callback: Callable[[str], None]) -> None:
        self._subscribers.append(callback)

    def write(self, message: str, level: str = "info") -> None:
        timestamped = f"{datetime.now():%H:%M:%S}  {message}"
        with self._lock:
            self.items.append(timestamped)
            getattr(self._logger, level if level in {"info", "warning", "error"} else "info")(message)
            for callback in list(self._subscribers):
                try:
                    callback(timestamped)
                except Exception:
                    continue
