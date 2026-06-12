from __future__ import annotations

import os
import shutil
from dataclasses import dataclass
from pathlib import Path

from core.app_paths import AppPaths
from core.env_var import MANAGED_PATHS, get_user_environment
from core.process_runner import run_command


@dataclass
class DiagnosticItem:
    name: str
    status: str
    message: str


def run_diagnostics(paths: AppPaths) -> list[DiagnosticItem]:
    environment = get_user_environment()
    items: list[DiagnosticItem] = []
    devenv_home = environment.get("DEVENV_HOME", "")
    java_home = environment.get("JAVA_HOME", "")
    user_path = environment.get("Path", environment.get("PATH", ""))
    items.append(_check("DEVENV_HOME", Path(devenv_home).resolve() == paths.root.resolve() if devenv_home else False, devenv_home or "未配置"))
    expected_java = r"%DEVENV_HOME%\current\jdk"
    items.append(_check("JAVA_HOME", java_home.casefold() == expected_java.casefold(), java_home or "未配置"))
    normalized_path = {part.strip().rstrip("\\/").casefold() for part in user_path.split(";")}
    for managed in MANAGED_PATHS:
        items.append(_check(f"PATH: {managed}", managed.casefold() in normalized_path, "已配置" if managed.casefold() in normalized_path else "缺失"))
    commands = {
        "JDK java": (paths.current / "jdk/bin/java.exe", ["-version"], "java", ["-version"]),
        "JDK javac": (paths.current / "jdk/bin/javac.exe", ["-version"], "javac", ["-version"]),
        "Python": (paths.current / "python/python.exe", ["--version"], "python", ["--version"]),
        "Python pip": (paths.current / "python/python.exe", ["-m", "pip", "--version"], "python", ["-m", "pip", "--version"]),
        "Node.js": (paths.current / "node/node.exe", ["-v"], "node", ["-v"]),
        "npm": (paths.current / "node/npm.cmd", ["-v"], "npm.cmd", ["-v"]),
    }
    for name, (executable, args, system_command, system_args) in commands.items():
        if not executable.exists():
            items.append(DiagnosticItem(name, "WARNING", _describe_system_fallback(name, system_command, system_args)))
            continue
        result = run_command([str(executable), *args], timeout=20)
        items.append(DiagnosticItem(name, "OK" if result.success else "ERROR", result.output or "无输出"))
    return items


def _check(name: str, condition: bool, message: str) -> DiagnosticItem:
    return DiagnosticItem(name, "OK" if condition else "WARNING", message)


def _describe_system_fallback(name: str, command: str, args: list[str]) -> str:
    resolved = shutil.which(command)
    if resolved:
        result = run_command([resolved, *args], timeout=20)
        if result.success:
            detail = result.output.splitlines()[0] if result.output else "版本命令无输出"
            return f"DevEnv 未安装或未激活；检测到系统版本：{detail}（{resolved}）"
    if name.startswith("Python"):
        launcher = shutil.which("py")
        if launcher:
            launcher_args = ["-3", *args]
            result = run_command([launcher, *launcher_args], timeout=20)
            if result.success:
                detail = result.output.splitlines()[0] if result.output else "版本命令无输出"
                return f"DevEnv 未安装或未激活；检测到 Python Launcher：{detail}（{launcher}）"
    if resolved:
        return f"DevEnv 未安装或未激活；系统命令存在但不可执行（{resolved}）"
    return "DevEnv 未安装或未激活；系统 PATH 中也未检测到可用命令"


def format_report(items: list[DiagnosticItem]) -> str:
    return "\n".join(f"[{item.status}] {item.name}: {item.message}" for item in items)
