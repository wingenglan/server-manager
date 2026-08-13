# Acceptance Status

更新时间：2026-08-13

标记：`[x]` 已由自动化或真实环境验证；`[ ]` 尚未验证；`[~]` 正在实现。没有证据的功能不得标记完成。

## Milestone 0

- [x] Tauri 2 + React + TypeScript 项目骨架
- [x] React Router、TanStack Query、Zustand 与设计 token
- [x] Rust typed IPC 与 AppError
- [x] SQLite migrations 与 repository
- [x] OS Keychain abstraction
- [x] tracing 与 secret redaction
- [x] Rust fmt、clippy、test 已通过；frontend lint/typecheck/test/build 全通过
- [x] 空服务器页和设置页通过前端 production build

## Milestone 1 · Server + SSH

- [~] 添加/编辑/删除服务器档案（password/private key；SSH Agent 为 P1）
- [~] 严格 Host Key trust/change detection（实现完成，真实 UI 确认待验证）
- [~] 复用 SSH session、connect/disconnect 与断线检测
- [~] 真实 PTY、多终端、resize、搜索、断线状态（实现完成，真实登录待验证）
- [ ] Ubuntu 密码认证真实验证
- [ ] Rocky/Alma 私钥认证真实验证

## Milestone 2 · Files

- [~] SFTP 浏览、虚拟列表、过滤、历史与快捷键（实现完成，真实登录待验证）
- [~] streaming upload/download、递归传输、进度、取消与冲突（实现完成，真实登录待验证）
- [~] Monaco 编辑、Large File Viewer、mtime/size 冲突、fsync/原子保存与 backup + sudo install、Copy/Move、chmod、symlink（实现完成，真实登录待验证）

## Milestone 3 · Overview / Processes / Ports / Services

- [~] Overview CPU/RAM/disk/load/network 双采样、top processes、mounts、身份/网络/平台/运行摘要与 5 秒刷新（真实登录待验证）
- [x] `ps`、`ss`、`lsof`、systemd fixture 强类型解析测试
- [~] 统一搜索与 `8080` inspect → SIGTERM/SIGKILL → process/port verify；systemd detail/logs/action/enable/disable confirm（实现完成，真实登录待验证）

## Milestone 4 · Tools + Nginx

- [~] 工具能力探测、PlatformAdapter、安装计划、streaming 安装与安装后验证（真实环境未验证）
- [~] Nginx AST/source mapping、include 源文件聚合、反向代理列表、证书到期元数据、后端探活（fixture 已覆盖，真实 `nginx -T` 未验证）
- [~] Wizard 与 backup/test/rollback/reload、HTTPS 证书存在性检查（受控 `/etc/nginx/conf.d` include 检查已实现，真实配置未验证）

## Milestone 5-6 · Docker

- [~] Overview、containers、健康/创建/Compose/资源摘要、logs（search/pause/clear/copy/download/tail）、inspect、stats 短期采样、top、受控 exec、30 秒有时限 follow logs、restart policy/CPU/memory（真实环境未验证）
- [~] Images/pull/run/remove、volumes/networks inspect/create/delete（pull 已 streaming；真实环境未验证）
- [~] Compose discovery、services/logs、默认脱敏 config、up/start/stop/restart/pull/build/down、原始 YAML 显式编辑校验/失败恢复和逐项 cleanup 已接入（真实环境未验证）

## Milestone 7 · Backup / Polish

- [~] 普通服务器配置导出/导入与 Argon2id + AES-256-GCM 完整备份（代码和错误路径已实现，Rust 运行时/真实 UI 未验证）
- [~] 全局任务中心、审计记录、脱敏诊断导出、档案复制、跨页面 toast、locale 偏好结构和更新预留入口（真实运行时未验证）；完整翻译资源、真实更新通道、快捷键、性能与错误体验仍待补
- [x] Windows 安装包（MSI + NSIS，x64）
- [ ] macOS 安装包
- [ ] 主需求 A-K 全量手工验收

## 当前真实服务器

- [ ] Host Key 指纹由用户确认
- [ ] 登录与 Overview
- [ ] 文件/Nginx/Docker 能力矩阵
- [ ] 破坏性验收（仅在隔离测试资源上执行）

## 自动化证据（本次接手，2026-08-13）

- `pnpm install --frozen-lockfile`：通过，pnpm 10.28.2，331 packages；仅有 esbuild build script 被 pnpm 忽略提示
- `pnpm lint`：通过
- `pnpm typecheck`：通过
- `pnpm test --run`：通过，4 files / 7 tests
- `pnpm build`：通过，Vite 7.3.6，Tools/Nginx/Docker route chunks 生成
- `cargo fmt --check`：通过（Rust stable 1.97.1 / rustfmt 1.9）
- `cargo check --all-targets --all-features`：通过（MSVC 14.44.35207 + Windows SDK 10.0.26100.0）
- `cargo clippy --all-targets --all-features -- -D warnings`：通过
- `cargo test --all-features`：通过，28 tests / 0 failed
- `pnpm tauri build`：通过，生成 x64 MSI/NSIS；release 使用 NASM 3.02 完整汇编
- release 主程序 smoke launch：通过，进程成功启动并可关闭
- 安装包 SHA-256：见 `docs/CURRENT_STATE.md` 的 Build/packaging 状态
- `git diff --check`：通过（仅有 Git 的 LF→CRLF 提示）
- 测试服务器 TCP/22 与 Host Key 预检成功；本机公钥未被授权；仍未通过产品 UI 使用密码登录，也未执行远端变更
