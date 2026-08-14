# Acceptance Status

更新时间：2026-08-14

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
- [~] 0.3.0 终端快捷指令：默认指令分组、前缀匹配、Tab 插入、变量填写、全局/服务器覆盖、建议隐藏/单 shell 开关和命令历史定位（代码与自动化测试通过，真实 UI/服务器待验收）

## Milestone 2 · Files

- [~] SFTP 浏览、虚拟列表、过滤、历史与快捷键（实现完成，真实登录待验证）
- [~] streaming upload/download、递归传输、进度、取消与冲突（实现完成，真实登录待验证）
- [~] Monaco 编辑、Large File Viewer、mtime/size 冲突、fsync/原子保存与 backup + sudo install、Copy/Move、chmod、symlink（实现完成，真实登录待验证）

## Milestone 3 · Overview / Processes / Ports / Services

- [~] Overview CPU/RAM/disk/load/network 双采样、top processes、mounts、身份/网络/平台/运行摘要与 5 秒刷新（真实登录待验证）
- [x] `ps`、`ss`、`lsof`、systemd fixture 强类型解析测试
- [~] 统一搜索与 `8080` inspect → SIGTERM/SIGKILL → process/port verify；systemd detail/logs/action/enable/disable confirm（实现完成，真实登录待验证）
- [~] 0.3.0 Overview 本地短期历史图表：CPU、内存、网络、磁盘，1h/6h/24h（代码与自动化测试通过，真实采样待验收）

## Milestone 3.5 · Logs / Tasks

- [~] 统一日志中心：system journal、systemd/Nginx、Docker/Compose，包含 follow、暂停、搜索、复制、下载和有界缓冲（代码与自动化测试通过，真实服务器权限/命令待验收）
- [~] 任务持久化：task_records、启动中断标记、手动重试入口、筛选和清理；不得自动重放危险任务（代码与自动化测试通过，真实重启验收待完成）

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
- [~] 0.3.0 Windows x64 MSI/NSIS 已生成并完成 release smoke launch；覆盖升级、安装后启动和卸载仍待在用户环境验证

## 当前真实服务器

- [ ] Host Key 指纹由用户确认
- [ ] 登录与 Overview
- [ ] 文件/Nginx/Docker 能力矩阵
- [ ] 破坏性验收（仅在隔离测试资源上执行）

## 自动化证据（本次接手，2026-08-14）

- `pnpm install --frozen-lockfile`：通过，pnpm 10.28.2，331 packages；仅有 esbuild build script 被 pnpm 忽略提示
- `pnpm lint`：通过
- `pnpm typecheck`：通过
- `pnpm test --run`：通过，6 files / 13 tests（含快捷指令匹配、任务与文件传输回归）
- `pnpm build`：通过，Vite 7.3.6，Tools/Nginx/Docker route chunks 生成
- `cargo fmt --check`：通过（Rust stable 1.97.1 / rustfmt 1.9）
- `cargo check --all-targets --all-features`：通过（MSVC 14.44.35207 + Windows SDK 10.0.26100.0）
- `cargo clippy --all-targets --all-features -- -D warnings`：通过
- `cargo test --all-features`：通过，31 tests / 0 failed
- `pnpm tauri build`：通过，生成 0.3.0 x64 MSI/NSIS；release 使用 NASM 3.02 完整汇编
- release 主程序 smoke launch：通过，进程成功启动并可关闭
- 安装包 SHA-256：见 `docs/CURRENT_STATE.md` 的 Build/packaging 状态（已更新为 0.3.0）
- `git diff --check`：通过（仅有 Git 的 LF→CRLF 提示）
- 测试服务器 TCP/22 与 Host Key 预检成功；本机公钥未被授权；仍未通过产品 UI 使用密码登录，也未执行远端变更。当前执行环境没有可用的桌面 UI 自动化通道，因此不能把密码登录、Host Key 确认和远端写入验收标记为通过。

## 0.3.0 本地实现证据

- 快捷指令：SQLite migration、默认指令分组、全局/服务器覆盖、标签/分组筛选、启用状态、变量提取、大小写不敏感的前缀/分词匹配、最多 6 条建议、Tab 插入和管理入口已接入；建议框支持本次/本 shell 隐藏，单 shell 可切换，命令历史支持定位输出；Enter 仍执行终端命令。
- 日志中心：统一 `LogQuery`，system journal/systemd/Nginx/Docker/Compose 读取与 30 秒 follow、暂停、搜索、复制、下载、重连和有界缓冲已接入；不读取任意远程文件日志。
- 监控历史：Overview 采样成功后写入本地 `metric_samples`，保留 24 小时/每服务器 20,000 条上限，图表展示最多 500 点并支持 1h/6h/24h。
- 任务中心：`task_records` 保存非敏感元数据，启动时把 queued/running 标为 interrupted；中断任务只提供手动返回原模块重试入口，不自动重放危险操作。

## 真实服务器验收阻塞项

真实服务器的 A–K 流程、日志权限差异、Docker/Compose、Overview 历史采样、应用重启后的 interrupted 任务和隔离写入清理仍需用户在 Relay UI 中执行并回填截图/结果。代码层和本地自动化证据已完成，但不能替代真实服务器证据。
