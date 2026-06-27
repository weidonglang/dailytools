# DevEnv Manager 设计说明

## 总体架构

DevEnv Manager 使用 Tauri 2 桌面外壳、Rust 后端和 TypeScript 前端。前端负责交互、风险说明、计划预览和报告展示；后端负责路径校验、运行时安装、环境变量读写、扫描、备份、验证和安全边界。

## 前端结构

- `tauri/src/main.ts`：单页应用入口、状态管理、事件绑定和页面渲染。
- `tauri/src/components/`：风险标签、确认弹窗、安全说明、功能说明卡片。
- `tauri/src/envReliability.ts`：环境可靠性说明。
- `tauri/src/safetyText.ts`：风险等级和安全文案。

前端不直接执行系统修改。所有修改类操作都调用后端命令，并在执行前展示计划、风险和确认。

## 后端结构

- `tauri/src-tauri/src/lib.rs`：Tauri 命令注册、运行时安装、项目分析、端口、工具链和平台能力。
- `cleanup/`：C 盘急救、清理计划、空间搬家、重复文件、应用占用分析。
- `env_core/`：环境快照、PATH 规则、Java/Python/Node/Maven/Gradle 可靠性、计划、应用、验证、回滚和报告。
- `safety/`：风险等级、功能说明、免责声明、确认规则和报告尾部。
- `mysql_repair/`：MySQL 只读诊断和受控修复计划。

## 核心数据结构

- `EnvReliabilitySnapshot`：环境可靠性快照。
- `EnvRepairPlan`：环境变量修复计划。
- `PythonIntegrityReport`：Python 组件完整性报告。
- `RuntimeStrongVerificationReport`：运行时强验证报告。
- `IdeaProjectReport`：IDEA / IntelliJ 项目只读分析。
- `JavaConsumerReport`：Nacos/Nexus/Maven/Gradle/Spring Boot 等 Java 消费者环境验证。
- `CleanupPlan` / `CleanupResult`：清理计划和清理报告。

## 运行时安装验证流程

```text
查询版本元数据 → URL 白名单 → 下载临时文件 → SHA256 校验 → 解压到临时目录
→ 目录结构校验 → 移动正式目录 → 可执行文件检查 → 组件完整性检查
→ 写 installed.json → 更新 current 指针 → 环境生效检查 → 安装报告
```

如果核心组件检查失败，不写入 `installed.json`。已存在目录不会直接当作成功，会重新验证后再决定是否登记。

## 环境变量修复流程

```text
读取进程环境和用户环境 → 生成快照 → 生成一次性计划 → 展示 diff
→ 用户确认 → 写入前备份 → 写用户级变量 → 广播环境变化 → 回读验证 → 报告
```

只修改当前用户级环境变量，不修改系统级环境变量，不删除未知用户 PATH。

## 项目分析流程

项目页支持选择文件夹。选择后执行：

```text
路径规范化 → 是否存在 → 是否目录 → 项目文件识别 → IDEA 只读分析
→ Java 消费者环境验证 → 推荐运行时 → 固定 action id
```

项目配置写入仅限 `.vscode/settings.json`、`.vscode/tasks.json`、`.idea/misc.xml` 和 `.idea/compiler.xml`，写入前备份。

## 风险控制流程

风险等级分为 Info、Low、Medium、High、Critical。中高风险操作需要确认；极高风险操作需要更严格确认。报告会附加风险与限制说明。

## 报告生成流程

诊断、环境修复、清理、空间搬家、数据库修复和扩容检测都会记录生成时间、范围、跳过项、失败原因和建议。报告会脱敏用户目录和常见敏感键值。

## 安全边界

- 不清理浏览器凭据。
- 不读取微信/QQ 聊天数据库。
- 不修改系统级环境变量。
- 不接管 IDE 内置 JDK、Scoop 或 Chocolatey 运行时。
- 不自动移动恢复分区。
- 不执行未知脚本。
