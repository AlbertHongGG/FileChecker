use crate::core::sync::SyncExecutor;
use crate::error::AppError;
use crate::models::{SyncProgressPayload, SyncTask};
use std::path::PathBuf;
use tauri::{Emitter, Window};

#[tauri::command]
pub async fn execute_sync(
    dir_a: String,
    dir_b: String,
    tasks: Vec<SyncTask>,
    window: Window,
) -> Result<usize, AppError> {
    tokio::task::spawn_blocking(move || {
        let root_a = PathBuf::from(&dir_a);
        let root_b = PathBuf::from(&dir_b);
        let win = window.clone();

        SyncExecutor::execute(&root_a, &root_b, tasks, move |current, total, current_file| {
            let _ = win.emit(
                "sync-progress",
                SyncProgressPayload {
                    current,
                    total,
                    current_file: current_file.to_string(),
                },
            );
        })
    })
    .await
    .map_err(|e| AppError::TaskExecution(e.to_string()))?
}
