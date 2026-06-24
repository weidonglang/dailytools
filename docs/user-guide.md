# DevEnv Manager 1.3 操作手册

本手册适用于 Windows 10/11 上的 DevEnv Manager 1.3。程序定位是开发环境诊断器与安全操作面板，不替代 npm、pnpm、pip、uv、Maven、Gradle、Cargo、chsrc、Scoop、Chocolatey、WSL 等成熟工具。

## 1. 下载、安装与校验

只从项目的 GitHub Releases 页面下载安装包。发布页同时提供：

- `DevEnv.Manager_1.3.0_x64-setup.exe`：推荐安装包。
- `DevEnv.Manager_1.3.0_x64_en-US.msi`：MSI 安装包。
- `devenv.exe`：命令行工具。
- `SHA256SUMS.txt`：文件校验值。

PowerShell 校验示例：

```powershell
Get-FileHash .\DevEnv.Manager_1.3.0_x64-setup.exe -Algorithm SHA256
Get-FileHash .\DevEnv.Manager_1.3.0_x64_en-US.msi -Algorithm SHA256
Get-FileHash .\devenv.exe -Algorithm SHA256
```

结果必须与 `SHA256SUMS.txt` 完全一致。Windows SmartScreen 首次提示时，应先确认发布者、文件名和 SHA256，不要直接忽略警告。

## 2. 首次使用

1. 打开“总览”，确认默认根目录。选择盘符根目录时，程序会自动使用 `DevEnvManager` 子目录。
2. 查看“当前实际生效环境”，确认 Java、Python、Node.js、Maven、Gradle、Go 的真实路径与来源。
3. 打开“环境医生”运行诊断。先看证据，再执行修复。
4. 在“版本管理”安装或切换受管运行时。
5. 重新打开终端或 IDE，验证新的用户级环境变量。

程序不会修改系统级环境变量。已经启动的终端、IDE 和服务不会自动继承新环境变量。

## 3. 页面导航

侧边栏按工作流分组：

- 诊断：总览、环境医生、端口。
- 环境与运行时：版本管理、环境。
- 项目与生态：项目、工具链、平台/镜像。
- 维护与系统：C 盘急救、工具箱。

独立列表超过 5 项时会显示上一页、下一页、当前页和总数。端口表格每页显示 10 项；筛选或排序后分页仍然保留。

### 环境配置、备份与回滚

1. 打开“环境”，先点击“预览配置”。
2. 核对 `DEVENV_HOME`、`JAVA_HOME` 和 PATH 条目数，并检查新增、移除项和风险提示。
3. 预览只在 10 分钟内有效且只能应用一次；确认无误后再点击“应用这份预览”。
4. 如果预览后其他程序修改了用户环境变量，应用会被拒绝，必须重新预览，避免覆盖外部修改。
5. 每次应用、PATH 清理、JDK 切换或历史恢复前都会保存环境备份；页面可查看并恢复最近 20 份历史记录。

环境操作只写当前用户作用域。恢复历史备份同样需要二次确认，并会先保存恢复前的当前状态。

## 4. 版本安装与切换

### JDK

1. 选择发行版与主版本。
2. 点击安装，等待下载、SHA256 校验、解压和健康检查完成。
3. 切换 JDK 前确认目标目录。
4. 切换完成后点击“检查当前 JDK”。
5. 同时核对 `JAVA_HOME`、PATH 中的 `java`/`javac`、Maven JVM 和 Gradle JVM。

如果任何一步不一致，切换会报告错误并尝试恢复切换前的指针和用户环境变量。

### Python、Node.js、Maven、Gradle 与 Go

受管版本安装在 DevEnv Manager 根目录内，通过 `current` 指针切换。外部安装由 Scoop、Chocolatey 或 Windows 卸载注册表管理时，程序不会把它们误认为受管运行时。

下载失败时记录完整错误中的原始域名、最终重定向地址和来源。不要通过关闭安全白名单来绕过错误。

## 5. 项目分析与配置生成

1. 在“项目”页填写项目根目录并点击“分析”。
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

## 6. 镜像与 chsrc

内置镜像面板仍可管理 npm、pip、GOPROXY、Maven 与 Gradle。Maven/Gradle 配置写入前会生成时间戳备份。

1. 通过 Scoop 或 WinGet 安装官方 [RubyMetric/chsrc](https://github.com/RubyMetric/chsrc)。
2. 打开“平台/镜像”，点击检查平台工具链。
3. 在“chsrc 统一换源”选择目标。
4. 可查看当前源、列出可用源或测速。
5. “自动选择”“使用源 ID”“恢复官方源”会修改配置，必须二次确认。

DevEnv Manager 只调用官方 `chsrc`，不会复制其换源实现。为避免任意参数和 URL 注入，只支持固定目标；指定镜像时只能填写 `chsrc list <target>` 返回的源 ID，不接受自定义 URL。

## 7. C 盘急救 Phase 2/3

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
- 微信/QQ 只汇总已知数据根目录的文件元数据，不读取聊天数据库；浏览器只统计明确缓存目录，不触碰 Cookie、密码和登录态。
- 软件、网盘、剪辑软件和游戏库只展示占用、打开位置、卸载入口或迁移建议，不直接删除安装目录。

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

## 10. 常见故障

### 下载地址不在安全白名单

记录错误中的完整 URL、运行时类型、版本和发行版，然后提交 Issue。1.3 会验证每一次重定向，只允许 HTTPS 和明确登记的官方/CDN 域名。不要手动关闭校验。

### JDK 找不到或校验格式异常

先切换到 Temurin 验证网络，再记录失败发行版。1.3 兼容 Liberica API 的数组响应，并补充 Go 与 BellSoft 官方下载 CDN。

### 官方缓存命令不存在

确认工具已经安装，并重新打开 DevEnv Manager。程序会同时查找受管 Node、Python、Go 目录和当前 PATH。

### 配置应用失败

查看错误中给出的备份目录。在项目的 `.devenv-manager/backups/` 恢复文件；在“环境 → 配置模板”应用自动生成的项目切换前模板。

### 当前终端仍显示旧版本

关闭并重新打开 CMD、PowerShell、Windows Terminal 或 IDE。已启动进程不会自动继承更新后的用户环境变量。

## 11. 命令行

```powershell
devenv doctor --json
devenv list --json
devenv project check . --json
devenv cleanup scan --json
devenv profile list
devenv profile apply <id>
```

CLI 的 `cleanup scan` 仍然只扫描。真正清理必须在 GUI 中完成计划预览与二次确认。
