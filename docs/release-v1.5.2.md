# DevEnv Manager v1.5.2

1.5.2 是 Patch Release，重点收口后台安全、构建、CI、manifest 和部分端口/MySQL 后端防护，不新增系统管家式大功能。

端口管理 UI 重做、完整端口识别库、首次启动安全声明、Python/chsrc 恢复闭环、外部运行时安全和扫描体验增强未在 1.5.2 完整落地，计划集中进入 1.5.3 Quality Patch。

## 修复与增强

- 修复 update-manifest 字段兼容问题：支持 `downloadUrl`，兼容 `download_url`，检查阶段即校验 SHA256 和下载白名单。
- 增加后端 confirmation token：绑定 action、plan、risk、fingerprint、过期时间和一次性使用；MySQL 修复与结束进程已接入。
- 强化 MySQL 修复后端：新增部分诊断证据、结论分级、备份 manifest 持久化和系统库修复前 manifest 校验。
- 强化端口识别后端：部分改为 process-first，端口号只作为弱证据，并返回置信度、证据数量、冲突证据、风险和建议字段。
- 增加 rootDir 保存前校验，并统一去掉 Windows `\\?\\` 展示前缀。
- 收紧 Tauri CSP，拒绝远程脚本。
- 新增 GitHub Actions CI 与仓库卫生门禁。

## 验证

- `cargo test --all-targets`
- `cargo clippy --all-targets -- -D warnings`
- `npm run build`
- `python scripts/check_safety_wording.py`
- `python scripts/check_repo_hygiene.py`

## SHA256

- `DevEnv.Manager_1.5.2_x64-setup.exe`: `1244d8888bf1e197fa59131381c4e52a897e94ceec518d743e4fdd9a20224a90`
- `DevEnv.Manager_1.5.2_x64_en-US.msi`: `d6d9f3e24ebd7d2e29c632b037f95db6a3c647c1cae25bc6f9a7c718f62ccf60`

## 发布说明

`update-manifest.json` 已指向 NSIS 安装包。创建 GitHub Release `v1.5.2` 时同时上传 NSIS 与 MSI。
