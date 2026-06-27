# 常见问题排查

## Python 安装后 pip 不可用

在“版本管理 → Python 环境分析”点击“完整性检查”。如果是受管 Python，可生成 pip 修复计划；如果是非受管 Python，请使用其官方安装器或包管理器修复。

## Python 缺少 venv

`venv` 是核心检查项。受管 Python 缺少 `venv` 时不会登记为完全可用。建议重新安装该受管版本，或使用完整 Python 安装包。

## Python ssl 不可用

`ssl` 不可用会影响 pip HTTPS 下载。受管 Python 安装后会检查 `ssl.OPENSSL_VERSION`，失败时显示组件缺失。

## pip 和 python -m pip 不一致

优先运行：

```powershell
python -m pip --version
```

如果 `pip --version` 指向另一个 `site-packages`，说明 `pip.exe` 属于其他 Python。

## Store Alias 抢占 python

如果 python 路径包含 `WindowsApps`，请打开 Windows“管理应用执行别名”手动关闭 Python Alias。DevEnv Manager 只提示，不自动关闭。

## JDK 安装后 Nacos 识别不到

在“项目”页选择 Nacos 根目录，点击“验证 Nacos Java”。常见原因包括 `JAVA_HOME` 是间接引用、当前进程环境未刷新、PATH 首个 Java 与 `JAVA_HOME` 不一致、缺少 `javac.exe`。

## Nexus 识别不到 JAVA_HOME

选择 Nexus 根目录，点击“验证 Nexus Java”。如果服务已经启动，修改用户环境后通常需要重启服务进程。

## Maven 使用错误 JDK

在“环境”查看 `JAVA_HOME` raw/expanded 和 PATH 首个 Java；在“运行时强验证”检查 Maven 与当前 JDK。

## Gradle 使用错误 JDK

Gradle Daemon 可能保留旧 JDK。切换 JDK 后请重启终端、IDE 或 Gradle Daemon，再重新验证。

## IDEA 项目 JDK 与命令行不一致

在“项目”页点击“只读读取 IDEA 配置”。如果 IDEA Project SDK 与当前 `JAVA_HOME` 不一致，请在 IDEA 中检查 Project SDK 或切换 DevEnv Manager 当前 JDK。

## 项目启动向导如何选择文件夹

点击“选择文件夹”，选择项目根目录。程序会检查路径是否存在、是否为目录，并识别常见项目文件。

## PATH 改了但终端不生效

已经打开的 CMD、PowerShell、Windows Terminal、IDE 和服务不会自动继承新用户环境。请重新打开相关程序。

## 端口被占用但不知道能不能结束

先查看端口详情、进程路径、父进程和服务名。系统关键进程会被拦截；普通进程仍建议确认用途后再结束。
