from __future__ import annotations

from pathlib import Path

from core.downloader import download_file
from core.extractor import install_zip_payload
from core.http_client import get_json
from managers.base_manager import BaseRuntimeManager, Progress


class JdkManager(BaseRuntimeManager):
    kind = "jdk"
    collection = "jdks"
    executable = "java"
    supported_versions = ("17", "21")

    def resolve_release(self, version: str) -> dict[str, str]:
        if version not in self.supported_versions:
            raise ValueError(f"暂不支持 JDK {version}")
        url = (
            "https://api.adoptium.net/v3/assets/latest/"
            f"{version}/hotspot?architecture=x64&image_type=jdk&os=windows&vendor=eclipse"
        )
        assets = get_json(url)
        if not assets:
            raise RuntimeError(f"未找到 JDK {version} 的 Windows x64 版本")
        package = assets[0]["binary"]["package"]
        return {
            "url": package["link"],
            "name": package["name"],
            "sha256": package.get("checksum", ""),
        }

    def install(self, version: str, progress: Progress) -> Path:
        self.event_log.write(f"开始安装 JDK {version}")
        progress(2, "正在查询 Adoptium")
        release = self.resolve_release(version)
        archive = self.config.paths.downloads / release["name"]
        target = self.config.paths.jdks / f"temurin-{version}"
        self.config.paths.assert_inside_root(target)
        progress(8, "正在下载 JDK")
        download_file(
            release["url"],
            archive,
            lambda done, total: progress(8 + int(done * 62 / total) if total else 35, "正在下载 JDK"),
            int(self.config.settings["download_timeout_seconds"]),
            release["sha256"] or None,
        )
        progress(72, "正在解压 JDK")
        install_zip_payload(archive, target, ("bin/java.exe", "bin/javac.exe"))
        progress(90, "正在验证 JDK")
        output = self.verify(target / "bin/java.exe", ["-version"])
        self.verify(target / "bin/javac.exe", ["-version"])
        self.record_install(
            version,
            target,
            target / "bin/java.exe",
            {"distribution": "temurin", "detail": output.splitlines()[0] if output else ""},
        )
        self.switch(version)
        self.event_log.write(f"安装成功 JDK {version}")
        return target
