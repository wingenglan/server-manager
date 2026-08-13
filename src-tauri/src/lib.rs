#![allow(clippy::result_large_err)]

mod app;
mod commands;
mod domain;
mod errors;
mod infra;
mod security;

use tauri::Manager;
use tracing_subscriber::EnvFilter;

/** 初始化每次启动覆盖的本地应用日志文件；无法创建日志文件时回退到标准错误输出。 */
fn initialize_logging(app: &tauri::AppHandle) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let log_file = app.path().app_log_dir().ok().and_then(|directory| {
        std::fs::create_dir_all(&directory).ok()?;
        let path = directory.join("relay.log");
        std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .ok()
    });
    if let Some(file) = log_file {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .with_ansi(false)
            .without_time()
            .with_writer(file)
            .try_init();
    } else {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .without_time()
            .try_init();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            initialize_logging(app.handle());
            let state = tauri::async_runtime::block_on(app::AppState::initialize(app.handle()))?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_servers,
            commands::get_server,
            commands::list_server_groups,
            commands::create_server_group,
            commands::save_server,
            commands::duplicate_server,
            commands::delete_server,
            commands::connection_state,
            commands::connect_server,
            commands::reconnect_server,
            commands::trust_host_key,
            commands::disconnect_server,
            commands::get_system_overview,
            commands::open_terminal,
            commands::write_terminal,
            commands::resize_terminal,
            commands::close_terminal,
            commands::list_remote_directory,
            commands::read_remote_text,
            commands::read_remote_image_preview,
            commands::read_remote_tail,
            commands::save_remote_text,
            commands::save_remote_text_privileged,
            commands::create_remote_entry,
            commands::rename_remote_entry,
            commands::remove_remote_entry,
            commands::chmod_remote,
            commands::create_remote_symlink,
            commands::copy_move_remote,
            commands::upload_remote,
            commands::download_remote,
            commands::cancel_transfer,
            commands::cancel_command_task,
            commands::get_operations,
            commands::terminate_process,
            commands::manage_service,
            commands::get_service_detail,
            commands::get_service_logs,
            commands::export_servers,
            commands::import_servers,
            commands::export_diagnostics,
            commands::list_audit_events,
            commands::export_full_backup,
            commands::import_full_backup,
            commands::list_tools,
            commands::get_tool_install_plan,
            commands::install_tool,
            commands::get_nginx,
            commands::test_nginx_config,
            commands::probe_nginx_backend,
            commands::save_nginx_proxy,
            commands::get_docker,
            commands::docker_container_action,
            commands::docker_container_logs,
            commands::docker_container_inspect,
            commands::docker_container_stats,
            commands::docker_container_top,
            commands::docker_container_exec,
            commands::docker_container_follow_logs,
            commands::docker_resource_action,
            commands::docker_image_action,
            commands::docker_resource_inspect,
            commands::docker_compose_action,
            commands::docker_compose_save_yaml,
            commands::docker_compose_details,
            commands::docker_compose_logs,
            commands::docker_pull_image,
            commands::docker_run_container,
        ])
        .run(tauri::generate_context!())
        .expect("fatal application bootstrap error");
}
