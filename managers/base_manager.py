from __future__ import annotations

from datetime import datetime
from pathlib import Path
from typing import Callable

from core.config_store import ConfigService
from core.env_var import switch_junction
from core.event_log import EventLog
from core.process_runner import run_command


Progress = Callable[[int, str], None]


class BaseRuntimeManager:
    kind = ""
    collection = ""
    executable = ""

    def __init__(self, config: ConfigService, event_log: EventLog) -> None:
        self.config = config
        self.event_log = event_log

    def list_installed(self) -> list[dict]:
        return self.config.installed().get(self.collection, [])

    def current_version(self) -> str | None:
        return self.config.installed().get("current", {}).get(self.kind)

    def record_install(self, version: str, path: Path, executable_path: Path, extra: dict | None = None) -> None:
        data = self.config.installed()
        records = [item for item in data[self.collection] if item.get("version") != version]
        record = {
            "version": version,
            "path": str(path),
            f"{self.executable}_exe": str(executable_path),
            "installed_at": datetime.now().isoformat(timespec="seconds"),
        }
        if extra:
            record.update(extra)
        records.append(record)
        data[self.collection] = records
        self.config.update_installed(data)

    def switch(self, version: str) -> None:
        record = next((item for item in self.list_installed() if item.get("version") == version), None)
        if not record:
            raise RuntimeError(f"尚未安装 {self.kind} {version}")
        target = Path(record["path"])
        if not target.exists():
            raise RuntimeError(f"版本目录不存在：{target}")
        switch_junction(
            self.config.paths.current / self.kind,
            target,
            self.config.paths.root,
        )
        data = self.config.installed()
        data["current"][self.kind] = version
        self.config.update_installed(data)
        self.event_log.write(f"已切换当前 {self.kind} 到 {version}")

    def verify(self, executable: Path, args: list[str]) -> str:
        result = run_command([str(executable), *args], timeout=30)
        if not result.success:
            raise RuntimeError(result.output or f"{executable.name} 验证失败")
        return result.output
