use crate::app::AppState;
use crate::domain::files::{DirectoryListing, RemoteTextFile, SaveTextInput};
use crate::domain::metrics::SystemOverview;
use crate::domain::server::{SaveServerInput, ServerProfile};
use crate::domain::ssh::{ConnectOutcome, ConnectionSnapshot, TrustHostKeyInput};
use crate::errors::AppResult;
use tauri::{ipc::Channel, State};

#[tauri::command]
pub async fn list_servers(state: State<'_, AppState>) -> AppResult<Vec<ServerProfile>> {
    state.servers.list().await
}

#[tauri::command]
pub async fn get_server(server_id: String, state: State<'_, AppState>) -> AppResult<ServerProfile> {
    state.servers.get(&server_id).await
}

#[tauri::command]
pub async fn save_server(
    input: SaveServerInput,
    state: State<'_, AppState>,
) -> AppResult<ServerProfile> {
    state.servers.save(input).await
}

#[tauri::command]
pub async fn delete_server(server_id: String, state: State<'_, AppState>) -> AppResult<()> {
    state.ssh.disconnect(&server_id).await?;
    state.servers.delete(&server_id).await
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
    state.ssh.connect(&server_id).await
}

#[tauri::command]
pub async fn trust_host_key(
    challenge: TrustHostKeyInput,
    state: State<'_, AppState>,
) -> AppResult<ConnectionSnapshot> {
    state.ssh.trust(challenge).await
}

#[tauri::command]
pub async fn disconnect_server(server_id: String, state: State<'_, AppState>) -> AppResult<()> {
    state.ssh.disconnect(&server_id).await
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

#[tauri::command]
pub async fn save_remote_text(
    input: SaveTextInput,
    state: State<'_, AppState>,
) -> AppResult<RemoteTextFile> {
    crate::domain::files::save_text(&state.ssh, input).await
}

#[tauri::command]
pub async fn save_remote_text_privileged(
    input: SaveTextInput,
    state: State<'_, AppState>,
) -> AppResult<RemoteTextFile> {
    crate::domain::files::save_text_privileged(&state.ssh, input).await
}

#[tauri::command]
pub async fn create_remote_entry(
    server_id: String,
    path: String,
    directory: bool,
    state: State<'_, AppState>,
) -> AppResult<()> {
    crate::domain::files::create(&state.ssh, &server_id, &path, directory).await
}

#[tauri::command]
pub async fn rename_remote_entry(
    server_id: String,
    old_path: String,
    new_path: String,
    state: State<'_, AppState>,
) -> AppResult<()> {
    crate::domain::files::rename(&state.ssh, &server_id, &old_path, &new_path).await
}

#[tauri::command]
pub async fn remove_remote_entry(
    server_id: String,
    path: String,
    recursive: bool,
    state: State<'_, AppState>,
) -> AppResult<()> {
    crate::domain::files::remove(&state.ssh, &server_id, &path, recursive).await
}

#[tauri::command]
pub async fn upload_remote(
    transfer_id: String,
    server_id: String,
    local_path: String,
    remote_directory: String,
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

#[tauri::command]
pub async fn get_operations(
    server_id: String,
    state: State<'_, AppState>,
) -> AppResult<crate::domain::operations::OperationsSnapshot> {
    crate::domain::operations::snapshot(&state.ssh, &server_id).await
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
    crate::domain::operations::service_action(&state.ssh, &server_id, &service, &action).await
}
