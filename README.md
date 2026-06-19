# DevEnv Manager

面向 Windows 的一站式开发环境管理器。

适合：

- Java / Spring Boot 学习者
- Python 初学者和多版本用户
- 前端开发者
- 经常被 PATH、JAVA_HOME、pip、npm、端口占用折磨的人
- 想把 JDK、Python、Node.js、Maven、Gradle 安装到固定目录并快速切换的人

一句话：Windows 开发环境坏了？用 DevEnv Manager 一键诊断、一键修复、一键跑项目。

## 当前版本

仓库包含两个实现：

- `tauri/`：最新 Tauri 2 + Rust + TypeScript 重构版，后续主线维护。
- 根目录 Python/CustomTkinter 版本：旧版实现，保留用于对照和迁移。

## Tauri/Rust 重构版能力

- JDK、Python、Node.js、Maven、Gradle 的下载、安装、切换、卸载和 `current` 指针管理。
- 安装根目录智能规范化：选择 `D:\` 时自动使用 `D:\DevEnvManager`，避免把文件散在盘符根目录。
- 安装和切换后自动健康检查，验证命令和环境变量是否真的生效。
- 下载安装过程展示进度：查询、下载、解压、静默安装、验证。
- 用户级环境变量管理：`DEVENV_HOME`、`JAVA_HOME`、PATH 备份、恢复、清理。
- 环境医生：一键诊断、评分、问题列表、修复建议、导出 Markdown 报告。
- Python 冲突分析：检测默认 `python`、`pip`、`py -0p`、注册表、Microsoft Store 执行别名风险。
- 配置模板：保存当前 JDK/Python/Node/Maven/Gradle 组合和用户环境变量，可快速恢复。
- 项目启动向导：识别 Node/Python/Maven/Gradle/Rust/Tauri/.NET/Go 项目，给出运行时建议和常用操作。
- Git / GitHub 工具链：检测 Git、Git Bash、Git LFS、OpenSSH、用户身份、SSH 公钥和 GitHub HTTPS/SSH 连接，可安全配置身份或生成 ed25519 Key。
- Node.js 生态：检测 npm、npx、pnpm、Yarn、Corepack、registry、全局目录和 pnpm store，支持安装包管理器及切换官方源/npmmirror。
- Python 生态：检测 pip、uv、Poetry、virtualenv 和 pip 配置，支持安装工具及切换官方/国内 PyPI 镜像。
- 统一工具注册表：为 JDK、Python、Node.js、Maven、Gradle、Git、Go、Rust、.NET 和生态工具提供统一能力元数据。
- 端口管理：扫描 TCP/UDP、固定列宽排序、常用端口筛选、智能搜索、安全结束普通进程。
- 网络诊断、下载缓存管理、命令面板、自身卸载入口。
- 后台执行耗时任务，隐藏 Windows 命令窗口，减少闪屏和界面卡顿。

## 使用流程

1. 打开 Tauri 新版。
2. 在“总览”确认安装根目录，默认优先使用 `D:\DevEnvManager`。
3. 在“运行时”安装或切换 JDK / Python / Node.js / Maven / Gradle。
4. 在“环境”点击“配置”，写入用户级 `DEVENV_HOME`、`JAVA_HOME` 和受管 PATH。
5. 在“环境医生”点击“一键诊断”，查看评分、问题和建议。
6. 在“运行时”的 Python 环境分析里检查 pip 是否和当前 Python 匹配。
7. 在“项目启动向导”输入项目目录，分析项目并运行安装依赖、测试或开发服务。
8. 在“端口”搜索 `8080`、`spring`、`mysql`、`vite` 等关键词快速定位冲突。
9. 在“工具链”检查 Git/GitHub、Node 和 Python 生态，需要时配置 Git 身份、包管理器或镜像源。

## 开发运行

```powershell
cd tauri
npm install
npm run tauri:dev
```

## 打包发布

```powershell
cd tauri
npm run tauri:build
```

输出位置：

- `tauri\src-tauri\target\release\bundle\nsis\DevEnv Manager_0.2.0_x64-setup.exe`
- `tauri\src-tauri\target\release\bundle\msi\DevEnv Manager_0.2.0_x64_en-US.msi`
- `tauri\src-tauri\target\release\dailytools-tauri.exe`

## 测试

```powershell
cd tauri\src-tauri
cargo test

cd ..\
npm run build
```

## 安全说明

- 只修改当前用户级环境变量，不修改系统级环境变量。
- 下载域名走白名单：Adoptium、GitHub、Node.js、Python、Apache、Gradle 等官方源。
- ZIP 解压阻止路径穿越。
- 删除和卸载受管运行时前会校验路径必须位于 DevEnv Manager 根目录内。
- 外部运行时只通过 Windows 正式卸载注册表入口启动卸载器，不直接删除用户外部目录。
- 结束进程会拦截 PID 0、PID 4、System、lsass.exe、csrss.exe、wininit.exe、winlogon.exe、services.exe 等关键进程。
- 导出诊断报告会脱敏用户目录和常见敏感键值，不导出私钥、token、密码。
- SSH Key 生成发现同名密钥时会拒绝覆盖，界面只允许复制公钥，绝不读取或显示私钥。
- npm 和 pip 镜像只能从内置白名单选择，工具安装包名使用固定白名单，避免拼接任意命令。
- 不包含账号系统、云同步、遥测、广告或联网统计。

## 常见问题

**为什么修改环境变量后当前终端没有变化？**  
已经启动的 CMD、PowerShell、IDE 不会自动继承新环境变量，请重新打开终端或 IDE。

**为什么选择 D 盘后安装到了 `D:\DevEnvManager`？**  
这是有意设计，避免把 `current`、`envs`、`downloads` 等目录直接散在 D 盘根目录。

**为什么 Python 安装后没有全局 `py` 命令？**  
受管 Python 默认不安装全局 Python Launcher，避免和你已有的系统 Python 抢占。建议使用受管 PATH 中的 `python`，或在 Python 分析里检查现有 `py -0p`。

**为什么卸载外部运行时提示找不到卸载入口？**  
它可能是绿色版、IDE 内置运行时、Scoop/压缩包安装，或没有注册到 Windows 卸载表。此时工具不会直接删除外部目录，只会给出手动建议。

**为什么 Maven / Gradle 要求 JAVA_HOME？**  
Maven 和 Gradle 需要有效 JDK。新版会在受管命令执行时注入 `JAVA_HOME=%DEVENV_HOME%\current\jdk`，安装或切换后也会重新验证。

## 手动测试清单

- 全新 Windows 环境首次打开。
- 修改安装根目录，特别是选择 `D:\`。
- 安装 JDK 17/21，并切换验证。
- 安装 Python 3.12/3.14，并验证 `python -m pip --version`。
- 安装 Node.js 22，并验证 `npm --version`。
- 安装 Maven / Gradle，并验证 JAVA_HOME 处理。
- 配置用户环境变量后重新打开终端测试。
- 故意制造重复或失效 PATH，再清理。
- 运行环境医生并导出报告。
- 检测多个 Python 和 Microsoft Store Python 别名。
- 扫描 8080、5173、3306、5432、6379 端口。
- 结束普通 node/java 进程，确认系统关键进程被拦截。
- 分析 Node/Python/Java/Tauri/Rust 项目。
- 生成 VS Code 配置。
- 清理下载缓存。
- 启动自身卸载入口。
- 检查 Git 身份、Git LFS、SSH Key 和 GitHub HTTPS/SSH 状态。
- 配置测试用 Git 身份，确认刷新后立即显示；已有 `id_ed25519` 时确认不会覆盖。
- 检查 npm/pnpm/Yarn/Corepack，切换 npm 官方源与 npmmirror 后验证。
- 检查 pip/uv/Poetry/virtualenv，切换 PyPI 镜像并安装一个缺失工具后验证。
- 打包 Tauri 安装包。

## 路线图

已实现的 P0 / P1：

- 环境医生
- Python 多版本冲突分析
- 项目启动向导
- 诊断报告导出
- 配置模板保存和恢复
- 安装进度和切换后验证
- Git / GitHub 基础诊断、身份配置、SSH Key 生成与连接测试
- Node.js 包管理器检测、安装、registry 和全局目录管理
- Python 工具链检测、安装和 pip 镜像管理
- 统一工具元数据注册表

后续规划：

- Go、Rust、.NET SDK 更完整的检测、安装引导和项目动作。
- 镜像源与网络加速中心。
- Docker / WSL 检测。
- 数据库和本地服务端口解释器。
- 配置模板导入、导出和团队模板。
- 更新检查和 GitHub Releases 入口。
- 可选 CLI：`devenv doctor`、`devenv use jdk 21`、`devenv project check .`。

## 旧版 Python 开发运行

要求 Windows 10/11 和 Python 3.11 或更高版本。

```powershell
py -3.11 -m venv .venv
.\.venv\Scripts\Activate.ps1
python -m pip install -r requirements.txt
python main.py
```

旧版打包：

```powershell
.\build_exe.bat
```

输出位于 `dist\DevEnvManager.exe`。
