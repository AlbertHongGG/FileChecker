use crate::error::{AppError, AppResult};
use crate::models::{SyncAction, SyncTask};
use std::fs;
use std::path::{Path, PathBuf};

pub struct SyncExecutor;

impl SyncExecutor {
    /// Validates that a relative path does not escape the boundary via parent dir traversal ('..').
    fn sanitize_relative_path(rel_path: &str) -> AppResult<PathBuf> {
        let path = Path::new(rel_path);
        for comp in path.components() {
            if let std::path::Component::ParentDir = comp {
                return Err(AppError::InvalidOperation(format!(
                    "Path traversal attempt detected: {}",
                    rel_path
                )));
            }
        }
        Ok(path.to_path_buf())
    }

    /// Attempts to clear read-only flag before deletion on Windows
    fn safe_remove(path: &Path, rel_path: &str) -> AppResult<()> {
        if !path.exists() {
            return Ok(());
        }

        // If file is read-only, remove read-only attribute so delete succeeds
        if let Ok(meta) = fs::metadata(path) {
            let mut permissions = meta.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            if permissions.readonly() {
                permissions.set_readonly(false);
                let _ = fs::set_permissions(path, permissions);
            }
        }

        if path.is_dir() {
            fs::remove_dir_all(path).map_err(|e| AppError::SyncFailed {
                path: rel_path.to_string(),
                reason: format!("Failed to delete directory: {}", e),
            })
        } else {
            fs::remove_file(path).map_err(|e| AppError::SyncFailed {
                path: rel_path.to_string(),
                reason: format!("Failed to delete file: {}", e),
            })
        }
    }

    /// Recursively cleans up empty parent directories up to root boundary
    fn clean_empty_parents(root: &Path, target: &Path) {
        let mut curr = target.parent();
        while let Some(parent) = curr {
            if parent == root || !parent.starts_with(root) {
                break;
            }
            // Check if directory is empty
            if let Ok(mut entries) = fs::read_dir(parent) {
                if entries.next().is_none() {
                    // It is empty, remove it
                    let _ = fs::remove_dir(parent);
                    curr = parent.parent();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    pub fn execute<F>(
        dir_a: &Path,
        dir_b: &Path,
        tasks: Vec<SyncTask>,
        on_progress: F,
    ) -> AppResult<usize>
    where
        F: Fn(usize, usize, &str),
    {
        let total = tasks.len();
        let mut completed = 0;

        for (index, task) in tasks.iter().enumerate() {
            if task.action == SyncAction::Skip {
                on_progress(index + 1, total, &task.relative_path);
                continue;
            }

            let clean_rel = Self::sanitize_relative_path(&task.relative_path)?;
            let path_a = dir_a.join(&clean_rel);
            let path_b = dir_b.join(&clean_rel);

            match task.action {
                SyncAction::CopyAtoB => {
                    if path_a.exists() {
                        if path_a.is_dir() {
                            fs::create_dir_all(&path_b).map_err(|e| AppError::SyncFailed {
                                path: task.relative_path.clone(),
                                reason: format!("Failed to create directory B: {}", e),
                            })?;
                        } else {
                            if let Some(parent) = path_b.parent() {
                                fs::create_dir_all(parent).map_err(|e| AppError::SyncFailed {
                                    path: task.relative_path.clone(),
                                    reason: format!("Failed to create parent directory for B: {}", e),
                                })?;
                            }
                            fs::copy(&path_a, &path_b).map_err(|e| AppError::SyncFailed {
                                path: task.relative_path.clone(),
                                reason: format!("Failed to copy to B: {}", e),
                            })?;
                        }
                    }
                }
                SyncAction::CopyBtoA => {
                    if path_b.exists() {
                        if path_b.is_dir() {
                            fs::create_dir_all(&path_a).map_err(|e| AppError::SyncFailed {
                                path: task.relative_path.clone(),
                                reason: format!("Failed to create directory A: {}", e),
                            })?;
                        } else {
                            if let Some(parent) = path_a.parent() {
                                fs::create_dir_all(parent).map_err(|e| AppError::SyncFailed {
                                    path: task.relative_path.clone(),
                                    reason: format!("Failed to create parent directory for A: {}", e),
                                })?;
                            }
                            fs::copy(&path_b, &path_a).map_err(|e| AppError::SyncFailed {
                                path: task.relative_path.clone(),
                                reason: format!("Failed to copy to A: {}", e),
                            })?;
                        }
                    }
                }
                SyncAction::DeleteA => {
                    Self::safe_remove(&path_a, &task.relative_path)?;
                    Self::clean_empty_parents(dir_a, &path_a);
                }
                SyncAction::DeleteB => {
                    Self::safe_remove(&path_b, &task.relative_path)?;
                    Self::clean_empty_parents(dir_b, &path_b);
                }
                SyncAction::DeleteBoth => {
                    Self::safe_remove(&path_a, &task.relative_path)?;
                    Self::safe_remove(&path_b, &task.relative_path)?;
                    Self::clean_empty_parents(dir_a, &path_a);
                    Self::clean_empty_parents(dir_b, &path_b);
                }
                SyncAction::Skip => {}
            }

            completed += 1;
            on_progress(index + 1, total, &task.relative_path);
        }

        Ok(completed)
    }
}
