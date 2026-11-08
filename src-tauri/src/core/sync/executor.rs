use super::observer::SyncObserver;
use super::plan::SyncExecutionPlan;
use crate::error::{AppError, AppResult};
use crate::platform::{MediaType, VolumeCapability};
use rayon::prelude::*;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncExecutionStrategy {
    Sequential,
    Parallel { threads: usize },
}

impl SyncExecutionStrategy {
    pub fn from_capabilities(cap_a: &VolumeCapability, cap_b: &VolumeCapability) -> Self {
        // If either volume incurs seek penalty (HDD), enforce Sequential mode to avoid head thrashing.
        if cap_a.media_type == MediaType::Hdd || cap_b.media_type == MediaType::Hdd {
            SyncExecutionStrategy::Sequential
        } else {
            // Both are SSDs (or unknown assumed SSD) -> use controlled parallel worker pool
            SyncExecutionStrategy::Parallel { threads: 6 }
        }
    }
}

pub struct SyncExecutor;

impl SyncExecutor {
    /// Fast file copy without redundant pre-emptive metadata checks.
    /// If destination exists and has Read-Only attribute (Windows), clears Read-Only and retries.
    pub fn fast_copy(src: &Path, dst: &Path) -> AppResult<()> {
        match fs::copy(src, dst) {
            Ok(_) => Ok(()),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    if let Ok(meta) = fs::metadata(dst) {
                        let mut perms = meta.permissions();
                        #[allow(clippy::permissions_set_readonly_false)]
                        if perms.readonly() {
                            perms.set_readonly(false);
                            let _ = fs::set_permissions(dst, perms);
                            return fs::copy(src, dst).map(|_| ()).map_err(|retry_err| {
                                AppError::SyncFailed {
                                    path: src.to_string_lossy().to_string(),
                                    reason: format!(
                                        "Failed to copy after clearing read-only: {}",
                                        retry_err
                                    ),
                                }
                            });
                        }
                    }
                }
                Err(AppError::SyncFailed {
                    path: src.to_string_lossy().to_string(),
                    reason: format!("Failed to copy file: {}", e),
                })
            }
        }
    }

    /// Fast file removal without pre-emptive metadata queries.
    /// Only queries metadata if permission denied (clears Read-Only attribute on Windows).
    pub fn fast_remove_file(target: &Path) -> AppResult<()> {
        match fs::remove_file(target) {
            Ok(_) => Ok(()),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    return Ok(());
                }
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    if let Ok(meta) = fs::metadata(target) {
                        let mut perms = meta.permissions();
                        #[allow(clippy::permissions_set_readonly_false)]
                        if perms.readonly() {
                            perms.set_readonly(false);
                            let _ = fs::set_permissions(target, perms);
                            return fs::remove_file(target).map_err(|retry_err| {
                                AppError::SyncFailed {
                                    path: target.to_string_lossy().to_string(),
                                    reason: format!(
                                        "Failed to delete file after clearing read-only: {}",
                                        retry_err
                                    ),
                                }
                            });
                        }
                    }
                }
                Err(AppError::SyncFailed {
                    path: target.to_string_lossy().to_string(),
                    reason: format!("Failed to delete file: {}", e),
                })
            }
        }
    }

    /// Removes a directory and all of its contents.
    pub fn fast_remove_dir(target: &Path) -> AppResult<()> {
        match fs::remove_dir_all(target) {
            Ok(_) => Ok(()),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    return Ok(());
                }
                Err(AppError::SyncFailed {
                    path: target.to_string_lossy().to_string(),
                    reason: format!("Failed to delete directory: {}", e),
                })
            }
        }
    }

    /// Executes the pre-compiled plan using the selected hardware-aware execution strategy.
    pub fn execute_plan(
        plan: &SyncExecutionPlan,
        strategy: SyncExecutionStrategy,
        observer: &SyncObserver,
    ) -> AppResult<usize> {
        // Step 1: Pre-create all destination directories
        for dir in &plan.dirs_to_create {
            if observer.is_cancelled() {
                return Err(AppError::ScanCancelled);
            }
            fs::create_dir_all(dir).map_err(|e| AppError::SyncFailed {
                path: dir.to_string_lossy().to_string(),
                reason: format!("Failed to create destination directory: {}", e),
            })?;
        }

        // Step 2: Directory deletions (top-level folders first)
        for op in &plan.delete_dir_ops {
            if observer.is_cancelled() {
                return Err(AppError::ScanCancelled);
            }
            Self::fast_remove_dir(&op.target)?;
            observer.inc(&op.rel_path);
        }

        // Step 3: File deletions
        match strategy {
            SyncExecutionStrategy::Parallel { threads } => {
                let pool = rayon::ThreadPoolBuilder::new()
                    .num_threads(threads)
                    .build()
                    .map_err(|e| AppError::TaskExecution(e.to_string()))?;

                pool.install(|| {
                    plan.delete_file_ops.par_iter().try_for_each(|op| {
                        if observer.is_cancelled() {
                            return Err(AppError::ScanCancelled);
                        }
                        Self::fast_remove_file(&op.target)?;
                        observer.inc(&op.rel_path);
                        Ok(())
                    })
                })?;
            }
            SyncExecutionStrategy::Sequential => {
                for op in &plan.delete_file_ops {
                    if observer.is_cancelled() {
                        return Err(AppError::ScanCancelled);
                    }
                    Self::fast_remove_file(&op.target)?;
                    observer.inc(&op.rel_path);
                }
            }
        }

        // Step 4: File copies
        match strategy {
            SyncExecutionStrategy::Parallel { threads } => {
                let pool = rayon::ThreadPoolBuilder::new()
                    .num_threads(threads)
                    .build()
                    .map_err(|e| AppError::TaskExecution(e.to_string()))?;

                pool.install(|| {
                    plan.copy_ops.par_iter().try_for_each(|op| {
                        if observer.is_cancelled() {
                            return Err(AppError::ScanCancelled);
                        }
                        Self::fast_copy(&op.src, &op.dst)?;
                        observer.inc(&op.rel_path);
                        Ok(())
                    })
                })?;
            }
            SyncExecutionStrategy::Sequential => {
                for op in &plan.copy_ops {
                    if observer.is_cancelled() {
                        return Err(AppError::ScanCancelled);
                    }
                    Self::fast_copy(&op.src, &op.dst)?;
                    observer.inc(&op.rel_path);
                }
            }
        }

        // Step 5: Advance progress for tasks that were coalesced under deleted directories
        for _ in 0..plan.coalesced_count {
            observer.inc("");
        }

        // Step 6: Deferred bottom-up empty parent directory cleanup
        // Directly invoking fs::remove_dir returns DirectoryNotEmpty in O(1) if non-empty,
        // eliminating expensive fs::read_dir enumeration syscalls.
        for parent in &plan.candidate_empty_parents {
            let _ = fs::remove_dir(parent);
        }

        Ok(observer.completed())
    }
}
