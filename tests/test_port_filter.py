from port.port_models import PortRecord
from port.port_scanner import filter_records, find_conflicting_keys, summarize_records


RECORDS = [
    PortRecord("TCP", "127.0.0.1", 8080, "", "LISTEN", 123, "java.exe"),
    PortRecord("TCP", "0.0.0.0", 443, "", "LISTEN", 4, "System"),
    PortRecord("UDP", "0.0.0.0", 5353, "", "UDP", 456, "service.exe"),
    PortRecord("TCP", "0.0.0.0", 8080, "", "LISTEN", 789, "node.exe"),
]


def test_filter_by_port_and_process():
    assert filter_records(RECORDS, "pid:123", hide_system=False) == [RECORDS[0]]
    assert filter_records(RECORDS, "java", hide_system=False) == [RECORDS[0]]


def test_hide_system_and_listening_only():
    result = filter_records(RECORDS, listening_only=True, hide_system=True)
    assert RECORDS[1] not in result
    assert RECORDS[0] in result
    assert RECORDS[2] in result


def test_smart_query_range_and_exclusion():
    result = filter_records(RECORDS, "port:8000-9000 -name:node", hide_system=False)
    assert result == [RECORDS[0]]


def test_conflict_detection_and_summary():
    assert ("TCP", 8080) in find_conflicting_keys(RECORDS)
    result = filter_records(RECORDS, conflict_only=True, hide_system=False)
    assert result == [RECORDS[0], RECORDS[3]]
    assert summarize_records(RECORDS)["conflicts"] == 1
