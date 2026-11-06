pub mod hdd_sequential;
pub mod ssd_parallel;

use crate::error::AppResult;
use crate::models::FileMetadata;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Arc;

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

pub fn scan_directory_adaptive(
    base_path: &Path,
    is_hdd: bool,
    prune_ignored: bool,
    cancel_token: &Arc<AtomicBool>,
    counter: &Arc<AtomicUsize>,
) -> AppResult<HashMap<String, FileMetadata>> {
    let prune_set = if prune_ignored {
        Some(get_default_prune_set())
    } else {
        None
    };

    if is_hdd {
        let mut map = HashMap::new();
        hdd_sequential::scan_hdd_sequential(
            base_path,
            base_path,
            &prune_set,
            cancel_token,
            counter,
            &mut map,
        )?;
        Ok(map)
    } else {
        ssd_parallel::scan_ssd_parallel(base_path, prune_set, cancel_token, counter)
    }
}
