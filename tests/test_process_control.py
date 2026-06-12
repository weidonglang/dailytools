from port.process_control import kill_process


def test_pid_zero_and_four_are_blocked():
    for pid in (0, 4):
        result = kill_process(pid)
        assert result.blocked
        assert not result.success
