from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class PortRecord:
    protocol: str
    local_address: str
    local_port: int
    remote_address: str
    status: str
    pid: int | None
    process_name: str
    process_path: str | None = None
    cmdline: str | None = None
    username: str | None = None

    def searchable_text(self) -> str:
        return " ".join(
            str(value)
            for value in (
                self.protocol,
                self.local_address,
                self.local_port,
                self.remote_address,
                self.status,
                self.pid or "",
                self.process_name,
                self.process_path or "",
                self.username or "",
            )
        ).casefold()

    def display_values(self) -> tuple[str, ...]:
        return (
            self.protocol,
            self.local_address,
            str(self.local_port),
            self.remote_address,
            self.status,
            str(self.pid) if self.pid is not None else "-",
            self.process_name,
        )

    def details(self) -> str:
        return "\n".join(
            (
                f"协议：{self.protocol}",
                f"本地地址：{self.local_address}",
                f"端口：{self.local_port}",
                f"远程地址：{self.remote_address or '-'}",
                f"状态：{self.status}",
                f"PID：{self.pid if self.pid is not None else '-'}",
                f"进程名：{self.process_name}",
                f"进程路径：{self.process_path or '-'}",
                f"命令行：{self.cmdline or '-'}",
                f"用户名：{self.username or '-'}",
            )
        )
