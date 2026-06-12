from __future__ import annotations

import re
from pathlib import Path

from packaging.version import Version

from core.downloader import download_file
from core.extractor import install_zip_payload
from core.http_client import get_json, get_text
from managers.base_manager import BaseRuntimeManager, Progress


class NodeManager(BaseRuntimeManager):
    kind = "node"
    collection = "nodes"
    executable = "node"
    supported_versions = ("20", "22")

    def resolve_release(self, version: str) -> dict[str, str]:
        if version not in self.supported_versions:
            raise ValueError(f"暂不支持 Node.js {version}")
        matching = [
            item for item in get_json("https://nodejs.org/dist/index.json")
            if item["version"].lstrip("v").split(".", 1)[0] == version and "win-x64-zip" in item.get("files", [])
        ]
        if not matching:
            raise RuntimeError(f"未找到 Node.js {version} 的 Windows x64 版本")
        latest = max(matching, key=lambda item: Version(item["version"].lstrip("v")))
        tag = latest["version"]
        filename = f"node-{tag}-win-x64.zip"
        return {"url": f"https://nodejs.org/dist/{tag}/{filename}", "name": filename, "tag": tag}

    def _resolve_checksum(self, release: dict[str, str]) -> str | None:
        text = get_text(f"https://nodejs.org/dist/{release['tag']}/SHASUMS256.txt")
        pattern = rf"^([a-fA-F0-9]{{64}})\s+{re.escape(release['name'])}$"
        match = re.search(pattern, text, re.MULTILINE)
        return match.group(1) if match else None

    def install(self, version: str, progress: Progress) -> Path:
        self.event_log.write(f"开始安装 Node.js {version}")
        progress(2, "正在查询 Node.js 官方版本")
        release = self.resolve_release(version)
        checksum = self._resolve_checksum(release)
        archive = self.config.paths.downloads / release["name"]
        target = self.config.paths.nodes / f"node-{version}"
        self.config.paths.assert_inside_root(target)
        download_file(
            release["url"],
            archive,
            lambda done, total: progress(8 + int(done * 62 / total) if total else 35, "正在下载 Node.js"),
            int(self.config.settings["download_timeout_seconds"]),
            checksum,
        )
        progress(72, "正在解压 Node.js")
        install_zip_payload(archive, target, ("node.exe", "npm.cmd", "npx.cmd"))
        progress(90, "正在验证 Node.js")
        output = self.verify(target / "node.exe", ["-v"])
        self.verify(target / "npm.cmd", ["-v"])
        self.record_install(
            version,
            target,
            target / "node.exe",
            {"detail": output.splitlines()[0] if output else release["tag"]},
        )
        self.switch(version)
        self.event_log.write(f"安装成功 Node.js {version}")
        return target
