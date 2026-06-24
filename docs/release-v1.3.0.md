# DevEnv Manager 1.3.0

发布日期：2026-06-24

1.3.0 集中完成公开 Issue #30–#34，并连续交付 C 盘急救大师 Phase 2/3。这个版本一方面把清理限制在可解释、可确认、可重新验证和可报告的安全闭环内，另一方面为桌面、下载、大文件、重复文件和常见应用增加严格只读的占用分析。

## C 盘急救 Phase 2

- 固定执行链：扫描、选择、计划预览、二次确认、重新扫描校验、清理、验证、报告。
- 清理计划保存在后端、30 分钟过期且只能执行一次；客户端篡改路径、分类、风险或大小会被拒绝。
- 用户 Temp、LocalAppData Temp 中超过 24 小时的普通项目可加入计划。
- DevEnv Manager 下载缓存和超过 24 小时的旧日志可加入计划。
- 普通文件与目录优先移入 Windows 回收站。
- Windows Temp、回收站、Windows Update、Windows.old、系统文件、个人目录、浏览器、微信/QQ 和 WPS 保持只读。
- Maven、Gradle、Cargo registry/cache 和 Cargo target 保持只读。
- npm、pnpm、Yarn、pip、uv、Poetry、Go 与 NuGet 使用官方命令清理，不直接删除缓存目录。
- 清理结果支持复制 Markdown 和导出 JSON。

## Issue #30–#34

- #30：侧边栏按诊断、环境与运行时、项目与生态、维护与系统分组；平台页新增官方 RubyMetric/chsrc 固定目标入口，支持查看、列出、测速、自动选择、指定源 ID 和恢复官方源。
- #31：新增独立的 `docs/user-guide.md`，覆盖安装校验、逐页操作、安全边界、项目配置、chsrc、Phase 2、恢复和故障排查。
- #32：独立列表超过 5 项统一分页；端口表格每页 10 项，显示页码与总数。
- #33：补充 Go 与 BellSoft 官方 CDN，逐跳验证 HTTPS 重定向；兼容 Liberica 数组响应，并补充下载域名测试。
- #34：项目页新增 VS Code/IDEA 配置预览和细微编辑；只允许四个固定配置文件，已有文件自动备份；运行时切换前自动保存时间命名的环境模板，失败时尝试恢复。

## C 盘急救 Phase 3

- 桌面急救：分类大文件、快捷方式、安装包、压缩包、截图、旧文件与重复候选，只生成整理建议。
- 下载目录：分类安装包、压缩包、视频、图片、文档、ISO/磁盘镜像、30 天旧文件和 1GB 大文件。
- 大文件：用户指定扫描范围和最小体积，返回 Top 100，支持打开所在目录和复制路径。
- 重复文件：先按大小分组，仅对候选计算 SHA256；结果只展示，不提供删除。
- 微信/QQ：只统计已知数据根目录的元数据，不读取或列出聊天数据库内容。
- 浏览器：只统计明确 Cache、Code Cache、GPUCache、Firefox cache2/startupCache，不接触 Cookie、密码库和登录态。
- 网盘、剪辑软件与游戏平台：展示常见路径占用与迁移建议，不直接移动或删除。
- 已安装软件：读取 Windows 卸载表的名称、发布者、安装位置、登记大小和卸载入口；卸载统一跳转 Windows“已安装的应用”。

## 环境配置增强

- 配置前展示 DEVENV_HOME、JAVA_HOME、PATH 条目数量、新增与移除差异。
- 预览有效期 10 分钟且只能应用一次，避免旧页面重复写入。
- 写入前保存最新备份和最多 20 份历史备份。
- 支持查看并恢复指定历史备份；恢复前再次保存当前状态。
- PATH 清理和 JDK 切换也进入统一备份历史。

## 安全说明

- 下载白名单仍是精确域名，不允许任意子域或关闭校验。
- chsrc 不接受自定义 URL，只接受固定目标和源 ID。
- 项目配置单文件最大 64 KB，拒绝符号链接与目录穿越。
- CLI 的 `cleanup scan` 继续保持只读；清理必须在 GUI 中完成预览和确认。

## 验证

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --all-targets`
- `npm run build`
- `npm run tauri:build`
- C 盘急救计划、保护路径、失败记录、下载缓存和缺失官方命令单元测试
- Release 程序启动与界面流程检查
- Rust 单元测试 55 项全部通过，Clippy 以 `-D warnings` 通过
- `dailytools-tauri.exe` 为 5,047,808 B，相对最近正式版 1.1.0 增加 221,184 B；Phase 2/3 合并增量远低于 12 MB

## 下载

- `DevEnv.Manager_1.3.0_x64-setup.exe`：推荐 NSIS 安装包。
- `DevEnv.Manager_1.3.0_x64_en-US.msi`：MSI 安装包。
- `devenv.exe`：命令行工具。
- `SHA256SUMS.txt`：发布资产 SHA256。
