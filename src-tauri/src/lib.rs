mod commands;
mod db;
mod models;
mod services;
mod terminal;

use services::stream_manager::StreamManager;
use terminal::ai_shell::AiShellManager;
use terminal::pty_manager::PtyManager;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::block_on(async {
                let pool = db::init(&handle).await.expect("database init failed");
                handle.manage(pool);
            });
            app.manage(StreamManager::new());
            app.manage(PtyManager::new());
            app.manage(AiShellManager::new());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::chat::send_message,
            commands::chat::cancel_stream,
            commands::chat::confirm_tool,
            commands::chat::regenerate,
            commands::sessions::create_session,
            commands::sessions::list_sessions,
            commands::sessions::rename_session,
            commands::sessions::delete_session,
            commands::messages::list_messages,
            commands::messages::delete_message,
            commands::messages::update_message,
            commands::messages::switch_version,
            commands::messages::get_message_versions,
            commands::model_config::create_model_config,
            commands::model_config::list_model_configs,
            commands::model_config::update_model_config,
            commands::model_config::delete_model_config,
            commands::model_config::fetch_model_list,
            commands::api_log::list_logs,
            commands::api_log::get_log_detail,
            commands::file_ops::save_avatar,
            commands::file_ops::get_avatar,
            commands::file_ops::delete_avatar,
            commands::file_ops::read_file_base64,
            commands::terminal::create_terminal,
            commands::terminal::write_to_terminal,
            commands::terminal::resize_terminal,
            commands::terminal::close_terminal,
            commands::web_search::create_search_config,
            commands::web_search::list_search_configs,
            commands::web_search::update_search_config,
            commands::web_search::delete_search_config,
            commands::keyring_ops::get_api_key,
            commands::keyring_ops::save_api_key,
            commands::keyring_ops::delete_api_key,
            commands::export::export_json,
            commands::export::export_markdown,
            commands::export::import_json,
            commands::export::import_markdown,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run app");
}
