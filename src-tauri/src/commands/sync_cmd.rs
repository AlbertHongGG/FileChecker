use crate::core::sync::{SyncExecutionPlan, SyncExecutionStrategy, SyncExecutor, SyncObserver};
use crate::error::AppError;
use crate::models::SyncTask;
use crate::platform::probe_volume;
use crate::state::SessionManager;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
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
    let cancel_token = session.new_token();
    let mft_store = session.mft_store();
    let da_letter = dir_a.chars().next().map(|c| c.to_ascii_uppercase());
    let db_letter = dir_b.chars().next().map(|c| c.to_ascii_uppercase());
    let win = window.clone();

    tokio::task::spawn_blocking(move || {
        let root_a = PathBuf::from(&dir_a);
        let root_b = PathBuf::from(&dir_b);

        if !root_a.exists() {
            return Err(AppError::DirectoryNotFound(format!(
                "Directory A does not exist: {}",
                dir_a
            )));
        }
        if !root_b.exists() {
            return Err(AppError::DirectoryNotFound(format!(
                "Directory B does not exist: {}",
                dir_b
            )));
        }

        // Hardware-aware strategy determination
        let cap_a = probe_volume(&root_a);
        let cap_b = probe_volume(&root_b);
        let strategy = SyncExecutionStrategy::from_capabilities(&cap_a, &cap_b);

        // Compile optimized execution plan
        let plan = SyncExecutionPlan::compile(&root_a, &root_b, tasks)?;
        let total_tasks = plan.total_tasks;

        let observer = Arc::new(SyncObserver::new(total_tasks, cancel_token.clone()));

        // Background progress ticker (50ms rate-limited IPC to prevent WebView flooding)
        let win_ticker = win.clone();
        let cancel_ticker = cancel_token.clone();
        let observer_clone = observer.clone();

        let ticker_handle = std::thread::spawn(move || {
            while !cancel_ticker.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(50));
                let progress = observer_clone.snapshot();
                let _ = win_ticker.emit("sync-progress", progress);
            }
        });

        // Execute compiled plan
        let result = SyncExecutor::execute_plan(&plan, strategy, &observer);

        // Terminate progress ticker
        cancel_token.store(true, Ordering::Relaxed);
        let _ = ticker_handle.join();

        // Emit final 100% progress payload
        let _ = win.emit("sync-progress", observer.snapshot());

        let count = result?;

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
