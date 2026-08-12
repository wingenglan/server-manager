# Next steps

本文是接手执行顺序，不替代主需求。每完成一个步骤都应先保持项目可编译/可测试，再更新 `ACCEPTANCE.md`。

## P0 — 接手与真实基础链路（必须先做）

### 1. 新电脑复现基线

- 安装 Node.js 22、pnpm 10、Rust stable MSVC、VS C++ Build Tools + Windows SDK、WebView2。
- `pnpm install --frozen-lockfile`。
- 运行 `HANDOFF.md` 的所有前端/Rust门禁。
- 运行 `pnpm tauri dev`，确认空状态/设置/add dialog 正常。
- 查看 GitHub Actions 首轮 Windows/macOS CI；修复平台差异。

完成条件：本地全量门禁与 CI 均绿；把确切版本和结果写入 `CURRENT_STATE.md`。

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

- connection test、有限退避 reconnect、connection error state；真实 sidebar online/offline。
- Server groups/search/recent/favorite、duplicate/edit/delete 完整交互。
- 文件冲突策略（Skip/Replace/Rename/Apply to all）、文件夹选择上传、retry、远端内部 move/copy、chmod、large file viewer。
- editor Reload/Compare、sudo permission failure UX；测试 destructive confirm 与 file conflict UI。
- Overview top processes、mount 列表、network sampling 稳定性、warning thresholds。
- `ss` 权限不足提示 + sudo rescan + fallback；process/service detail 与 logs。
- command palette 从 placeholder 变成 server/terminal/files/port 的真实 command/filter。

完成条件：Milestone 1–3 的 P0 全部有 UI + backend + errors + tests + real evidence。

## P0 — Milestone 4: Tools + Nginx

### 4. Tools registry 与安装

- 添加 `PlatformAdapter`/capability model，集中 Ubuntu/Debian 与 RHEL family 差异。
- registry 至少覆盖 Nginx、Docker、Git、curl/wget、tar/unzip。
- detection 返回 installed/version/running/permission；不假设命令存在。
- install 前展示 package manager、exact plan、风险与 sudo 要求；用户确认后 streaming output/cancel/verify。
- 不自动升级、不未经点击安装。

### 5. Nginx 管理

- 从 `nginx -T` 解析 stdout/stderr，保留 source file mapping 与未知指令。
- fixtures 覆盖 include、server_name、listen、proxy_pass、upstream、注释、多个 server block。
- reverse proxy list 显示 host/listen/target/source/status。
- Wizard 只写独立 managed conf；保存流程 backup → temp/atomic → `nginx -t` → reload → verify；任何失败 rollback。
- 不把证书 private key 内容拉到前端。

完成条件：主需求 G 在隔离配置上真实通过，错误配置不影响原 Nginx。

## P0 — Milestone 5–6: Docker

### 6. Docker Core

- CLI-over-SSH，优先 `--format json`/inspect JSON；不得暴露 daemon TCP/socket。
- containers list/actions/logs follow/inspect/stats/exec；actions confirm + verify。
- images list/pull streaming/run wizard/remove；secret-like env 默认 mask。
- fixtures 与 parser tests 先行，UI 使用真实数据。

### 7. Docker Extended

- volumes/networks inspect/create/delete with high-risk confirm。
- Compose project discovery、services/logs/restart、Monaco yaml、`docker compose config`、apply。
- cleanup 逐项选择并预览；volume 和 `down -v` 强确认。

完成条件：主需求 H–J 真实通过。

## P0 — Milestone 7: Backup, polish, packaging

### 8. Import/export

- 普通 versioned JSON export 不含 secret。
- 完整 backup：Argon2id + modern AEAD，保存 KDF params/salt/nonce/ciphertext；wrong password fail。
- import 后 secret 重新进入 OS Keychain；不得留临时明文。

### 9. Product polish

- settings persistence、dark/light、i18n structure、shortcuts、real notifications/toasts/task center。
- local audit events（无 secret）、redacted diagnostics export、app log、About/version。
- performance：1k/10k lists、50 server probe limits、page visibility pollers、stream cancellation。
- crash 后不重放 destructive task。

### 10. Full release

- 跑 A–K acceptance；Ubuntu password + Rocky/Alma private key target 都需要证据（当前只提供 Ubuntu-like test server）。
- Windows/macOS build；代码不得硬编码 Windows-only 路径。
- `pnpm tauri build`，记录安装包绝对/仓库相对路径、size、SHA-256。
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
