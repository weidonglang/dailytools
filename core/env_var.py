from __future__ import annotations

import ctypes
import json
import os
import subprocess
from datetime import datetime
from pathlib import Path

if os.name == "nt":
    import winreg

from core.app_paths import AppPaths


MANAGED_PATHS = (
    r"%DEVENV_HOME%\current\jdk\bin",
    r"%DEVENV_HOME%\current\python",
    r"%DEVENV_HOME%\current\python\Scripts",
    r"%DEVENV_HOME%\current\node",
)


def merge_path(existing: str, additions: tuple[str, ...] = MANAGED_PATHS) -> str:
    def key(value: str) -> str:
        return value.strip().strip('"').rstrip("\\/").casefold()

    managed_keys = {key(item) for item in additions}
    retained: list[str] = []
    seen: set[str] = set()
    for item in existing.split(";"):
        item = item.strip()
        item_key = key(item)
        if not item or item_key in managed_keys or item_key in seen:
            continue
        seen.add(item_key)
        retained.append(item)
    return ";".join([*additions, *retained])


def get_user_environment() -> dict[str, str]:
    if os.name != "nt":
        return dict(os.environ)
    result: dict[str, str] = {}
    with winreg.CreateKey(winreg.HKEY_CURRENT_USER, "Environment") as key:
        index = 0
        while True:
            try:
                name, value, _ = winreg.EnumValue(key, index)
                result[name] = str(value)
                index += 1
            except OSError:
                break
    return result


def configure_user_environment(paths: AppPaths) -> None:
    if os.name != "nt":
        raise RuntimeError("环境变量配置仅支持 Windows")
    paths.ensure()
    environment = get_user_environment()
    backup = {
        "created_at": datetime.now().isoformat(timespec="seconds"),
        "DEVENV_HOME": environment.get("DEVENV_HOME"),
        "JAVA_HOME": environment.get("JAVA_HOME"),
        "Path": environment.get("Path", environment.get("PATH", "")),
    }
    paths.env_backup_file.write_text(json.dumps(backup, ensure_ascii=False, indent=2), encoding="utf-8")
    old_path = backup["Path"] or ""
    with winreg.CreateKey(winreg.HKEY_CURRENT_USER, "Environment") as key:
        winreg.SetValueEx(key, "DEVENV_HOME", 0, winreg.REG_SZ, str(paths.root))
        winreg.SetValueEx(key, "JAVA_HOME", 0, winreg.REG_EXPAND_SZ, r"%DEVENV_HOME%\current\jdk")
        winreg.SetValueEx(key, "Path", 0, winreg.REG_EXPAND_SZ, merge_path(old_path))
    broadcast_environment_change()


def broadcast_environment_change() -> None:
    if os.name != "nt":
        return
    HWND_BROADCAST = 0xFFFF
    WM_SETTINGCHANGE = 0x001A
    SMTO_ABORTIFHUNG = 0x0002
    result = ctypes.c_ulong()
    ctypes.windll.user32.SendMessageTimeoutW(
        HWND_BROADCAST,
        WM_SETTINGCHANGE,
        0,
        "Environment",
        SMTO_ABORTIFHUNG,
        5000,
        ctypes.byref(result),
    )


def is_junction(path: Path) -> bool:
    if not path.exists():
        return False
    try:
        output = subprocess.run(
            ["cmd", "/c", "fsutil", "reparsepoint", "query", str(path)],
            capture_output=True,
            text=True,
            timeout=10,
            creationflags=subprocess.CREATE_NO_WINDOW,
        )
        return output.returncode == 0
    except (OSError, subprocess.SubprocessError):
        return False


def switch_junction(link: Path, target: Path, root: Path) -> None:
    root_resolved = root.resolve()
    target_resolved = target.resolve()
    if root_resolved not in target_resolved.parents:
        raise ValueError("版本目录不在安装根目录内")
    link.parent.mkdir(parents=True, exist_ok=True)
    if link.exists():
        if not is_junction(link):
            raise RuntimeError(f"拒绝删除非链接目录：{link}")
        result = subprocess.run(
            ["cmd", "/c", "rmdir", str(link)],
            capture_output=True,
            text=True,
            creationflags=subprocess.CREATE_NO_WINDOW,
        )
        if result.returncode != 0:
            raise RuntimeError(result.stderr.strip() or "删除旧版本指针失败")
    result = subprocess.run(
        ["cmd", "/c", "mklink", "/J", str(link), str(target)],
        capture_output=True,
        text=True,
        encoding="gbk",
        errors="replace",
        creationflags=subprocess.CREATE_NO_WINDOW,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or result.stdout.strip() or "创建版本指针失败")
