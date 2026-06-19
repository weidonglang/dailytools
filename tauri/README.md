# DevEnv Manager Tauri

这是 `dailytools` 的 Tauri + Rust 重构预览版。当前目标不是替换原 Python 版本，而是先验证轻量打包、Rust 后端命令桥接和核心工具体验。

## 当前能力

- Tauri 2 + Rust 桌面外壳。
- TypeScript 前端仪表盘。
- Windows 端口扫描 MVP。
- 安全结束端口进程，拦截 PID 0、PID 4 和关键系统进程。
- 系统 PATH、`JAVA_HOME`、`DEVENV_HOME` 快照。
- 用户环境变量配置、备份和恢复。
- Java、Python、Node.js、npm、Maven、Gradle 运行时发现。
- JDK、Python、Node.js、Maven、Gradle 下载、安装、记录、切换和卸载。
- 项目健康检查 MVP。
- 网络诊断和代理状态快照。
- 下载缓存列表、SHA-256 可选计算和清理。
- 命令面板，可在指定工作目录执行常用命令。
- VS Code `settings.json` / `tasks.json` 生成。
- 安装任务进度事件反馈。
- MSI 和 NSIS 安装包构建。

## 开发

```powershell
npm install
npm run tauri:dev
```

## 构建

```powershell
npm run tauri:build
```

产物位置：

- `src-tauri\target\release\dailytools-tauri.exe`
- `src-tauri\target\release\bundle\msi\DevEnv Manager_0.1.0_x64_en-US.msi`
- `src-tauri\target\release\bundle\nsis\DevEnv Manager_0.1.0_x64-setup.exe`

## 下一步迁移

1. 把端口扫描从 `netstat` 替换为原生 Windows API。
2. 为长耗时安装任务增加取消能力和更细粒度下载进度。
3. 补齐 Python 安装器卸载流程和已有 Python 复制模式。
4. 迁移项目绑定、本地服务、pip/npm 源切换和虚拟环境管理。
5. 拆分 Rust 后端模块并补齐单元测试。
