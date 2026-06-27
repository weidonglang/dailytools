# Python 完整性检查与 pip 修复

DevEnv Manager 1.5.1 对受管 Python 增加安装后完整性检查，并保留现有 Python/pip 冲突分析。

## 检查项

核心检查：

- `python --version`
- `python -c "import sys; print(sys.executable)"`
- `python -m pip --version`
- `python -m venv --help`
- `python -c "import ssl; print(ssl.OPENSSL_VERSION)"`
- `python -c "import sqlite3; print(sqlite3.sqlite_version)"`
- `python -c "import ctypes"`
- `Scripts\pip.exe`

可选检查：

- `python -c "import tkinter"`

`tkinter` 失败不阻断安装，但会提示 Python GUI 相关库可能不可用。

## pip 缺失修复

当 `python -m pip --version` 失败时：

1. 判断当前 Python 是否位于 DevEnv Manager 受管目录。
2. 非受管 Python 只提示问题，不执行修复。
3. 受管 Python 可生成修复计划。
4. 修复计划展示将执行的命令：

```powershell
python -m ensurepip --upgrade
python -m pip install --upgrade pip
```

执行后重新验证 `python -m pip --version`。

## 安全边界

- 不下载 `get-pip.py`。
- 不删除系统 `pip.exe`。
- 不修复非受管 Python。
- 不自动关闭 Microsoft Store Alias。
- 不修改系统级 PATH。
- 不把 pip 安装到错误 Python。

## 常见风险

`pip.exe` 不一定属于当前 `python.exe`。如果 `pip --version` 与 `python -m pip --version` 显示的 `site-packages` 路径不一致，请优先使用 `python -m pip`。
