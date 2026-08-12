use crate::domain::server::ServerRecord;
use crate::errors::{AppError, AppResult};
use crate::infra::db::{KnownHost, ServerRepository};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use russh::client;
use russh::keys::ssh_key::{HashAlg, PublicKey};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Semaphore};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostKeyChallenge {
    pub server_id: String,
    pub host: String,
    pub port: u16,
    pub key_type: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustHostKeyInput {
    pub server_id: String,
    pub host: String,
    pub port: u16,
    pub key_type: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionSnapshot {
    pub server_id: String,
    pub status: ConnectionStatus,
    pub connected_at: Option<DateTime<Utc>>,
    pub error: Option<AppError>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    Offline,
    Connecting,
    Online,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum ConnectOutcome {
    Connected(ConnectionSnapshot),
    HostKey(HostKeyChallenge),
}

#[derive(Debug, Clone)]
struct PendingHostKey {
    server_id: String,
    host: String,
    port: u16,
    key_type: String,
    fingerprint: String,
    public_key: String,
    observed_at: Instant,
}

struct ManagedConnection {
    handle: client::Handle<HostKeyHandler>,
    connected_at: DateTime<Utc>,
    limiter: Arc<Semaphore>,
}

#[derive(Clone)]
pub struct SshConnectionManager {
    repository: ServerRepository,
    connections: Arc<DashMap<String, Arc<ManagedConnection>>>,
    pending: Arc<DashMap<String, PendingHostKey>>,
    connecting: Arc<DashMap<String, ()>>,
    terminals: Arc<DashMap<String, ManagedTerminal>>,
}

impl SshConnectionManager {
    pub fn new(repository: ServerRepository) -> Self {
        Self {
            repository,
            connections: Arc::new(DashMap::new()),
            pending: Arc::new(DashMap::new()),
            connecting: Arc::new(DashMap::new()),
            terminals: Arc::new(DashMap::new()),
        }
    }

    pub fn snapshot(&self, server_id: &str) -> ConnectionSnapshot {
        let closed = self
            .connections
            .get(server_id)
            .map(|connection| connection.handle.is_closed())
            .unwrap_or(false);
        if closed {
            self.connections.remove(server_id);
        }
        let (status, connected_at) = if let Some(connection) = self.connections.get(server_id) {
            (ConnectionStatus::Online, Some(connection.connected_at))
        } else if self.connecting.contains_key(server_id) {
            (ConnectionStatus::Connecting, None)
        } else {
            (ConnectionStatus::Offline, None)
        };
        ConnectionSnapshot {
            server_id: server_id.to_string(),
            status,
            connected_at,
            error: None,
        }
    }

    pub async fn connect(&self, server_id: &str) -> AppResult<ConnectOutcome> {
        if self.connections.contains_key(server_id) {
            return Ok(ConnectOutcome::Connected(self.snapshot(server_id)));
        }
        if self.connecting.insert(server_id.to_string(), ()).is_some() {
            return Err(
                AppError::new("SSH_CONNECT_IN_PROGRESS", "ssh", "该服务器正在连接")
                    .for_server(server_id),
            );
        }
        let result = self.connect_inner(server_id).await;
        self.connecting.remove(server_id);
        result
    }

    async fn connect_inner(&self, server_id: &str) -> AppResult<ConnectOutcome> {
        let record = self.repository.record(server_id).await?;
        let identity = format!("{}:{}", record.host, record.port);
        let known = self.repository.known_host(&identity).await?;
        let observation = Arc::new(Mutex::new(None));
        let handler = HostKeyHandler {
            expected: known,
            observation: observation.clone(),
        };
        let config = client::Config {
            inactivity_timeout: Some(Duration::from_secs(record.connect_timeout().max(5))),
            keepalive_interval: Some(Duration::from_secs(record.keepalive().max(5))),
            keepalive_max: 3,
            ..Default::default()
        };
        let address = (record.host.as_str(), record.port as u16);
        let connection = tokio::time::timeout(
            Duration::from_secs(record.connect_timeout().max(5)),
            client::connect(Arc::new(config), address, handler),
        )
        .await;

        let mut handle = match connection {
            Err(_) => {
                return Err(AppError::new("NETWORK_TIMEOUT", "network", "SSH 连接超时")
                    .for_server(server_id)
                    .suggestion("检查主机地址、端口与网络连通性"))
            }
            Ok(Err(error)) => {
                let observed = observation.lock().ok().and_then(|value| value.clone());
                if let Some(HostKeyObservation::Unknown(value)) = observed {
                    let pending = PendingHostKey {
                        server_id: server_id.to_string(),
                        host: record.host.clone(),
                        port: record.port as u16,
                        key_type: value.key_type,
                        fingerprint: value.fingerprint,
                        public_key: value.public_key,
                        observed_at: Instant::now(),
                    };
                    let challenge = pending.challenge();
                    self.pending.insert(server_id.to_string(), pending);
                    return Ok(ConnectOutcome::HostKey(challenge));
                }
                if matches!(observed, Some(HostKeyObservation::Changed)) {
                    return Err(AppError::new(
                        "HOST_KEY_CHANGED",
                        "security",
                        "服务器 Host Key 已发生变化",
                    )
                    .for_server(server_id)
                    .suggestion("停止连接并通过可信渠道核对服务器身份")
                    .fatal());
                }
                return Err(map_connect_error(error, server_id));
            }
            Ok(Ok(handle)) => handle,
        };

        authenticate(&mut handle, &record, &self.repository).await?;
        let managed = Arc::new(ManagedConnection {
            handle,
            connected_at: Utc::now(),
            limiter: Arc::new(Semaphore::new(8)),
        });
        self.connections.insert(server_id.to_string(), managed);
        self.repository.mark_connected(server_id).await?;
        Ok(ConnectOutcome::Connected(self.snapshot(server_id)))
    }

    pub async fn trust(&self, input: TrustHostKeyInput) -> AppResult<ConnectionSnapshot> {
        let Some((_, pending)) = self.pending.remove(&input.server_id) else {
            return Err(AppError::new(
                "HOST_KEY_CHALLENGE_EXPIRED",
                "security",
                "Host Key 确认已过期，请重新连接",
            ));
        };
        if pending.observed_at.elapsed() > Duration::from_secs(300) {
            return Err(AppError::new(
                "HOST_KEY_CHALLENGE_EXPIRED",
                "security",
                "Host Key 确认已超过 5 分钟，请重新连接",
            ));
        }
        if pending.host != input.host
            || pending.port != input.port
            || pending.key_type != input.key_type
            || pending.fingerprint != input.fingerprint
        {
            return Err(AppError::new(
                "HOST_KEY_CHALLENGE_MISMATCH",
                "security",
                "Host Key 确认内容与握手结果不一致",
            )
            .fatal());
        }
        self.repository
            .trust_host(&KnownHost {
                server_identity: format!("{}:{}", pending.host, pending.port),
                key_type: pending.key_type,
                fingerprint: pending.fingerprint,
                public_key: pending.public_key,
            })
            .await?;
        match self.connect(&input.server_id).await? {
            ConnectOutcome::Connected(snapshot) => Ok(snapshot),
            ConnectOutcome::HostKey(_) => Err(AppError::new(
                "HOST_KEY_UNKNOWN",
                "security",
                "保存 Host Key 后仍无法验证服务器身份",
            )
            .fatal()),
        }
    }

    pub async fn disconnect(&self, server_id: &str) -> AppResult<()> {
        let terminal_ids: Vec<String> = self
            .terminals
            .iter()
            .filter(|entry| entry.server_id == server_id)
            .map(|entry| entry.key().clone())
            .collect();
        for terminal_id in terminal_ids {
            let _ = self.close_terminal(&terminal_id).await;
        }
        if let Some((_, connection)) = self.connections.remove(server_id) {
            connection
                .handle
                .disconnect(russh::Disconnect::ByApplication, "user disconnect", "en")
                .await
                .map_err(|error| {
                    AppError::new("SSH_DISCONNECT_FAILED", "ssh", "SSH 断开失败")
                        .details(error)
                        .for_server(server_id)
                })?;
        }
        Ok(())
    }

    pub async fn open_terminal(
        &self,
        server_id: &str,
        columns: u32,
        rows: u32,
    ) -> AppResult<(String, mpsc::Receiver<TerminalEvent>)> {
        let connection = self
            .connections
            .get(server_id)
            .map(|value| value.clone())
            .ok_or_else(|| {
                AppError::new("SSH_NOT_CONNECTED", "ssh", "服务器尚未连接").for_server(server_id)
            })?;
        let channel = connection
            .handle
            .channel_open_session()
            .await
            .map_err(|error| {
                AppError::new("TERMINAL_OPEN_FAILED", "terminal", "无法创建终端 channel")
                    .details(error)
                    .for_server(server_id)
            })?;
        channel
            .request_pty(
                true,
                "xterm-256color",
                columns.max(20),
                rows.max(5),
                0,
                0,
                &[],
            )
            .await
            .map_err(|error| {
                AppError::new("TERMINAL_PTY_FAILED", "terminal", "远端拒绝创建 PTY")
                    .details(error)
                    .for_server(server_id)
            })?;
        channel.request_shell(true).await.map_err(|error| {
            AppError::new("TERMINAL_OPEN_FAILED", "terminal", "远端拒绝启动交互 Shell")
                .details(error)
                .for_server(server_id)
        })?;

        let terminal_id = uuid::Uuid::new_v4().to_string();
        let (sender, receiver) = mpsc::channel(256);
        let (control_sender, mut control_receiver) = mpsc::channel(256);
        self.terminals.insert(
            terminal_id.clone(),
            ManagedTerminal {
                server_id: server_id.to_string(),
                control: control_sender,
            },
        );
        let terminals = self.terminals.clone();
        let spawned_id = terminal_id.clone();
        tokio::spawn(async move {
            let mut channel = channel;
            loop {
                tokio::select! {
                    message = channel.wait() => {
                        let Some(message) = message else { break; };
                        let event = match message {
                            russh::ChannelMsg::Data { data } => TerminalEvent::Data {
                                data: base64::Engine::encode(
                                    &base64::engine::general_purpose::STANDARD,
                                    data,
                                ),
                            },
                            russh::ChannelMsg::ExtendedData { data, .. } => TerminalEvent::Data {
                                data: base64::Engine::encode(
                                    &base64::engine::general_purpose::STANDARD,
                                    data,
                                ),
                            },
                            russh::ChannelMsg::ExitStatus { exit_status } => TerminalEvent::Exit { exit_status },
                            russh::ChannelMsg::Close | russh::ChannelMsg::Eof => break,
                            _ => continue,
                        };
                        if sender.send(event).await.is_err() { break; }
                    }
                    control = control_receiver.recv() => {
                        match control {
                            Some(TerminalControl::Data(data)) => {
                                if channel.data_bytes(data).await.is_err() { break; }
                            }
                            Some(TerminalControl::Resize { columns, rows }) => {
                                if channel.window_change(columns, rows, 0, 0).await.is_err() { break; }
                            }
                            Some(TerminalControl::Close) | None => {
                                let _ = channel.close().await;
                                break;
                            }
                        }
                    }
                }
            }
            terminals.remove(&spawned_id);
            let _ = sender.send(TerminalEvent::Closed).await;
        });
        Ok((terminal_id, receiver))
    }

    pub async fn write_terminal(&self, terminal_id: &str, data: &[u8]) -> AppResult<()> {
        let terminal = self
            .terminals
            .get(terminal_id)
            .map(|value| value.control.clone())
            .ok_or_else(|| AppError::new("TERMINAL_NOT_FOUND", "terminal", "终端会话已关闭"))?;
        terminal
            .send(TerminalControl::Data(data.to_vec()))
            .await
            .map_err(|_| AppError::new("TERMINAL_WRITE_FAILED", "terminal", "终端输入发送失败"))
    }

    pub async fn resize_terminal(
        &self,
        terminal_id: &str,
        columns: u32,
        rows: u32,
    ) -> AppResult<()> {
        let terminal = self
            .terminals
            .get(terminal_id)
            .map(|value| value.control.clone())
            .ok_or_else(|| AppError::new("TERMINAL_NOT_FOUND", "terminal", "终端会话已关闭"))?;
        terminal
            .send(TerminalControl::Resize {
                columns: columns.max(20),
                rows: rows.max(5),
            })
            .await
            .map_err(|_| AppError::new("TERMINAL_RESIZE_FAILED", "terminal", "终端尺寸更新失败"))
    }

    pub async fn close_terminal(&self, terminal_id: &str) -> AppResult<()> {
        if let Some((_, terminal)) = self.terminals.remove(terminal_id) {
            let _ = terminal.control.send(TerminalControl::Close).await;
        }
        Ok(())
    }

    pub async fn open_sftp(&self, server_id: &str) -> AppResult<russh_sftp::client::SftpSession> {
        let connection = self
            .connections
            .get(server_id)
            .map(|value| value.clone())
            .ok_or_else(|| {
                AppError::new("SSH_NOT_CONNECTED", "ssh", "服务器尚未连接").for_server(server_id)
            })?;
        let channel = connection
            .handle
            .channel_open_session()
            .await
            .map_err(|error| {
                AppError::new("SFTP_FAILED", "sftp", "无法创建 SFTP channel")
                    .details(error)
                    .for_server(server_id)
            })?;
        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|error| {
                AppError::new("SFTP_FAILED", "sftp", "远端拒绝启动 SFTP 子系统")
                    .details(error)
                    .for_server(server_id)
            })?;
        let session = russh_sftp::client::SftpSession::new(channel.into_stream())
            .await
            .map_err(|error| {
                AppError::new("SFTP_FAILED", "sftp", "SFTP 协议初始化失败")
                    .details(error)
                    .for_server(server_id)
            })?;
        session.set_timeout(30);
        Ok(session)
    }

    pub async fn execute(
        &self,
        server_id: &str,
        command: &str,
        timeout: Duration,
    ) -> AppResult<RemoteCommandResult> {
        self.execute_with_input(server_id, command, None, timeout)
            .await
    }

    pub async fn execute_privileged(
        &self,
        server_id: &str,
        command: &str,
        timeout: Duration,
    ) -> AppResult<RemoteCommandResult> {
        let record = self.repository.record(server_id).await?;
        match record.sudo_mode.as_str() {
            "passwordless" => {
                self.execute_with_input(
                    server_id,
                    &format!(
                        "sudo -n -- sh -c {}",
                        crate::security::shell_escape(command)
                    ),
                    None,
                    timeout,
                )
                .await
            }
            "password" => {
                let reference = record.sudo_secret_ref.as_deref().ok_or_else(|| {
                    AppError::new("SUDO_AUTH_FAILED", "privilege", "服务器未保存 sudo 密码")
                        .for_server(server_id)
                })?;
                let secret = self.repository.credential(reference)?;
                let mut input = zeroize::Zeroizing::new(secret.expose_secret().as_bytes().to_vec());
                input.push(b'\n');
                let result = self
                    .execute_with_input(
                        server_id,
                        &format!(
                            "sudo -S -p '' -- sh -c {}",
                            crate::security::shell_escape(command)
                        ),
                        Some(input.as_slice()),
                        timeout,
                    )
                    .await?;
                if result.exit_code != 0 && result.stderr.to_ascii_lowercase().contains("password")
                {
                    return Err(
                        AppError::new("SUDO_AUTH_FAILED", "privilege", "sudo 认证失败")
                            .for_server(server_id),
                    );
                }
                Ok(result)
            }
            _ => Err(
                AppError::new("SUDO_REQUIRED", "privilege", "该服务器档案未启用 sudo")
                    .for_server(server_id)
                    .suggestion("编辑服务器档案并选择 sudo 模式"),
            ),
        }
    }

    async fn execute_with_input(
        &self,
        server_id: &str,
        command: &str,
        input: Option<&[u8]>,
        timeout: Duration,
    ) -> AppResult<RemoteCommandResult> {
        let connection = self
            .connections
            .get(server_id)
            .map(|value| value.clone())
            .ok_or_else(|| {
                AppError::new("SSH_NOT_CONNECTED", "ssh", "服务器尚未连接").for_server(server_id)
            })?;
        let _permit =
            connection.limiter.acquire().await.map_err(|_| {
                AppError::new("CANCELLED", "task", "命令已取消").for_server(server_id)
            })?;
        let started = Instant::now();
        let future = async {
            let mut channel = connection
                .handle
                .channel_open_session()
                .await
                .map_err(|error| {
                    AppError::new("SSH_CHANNEL_FAILED", "ssh", "无法创建 SSH channel")
                        .details(error)
                        .for_server(server_id)
                })?;
            channel
                .exec(true, format!("LC_ALL=C {}", command))
                .await
                .map_err(|error| {
                    AppError::new(
                        "REMOTE_COMMAND_FAILED",
                        "remote_command",
                        "远程命令启动失败",
                    )
                    .details(error)
                    .for_server(server_id)
                })?;
            if let Some(input) = input {
                channel.data_bytes(input.to_vec()).await.map_err(|error| {
                    AppError::new(
                        "REMOTE_COMMAND_FAILED",
                        "remote_command",
                        "远程标准输入发送失败",
                    )
                    .details(error)
                    .for_server(server_id)
                })?;
                channel.eof().await.map_err(|error| {
                    AppError::new(
                        "REMOTE_COMMAND_FAILED",
                        "remote_command",
                        "远程标准输入关闭失败",
                    )
                    .details(error)
                    .for_server(server_id)
                })?;
            }
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let mut exit_code = None;
            while let Some(message) = channel.wait().await {
                match message {
                    russh::ChannelMsg::Data { data } => stdout.extend_from_slice(&data),
                    russh::ChannelMsg::ExtendedData { data, ext: 1 } => {
                        stderr.extend_from_slice(&data)
                    }
                    russh::ChannelMsg::ExitStatus { exit_status } => exit_code = Some(exit_status),
                    _ => {}
                }
            }
            Ok(RemoteCommandResult {
                exit_code: exit_code.unwrap_or(255),
                stdout: String::from_utf8_lossy(&stdout).into_owned(),
                stderr: String::from_utf8_lossy(&stderr).into_owned(),
                duration_ms: started.elapsed().as_millis() as u64,
            })
        };
        tokio::time::timeout(timeout, future).await.map_err(|_| {
            AppError::new("COMMAND_TIMEOUT", "remote_command", "远程命令执行超时")
                .for_server(server_id)
        })?
    }
}

#[derive(Clone)]
struct ManagedTerminal {
    server_id: String,
    control: mpsc::Sender<TerminalControl>,
}

enum TerminalControl {
    Data(Vec<u8>),
    Resize { columns: u32, rows: u32 },
    Close,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum TerminalEvent {
    Data { data: String },
    Exit { exit_status: u32 },
    Closed,
}

impl PendingHostKey {
    fn challenge(&self) -> HostKeyChallenge {
        HostKeyChallenge {
            server_id: self.server_id.clone(),
            host: self.host.clone(),
            port: self.port,
            key_type: self.key_type.clone(),
            fingerprint: self.fingerprint.clone(),
        }
    }
}

#[derive(Debug, Clone)]
enum HostKeyObservation {
    Unknown(ObservedKey),
    Changed,
}

#[derive(Debug, Clone)]
struct ObservedKey {
    key_type: String,
    fingerprint: String,
    public_key: String,
}

#[derive(Clone)]
struct HostKeyHandler {
    expected: Option<KnownHost>,
    observation: Arc<Mutex<Option<HostKeyObservation>>>,
}

impl client::Handler for HostKeyHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        let public_key = server_public_key.to_openssh().map_err(russh::Error::from)?;
        if let Some(expected) = &self.expected {
            if expected.public_key == public_key {
                return Ok(true);
            }
            if let Ok(mut observation) = self.observation.lock() {
                *observation = Some(HostKeyObservation::Changed);
            }
            return Ok(false);
        }
        let value = ObservedKey {
            key_type: server_public_key.algorithm().as_str().to_string(),
            fingerprint: server_public_key.fingerprint(HashAlg::Sha256).to_string(),
            public_key,
        };
        if let Ok(mut observation) = self.observation.lock() {
            *observation = Some(HostKeyObservation::Unknown(value));
        }
        Ok(false)
    }
}

async fn authenticate(
    handle: &mut client::Handle<HostKeyHandler>,
    record: &ServerRecord,
    repository: &ServerRepository,
) -> AppResult<()> {
    let authenticated = match record.auth_type.as_str() {
        "password" => {
            let reference = record.password_secret_ref.as_deref().ok_or_else(|| {
                AppError::new("SSH_AUTH_FAILED", "authentication", "服务器未保存 SSH 密码")
            })?;
            let secret = repository.credential(reference)?;
            handle
                .authenticate_password(&record.username, secret.expose_secret())
                .await
                .map_err(|error| {
                    AppError::new("SSH_AUTH_FAILED", "authentication", "SSH 密码认证失败")
                        .details(error)
                        .for_server(&record.id)
                })?
                .success()
        }
        "private_key" => {
            let path = record.private_key_path.as_deref().ok_or_else(|| {
                AppError::new("SSH_AUTH_FAILED", "authentication", "服务器未配置私钥路径")
            })?;
            let passphrase = record
                .key_passphrase_secret_ref
                .as_deref()
                .map(|reference| repository.credential(reference))
                .transpose()?;
            let key = russh::keys::load_secret_key(
                path,
                passphrase.as_ref().map(|value| value.expose_secret()),
            )
            .map_err(|error| {
                AppError::new("SSH_AUTH_FAILED", "authentication", "无法读取 SSH 私钥")
                    .details(error)
                    .for_server(&record.id)
            })?;
            let hash_algorithm = handle
                .best_supported_rsa_hash()
                .await
                .map_err(|error| {
                    AppError::new("SSH_AUTH_FAILED", "authentication", "无法协商 RSA 签名算法")
                        .details(error)
                        .for_server(&record.id)
                })?
                .flatten();
            let key = russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key), hash_algorithm);
            handle
                .authenticate_publickey(&record.username, key)
                .await
                .map_err(|error| {
                    AppError::new("SSH_AUTH_FAILED", "authentication", "SSH 私钥认证失败")
                        .details(error)
                        .for_server(&record.id)
                })?
                .success()
        }
        _ => {
            return Err(AppError::new(
                "SSH_AGENT_UNAVAILABLE",
                "authentication",
                "当前环境尚未提供 SSH Agent",
            )
            .for_server(&record.id))
        }
    };
    if !authenticated {
        return Err(
            AppError::new("SSH_AUTH_FAILED", "authentication", "SSH 认证被服务器拒绝")
                .for_server(&record.id)
                .suggestion("检查用户名、认证方式与凭据"),
        );
    }
    Ok(())
}

fn map_connect_error(error: russh::Error, server_id: &str) -> AppError {
    AppError::new("SSH_CONNECT_FAILED", "ssh", "无法建立 SSH 连接")
        .details(error)
        .for_server(server_id)
        .suggestion("检查 SSH 服务、端口、防火墙与支持的密钥算法")
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteCommandResult {
    pub exit_code: u32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}
