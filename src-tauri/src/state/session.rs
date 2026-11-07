use crate::core::scanner::ntfs::MftIndexStore;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct SessionManager {
    cancel_token: Mutex<Option<Arc<AtomicBool>>>,
    mft_store: Arc<MftIndexStore>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            cancel_token: Mutex::new(None),
            mft_store: Arc::new(MftIndexStore::new()),
        }
    }

    pub fn mft_store(&self) -> Arc<MftIndexStore> {
        self.mft_store.clone()
    }

    pub fn cancel_current(&self) {
        if let Ok(guard) = self.cancel_token.lock() {
            if let Some(token) = guard.as_ref() {
                token.store(true, Ordering::SeqCst);
            }
        }
    }

    pub fn new_token(&self) -> Arc<AtomicBool> {
        let mut guard = self.cancel_token.lock().unwrap();
        if let Some(token) = guard.as_ref() {
            token.store(true, Ordering::SeqCst);
        }
        let new_token = Arc::new(AtomicBool::new(false));
        *guard = Some(new_token.clone());
        new_token
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}
