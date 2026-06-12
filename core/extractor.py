from __future__ import annotations

import shutil
import tempfile
import zipfile
from pathlib import Path


class ExtractionError(RuntimeError):
    pass


def safe_extract_zip(archive: Path, destination: Path) -> Path:
    destination.mkdir(parents=True, exist_ok=True)
    destination_resolved = destination.resolve()
    try:
        with zipfile.ZipFile(archive) as bundle:
            for member in bundle.infolist():
                target = (destination / member.filename).resolve()
                if target != destination_resolved and destination_resolved not in target.parents:
                    raise ExtractionError(f"压缩包包含危险路径：{member.filename}")
            bundle.extractall(destination)
    except (zipfile.BadZipFile, OSError) as exc:
        raise ExtractionError(f"解压失败：{exc}") from exc
    return destination


def install_zip_payload(
    archive: Path,
    target: Path,
    required_files: tuple[str, ...],
) -> None:
    if target.exists():
        raise ExtractionError(f"目标版本已经存在：{target}")
    target.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="devenv-", dir=target.parent) as temp_name:
        temp = Path(temp_name)
        safe_extract_zip(archive, temp)
        candidates = [temp] + [item for item in temp.iterdir() if item.is_dir()]
        payload = next(
            (candidate for candidate in candidates if all((candidate / name).exists() for name in required_files)),
            None,
        )
        if payload is None:
            raise ExtractionError("无法识别压缩包中的运行时根目录")
        shutil.move(str(payload), str(target))
