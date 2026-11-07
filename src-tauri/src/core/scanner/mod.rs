pub mod anchor;
pub mod hdd_sequential;
pub mod ssd_parallel;

use crate::error::{AppError, AppResult};
use crate::models::{AnchorItem, FileMetadata};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

pub fn get_default_prune_set() -> HashSet<&'static str> {
    let mut set = HashSet::new();
    set.insert("node_modules");
    set.insert("target");
    set.insert(".git");
    set.insert("dist");
    set.insert(".next");
    set.insert(".nuxt");
    set.insert(".venv");
    set.insert("venv");
    set.insert("__pycache__");
    set.insert(".idea");
    set.insert(".vscode");
    set.insert(".cache");
    set.insert("bin");
    set.insert("obj");
    set.insert(".turbo");
    set.insert(".output");
    set.insert(".gradle");
    set
}

pub fn scan_anchor_guided(
    base_path: &Path,
    anchors: &[AnchorItem],
    is_hdd: bool,
    prune_ignored: bool,
    cancel_token: &Arc<AtomicBool>,
    completed: &Arc<AtomicUsize>,
    current_name: &Arc<Mutex<String>>,
    counter: &Arc<AtomicUsize>,
) -> AppResult<HashMap<String, FileMetadata>> {
    let prune_set = if prune_ignored {
        Some(get_default_prune_set())
    } else {
        None
    };

    let mut map = HashMap::new();

    for anchor in anchors {
        if cancel_token.load(Ordering::Relaxed) {
            return Err(AppError::ScanCancelled);
        }

        if let Ok(mut lock) = current_name.lock() {
            *lock = anchor.name.clone();
        }

        if anchor.is_dir {
            // Record the anchor folder entry itself
            if let Ok(rel) = anchor.path.strip_prefix(base_path) {
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                map.insert(
                    rel_str,
                    FileMetadata {
                        size: 0,
                        modified_timestamp: 0,
                        is_dir: true,
                    },
                );
            }

            // Scan subtree
            if is_hdd {
                hdd_sequential::scan_hdd_sequential(
                    &anchor.path,
                    base_path,
                    &prune_set,
                    cancel_token,
                    counter,
                    &mut map,
                )?;
            } else {
                ssd_parallel::scan_ssd_parallel_subtree(
                    &anchor.path,
                    base_path,
                    prune_set.clone(),
                    cancel_token,
                    counter,
                    &mut map,
                )?;
            }
        } else {
            // Anchor is a top-level file
            if let Ok(rel) = anchor.path.strip_prefix(base_path) {
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                let meta = fs::symlink_metadata(&anchor.path).ok();
                let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
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
                        is_dir: false,
                    },
                );

                counter.fetch_add(1, Ordering::Relaxed);
            }
        }

        completed.fetch_add(1, Ordering::Relaxed);
    }

    if let Ok(mut lock) = current_name.lock() {
        *lock = String::new();
    }

    Ok(map)
}
