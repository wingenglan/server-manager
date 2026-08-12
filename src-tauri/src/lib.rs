#![allow(clippy::result_large_err)]

mod app;
mod commands;
mod domain;
mod errors;
mod infra;
mod security;

use tauri::Manager;
use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .without_time()
        .try_init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = tauri::async_runtime::block_on(app::AppState::initialize(app.handle()))?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_servers,
            commands::get_server,
            commands::save_server,
            commands::delete_server,
            commands::connection_state,
            commands::connect_server,
            commands::trust_host_key,
            commands::disconnect_server,
            commands::get_system_overview,
            commands::open_terminal,
            commands::write_terminal,
            commands::resize_terminal,
            commands::close_terminal,
            commands::list_remote_directory,
            commands::read_remote_text,
            commands::save_remote_text,
            commands::save_remote_text_privileged,
            commands::create_remote_entry,
            commands::rename_remote_entry,
            commands::remove_remote_entry,
            commands::upload_remote,
            commands::download_remote,
            commands::cancel_transfer,
            commands::get_operations,
            commands::terminate_process,
            commands::manage_service,
        ])
        .run(tauri::generate_context!())
        .expect("fatal application bootstrap error");
}
