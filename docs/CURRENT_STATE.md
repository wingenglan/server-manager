# Current implementation state

最后更新：2026-08-14。本文记录代码事实和验证证据；产品验收状态以 [`ACCEPTANCE.md`](ACCEPTANCE.md) 为准。0.3.0 新增能力的代码和本地自动化已完成，真实服务器 UI 验收仍保持未完成。

## 仓库与技术栈

- 根目录：`agentless-server-manager`
- Tauri 2 + Rust stable + Tokio
- React 19 + TypeScript + Vite + React Router
- TanStack Query、Zustand、Radix、Tailwind CSS 4
- xterm.js、Monaco、TanStack Virtual、ECharts；终端快捷指令、统一日志流和 Overview 短期历史图表已接入。
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
- 0.3.0 快捷指令已接入：全局默认 + 单服务器覆盖、CRUD、标签、启用状态、`{{变量}}` 参数、前缀/分词匹配、最多 6 条建议、上下键/Tab/Esc 和管理入口；真实服务器验证待完成。

已知边界：

- 未用 Relay UI 在真实服务器完成 password auth 与终端验收。
- Overview 的连接失败状态支持有限退避重连；终端在 SSH 断开后仍需用户点击“重开会话”，后台自动恢复终端尚未实现。
- Server 分组、侧栏搜索、favorite/recent 排序、分组编辑和 profile duplicate 已接入；SSH Agent/ProxyJump 尚未实现；普通配置与加密完整备份已在设置页提供。
- Sidebar 会每 15 秒读取本地 SSH 会话快照并显示 online/connecting/error/offline；它不会为所有服务器后台自动发起 SSH 连接。

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
- 已增加明确的“上传文件夹”选择器，并复用 Rust 传输层递归上传。
- 同名冲突 Skip/Replace/Rename 已接入上传批次策略；当前选择会应用于该批次。
- 远程拖出到任意系统目标未做，仅 Download；远端内部 Copy/Move、chmod、symlink 已接入并由后端验证。
- 大文件 viewer 已支持通过远端 `tail` 展示有限尾部；图片预览、文件夹预览、书签和编辑器 Compare/Reload 已接入；系统默认应用安全临时目录仍未作为默认打开方式。
- 传输中心已支持失败/取消任务重新提交；文件传输的进程内传输状态仍由 `transferStore` 管理，0.3.0 的 `task_records` 持久化已覆盖远程命令/流式任务元数据。

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
- hostname/OS/kernel/arch/user/remote time/timezone/IP/gateway/package manager/systemd、CPU/RAM/swap/load/disk、Docker/Nginx、failed units/listening count；Overview runtime 卡片可在确认后启动/重启 Docker/Nginx 服务。
- `ps -eo ...`、`ss -H -lntup`、`systemctl list-units` 强类型 parser 与 fixtures。
- 页面统一搜索 port/PID/user/process/command/service，并按 ports/processes/services 分组展示。
- 释放端口/结束进程：先展示对象，默认 SIGTERM，可显式选择 SIGKILL 或已配置 sudo；之后 `kill -0` + `ss` 验证。
- systemd start/stop/restart 走显式 privileged runner。

已知边界：

- 未在真实服务器验收实时 Overview 或 8080 release 流程。
- Overview 已显示 top CPU/memory processes 和 mount 摘要；本地 metric_samples 历史、CPU/内存/网络/磁盘图表及 1h/6h/24h 切换已接入，真实采样待验收。
- operations 进程表暂截前 300 行，尚未虚拟化；systemd 服务 detail/logs/action/enable/disable 已接入，权限 fallback 尚未做。
- 运行现场端口探测优先 `ss`，缺失时回退 `lsof`；页面可在明确勾选后以已配置 sudo 重扫。Overview 现在额外返回 OS Adapter、真实 command paths 和防火墙能力。

## 尚未实现的主要范围

### Milestone 4 — Tools + Nginx（代码已接入，真实验收未闭环）

- `src-tauri/src/domain/tools/mod.rs`：固定工具 registry，探测 installed/version/running/package manager，生成 apt/dnf 安装计划；安装由用户确认后执行并重新验证。
- `src-tauri/src/domain/nginx/mod.rs`：解析 `nginx -T` 的 source marker、server/location/upstream/proxy_pass/listen/server_name，保留未知 directive warning 与源文件/行号；managed conf 仅在检测到 `/etc/nginx/conf.d/` include 时启用，写入走临时文件、备份、`nginx -t`、reload 和失败恢复。
- 前端 `src/features/tools/ToolsPage.tsx`、`src/features/nginx/NginxPage.tsx` 已接入 typed IPC、loading/error/empty、确认和配置风险提示。
- 当前缺口：工具安装已通过独立 SSH channel 流式回传 stdout/stderr，并支持取消远程 channel；Nginx HTTPS wizard/证书存在性、证书有效期元数据、后端连通性测试、`nginx -T` 源文件聚合已实现；配置文件可跳转到 Files/Monaco 编辑，完整 directive tree 和真实服务器验证仍未完成；TLS metadata 不读取私钥内容。

### Milestone 5–6 — Docker（部分代码已接入，真实验收未闭环）

- `src-tauri/src/domain/docker/mod.rs`：CLI-over-SSH 的 Engine/container/image/volume/network JSON parser；容器生命周期动作、删除确认、inspect 状态验证、tail 日志、inspect/stats/top 只读查询、pull streaming 和受控 run。
- 前端 `src/features/docker/DockerPage.tsx` 已提供容器/镜像列表、筛选、健康/创建时间/Compose/资源限制摘要、确认后的生命周期操作、重命名/复制 ID 与名称/打开发布端口、日志筛选/刷新/复制/下载/清空视图/tail、格式化 inspect/stats/top 搜索复制、Stats 短期 session 采样、pull/run 和 volume/network Inspect/create/delete。
- Compose 原始 YAML 可从项目 config path 显式读取并编辑；保存前执行 `docker compose config -q`，失败自动恢复原文件；默认渲染配置仍脱敏只读；项目支持 up/start/stop/restart/pull/build/down。统一任务中心已覆盖文件传输、工具安装、Docker pull/follow，容器 snapshot 已补 restart policy/CPU limit/memory limit 字段；真实服务器验证仍未完成。

### Milestone 7 — Backup + Polish

- 已实现普通 JSON 导出/导入：`src-tauri/src/domain/server/mod.rs`、`src/features/settings/SettingsPage.tsx`；只导出非敏感配置，导入生成新 ID，密码/私钥内容/sudo 凭据不出 Keychain。
- 已补 locale 偏好和文档 `lang` 属性结构、跨页面通知 toast、Tauri app log（每次启动写入 app log 目录的 `relay.log`）、更新能力预留入口；完整翻译资源、真实更新通道和 macOS 包仍未完成，Windows 0.3.0 MSI/NSIS 已生成。设置页已补 About/version；统一任务中心已覆盖文件传输、工具安装、Docker pull/follow 和 Compose/资源动作元数据；脱敏 diagnostics export、audit writes、档案 duplicate、Compose YAML 安全写回、Argon2id + AES-256-GCM 完整备份与主题/连接恢复偏好已接入。

### 0.3.0 已实现能力（真实验收待完成）

- 终端：快捷指令 registry、全局/服务器覆盖、默认命令、前缀/分词匹配、标签筛选、变量填写、Tab 插入和管理入口；快捷指令只插入，不自动执行。
- 日志：system journal、systemd/Nginx、Docker/Compose 的统一查询、30 秒 follow、暂停、搜索、复制、下载、断线重连和有界缓冲；远程日志内容不落本地 SQLite。
- 监控：本地 `metric_samples` 滚动历史、24 小时/每服务器 20,000 条限制、1h/6h/24h 趋势图和最多 500 点降采样。
- 任务：`task_records` 持久化、启动恢复、interrupted 状态、状态/服务器筛选、手动返回原模块重试入口和清理；不自动重放危险任务。

## 自动化与手工证据

本次接手后的可复现结果（2026-08-14）：

```text
pnpm install --frozen-lockfile    PASS (pnpm 10.28.2; 331 packages)
pnpm lint                         PASS
pnpm typecheck                    PASS
pnpm test --run                   PASS (6 files, 13 tests)
pnpm build                        PASS (Vite 7.3.6)
cargo fmt --check                 PASS (Rust stable 1.97.1 / rustfmt 1.9)
cargo check --all-targets ...     PASS (MSVC 14.44.35207 + Windows SDK 10.0.26100.0)
cargo clippy --all-targets ...    PASS (-D warnings)
cargo test --all-features        PASS (31 tests, 0 failed; NASM 3.02)
pnpm tauri build                  PASS (0.3.0 x64 MSI + NSIS)
git diff --check                  PASS
```

前端 production bundle 已由本次变更重新生成；Rust 已在 MSVC/Windows SDK/NASM 完整环境中通过 check、clippy、test，Tauri release 安装包也已生成。

截图：`docs/screenshots/milestone-0-*.png`，是空服务器/add dialog/compact QA，不代表后续功能真实验收。

真实服务器证据：TCP/22、SSH banner、ed25519 fingerprint 已预检；本机 public key 未获授权。没有通过产品 UI 使用密码登录，没有对远端执行变更。当前执行环境没有桌面 UI 自动化通道，真实服务器验收必须由用户在 Relay UI 中完成。

## Build/packaging 状态

- 构建环境：Visual Studio Build Tools 2022 17.14.37，MSVC 14.44.35207，Windows SDK 10.0.26100.0，NASM 3.02，Rust 1.97.1，pnpm 10.28.2，Node 23.11.1。
- `pnpm tauri build`：通过；主程序 `src-tauri/target/release/agentless-server-manager.exe` 已成功生成并 smoke launch。
- NSIS 安装包：[agentless-server-manager_0.3.0_x64-setup.exe](../src-tauri/target/release/bundle/nsis/agentless-server-manager_0.3.0_x64-setup.exe)，9,572,734 bytes（9.13 MiB），SHA-256 `BC69CB7079582812031A4E0782CC9CF55148D420FBB108CD7F22F7201FD12CC1`。
- MSI 安装包：[agentless-server-manager_0.3.0_x64_en-US.msi](../src-tauri/target/release/bundle/msi/agentless-server-manager_0.3.0_x64_en-US.msi)，12,402,688 bytes（11.83 MiB），SHA-256 `9CBB55AAB73B298382A6BC65EE7CE3ADC0DF77C66CAE816D08E64DBD9078270F`。
- release 主程序：[agentless-server-manager.exe](../src-tauri/target/release/agentless-server-manager.exe)，28,370,944 bytes（27.06 MiB），SHA-256 `DDDBA7F3E0493DA6792C499EBB9E1DC76C99D273D519667B96E943FBD25679B7`。
- 安装包验证：MSI/NSIS 已由 Tauri release bundle 生成，release exe 已隐藏启动 5 秒并按精确 PID 关闭；覆盖升级、安装后启动和卸载未在用户已有安装环境上执行，以避免删除或覆盖用户数据。

## 本次工作验证状态

- `pnpm install --frozen-lockfile`、`pnpm lint`、`pnpm typecheck`、`pnpm test --run`、`pnpm build`：均通过。
- Rust stable 1.97.1 已安装；`cargo fmt --check` 通过。
- `cargo check --all-targets --all-features`、严格 Clippy、`cargo test --all-features` 均通过；0.3.0 release exe 已验证进程成功启动并关闭。
- `git diff --check`：通过；Git 输出的 LF→CRLF 是工作区换行提示，不是 diff 错误。

## Git 检查点

- `e918cef feat: build secure SSH operations foundation`
- `17c32e6 feat: add files transfers and runtime operations`
- `48eece0 feat: 完成 Relay 0.3.0 运维工作区`
- 文档收尾提交与远端推送状态以 `git log -1` 和 `git status --short --branch` 为准。

## 需优先审计的技术债

1. `AppError` 使用 crate-level `clippy::result_large_err` 豁免；这是为了 IPC 结构化错误，后续可评估 Box，但不要无理由破坏序列化形状。
2. `security::redact` 是基础 marker redaction，尚非完整 secret-aware tracing layer。
3. Remote command abstraction 仍以固定 domain-generated command string 为核心，未形成主需求完整 `RemoteCommandRequest`、统一 task progress 类型；安装/pull/follow 已有 stream/cancel。
4. Tool/Docker/日志流式任务已有 task id、取消 endpoint、SSH channel close 和全局任务中心；远程命令/流式任务的脱敏元数据已持久化并可标记 interrupted，文件传输的详细进度与关闭应用后的恢复仍为 P1。
5. `audit_events` 已由服务器档案、连接、文件、服务、工具、Nginx 和 Docker 变更命令写入；仍需真实 UI 验证审计展示与脱敏导出。
6. Tauri capability 为 core + dialog open/save；每加插件都需最小 scope 审计。
7. CI 声明 Windows/macOS，但尚未在 GitHub Actions 远程实际观察结果；push 后立即查看首轮 CI。
8. 文件预览/书签、终端快捷指令、日志、监控和任务持久化均已同步到本文件；剩余主要证据缺口是用户真实服务器和安装流程验收。
