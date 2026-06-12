# Codex 开发计划文件：DevEnv Manager + 可观测控制面板 + 端口占用管理器

## 0. 项目定位

项目名称：**DevEnv Manager**

项目目标：开发一个面向 Windows 的桌面端开发环境管理工具，使用 Python 开发并打包为独立 exe。用户电脑即使没有安装 JDK、Python、Node.js，也可以通过本工具完成指定版本的下载、安装、切换、诊断和修复。

本项目不是简单下载器，而是一个综合型开发环境控制台，包含三大核心能力：

1. **开发环境版本管理**
   - 支持 JDK / Python / Node.js 多版本下载安装。
   - 支持自定义安装目录，不默认安装到 C 盘。
   - 支持多个版本并存，通过 current 指针快速切换当前版本。
   - 支持用户级环境变量配置与 PATH 去重修复。

2. **可观测控制面板**
   - 展示当前 JDK / Python / Node.js 激活版本。
   - 展示下载任务、安装任务、切换任务、诊断任务状态。
   - 展示最近操作日志、环境变量状态、版本健康状态。
   - 后续可扩展为本地开发服务运行状态面板。

3. **端口占用搜索控制面板**
   - 实现类似任务管理器的端口搜索、筛选、排序、刷新、复制和结束进程能力。
   - 支持按端口、PID、进程名、协议、状态搜索。
   - 支持常用端口和常用进程快捷筛选。
   - 支持安全保护，避免误杀 PID 0 / PID 4 / System / 关键系统进程。

---

## 1. 技术栈要求

### 1.1 开发语言

- Python 3.11+

### 1.2 GUI 框架

优先使用：

- `customtkinter`

可选：

- `tkinter`
- `ttkbootstrap`

第一版建议使用 `customtkinter + tkinter.ttk.Treeview`。

### 1.3 核心依赖

```txt
customtkinter
requests
psutil
pyinstaller
```

可选依赖：

```txt
packaging
```

### 1.4 打包方式

使用 PyInstaller 打包为 Windows exe：

```bash
pyinstaller -F -w --name DevEnvManager main.py
```

如项目存在资源文件，改用：

```bash
pyinstaller --noconfirm --windowed --name DevEnvManager main.py
```

---

## 2. 总体目录结构

请按如下结构创建项目：

```txt
DevEnvManager/
├─ main.py
├─ requirements.txt
├─ README.md
├─ build_exe.bat
├─ app/
│  ├─ __init__.py
│  ├─ ui_main.py
│  ├─ ui_env_manager.py
│  ├─ ui_observability.py
│  ├─ ui_port_manager.py
│  └─ ui_components.py
├─ core/
│  ├─ __init__.py
│  ├─ app_paths.py
│  ├─ config_store.py
│  ├─ downloader.py
│  ├─ extractor.py
│  ├─ env_var.py
│  ├─ process_runner.py
│  ├─ task_bus.py
│  ├─ event_log.py
│  └─ doctor.py
├─ managers/
│  ├─ __init__.py
│  ├─ jdk_manager.py
│  ├─ python_manager.py
│  └─ node_manager.py
├─ port/
│  ├─ __init__.py
│  ├─ port_scanner.py
│  ├─ process_control.py
│  └─ port_models.py
├─ observability/
│  ├─ __init__.py
│  ├─ metrics.py
│  ├─ health.py
│  └─ runtime_state.py
├─ resources/
│  └─ app.ico
└─ tests/
   └─ test_path_merge.py
```

---

## 3. 安装根目录设计

用户第一次启动时，允许选择安装根目录。默认建议为：

```txt
D:\DevEnvManager
```

如果 D 盘不存在，再回退到：

```txt
%USERPROFILE%\DevEnvManager
```

目录结构：

```txt
D:\DevEnvManager
├─ envs
│  ├─ jdks
│  │  ├─ temurin-8
│  │  ├─ temurin-17
│  │  └─ temurin-21
│  ├─ pythons
│  │  ├─ python-3.10
│  │  ├─ python-3.11
│  │  └─ python-3.12
│  └─ nodes
│     ├─ node-18
│     ├─ node-20
│     └─ node-22
├─ current
│  ├─ jdk
│  ├─ python
│  └─ node
├─ downloads
├─ config
│  ├─ settings.json
│  ├─ installed.json
│  └─ env_backup.json
└─ logs
   └─ app.log
```

要求：

- 所有 JDK / Python / Node.js 都必须安装到用户指定的根目录下。
- 不允许默认安装到 C 盘。
- 不允许安装器自动污染系统 PATH。
- 第一版只修改当前用户环境变量，不修改系统环境变量。

---

## 4. 环境变量设计

### 4.1 用户级环境变量

只写入：

```txt
HKEY_CURRENT_USER\Environment
```

不要默认写入系统级环境变量。

### 4.2 固定变量

```txt
DEVENV_HOME=D:\DevEnvManager
JAVA_HOME=%DEVENV_HOME%\current\jdk
```

### 4.3 PATH 插入项

将以下路径插入用户 PATH 的最前面，并做去重处理：

```txt
%DEVENV_HOME%\current\jdk\bin
%DEVENV_HOME%\current\python
%DEVENV_HOME%\current\python\Scripts
%DEVENV_HOME%\current\node
```

要求：

- 不能粗暴覆盖用户已有 PATH。
- 插入前先备份原 PATH 到 `config/env_backup.json`。
- 重复执行配置时不能无限追加相同路径。
- 修改环境变量后广播 `WM_SETTINGCHANGE`。
- UI 需要提示：已经打开的 CMD / PowerShell / IDEA 可能需要重启后才能生效。

---

## 5. 版本切换机制

### 5.1 原则

不要反复安装 / 卸载版本，而是多个版本并存。

切换版本时，只改变：

```txt
D:\DevEnvManager\current\jdk
D:\DevEnvManager\current\python
D:\DevEnvManager\current\node
```

使其指向具体版本目录。

### 5.2 Windows 指针实现

优先使用 junction：

```bat
mklink /J D:\DevEnvManager\current\jdk D:\DevEnvManager\envs\jdks\temurin-17
```

原因：

- Windows 普通用户使用 junction 通常比 symlink 更稳定。
- symlink 可能需要管理员权限或开发者模式。

### 5.3 删除 current 的安全要求

删除 current 指针时，必须确认它是 junction / link，不允许误删真实安装目录。

实现时可以先用 `rmdir current\jdk` 删除 junction。

禁止使用 `shutil.rmtree(current_jdk)` 直接递归删除，避免误删真实 JDK 目录。

---

## 6. JDK 管理模块

文件：`managers/jdk_manager.py`

### 6.1 功能

- 查询可安装 JDK 版本。
- 下载指定版本 JDK。
- 解压到指定目录。
- 切换当前 JDK。
- 诊断当前 JDK。

### 6.2 版本范围

第一版支持：

```txt
JDK 8
JDK 17
JDK 21
```

### 6.3 下载来源

优先使用 Eclipse Temurin / Adoptium。

第一版可以先内置稳定下载链接，后续再接 Adoptium API。

### 6.4 安装方式

使用 zip 包，不使用 msi。

流程：

```txt
下载 zip
校验文件存在和大小
解压到临时目录
识别内部 jdk 根目录
移动到 envs/jdks/temurin-版本
检查 bin/java.exe 和 bin/javac.exe
运行 java -version
写入 installed.json
```

### 6.5 验证命令

```bash
java -version
javac -version
```

---

## 7. Python 管理模块

文件：`managers/python_manager.py`

### 7.1 功能

- 下载 Python 官方 Windows 安装器。
- 静默安装到指定目录。
- 切换当前 Python。
- 检查 pip。
- 支持为项目创建 venv。

### 7.2 版本范围

第一版支持：

```txt
Python 3.10
Python 3.11
Python 3.12
```

### 7.3 安装方式

下载 Python 官方 amd64 安装器，然后静默安装。

安装命令格式：

```bat
python-3.11.x-amd64.exe /quiet InstallAllUsers=0 TargetDir="D:\DevEnvManager\envs\pythons\python-3.11" PrependPath=0 AppendPath=0 Include_launcher=0 Include_pip=1 Include_test=0
```

### 7.4 关键要求

- `InstallAllUsers=0`，避免管理员权限。
- `TargetDir` 必须是用户选择的根目录下。
- `PrependPath=0`，禁止安装器自动改 PATH。
- `AppendPath=0`，禁止安装器自动改 PATH。
- `Include_launcher=0`，第一版不安装全局 py launcher，避免和系统已有 Python 冲突。
- `Include_pip=1`，必须安装 pip。

### 7.5 验证命令

```bash
python --version
python -m pip --version
```

### 7.6 venv 功能

后续增强：

```bash
python -m venv .venv
```

UI 可提供：

- 选择项目目录。
- 选择 Python 版本。
- 创建 `.venv`。
- 生成 `activate.cmd`。

---

## 8. Node.js 管理模块

文件：`managers/node_manager.py`

### 8.1 功能

- 下载 Node.js Windows x64 zip。
- 解压到指定目录。
- 切换当前 Node.js。
- 检查 npm / npx。

### 8.2 版本范围

第一版支持：

```txt
Node.js 18
Node.js 20
Node.js 22
```

### 8.3 安装方式

使用 zip，不使用 msi。

流程：

```txt
下载 node-vxx-win-x64.zip
下载或内置 SHA-256 校验信息
解压到临时目录
移动到 envs/nodes/node-版本
检查 node.exe / npm.cmd / npx.cmd
运行 node -v
运行 npm -v
写入 installed.json
```

---

## 9. 可观测控制面板

文件：`app/ui_observability.py`

### 9.1 页面目标

做一个可被观察的控制面板，不只是普通配置页。它要让用户清楚看到：

- 当前激活了什么版本。
- 哪些工具已安装。
- 哪些工具可用 / 不可用。
- 下载 / 安装 / 切换任务是否成功。
- PATH 和环境变量是否正常。
- 最近发生了什么操作。

### 9.2 页面布局

建议使用三块：

```txt
顶部：环境健康卡片
中部：任务与指标面板
底部：事件日志
```

### 9.3 健康卡片

展示：

```txt
JDK      当前版本 / 状态 / 路径
Python   当前版本 / 状态 / 路径
Node.js  当前版本 / 状态 / 路径
PATH     是否已配置 / 是否重复 / 是否优先生效
```

状态类型：

```txt
正常
未安装
未激活
路径异常
版本不匹配
命令不可用
```

### 9.4 指标面板

展示：

```txt
已安装 JDK 数量
已安装 Python 数量
已安装 Node.js 数量
最近下载任务数量
最近安装成功数量
最近安装失败数量
最近切换次数
端口占用数量
监听端口数量
```

### 9.5 事件日志

事件包括：

```txt
开始下载 JDK 17
下载完成 JDK 17
安装成功 Python 3.11
切换当前 Node.js 到 22
PATH 已修复
端口扫描完成，共发现 36 条记录
结束进程 PID 12345 成功
结束进程 PID 4 被拦截
```

要求：

- 日志在 UI 内可见。
- 同时写入 `logs/app.log`。
- 日志最多保留最近 1000 条。

---

## 10. 端口占用搜索控制面板

参考用户提供截图，实现一个类似端口管理器的页面。

文件：`app/ui_port_manager.py`

核心逻辑文件：

```txt
port/port_scanner.py
port/process_control.py
port/port_models.py
```

### 10.1 页面标题

```txt
端口占用搜索控制面板
```

副标题：

```txt
扫描本机端口占用，像任务管理器一样搜索、筛选、排序并结束指定进程。
```

右上角展示：

```txt
当前权限：普通用户，部分进程可能无法结束
```

如检测到管理员权限：

```txt
当前权限：管理员
```

### 10.2 搜索与操作区

包含：

```txt
搜索输入框
扫描 / 刷新按钮，快捷键 F5
结束选中按钮，快捷键 Del
清空搜索按钮，快捷键 Esc
复制选中信息按钮
只看监听端口复选框
隐藏 PID 0/4 复选框
10 秒自动刷新复选框
```

搜索支持：

```txt
端口
PID
进程名
协议
状态
本地地址
远程地址
```

### 10.3 常用端口快捷按钮

至少提供：

```txt
Spring 8080
Vite 5173
React 3000
Vue 5173
MySQL 3306
Redis 6379
PostgreSQL 5432
MongoDB 27017
Nginx 80
HTTPS 443
```

点击按钮后，搜索框填入对应端口并立即筛选。

注意截图中 PostgreSQL 和 MongoDB 按钮文字可能因为宽度截断，实际实现中应完整显示。

### 10.4 常用进程快捷按钮

至少提供：

```txt
java
node
python
mysqld
redis
nginx
idea
```

点击按钮后，搜索框填入对应进程名并立即筛选。

### 10.5 结果表格

使用 `ttk.Treeview`。

列：

```txt
协议
本地地址
端口
远程地址
状态
PID
进程名
```

可选增强列：

```txt
进程路径
启动时间
用户名
```

表格要求：

- 支持列宽拖动。
- 支持点击列头排序。
- 支持单选 / 多选。
- 双击行显示详情。
- 选中行后，下方详情面板更新。

### 10.6 详情面板

标题：

```txt
选中项详情
```

未选择时：

```txt
未选择进程。
```

选中后展示：

```txt
协议：TCP
本地地址：127.0.0.1
端口：8080
远程地址：-
状态：LISTEN
PID：12345
进程名：java.exe
进程路径：...
命令行：...
用户名：...
```

### 10.7 底部提示

状态提示：

```txt
点击“扫描”开始。可按端口、PID、进程名、协议或状态搜索。
```

安全提示：

```txt
安全提示：不要直接结束 PID 4/System。80、443 被 System 占用时，优先检查 IIS、HTTP.sys、Hyper-V、Docker、WSL 或系统服务。
```

### 10.8 扫描实现

使用 `psutil.net_connections(kind="inet")`。

每条记录字段：

```python
protocol: TCP / UDP
local_address: str
local_port: int
remote_address: str
status: str
pid: int | None
process_name: str
process_path: str | None
cmdline: str | None
username: str | None
```

处理细节：

- TCP / UDP 都要支持。
- UDP 没有 LISTEN 状态时要合理展示。
- `pid is None` 时显示 `-`。
- 访问某些进程信息可能 AccessDenied，要捕获异常并显示 `权限不足`。
- 扫描不能卡死 UI，必须放入后台线程。

### 10.9 结束进程实现

文件：`port/process_control.py`

功能：

```python
kill_process(pid: int) -> KillResult
```

安全规则：

必须拦截：

```txt
PID 0
PID 4
System
Idle
Registry
smss.exe
csrss.exe
wininit.exe
winlogon.exe
services.exe
lsass.exe
svchost.exe 默认不直接杀，必须二次确认
```

结束普通开发进程时，如：

```txt
java.exe
node.exe
python.exe
mysqld.exe
redis-server.exe
nginx.exe
```

允许结束，但必须弹出确认框：

```txt
确定要结束 PID xxx / 进程名 xxx 吗？这可能导致对应服务停止。
```

实现方式：

```python
psutil.Process(pid).terminate()
等待 3 秒
如未退出，再询问是否 kill()
```

不要默认强杀。

### 10.10 自动刷新

支持 10 秒自动刷新：

- 勾选后每 10 秒重新扫描。
- 如果正在扫描，不要并发启动第二次扫描。
- 用户正在选择行时，刷新后尽量保持选中 PID / 端口。

---

## 11. 首页 UI 设计

文件：`app/ui_main.py`

主窗口建议：

```txt
左侧导航栏
├─ 首页
├─ 环境管理
├─ 可观测面板
├─ 端口管理器
└─ 设置

右侧内容区
```

窗口大小建议：

```txt
1400 x 850
```

支持最小大小：

```txt
1100 x 700
```

首页展示：

```txt
当前 JDK：版本 / 路径 / 状态
当前 Python：版本 / 路径 / 状态
当前 Node.js：版本 / 路径 / 状态
安装根目录
一键诊断按钮
一键修复 PATH 按钮
打开安装目录按钮
打开日志按钮
```

---

## 12. 配置文件设计

### 12.1 settings.json

```json
{
  "root_dir": "D:/DevEnvManager",
  "auto_check_update": false,
  "download_timeout_seconds": 60,
  "theme": "system",
  "last_page": "home"
}
```

### 12.2 installed.json

```json
{
  "jdks": [
    {
      "version": "17",
      "distribution": "temurin",
      "path": "D:/DevEnvManager/envs/jdks/temurin-17",
      "java_exe": "D:/DevEnvManager/envs/jdks/temurin-17/bin/java.exe",
      "installed_at": "2026-06-11T10:00:00"
    }
  ],
  "pythons": [
    {
      "version": "3.11",
      "path": "D:/DevEnvManager/envs/pythons/python-3.11",
      "python_exe": "D:/DevEnvManager/envs/pythons/python-3.11/python.exe",
      "installed_at": "2026-06-11T10:00:00"
    }
  ],
  "nodes": [
    {
      "version": "22",
      "path": "D:/DevEnvManager/envs/nodes/node-22",
      "node_exe": "D:/DevEnvManager/envs/nodes/node-22/node.exe",
      "installed_at": "2026-06-11T10:00:00"
    }
  ],
  "current": {
    "jdk": "17",
    "python": "3.11",
    "node": "22"
  }
}
```

---

## 13. 任务系统设计

文件：`core/task_bus.py`

下载、安装、扫描端口、诊断都不能阻塞 UI。

实现一个简单后台任务系统：

```python
class BackgroundTask:
    name: str
    status: str  # pending/running/success/failed
    progress: int
    message: str
    started_at: datetime
    finished_at: datetime | None
```

UI 可以订阅任务状态变化。

第一版可以用：

```python
threading.Thread
queue.Queue
```

不要引入过重框架。

---

## 14. 环境诊断功能

文件：`core/doctor.py`

### 14.1 检查项

```txt
DEVENV_HOME 是否存在
JAVA_HOME 是否正确
PATH 是否包含 current/jdk/bin
PATH 是否包含 current/python
PATH 是否包含 current/python/Scripts
PATH 是否包含 current/node
current/jdk 是否存在
current/python 是否存在
current/node 是否存在
java -version 是否可执行
javac -version 是否可执行
python --version 是否可执行
python -m pip --version 是否可执行
node -v 是否可执行
npm -v 是否可执行
```

### 14.2 输出结果

```txt
OK
WARNING
ERROR
```

UI 展示每项诊断结果，并提供：

```txt
一键修复 PATH
打开安装目录
复制诊断报告
```

---

## 15. 下载实现要求

文件：`core/downloader.py`

功能：

- requests 流式下载。
- 支持进度回调。
- 下载到 `.part` 临时文件。
- 成功后改名为正式文件。
- 下载失败保留错误信息。

伪代码：

```python
def download_file(url, target_path, progress_callback=None):
    tmp_path = target_path.with_suffix(target_path.suffix + ".part")
    with requests.get(url, stream=True, timeout=60) as r:
        r.raise_for_status()
        total = int(r.headers.get("content-length", 0))
        with open(tmp_path, "wb") as f:
            for chunk in r.iter_content(chunk_size=1024 * 1024):
                if chunk:
                    f.write(chunk)
                    progress_callback(downloaded, total)
    tmp_path.replace(target_path)
```

---

## 16. 安全要求

必须实现以下安全保护：

1. 下载来源只允许白名单域名。
2. 安装目录必须位于用户选择的 `root_dir` 下。
3. 解压 zip 时防止路径穿越，即禁止 zip 内文件写到目标目录之外。
4. 修改 PATH 前必须备份旧值。
5. 删除 current 指针时禁止误删真实目录。
6. 结束进程前必须确认。
7. 禁止结束 PID 0 / PID 4 / System 关键进程。
8. 遇到权限不足要提示，不要崩溃。

---

## 17. 第一阶段 MVP 目标

第一阶段只要求完成可运行版本，不追求所有细节完美。

必须实现：

```txt
1. 桌面 GUI 主窗口
2. 自定义安装根目录
3. JDK 17 / 21 安装与切换
4. Python 3.10 / 3.11 安装与切换
5. Node 20 / 22 安装与切换
6. 用户级环境变量配置
7. 一键诊断
8. 可观测面板基础指标
9. 端口扫描表格
10. 按端口 / PID / 进程名搜索
11. 常用端口快捷按钮
12. 常用进程快捷按钮
13. 结束选中进程，带安全确认
14. 复制选中信息
15. 打包成 exe
```

暂不强制实现：

```txt
1. 所有远程版本自动发现
2. 下载断点续传
3. SHA-256 完整校验
4. 项目级 .devenv.json
5. 自动生成 IDEA / VS Code 配置
6. 多语言国际化
```

---

## 18. 第二阶段增强目标

```txt
1. 接入 Adoptium API 查询 JDK 版本
2. 接入 Node 官方版本索引
3. 下载 SHA-256 校验
4. 项目级环境配置 .devenv.json
5. 为 Python 项目创建 .venv
6. 生成 activate.cmd / activate.ps1
7. PATH 一键恢复
8. 端口表格导出 CSV
9. 端口占用变化趋势
10. 更完整的日志面板
```

---

## 19. UI 细节要求：端口管理器参考图

端口管理器页面要尽量接近用户提供截图的结构：

```txt
标题区：端口占用搜索控制面板
说明区：扫描本机端口占用，像任务管理器一样搜索、筛选、排序并结束指定进程。
右上角：当前权限

搜索与操作区：
搜索框 + 扫描/刷新 + 结束选中 + 清空搜索 + 复制选中信息 + 复选框
常用端口按钮行
常用进程按钮行

中间：端口结果表格

下方：选中项详情

底部：普通提示 + 安全提示
```

表格列必须包含：

```txt
协议 / 本地地址 / 端口 / 远程地址 / 状态 / PID / 进程名
```

按钮文案必须包含：

```txt
扫描 / 刷新  F5
结束选中  Del
清空搜索  Esc
复制选中信息
只看监听端口
隐藏 PID 0/4
10 秒自动刷新
```

---

## 20. 验收标准

### 20.1 环境管理验收

在一台没有配置 JDK / Python / Node.js PATH 的 Windows 电脑上：

1. 双击 exe 能打开软件。
2. 能选择安装根目录为 D 盘路径。
3. 能安装 JDK 17。
4. 能安装 Python 3.11。
5. 能安装 Node 22。
6. 能一键配置用户环境变量。
7. 新开 PowerShell 后执行：

```bash
java -version
python --version
node -v
npm -v
```

均能输出当前版本。

8. 切换 JDK 21 后，新开终端 `java -version` 变为 JDK 21。
9. 切换 Python 3.10 后，新开终端 `python --version` 变为 Python 3.10。
10. 切换 Node 20 后，新开终端 `node -v` 变为 Node 20。

### 20.2 可观测面板验收

1. 能展示当前版本。
2. 能展示安装数量。
3. 能展示诊断状态。
4. 能展示最近操作日志。
5. 下载、安装、切换、端口扫描均有日志记录。

### 20.3 端口管理器验收

1. 点击扫描后能列出当前 TCP / UDP 占用。
2. 搜索 `8080` 能筛选 8080 端口。
3. 搜索 `java` 能筛选 Java 进程。
4. 点击 `Vite 5173` 能筛选 5173。
5. 点击 `node` 能筛选 node 进程。
6. 勾选只看监听端口后，只展示 LISTEN 状态。
7. 勾选隐藏 PID 0/4 后，不展示 PID 0 / 4。
8. 选中普通开发进程后，点击结束选中会弹出确认框。
9. PID 4 / System 被拦截，不允许结束。
10. 复制选中信息能复制到剪贴板。
11. F5 刷新、Del 结束、Esc 清空搜索可用。

---

## 21. 开发顺序建议

请按以下顺序实现，不要一开始就写复杂 UI：

### 第 1 步：基础框架

- 创建项目结构。
- 创建主窗口。
- 创建左侧导航和三个页面空壳。
- 实现 settings.json 读写。

### 第 2 步：端口管理器

- 先实现 `psutil.net_connections` 扫描。
- 再实现表格展示。
- 再实现搜索筛选。
- 再实现结束进程。
- 再实现自动刷新和快捷键。

优先做端口管理器，因为它不依赖下载安装网络环境，最容易本地验证。

### 第 3 步：环境变量与诊断

- 实现 DEVENV_HOME / JAVA_HOME / PATH 配置。
- 实现 doctor。
- 实现可观测面板基础卡片。

### 第 4 步：JDK 与 Node

- 先用内置下载链接。
- 支持 zip 下载和解压。
- 支持 current junction 切换。

### 第 5 步：Python

- 实现官方安装器下载。
- 实现静默安装。
- 实现 current junction 切换。

### 第 6 步：打包与验收

- 编写 `build_exe.bat`。
- 使用 PyInstaller 打包。
- 在干净 Windows 环境测试。

---

## 22. build_exe.bat

请生成：

```bat
@echo off
chcp 65001 >nul
setlocal

if not exist .venv (
    python -m venv .venv
)

call .venv\Scripts\activate
python -m pip install --upgrade pip
pip install -r requirements.txt

pyinstaller --noconfirm --windowed --name DevEnvManager main.py

echo.
echo Build finished. Check dist\DevEnvManager\ or dist\DevEnvManager.exe
pause
```

---

## 23. README 内容要求

README 至少包含：

```txt
项目简介
功能列表
运行环境
开发运行方式
打包方式
使用说明
安全说明
常见问题
```

常见问题必须包含：

```txt
为什么修改环境变量后当前终端没有变化？
为什么 PID 4 不能结束？
为什么 80 / 443 被 System 占用？
为什么 Python 安装后没有 py 命令？
为什么不默认安装到 C 盘？
```

---

## 24. 代码风格要求

- 所有文件使用 UTF-8。
- 尽量使用 `pathlib.Path`。
- 子进程调用使用 list 参数，不要拼接 shell 字符串。
- 所有外部调用必须捕获异常并返回可显示的错误消息。
- UI 线程不要做耗时任务。
- 关键操作写日志。
- 删除文件和结束进程必须谨慎。
- 所有用户可见文案使用中文。

---

## 25. 最终交付物

```txt
1. 完整 Python 源码
2. requirements.txt
3. README.md
4. build_exe.bat
5. 可运行 DevEnvManager.exe
6. 示例截图
7. 简单测试说明
```

---

## 26. 项目简历描述

可写为：

**DevEnv Manager：Windows 多版本开发环境管理与端口占用控制工具**

基于 Python 和 CustomTkinter 开发 Windows 桌面端开发环境管理器，并使用 PyInstaller 打包为独立 exe；支持在无 JDK、无 Python、无 Node.js 的环境中自动下载并安装指定版本运行时，提供自定义安装目录、多版本并存、current 指针快速切换、用户级环境变量配置、PATH 去重修复和环境诊断能力。同时实现可观测控制面板与端口占用搜索控制面板，支持 TCP/UDP 端口扫描、按端口/PID/进程名筛选、常用端口快捷查询、进程详情展示、安全结束进程和操作日志记录，用于提升 Java 后端、Python AI 服务和 Vue 前端项目的本地开发环境配置效率。
