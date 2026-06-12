from __future__ import annotations

import os
import re
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path

from core.config_store import ConfigService
from core.process_runner import run_command

if os.name == "nt":
    import winreg


@dataclass(frozen=True)
class RuntimeInstallation:
    kind: str
    version: str
    path: Path
    executable: Path
    source: str
    managed: bool = False
    current: bool = False
    managed_id: str | None = None


def discover_all(config: ConfigService) -> list[RuntimeInstallation]:
    installations = _managed_installations(config)
    candidates = {
        "jdk": _discover_jdks(),
        "python": _discover_pythons(),
        "node": _discover_nodes(),
    }
    managed_paths = {item.path.resolve() for item in installations if item.path.exists()}
    for kind, items in candidates.items():
        for path, executable, source in items:
            try:
                resolved_path = path.resolve()
            except OSError:
                continue
            if resolved_path in managed_paths:
                continue
            version = detect_version(kind, executable)
            if version:
                installations.append(
                    RuntimeInstallation(kind, version, resolved_path, executable.resolve(), source)
                )
    unique: dict[tuple[str, str], RuntimeInstallation] = {}
    for item in installations:
        unique[(item.kind, str(item.path).casefold())] = item
    return sorted(unique.values(), key=lambda item: (item.kind, _version_key(item.version), str(item.path)), reverse=False)


def detect_version(kind: str, executable: Path) -> str:
    args = {
        "jdk": ["-version"],
        "python": ["--version"],
        "node": ["-v"],
    }[kind]
    result = run_command([str(executable), *args], timeout=15)
    if not result.success:
        return ""
    text = result.output
    patterns = {
        "jdk": r'(?:version\s+")?(\d+(?:\.\d+){0,3})',
        "python": r"Python\s+(\d+(?:\.\d+){1,2})",
        "node": r"v(\d+(?:\.\d+){1,2})",
    }
    match = re.search(patterns[kind], text, re.IGNORECASE)
    if not match:
        return text.splitlines()[0] if text else "未知"
    version = match.group(1)
    if kind == "jdk" and version.startswith("1.8."):
        return "8" + version[3:]
    return version


def _managed_installations(config: ConfigService) -> list[RuntimeInstallation]:
    data = config.installed()
    mapping = (
        ("jdk", "jdks", "bin/java.exe"),
        ("python", "pythons", "python.exe"),
        ("node", "nodes", "node.exe"),
    )
    result = []
    for kind, collection, relative_executable in mapping:
        for record in data.get(collection, []):
            path = Path(record["path"])
            executable = path / relative_executable
            if not executable.exists():
                continue
            detail = record.get("detail", "")
            version = _detail_version(kind, detail) or detect_version(kind, executable) or record["version"]
            result.append(
                RuntimeInstallation(
                    kind=kind,
                    version=version,
                    path=path,
                    executable=executable,
                    source="DevEnv 管理",
                    managed=True,
                    current=data.get("current", {}).get(kind) == record.get("version"),
                    managed_id=record.get("version"),
                )
            )
    return result


def _discover_jdks() -> list[tuple[Path, Path, str]]:
    candidates: list[tuple[Path, Path, str]] = []
    if os.name == "nt":
        registry_roots = (
            r"SOFTWARE\JavaSoft\JDK",
            r"SOFTWARE\JavaSoft\Java Development Kit",
            r"SOFTWARE\Eclipse Adoptium\JDK",
            r"SOFTWARE\AdoptOpenJDK\JDK",
        )
        for hive in (winreg.HKEY_CURRENT_USER, winreg.HKEY_LOCAL_MACHINE):
            for key_path in registry_roots:
                candidates.extend(_registry_java_homes(hive, key_path))
    java = shutil.which("java")
    if java:
        executable = Path(java)
        home = executable.parent.parent
        candidates.append((home, executable, "系统 PATH"))
    for base in (Path(os.environ.get("ProgramFiles", "C:/Program Files")) / "Java", Path("C:/Program Files/Eclipse Adoptium")):
        if base.exists():
            for home in base.iterdir():
                executable = home / "bin/java.exe"
                if executable.exists():
                    candidates.append((home, executable, "常用安装目录"))
    return candidates


def _registry_java_homes(hive, key_path: str) -> list[tuple[Path, Path, str]]:
    result = []
    for access in (winreg.KEY_READ | winreg.KEY_WOW64_64KEY, winreg.KEY_READ | winreg.KEY_WOW64_32KEY):
        try:
            with winreg.OpenKey(hive, key_path, 0, access) as root:
                for index in range(winreg.QueryInfoKey(root)[0]):
                    version = winreg.EnumKey(root, index)
                    try:
                        with winreg.OpenKey(root, version) as subkey:
                            home = Path(winreg.QueryValueEx(subkey, "JavaHome")[0])
                        executable = home / "bin/java.exe"
                        if executable.exists():
                            result.append((home, executable, "Windows 注册表"))
                    except OSError:
                        continue
        except OSError:
            continue
    return result


def _discover_pythons() -> list[tuple[Path, Path, str]]:
    candidates: list[tuple[Path, Path, str]] = []
    if os.name == "nt":
        for hive in (winreg.HKEY_CURRENT_USER, winreg.HKEY_LOCAL_MACHINE):
            for access in (winreg.KEY_READ | winreg.KEY_WOW64_64KEY, winreg.KEY_READ | winreg.KEY_WOW64_32KEY):
                try:
                    with winreg.OpenKey(hive, r"Software\Python\PythonCore", 0, access) as root:
                        for index in range(winreg.QueryInfoKey(root)[0]):
                            version = winreg.EnumKey(root, index)
                            try:
                                with winreg.OpenKey(root, rf"{version}\InstallPath") as subkey:
                                    home = Path(winreg.QueryValue(subkey, ""))
                                executable = home / "python.exe"
                                if executable.exists():
                                    candidates.append((home, executable, "Windows 注册表"))
                            except OSError:
                                continue
                except OSError:
                    continue
    launcher = shutil.which("py")
    if launcher:
        try:
            completed = subprocess.run(
                [launcher, "-0p"],
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                timeout=15,
                creationflags=subprocess.CREATE_NO_WINDOW,
            )
            for line in completed.stdout.splitlines():
                match = re.search(r"([A-Za-z]:\\.+?python\.exe)\s*$", line.strip())
                if match:
                    executable = Path(match.group(1))
                    if executable.exists():
                        candidates.append((executable.parent, executable, "Python Launcher"))
        except (OSError, subprocess.SubprocessError):
            pass
    return candidates


def _discover_nodes() -> list[tuple[Path, Path, str]]:
    candidates: list[tuple[Path, Path, str]] = []
    node = shutil.which("node")
    if node:
        executable = Path(node)
        candidates.append((executable.parent, executable, "系统 PATH"))
    for home in (
        Path(os.environ.get("ProgramFiles", "C:/Program Files")) / "nodejs",
        Path(os.environ.get("APPDATA", str(Path.home()))) / "nvm/current",
    ):
        executable = home / "node.exe"
        if executable.exists():
            candidates.append((home, executable, "常用安装目录"))
    return candidates


def _detail_version(kind: str, detail: str) -> str:
    if not detail:
        return ""
    patterns = {
        "jdk": r'(?:version\s+")?(\d+(?:\.\d+){0,3})',
        "python": r"Python\s+(\d+(?:\.\d+){1,2})",
        "node": r"v(\d+(?:\.\d+){1,2})",
    }
    match = re.search(patterns[kind], detail, re.IGNORECASE)
    return match.group(1) if match else ""


def _version_key(version: str) -> tuple[int, ...]:
    numbers = re.findall(r"\d+", version)
    return tuple(int(value) for value in numbers[:4]) or (0,)
