use crate::core::diff::DiffComparator;
use crate::core::scanner::ntfs::NtfsMftEngine;
use crate::core::scanner::win32_engine::Win32LargeFetchEngine;
use crate::core::scanner::{ScanEngine, ScanObserver, ScanPolicy};
use crate::error::AppError;
use crate::models::{ScanProgressPayload, UnifiedEntry};
use crate::platform::{probe_volume, EngineKind, MediaType};
use crate::state::SessionManager;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
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
    let mft_store = session.mft_store();
    let win = window.clone();

    tokio::task::spawn_blocking(move || {
        let path_a = PathBuf::from(&dir_a);
        let path_b = PathBuf::from(&dir_b);

        if !path_a.exists() {
            return Err(AppError::DirectoryNotFound(format!(
                "Directory A does not exist: {}",
                dir_a
            )));
        }
        if !path_b.exists() {
            return Err(AppError::DirectoryNotFound(format!(
                "Directory B does not exist: {}",
                dir_b
            )));
        }

        let cap_a = probe_volume(&path_a);
        let cap_b = probe_volume(&path_b);

        let engine_kind_a = cap_a.recommended_engine();
        let engine_kind_b = cap_b.recommended_engine();

        let engine_str_a = cap_a.engine_badge().label();
        let engine_str_b = cap_b.engine_badge().label();

        let is_hdd_a = cap_a.media_type == MediaType::Hdd;
        let is_hdd_b = cap_b.media_type == MediaType::Hdd;

        let engine_a: Box<dyn ScanEngine> = match engine_kind_a {
            EngineKind::NtfsMft => Box::new(NtfsMftEngine::new(mft_store.clone(), is_hdd_a)),
            EngineKind::Win32LargeFetchSsd => Box::new(Win32LargeFetchEngine::new(false)),
            EngineKind::Win32LargeFetchHdd => Box::new(Win32LargeFetchEngine::new(true)),
        };

        let engine_b: Box<dyn ScanEngine> = match engine_kind_b {
            EngineKind::NtfsMft => Box::new(NtfsMftEngine::new(mft_store.clone(), is_hdd_b)),
            EngineKind::Win32LargeFetchSsd => Box::new(Win32LargeFetchEngine::new(false)),
            EngineKind::Win32LargeFetchHdd => Box::new(Win32LargeFetchEngine::new(true)),
        };

        let observer_a = Arc::new(ScanObserver::new(cancel_token.clone()));
        let observer_b = Arc::new(ScanObserver::new(cancel_token.clone()));

        // Background progress ticker
        let win_ticker = win.clone();
        let cancel_ticker = cancel_token.clone();
        let obs_a_clone = observer_a.clone();
        let obs_b_clone = observer_b.clone();
        let eng_a_name = engine_str_a.to_string();
        let eng_b_name = engine_str_b.to_string();

        let ticker_handle = std::thread::spawn(move || {
            while !cancel_ticker.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(50));
                let side_a = obs_a_clone.snapshot(&eng_a_name, is_hdd_a);
                let side_b = obs_b_clone.snapshot(&eng_b_name, is_hdd_b);

                let _ = win_ticker.emit(
                    "scan-progress",
                    ScanProgressPayload {
                        side_a,
                        side_b,
                    },
                );
            }
        });

        let policy = ScanPolicy::new(prune_ignored);

        // Smart scheduling: if both dirs reside on the same physical HDD and not using MFT,
        // serialize scanning to prevent head thrashing. Otherwise, scan concurrently.
        let same_physical_hdd = is_hdd_a
            && is_hdd_b
            && cap_a.disk_number.is_some()
            && cap_a.disk_number == cap_b.disk_number;

        let should_serialize = same_physical_hdd
            && (engine_kind_a != EngineKind::NtfsMft || engine_kind_b != EngineKind::NtfsMft);

        let (res_a, res_b) = if should_serialize {
            let res_a = engine_a.scan(&path_a, &policy, &observer_a);
            let res_b = if res_a.is_ok() {
                engine_b.scan(&path_b, &policy, &observer_b)
            } else {
                Err(AppError::ScanCancelled)
            };
            (res_a, res_b)
        } else {
            rayon::join(
                || engine_a.scan(&path_a, &policy, &observer_a),
                || engine_b.scan(&path_b, &policy, &observer_b),
            )
        };

        // Stop progress ticker
        cancel_token.store(true, Ordering::Relaxed);
        let _ = ticker_handle.join();

        let map_a = res_a?;
        let map_b = res_b?;

        // Zero-copy diff computation
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
