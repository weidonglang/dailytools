# DevEnv Manager

面向 Windows 的开发环境诊断器 + 安全操作面板。

它不是 IDE，不是包管理器，也不是要替代 uv、pnpm、Vite、Scoop、WSL、Docker、mise、sdkman 这类成熟工具。它更像一个轻量的 Windows 开发环境整理工具：帮你看清楚本机 JDK、Python、Node.js、PATH、JAVA_HOME、pip、npm、端口、工具链和项目启动配置到底哪里乱了，然后尽量用可预览、可回滚、低打扰的方式处理。

一句话：Windows 开发环境坏了，先诊断清楚，再安全修。

## 适合谁

- Java / Spring Boot 学习者，尤其是经常切换 JDK 8 / 11 / 17 / 21 / 25 的用户。
- Python 初学者、多版本用户，以及经常遇到 `python`、`pip`、`py` 指向混乱的人。
- 前端开发者，尤其是同时使用 Node.js、npm、pnpm、Yarn、Vite 的 Windows 用户。
- 经常被 PATH、JAVA_HOME、pip、npm、端口占用、镜像源配置折腾的人。
- 想把 JDK、Python、Node.js、Maven、Gradle、Go 安装到固定目录并快速切换的人。
- 想给学弟学妹、实验室电脑或教学机快速整理开发环境的人。

## 不适合谁

- 已经熟练使用 WSL、Scoop、Chocolatey、mise、asdf、sdkman、pyenv、nvm 等工具，并且不需要 GUI 诊断面板的用户。
- 希望一个工具永久覆盖所有语言、所有 SDK、所有包管理器和所有 Linux 发行版的用户。
- 希望让 AI 或工具自动接管终端、自动执行未知命令的用户。

如果你已经有成熟工作流，建议继续使用原来的工具。DevEnv Manager 更适合做诊断、解释、辅助配置和受管目录内的版本切换。

## 项目定位

DevEnv Manager 的目标是：

- 优先诊断，而不是上来就修改系统。
- 优先用户级配置，而不是系统级配置。
- 优先调用或检测成熟工具，而不是重新实现完整生态。
- 优先可预览、可恢复、可拒绝的操作。
- 优先解决 Windows 新手和多环境用户的真实痛点。

它不会刻意追求“大而全”。后续新功能会尽量遵守：低风险、可回滚、可测试、维护成本可控。

## 下载与校验

主下载地址建议使用 GitHub Release：

https://github.com/weidonglang/DevEnv-Manager/releases

如果后续提供国内镜像下载，会尽量同时提供 SHA256。建议优先从 GitHub Release 下载；从镜像下载时，请对照发布说明中的 SHA256 校验安装包。

## 1.0 正式版

仓库包含两个实现：

- `tauri/`：Tauri 2 + Rust + TypeScript 重构版，后续主线维护。
- 根目录 Python/CustomTkinter 版本：旧版实现，保留用于对照和迁移。

## Tauri/Rust 重构版能力

- Temurin、Zulu、Liberica、Microsoft OpenJDK，以及 Python、Node.js、Maven、Gradle、Go 的下载、安装、切换、卸载和 `current` 指针管理。
- 安装根目录智能规范化：选择 `D:\` 时自动使用 `D:\DevEnvManager`，避免把文件散在盘符根目录。
- 安装和切换后自动健康检查，验证命令和环境变量是否真的生效。
- 下载安装过程展示进度：查询、下载、解压、静默安装、验证。
- 用户级环境变量管理：`DEVENV_HOME`、`JAVA_HOME`、PATH 备份、恢复、清理。
- 环境医生：按类别展示诊断项和修复建议，可导出 Markdown/JSON 或复制脱敏报告。
- Python 冲突分析：检测默认 `python`、`pip`、`py -0p`、注册表、Microsoft Store 执行别名风险。
- 配置模板：保存当前 JDK/Python/Node/Maven/Gradle/Go 组合和用户环境变量，导入前预览差异，并可自动补齐缺失版本后恢复。
- 项目启动向导：识别 Node/Python/Maven/Gradle/Rust/Tauri/.NET/Go 项目，给出运行时建议和常用操作。
- Git / GitHub 工具链：检测 Git、Git Bash、Git LFS、OpenSSH、用户身份、SSH 公钥和 GitHub HTTPS/SSH 连接，可安全配置身份或生成 ed25519 Key。
- Node.js 生态：检测 npm、npx、pnpm、Yarn、Corepack、registry、全局目录和 pnpm store，支持安装包管理器及切换官方源/npmmirror。
- Python 生态：检测 pip、uv、Poetry、virtualenv 和 pip 配置，支持安装工具及切换官方/国内 PyPI 镜像。
- 统一工具注册表：为 JDK、Python、Node.js、Maven、Gradle、Git、Go、Rust、.NET 和生态工具提供统一能力元数据。
- Go 管理：从 `go.dev` 官方索引解析稳定版，校验 SHA256 后安装、切换和卸载 Windows x64 ZIP。
- Rust / rustup 诊断：检测 rustup、rustc、Cargo、已安装工具链、默认工具链和 MSVC Build Tools，支持切换 stable 与更新。
- .NET SDK 诊断：检测 SDK/Runtime 列表，识别项目 `global.json`，支持 restore、build 和 test 项目动作。
- 镜像加速中心：集中查看 npm、pip、GOPROXY、Maven、Gradle 和 Cargo 配置，Maven/Gradle 写入前自动备份并可恢复。
- 端口管理：固定列宽排序、智能搜索、实时新增占用提醒、进程/父进程/Windows 服务解释、7 天历史、安全结束进程，以及 Spring Boot、Tomcat、Vite、`.env` 项目端口识别和备份修改。
- Docker Desktop、WSL 和常见数据库服务检测与基础管理。
- 程序内更新：检查官方 Release，下载更新包时强制 SHA256 校验。
- 开发缓存清理：扫描临时文件、开发缓存、浏览器纯缓存和崩溃转储，逐项预览后移入 Windows 回收站，并保存清理历史。
- `devenv` CLI：环境诊断、版本查看/切换、项目检查、清理扫描和配置模板应用。
- 网络诊断、下载缓存管理、命令面板、自身卸载入口。
- 后台执行耗时任务，隐藏 Windows 命令窗口，减少闪屏和界面卡顿。

## 使用流程

1. 打开 Tauri 新版。
2. 在“总览”确认安装根目录，默认优先使用 `D:\DevEnvManager`。
3. 在“版本管理”安装或切换 JDK / Python / Node.js / Maven / Gradle / Go。
4. 在“环境”点击“配置”，写入用户级 `DEVENV_HOME`、`JAVA_HOME` 和受管 PATH。
5. 在“环境医生”点击“一键诊断”，查看评分、问题和建议。
6. 在“版本管理”的 Python 环境分析里检查 pip 是否和当前 Python 匹配。
7. 在“项目启动向导”输入项目目录，分析项目并运行安装依赖、测试或开发服务。
8. 在“端口”搜索 `8080`、`spring`、`mysql`、`vite` 等关键词快速定位冲突。
9. 在“工具链”检查 Git/GitHub、Node 和 Python 生态，需要时配置 Git 身份、包管理器或镜像源。
10. 在“平台/镜像”检查 Go、Rust、.NET、Docker、WSL 和常见镜像配置。

## 命令行 CLI

安装包附带的 `devenv.exe` 可用于终端和自动化：

```powershell
devenv doctor
devenv doctor --json
devenv list --json
devenv use jdk 21
devenv use python 3.12
devenv project check .
devenv cleanup scan --json
devenv profile list
devenv profile apply "Java 21 + Python 3.12"
```

## 安全说明

DevEnv Manager 会尽量保守处理高风险操作，但它仍然是开发环境管理工具，请先看清楚提示再执行。

- 只修改当前用户级环境变量，不修改系统级环境变量。
- 下载域名走白名单：Adoptium、GitHub、Node.js、Python、Apache、Gradle、Go 等官方源。
- ZIP 解压阻止路径穿越。
- 删除和卸载受管运行时前会校验路径必须位于 DevEnv Manager 根目录内。
- 外部运行时优先使用 Windows 卸载注册表；Scoop/Chocolatey 项使用对应包管理器；严格识别的独立绿色运行时只移入回收站；IDE 内置 JDK 始终受保护。
- 开发缓存清理只接受本轮扫描返回的候选 ID，执行前重新扫描并校验真实路径；受管运行时、`current`、文档目录和符号链接不会进入候选。
- 清理优先移入 Windows 回收站，不做不可恢复的永久删除。
- 结束进程会拦截 PID 0、PID 4、System、lsass.exe、csrss.exe、wininit.exe、winlogon.exe、services.exe 等关键进程。
- 导出诊断报告会脱敏用户目录和常见敏感键值，不导出私钥、token、密码。
- SSH Key 生成发现同名密钥时会拒绝覆盖，界面只允许复制公钥，绝不读取或显示私钥。
- npm 和 pip 镜像只能从内置白名单选择，工具安装包名使用固定白名单，避免拼接任意命令。
- Go 只从 `go.dev` 白名单下载，并使用官方索引提供的 SHA256 校验安装包。
- Maven/Gradle 只写用户目录中的固定配置文件；已有文件会先生成时间戳备份，界面提供最近备份恢复入口。
- 启动时自动检查更新应可关闭、低打扰、失败静默降级，不上传用户环境信息。
- 不包含账号系统、云同步、遥测、广告或联网统计。

### 关于命令面板

工具箱中的命令面板属于高级功能。它用于运行开发相关命令，不建议粘贴 AI、网页或聊天中来源不明的命令。

后续会继续收紧命令面板的安全边界，包括白名单、风险拦截、管理员态提示和强确认。受管清理、受管卸载、环境变量修复与命令面板是不同风险等级的功能，请不要把命令面板当成自动执行器。

## 维护边界

DevEnv Manager 是个人维护项目，目标是帮助 Windows 用户诊断和整理开发环境，而不是替代 uv、pnpm、Vite、Docker、WSL、mise、sdkman、Scoop 等成熟工具。

本项目会优先保证以下能力：

- 环境诊断。
- 受管运行时安装与切换。
- 用户级环境变量修复。
- 可预览、可回滚的安全操作。
- 对成熟工具的检测与辅助配置。

本项目不会承诺长期覆盖所有语言、所有 SDK、所有包管理器和所有 Linux 发行版。

涉及命令执行、清理、卸载、服务停止等高风险操作时，请先确认影响范围。项目会尽量采用白名单、预览、回收站、备份和用户级配置，避免不可逆修改。

## 维护策略

- 安全问题优先级最高。
- 明确 bug 优先于新功能。
- 文档边界优先于功能宣传。
- 新功能必须满足：低风险、可回滚、可测试、维护成本可控。
- 能调用成熟工具解决的问题，不重新实现。
- 默认不执行破坏性操作，优先提供诊断和建议。
- 高风险功能必须有明确提示和二次确认。

## 常见问题

**为什么修改环境变量后当前终端没有变化？**  
已经启动的 CMD、PowerShell、IDE 不会自动继承新环境变量，请重新打开终端或 IDE。

**为什么选择 D 盘后安装到了 `D:\DevEnvManager`？**  
这是有意设计，避免把 `current`、`envs`、`downloads` 等目录直接散在 D 盘根目录。

**为什么 Python 安装后没有全局 `py` 命令？**  
受管 Python 使用官方 NuGet 完整包直接解压到 DevEnv Manager 根目录，并验证 `python`、`pip` 和 `venv`，但不会安装全局 Python Launcher，以免和已有系统 Python 抢占。建议使用受管 PATH 中的 `python`。

**为什么有些外部运行时仍不能卸载？**  
IDE 内置 JDK、无法确认所有权的目录和未知便携版会被有意拦截。注册表、Scoop、Chocolatey 和严格识别的独立绿色运行时可直接处理。

**为什么 Maven / Gradle 要求 JAVA_HOME？**  
Maven 和 Gradle 需要有效 JDK。新版会在受管命令执行时注入 `JAVA_HOME=%DEVENV_HOME%\current\jdk`，安装或切换后也会重新验证。

**为什么不直接推荐所有人用 WSL 或 Scoop？**  
会用 WSL、Scoop、mise、asdf、sdkman 的用户可以继续用它们。DevEnv Manager 面向的是更需要 GUI 诊断、环境解释、JDK/Python/Node 快速切换和安全修复入口的 Windows 用户。后续会优先做好 WSL/Scoop 检测与提示，而不是立刻接管它们。

**会做 Linux 版吗？**  
会先评估 WSL 场景。完整 Linux 版不会简单照搬 Windows 逻辑，因为 Linux 生态已有很多成熟工具。更合适的方向可能是检测和编排已有工具，而不是重新造一套。

**AI 通过命令行部署的运行时或 SDK 能扫出来吗？**  
能扫出一部分。只要 AI 最后把运行时、SDK、PATH、项目配置、包管理器配置或日志落到本机，就可以通过环境变量、PATH、项目文件、注册表、工具链配置等方式诊断。若只是临时执行过命令且没有留下文件、环境变量或日志，就无法凭空判断。后续可以考虑增加 AI/CLI 执行痕迹检测，但必须先做脱敏和用户授权。

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

- `tauri\src-tauri\target\release\bundle\nsis\DevEnv Manager_1.0.0_x64-setup.exe`
- `tauri\src-tauri\target\release\bundle\msi\DevEnv Manager_1.0.0_x64_en-US.msi`
- `tauri\src-tauri\target\release\dailytools-tauri.exe`
- `tauri\src-tauri\target\release\devenv.exe`

## 测试

```powershell
cd tauri\src-tauri
cargo test

cd ..\
npm run build
```

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
- 安装并切换 Go 1.25/1.26，验证 `go version` 与 `go env`。
- 切换 GOPROXY 后重新检查，确认只修改当前用户环境变量。
- 检查 rustup、Rust 工具链和 MSVC Build Tools，执行 stable 切换。
- 检查 .NET SDK/Runtime，并用含 `global.json` 的项目验证版本匹配提示。
- 写入 Maven/Gradle 测试镜像，确认先生成备份，再测试最近备份恢复。
- 测试命令面板风险提示和高风险命令拦截策略。
- 打包 Tauri 安装包。

## 路线图

已实现的 P0 / P1 / P2：

- 环境医生。
- Python 多版本冲突分析。
- 项目启动向导。
- 诊断报告导出。
- 配置模板保存和恢复。
- 安装进度和切换后验证。
- Git / GitHub 基础诊断、身份配置、SSH Key 生成与连接测试。
- Node.js 包管理器检测、安装、registry 和全局目录管理。
- Python 工具链检测、安装和 pip 镜像管理。
- 统一工具元数据注册表。
- Go 受管安装、切换、卸载和代理管理。
- Rust/rustup 与 MSVC Build Tools 诊断。
- .NET SDK/Runtime 与 `global.json` 诊断。
- Maven/Gradle 镜像配置备份和恢复。
- 端口进程解释、Windows 服务映射和 7 天历史。
- Docker / WSL 与数据库本地服务检查。
- 配置模板导入、导出和团队分享。
- Markdown/JSON 脱敏诊断报告和复制分享。
- 项目 JDK 建议、JDK 发行版扩展结构和更新检查。
- 安全存储扫描、预览、回收站清理和历史记录。
- `devenv` 命令行工具。
- Temurin、Zulu、Liberica、Microsoft OpenJDK 自动安装。
- 程序内下载、SHA256 校验和自动升级。
- Docker Desktop、WSL 发行版和数据库服务管理。
- 项目端口配置识别、备份修改和占用提醒。
- 配置模板差异预览和缺失版本补齐。
- 环境医生安全一键修复。

后续规划：

- 收紧命令面板安全边界：白名单、风险拦截、管理员态提示和强确认。
- 明确产品定位和维护边界，避免无限扩展新生态。
- 优先打磨 JDK 多版本切换、JAVA_HOME/PATH 验证、Maven/Gradle JDK 匹配。
- 增强 WSL、Scoop、Chocolatey 检测和解释，但不默认接管它们。
- 增加 AI/CLI 执行痕迹检测的可行性评估，前提是本地读取、脱敏展示、用户授权。
- 启动时自动检查更新保持可配置、低打扰、不阻塞启动、不上传遥测。
- 在不牺牲预览与回收站保护的前提下扩充开发缓存清理类型。

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
