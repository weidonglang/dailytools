from __future__ import annotations

from dataclasses import dataclass

from core.config_store import ConfigService


@dataclass
class RuntimeState:
    kind: str
    version: str | None
    path: str
    healthy: bool


def collect_runtime_states(config: ConfigService) -> list[RuntimeState]:
    data = config.installed()
    mapping = (("jdk", "jdks"), ("python", "pythons"), ("node", "nodes"))
    states: list[RuntimeState] = []
    for kind, collection in mapping:
        version = data["current"].get(kind)
        record = next((item for item in data[collection] if item["version"] == version), None)
        path = record["path"] if record else ""
        states.append(RuntimeState(kind, version, path, bool(record and __import__("pathlib").Path(path).exists())))
    return states
