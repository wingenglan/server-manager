# Remote Compatibility

| 能力 | Ubuntu / Debian | Rocky / Alma / RHEL | Fallback |
|---|---|---|---|
| 系统识别 | `/etc/os-release` | `/etc/os-release` | 基础 Linux，标记 unsupported |
| 包管理 | `apt-get` / `dpkg-query` | `dnf`，其次 `yum` / `rpm` | 禁用安装 |
| 服务 | `systemctl` / `journalctl` | `systemctl` / `journalctl` | 仅探测，不伪装支持 |
| 端口 | `ss` | `ss` | `lsof`（如存在） |
| 网络 | `/proc/net/dev` + `ip` | `/proc/net/dev` + `ip` | 仅 `/proc` 指标 |
| 文件 | SFTP v3 | SFTP v3 | 无 SFTP 时明确报错 |
| Docker | 远程 `docker` CLI | 远程 `docker` CLI | 未安装/权限不足分开呈现 |
| Nginx | `nginx -T` + systemd | `nginx -T` + systemd | 编译参数推导路径 |

所有能力先进入 `ServerCapabilities`；前端据此禁用不可用操作并显示原因。发行版差异集中在 `PlatformAdapter`，不散落在 UI 或业务 service。
