# DevEnv Manager 1.1.0

发布日期：2026-06-23

1.1.0 是一次以安全边界、JDK 可靠性和可理解性为核心的稳定性版本，对公开 Issue #14–#28 进行了集中收敛。

## 核心改进

- 总览实时展示当前实际生效的 Java、Python、Node.js、Maven、Gradle 与 Go；本机完整发现列表默认折叠。
- 通知与任务进度固定在视口右下角，页面滚动后仍可看到结果。
- 环境医生统一“需要关注”口径；可选工具缺失和普通端口占用不再扣分。
- 新增统一 JDK 生效链检查：`JAVA_HOME`、用户 PATH 首个 `java.exe` / `javac.exe`、版本、Maven、Gradle 与来源。
- JDK 切换和用户环境写入后重新验证；修复旧受管 JDK 指针错误抢占有效 `JAVA_HOME` 的情况。
- Java 项目诊断读取 Maven/Gradle 版本线索，明确提示项目 JDK 是否匹配。
- 新增 Nacos 布局识别和固定 action id 的单机启动操作，启动时注入已验证的受管 `JAVA_HOME`。
- 命令面板启用后端白名单，拦截系统 Shell、磁盘、注册表、权限、服务管理与破坏性 Git 命令；安装、更新和发布类命令需要二次确认。
- 新增 AI Agent / CLI 痕迹只读分析。只读取路径、文件名和可验证配置线索，不读取会话正文、shell history、token 或密钥。
- Scoop / Chocolatey 来源在运行时列表明确标注，卸载操作调用对应包管理器，不直接删除其目录。
- 自动更新每天最多检查一次，延迟后台执行；失败只在更新区域显示，不阻塞启动，也不上传遥测。
- C 盘急救默认不进入 Desktop、Downloads、Documents、Pictures、Videos、Music。
- WPS 只匹配明确 cache/temp/log 路径，排除文档、备份中心、云同步和账号数据，并保持 scan-only。

## Issue 对照

- #14：实时版本、固定通知、默认折叠、医生计数、页面使用说明与环境变量写后校验。
- #15：命令白名单、危险类别拦截、管理员态收紧、受管 action id 与 README 风险说明。
- #16：明确项目定位、不替代的成熟工具、适用与不适用人群。
- #17：AI Agent / CLI 路径、全局工具与项目配置线索分析，严格隐私边界。
- #18：明确 Windows 优先、WSL 诊断/编排边界，不承诺简单移植 Linux 桌面版。
- #19：自动更新频率、异步行为、失败降级、无遥测说明和 SHA256 强校验。
- #20：README 维护边界和高风险功能保守默认值。
- #21：JDK 生效来源、一致性验证、切换后复检和环境快照。
- #22：Maven/Gradle 项目 JDK 要求与当前生效环境匹配诊断。
- #23：README 顶部正式下载入口、完整链接与 SHA256 校验示例。
- #24：Scoop / Chocolatey 来源解释与包管理器卸载流程。
- #25：清理能力收敛为明确开发/临时缓存，个人目录默认排除。
- #26：统一 JDK 决策与 JAVA_HOME/PATH/java/javac 一致性检查。
- #27：Nacos 环境识别和固定、已验证的 JAVA_HOME 启动链。
- #28：WPS 明确缓存、临时、日志目录只读扫描，排除用户内容。

## 安全说明

- 不会把 AI 输出自动交给系统 Shell。
- 不修改系统级环境变量。
- C 盘急救仍不删除文件。
- Agent 痕迹分析不上传数据，也不读取历史命令正文。
- 安装包、MSI 与 CLI 均在 Release 附带 `SHA256SUMS.txt`。

## 发布文件

- `DevEnv.Manager_1.1.0_x64-setup.exe`：推荐的 NSIS 安装包。
- `DevEnv.Manager_1.1.0_x64_en-US.msi`：MSI 安装包。
- `devenv.exe`：命令行工具。
- `SHA256SUMS.txt`：发布文件校验和。
