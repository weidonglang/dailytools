from core.doctor import _describe_system_fallback


def test_system_fallback_message_is_explicit():
    message = _describe_system_fallback("JDK java", "java", ["-version"])
    assert message.startswith("DevEnv 未安装或未激活；")
