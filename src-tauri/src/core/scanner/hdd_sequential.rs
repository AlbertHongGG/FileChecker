use crate::error::{AppError, AppResult};
use crate::models::FileMetadata;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

pub fn scan_hdd_sequential(
    current_dir: &Path,
    base_path: &Path,
    prune_set: &Option<HashSet<&'static str>>,
    cancel_token: &Arc<AtomicBool>,
    counter: &Arc<AtomicUsize>,
    map: &mut HashMap<String, FileMetadata>,
) -> AppResult<()> {
    if cancel_token.load(Ordering::Relaxed) {
        return Err(AppError::ScanCancelled);
    }

    let entries = match fs::read_dir(current_dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry_res in entries {
        if cancel_token.load(Ordering::Relaxed) {
            return Err(AppError::ScanCancelled);
        }

        let entry = match entry_res {
            Ok(e) => e,
            Err(_) => continue,
        };

        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();
        let is_dir = file_type.is_dir();

        // Directory level pruning (skips entire subtrees)
        if is_dir {
            if let Some(ref set) = prune_set {
                if set.contains(name_str.as_ref()) {
                    continue;
                }
            }
        }

        let path = entry.path();
        if let Ok(rel) = path.strip_prefix(base_path) {
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            if !rel_str.is_empty() {
                let meta = entry.metadata().ok();
                let size = if is_dir {
                    0
                } else {
                    meta.as_ref().map(|m| m.len()).unwrap_or(0)
                };
                let modified_timestamp = meta
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);

                map.insert(
                    rel_str,
                    FileMetadata {
                        size,
                        modified_timestamp,
                        is_dir,
                    },
                );

                counter.fetch_add(1, Ordering::Relaxed);
            }
        }

        if is_dir {
            scan_hdd_sequential(&path, base_path, prune_set, cancel_token, counter, map)?;
        }
    }

    Ok(())
}
