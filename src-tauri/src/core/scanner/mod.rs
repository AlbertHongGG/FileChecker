pub mod ntfs;
pub mod win32_engine;

use crate::error::AppResult;
use crate::models::{FileMetadata, ScanStage, SideScanProgress};
use rustc_hash::{FxHashMap, FxHashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

pub type ScanSnapshot = FxHashMap<String, FileMetadata>;

#[derive(Debug, Clone)]
pub struct ScanPolicy {
    pub prune_ignored: bool,
    prune_set: FxHashSet<&'static str>,
}

impl ScanPolicy {
    pub fn new(prune_ignored: bool) -> Self {
        let mut prune_set = FxHashSet::default();
        if prune_ignored {
            // General / Dependencies
            prune_set.insert("node_modules");
            prune_set.insert("vendor");
            prune_set.insert(".git");
            prune_set.insert(".idea");
            prune_set.insert(".vscode");
            prune_set.insert(".vs");
            prune_set.insert(".cache");

            // Android / Flutter / Java / Build outputs
            prune_set.insert("build");
            prune_set.insert(".gradle");
            prune_set.insert(".dart_tool");
            prune_set.insert(".kotlin");
            prune_set.insert("bin");
            prune_set.insert("obj");
            prune_set.insert("target");
            prune_set.insert("out");
            prune_set.insert("dist");
            prune_set.insert(".output");

            // Web Frameworks
            prune_set.insert(".next");
            prune_set.insert(".nuxt");
            prune_set.insert(".turbo");
            prune_set.insert(".svelte-kit");
            prune_set.insert(".angular");
            prune_set.insert(".parcel-cache");
            prune_set.insert(".vite");
            prune_set.insert(".vercel");
            prune_set.insert("coverage");
            prune_set.insert("storybook-static");

            // Python / AI
            prune_set.insert(".venv");
            prune_set.insert("venv");
            prune_set.insert("__pycache__");
            prune_set.insert(".pytest_cache");
            prune_set.insert(".mypy_cache");
            prune_set.insert(".ruff_cache");
            prune_set.insert(".tox");

            // C/C++ / iOS / Infra
            prune_set.insert("cmake-build-debug");
            prune_set.insert("cmake-build-release");
            prune_set.insert("Pods");
            prune_set.insert("DerivedData");
            prune_set.insert(".terraform");
        }
        Self {
            prune_ignored,
            prune_set,
        }
    }

    #[inline]
    pub fn should_prune(&self, name: &str) -> bool {
        if !self.prune_ignored {
            return false;
        }
        self.prune_set.contains(name)
    }
}

pub struct ScanObserver {
    pub cancel_token: Arc<AtomicBool>,
    stage: Arc<RwLock<ScanStage>>,
}

impl ScanObserver {
    pub fn new(cancel_token: Arc<AtomicBool>) -> Self {
        Self {
            cancel_token,
            stage: Arc::new(RwLock::new(ScanStage::Initializing)),
        }
    }

    #[inline]
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.load(Ordering::Relaxed)
    }

    pub fn set_reading_mft(&self, completed_blocks: usize, total_blocks: usize) {
        if let Ok(mut stage) = self.stage.write() {
            *stage = ScanStage::ReadingMftBlocks {
                completed_blocks,
                total_blocks,
            };
        }
    }

    pub fn set_building_index(&self, records_parsed: usize) {
        if let Ok(mut stage) = self.stage.write() {
            *stage = ScanStage::BuildingIndex { records_parsed };
        }
    }

    pub fn set_extracting_subtree(&self, files_indexed: usize) {
        if let Ok(mut stage) = self.stage.write() {
            *stage = ScanStage::ExtractingSubtree { files_indexed };
        }
    }

    pub fn set_walking_dirs(
        &self,
        completed_dirs: usize,
        total_dirs: usize,
        files_scanned: usize,
        current_dir: &str,
    ) {
        if let Ok(mut stage) = self.stage.write() {
            *stage = ScanStage::WalkingDirectories {
                completed_dirs,
                total_dirs,
                files_scanned,
                current_dir: current_dir.to_string(),
            };
        }
    }

    pub fn mark_completed(&self, total_files: usize) {
        if let Ok(mut stage) = self.stage.write() {
            *stage = ScanStage::Completed { total_files };
        }
    }

    pub fn mark_cancelled(&self) {
        if let Ok(mut stage) = self.stage.write() {
            *stage = ScanStage::Cancelled;
        }
    }

    pub fn mark_failed(&self, error: &str) {
        if let Ok(mut stage) = self.stage.write() {
            *stage = ScanStage::Failed {
                error: error.to_string(),
            };
        }
    }

    pub fn current_stage(&self) -> ScanStage {
        self.stage
            .read()
            .map(|s| s.clone())
            .unwrap_or(ScanStage::Initializing)
    }

    pub fn snapshot(&self, engine: &str, is_hdd: bool) -> SideScanProgress {
        let stage = self.current_stage();
        SideScanProgress::new(engine.to_string(), is_hdd, stage)
    }
}

pub trait ScanEngine: Send + Sync {
    fn scan(
        &self,
        root: &Path,
        policy: &ScanPolicy,
        observer: &ScanObserver,
    ) -> AppResult<ScanSnapshot>;
}
