# DevEnv Manager

[项目主页](https://github.com/weidonglang/DevEnv-Manager) · [Release 下载](https://github.com/weidonglang/DevEnv-Manager/releases) · [问题反馈](https://github.com/weidonglang/DevEnv-Manager/issues)

面向 Windows 的开发环境诊断器与安全操作面板。当前版本：**1.1.0**。

适合：

- Java / Spring Boot 学习者
- Python 初学者和多版本用户
- 前端开发者
- 经常被 PATH、JAVA_HOME、pip、npm、端口占用折磨的人
- 想把 JDK、Python、Node.js、Maven、Gradle 安装到固定目录并快速切换的人

一句话：Windows 开发环境乱了？先看清实际生效的版本、PATH、JAVA_HOME 和工具来源，再决定是否执行受管修复。

## 项目定位

DevEnv Manager 解决的是 Windows 上多个开发生态互相影响的问题：运行时版本不透明、`JAVA_HOME` 与 PATH 不一致、端口冲突、包管理器来源混杂、项目要求与本机环境不匹配，以及安全操作缺少预览和恢复入口。

它不是新的包管理器、构建工具或“系统管家”。项目优先做三件事：

1. **诊断**：展示可验证的路径、版本、来源和冲突证据。
2. **编排**：调用成熟工具或 DevEnv Manager 自己的受管运行时。
3. **保护**：用户级修改、固定 action id、白名单、备份、校验、只读预览和明确确认。

### 不会替代什么

本项目不会替代 uv、pip、npm、pnpm、Yarn、Vite、Maven、Gradle、Docker、WSL、Scoop、Chocolatey、mise、asdf、sdkman、pyenv、nvm、rustup 或 `dotnet` CLI。能由成熟工具完成的工作，DevEnv Manager 优先检测、解释、辅助配置或调用对应工具。

### 适合谁 / 不适合谁

- 适合不确定当前真正生效版本、需要同时维护多套 JDK/Python/Node，或经常遇到 Windows PATH 与端口问题的用户。
- 适合希望用图形界面查看诊断证据，同时保留 CLI 自动化入口的用户。
- 不适合希望软件自动接管整台机器、清理任意个人文件或替代专业包管理器的场景。
- 熟练使用 mise/asdf/Scoop/Chocolatey 且环境已经稳定的用户，可以只使用诊断能力。

## 1.1 正式版

仓库包含两个实现：

- `tauri/`：最新 Tauri 2 + Rust + TypeScript 重构版，后续主线维护。
- 根目录 Python/CustomTkinter 版本：旧版实现，保留用于对照和迁移。

## Tauri/Rust 重构版能力

- Temurin、Zulu、Liberica、Microsoft OpenJDK，以及 Python、Node.js、Maven、Gradle 的下载、安装、切换、卸载和 `current` 指针管理。
- 安装根目录智能规范化：选择 `D:\` 时自动使用 `D:\DevEnvManager`，避免把文件散在盘符根目录。
- 安装和切换后自动健康检查，验证命令和环境变量是否真的生效；JDK 会交叉核对 `JAVA_HOME`、PATH、`java`、`javac`、Maven 与 Gradle。
- 下载安装过程展示进度：查询、下载、解压、静默安装、验证。
- 用户级环境变量管理：`DEVENV_HOME`、`JAVA_HOME`、PATH 备份、恢复、清理。
- 环境医生：按类别对齐展示诊断项和修复建议，可导出 Markdown/JSON 或复制脱敏报告。
- Python 冲突分析：检测默认 `python`、`pip`、`py -0p`、注册表、Microsoft Store 执行别名风险。
- 配置模板：保存当前 JDK/Python/Node/Maven/Gradle 组合和用户环境变量，导入前预览差异，并可自动补齐缺失版本后恢复。
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
- Docker Desktop 安装、更新、启动和关闭；WSL 更新、发行版安装、启动、停止和默认发行版管理。
- MySQL、PostgreSQL、Redis、MongoDB、Elasticsearch、SQL Server 服务检测、启动、停止、重启、日志和安装目录访问。
- 程序内更新：每天最多自动检查一次，只读取固定更新清单，不上传遥测；下载包强制 SHA256 校验。
- C 盘急救大师 Phase 1：查看磁盘容量，只读扫描明确临时目录、DevEnv Manager 数据、开发缓存及 WPS 明确缓存/日志路径；默认不进入个人目录。
- 命令面板安全模式：仅允许常见开发工具，拦截系统 Shell、磁盘、注册表、权限、服务和破坏性 Git 命令。
- AI Agent / CLI 痕迹分析：用户主动触发后只读检查可验证路径与项目配置文件名，不读取会话正文、history、token 或密钥。
- `devenv` CLI：环境诊断、版本查看/切换、项目检查、清理扫描和配置模板应用。
- 网络诊断、下载缓存管理、命令面板、自身卸载入口。
- 后台执行耗时任务，隐藏 Windows 命令窗口，减少闪屏和界面卡顿。

## 使用流程

1. 打开 Tauri 新版。
2. 在“总览”确认安装根目录，默认优先使用 `D:\DevEnvManager`。
3. 在“版本管理”安装或切换 JDK / Python / Node.js / Maven / Gradle。
4. 在“环境”点击“配置”，写入用户级 `DEVENV_HOME`、`JAVA_HOME` 和受管 PATH。
5. 在“环境医生”点击“一键诊断”，查看评分、问题和建议。
6. 在“版本管理”的 Python 环境分析里检查 pip 是否和当前 Python 匹配。
7. 在“项目启动向导”输入项目目录，分析项目并运行安装依赖、测试或开发服务。
8. 在“端口”搜索 `8080`、`spring`、`mysql`、`vite` 等关键词快速定位冲突。
9. 在“工具链”检查 Git/GitHub、Node 和 Python 生态，需要时配置 Git 身份、包管理器或镜像源。
10. 在“平台/镜像”安装 Go、检查 Rust/.NET，或管理 GOPROXY、Maven 和 Gradle 镜像。

## 页面使用说明

| 页面 | 建议使用方式 |
| --- | --- |
| 总览 | 查看当前实际生效的 Java、Python、Node、Maven、Gradle、Go 版本和来源；每 30 秒只读刷新。 |
| 环境医生 | 先诊断，再按证据处理。评分只惩罚真实问题，可选工具缺失和普通端口占用不再扣分。 |
| 版本管理 | “本机环境发现”默认折叠；JDK 切换后使用“检查当前 JDK”验证完整生效链。 |
| 环境 | 只修改当前用户环境变量；写入后立即回读校验，并保留恢复快照。 |
| 项目 | 只读分析项目配置；执行时只接受后端生成的固定 action id。支持 Maven/Gradle JDK 建议和 Nacos 单机启动检查。 |
| 工具链 | 检测并辅助配置成熟生态工具，不重新实现它们。 |
| 平台/镜像 | Windows 主机与 WSL 分开看待；WSL 内 SDK 优先使用 Linux 生态成熟工具。 |
| C盘急救 | 1.1.0 仍为 scan-only，不删除文件，也不扫描个人目录。 |
| 工具箱 | 命令面板属于高级功能。不要粘贴不理解的 AI、网页或聊天命令。 |

## 下载与 SHA256 校验

正式版本以 [GitHub Releases](https://github.com/weidonglang/DevEnv-Manager/releases) 为唯一主来源。国内镜像如后续提供，只作为备用入口，并应与 Release 中的 `SHA256SUMS.txt` 对照。

```powershell
Get-FileHash .\DevEnv.Manager_1.1.0_x64-setup.exe -Algorithm SHA256
Get-FileHash .\DevEnv.Manager_1.1.0_x64_en-US.msi -Algorithm SHA256
Get-FileHash .\devenv.exe -Algorithm SHA256
```

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

- `tauri\src-tauri\target\release\bundle\nsis\DevEnv Manager_1.1.0_x64-setup.exe`
- `tauri\src-tauri\target\release\bundle\msi\DevEnv Manager_1.1.0_x64_en-US.msi`
- `tauri\src-tauri\target\release\dailytools-tauri.exe`
- `tauri\src-tauri\target\release\devenv.exe`

### Release 体积记录

使用相同的 Rust release 配置（LTO、`opt-level = "z"`、strip）对实现前后裸 exe 进行对比：

| 文件 | 1.0.0 | Phase 1 | 1.1.0 | 1.1.0 相对 Phase 1 |
| --- | ---: | ---: | ---: | ---: |
| `dailytools-tauri.exe` | 4,758,016 B | 4,726,272 B | 4,826,624 B | +100,352 B |
| `devenv.exe` | 2,073,600 B | 2,066,944 B | 2,102,784 B | +35,840 B |

1.1.0 未引入新依赖，主程序与 CLI 的增量均远低于 10 MB。

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
- 外部运行时优先使用 Windows 卸载注册表；Scoop/Chocolatey 项使用对应包管理器；严格识别的独立绿色运行时只移入回收站；IDE 内置 JDK 始终受保护。
- C 盘急救大师 Phase 1 严格为 scan-only：不删除、不移动、不清空回收站，也不修改被扫描文件。
- 默认扫描不进入 Desktop、Downloads、Documents、Pictures、Videos、Music；`C:\Windows`、Program Files、当前项目、受管运行时、浏览器凭据和微信/QQ 数据库受保护，符号链接不会被跟随。
- WPS 只匹配明确命名的 cache/temp/log 路径，备份中心、云文档、账号数据和普通文档不进入结果，1.1.0 不执行清理。
- 命令面板不是 AI 自动执行器。系统 Shell、磁盘/注册表/权限/服务命令及破坏性 Git 命令会被拒绝；安装、更新、发布类命令需要二次确认。
- 结束进程会拦截 PID 0、PID 4、System、lsass.exe、csrss.exe、wininit.exe、winlogon.exe、services.exe 等关键进程。
- 导出诊断报告会脱敏用户目录和常见敏感键值，不导出私钥、token、密码。
- SSH Key 生成发现同名密钥时会拒绝覆盖，界面只允许复制公钥，绝不读取或显示私钥。
- npm 和 pip 镜像只能从内置白名单选择，工具安装包名使用固定白名单，避免拼接任意命令。
- Go 只从 `go.dev` 白名单下载，并使用官方索引提供的 SHA256 校验安装包。
- Maven/Gradle 只写用户目录中的固定配置文件；已有文件会先生成时间戳备份，界面提供最近备份恢复入口。
- 不包含账号系统、云同步、遥测、广告或联网统计。

## 维护边界

DevEnv Manager 是个人维护项目，不承诺长期覆盖所有语言、SDK、包管理器和 Linux 发行版。维护优先级固定为：

1. 安全问题和数据保护
2. 可复现的明确 bug
3. 核心 JDK/Python/Node、PATH、端口诊断体验
4. 文档澄清和低风险、可测试功能
5. 新生态扩展

新功能必须尽量满足低风险、可回滚、可测试和维护成本可控。涉及命令执行、清理、卸载、服务停止与自动修复的功能会采用更保守的默认行为。Windows 是主平台；WSL 以诊断和编排为主，完整 Linux 桌面版本需独立评估，不会简单照搬 Windows 实现。

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

## 手动测试清单

- 全新 Windows 环境首次打开。
- 修改安装根目录，特别是选择 `D:\`。
- 安装 JDK 17/21，并切换验证。
- 安装 Python 3.12/3.14，并验证 `python -m pip --version`。
- 安装 Node.js 22，并验证 `npm --version`。
- 安装 Maven / Gradle，并验证 JAVA_HOME 处理。
- 配置用户环境变量后重新打开终端测试。
- 同时安装多个 JDK，制造 JAVA_HOME、PATH、java、javac 不一致并检查明确告警。
- 用 Maven/Gradle 项目验证项目要求、JAVA_HOME 和构建工具 JVM 的匹配结果。
- 用 Nacos 目录验证 JDK 8+ 检查与固定 action id 启动入口。
- 故意制造重复或失效 PATH，再清理。
- 运行环境医生并导出报告。
- 检测多个 Python 和 Microsoft Store Python 别名。
- 扫描 8080、5173、3306、5432、6379 端口。
- 结束普通 node/java 进程，确认系统关键进程被拦截。
- 分析 Node/Python/Java/Tauri/Rust 项目。
- 生成 VS Code 配置。
- 清理下载缓存。
- 在命令面板验证 `node --version` 可运行、`npm install` 需确认、PowerShell 与 `git reset --hard` 被拦截。
- 运行 C 盘急救扫描，确认个人目录不进入报告，WPS 仅出现明确 cache/temp/log 路径。
- 主动运行 AI Agent / CLI 痕迹分析，确认不展示 history、会话正文或敏感值。
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
- 打包 Tauri 安装包。

## 路线图

已实现的 P0 / P1 / P2：

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
- Go 受管安装、切换、卸载和代理管理
- Rust/rustup 与 MSVC Build Tools 诊断
- .NET SDK/Runtime 与 `global.json` 诊断
- Maven/Gradle 镜像配置备份和恢复
- 端口进程解释、Windows 服务映射和 7 天历史
- Docker / WSL 与数据库本地服务检查
- 配置模板导入、导出和团队分享
- Markdown/JSON 脱敏诊断报告和复制分享
- 项目 JDK 建议、JDK 发行版扩展结构和更新检查
- C 盘急救大师 Phase 1 安全底座、磁盘体检与只读扫描
- `devenv` 命令行工具
- Temurin、Zulu、Liberica、Microsoft OpenJDK 自动安装
- 程序内下载、SHA256 校验和自动升级
- Docker Desktop、WSL 发行版和数据库服务管理
- 项目端口配置识别、备份修改和占用提醒
- 配置模板差异预览和缺失版本补齐
- 环境医生安全一键修复
- 命令面板白名单与管理员态保护
- JDK/JAVA_HOME/PATH/java/javac/Maven/Gradle 一致性诊断
- Nacos Java 环境识别与受管启动
- AI Agent / CLI 痕迹只读分析

后续规划：

- 优先增加真实 Windows 环境的自动化安装与多 JDK 回归测试，不无限扩展新生态。
- 清理执行能力继续保持关闭，直到候选二次校验、回收站恢复与真实环境回归全部具备。
- 评估 WSL 内只读 SDK 诊断；不承诺短期完整 Linux 桌面版。

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
