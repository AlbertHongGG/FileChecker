pub mod commands;
pub mod core;
pub mod error;
pub mod models;
pub mod platform;
pub mod state;

use state::SessionManager;
use std::sync::Arc;

pub fn run() {
    let session_manager = Arc::new(SessionManager::new());

    tauri::Builder::default()
        .manage(session_manager)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::compare_directories,
            commands::cancel_comparison,
            commands::get_file_content_pair,
            commands::execute_sync,
            commands::reveal_in_explorer
        ])
        .run(tauri::generate_context!())
        .expect("error while running filechecker application");
}
