from core.env_var import MANAGED_PATHS, merge_path


def test_managed_paths_are_added_first_and_only_once():
    original = r"C:\Windows;C:\Tools;%DEVENV_HOME%\current\node;C:\Windows"
    merged = merge_path(original)
    parts = merged.split(";")
    assert tuple(parts[:4]) == MANAGED_PATHS
    assert parts.count(r"%DEVENV_HOME%\current\node") == 1
    assert parts.count(r"C:\Windows") == 1
    assert r"C:\Tools" in parts


def test_path_matching_is_case_insensitive():
    original = r"%devenv_home%\CURRENT\JDK\BIN;C:\Other"
    merged = merge_path(original)
    assert merged.casefold().count(r"%devenv_home%\current\jdk\bin") == 1
