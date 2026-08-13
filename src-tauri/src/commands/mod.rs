use crate::app::AppState;
use crate::domain::files::{DirectoryListing, RemoteTextFile, SaveTextInput};
use crate::domain::metrics::SystemOverview;
use crate::domain::server::{SaveServerInput, ServerProfile};
use crate::domain::ssh::{ConnectOutcome, ConnectionSnapshot, TrustHostKeyInput};
use crate::errors::AppResult;
use tauri::{ipc::Channel, State};

/// 尽力写入本地审计记录；审计失败不会掩盖已经完成的远端动作，但会写入应用日志。
async fn write_audit(
    state: &AppState,
    server_id: Option<&str>,
    action: &str,
    resource_type: &str,
    resource_id: Option<&str>,
    result: &str,
    summary: String,
) {
    if let Err(error) = state
        .servers
        .record_audit(
            server_id,
            action,
            resource_type,
            resource_id,
            result,
            &summary,
        )
        .await
    {
        tracing::warn!(error = %error, action, "写入本地审计记录失败");
    }
}

/// 将命令结果转换为成功/失败审计事件，避免业务错误被审计写入错误遮蔽。
async fn audit_outcome<T>(
    state: &AppState,
    server_id: Option<&str>,
    action: &str,
    resource_type: &str,
    resource_id: Option<&str>,
    outcome: &AppResult<T>,
    success_summary: String,
) {
    match outcome {
        Ok(_) => {
            write_audit(
                state,
                server_id,
                action,
                resource_type,
                resource_id,
                "success",
                success_summary,
            )
            .await
        }
        Err(error) => {
            write_audit(
                state,
                server_id,
                action,
                resource_type,
                resource_id,
                "failed",
                format!("{}：{}", success_summary, error.code),
            )
            .await
        }
    }
}

#[tauri::command]
pub async fn list_servers(state: State<'_, AppState>) -> AppResult<Vec<ServerProfile>> {
    state.servers.list().await
}

#[tauri::command]
pub async fn get_server(server_id: String, state: State<'_, AppState>) -> AppResult<ServerProfile> {
    state.servers.get(&server_id).await
}

/// 列出本地服务器分组。
#[tauri::command]
pub async fn list_server_groups(
    state: State<'_, AppState>,
) -> AppResult<Vec<crate::domain::server::ServerGroup>> {
    state.servers.list_groups().await
}

/// 创建本地服务器分组。
#[tauri::command]
pub async fn create_server_group(
    name: String,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::server::ServerGroup> {
    state.servers.create_group(name).await
}

#[tauri::command]
pub async fn save_server(
    input: SaveServerInput,
    state: State<'_, AppState>,
) -> AppResult<ServerProfile> {
    let action = if input.id.is_some() {
        "update_server"
    } else {
        "create_server"
    };
    let profile = state.servers.save(input).await?;
    write_audit(
        &state,
        Some(&profile.id),
        action,
        "server",
        Some(&profile.id),
        "success",
        format!(
            "服务器档案已{}：{}",
            if action == "create_server" {
                "创建"
            } else {
                "更新"
            },
            profile.name
        ),
    )
    .await;
    Ok(profile)
}

/// 复制服务器公共配置并打开新档案；系统凭据不会复制到副本。
#[tauri::command]
pub async fn duplicate_server(
    server_id: String,
    state: State<'_, AppState>,
) -> AppResult<ServerProfile> {
    let profile = state.servers.duplicate(&server_id).await?;
    write_audit(
        &state,
        Some(&profile.id),
        "duplicate_server",
        "server",
        Some(&profile.id),
        "success",
        format!("已从服务器档案复制副本：{}", profile.name),
    )
    .await;
    Ok(profile)
}

#[tauri::command]
pub async fn delete_server(server_id: String, state: State<'_, AppState>) -> AppResult<()> {
    state.ssh.disconnect(&server_id).await?;
    state.servers.delete(&server_id).await?;
    write_audit(
        &state,
        None,
        "delete_server",
        "server",
        Some(&server_id),
        "success",
        "服务器档案已删除；远端未被修改".into(),
    )
    .await;
    Ok(())
}

#[tauri::command]
pub async fn connection_state(
    server_id: String,
    state: State<'_, AppState>,
) -> AppResult<ConnectionSnapshot> {
    state.servers.record(&server_id).await?;
    Ok(state.ssh.snapshot(&server_id))
}

#[tauri::command]
pub async fn connect_server(
    server_id: String,
    state: State<'_, AppState>,
) -> AppResult<ConnectOutcome> {
    let result = state.ssh.connect(&server_id).await;
    match &result {
        Ok(ConnectOutcome::Connected(_)) => {
            write_audit(
                &state,
                Some(&server_id),
                "connect_server",
                "connection",
                Some(&server_id),
                "success",
                "SSH 会话已建立".into(),
            )
            .await
        }
        Ok(ConnectOutcome::HostKey(_)) => {
            write_audit(
                &state,
                Some(&server_id),
                "connect_server",
                "connection",
                Some(&server_id),
                "pending",
                "等待用户核对 Host Key".into(),
            )
            .await
        }
        Err(error) => {
            write_audit(
                &state,
                Some(&server_id),
                "connect_server",
                "connection",
                Some(&server_id),
                "failed",
                format!("SSH 连接失败：{}", error.code),
            )
            .await
        }
    }
    result
}

/// 断开旧 SSH 会话并执行有限退避重连，审计结果与普通连接保持一致。
#[tauri::command]
pub async fn reconnect_server(
    server_id: String,
    state: State<'_, AppState>,
) -> AppResult<ConnectOutcome> {
    let result = state.ssh.reconnect(&server_id).await;
    match &result {
        Ok(ConnectOutcome::Connected(_)) => {
            write_audit(
                &state,
                Some(&server_id),
                "reconnect_server",
                "connection",
                Some(&server_id),
                "success",
                "SSH 会话已通过有限退避重连".into(),
            )
            .await
        }
        Ok(ConnectOutcome::HostKey(_)) => {
            write_audit(
                &state,
                Some(&server_id),
                "reconnect_server",
                "connection",
                Some(&server_id),
                "pending",
                "重连等待用户核对 Host Key".into(),
            )
            .await
        }
        Err(error) => {
            write_audit(
                &state,
                Some(&server_id),
                "reconnect_server",
                "connection",
                Some(&server_id),
                "failed",
                format!("SSH 重连失败：{}", error.code),
            )
            .await
        }
    }
    result
}

#[tauri::command]
pub async fn trust_host_key(
    challenge: TrustHostKeyInput,
    state: State<'_, AppState>,
) -> AppResult<ConnectionSnapshot> {
    let server_id = challenge.server_id.clone();
    let result = state.ssh.trust(challenge).await;
    match &result {
        Ok(_) => {
            write_audit(
                &state,
                Some(&server_id),
                "trust_host_key",
                "known_host",
                Some(&server_id),
                "success",
                "用户确认并信任 Host Key".into(),
            )
            .await
        }
        Err(error) => {
            write_audit(
                &state,
                Some(&server_id),
                "trust_host_key",
                "known_host",
                Some(&server_id),
                "failed",
                format!("Host Key 信任失败：{}", error.code),
            )
            .await
        }
    }
    result
}

#[tauri::command]
pub async fn disconnect_server(server_id: String, state: State<'_, AppState>) -> AppResult<()> {
    let result = state.ssh.disconnect(&server_id).await;
    match &result {
        Ok(_) => {
            write_audit(
                &state,
                Some(&server_id),
                "disconnect_server",
                "connection",
                Some(&server_id),
                "success",
                "SSH 会话已断开".into(),
            )
            .await
        }
        Err(error) => {
            write_audit(
                &state,
                Some(&server_id),
                "disconnect_server",
                "connection",
                Some(&server_id),
                "failed",
                format!("SSH 断开失败：{}", error.code),
            )
            .await
        }
    }
    result
}

#[tauri::command]
pub async fn get_system_overview(
    server_id: String,
    state: State<'_, AppState>,
) -> AppResult<SystemOverview> {
    crate::domain::metrics::probe(&state.ssh, &server_id).await
}

#[tauri::command]
pub async fn open_terminal(
    server_id: String,
    columns: u32,
    rows: u32,
    on_event: Channel<crate::domain::ssh::TerminalEvent>,
    state: State<'_, AppState>,
) -> AppResult<String> {
    let (terminal_id, mut events) = state.ssh.open_terminal(&server_id, columns, rows).await?;
    tauri::async_runtime::spawn(async move {
        while let Some(event) = events.recv().await {
            if on_event.send(event).is_err() {
                break;
            }
        }
    });
    Ok(terminal_id)
}

#[tauri::command]
pub async fn write_terminal(
    terminal_id: String,
    data: Vec<u8>,
    state: State<'_, AppState>,
) -> AppResult<()> {
    state.ssh.write_terminal(&terminal_id, &data).await
}

#[tauri::command]
pub async fn resize_terminal(
    terminal_id: String,
    columns: u32,
    rows: u32,
    state: State<'_, AppState>,
) -> AppResult<()> {
    state.ssh.resize_terminal(&terminal_id, columns, rows).await
}

#[tauri::command]
pub async fn close_terminal(terminal_id: String, state: State<'_, AppState>) -> AppResult<()> {
    state.ssh.close_terminal(&terminal_id).await
}

#[tauri::command]
pub async fn list_remote_directory(
    server_id: String,
    path: String,
    state: State<'_, AppState>,
) -> AppResult<DirectoryListing> {
    crate::domain::files::list(&state.ssh, &server_id, &path).await
}

#[tauri::command]
pub async fn read_remote_text(
    server_id: String,
    path: String,
    state: State<'_, AppState>,
) -> AppResult<RemoteTextFile> {
    crate::domain::files::read_text(&state.ssh, &server_id, &path).await
}

/// 读取远程图片预览数据，供文件页右侧预览面板使用。
#[tauri::command]
pub async fn read_remote_image_preview(
    server_id: String,
    path: String,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::files::RemoteBinaryPreview> {
    crate::domain::files::read_image_preview(&state.ssh, &server_id, &path).await
}

/// 读取大文件有限尾部供 Large File Viewer 展示，不将整文件载入本地。
#[tauri::command]
pub async fn read_remote_tail(
    server_id: String,
    path: String,
    lines: u32,
    state: State<'_, AppState>,
) -> AppResult<RemoteTextFile> {
    crate::domain::files::read_tail(&state.ssh, &server_id, &path, lines).await
}

#[tauri::command]
pub async fn save_remote_text(
    input: SaveTextInput,
    state: State<'_, AppState>,
) -> AppResult<RemoteTextFile> {
    let server_id = input.server_id.clone();
    let path = input.path.clone();
    let result = crate::domain::files::save_text(&state.ssh, input).await;
    audit_outcome(
        &state,
        Some(&server_id),
        "save_remote_text",
        "file",
        Some(&path),
        &result,
        format!("保存远程文件：{}", path),
    )
    .await;
    result
}

#[tauri::command]
pub async fn save_remote_text_privileged(
    input: SaveTextInput,
    state: State<'_, AppState>,
) -> AppResult<RemoteTextFile> {
    let server_id = input.server_id.clone();
    let path = input.path.clone();
    let result = crate::domain::files::save_text_privileged(&state.ssh, input).await;
    audit_outcome(
        &state,
        Some(&server_id),
        "save_remote_text_privileged",
        "file",
        Some(&path),
        &result,
        format!("sudo 保存远程文件：{}", path),
    )
    .await;
    result
}

#[tauri::command]
pub async fn create_remote_entry(
    server_id: String,
    path: String,
    directory: bool,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let result = crate::domain::files::create(&state.ssh, &server_id, &path, directory).await;
    audit_outcome(
        &state,
        Some(&server_id),
        "create_remote_entry",
        "file",
        Some(&path),
        &result,
        format!(
            "创建远程{}：{}",
            if directory { "目录" } else { "文件" },
            path
        ),
    )
    .await;
    result
}

#[tauri::command]
pub async fn rename_remote_entry(
    server_id: String,
    old_path: String,
    new_path: String,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let result = crate::domain::files::rename(&state.ssh, &server_id, &old_path, &new_path).await;
    audit_outcome(
        &state,
        Some(&server_id),
        "rename_remote_entry",
        "file",
        Some(&old_path),
        &result,
        format!("重命名远程对象为：{}", new_path),
    )
    .await;
    result
}

#[tauri::command]
pub async fn remove_remote_entry(
    server_id: String,
    path: String,
    recursive: bool,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let result = crate::domain::files::remove(&state.ssh, &server_id, &path, recursive).await;
    audit_outcome(
        &state,
        Some(&server_id),
        "remove_remote_entry",
        "file",
        Some(&path),
        &result,
        format!("删除远程对象：{}", path),
    )
    .await;
    result
}

/// 修改远程文件或文件夹的 Unix 权限，并验证 SFTP 元数据结果。
#[tauri::command]
pub async fn chmod_remote(
    input: crate::domain::files::ChmodInput,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let server_id = input.server_id.clone();
    let path = input.path.clone();
    let result = crate::domain::files::chmod(&state.ssh, input).await;
    audit_outcome(
        &state,
        Some(&server_id),
        "chmod_remote",
        "file",
        Some(&path),
        &result,
        format!("修改远程权限：{}", path),
    )
    .await;
    result
}

/// 创建远程符号链接并验证链接对象，不跟随目标执行写入。
#[tauri::command]
pub async fn create_remote_symlink(
    input: crate::domain::files::SymlinkInput,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let server_id = input.server_id.clone();
    let link_path = input.link_path.clone();
    let result = crate::domain::files::symlink(&state.ssh, input).await;
    audit_outcome(
        &state,
        Some(&server_id),
        "create_remote_symlink",
        "file",
        Some(&link_path),
        &result,
        format!("创建远程符号链接：{}", link_path),
    )
    .await;
    result
}

/// 在同一台远程服务器内部复制或移动文件，并由后端拒绝覆盖已存在的目标。
#[tauri::command]
pub async fn copy_move_remote(
    input: crate::domain::files::CopyMoveInput,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let server_id = input.server_id.clone();
    let source_path = input.source_path.clone();
    let result = crate::domain::files::copy_move(&state.ssh, input).await;
    audit_outcome(
        &state,
        Some(&server_id),
        "copy_move_remote",
        "file",
        Some(&source_path),
        &result,
        format!("复制或移动远程对象：{}", source_path),
    )
    .await;
    result
}

#[tauri::command]
pub async fn upload_remote(
    transfer_id: String,
    server_id: String,
    local_path: String,
    remote_directory: String,
    conflict: String,
    on_event: Channel<crate::domain::transfer::TransferEvent>,
    state: State<'_, AppState>,
) -> AppResult<()> {
    crate::domain::transfer::upload(
        &state.transfers,
        &state.ssh,
        &transfer_id,
        &server_id,
        &local_path,
        &remote_directory,
        &conflict,
        &on_event,
    )
    .await
}

#[tauri::command]
pub async fn download_remote(
    transfer_id: String,
    server_id: String,
    remote_path: String,
    local_directory: String,
    on_event: Channel<crate::domain::transfer::TransferEvent>,
    state: State<'_, AppState>,
) -> AppResult<()> {
    crate::domain::transfer::download(
        &state.transfers,
        &state.ssh,
        &transfer_id,
        &server_id,
        &remote_path,
        &local_directory,
        &on_event,
    )
    .await
}

#[tauri::command]
pub fn cancel_transfer(transfer_id: String, state: State<'_, AppState>) -> AppResult<()> {
    state.transfers.cancel(&transfer_id)
}

/// 取消一个正在执行的流式 SSH 命令，并关闭对应远端 channel。
#[tauri::command]
pub fn cancel_command_task(task_id: String, state: State<'_, AppState>) -> AppResult<()> {
    state.ssh.cancel_task(&task_id)
}

#[tauri::command]
pub async fn get_operations(
    server_id: String,
    privileged: bool,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::operations::OperationsSnapshot> {
    crate::domain::operations::snapshot(&state.ssh, &server_id, privileged).await
}

#[tauri::command]
pub async fn terminate_process(
    input: crate::domain::operations::TerminateProcessInput,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::operations::TerminationResult> {
    crate::domain::operations::terminate(&state.ssh, input).await
}

#[tauri::command]
pub async fn manage_service(
    server_id: String,
    service: String,
    action: String,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let result =
        crate::domain::operations::service_action(&state.ssh, &server_id, &service, &action).await;
    audit_outcome(
        &state,
        Some(&server_id),
        "manage_service",
        "systemd_service",
        Some(&service),
        &result,
        format!("服务 {}：{}", action, service),
    )
    .await;
    result
}

/// 查询 systemd 服务的详细状态和来源路径。
#[tauri::command]
pub async fn get_service_detail(
    server_id: String,
    service: String,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::operations::ServiceDetail> {
    crate::domain::operations::service_detail(&state.ssh, &server_id, &service).await
}

/// 查询 systemd 服务最近日志。
#[tauri::command]
pub async fn get_service_logs(
    server_id: String,
    service: String,
    lines: u32,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::operations::ServiceLogs> {
    crate::domain::operations::service_logs(&state.ssh, &server_id, &service, lines).await
}

/// 导出不含 secret 的服务器档案配置。
#[tauri::command]
pub async fn export_servers(
    state: State<'_, AppState>,
) -> AppResult<crate::domain::server::PublicServerExport> {
    state.servers.export_public().await
}

/// 导入公共服务器配置并为每条记录生成新的本地 ID。
#[tauri::command]
pub async fn import_servers(
    values: Vec<crate::domain::server::PublicServerImport>,
    state: State<'_, AppState>,
) -> AppResult<Vec<crate::domain::server::ServerProfile>> {
    state.servers.import_public(values).await
}

/// 导出本地档案、连接状态和最近审计元数据；响应不含凭据、命令输出或私钥内容。
#[tauri::command]
pub async fn export_diagnostics(
    state: State<'_, AppState>,
) -> AppResult<crate::domain::diagnostics::DiagnosticsExport> {
    let profiles = state.servers.list().await?;
    let connections = profiles
        .iter()
        .map(|profile| state.ssh.snapshot(&profile.id))
        .collect();
    let recent_audit = state.servers.list_audit(100).await?;
    Ok(crate::domain::diagnostics::DiagnosticsExport::build(
        profiles,
        connections,
        recent_audit,
    ))
}

/// 读取最近的本地审计事件，不返回任何远端命令输出。
#[tauri::command]
pub async fn list_audit_events(
    limit: u32,
    state: State<'_, AppState>,
) -> AppResult<Vec<crate::infra::db::AuditEvent>> {
    state.servers.list_audit(limit).await
}

/// 读取配置和系统凭据并输出 Argon2id/AES-256-GCM 加密备份文本。
#[tauri::command]
pub async fn export_full_backup(
    input: crate::domain::backup::ExportBackupInput,
    state: State<'_, AppState>,
) -> AppResult<String> {
    let payload = state.servers.export_backup().await?;
    crate::domain::backup::encrypt(&payload, &input.password)
}

/// 解密完整备份并将服务器档案和 secret 导入本地安全存储。
#[tauri::command]
pub async fn import_full_backup(
    input: crate::domain::backup::ImportBackupInput,
    state: State<'_, AppState>,
) -> AppResult<Vec<crate::domain::server::ServerProfile>> {
    let payload = crate::domain::backup::decrypt(&input.backup, &input.password)?;
    state.servers.import_backup(payload).await
}

/// 查询远端工具注册表的安装、版本和运行状态。
#[tauri::command]
pub async fn list_tools(
    server_id: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<crate::domain::tools::ToolStatus>> {
    crate::domain::tools::list(&state.ssh, &server_id).await
}

/// 返回用户确认前可展示的工具安装计划。
#[tauri::command]
pub async fn get_tool_install_plan(
    server_id: String,
    tool_id: String,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::tools::ToolInstallPlan> {
    crate::domain::tools::install_plan(&state.ssh, &server_id, &tool_id).await
}

/// 执行用户明确确认的工具安装，并验证安装结果。
#[tauri::command]
pub async fn install_tool(
    input: crate::domain::tools::InstallToolInput,
    on_event: Channel<crate::domain::ssh::CommandEvent>,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::tools::ToolInstallResult> {
    let server_id = input.server_id.clone();
    let tool_id = input.tool_id.clone();
    let result = crate::domain::tools::install(&state.ssh, input, &on_event).await;
    audit_outcome(
        &state,
        Some(&server_id),
        "install_tool",
        "tool",
        Some(&tool_id),
        &result,
        format!("安装工具：{}", tool_id),
    )
    .await;
    result
}

/// 查询 Nginx 真实配置摘要和反向代理 source mapping。
#[tauri::command]
pub async fn get_nginx(
    server_id: String,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::nginx::NginxSnapshot> {
    crate::domain::nginx::snapshot(&state.ssh, &server_id).await
}

/// 运行只读的 Nginx 配置语法检查。
#[tauri::command]
pub async fn test_nginx_config(server_id: String, state: State<'_, AppState>) -> AppResult<bool> {
    crate::domain::nginx::test_config(&state.ssh, &server_id).await
}

/// 从远端服务器探测 Nginx 代理目标的可达性和 HTTP 状态。
#[tauri::command]
pub async fn probe_nginx_backend(
    input: crate::domain::nginx::NginxBackendProbeInput,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::nginx::NginxBackendProbeResult> {
    crate::domain::nginx::probe_backend(&state.ssh, input).await
}

/// 写入受控 managed conf，失败时由 Rust 端恢复备份并阻止 reload。
#[tauri::command]
pub async fn save_nginx_proxy(
    input: crate::domain::nginx::NginxProxyInput,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::nginx::NginxSnapshot> {
    let server_id = input.server_id.clone();
    let name = input.name.clone();
    let result = crate::domain::nginx::save_proxy(&state.ssh, input).await;
    audit_outcome(
        &state,
        Some(&server_id),
        "save_nginx_proxy",
        "nginx_proxy",
        Some(&name),
        &result,
        format!("保存 Nginx 代理：{}", name),
    )
    .await;
    result
}

/// 查询远程 Docker Engine、容器和镜像列表。
#[tauri::command]
pub async fn get_docker(
    server_id: String,
    privileged: bool,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::docker::DockerSnapshot> {
    crate::domain::docker::snapshot(&state.ssh, &server_id, privileged).await
}

/// 执行已确认的 Docker 容器动作并验证状态。
#[tauri::command]
pub async fn docker_container_action(
    input: crate::domain::docker::DockerActionInput,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::docker::DockerActionResult> {
    let server_id = input.server_id.clone();
    let container_id = input.container_id.clone();
    let action = input.action.clone();
    let result = crate::domain::docker::action(&state.ssh, input).await;
    audit_outcome(
        &state,
        Some(&server_id),
        "docker_container_action",
        "docker_container",
        Some(&container_id),
        &result,
        format!("Docker 容器 {}：{}", action, container_id),
    )
    .await;
    result
}

/// 读取远端容器最近日志，不将日志写入本地数据库。
#[tauri::command]
pub async fn docker_container_logs(
    server_id: String,
    container_id: String,
    tail: u32,
    privileged: bool,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::docker::DockerLogs> {
    crate::domain::docker::logs(&state.ssh, &server_id, &container_id, tail, privileged).await
}

/// 读取单个容器的原始 inspect JSON。
#[tauri::command]
pub async fn docker_container_inspect(
    server_id: String,
    container_id: String,
    privileged: bool,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::docker::DockerTextResult> {
    crate::domain::docker::inspect(&state.ssh, &server_id, &container_id, privileged).await
}

/// 读取单个容器的一次性资源统计。
#[tauri::command]
pub async fn docker_container_stats(
    server_id: String,
    container_id: String,
    privileged: bool,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::docker::DockerTextResult> {
    crate::domain::docker::stats(&state.ssh, &server_id, &container_id, privileged).await
}

/// 读取单个容器内的进程列表。
#[tauri::command]
pub async fn docker_container_top(
    server_id: String,
    container_id: String,
    privileged: bool,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::docker::DockerTextResult> {
    crate::domain::docker::top(&state.ssh, &server_id, &container_id, privileged).await
}

/// 在容器内执行受控命令并返回一次性输出。
#[tauri::command]
pub async fn docker_container_exec(
    input: crate::domain::docker::DockerExecInput,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::docker::DockerTextResult> {
    crate::domain::docker::exec(&state.ssh, input).await
}

/// 跟随容器日志最多 30 秒，并通过 Channel 转发输出块。
#[tauri::command]
pub async fn docker_container_follow_logs(
    server_id: String,
    container_id: String,
    tail: u32,
    sudo: bool,
    task_id: String,
    on_event: Channel<crate::domain::ssh::CommandEvent>,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::docker::DockerLogs> {
    crate::domain::docker::follow_logs(
        &state.ssh,
        &server_id,
        &container_id,
        tail,
        sudo,
        &task_id,
        &on_event,
    )
    .await
}

/// 创建或删除 Docker volume/network，并返回远端 inspect 验证结果。
#[tauri::command]
pub async fn docker_resource_action(
    input: crate::domain::docker::DockerResourceActionInput,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::docker::DockerResourceActionResult> {
    let server_id = input.server_id.clone();
    let name = input.name.clone();
    let action = input.action.clone();
    let result = crate::domain::docker::resource_action(&state.ssh, input).await;
    audit_outcome(
        &state,
        Some(&server_id),
        "docker_resource_action",
        "docker_resource",
        Some(&name),
        &result,
        format!("Docker 资源 {}：{}", action, name),
    )
    .await;
    result
}

/// 执行已确认的 Docker 镜像删除，并记录不含镜像输出的本地审计事件。
#[tauri::command]
pub async fn docker_image_action(
    input: crate::domain::docker::DockerImageActionInput,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::docker::DockerResourceActionResult> {
    let server_id = input.server_id.clone();
    let image = input.image.clone();
    let result = crate::domain::docker::image_action(&state.ssh, input).await;
    audit_outcome(
        &state,
        Some(&server_id),
        "docker_image_action",
        "docker_image",
        Some(&image),
        &result,
        format!("Docker 镜像操作：{image}"),
    )
    .await;
    result
}

/// 读取 Docker volume/network inspect JSON，并保持结果只在当前 UI 响应中存在。
#[tauri::command]
pub async fn docker_resource_inspect(
    input: crate::domain::docker::DockerResourceInspectInput,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::docker::DockerTextResult> {
    crate::domain::docker::resource_inspect(&state.ssh, input).await
}

/// 执行 Compose 项目生命周期操作并验证项目列表。
#[tauri::command]
pub async fn docker_compose_action(
    input: crate::domain::docker::DockerComposeActionInput,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::docker::DockerResourceActionResult> {
    let server_id = input.server_id.clone();
    let project = input.project.clone();
    let action = input.action.clone();
    let result = crate::domain::docker::compose_action(&state.ssh, input).await;
    audit_outcome(
        &state,
        Some(&server_id),
        "docker_compose_action",
        "docker_compose",
        Some(&project),
        &result,
        format!("Compose 项目 {}：{}", action, project),
    )
    .await;
    result
}

/// 保存 Compose 原始 YAML，先执行 `docker compose config -q`，失败时自动恢复备份。
#[tauri::command]
pub async fn docker_compose_save_yaml(
    input: crate::domain::docker::DockerComposeYamlInput,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::files::RemoteTextFile> {
    let server_id = input.server_id.clone();
    let path = input.config_path.clone();
    let result = crate::domain::docker::save_compose_yaml(&state.ssh, input).await;
    audit_outcome(
        &state,
        Some(&server_id),
        "docker_compose_save_yaml",
        "docker_compose",
        Some(&path),
        &result,
        format!("保存 Compose YAML：{}", path),
    )
    .await;
    result
}

/// 读取 Compose 项目的服务、渲染配置和资源候选，不修改远端状态。
#[tauri::command]
pub async fn docker_compose_details(
    server_id: String,
    project: String,
    working_dir: Option<String>,
    sudo: bool,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::docker::DockerComposeDetails> {
    crate::domain::docker::compose_details(
        &state.ssh,
        &server_id,
        &project,
        working_dir.as_deref(),
        sudo,
    )
    .await
}

/// 读取 Compose 项目或单个服务的最近日志，不保存日志内容。
#[tauri::command]
pub async fn docker_compose_logs(
    server_id: String,
    project: String,
    working_dir: Option<String>,
    service: Option<String>,
    tail: u32,
    sudo: bool,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::docker::DockerLogs> {
    crate::domain::docker::compose_logs(
        &state.ssh,
        &server_id,
        &project,
        working_dir.as_deref(),
        service.as_deref(),
        tail,
        sudo,
    )
    .await
}

/// 拉取单个 Docker 镜像，并通过 Channel 转发 layer 输出。
#[tauri::command]
pub async fn docker_pull_image(
    input: crate::domain::docker::DockerPullInput,
    on_event: Channel<crate::domain::ssh::CommandEvent>,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::docker::DockerPullResult> {
    let server_id = input.server_id.clone();
    let image = input.image.clone();
    let result = crate::domain::docker::pull(&state.ssh, input, &on_event).await;
    audit_outcome(
        &state,
        Some(&server_id),
        "docker_pull_image",
        "docker_image",
        Some(&image),
        &result,
        format!("拉取 Docker 镜像：{}", image),
    )
    .await;
    result
}

/// 执行受控 Run Container 向导，并验证容器创建结果。
#[tauri::command]
pub async fn docker_run_container(
    input: crate::domain::docker::DockerRunInput,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::docker::DockerRunResult> {
    let server_id = input.server_id.clone();
    let image = input.image.clone();
    let result = crate::domain::docker::run(&state.ssh, input).await;
    audit_outcome(
        &state,
        Some(&server_id),
        "docker_run_container",
        "docker_container",
        None,
        &result,
        format!("运行 Docker 镜像：{}", image),
    )
    .await;
    result
}
