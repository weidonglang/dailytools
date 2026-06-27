# DevEnv Manager 1.5.1 Final Stable

发布日期：2026-06-27

## 核心改动

- Python 受管安装后完整性检查：pip、venv、ssl、sqlite3、ctypes 为核心检查，tkinter 为可选提示。
- pip 缺失修复计划仅面向 DevEnv Manager 受管 Python。
- 项目启动向导支持选择文件夹，选择后自动校验并可分析项目。
- IDEA / IntelliJ 项目配置只读分析，输出项目 SDK、language level、模块 SDK 和当前 JAVA_HOME 匹配建议。
- JDK/Python/Node/Maven/Gradle/Go 统一运行时强验证报告。
- Nacos/Nexus/Maven/Gradle/Spring Boot/bat 脚本 Java 消费者环境验证。
- 下载、安装、登记、current 指针与环境生效闭环说明补齐。
- 新增软著材料目录 `docs/software-copyright/`。

## 完成 issue

- #49 Python tkinter 组件提示。
- #50 1.5.x 总控稳定版收口。
- #51 Python 安装完整性与 pip 修复闭环。
- #52 项目启动向导和目录输入支持选择文件夹。
- #53 IDEA / IntelliJ 项目配置只读分析。
- #54 运行时强验证与统一状态模型。
- #55 Java 消费者环境验证。
- #56 下载、安装、登记、current 指针与环境生效闭环。

## 测试结果

- `cargo test --all-targets`：已通过。
- `npm run build`：已通过。
- `python scripts/check_safety_wording.py`：已通过。
- `cargo clippy --all-targets -- -D warnings`：已通过。
- `npm run tauri:build`：已通过。

## 构建产物

| 文件 | 大小 | SHA256 |
| --- | ---: | --- |
| `DevEnv.Manager_1.5.1_x64-setup.exe` | 2,498,362 B | `645efdb09c2266b9eafe99e380eab43e0bf41a5f36bf8a017cac9806b3687609` |
| `DevEnv.Manager_1.5.1_x64_en-US.msi` | 4,255,744 B | `80c69073e28fa68c28d67434ddc79089dba8957fba7e6c634cae2604ed38668c` |
| `dailytools-tauri.exe` | 5,863,936 B | `1af3eb84016f70c0733f6c7d6ec6f8bdf31f2854e983e93949b6ae702d55b37a` |
| `devenv.exe` | 2,350,080 B | `f5be783d0f22d1b7e0782ded2b3ce182f941ad546e22ff0a6363e759e89b61ed` |
| `SHA256SUMS.txt` | 370 B | `5a6114534049a30898530dbdece683b1078078d5e9b9d2f4a50fd94e05681103` |

体积变化：`dailytools-tauri.exe` 相对 1.5.0 增加 185,856 B；`devenv.exe` 相对 1.5.0 减少 5,632 B。

## 已知限制

- Nexus 验证依赖用户选择 Nexus 根目录；没有 Nexus 时可使用 mock 根目录验证说明逻辑。
- IDEA `workspace.xml` 不全量读取，避免导出最近文件和私人路径。
- 非受管 Python 只提示问题，不自动修复。

## 软著材料说明

软著材料位于 `docs/software-copyright/`。建议登记软件版本为“DevEnv Manager 开发环境诊断与安全配置管理软件 V1.0”，对应 GitHub Release `v1.5.1`。
