# Next steps

本文是接手执行顺序，不替代主需求。每完成一个步骤都应先保持项目可编译/可测试，再更新 `ACCEPTANCE.md`。

当前代码事实：Tools/Nginx、PlatformAdapter、Docker extended、Server groups/duplicate、文件冲突策略与 Copy/Move/chmod/symlink、Overview top/mount/capabilities、systemd detail/logs、有限退避重连、ss/lsof 端口回退和 Argon2id+AES-GCM backup、task center、audit、diagnostics 已有 Rust/typed IPC/UI 基础实现；0.3.0 终端快捷指令、统一日志、监控历史和任务持久化已完成，前端/Rust 本地门禁与 Windows x64 MSI/NSIS 构建均通过。剩余工作以真实服务器 UI 验收、用户环境安装验证和后续跨平台补齐为主。

## 0.3.0 执行顺序（已完成代码与打包，真实验收待用户执行）

1. 文档基线：已同步 HANDOFF、CURRENT_STATE、ARCHITECTURE、SECURITY、ACCEPTANCE 和本文件。
2. 终端快捷指令：已完成 SQLite/typed IPC、默认指令分组、全局/服务器覆盖、标签、变量、匹配、Tab 插入；已补不遮挡输入区的建议框、单 shell 开关、建议隐藏操作和内存命令历史定位。
3. 日志中心：已完成 system journal、systemd/Nginx、Docker/Compose、follow、暂停、搜索、下载和最大缓冲。
4. 监控图表：已完成本地 24 小时/20,000 条滚动采样与 Overview 1h/6h/24h 趋势图。
5. 任务持久化：已完成 `task_records`、启动中断标记、手动返回原模块重试入口和禁止自动重放。
6. 真实验收：待用户通过 Relay UI 执行，只读优先，写入限定在 `/tmp/relay-acceptance-*` 和隔离 Docker 资源，并回填证据。
7. 版本 0.3.0：MSI/NSIS 已生成，release smoke launch 已通过；覆盖升级、安装后启动和卸载待用户环境验证。

## P0 — 接手与真实基础链路（必须先做）

### 1. 新电脑复现基线

- 安装 Node.js 22、pnpm 10、Rust stable MSVC、VS C++ Build Tools + Windows SDK、WebView2。
- `pnpm install --frozen-lockfile`。
- 运行 `HANDOFF.md` 的所有前端/Rust门禁。
- 运行 `pnpm tauri dev`，确认空状态/设置/add dialog 正常。
- 查看 GitHub Actions 首轮 Windows/macOS CI；修复平台差异。

完成条件：本地全量门禁与 Tauri release build 均绿；确切版本、产物和结果已写入 `CURRENT_STATE.md`。CI 仍需 push 后观察。

### 2. 用 Relay UI 验证测试服务器

使用 `HANDOFF.md` 的测试机信息：

1. 添加 password profile，密码必须通过 UI 进入 OS Keychain。
2. 首次连接核对 ed25519 fingerprint，必须与交接记录一致后才能信任。
3. 重启应用，确认 profile 存在；检查 SQLite 没有明文密码。
4. 验证 Overview、两个 terminal tabs、resize、Ctrl+C、搜索、断开提示。
5. 只读浏览 `/`, `/etc`, `/tmp`，打开一个小文本文件但先不保存。
6. 在测试专用目录（建议 `/tmp/relay-acceptance-*`）上传/下载/rename/new folder/delete；完成后清理。
7. 在测试目录验证 editor conflict 与普通保存；有必要时用专用文件验证 sudo backup/save。
8. 用隔离进程监听高位端口（不要占用系统服务），验证 inspect → SIGTERM → rescan → released。

完成条件：修复发现的 bug，`ACCEPTANCE.md` 只把有证据的项改 `[x]`；记录测试资源与清理结果。不要在首次验证时安装/修改 Nginx 或 Docker。

### 3. 补齐 Milestone 1–3 P0 缺口

按优先级：

- connection test、有限退避 reconnect、connection error state 已接入；真实 sidebar online/offline 仍需 Relay UI 证据。
- Server groups/search/recent/favorite、edit/delete、duplicate 已接入；仍需远端真实证据。
- 文件冲突策略（Skip/Replace/Rename，当前上传批次 Apply-to-all）、文件夹选择上传、远端内部 Copy/Move、chmod、symlink、Large File Viewer 和失败/取消后重新提交已接入；pause/resume 仍为 P1。
- sudo permission failure UX；测试 destructive confirm 与 file conflict UI（editor Reload/Compare 已接入）。
- Overview top processes、mount 摘要、network sampling 已接入；仍需 warning thresholds、virtualization、真实证据。
- process/service detail、logs、enable/disable、`ss` 权限不足提示、sudo rescan 和 lsof fallback 已接入；仍需真实权限组合证据。
- command palette 已支持 server/terminal/files/tools/Nginx/Docker/operations 导航；仍需真实 command/filter 证据。

完成条件：Milestone 1–3 的 P0 全部有 UI + backend + errors + tests + real evidence。

## P0 — Milestone 4: Tools + Nginx

### 4. Tools registry 与安装

- 添加 `PlatformAdapter`/capability model，集中 Ubuntu/Debian 与 RHEL family 差异（已完成，需真实目标验证）。
- registry 至少覆盖 Nginx、Docker、Git、curl/wget、tar/unzip。
- detection 返回 installed/version/running/permission；不假设命令存在。
- install 前展示 package manager、exact plan、风险与 sudo 要求；用户确认后 streaming output/verify，并支持取消远程 channel；全局 task center 已接入。
- 不自动升级、不未经点击安装。

### 5. Nginx 管理

- 从 `nginx -T` 解析 stdout/stderr，保留 source file mapping 与未知指令。
- fixtures 覆盖 include、server_name、listen、proxy_pass、upstream、注释、多个 server block。
- reverse proxy list 显示 host/listen/target/source/status。
- Wizard 只写独立 managed conf；保存流程 backup → temp/atomic → `nginx -t` → reload → verify；任何失败 rollback。
- HTTPS wizard 会要求证书/私钥路径已存在；证书 expiry 只读取公钥证书元数据，不把 private key 内容拉到前端。

完成条件：主需求 G 在隔离配置上真实通过，错误配置不影响原 Nginx。

## P0 — Milestone 5–6: Docker

### 6. Docker Core

- CLI-over-SSH，优先 `--format json`/inspect JSON；不得暴露 daemon TCP/socket。
- containers list/actions/logs follow/inspect/stats/exec；actions confirm + verify；follow 为 30 秒有时限并支持取消。
- images list/pull streaming/run wizard/remove；secret-like env 默认 mask。
- fixtures 与 parser tests 先行，UI 使用真实数据。

### 7. Docker Extended

- volumes/networks inspect/create/delete with high-risk confirm。
- Compose project discovery、up/down/restart、services/logs、默认脱敏 Monaco yaml/config、原始 YAML 显式编辑校验/失败恢复、资源候选和逐项 cleanup 已接入；仍需真实验收。
- cleanup 逐项选择并预览；volume 和 `down -v` 强确认。

完成条件：主需求 H–J 真实通过。

## P0 — Milestone 7: Backup, polish, packaging

### 8. Import/export

- 普通 versioned JSON export/import 与 Argon2id + AES-256-GCM 完整备份已实现；需在 Rust 门禁和真实 UI 通过后补证据。
- 完整 backup 保存 KDF params/salt/nonce/ciphertext；wrong password fail；import 后 secret 重新进入 OS Keychain，不留持久化明文。

### 9. Product polish

- settings persistence、dark/light、locale preference structure、shortcuts、cross-page notifications/toasts/task center（toast 与 locale 结构已接入，仍需真实桌面验证和完整翻译资源）。
- local audit events（无 secret）、redacted diagnostics export、app log、About/version（app log 已写入 Tauri app log 目录，仍需桌面路径验证）。
- performance：1k/10k lists、50 server probe limits、page visibility pollers、stream cancellation。
- crash 后不重放 destructive task。

### 10. Full release

- 跑 A–K acceptance；Ubuntu password + Rocky/Alma private key target 都需要证据（当前只提供 Ubuntu-like test server）。
- Windows/macOS build；Windows x64 MSI/NSIS 已完成，macOS 仍待 CI/目标机验证；代码不得硬编码 Windows-only 路径。
- Windows `pnpm tauri build` 已完成，0.3.0 路径、size、SHA-256 已记录在 `docs/CURRENT_STATE.md`；后续只需补用户环境安装验收和 macOS 包。
- 更新 README screenshots、support matrix、known limitations 与 release notes。

## 每个切片的固定质量门

```bash
pnpm lint && pnpm typecheck && pnpm test --run && pnpm build
cd src-tauri
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

涉及 UI 的切片还要启动桌面应用做视觉/交互 QA；涉及远端能力的切片必须在真实目标验证，且记录权限不足、断网、取消与失败路径。
