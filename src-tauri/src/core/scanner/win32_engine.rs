use super::{ScanEngine, ScanObserver, ScanPolicy, ScanSnapshot};
use crate::error::{AppError, AppResult};
use crate::models::FileMetadata;
use crate::platform::windows::win32_fs::{read_dir_large_fetch, RawDirEntry};
use rayon::prelude::*;
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

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
            return Ok(FxHashMap::default());
        }

        let top_entries = read_dir_large_fetch(root).map_err(|e| {
            AppError::TaskExecution(format!("Failed to read root directory: {}", e))
        })?;

        let filtered_top: Vec<RawDirEntry> = top_entries
            .into_iter()
            .filter(|e| !e.is_dir || !policy.should_prune(&e.name))
            .collect();

        observer.set_progress(0, filtered_top.len());

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
                    let mut stack: Vec<(PathBuf, String)> = vec![(top_path, top_rel)];
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
                                    observer.inc_files(file_batch);
                                    file_batch = 0;
                                }
                            }
                        }
                    }

                    if file_batch > 0 {
                        observer.inc_files(file_batch);
                    }
                } else {
                    local_map.insert(
                        top_rel,
                        FileMetadata {
                            size: top.size,
                            modified_timestamp: top.modified_timestamp,
                            is_dir: false,
                        },
                    );
                    observer.inc_files(1);
                }

                observer.completed_units.fetch_add(1, Ordering::Relaxed);
                Ok(local_map)
            })
            .collect();

        let mut final_map = FxHashMap::default();
        for res in results {
            let map = res?;
            final_map.extend(map);
        }

        observer.set_current_name("");
        Ok(final_map)
    }

    fn scan_hdd(
        &self,
        root: &Path,
        policy: &ScanPolicy,
        observer: &ScanObserver,
    ) -> AppResult<ScanSnapshot> {
        if !root.exists() {
            return Ok(FxHashMap::default());
        }

        let top_entries = read_dir_large_fetch(root).map_err(|e| {
            AppError::TaskExecution(format!("Failed to read root directory: {}", e))
        })?;

        let filtered_top: Vec<RawDirEntry> = top_entries
            .into_iter()
            .filter(|e| !e.is_dir || !policy.should_prune(&e.name))
            .collect();

        observer.set_progress(0, filtered_top.len());
        let mut final_map = FxHashMap::default();

        for top in filtered_top {
            if observer.is_cancelled() {
                return Err(AppError::ScanCancelled);
            }

            observer.set_current_name(&top.name);
            let top_rel = top.name.clone();

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
                let mut stack: Vec<(PathBuf, String)> = vec![(top_path, top_rel)];
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
                                observer.inc_files(file_batch);
                                file_batch = 0;
                            }
                        }
                    }
                }

                if file_batch > 0 {
                    observer.inc_files(file_batch);
                }
            } else {
                final_map.insert(
                    top_rel,
                    FileMetadata {
                        size: top.size,
                        modified_timestamp: top.modified_timestamp,
                        is_dir: false,
                    },
                );
                observer.inc_files(1);
            }

            observer.completed_units.fetch_add(1, Ordering::Relaxed);
        }

        observer.set_current_name("");
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
