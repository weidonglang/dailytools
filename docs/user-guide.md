# DevEnv Manager 1.5.1 Final Stable 操作手册

本手册适用于 Windows 10/11 上的 DevEnv Manager 1.5.1。程序定位是开发环境诊断器与安全操作面板，不替代 npm、pnpm、pip、uv、Maven、Gradle、Cargo、chsrc、Scoop、Chocolatey、WSL 等成熟工具。

## 1. 下载、安装与校验

只从项目的 GitHub Releases 页面下载安装包。发布页同时提供：

- `DevEnv.Manager_1.5.1_x64-setup.exe`：推荐安装包。
- `DevEnv.Manager_1.5.1_x64_en-US.msi`：MSI 安装包。
- `devenv.exe`：命令行工具。
- `SHA256SUMS.txt`：文件校验值。

PowerShell 校验示例：

```powershell
Get-FileHash .\DevEnv.Manager_1.5.1_x64-setup.exe -Algorithm SHA256
Get-FileHash .\DevEnv.Manager_1.5.1_x64_en-US.msi -Algorithm SHA256
Get-FileHash .\devenv.exe -Algorithm SHA256
```

结果必须与 `SHA256SUMS.txt` 完全一致。Windows SmartScreen 首次提示时，应先确认发布者、文件名和 SHA256，不要直接忽略警告。

## 2. 首次使用

1. 打开“总览”，确认默认根目录。选择盘符根目录时，程序会自动使用 `DevEnvManager` 子目录。
2. 查看“当前实际生效环境”，确认 Java、Python、Node.js、Maven、Gradle、Go 的真实路径与来源。
3. 打开“环境医生”运行诊断。先看证据，再执行修复。
4. 在“版本管理”安装或切换受管运行时。
5. 在“环境”检查可靠性快照，确认 `JAVA_HOME` raw/expanded、PATH 首个命中项和工具来源。
6. 重新打开终端或 IDE，验证新的用户级环境变量。

程序不会修改系统级环境变量。已经启动的终端、IDE 和服务不会自动继承新环境变量。

## 3. 页面导航

侧边栏按工作流分组：

- 诊断：总览、环境医生、端口。
- 环境与运行时：版本管理、环境。
- 项目与生态：项目、工具链、平台/镜像。
- 维护与系统：C 盘急救、工具箱。

独立列表超过 5 项时会显示上一页、下一页、当前页和总数。端口表格每页显示 10 项；筛选或排序后分页仍然保留。

### 环境配置、备份与回滚

1. 打开“环境”，先点击“检查可靠性”。
2. 核对当前进程环境与 Windows 用户环境的 `DEVENV_HOME`、`JAVA_HOME` raw/expanded、PATH 条目、重复项、旧受管项和首个命中的 Java/Python/Node。
3. 需要普通配置时点击“预览配置”；需要修复 Java 时填写 JDK 根目录并生成“Java 稳定修复计划”。
4. 核对计划中的 diff、备份名、风险等级和执行后验证项。
5. 计划有效期为 30 分钟，只能执行一次；确认后程序会再次检查用户环境指纹，避免覆盖外部修改。
6. 每次应用、PATH 清理、JDK 切换或历史恢复前都会保存环境备份；页面可查看、比对并恢复历史记录。

环境操作只写当前用户作用域。恢复历史备份同样需要二次确认，并会先保存恢复前的当前状态。

### 安全说明与风险等级

首次启动会显示“使用前请阅读”。这段说明只保存在本机配置中，不上传任何数据。你可以在“设置 / 关于 / 安全说明”重新查看。

主要页面都会说明：

- 这个功能能做什么、不会做什么。
- 风险等级、是否需要管理员权限、是否可恢复。
- 执行前建议、可能失败原因和备份位置。

中风险和高风险操作会要求二次确认；涉及分区、服务注册/删除、数据库 Data 修复等极高风险操作会要求三次确认和指定文字确认。

### Python 诊断与修复

1. 打开“版本管理 → Python 环境分析”，点击“分析”。
2. 核对默认 `python`、默认 `pip`、`py -0p`、用户 PATH 条目数、当前进程是否仍使用旧 PATH，以及 Microsoft Store 执行别名风险。
3. 需要处理时选择“修复 pip”和/或“调整用户 PATH”，点击“生成可审计修复计划”。
4. 计划会展示准确的 Python 路径、`ensurepip`/pip 命令、PATH 新增项和备份名称；计划 10 分钟过期且只能使用一次。
5. 二次确认后，程序先保存环境备份，再执行 pip 修复和 PATH 写入，最后用同一个 Python 回读 `python -m pip --version`。

修复不会卸载其他 Python，也不会自动关闭 Microsoft Store 执行别名。Store 别名需在 Windows“管理应用执行别名”中由用户处理。1.5.1 会明确提示 `pip.exe` 不一定属于当前 `python.exe`，推荐使用 `python -m pip`。Windows 命令输出会按 UTF-16、UTF-8 与当前系统代码页依次解码，减少中文 CMD 乱码；SHA-256 文本只接受与目标文件名匹配的 64 位十六进制值。

### Python 完整性检查

安装受管 Python 后会检查：

- `python --version`
- `python -c "import sys; print(sys.executable)"`
- `python -m pip --version`
- `python -m venv --help`
- `ssl`
- `sqlite3`
- `ctypes`
- `tkinter`（可选）
- `Scripts\pip.exe`

`pip`、`venv`、`ssl`、`sqlite3`、`ctypes` 是核心检查。核心组件失败时不会登记为已安装；`tkinter` 失败会提示 GUI 相关库可能不可用。受管 Python 缺少 pip 时可生成修复计划，计划会显示 `ensurepip` 与 pip 升级命令。非受管 Python 只提示问题，不替用户修改系统安装。

## 4. 版本安装与切换

### JDK

1. 选择发行版与主版本。
2. 点击安装，等待下载、SHA256 校验、解压和健康检查完成。
3. 切换 JDK 前确认目标目录。
4. 切换完成后点击“检查当前 JDK”。
5. 同时核对 `JAVA_HOME`、PATH 中的 `java`/`javac`、Maven JVM 和 Gradle JVM。`JAVA_HOME` 必须是 JDK 根目录，不能是 `bin` 目录，也不能写成间接引用。

如果任何一步不一致，切换会报告错误并尝试恢复切换前的指针和用户环境变量。

### Python、Node.js、Maven、Gradle 与 Go

受管版本安装在 DevEnv Manager 根目录内，通过 `current` 指针切换。外部安装由 Scoop、Chocolatey 或 Windows 卸载注册表管理时，程序不会把它们误认为受管运行时。

下载失败时记录完整错误中的原始域名、最终重定向地址和来源。不要通过关闭安全白名单来绕过错误。

## 5. 项目分析与配置生成

1. 在“项目”页点击“选择文件夹”，或填写项目根目录并点击“分析”。
2. 确认识别到的项目类型、配置文件、运行时要求和固定操作。
3. 点击“生成 VS Code / IDEA 配置预览”。
4. 在预览面板选择要写入的文件，并按需微调内容。
5. 如需切换运行时，在 JDK、Python、Node.js、Maven、Gradle、Go 下拉框选择已安装版本。
6. 点击“二次确认并应用”，阅读写入文件数和切换数量后确认。

安全边界：

- 只允许写入 `.vscode/settings.json`、`.vscode/tasks.json`、`.idea/misc.xml`、`.idea/compiler.xml`。
- 单个配置文件最大 64 KB。
- 符号链接文件或符号链接目录会被拒绝。
- 已有文件备份到 `.devenv-manager/backups/<时间戳>/`。
- 切换运行时前自动创建“自动备份 项目切换 <时间戳>”环境模板。
- 任一运行时切换失败时，程序会尝试恢复项目文件和切换前环境模板。

### IDEA / IntelliJ 配置只读分析

点击“只读读取 IDEA 配置”后，程序只读取：

- `.idea/misc.xml`
- `.idea/modules.xml`
- `.idea/compiler.xml`
- `*.iml`

如果存在 `.idea/workspace.xml`，只提示它存在，不全量导出最近文件、历史路径、token、密码或私人配置。分析结果会展示 Project SDK、language level、模块 SDK、Maven importer JDK、Gradle JVM、编译目标和当前 `JAVA_HOME` 是否大致匹配。程序不会自动修改 `.idea`。

### Nacos / Nexus / Maven / Gradle Java 验证

项目页可以验证 Nacos 或 Nexus 会看到的 Java 环境。验证会读取最新 Windows 用户环境，展开 `JAVA_HOME`，检查 `java.exe`、`javac.exe`、PATH 首个 Java、间接引用风险，以及当前进程环境是否落后。Maven、Gradle、Spring Boot bat/cmd 脚本也会使用同一套解释口径。

## 6. 镜像与 chsrc

内置镜像面板仍可管理 npm、pip、GOPROXY、Maven 与 Gradle。Maven/Gradle 配置写入前会生成时间戳备份。

1. 通过 Scoop 或 WinGet 安装官方 [RubyMetric/chsrc](https://github.com/RubyMetric/chsrc)。
2. 打开“平台/镜像”，点击检查平台工具链。
3. 在“chsrc 统一换源”选择目标。
4. 可查看当前源、列出可用源或测速。
5. “自动选择”“使用源 ID”“恢复官方源”会修改配置，必须二次确认。

DevEnv Manager 只调用官方 `chsrc`，不会复制其换源实现。为避免任意参数和 URL 注入，只支持固定目标；指定镜像时只能填写 `chsrc list <target>` 返回的源 ID，不接受自定义 URL。

## 7. C 盘急救 Phase 2/3/4

完整流程固定为：

```text
扫描 → 选择 → 计划预览 → 二次确认 → 重新扫描与校验 → 清理 → 验证 → 报告
```

### 保守清理

只选择 DevEnv Manager 下载缓存和超过 24 小时的旧日志。适合第一次使用。

### 推荐清理

在保守清理基础上，加入用户 Temp 和 LocalAppData Temp 中超过 24 小时、且未被保护规则排除的普通项目。

### 专家扫描

默认折叠，只展示系统缓存、回收站、WPS 明确缓存路径等高风险统计。专家扫描不代表允许清理，高风险项目不会进入清理计划。

### 清理计划

计划只包含用户当前勾选、后端本轮扫描确认可清理的项目。计划有效期为 30 分钟，只能执行一次。执行时会再次扫描并逐项核对 ID、路径、分类、风险和符号链接状态；前端修改计划内容会被拒绝。

普通文件与目录优先移入 Windows 回收站。清理后路径仍存在时不会计入释放空间，并会记录为失败。

### 永远不会自动清理

- `C:\Windows`、Program Files、ProgramData\Microsoft。
- Windows Temp、Windows Update、Windows.old、休眠文件、分页文件和系统还原点。
- 回收站本身。
- Desktop、Downloads、Documents、Pictures、Videos、Music。
- 当前项目和 DevEnv Manager 受管运行时。
- 浏览器用户目录、Cookie、Login Data、密码存储。
- 微信、QQ 数据库和用户数据。
- WPS 文档、备份中心、云文档和账号数据。
- Maven `.m2/repository`、Gradle caches、Cargo registry/cache 和 Cargo target。

### Phase 3 只读空间分析

- “桌面急救”和“下载目录”按类型、大小与修改时间分类，给出整理建议，不删除或移动文件。
- “大文件”只扫描用户指定目录，展示 Top 100；可以复制路径或打开所在位置。
- “重复文件”必须再次确认扫描范围，默认最小 10 MB；先按大小筛选，再计算 SHA256，结果不提供删除按钮。
- 大文件和重复文件结果可以加入“空间搬家 → 归档计划”；Phase 3 只保存普通文件路径与大小，不移动文件。聊天、浏览器凭据、系统目录、当前项目和受管运行时会被拒绝。
- 微信/QQ 只汇总已知数据根目录的文件元数据，不读取聊天数据库；浏览器只统计明确缓存目录，不触碰 Cookie、密码和登录态。
- 软件、网盘、剪辑软件和游戏库只展示占用、打开位置、卸载入口或迁移建议，不直接删除安装目录。

### Phase 4 空间搬家与 C 盘扩容

空间搬家必须先生成计划，再执行确认：

```text
选择源目录和目标盘 → 生成 MovePlan → 检查风险和目标路径 → 二次确认 → 执行 → 写回滚记录/报告
```

支持的模式：

- 归档整理：桌面/下载目录按安装包、压缩包、视频、图片、ISO 和旧文件分类移动到目标盘。
- 缓存搬家：仅允许白名单开发缓存目录，执行前请关闭相关工具。
- 用户目录搬家：仅 Documents/Pictures/Videos/Music 等用户确认目录，默认高风险提醒。
- Junction 桥接：复制到非 C 盘目标、校验文件数量和总大小、把源目录改名为 `.devenv-backup-*`、创建 Junction，并写入可自动回滚记录。

禁止搬家 Windows、Program Files、ProgramData\Microsoft、AppData\Roaming\Microsoft、浏览器 Cookie/密码、微信/QQ 数据库、杀毒/驱动/系统服务目录、DevEnv Manager 受管运行时 current 和当前项目目录。

“扩容检测”会只读解析 C 盘所在磁盘、右侧相邻分区、未分配空间、恢复分区阻挡、D 盘是否同盘和疑似 BitLocker 状态。只有以下计划允许执行：

- `safe_extend_unallocated`：C 盘右侧紧邻未分配空间，NTFS，未发现 BitLocker 风险。
- `delete_empty_adjacent_partition_then_extend`：C 盘右侧紧邻空分区，用户确认没有文件，三次确认后才允许。

恢复分区阻挡、D 盘不相邻、有数据、不同物理磁盘等情况只生成解释报告，不提供执行按钮。分区操作前务必备份重要数据并接入电源。

## 8. 开发缓存清理

开发缓存通过官方命令清理，不直接删除缓存目录：

| 按钮 | 执行命令 |
| --- | --- |
| npm | `npm cache clean --force` |
| pnpm | `pnpm store prune` |
| Yarn | `yarn cache clean` |
| pip | `python -m pip cache purge` |
| uv | `uv cache clean` |
| Poetry | `poetry cache clear pypi --all` |
| Go 构建缓存 | `go clean -cache` |
| Go 模块缓存 | `go clean -modcache` |
| NuGet | `dotnet nuget locals all --clear` |

每次执行前都会显示确认。缺少命令时只返回友好错误，不会改为直接删除目录。

Go 模块缓存可能需要重新下载大量依赖；离线工作前不要清理。Maven、Gradle、Cargo 和项目 target 保持只读扫描。

## 9. 清理报告与恢复

清理报告显示：

- 实际释放字节数。
- 已完成、跳过、失败数量。
- 每个失败路径和原因。
- 计划 ID、开始与完成时间。

可以复制 Markdown 报告，或导出 JSON 到：

```text
<DevEnvManager 根目录>\reports\cleanup-report-<时间戳>.json
```

被移入回收站的普通项目可以通过 Windows 回收站恢复。开发缓存官方命令通常不可撤销，但缓存可由对应工具重新生成。

## 9.1 学习中心与推荐工具

首页和“学习中心”提供以下官方项目入口：

| 工具 | 适用场景 | 主要边界 |
| --- | --- | --- |
| Scoop | Windows CLI 与开发工具安装 | Scoop 安装的软件继续由 Scoop 管理，不直接删除目录。 |
| mise | 项目级多语言版本管理 | 避免与多个版本管理器同时抢占同一个 PATH。 |
| vfox | 通过插件管理多种 SDK | 使用前检查插件来源与项目配置。 |
| uv | Python 项目、虚拟环境和依赖 | 不代表系统全局 Python 来源已经正确。 |
| chsrc | 查看、测速和切换软件源 | 不安装运行时，不接受任意 URL 注入。 |

只读命令练习区允许固定的版本、位置与环境检查，例如 `python --version`、`python -m pip --version`、`py -0p`、`java -version`、`where.exe python`、`dotnet --info`。安装、配置、Shell、删除和包发布命令会被后端拒绝。学习中心不会自动配置环境。

## 9.2 MySQL 修复中心

打开“工具箱 → MySQL 修复中心”，点击“只读深度诊断”。程序检查常见 Program Files/ProgramData 路径，不默认深扫整个磁盘。

诊断会展示：

- 服务状态、推断服务名、`mysqld.exe`、`my.ini`、`basedir`、`datadir` 与端口。
- Data 是否可读，MySQL 5.x 的 `host.frm`、`user.frm`、`db.frm`、`plugin.frm` 是否齐全。
- 排除系统库后的候选业务库目录。
- 已有 `.err` 日志最后 80 行；不会自动运行可能写入 Data 的控制台实例。

安全流程：

```text
只读诊断 → 选择动作 → 一次性计划 → 二次确认 → 重新诊断 → 执行 → 再次诊断
```

- 服务注册与启动使用参数化进程调用，不把路径拼进任意 Shell。
- Data 备份目标必须不存在或为空，且不能位于 Data 内部；不跟随符号链接。
- 补回系统库前，必须存在 24 小时内由本程序完成的同一 Data 备份记录。
- 只在目标 `datadir\mysql` 不存在时复制 `basedir\data\mysql`，拒绝覆盖。
- root 认证恢复只生成按 MySQL 5.x/8.0 区分的人工向导；程序不接收、记录或导出密码。
- 服务恢复后使用 `mysqldump -p` 导出候选业务库，密码由数据库终端安全读取。

CLI 只开放诊断和计划生成：

```powershell
devenv db doctor mysql --json
devenv db repair-plan mysql <candidate-id> <backup|register_service|start_service|repair_system_schema|reset_root_guide|dump_guide>
```

CLI 不提供绕过 GUI 二次确认直接修改 Data 的入口。

## 10. 常见故障

### 下载地址不在安全白名单

记录错误中的完整 URL、运行时类型、版本和发行版，然后提交 Issue。1.5 会验证每一次重定向，只允许 HTTPS 和明确登记的官方/CDN 域名。不要手动关闭校验。

### JDK 找不到或校验格式异常

先切换到 Temurin 验证网络，再记录失败发行版。1.5 兼容 Liberica API 的数组响应，并补充 Go 与 BellSoft 官方下载 CDN。

### 官方缓存命令不存在

确认工具已经安装，并重新打开 DevEnv Manager。程序会同时查找受管 Node、Python、Go 目录和当前 PATH。

### 配置应用失败

查看错误中给出的备份目录。在项目的 `.devenv-manager/backups/` 恢复文件；在“环境 → 配置模板”应用自动生成的项目切换前模板。

### 当前终端仍显示旧版本

关闭并重新打开 CMD、PowerShell、Windows Terminal 或 IDE。已启动进程不会自动继承更新后的用户环境变量。

### Nacos 仍识别不到 Java

先在“环境”检查可靠性快照，再生成 Java 稳定修复计划。1.5 继续要求 `JAVA_HOME` 写成真实绝对路径，例如 `D:\DevEnvManager\current\jdk`，避免 Nacos 或脚本不展开 `%DEVENV_HOME%`。Nacos 验证会重新读取最新用户环境，要求 `JAVA_HOME` 同时存在 `bin\java.exe` 与 `bin\javac.exe`，回读版本后说明子进程会看到的 JAVA_HOME 和 Java 路径；如果仍失败，复制页面显示的 JAVA_HOME、Java 版本和 Nacos 根目录提交 Issue。

### Maven/Gradle 显示已安装但不可用

1.5 继续支持 Maven/Gradle 幂等修复。如果 Maven/Gradle 目录已经存在，程序会重新验证 `mvn.cmd` / `gradle.bat`，重新登记安装记录，修复 `current` 指针，并在受管命令中注入已验证的绝对 `JAVA_HOME`。Gradle 输出中的分隔线不会再被当作版本号展示。如果仍失败，请复制验证输出和安装目录提交 Issue。

## 11. 命令行

```powershell
devenv doctor --json
devenv env inspect
devenv env inspect --json
devenv env plan java --jdk "D:\DevEnvManager\current\jdk"
devenv env apply <plan-id> --confirm-risk
devenv env verify
devenv env backups
devenv env restore <backup-name> --confirm-risk
devenv java verify
devenv python verify
devenv nacos verify <nacos-root>
devenv safety disclaimer
devenv safety risks
devenv list --json
devenv project check . --json
devenv cleanup scan --json
devenv profile list
devenv profile apply <id>
```

CLI 的 `cleanup scan` 仍然只扫描。真正清理必须在 GUI 中完成计划预览与二次确认。
