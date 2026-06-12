from __future__ import annotations

import socket

import psutil

from port.port_models import PortRecord


def _format_address(address: tuple | object) -> tuple[str, int]:
    if not address:
        return "", 0
    try:
        host, port = address[0], address[1]
        return str(host), int(port)
    except (IndexError, TypeError, ValueError):
        return str(address), 0


def scan_ports() -> list[PortRecord]:
    records: list[PortRecord] = []
    process_cache: dict[int, tuple[str, str | None, str | None, str | None]] = {}
    for connection in psutil.net_connections(kind="inet"):
        protocol = "TCP" if connection.type == socket.SOCK_STREAM else "UDP"
        local_host, local_port = _format_address(connection.laddr)
        remote_host, remote_port = _format_address(connection.raddr)
        remote = f"{remote_host}:{remote_port}" if remote_host else ""
        pid = connection.pid
        name = "-"
        path = cmdline = username = None
        if pid is not None:
            if pid not in process_cache:
                try:
                    process = psutil.Process(pid)
                    try:
                        name = process.name()
                    except (psutil.AccessDenied, psutil.NoSuchProcess):
                        name = "权限不足"
                    try:
                        path = process.exe()
                    except (psutil.AccessDenied, psutil.NoSuchProcess):
                        path = None
                    try:
                        cmdline = " ".join(process.cmdline())
                    except (psutil.AccessDenied, psutil.NoSuchProcess):
                        cmdline = None
                    try:
                        username = process.username()
                    except (psutil.AccessDenied, psutil.NoSuchProcess):
                        username = None
                    process_cache[pid] = (name, path, cmdline, username)
                except (psutil.AccessDenied, psutil.NoSuchProcess):
                    process_cache[pid] = ("权限不足", None, None, None)
            name, path, cmdline, username = process_cache[pid]
        status = connection.status if protocol == "TCP" else "UDP"
        records.append(
            PortRecord(protocol, local_host, local_port, remote, status, pid, name, path, cmdline, username)
        )
    return sorted(records, key=lambda item: (item.local_port, item.protocol, item.pid or -1))


def filter_records(
    records: list[PortRecord],
    query: str = "",
    listening_only: bool = False,
    hide_system: bool = True,
) -> list[PortRecord]:
    needle = query.strip().casefold()
    result = []
    for record in records:
        if listening_only and record.status not in {"LISTEN", "UDP"}:
            continue
        if hide_system and record.pid in {0, 4}:
            continue
        if needle and needle not in record.searchable_text():
            continue
        result.append(record)
    return result
