from __future__ import annotations

import subprocess
from dataclasses import dataclass
from pathlib import Path


@dataclass
class CommandResult:
    success: bool
    returncode: int
    output: str


def run_command(args: list[str], cwd: Path | None = None, timeout: int = 60) -> CommandResult:
    try:
        completed = subprocess.run(
            args,
            cwd=str(cwd) if cwd else None,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
            creationflags=subprocess.CREATE_NO_WINDOW if hasattr(subprocess, "CREATE_NO_WINDOW") else 0,
            shell=False,
        )
        output = "\n".join(part.strip() for part in (completed.stdout, completed.stderr) if part.strip())
        return CommandResult(completed.returncode == 0, completed.returncode, output)
    except (OSError, subprocess.SubprocessError) as exc:
        return CommandResult(False, -1, str(exc))
