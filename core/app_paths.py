from __future__ import annotations

import os
import string
from dataclasses import dataclass
from pathlib import Path


APP_NAME = "DevEnvManager"


def default_root_dir() -> Path:
    for letter in string.ascii_uppercase[3:]:
        drive = Path(f"{letter}:/")
        if drive.exists() and letter == "D":
            return drive / APP_NAME
    return Path(os.environ.get("USERPROFILE", str(Path.home()))) / APP_NAME


def app_config_dir() -> Path:
    base = Path(os.environ.get("LOCALAPPDATA", str(Path.home())))
    path = base / APP_NAME
    path.mkdir(parents=True, exist_ok=True)
    return path


@dataclass(frozen=True)
class AppPaths:
    root: Path

    @property
    def envs(self) -> Path:
        return self.root / "envs"

    @property
    def jdks(self) -> Path:
        return self.envs / "jdks"

    @property
    def pythons(self) -> Path:
        return self.envs / "pythons"

    @property
    def nodes(self) -> Path:
        return self.envs / "nodes"

    @property
    def current(self) -> Path:
        return self.root / "current"

    @property
    def downloads(self) -> Path:
        return self.root / "downloads"

    @property
    def config(self) -> Path:
        return self.root / "config"

    @property
    def logs(self) -> Path:
        return self.root / "logs"

    @property
    def settings_file(self) -> Path:
        return app_config_dir() / "settings.json"

    @property
    def installed_file(self) -> Path:
        return self.config / "installed.json"

    @property
    def env_backup_file(self) -> Path:
        return self.config / "env_backup.json"

    @property
    def log_file(self) -> Path:
        return self.logs / "app.log"

    def ensure(self) -> None:
        for path in (
            self.root,
            self.jdks,
            self.pythons,
            self.nodes,
            self.current,
            self.downloads,
            self.config,
            self.logs,
        ):
            path.mkdir(parents=True, exist_ok=True)

    def assert_inside_root(self, path: Path) -> Path:
        root = self.root.resolve()
        candidate = path.resolve()
        if candidate != root and root not in candidate.parents:
            raise ValueError(f"目标路径不在安装根目录内：{candidate}")
        return candidate
