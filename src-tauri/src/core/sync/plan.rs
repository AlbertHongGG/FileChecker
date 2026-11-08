use crate::error::{AppError, AppResult};
use crate::models::{SyncAction, SyncTask};
use rustc_hash::FxHashSet;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileCopyOp {
    pub src: PathBuf,
    pub dst: PathBuf,
    pub rel_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDeleteOp {
    pub target: PathBuf,
    pub rel_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirDeleteOp {
    pub target: PathBuf,
    pub rel_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncExecutionPlan {
    pub dirs_to_create: Vec<PathBuf>,
    pub copy_ops: Vec<FileCopyOp>,
    pub delete_dir_ops: Vec<DirDeleteOp>,
    pub delete_file_ops: Vec<FileDeleteOp>,
    pub candidate_empty_parents: Vec<PathBuf>,
    pub total_tasks: usize,
    pub coalesced_count: usize,
}

impl SyncExecutionPlan {
    /// Validates that a relative path does not escape boundaries via '..' or absolute components.
    pub fn sanitize_relative_path(rel_path: &str) -> AppResult<PathBuf> {
        let path = Path::new(rel_path);
        for comp in path.components() {
            match comp {
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(AppError::InvalidOperation(format!(
                        "Invalid path component detected in: {}",
                        rel_path
                    )));
                }
                _ => {}
            }
        }
        Ok(path.to_path_buf())
    }

    /// Checks if any ancestor of rel_path is in the given set of deleted directories.
    fn is_subsumed(rel_path: &str, deleted_dirs: &FxHashSet<String>) -> bool {
        let mut curr = rel_path;
        while let Some(idx) = curr.rfind('/') {
            let parent = &curr[..idx];
            if deleted_dirs.contains(parent) {
                return true;
            }
            curr = parent;
        }
        false
    }

    /// Compiles raw SyncTasks into an optimized execution plan.
    pub fn compile(dir_a: &Path, dir_b: &Path, mut tasks: Vec<SyncTask>) -> AppResult<Self> {
        let total_tasks = tasks.len();

        // 1. Filter out skip tasks
        tasks.retain(|t| t.action != SyncAction::Skip);

        // 2. Sort tasks by relative_path to maximize locality and ensure parent dirs appear first
        tasks.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

        let mut dirs_to_create_set = FxHashSet::default();
        let mut deleted_dirs_a = FxHashSet::default();
        let mut deleted_dirs_b = FxHashSet::default();

        let mut copy_ops = Vec::new();
        let mut delete_dir_ops = Vec::new();
        let mut delete_file_ops = Vec::new();
        let mut candidate_empty_set = FxHashSet::default();
        let mut coalesced_count = 0;

        for task in &tasks {
            let clean_rel = Self::sanitize_relative_path(&task.relative_path)?;
            let rel_str = clean_rel.to_slash_lossy();

            match task.action {
                SyncAction::CopyAtoB => {
                    let src = dir_a.join(&clean_rel);
                    let dst = dir_b.join(&clean_rel);

                    if task.is_dir {
                        dirs_to_create_set.insert(dst);
                    } else {
                        if let Some(parent) = dst.parent() {
                            if parent != dir_b {
                                dirs_to_create_set.insert(parent.to_path_buf());
                            }
                        }
                        copy_ops.push(FileCopyOp {
                            src,
                            dst,
                            rel_path: task.relative_path.clone(),
                        });
                    }
                }
                SyncAction::CopyBtoA => {
                    let src = dir_b.join(&clean_rel);
                    let dst = dir_a.join(&clean_rel);

                    if task.is_dir {
                        dirs_to_create_set.insert(dst);
                    } else {
                        if let Some(parent) = dst.parent() {
                            if parent != dir_a {
                                dirs_to_create_set.insert(parent.to_path_buf());
                            }
                        }
                        copy_ops.push(FileCopyOp {
                            src,
                            dst,
                            rel_path: task.relative_path.clone(),
                        });
                    }
                }
                SyncAction::DeleteA => {
                    if Self::is_subsumed(&rel_str, &deleted_dirs_a) {
                        coalesced_count += 1;
                        continue;
                    }

                    let target = dir_a.join(&clean_rel);
                    Self::collect_ancestors(dir_a, &target, &mut candidate_empty_set);

                    if task.is_dir {
                        deleted_dirs_a.insert(rel_str.into_owned());
                        delete_dir_ops.push(DirDeleteOp {
                            target,
                            rel_path: task.relative_path.clone(),
                        });
                    } else {
                        delete_file_ops.push(FileDeleteOp {
                            target,
                            rel_path: task.relative_path.clone(),
                        });
                    }
                }
                SyncAction::DeleteB => {
                    if Self::is_subsumed(&rel_str, &deleted_dirs_b) {
                        coalesced_count += 1;
                        continue;
                    }

                    let target = dir_b.join(&clean_rel);
                    Self::collect_ancestors(dir_b, &target, &mut candidate_empty_set);

                    if task.is_dir {
                        deleted_dirs_b.insert(rel_str.into_owned());
                        delete_dir_ops.push(DirDeleteOp {
                            target,
                            rel_path: task.relative_path.clone(),
                        });
                    } else {
                        delete_file_ops.push(FileDeleteOp {
                            target,
                            rel_path: task.relative_path.clone(),
                        });
                    }
                }
                SyncAction::DeleteBoth => {
                    let subsumed_a = Self::is_subsumed(&rel_str, &deleted_dirs_a);
                    let subsumed_b = Self::is_subsumed(&rel_str, &deleted_dirs_b);

                    if subsumed_a && subsumed_b {
                        coalesced_count += 1;
                        continue;
                    }

                    let target_a = dir_a.join(&clean_rel);
                    let target_b = dir_b.join(&clean_rel);

                    if !subsumed_a {
                        Self::collect_ancestors(dir_a, &target_a, &mut candidate_empty_set);
                        if task.is_dir {
                            deleted_dirs_a.insert(rel_str.clone().into_owned());
                            delete_dir_ops.push(DirDeleteOp {
                                target: target_a,
                                rel_path: task.relative_path.clone(),
                            });
                        } else {
                            delete_file_ops.push(FileDeleteOp {
                                target: target_a,
                                rel_path: task.relative_path.clone(),
                            });
                        }
                    }

                    if !subsumed_b {
                        Self::collect_ancestors(dir_b, &target_b, &mut candidate_empty_set);
                        if task.is_dir {
                            deleted_dirs_b.insert(rel_str.into_owned());
                            delete_dir_ops.push(DirDeleteOp {
                                target: target_b,
                                rel_path: task.relative_path.clone(),
                            });
                        } else {
                            delete_file_ops.push(FileDeleteOp {
                                target: target_b,
                                rel_path: task.relative_path.clone(),
                            });
                        }
                    }

                    if subsumed_a || subsumed_b {
                        coalesced_count += 1;
                    }
                }
                SyncAction::Skip => {}
            }
        }

        // Sort dirs_to_create by component count ascending (shallow to deep)
        let mut dirs_to_create: Vec<PathBuf> = dirs_to_create_set.into_iter().collect();
        dirs_to_create.sort_by_key(|p| p.components().count());

        // Sort candidate_empty_parents by component count descending (deep to shallow)
        let mut candidate_empty_parents: Vec<PathBuf> = candidate_empty_set.into_iter().collect();
        candidate_empty_parents.sort_by(|a, b| b.components().count().cmp(&a.components().count()));

        Ok(Self {
            dirs_to_create,
            copy_ops,
            delete_dir_ops,
            delete_file_ops,
            candidate_empty_parents,
            total_tasks,
            coalesced_count,
        })
    }

    fn collect_ancestors(root: &Path, target: &Path, set: &mut FxHashSet<PathBuf>) {
        let mut curr = target.parent();
        while let Some(parent) = curr {
            if parent == root || !parent.starts_with(root) {
                break;
            }
            set.insert(parent.to_path_buf());
            curr = parent.parent();
        }
    }
}

trait PathToSlash {
    fn to_slash_lossy(&self) -> std::borrow::Cow<'_, str>;
}

impl PathToSlash for Path {
    fn to_slash_lossy(&self) -> std::borrow::Cow<'_, str> {
        let s = self.to_string_lossy();
        if s.contains('\\') {
            std::borrow::Cow::Owned(s.replace('\\', "/"))
        } else {
            s
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_relative_path() {
        assert!(SyncExecutionPlan::sanitize_relative_path("safe/file.txt").is_ok());
        assert!(SyncExecutionPlan::sanitize_relative_path("../escape.txt").is_err());
        assert!(SyncExecutionPlan::sanitize_relative_path("safe/../../escape.txt").is_err());
    }

    #[test]
    fn test_parent_directory_deduplication() {
        let dir_a = Path::new("C:/data/a");
        let dir_b = Path::new("C:/data/b");

        let tasks = vec![
            SyncTask {
                relative_path: "src/utils/math.rs".to_string(),
                action: SyncAction::CopyAtoB,
                is_dir: false,
            },
            SyncTask {
                relative_path: "src/utils/string.rs".to_string(),
                action: SyncAction::CopyAtoB,
                is_dir: false,
            },
            SyncTask {
                relative_path: "src/main.rs".to_string(),
                action: SyncAction::CopyAtoB,
                is_dir: false,
            },
        ];

        let plan = SyncExecutionPlan::compile(dir_a, dir_b, tasks).unwrap();
        // Destination directories to create: src and src/utils (only 2, not 3)
        assert_eq!(plan.dirs_to_create.len(), 2);
        assert_eq!(plan.copy_ops.len(), 3);
    }

    #[test]
    fn test_subtree_coalescing_deletes() {
        let dir_a = Path::new("C:/data/a");
        let dir_b = Path::new("C:/data/b");

        let tasks = vec![
            SyncTask {
                relative_path: "build".to_string(),
                action: SyncAction::DeleteB,
                is_dir: true,
            },
            SyncTask {
                relative_path: "build/output.log".to_string(),
                action: SyncAction::DeleteB,
                is_dir: false,
            },
            SyncTask {
                relative_path: "build/sub/cache.bin".to_string(),
                action: SyncAction::DeleteB,
                is_dir: false,
            },
            SyncTask {
                relative_path: "other/file.txt".to_string(),
                action: SyncAction::DeleteB,
                is_dir: false,
            },
        ];

        let plan = SyncExecutionPlan::compile(dir_a, dir_b, tasks).unwrap();
        // The 2 items inside "build" should be coalesced!
        assert_eq!(plan.delete_dir_ops.len(), 1);
        assert_eq!(plan.delete_file_ops.len(), 1);
        assert_eq!(plan.coalesced_count, 2);
    }

    #[test]
    fn test_candidate_empty_parents_order() {
        let dir_a = Path::new("C:/data/a");
        let dir_b = Path::new("C:/data/b");

        let tasks = vec![SyncTask {
            relative_path: "deep/nested/folder/file.txt".to_string(),
            action: SyncAction::DeleteA,
            is_dir: false,
        }];

        let plan = SyncExecutionPlan::compile(dir_a, dir_b, tasks).unwrap();
        // Candidate parents should be: deep/nested/folder, deep/nested, deep (deepest first)
        assert_eq!(plan.candidate_empty_parents.len(), 3);
        assert!(
            plan.candidate_empty_parents[0].components().count()
                >= plan.candidate_empty_parents[1].components().count()
        );
        assert!(
            plan.candidate_empty_parents[1].components().count()
                >= plan.candidate_empty_parents[2].components().count()
        );
    }
}
