# Current implementation state

最后更新：2026-08-13。本文记录代码事实和验证证据；产品验收状态以 [`ACCEPTANCE.md`](ACCEPTANCE.md) 为准。

## 仓库与技术栈

- 根目录：`agentless-server-manager`
- Tauri 2 + Rust stable + Tokio
- React 19 + TypeScript + Vite + React Router
- TanStack Query、Zustand、Radix、Tailwind CSS 4
- xterm.js、Monaco、TanStack Virtual；ECharts 已安装但图表尚未接入
- russh 0.62.6 + russh-sftp 2.4.0
- sqlx SQLite migrations + keyring OS credential store
- pnpm 与 Cargo lockfile 已提交

## 已完成的代码

### Milestone 0 — 项目骨架

- Tauri/React/TypeScript 项目、routing、工业控制台 design tokens、设置/空状态页面。
- 白名单 typed Tauri IPC、结构化 `AppError`、CSP 与最小 capability。
- SQLite migration：servers/groups/tags/known_hosts/recent_paths/settings/audit_events。
- OS Keychain abstraction；SQLite 只保存 secret reference。
- tracing 初始化、基础 secret redaction、POSIX shell argument escaping。
- CI：Windows + macOS 运行前端/Rust lint、tests 与 build。
- `pnpm tauri dev` 已实际启动 Windows 桌面进程及 WebView。

### Milestone 1 — Server + SSH（实现，真实验收未闭环）

后端主要位置：

- `src-tauri/src/domain/server/mod.rs`
- `src-tauri/src/infra/db/mod.rs`
- `src-tauri/src/domain/ssh/mod.rs`
- `src-tauri/src/security/mod.rs`

已有能力：

- 添加、编辑、删除 server profile；password/private key/private-key passphrase；sudo 模式；标签与收藏。
- 编辑时留空 secret 保留旧值；切换认证/sudo 模式会删除废弃 Keychain secret。
- Host Key unknown challenge、5 分钟过期、完整内容匹配、known host 保存、changed key 阻断。
- 每服务器复用一个 SSH session，短命令并发上限 8，connect/command timeout、keepalive、disconnect。
- snapshot 会通过 russh `is_closed()` 清除已关闭连接。
- password/private key auth；SSH Agent 尚未实现（P1）。
- 真实 PTY、raw byte Tauri Channel、resize、控制字符、UTF-8、ANSI、断开状态。
- 多终端 tabs、新建/关闭/重命名、搜索、清屏、字号、滚动缓冲、copy/paste、多行粘贴提醒。

已知边界：

- 未用 Relay UI 在真实服务器完成 password auth 与终端验收。
- reconnect 需要回 Overview 建立 SSH，再重开 tab；自动退避重连尚未实现。
- Server group UI、最近连接视图、profile duplicate/export、SSH Agent/ProxyJump 尚未实现。
- Sidebar 在线点目前主要表示选中态，不是所有服务器的实时后台探测结果。

### Milestone 2 — Files（主要实现，真实验收未闭环）

后端主要位置：

- `src-tauri/src/domain/files/mod.rs`
- `src-tauri/src/domain/transfer/mod.rs`

前端主要位置：

- `src/features/files/FilesPage.tsx`
- `src/features/files/TransferCenter.tsx`
- `src/features/files/transferStore.ts`

已有能力：

- 每次文件操作在现有 SSH session 上创建真实 SFTP subsystem channel。
- canonical path、目录列表、stat/lstat、create、rename、file/empty-dir delete、递归 delete。
- `/`、`/etc`、`/usr`、`/var` 等系统根本身禁止普通 UI 递归删除。
- 虚拟化详情列表、排序/过滤、隐藏文件、Back/Forward/Up、路径输入、多选、F2/Delete/Ctrl+A、右键菜单。
- <=10 MB UTF-8 文件用 Monaco 打开；dirty state、Ctrl+S、mtime+size conflict、force overwrite。
- 普通保存：同目录临时文件 → flush/fsync → preserve mode → atomic `mv` → 重读。
- sudo 保存：`/tmp` 临时文件 → 原文件 backup → `sudo install` → 清理 → 重读内容验证；sudo password 只走 SSH stdin。
- 本地拖入上传、文件选择上传、递归目录、streaming 128 KiB buffer、`.part`、速度/进度、cancel、传输中心。
- 下载文件/目录到用户选择的本地目录，streaming + local `.part` + rename。

已知边界：

- 未在真实 SFTP server 验证浏览、传输、冲突、sudo 保存。
- 上传选择器目前选文件；文件夹主要依赖拖放。需要增加明确的“选择文件夹上传”。
- 同名冲突 Skip/Replace/Rename/Apply-to-all 尚未做；当前上传完成时 `mv -f`。
- 远程拖出到任意系统目标未做，仅 Download；远端内部 Copy/Move UI 未做。
- 大文件 viewer、图片预览、系统默认应用安全临时目录、chmod/symlink UI、Compare/Reload UI 未完成。
- transfer retry/pause、全局任务持久化和关闭应用后的恢复未完成；取消已实现。

### Milestone 3 — Overview / Processes / Ports / Services（主要实现，真实验收未闭环）

后端主要位置：

- `src-tauri/src/domain/metrics/mod.rs`
- `src-tauri/src/domain/operations/mod.rs`

前端主要位置：

- `src/features/overview/OverviewPage.tsx`
- `src/features/operations/OperationsPage.tsx`

已有能力：

- Overview 通过固定 Rust domain probe 读取 `/etc/os-release`、`/proc`、`df`、`ip`、systemctl、Docker/Nginx capability。
- CPU 和 network 使用两个采样点计算；network 排除 loopback，不把累计字节伪装成速率。
- hostname/OS/kernel/arch/user/remote time/timezone/IP/gateway/package manager/systemd、CPU/RAM/swap/load/disk、Docker/Nginx、failed units/listening count。
- `ps -eo ...`、`ss -H -lntup`、`systemctl list-units` 强类型 parser 与 fixtures。
- 页面统一搜索 port/PID/user/process/command/service，并按 ports/processes/services 分组展示。
- 释放端口/结束进程：先展示对象，默认 SIGTERM，可显式选择 SIGKILL 或已配置 sudo；之后 `kill -0` + `ss` 验证。
- systemd start/stop/restart 走显式 privileged runner。

已知边界：

- 未在真实服务器验收实时 Overview 或 8080 release 流程。
- Overview 未显示 top CPU/memory processes、完整 mount 列表、virtualization、短期图表。
- operations 进程表暂截前 300 行，尚未虚拟化；服务日志/detail/enable/disable 尚未做。
- 非 root `ss` fallback、sudo rescan 与 `lsof` fallback 未完成。

## 尚未实现的主要范围

### Milestone 4 — Tools + Nginx

尚未实现：tool registry/detection/install plan、streaming install、capability refresh；Nginx `-T` source mapping/parser、reverse proxy list/wizard、backup/test/rollback/reload、日志与证书 metadata。

### Milestone 5–6 — Docker

尚未实现：Docker overview/containers/actions/logs/inspect/stats/exec；images/pull/run；volumes/networks；Compose discovery/config/apply；cleanup/builds/events。

### Milestone 7 — Backup + Polish

尚未实现：普通导出、Argon2id + AEAD credential backup/import；settings persistence、theme/light/i18n、完整 command palette、notifications/toasts/task center、diagnostics export、audit writes、update reservation、full packaging。

## 自动化与手工证据

最后一次全量通过（2026-08-13）：

```text
pnpm lint                         PASS
pnpm typecheck                    PASS
pnpm test --run                   PASS (7 tests)
pnpm build                        PASS (route chunks generated)
cargo fmt --all -- --check        PASS
cargo clippy ... -- -D warnings   PASS
cargo test --all-features         PASS (14 tests)
```

前端 route chunks：main 约 376 KB、Files 约 120 KB、Terminal 约 369 KB、Operations 约 9 KB（未压缩，具体 hash 会变化）。

截图：`docs/screenshots/milestone-0-*.png`，是空服务器/add dialog/compact QA，不代表后续功能真实验收。

真实服务器证据：TCP/22、SSH banner、ed25519 fingerprint 已预检；本机 public key 未获授权。没有通过产品 UI 使用密码登录，没有对远端执行变更。

## Build/packaging 状态

- `pnpm tauri dev`：成功启动过 Windows desktop process + WebView。
- `pnpm tauri build`：最近一次在 release 优化阶段被用户主动暂停；没有 `src-tauri/target/release/bundle/` 或可交付安装包。
- 新环境必须重跑 build。完成后将 bundle 路径和 SHA-256 写入本文与 `ACCEPTANCE.md`。

## Git 检查点

- `e918cef feat: build secure SSH operations foundation`
- `17c32e6 feat: add files transfers and runtime operations`
- 本次交接文档会再生成一个 commit；以远程 `main` 最新 log 为准。

## 需优先审计的技术债

1. `AppError` 使用 crate-level `clippy::result_large_err` 豁免；这是为了 IPC 结构化错误，后续可评估 Box，但不要无理由破坏序列化形状。
2. `security::redact` 是基础 marker redaction，尚非完整 secret-aware tracing layer。
3. Remote command abstraction 仍以固定 domain-generated command string 为核心，未形成主需求完整 `RemoteCommandRequest`、stream/cancel 类型。
4. `audit_events` 表已建但业务动作尚未写审计记录。
5. Tauri capability 为 core + dialog open/save；每加插件都需最小 scope 审计。
6. CI 声明 Windows/macOS，但尚未在 GitHub Actions 远程实际观察结果；push 后立即查看首轮 CI。
