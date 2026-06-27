# GitHub Issues #49-#56 收口记录

发布日期：2026-06-27  
对应版本：DevEnv Manager 1.5.1 Final Stable

## #49 Python 没有安装 Tkinter 图形界面组件

- 新增 Python 完整性检查。
- `pip`、`venv`、`ssl`、`sqlite3`、`ctypes` 作为核心检查。
- `tkinter` 作为可选 GUI 组件提示；缺失时不把 Python 判定为完全不可用，但会提醒 Tkinter/GUI 库可能无法运行。

## #50 1.5.x 稳定版总控

- 1.5.1 停止扩展新系统管家方向，集中收口开发环境诊断、安装完整性、项目向导、环境可靠性和安全说明。
- README、用户手册、测试报告、release checklist 与软著材料同步更新。

## #51 Python 完整性验证与 pip 修复闭环

- 受管 Python 安装后会进行组件完整性检查。
- 核心组件缺失时不写入 `installed.json`。
- pip 修复计划只允许 DevEnv Manager 受管 Python，且必须是当前生效 Python，避免误修系统或第三方 Python。
- CLI `devenv python verify` 同步输出环境一致性与完整性报告。

## #52 项目启动向导和目录输入选择文件夹

- 前端引入 Tauri dialog 插件，项目路径、命令工作目录、大文件/重复文件扫描范围、空间搬家来源与目标等输入支持选择文件夹。
- 选择项目路径后会校验“路径不存在 / 选择了文件 / 未识别项目”的常见错误并给出明确提示。

## #53 IDEA / IntelliJ 项目配置只读分析

- 只读取 `.idea/misc.xml`、`.idea/compiler.xml`、`.idea/modules.xml` 和根目录 `*.iml`。
- 不读取 `workspace.xml` 的私人内容，只给出存在提示。
- 输出项目 SDK、language level、模块 SDK 与当前 JAVA_HOME 的匹配建议。

## #54 运行时强验证与统一状态模型

- JDK/Python/Node/Maven/Gradle/Go 支持统一强验证报告。
- JDK 安装登记前校验 `java.exe`、`javac.exe`、`jar.exe`。
- Python 报告区分登记、组件、current 指针和当前环境生效状态。

## #55 Java 消费者环境验证

- 新增 Nacos/Nexus/Maven/Gradle/Spring Boot/bat/cmd 入口的 Java 环境验证。
- 报告包含用户环境 JAVA_HOME、展开结果、当前进程环境、PATH 首个 java、java/javac 可用性和解释说明。

## #56 下载、安装、登记、current 指针与环境生效闭环

- 安装流程保持下载校验、解压识别、运行时组件验证、登记、切换 current 指针、环境刷新/提示的闭环。
- 组件缺失或核心验证失败时不会写入 `installed.json`。
- 文档补充“新终端/IDE 环境可能与当前进程不同”的解释。

## 验证

- `cargo test --all-targets`：84 个测试通过。
- `cargo clippy --all-targets -- -D warnings`：通过。
- `npm run build`：通过。
- `npm run tauri:build`：通过，生成 NSIS/MSI。
- `python scripts/check_safety_wording.py`：通过。
