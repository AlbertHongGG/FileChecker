use crate::models::SyncProgressPayload;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

pub struct SyncObserver {
    completed: AtomicUsize,
    total: usize,
    current_file: Mutex<String>,
    cancel_token: Arc<AtomicBool>,
}

impl SyncObserver {
    pub fn new(total: usize, cancel_token: Arc<AtomicBool>) -> Self {
        Self {
            completed: AtomicUsize::new(0),
            total,
            current_file: Mutex::new(String::new()),
            cancel_token,
        }
    }

    #[inline]
    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.load(Ordering::Relaxed)
    }

    pub fn inc(&self, rel_path: &str) {
        self.completed.fetch_add(1, Ordering::Relaxed);
        // Use non-blocking try_lock so worker threads in parallel mode never contend on the lock
        if let Ok(mut guard) = self.current_file.try_lock() {
            guard.clear();
            guard.push_str(rel_path);
        }
    }

    pub fn completed(&self) -> usize {
        self.completed.load(Ordering::Relaxed)
    }

    pub fn total(&self) -> usize {
        self.total
    }

    pub fn snapshot(&self) -> SyncProgressPayload {
        let current = self.completed.load(Ordering::Relaxed).min(self.total);
        let current_file = self
            .current_file
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default();

        SyncProgressPayload {
            current,
            total: self.total,
            current_file,
        }
    }
}
