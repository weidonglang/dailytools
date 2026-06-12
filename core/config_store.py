from __future__ import annotations

import json
import threading
from copy import deepcopy
from pathlib import Path
from typing import Any

from core.app_paths import AppPaths, default_root_dir


DEFAULT_SETTINGS = {
    "root_dir": str(default_root_dir()),
    "auto_check_update": False,
    "download_timeout_seconds": 60,
    "theme": "system",
    "last_page": "home",
}

DEFAULT_INSTALLED = {
    "jdks": [],
    "pythons": [],
    "nodes": [],
    "current": {"jdk": None, "python": None, "node": None},
}


class JsonStore:
    def __init__(self, path: Path, defaults: dict[str, Any]) -> None:
        self.path = path
        self.defaults = defaults
        self._lock = threading.RLock()

    def load(self) -> dict[str, Any]:
        with self._lock:
            if not self.path.exists():
                data = deepcopy(self.defaults)
                self.save(data)
                return data
            try:
                with self.path.open("r", encoding="utf-8") as handle:
                    data = json.load(handle)
                return _merge_defaults(data, self.defaults)
            except (OSError, json.JSONDecodeError):
                backup = self.path.with_suffix(self.path.suffix + ".broken")
                try:
                    self.path.replace(backup)
                except OSError:
                    pass
                data = deepcopy(self.defaults)
                self.save(data)
                return data

    def save(self, data: dict[str, Any]) -> None:
        with self._lock:
            self.path.parent.mkdir(parents=True, exist_ok=True)
            temp = self.path.with_suffix(self.path.suffix + ".tmp")
            with temp.open("w", encoding="utf-8") as handle:
                json.dump(data, handle, ensure_ascii=False, indent=2)
            temp.replace(self.path)


def _merge_defaults(data: dict[str, Any], defaults: dict[str, Any]) -> dict[str, Any]:
    merged = deepcopy(defaults)
    for key, value in data.items():
        if isinstance(value, dict) and isinstance(merged.get(key), dict):
            merged[key] = _merge_defaults(value, merged[key])
        else:
            merged[key] = value
    return merged


class ConfigService:
    def __init__(self) -> None:
        bootstrap = AppPaths(default_root_dir())
        self.settings_store = JsonStore(bootstrap.settings_file, DEFAULT_SETTINGS)
        self.settings = self.settings_store.load()
        self.paths = AppPaths(Path(self.settings["root_dir"]).expanduser())
        self.paths.ensure()
        self.installed_store = JsonStore(self.paths.installed_file, DEFAULT_INSTALLED)

    def set_root(self, root: Path) -> None:
        root = root.expanduser().absolute()
        self.settings["root_dir"] = str(root)
        self.settings_store.save(self.settings)
        self.paths = AppPaths(root)
        self.paths.ensure()
        self.installed_store = JsonStore(self.paths.installed_file, DEFAULT_INSTALLED)
        self.installed_store.load()

    def save_setting(self, key: str, value: Any) -> None:
        self.settings[key] = value
        self.settings_store.save(self.settings)

    def installed(self) -> dict[str, Any]:
        return self.installed_store.load()

    def update_installed(self, data: dict[str, Any]) -> None:
        self.installed_store.save(data)
