from port.port_models import PortRecord
from port.port_scanner import filter_records


RECORDS = [
    PortRecord("TCP", "127.0.0.1", 8080, "", "LISTEN", 123, "java.exe"),
    PortRecord("TCP", "0.0.0.0", 443, "", "LISTEN", 4, "System"),
    PortRecord("UDP", "0.0.0.0", 5353, "", "UDP", 456, "service.exe"),
]


def test_filter_by_port_and_process():
    assert filter_records(RECORDS, "8080", hide_system=False) == [RECORDS[0]]
    assert filter_records(RECORDS, "java", hide_system=False) == [RECORDS[0]]


def test_hide_system_and_listening_only():
    result = filter_records(RECORDS, listening_only=True, hide_system=True)
    assert RECORDS[1] not in result
    assert RECORDS[0] in result
    assert RECORDS[2] in result
