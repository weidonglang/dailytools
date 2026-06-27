# DevEnv Manager 1.5.1 Release Checklist

## 自动化检查

- [x] `cargo test --all-targets`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `npm run build`
- [x] `npm run tauri:build`
- [x] `python scripts/check_safety_wording.py`

## 功能检查

- [x] Python pip/venv/ssl/sqlite3/ctypes/tkinter 检查
- [x] pip 缺失修复计划
- [x] pip 与 `python -m pip` 不一致提示
- [x] Store Alias 风险提示
- [x] JDK java/javac/jar 检查
- [x] Maven/Gradle Java 验证
- [x] Nacos/Nexus Java 消费者验证
- [x] 项目文件夹选择
- [x] IDEA 配置只读分析
- [x] 下载/安装/登记/current 指针闭环说明
- [x] 组件缺失不显示“完全可用”

## 手动验证记录

| 场景 | 结果 |
| --- | --- |
| JDK java/javac/jar/JAVA_HOME/PATH | 自动化检查覆盖，JDK 安装登记前校验 java/javac/jar |
| Maven/Gradle 使用当前 JDK | Java 消费者验证覆盖 Maven/Gradle 入口 |
| Nacos Java 环境 | Java 消费者验证覆盖 Nacos 根目录 |
| Nexus Java 环境 | 使用 mock 根目录验证说明路径 |
| Python pip/venv/ssl/sqlite3/ctypes/tkinter | 完整性检查覆盖，tkinter 为可选提示 |
| 环境变量备份恢复 | 自动化与手动流程均覆盖 |
| 危险操作确认 | 保持既有确认体系 |

## 构建产物

| 文件 | 大小 | SHA256 |
| --- | ---: | --- |
| `release-assets-v1.5.1/DevEnv.Manager_1.5.1_x64-setup.exe` | 2,498,362 B | `645efdb09c2266b9eafe99e380eab43e0bf41a5f36bf8a017cac9806b3687609` |
| `release-assets-v1.5.1/DevEnv.Manager_1.5.1_x64_en-US.msi` | 4,255,744 B | `80c69073e28fa68c28d67434ddc79089dba8957fba7e6c634cae2604ed38668c` |
| `release-assets-v1.5.1/dailytools-tauri.exe` | 5,863,936 B | `1af3eb84016f70c0733f6c7d6ec6f8bdf31f2854e983e93949b6ae702d55b37a` |
| `release-assets-v1.5.1/devenv.exe` | 2,350,080 B | `f5be783d0f22d1b7e0782ded2b3ce182f941ad546e22ff0a6363e759e89b61ed` |
| `release-assets-v1.5.1/SHA256SUMS.txt` | 370 B | `5a6114534049a30898530dbdece683b1078078d5e9b9d2f4a50fd94e05681103` |

## Issue 收口

- [x] #49 代码与文档侧已完成，发布后关闭
- [x] #50 代码与文档侧已完成，发布后关闭
- [x] #51 代码与文档侧已完成，发布后关闭
- [x] #52 代码与文档侧已完成，发布后关闭
- [x] #53 代码与文档侧已完成，发布后关闭
- [x] #54 代码与文档侧已完成，发布后关闭
- [x] #55 代码与文档侧已完成，发布后关闭
- [x] #56 代码与文档侧已完成，发布后关闭
