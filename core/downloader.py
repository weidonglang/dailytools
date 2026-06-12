from __future__ import annotations

import hashlib
from pathlib import Path
from typing import Callable
from urllib.parse import urlparse

import requests


ALLOWED_DOWNLOAD_HOSTS = {
    "api.adoptium.net",
    "github.com",
    "objects.githubusercontent.com",
    "release-assets.githubusercontent.com",
    "nodejs.org",
    "www.python.org",
    "python.org",
}

ProgressCallback = Callable[[int, int], None]


class DownloadError(RuntimeError):
    pass


def validate_download_url(url: str) -> None:
    parsed = urlparse(url)
    if parsed.scheme != "https" or (parsed.hostname or "").lower() not in ALLOWED_DOWNLOAD_HOSTS:
        raise DownloadError(f"下载地址不在安全白名单中：{url}")


def download_file(
    url: str,
    target_path: Path,
    progress_callback: ProgressCallback | None = None,
    timeout: int = 60,
    expected_sha256: str | None = None,
) -> Path:
    validate_download_url(url)
    target_path.parent.mkdir(parents=True, exist_ok=True)
    temp_path = target_path.with_suffix(target_path.suffix + ".part")
    downloaded = 0
    digest = hashlib.sha256()
    try:
        with requests.get(
            url,
            stream=True,
            timeout=(15, timeout),
            allow_redirects=True,
            headers={"User-Agent": "DevEnvManager/1.0"},
        ) as response:
            response.raise_for_status()
            validate_download_url(response.url)
            total = int(response.headers.get("content-length", 0))
            with temp_path.open("wb") as handle:
                for chunk in response.iter_content(chunk_size=1024 * 1024):
                    if not chunk:
                        continue
                    handle.write(chunk)
                    digest.update(chunk)
                    downloaded += len(chunk)
                    if progress_callback:
                        progress_callback(downloaded, total)
        if downloaded == 0:
            raise DownloadError("服务器返回了空文件")
        if expected_sha256 and digest.hexdigest().lower() != expected_sha256.lower():
            raise DownloadError("SHA-256 校验失败，文件可能不完整")
        temp_path.replace(target_path)
        return target_path
    except (requests.RequestException, OSError, DownloadError) as exc:
        try:
            temp_path.unlink(missing_ok=True)
        except OSError:
            pass
        if isinstance(exc, DownloadError):
            raise
        raise DownloadError(f"下载失败：{exc}") from exc
