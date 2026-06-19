# DevEnv Manager

DevEnv Manager 是面向 Windows 的多版本开发环境管理与端口占用控制工具。它可以从官方源自动下载、安装和切换 JDK、Python、Node.js，并提供环境诊断、端口进程管理、PATH 修复、系统运行时卸载入口和安装后健康检查。

当前仓库包含两个版本：

- `tauri/`：最新 Tauri + Rust 重构版，安装包体积更小，启动更轻，后续主要维护这个版本。
- 根目录 Python/CustomTkinter 版本：旧版实现，保留用于对照和迁移。

## Tauri/Rust 重构版

重构版位于 `tauri/`，使用 Tauri 2、Rust 和 TypeScript 实现。

### 新版能力

- 更小的 Windows 安装包：NSIS 安装包约 1-2 MB，独立 exe 约 4 MB。
- 安装根目录智能规范化：选择 `D:\` 时会自动使用 `D:\DevEnvManager`，避免污染盘符根目录。
- JDK、Python、Node.js、Maven、Gradle 的下载、安装、切换、卸载和 current 指针管理。
- 安装或切换运行时后自动执行健康检查，验证命令和环境变量是否真的生效。
- 发现系统已有 Java/Python/Node.js/Maven/Gradle，并尝试通过 Windows 卸载注册表启动正式卸载器。
- 环境健康检查：检查 `DEVENV_HOME`、`JAVA_HOME`、PATH、受管 JDK/Python/Node/Maven/Gradle。
- PATH 检查会区分真实失效、重复项和“托管路径待安装”，支持清理真实失效 PATH。
- 端口管理支持固定列宽排序、常用端口筛选和智能搜索，例如 `8080`、`java web`、`mysql`、`redis`。
- 后台执行安装、切换、扫描和诊断任务，避免界面卡死和命令窗口闪烁。
- 工具箱支持启动 DevEnv Manager 自身卸载程序。

### 新版开发运行

```powershell
cd tauri
npm install
npm run tauri:dev
```

### 新版打包

```powershell
cd tauri
npm run tauri:build
```

输出位于：

- `tauri\src-tauri\target\release\bundle\nsis\DevEnv Manager_0.1.0_x64-setup.exe`
- `tauri\src-tauri\target\release\bundle\msi\DevEnv Manager_0.1.0_x64_en-US.msi`
- `tauri\src-tauri\target\release\dailytools-tauri.exe`

## 功能

- JDK 17/21：通过 Eclipse Adoptium API 获取 Temurin Windows x64 ZIP。
- Python 3.10/3.11：自动选择仍提供 Windows x64 安装器的最新补丁版本。
- Node.js 20/22：通过 Node.js 官方版本索引获取 Windows x64 ZIP。
- 多版本并存，通过 `current` junction 切换当前版本。
- 仅配置当前用户的 `DEVENV_HOME`、`JAVA_HOME` 和 `PATH`。
- PATH 修改前备份，重复配置不会重复追加。
- 环境诊断、任务状态、运行时状态和最近事件日志。
- TCP/UDP 端口扫描、搜索、快捷筛选、排序、详情和复制。
- 安全结束普通进程，拦截 PID 0、PID 4 和关键系统进程。

## 旧版 Python 开发运行

要求 Windows 10/11 和 Python 3.11 或更高版本。

```powershell
py -3.11 -m venv .venv
.\.venv\Scripts\Activate.ps1
python -m pip install -r requirements.txt
python main.py
```

## 旧版 Python 打包

双击 `build_exe.bat`，或执行：

```powershell
.\build_exe.bat
```

输出位于 `dist\DevEnvManager.exe`。目标电脑不需要预装 Python、JDK 或 Node.js。

## 使用说明

1. 在“设置”中选择安装根目录，默认优先使用 `D:\DevEnvManager`。
2. 在“环境管理”中安装所需版本，安装成功后会自动激活。
3. 在首页点击“一键修复 PATH”，然后重新打开终端或 IDE。
4. 使用“一键诊断”验证命令和环境变量。
5. 在“端口管理器”中扫描、搜索和处理端口占用。

## 安全说明

- 下载只允许 Adoptium、GitHub、Node.js 和 Python 官方域名。
- JDK 与 Node.js 下载会执行官方 SHA-256 校验。
- ZIP 解压会阻止路径穿越。
- 安装路径必须位于用户选择的根目录内。
- 软件不修改系统级环境变量。
- PID 0、PID 4 和关键 Windows 进程禁止结束。

## 常见问题

**为什么修改环境变量后当前终端没有变化？**  
已启动的 CMD、PowerShell 和 IDE 不会自动刷新进程环境，请重新打开。

**为什么 PID 4 不能结束？**  
PID 4 是 Windows System 进程，直接结束会危害系统稳定性。

**为什么 80/443 被 System 占用？**  
常见原因包括 IIS、HTTP.sys、Hyper-V、Docker、WSL 或其他系统服务。

**为什么 Python 安装后没有 `py` 命令？**  
本工具刻意不安装全局 Python Launcher，避免与已有 Python 冲突。请使用 `python`。

**为什么不默认安装到 C 盘？**  
运行时和下载文件体积较大，默认优先放在 D 盘，并允许用户自定义位置。

## 测试

```powershell
python -m pytest -q
```
