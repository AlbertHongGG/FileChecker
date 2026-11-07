pub mod ntfs;
pub mod win32_engine;

use crate::error::AppResult;
use crate::models::FileMetadata;
use rustc_hash::{FxHashMap, FxHashSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

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
    pub count: Arc<AtomicUsize>,
    pub completed_units: Arc<AtomicUsize>,
    pub total_units: Arc<AtomicUsize>,
    pub current_name: Arc<Mutex<String>>,
}

impl ScanObserver {
    pub fn new(cancel_token: Arc<AtomicBool>) -> Self {
        Self {
            cancel_token,
            count: Arc::new(AtomicUsize::new(0)),
            completed_units: Arc::new(AtomicUsize::new(0)),
            total_units: Arc::new(AtomicUsize::new(0)),
            current_name: Arc::new(Mutex::new(String::new())),
        }
    }

    #[inline]
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn inc_files(&self, n: usize) {
        self.count.fetch_add(n, Ordering::Relaxed);
    }

    pub fn set_current_name(&self, name: &str) {
        if let Ok(mut lock) = self.current_name.lock() {
            *lock = name.to_string();
        }
    }

    pub fn set_progress(&self, completed: usize, total: usize) {
        self.completed_units.store(completed, Ordering::Relaxed);
        self.total_units.store(total, Ordering::Relaxed);
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
