use crate::core::diff::DiffComparator;
use crate::core::scanner::scan_directory_adaptive;
use crate::error::AppError;
use crate::models::{ScanProgressPayload, UnifiedEntry};
use crate::platform::is_slow_or_hdd;
use crate::state::SessionManager;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{Emitter, State, Window};

#[tauri::command]
pub async fn compare_directories(
    dir_a: String,
    dir_b: String,
    prune_ignored: bool,
    window: Window,
    session: State<'_, Arc<SessionManager>>,
) -> Result<Vec<UnifiedEntry>, AppError> {
    let cancel_token = session.new_token();
    let win = window.clone();

    tokio::task::spawn_blocking(move || {
        let path_a = PathBuf::from(&dir_a);
        let path_b = PathBuf::from(&dir_b);

        if !path_a.exists() {
            return Err(AppError::DirectoryNotFound(format!("Directory A does not exist: {}", dir_a)));
        }
        if !path_b.exists() {
            return Err(AppError::DirectoryNotFound(format!("Directory B does not exist: {}", dir_b)));
        }

        // Hardware characteristics detection
        let is_hdd_a = is_slow_or_hdd(&path_a);
        let is_hdd_b = is_slow_or_hdd(&path_b);

        let counter_a = Arc::new(AtomicUsize::new(0));
        let counter_b = Arc::new(AtomicUsize::new(0));

        // Background progress ticker
        let win_ticker = win.clone();
        let cancel_ticker = cancel_token.clone();
        let c_a = counter_a.clone();
        let c_b = counter_b.clone();

        let ticker_handle = std::thread::spawn(move || {
            while !cancel_ticker.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(60));
                let count_a = c_a.load(Ordering::Relaxed);
                let count_b = c_b.load(Ordering::Relaxed);
                let _ = win_ticker.emit(
                    "scan-progress",
                    ScanProgressPayload {
                        count_a,
                        count_b,
                        is_hdd_a,
                        is_hdd_b,
                        phase: "Scanning".to_string(),
                    },
                );
            }
        });

        // Dual concurrency: scan A and B in parallel via Rayon
        let token_a = cancel_token.clone();
        let token_b = cancel_token.clone();
        let p_a = path_a.clone();
        let p_b = path_b.clone();
        let cnt_a = counter_a.clone();
        let cnt_b = counter_b.clone();

        let (res_a, res_b) = rayon::join(
            move || scan_directory_adaptive(&p_a, is_hdd_a, prune_ignored, &token_a, &cnt_a),
            move || scan_directory_adaptive(&p_b, is_hdd_b, prune_ignored, &token_b, &cnt_b),
        );

        // Stop progress ticker
        cancel_token.store(true, Ordering::Relaxed);
        let _ = ticker_handle.join();

        let map_a = res_a?;
        let map_b = res_b?;

        // Diff computation
        let entries = DiffComparator::compare(map_a, map_b);
        Ok(entries)
    })
    .await
    .map_err(|e| AppError::TaskExecution(e.to_string()))?
}

#[tauri::command]
pub fn cancel_comparison(session: State<'_, Arc<SessionManager>>) {
    session.cancel_current();
}
