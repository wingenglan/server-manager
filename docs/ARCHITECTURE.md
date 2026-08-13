# Architecture

## 边界

Relay 是单机桌面应用。React WebView 只负责展示和输入；所有持久化、凭据访问、SSH/SFTP、远程命令、解析与危险操作验证均位于 Rust 边界内。远端服务器只需现有 OpenSSH 与标准 Linux 工具。

```text
React features
  -> typed Tauri commands/events
Application state (repositories, task manager, SSH pool)
  -> domain services (server, metrics, files, process, services, nginx, docker)
Infrastructure (SQLite, OS keychain, russh, russh-sftp, platform adapters)
  -> SSH/SFTP
Remote Linux
```

## 本地数据

SQLite 使用编译期迁移创建 `servers`、`server_groups`、`tags`、`server_tags`、`known_hosts`、`recent_paths`、`app_settings` 和 `audit_events`。数据库不保存 secret，secret 列只含随机 reference id。

## SSH 生命周期

每个服务器由连接管理器维护至多一个可复用认证会话，普通短命令通过 semaphore 限制为 8 个并发 channel。连接流程先完成 Host Key 校验，再读取凭据认证。未知 Host Key 仅进入短期 pending challenge；用户确认后写入 `known_hosts`。密钥变化不会降级为重新信任。

## 远程命令

React 不传裸 shell。各 domain service 构建固定 program 与经过 POSIX shell escaping 的参数；默认注入 `LC_ALL=C`，并返回结构化 `RemoteCommandResult`。sudo 通过独立 privileged runner 的 stdin 通道实现，secret 永不进入 argv 或日志。

## 前端

前端按 feature 切分，TanStack Query 管理远端状态，Zustand 仅管理 UI session，Radix 提供可访问交互原语。列表达到大数据量时使用 TanStack Virtual；终端和日志使用 Tauri Channel/Event 流式传输。

Tools、Nginx 和 Docker 当前也遵循同一 Rust boundary：工具 registry 生成固定包管理命令，Nginx parser 消费 `nginx -T` 并只返回结构化 proxy/source mapping，前端不接触裸 shell 或 Docker socket；Docker 通过 SSH 执行远程 CLI 的 JSON 输出。
