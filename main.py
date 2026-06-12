from __future__ import annotations

import ctypes
import sys
from pathlib import Path


def _configure_runtime() -> None:
    if sys.platform == "win32":
        try:
            ctypes.windll.shcore.SetProcessDpiAwareness(1)
        except (AttributeError, OSError):
            pass
    project_dir = Path(__file__).resolve().parent
    if str(project_dir) not in sys.path:
        sys.path.insert(0, str(project_dir))


def main() -> None:
    _configure_runtime()
    from app.ui_main import DevEnvManagerApp

    app = DevEnvManagerApp()
    app.mainloop()


if __name__ == "__main__":
    main()
