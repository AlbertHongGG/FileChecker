use super::{ScanEngine, ScanObserver, ScanPolicy, ScanSnapshot};
use crate::error::{AppError, AppResult};
use crate::models::FileMetadata;
use crate::platform::windows::win32_fs::{read_dir_large_fetch, RawDirEntry};
use rayon::prelude::*;
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct Win32LargeFetchEngine {
    pub is_hdd: bool,
}

impl Win32LargeFetchEngine {
    pub fn new(is_hdd: bool) -> Self {
        Self { is_hdd }
    }

    fn scan_ssd(
        &self,
        root: &Path,
        policy: &ScanPolicy,
        observer: &ScanObserver,
    ) -> AppResult<ScanSnapshot> {
        if !root.exists() {
            observer.mark_completed(0);
            return Ok(FxHashMap::default());
        }

        let top_entries = read_dir_large_fetch(root).map_err(|e| {
            AppError::TaskExecution(format!("Failed to read root directory: {}", e))
        })?;

        let filtered_top: Vec<RawDirEntry> = top_entries
            .into_iter()
            .filter(|e| !e.is_dir || !policy.should_prune(&e.name))
            .collect();

        let total_dirs = filtered_top.len();
        let completed_dirs = Arc::new(AtomicUsize::new(0));
        let files_scanned = Arc::new(AtomicUsize::new(0));

        observer.set_walking_dirs(0, total_dirs, 0, "");

        let results: Vec<AppResult<FxHashMap<String, FileMetadata>>> = filtered_top
            .into_par_iter()
            .map(|top| {
                if observer.is_cancelled() {
                    return Err(AppError::ScanCancelled);
                }

                let mut local_map = FxHashMap::default();
                let top_rel = top.name.clone();

                if top.is_dir {
                    local_map.insert(
                        top_rel.clone(),
                        FileMetadata {
                            size: 0,
                            modified_timestamp: 0,
                            is_dir: true,
                        },
                    );

                    let top_path = root.join(&top.name);
                    let mut stack: Vec<(PathBuf, String)> = vec![(top_path, top_rel.clone())];
                    let mut file_batch = 0usize;

                    while let Some((dir_path, rel_prefix)) = stack.pop() {
                        if observer.is_cancelled() {
                            return Err(AppError::ScanCancelled);
                        }

                        let children = match read_dir_large_fetch(&dir_path) {
                            Ok(c) => c,
                            Err(_) => continue,
                        };

                        for child in children {
                            if child.is_dir {
                                if policy.should_prune(&child.name) {
                                    continue;
                                }
                                let child_rel = format!("{}/{}", rel_prefix, child.name);
                                local_map.insert(
                                    child_rel.clone(),
                                    FileMetadata {
                                        size: 0,
                                        modified_timestamp: 0,
                                        is_dir: true,
                                    },
                                );
                                stack.push((dir_path.join(&child.name), child_rel));
                            } else {
                                let child_rel = format!("{}/{}", rel_prefix, child.name);
                                local_map.insert(
                                    child_rel,
                                    FileMetadata {
                                        size: child.size,
                                        modified_timestamp: child.modified_timestamp,
                                        is_dir: false,
                                    },
                                );
                                file_batch += 1;
                                if file_batch >= 128 {
                                    let count = files_scanned.fetch_add(file_batch, Ordering::Relaxed) + file_batch;
                                    let done = completed_dirs.load(Ordering::Relaxed);
                                    observer.set_walking_dirs(done, total_dirs, count, &top_rel);
                                    file_batch = 0;
                                }
                            }
                        }
                    }

                    if file_batch > 0 {
                        let count = files_scanned.fetch_add(file_batch, Ordering::Relaxed) + file_batch;
                        let done = completed_dirs.load(Ordering::Relaxed);
                        observer.set_walking_dirs(done, total_dirs, count, &top_rel);
                    }
                } else {
                    local_map.insert(
                        top_rel.clone(),
                        FileMetadata {
                            size: top.size,
                            modified_timestamp: top.modified_timestamp,
                            is_dir: false,
                        },
                    );
                    let count = files_scanned.fetch_add(1, Ordering::Relaxed) + 1;
                    let done = completed_dirs.load(Ordering::Relaxed);
                    observer.set_walking_dirs(done, total_dirs, count, &top_rel);
                }

                let done = completed_dirs.fetch_add(1, Ordering::Relaxed) + 1;
                let count = files_scanned.load(Ordering::Relaxed);
                observer.set_walking_dirs(done, total_dirs, count, &top_rel);
                Ok(local_map)
            })
            .collect();

        let mut final_map = FxHashMap::default();
        for res in results {
            let map = res?;
            final_map.extend(map);
        }

        let total_files = final_map.values().filter(|m| !m.is_dir).count();
        observer.mark_completed(total_files);
        Ok(final_map)
    }

    fn scan_hdd(
        &self,
        root: &Path,
        policy: &ScanPolicy,
        observer: &ScanObserver,
    ) -> AppResult<ScanSnapshot> {
        if !root.exists() {
            observer.mark_completed(0);
            return Ok(FxHashMap::default());
        }

        let top_entries = read_dir_large_fetch(root).map_err(|e| {
            AppError::TaskExecution(format!("Failed to read root directory: {}", e))
        })?;

        let filtered_top: Vec<RawDirEntry> = top_entries
            .into_iter()
            .filter(|e| !e.is_dir || !policy.should_prune(&e.name))
            .collect();

        let total_dirs = filtered_top.len();
        let mut completed_dirs = 0usize;
        let mut files_scanned = 0usize;

        observer.set_walking_dirs(0, total_dirs, 0, "");
        let mut final_map = FxHashMap::default();

        for top in filtered_top {
            if observer.is_cancelled() {
                observer.mark_cancelled();
                return Err(AppError::ScanCancelled);
            }

            let top_rel = top.name.clone();
            observer.set_walking_dirs(completed_dirs, total_dirs, files_scanned, &top_rel);

            if top.is_dir {
                final_map.insert(
                    top_rel.clone(),
                    FileMetadata {
                        size: 0,
                        modified_timestamp: 0,
                        is_dir: true,
                    },
                );

                let top_path = root.join(&top.name);
                let mut stack: Vec<(PathBuf, String)> = vec![(top_path, top_rel.clone())];
                let mut file_batch = 0usize;

                while let Some((dir_path, rel_prefix)) = stack.pop() {
                    if observer.is_cancelled() {
                        observer.mark_cancelled();
                        return Err(AppError::ScanCancelled);
                    }

                    let children = match read_dir_large_fetch(&dir_path) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };

                    for child in children {
                        if child.is_dir {
                            if policy.should_prune(&child.name) {
                                continue;
                            }
                            let child_rel = format!("{}/{}", rel_prefix, child.name);
                            final_map.insert(
                                child_rel.clone(),
                                FileMetadata {
                                    size: 0,
                                    modified_timestamp: 0,
                                    is_dir: true,
                                },
                            );
                            stack.push((dir_path.join(&child.name), child_rel));
                        } else {
                            let child_rel = format!("{}/{}", rel_prefix, child.name);
                            final_map.insert(
                                child_rel,
                                FileMetadata {
                                    size: child.size,
                                    modified_timestamp: child.modified_timestamp,
                                    is_dir: false,
                                },
                            );
                            file_batch += 1;
                            if file_batch >= 128 {
                                files_scanned += file_batch;
                                observer.set_walking_dirs(completed_dirs, total_dirs, files_scanned, &top_rel);
                                file_batch = 0;
                            }
                        }
                    }
                }

                if file_batch > 0 {
                    files_scanned += file_batch;
                    observer.set_walking_dirs(completed_dirs, total_dirs, files_scanned, &top_rel);
                }
            } else {
                final_map.insert(
                    top_rel.clone(),
                    FileMetadata {
                        size: top.size,
                        modified_timestamp: top.modified_timestamp,
                        is_dir: false,
                    },
                );
                files_scanned += 1;
                observer.set_walking_dirs(completed_dirs, total_dirs, files_scanned, &top_rel);
            }

            completed_dirs += 1;
            observer.set_walking_dirs(completed_dirs, total_dirs, files_scanned, &top_rel);
        }

        let total_files = final_map.values().filter(|m| !m.is_dir).count();
        observer.mark_completed(total_files);
        Ok(final_map)
    }
}

impl ScanEngine for Win32LargeFetchEngine {
    fn scan(
        &self,
        root: &Path,
        policy: &ScanPolicy,
        observer: &ScanObserver,
    ) -> AppResult<ScanSnapshot> {
        if self.is_hdd {
            self.scan_hdd(root, policy, observer)
        } else {
            self.scan_ssd(root, policy, observer)
        }
    }
}
