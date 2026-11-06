use crate::error::{AppError, AppResult};
use crate::models::FileMetadata;
use jwalk::WalkDir;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

pub fn scan_ssd_parallel(
    base_path: &Path,
    prune_set: Option<HashSet<&'static str>>,
    cancel_token: &Arc<AtomicBool>,
    counter: &Arc<AtomicUsize>,
) -> AppResult<HashMap<String, FileMetadata>> {
    let mut map = HashMap::new();
    if !base_path.exists() || !base_path.is_dir() {
        return Ok(map);
    }

    let walker = WalkDir::new(base_path)
        .skip_hidden(false)
        .process_read_dir(move |_depth, _path, _state, children| {
            if let Some(ref set) = prune_set {
                children.retain(|entry_res| {
                    if let Ok(entry) = entry_res {
                        if entry.file_type.is_dir() {
                            let name = entry.file_name.to_string_lossy();
                            return !set.contains(name.as_ref());
                        }
                    }
                    true
                });
            }
        });

    for entry_res in walker {
        if cancel_token.load(Ordering::Relaxed) {
            return Err(AppError::ScanCancelled);
        }

        if let Ok(entry) = entry_res {
            let path = entry.path();
            if path == base_path {
                continue;
            }

            if let Ok(rel) = path.strip_prefix(base_path) {
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                if !rel_str.is_empty() {
                    let is_dir = entry.file_type.is_dir();
                    let size = if is_dir {
                        0
                    } else {
                        entry.metadata().map(|m| m.len()).unwrap_or(0)
                    };
                    let modified_timestamp = entry
                        .metadata()
                        .ok()
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
        }
    }

    Ok(map)
}
