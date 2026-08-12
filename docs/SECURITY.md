# Security Model

## Secret 流向

```text
密码输入框（React 内存）
  -> Tauri IPC 参数（一次性）
  -> Rust SecretString / zeroize
  -> OS Keychain
  -> SQLite 仅记录随机 secret_ref
```

密码不会进入 LocalStorage、SQLite、日志、诊断包或终端历史。保存完成后 React 表单清空。Keychain 写入失败时整个保存操作失败，不做明文降级。

## Host Key

1. SSH 握手获取远端公钥。
2. 未知密钥返回算法和 SHA-256 fingerprint，拒绝继续认证。
3. 用户明确选择“信任并连接”后保存完整 OpenSSH public key。
4. 后续密钥不一致返回 `HOST_KEY_CHANGED` 并中止；此场景不能在普通连接弹窗中覆盖旧密钥。

## sudo

普通 runner 不接受 sudo 密码。privileged runner 使用 `sudo -S -p '' --` 并通过 stdin 发送密码，argv、错误与 tracing 字段均只含脱敏摘要。所有高权限写入先校验目标路径，并在操作后验证结果。

## 日志与诊断

日志过滤器对 password/passphrase/token/authorization/private key 等模式做统一 redaction。诊断导出仅含版本、能力、结构化错误和脱敏环境，不含命令 stdin、终端输入或远程敏感文件内容。

## WebView

CSP 默认拒绝远程脚本与页面。当前主窗口 capability 只开启 Tauri core；前端没有系统浏览器、任意本地 shell、文件系统或数据库权限。需要外链时将按 URL scope 单独增加 opener 权限。
