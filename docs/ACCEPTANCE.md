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
- [x] Rust fmt/clippy/6 tests 与 frontend lint/typecheck/5 tests/build 全通过
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
- [~] Monaco 编辑、mtime/size 冲突、fsync/原子保存与 backup + sudo install（实现完成，真实登录待验证）

## Milestone 3 · Overview / Processes / Ports / Services

- [~] Overview CPU/RAM/disk/load/network 双采样、身份/网络/平台/运行摘要与 5 秒刷新（真实登录待验证）
- [x] `ps`、`ss`、systemd fixture 强类型解析测试
- [~] 统一搜索与 `8080` inspect → SIGTERM/SIGKILL → process/port verify（实现完成，真实登录待验证）

## Milestone 4 · Tools + Nginx

- [ ] 工具能力探测、安装计划与 streaming 安装
- [ ] Nginx AST/source mapping、反向代理列表
- [ ] Wizard 与 backup/test/rollback/reload

## Milestone 5-6 · Docker

- [ ] Overview、containers、logs、inspect、stats、exec
- [ ] Images/pull/run、volumes、networks
- [ ] Compose discovery/config/apply 与 cleanup

## Milestone 7 · Backup / Polish

- [ ] 普通导出与 Argon2id + AEAD 完整备份
- [ ] 诊断导出、快捷键、性能与错误体验
- [ ] Windows/macOS 安装包
- [ ] 主需求 A-K 全量手工验收

## 当前真实服务器

- [ ] Host Key 指纹由用户确认
- [ ] 登录与 Overview
- [ ] 文件/Nginx/Docker 能力矩阵
- [ ] 破坏性验收（仅在隔离测试资源上执行）

## 自动化证据（2026-08-13）

- `cargo fmt --check`：通过
- `cargo clippy --all-targets --all-features -- -D warnings`：通过
- `cargo test --all-features`：14 passed
- `pnpm lint && pnpm typecheck`：通过
- `pnpm test --run`：7 passed
- `pnpm build`：通过，Files/Terminal/Operations 按路由拆包
- `pnpm tauri dev`：Windows 桌面进程与 WebView 连接成功
- 测试服务器 TCP/22 与 Host Key 预检成功；本机公钥未被授权，未尝试把密码放入 CLI
