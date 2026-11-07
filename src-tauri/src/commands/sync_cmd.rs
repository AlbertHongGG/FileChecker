use crate::core::sync::SyncExecutor;
use crate::error::AppError;
use crate::models::{SyncProgressPayload, SyncTask};
use crate::state::SessionManager;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Emitter, State, Window};

#[tauri::command]
pub async fn execute_sync(
    dir_a: String,
    dir_b: String,
    tasks: Vec<SyncTask>,
    window: Window,
    session: State<'_, Arc<SessionManager>>,
) -> Result<usize, AppError> {
    let mft_store = session.mft_store();
    let da_letter = dir_a.chars().next().map(|c| c.to_ascii_uppercase());
    let db_letter = dir_b.chars().next().map(|c| c.to_ascii_uppercase());

    tokio::task::spawn_blocking(move || {
        let root_a = PathBuf::from(&dir_a);
        let root_b = PathBuf::from(&dir_b);
        let win = window.clone();

        let count = SyncExecutor::execute(&root_a, &root_b, tasks, move |current, total, current_file| {
            let _ = win.emit(
                "sync-progress",
                SyncProgressPayload {
                    current,
                    total,
                    current_file: current_file.to_string(),
                },
            );
        })?;

        // Invalidate MFT caches for modified drives
        if let Some(da) = da_letter {
            mft_store.invalidate_drive(da);
        }
        if let Some(db) = db_letter {
            mft_store.invalidate_drive(db);
        }

        Ok(count)
    })
    .await
    .map_err(|e| AppError::TaskExecution(e.to_string()))?
}
