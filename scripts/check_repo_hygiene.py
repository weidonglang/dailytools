from __future__ import annotations

import subprocess
import sys


BLOCKED_PARTS = {
    ".idea",
    "__pycache__",
    "build",
    "dist",
    "target",
    "node_modules",
}

BLOCKED_SUFFIXES = {
    ".pyc",
    ".pyo",
    ".toc",
}


def tracked_files() -> list[str]:
    result = subprocess.run(
        ["git", "ls-files"],
        check=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
    )
    return [line.strip().replace("\\", "/") for line in result.stdout.splitlines() if line.strip()]


def is_blocked(path: str) -> bool:
    parts = set(path.split("/"))
    if parts & BLOCKED_PARTS:
        return True
    return any(path.lower().endswith(suffix) for suffix in BLOCKED_SUFFIXES)


def main() -> int:
    blocked = [path for path in tracked_files() if is_blocked(path)]
    if blocked:
        print("Repository hygiene check failed: generated/local files are tracked.")
        for path in blocked[:200]:
            print(f"- {path}")
        if len(blocked) > 200:
            print(f"... and {len(blocked) - 200} more")
        return 1
    print("Repository hygiene check passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
