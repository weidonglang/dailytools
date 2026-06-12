from __future__ import annotations

from dataclasses import dataclass

import psutil


BLOCKED_PIDS = {0, 4}
BLOCKED_NAMES = {
    "system",
    "idle",
    "registry",
    "smss.exe",
    "csrss.exe",
    "wininit.exe",
    "winlogon.exe",
    "services.exe",
    "lsass.exe",
}
CAUTION_NAMES = {"svchost.exe"}


@dataclass
class KillResult:
    success: bool
    message: str
    needs_force: bool = False
    blocked: bool = False


def inspect_process(pid: int) -> tuple[str, bool]:
    if pid in BLOCKED_PIDS:
        return "System", True
    process = psutil.Process(pid)
    name = process.name()
    return name, name.casefold() in BLOCKED_NAMES


def kill_process(pid: int, force: bool = False, allow_caution: bool = False) -> KillResult:
    if pid in BLOCKED_PIDS:
        return KillResult(False, f"PID {pid} 是受保护的系统进程", blocked=True)
    try:
        process = psutil.Process(pid)
        name = process.name()
        lower_name = name.casefold()
        if lower_name in BLOCKED_NAMES:
            return KillResult(False, f"{name} 是受保护的关键系统进程", blocked=True)
        if lower_name in CAUTION_NAMES and not allow_caution:
            return KillResult(False, f"{name} 需要额外确认", blocked=True)
        if force:
            process.kill()
            process.wait(timeout=3)
            return KillResult(True, f"已强制结束 PID {pid} / {name}")
        process.terminate()
        try:
            process.wait(timeout=3)
            return KillResult(True, f"已结束 PID {pid} / {name}")
        except psutil.TimeoutExpired:
            return KillResult(False, f"PID {pid} 未在 3 秒内退出", needs_force=True)
    except psutil.NoSuchProcess:
        return KillResult(True, f"PID {pid} 已经退出")
    except psutil.AccessDenied:
        return KillResult(False, f"权限不足，无法结束 PID {pid}")
    except psutil.Error as exc:
        return KillResult(False, f"结束进程失败：{exc}")
