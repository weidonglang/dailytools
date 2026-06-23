# C 盘急救与开发缓存扫描安全架构

1.1.0 的存储能力严格处于 **scan-only** 阶段：只读取目录项元数据、估算大小并生成风险报告，不删除、移动、清空或修改被扫描文件。

## 默认扫描范围

- 用户 `%TEMP%` 和 AppData Local Temp；24 小时内项目受保护。
- Windows Temp、回收站、错误报告、缩略图缓存、DirectX Shader Cache：只统计，永不作为可执行清理项。
- DevEnv Manager 下载、日志和配置；配置只统计。
- npm、pnpm、Yarn、pip、uv、Poetry、Maven、Gradle、Cargo、Go 与 NuGet 的明确缓存路径。
- WPS 明确命名的 cache/temp/log 路径；仅预览。

## 默认排除

- Desktop、Downloads、Documents、Pictures、Videos、Music，不进入扫描。
- Windows、Program Files、ProgramData\Microsoft 等系统关键目录。
- 项目源码、当前工作区、DevEnv Manager 受管运行时和 `current` 指针。
- 浏览器 Cookie、Login Data、密码存储和用户配置。
- 微信、QQ、网盘、聊天数据库、WPS 文档/备份中心/云同步/账号数据。
- 符号链接、junction 和未知来源目录。

## 扫描约束

1. 不跟随符号链接。
2. 单目录设定最大访问条目数，触发上限只报告估算和警告。
3. 权限不足时跳过，不提权、不强制解锁。
4. 每一项包含来源、路径、大小、风险、原因和保护说明。
5. `cleanable` 只表示未来可评估，不代表 1.1.0 会执行删除。

## 后续若启用清理必须满足

- 后端重新扫描并只接受本轮候选 ID。
- 重新校验真实路径、文件类型和保护规则。
- 默认不选高风险类别，并逐项展示影响。
- 优先移入 Windows 回收站，不永久删除。
- 保留本地审计记录，失败只跳过，不强删。

在这些条件具备完整测试和恢复验证前，清理执行接口不会重新开放。
