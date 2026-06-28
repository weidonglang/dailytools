# DevEnv Manager v1.5.3

## 定位

v1.5.3 是质量补丁与稳定版收口，重点不是新增系统管家功能，而是修复真实用户反馈、减少误判、补齐安全边界和可恢复路径。

Tauri identifier 继续保留 `com.weidonglang.dailytools`，Rust/npm 包名继续保留 `dailytools-tauri`，用于兼容旧安装、升级路径和本地配置目录；产品展示名、Release 标题和下载文件名统一为 DevEnv Manager。

## 主要更新

### 1. 端口管理

- 新端口工作台布局，支持摘要、搜索、筛选、轻量表格和详情面板。
- Steam / QQ / Chrome / VS Code / unknown 端口误判抑制。
- 区分 LISTENING / ESTABLISHED，不把已建立连接当成本地监听服务。
- 展示端口识别证据、置信度、冲突证据和风险提示。

### 2. 外部运行时安全

- 外部 Go / JDK / Python / 包管理器目录不再由 DevEnv Manager 删除。
- Scoop / Chocolatey / mise / asdf / nvm / rustup 等路径只提供查看、复制、打开目录或系统卸载入口。
- 避免把外部运行时当成受管版本切换或清理。

### 3. 首次启动安全声明与白屏兜底

- 首次启动强制安全声明，确认后记录 version / acceptedAt。
- 增加白屏错误页，可重试、重置 UI 配置、打开日志目录和复制诊断信息。

### 4. MySQL 修复中心

- 增加结构化诊断结论：Healthy、UsableWithWarnings、PotentialRisk、LikelyBroken、PermissionUnknown、MultipleInstancesAmbiguous、UnsupportedLayout、FalsePositiveSuspected。
- 增加误报抑制：服务可用、端口监听和连接验证正常时，不再仅凭静态文件异常标记为严重损坏。
- 展示 backup manifest，包括最近备份时间、备份目录、文件数、总大小、ibdata1、业务库、系统库和有效期。
- 高危系统库修复前校验 backup manifest、确认语、confirmation token，并在执行前重新诊断。

### 5. Python / chsrc

- Python Store Alias 诊断展示当前 python / pip、PATH 首个 python / pip、`python -m pip`、`py -0p`、WindowsApps 和受管 Python 状态。
- 提供打开 Windows 应用执行别名设置、切换/安装受管 Python、重新检测和导出只读诊断报告入口。
- chsrc 缺失时展示原因、Scoop/WinGet 安装命令、官方项目入口和 npm / pip / GOPROXY / Maven / Gradle / Cargo 单项 fallback。

### 6. JDK 候选管理

- 区分 Managed / External / SystemInstaller / Scoop / Chocolatey / Mise / Asdf / IdeBundled / Unknown。
- 外部 JDK 可验证、复制路径、打开目录、作为用户级 JAVA_HOME 候选，但不可卸载、删除或接管第三方管理器目录。

### 7. 页面帮助和扫描体验

- 页面帮助支持每页默认折叠偏好，首次进入默认展开，配置异常时默认展开。
- 大文件 / 重复文件扫描支持进度、取消、访问文件数、候选文件数、quick hash / full hash 阶段提示。
- 打开分析路径时尽量定位到具体文件；路径不存在时提示重新扫描。

### 8. 隐私脱敏

- 报告、JSON、commandLine、Python 诊断等统一脱敏。
- 覆盖 password、passwd、pwd、token、secret、apikey、api_key、access_key、private_key、Authorization Bearer、`--token`、`--password` 和 Windows 用户目录。
- 保留正常版本号、端口号、工具名和 MySQL 错误码。

### 9. 前端维护性

- 完成 v1.5.3 稳定版前的低风险模块化收口：抽出通用类型、Tauri API 入口和 safety 组件聚合。
- 端口、MySQL、Python 等大页面逻辑暂不在 1.5.3 发布前重写，避免稳定版引入大规模 UI 回归；页面级拆分留到 1.5.4。

## 不做什么

- 不做杀毒。
- 不做驱动清理。
- 不做注册表清理。
- 不做抓包。
- 不做防火墙管理。
- 不自动删除外部运行时。
- 不自动修改系统级 PATH。
- 不静默执行高危修复。
- 不替代 uv、pip、npm、pnpm、Yarn、Vite、Maven、Gradle、Docker、WSL、Scoop、Chocolatey、mise、asdf、nvm、rustup 或 dotnet CLI。

## 自动验证

release/v1.5.3-stable 分支本地已通过：

- `cargo test --all-targets`：107 passed，bin targets 0 tests passed
- `cargo clippy --all-targets -- -D warnings`：passed
- `npm ci`：passed，0 vulnerabilities
- `npm run build`：passed
- `npm run tauri:build`：passed，生成 NSIS / MSI bundles
- `py -3 scripts\check_safety_wording.py`：passed
- `py -3 scripts\check_repo_hygiene.py`：passed

## 手动验收记录

发布前按最终开发计划执行 Windows 手动验收，重点覆盖：

- 首次启动安全声明与白屏错误页。
- 端口误判抑制和详情面板。
- 外部 Go / JDK / Python 与包管理器路径不会显示直接删除。
- Python Store Alias 与 chsrc 缺失恢复入口。
- JDK 候选验证和 JAVA_HOME 候选。
- MySQL 正常可用、权限不足、多实例和高危修复门禁。
- 桌面急救、大文件扫描取消、重复文件 quick hash / full hash。
- 导出报告和 commandLine 脱敏。

## Release asset

- 主安装包：`DevEnv.Manager_1.5.3_x64-setup.exe`
- `update-manifest.json` 必须在 GitHub Release asset 上传成功并记录真实 SHA256 后再更新。
