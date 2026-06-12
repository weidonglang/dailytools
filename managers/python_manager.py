from __future__ import annotations

import re
import shutil
import subprocess
import os
from pathlib import Path

from packaging.version import Version

from core.downloader import download_file
from core.http_client import HTTP, get_text
from managers.base_manager import BaseRuntimeManager, Progress


class PythonManager(BaseRuntimeManager):
    kind = "python"
    collection = "pythons"
    executable = "python"
    supported_versions = ("3.10", "3.11")

    def find_existing_install(self, version: str) -> Path | None:
        candidates: list[Path] = []
        if os.name == "nt":
            import winreg

            for hive in (winreg.HKEY_CURRENT_USER, winreg.HKEY_LOCAL_MACHINE):
                for access in (winreg.KEY_READ, winreg.KEY_READ | winreg.KEY_WOW64_64KEY, winreg.KEY_READ | winreg.KEY_WOW64_32KEY):
                    try:
                        with winreg.OpenKey(hive, rf"Software\Python\PythonCore\{version}\InstallPath", 0, access) as key:
                            candidates.append(Path(winreg.QueryValue(key, "")))
                    except OSError:
                        continue
        for candidate in candidates:
            executable = candidate / "python.exe"
            if executable.exists():
                result = self.verify(executable, ["--version"])
                if f"Python {version}." in result:
                    return candidate.resolve()
        return None

    def resolve_release(self, version: str) -> dict[str, str]:
        if version not in self.supported_versions:
            raise ValueError(f"暂不支持 Python {version}")
        index_text = get_text("https://www.python.org/ftp/python/")
        versions = {
            match for match in re.findall(r'href="(\d+\.\d+\.\d+)/"', index_text)
            if match.startswith(version + ".")
        }
        if not versions:
            raise RuntimeError(f"未找到 Python {version} 的官方安装器")
        for candidate in sorted((Version(item) for item in versions), reverse=True):
            full_version = str(candidate)
            filename = f"python-{full_version}-amd64.exe"
            url = f"https://www.python.org/ftp/python/{full_version}/{filename}"
            check = HTTP.head(url, timeout=20, allow_redirects=True)
            if check.status_code < 400:
                return {"url": url, "name": filename, "full_version": full_version}
        raise RuntimeError(f"Python {version} 没有可用的 Windows x64 安装器")

    def install(self, version: str, progress: Progress) -> Path:
        self.event_log.write(f"开始安装 Python {version}")
        progress(2, "正在查询 Python 官方版本")
        release = self.resolve_release(version)
        installer = self.config.paths.downloads / release["name"]
        target = self.config.paths.pythons / f"python-{version}"
        self.config.paths.assert_inside_root(target)
        if target.exists():
            raise RuntimeError(f"Python {version} 已安装，如目录损坏请先手动移走：{target}")
        existing = self.find_existing_install(version)
        if existing and existing != target.resolve():
            progress(35, f"检测到现有 Python {version}，正在创建受管副本")
            try:
                shutil.copytree(
                    existing,
                    target,
                    ignore=shutil.ignore_patterns("__pycache__", "*.pyc", "*.pyo"),
                )
            except OSError as exc:
                raise RuntimeError(f"复制现有 Python 失败：{exc}") from exc
            return self._finalize_install(version, release, target, progress)
        download_file(
            release["url"],
            installer,
            lambda done, total: progress(8 + int(done * 55 / total) if total else 30, "正在下载安装器"),
            int(self.config.settings["download_timeout_seconds"]),
        )
        progress(66, "正在静默安装 Python")
        args = [
            str(installer),
            "/quiet",
            "InstallAllUsers=0",
            f"TargetDir={target}",
            "PrependPath=0",
            "AppendPath=0",
            "Include_launcher=0",
            "Include_pip=1",
            "Include_test=0",
            "Include_doc=0",
        ]
        try:
            completed = subprocess.run(
                args,
                timeout=600,
                capture_output=True,
                text=True,
                creationflags=subprocess.CREATE_NO_WINDOW,
            )
        except (OSError, subprocess.SubprocessError) as exc:
            raise RuntimeError(f"Python 安装器执行失败：{exc}") from exc
        if completed.returncode != 0:
            raise RuntimeError(f"Python 安装失败，退出码 {completed.returncode}")
        return self._finalize_install(version, release, target, progress)

    def _finalize_install(self, version: str, release: dict[str, str], target: Path, progress: Progress) -> Path:
        python_exe = target / "python.exe"
        progress(90, "正在验证 Python 和 pip")
        output = self.verify(python_exe, ["--version"])
        self.verify(python_exe, ["-m", "pip", "--version"])
        self.record_install(
            version,
            target,
            python_exe,
            {"detail": output.splitlines()[0] if output else release["full_version"]},
        )
        self.switch(version)
        self.event_log.write(f"安装成功 Python {version}")
        return target

    def create_venv(self, version: str, project_dir: Path) -> Path:
        record = next((item for item in self.list_installed() if item["version"] == version), None)
        if not record:
            raise RuntimeError(f"尚未安装 Python {version}")
        venv = project_dir / ".venv"
        result = self.verify(Path(record["python_exe"]), ["-m", "venv", str(venv)])
        self.event_log.write(f"已创建虚拟环境：{venv}")
        return venv
