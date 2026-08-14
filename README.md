# Relay · Server Manager

> **继续开发请从 [`HANDOFF.md`](HANDOFF.md) 开始。** 它包含当前实现状态、测试服务器、验证证据、下一步顺序和后续交接规范。

Relay 是一个基于 Tauri 2、React 和 Rust 的本地优先 Linux 桌面运维客户端。它通过 SSH、SFTP 与远程标准命令管理多台服务器，并采用严格的凭据、Host Key 和命令边界保护，不暴露 Docker API。

> 当前仓库正在按 `docs/ACCEPTANCE.md` 的里程碑清单持续开发。未标记通过的能力不得视为已完成。

## 开发环境

- Node.js 22+
- pnpm 10+
- Rust stable（MSVC toolchain on Windows）
- Tauri 2 的平台依赖：Windows WebView2、macOS Xcode Command Line Tools，或 Linux WebKitGTK

```bash
pnpm install
pnpm tauri dev
```

## 构建与测试

```bash
pnpm lint
pnpm typecheck
pnpm test
pnpm build
pnpm tauri build

cd src-tauri
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

安装包由 `pnpm tauri build` 生成到 `src-tauri/target/release/bundle/`。

## 安全模型

- 服务器档案与非敏感偏好保存在本机 SQLite。
- SSH 密码、私钥口令和 sudo 密码只写入操作系统安全存储；SQLite 仅保存不可逆引用。
- 首次连接必须人工确认 Host Key 指纹；已知主机密钥变化会中止连接。
- React 端只能调用白名单 Tauri commands，不能执行任意 shell。
- 远程 Docker 通过 SSH 上的 CLI 操作，不开放 `/var/run/docker.sock`。

详见 [安全设计](docs/SECURITY.md)、[架构](docs/ARCHITECTURE.md) 与 [当前实现快照](docs/CURRENT_STATE.md)。

## 支持的远端系统

P0 目标为 Ubuntu/Debian 与 Rocky/Alma/RHEL 的 systemd + OpenSSH 环境。其他 Linux 发行版仅提供能力探测后可确认安全的基础功能，详见 [兼容性矩阵](docs/REMOTE_COMPATIBILITY.md)。

## 截图

发布前截图将放在 `docs/screenshots/`，并与实际构建保持一致。

## License

MIT
