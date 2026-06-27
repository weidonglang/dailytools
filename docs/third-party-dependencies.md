# 第三方依赖说明

## Rust / Tauri

| 依赖 | 用途 | 运行时核心 | 是否修改源码 | 许可证 |
| --- | --- | --- | --- | --- |
| Tauri | 桌面应用框架、命令桥接、打包 | 是 | 否 | Apache-2.0 / MIT |
| tauri-plugin-dialog | 选择文件夹对话框 | 是 | 否 | Apache-2.0 / MIT |
| serde | 数据结构序列化/反序列化 | 是 | 否 | Apache-2.0 / MIT |
| serde_json | JSON 配置、报告、installed.json | 是 | 否 | Apache-2.0 / MIT |
| reqwest | HTTPS 下载和版本元数据请求 | 是 | 否 | Apache-2.0 / MIT |
| sha2 | SHA256 校验 | 是 | 否 | MIT / Apache-2.0 |
| sysinfo | 进程和系统信息 | 是 | 否 | MIT |
| zip | ZIP 解压 | 是 | 否 | MIT |
| winreg | Windows 用户环境和卸载注册表读取/写入 | 是 | 否 | MIT |
| trash | 普通清理项移入 Windows 回收站 | 是 | 否 | MIT |
| tempfile | 测试与临时解压目录 | 否 | 否 | MIT / Apache-2.0 |
| dirs | Windows 用户目录定位 | 是 | 否 | MIT / Apache-2.0 |

## 前端

| 依赖 | 用途 | 运行时核心 | 是否修改源码 | 许可证 |
| --- | --- | --- | --- | --- |
| @tauri-apps/api | 前端调用 Tauri 命令和事件 | 是 | 否 | Apache-2.0 / MIT |
| @tauri-apps/plugin-dialog | 前端打开选择文件夹对话框 | 是 | 否 | Apache-2.0 / MIT |
| lucide | 图标 | 是 | 否 | ISC |
| Vite | 前端构建工具 | 否 | 否 | MIT |
| TypeScript | 类型检查和编译 | 否 | 否 | Apache-2.0 |

本项目未修改上述依赖源码。
