# Relay 项目交接入口

> **给下一位 AI Coding Agent：先完整阅读本文件，再执行任何修改。** 这是稳定入口；详细状态在下方链接中。Relay 0.3.0 的代码、自动化门禁和 Windows release 包已完成；真实服务器 UI 验收与用户环境安装验证仍未闭环。

最后更新：2026-08-14（Asia/Shanghai）

当前阶段：Milestone 0 已完成；Milestone 1–7 的主要代码、本地交互和 Relay 0.3.0 新增模块已补齐，但真实服务器验收未完成。0.3.0 已完成终端快捷指令、统一日志中心、短期监控图表、任务持久化、Windows 打包和最终文档同步。

交付分支：`main`，远程仓库 `git@github.com:wingenglan/server-manager.git`。

最近代码检查点：`48eece0 feat: 完成 Relay 0.3.0 运维工作区`。安装包、SHA-256、自动化证据和未完成的真实验收项见 [`docs/CURRENT_STATE.md`](docs/CURRENT_STATE.md) 与 [`docs/ACCEPTANCE.md`](docs/ACCEPTANCE.md)。

## 必读顺序

1. [`doc/agentless_server_manager_codex_master_prompt_zh.md`](doc/agentless_server_manager_codex_master_prompt_zh.md) — 最高优先级完整产品需求，必须完整阅读。
2. [`docs/CURRENT_STATE.md`](docs/CURRENT_STATE.md) — 已完成代码、验证证据、未验证边界与已知问题。
3. [`docs/NEXT_STEPS.md`](docs/NEXT_STEPS.md) — 推荐接手顺序和每一步的完成条件。
4. [`docs/SECURITY.md`](docs/SECURITY.md) — secret、Host Key、sudo 和 WebView 安全边界。
5. [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) 与 [`docs/DECISIONS.md`](docs/DECISIONS.md) — 分层和已接受技术决定。
6. [`docs/ACCEPTANCE.md`](docs/ACCEPTANCE.md) — 只能按证据维护的验收状态。
7. [`docs/HANDOFF_PROTOCOL.md`](docs/HANDOFF_PROTOCOL.md) — 你下一次暂停/换 Agent 时必须执行的交接规范。

仓库根目录的 [`AGENTS.md`](AGENTS.md) 是持续生效的 Agent 工作规则。

## 立即开始的操作

```bash
git status --short
git log --oneline -5
pnpm install --frozen-lockfile
pnpm lint
pnpm typecheck
pnpm test --run
pnpm build

cd src-tauri
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Windows 需要 Rust stable MSVC、Visual Studio Build Tools 的“使用 C++ 的桌面开发”、Windows 10/11 SDK、WebView2 和 Node.js 22+/pnpm 10+。若 `link.exe` 或 `kernel32.lib` 缺失，先修复 C++/Windows SDK 安装，不要修改项目依赖来绕过工具链。

## 测试服务器（用户明确允许写入交接并用于本项目）

| 项目 | 值 |
|---|---|
| Host | `8.138.151.118` |
| SSH port | `22` |
| Username | `root` |
| Password | `@Wingeng1218` |
| SSH banner | `OpenSSH_9.6p1 Ubuntu-3ubuntu13.18` |
| 已观测 Host Key | `ssh-ed25519` |
| SHA-256 fingerprint | `SHA256:HgdpNiGhsnHgyvH1EXVQZTt+kTsHKo1/Myy4Celhlfw` |

使用要求：

- 这是测试机，但仍应通过 Relay UI 输入密码并确认上述指纹；不要把密码塞入 shell 命令、源码、fixture、日志或 CI。
- 本机现有公钥已用 `BatchMode=yes` 做过只读认证探测，服务器返回 `Permission denied (publickey,password)`；没有远程命令被执行。
- TCP/22、SSH banner 和 Host Key 已做非侵入式预检。尚未通过 Relay UI 完成密码认证、Overview、Terminal、SFTP 或破坏性验收。
- 远端安装/删除/kill/Nginx/Docker 改动必须由用户在产品 UI 中明确触发。破坏性测试只使用隔离资源，并在验收文档记录清理结果。

## 当前最重要的事实

- 代码不是 Mock：SSH、PTY、SFTP、系统探测、传输和运行现场均走 Rust + russh/russh-sftp 的真实路径。
- 当前最大的证据缺口是没有通过产品 UI 使用密码登录测试机。未验证的 Milestone 1–3 项仍标为 `[~]`。
- Tools/Nginx 已有 registry、PlatformAdapter、配置 parser、证书到期元数据、HTTPS 文件存在性检查、后端探活、`nginx -T` 源文件聚合和 managed conf safety flow；Docker 已有 CLI-over-SSH 的 overview/container/image/log/follow/inspect/stats/top/exec/volume/network/pull/run、容器重命名/复制 ID/打开发布端口、日志筛选导出和 Compose project discovery、services/logs、默认脱敏 config、原始 YAML 显式编辑/校验/失败恢复、逐项 cleanup、容器资源限制字段；工具安装、pull、follow 支持取消远程 channel。仍缺真实服务器证据、完整翻译资源和真实更新通道；Windows x64 MSI/NSIS 安装包已生成。普通配置 JSON、Argon2id+AES-256-GCM 完整备份、全局任务中心、跨页面 toast、locale 偏好结构、更新预留入口、Tauri app log、脱敏诊断、审计记录与档案复制均已接入。
- 本次增量已补 Docker 容器重命名/复制/发布端口、镜像删除、卷网络 inspect、日志 search/pause/clear/tail/copy/download、Inspect/Stats 搜索复制与短期采样、Compose start/stop/pull/build 和资源任务中心；Overview runtime 可打开 Docker/Nginx systemd 日志；Nginx `nginx -T` 聚合 include 源文件并可跳转 Files；设置页已补 locale、toast、app log 和更新预留入口。
- 本次接手验证：前端 lint/typecheck/test/build、`git diff --check`、Rust fmt/check/clippy/test 均通过；`pnpm tauri build` 已生成 0.3.0 Windows x64 MSI/NSIS，release exe 已 smoke launch。详见 [`docs/CURRENT_STATE.md`](docs/CURRENT_STATE.md)。
- 安装包路径和 SHA-256 已记录在 [`docs/CURRENT_STATE.md`](docs/CURRENT_STATE.md)；不要提交 `src-tauri/target/` 构建目录。
- `src-tauri/target/` 曾约 11.6 GB，已被 `src-tauri/.gitignore` 排除。不要提交构建目录。

## 0.3.0 执行边界

- 快捷指令保存为全局默认或单服务器覆盖；Tab 只插入命令，Enter 仍由远端终端执行。
- 支持 `{{变量}}` 参数填写；不记录完整终端输入、命令输出或快捷指令中的敏感凭据。
- 日志中心覆盖 system journal、systemd/Nginx、Docker/Compose；远程日志只流式展示，不默认持久化内容。
- 监控历史只在本地保存 24 小时或每服务器最多 20,000 条采样，不安装远程 Agent。
- 应用重启时把 queued/running 任务标为 `interrupted`，只允许用户手动重试，不自动重放危险任务。

## 下一条推荐任务

下一步通过 Relay UI 添加上述测试机、核对 Host Key、完成密码连接，验证 Overview、双终端、SFTP 只读浏览、快捷指令、日志、图表历史和任务中断恢复；基础链路通过后再验收 Tools、Nginx `-T`/HTTPS 探活、Docker read-only 与隔离资源写操作。由于当前执行环境没有桌面 UI 自动化通道，这部分必须由用户在本地安装包中完成并回填 [`docs/ACCEPTANCE.md`](docs/ACCEPTANCE.md)。随后补完整翻译资源、真实更新通道、macOS 包和全量 A-K 证据。

## 交接完成的判定

接手 Agent 不能只读本文件后口头总结。它应：

1. 确认已读主需求与交接文档；
2. 报告 `git status` 和基线检查结果；
3. 从 `docs/NEXT_STEPS.md` 第一项未完成任务继续；
4. 在下次暂停时按 `docs/HANDOFF_PROTOCOL.md` 更新交接链。
