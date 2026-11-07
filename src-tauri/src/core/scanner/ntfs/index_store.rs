use super::volume_index::MftVolumeIndex;
use super::volume_reader::NtfsVolumeReader;
use crate::core::scanner::ScanObserver;
use crate::error::AppResult;
use rustc_hash::FxHashMap;
use std::sync::{Arc, RwLock};

pub struct MftIndexStore {
    cache: RwLock<FxHashMap<char, Arc<MftVolumeIndex>>>,
}

impl MftIndexStore {
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(FxHashMap::default()),
        }
    }

    pub fn get_or_build(
        &self,
        drive_letter: char,
        observer: &ScanObserver,
    ) -> AppResult<Arc<MftVolumeIndex>> {
        let mut reader = NtfsVolumeReader::open(drive_letter)?;
        let current_usn = reader.query_usn_state();

        // 1. Fast read-lock check for valid cached index
        {
            let read_guard = self.cache.read().unwrap();
            if let Some(cached) = read_guard.get(&drive_letter) {
                if let (Some(cur), Some(cached_usn)) = (&current_usn, &cached.usn_state) {
                    if cur.volume_serial == cached_usn.volume_serial
                        && cur.journal_id == cached_usn.journal_id
                        && cur.next_usn == cached_usn.next_usn
                    {
                        return Ok(cached.clone());
                    }
                }
            }
        }

        // 2. Cache miss or stale: Acquire write lock and build
        let mut write_guard = self.cache.write().unwrap();

        // Double-check under write lock
        if let Some(cached) = write_guard.get(&drive_letter) {
            if let (Some(cur), Some(cached_usn)) = (&current_usn, &cached.usn_state) {
                if cur.volume_serial == cached_usn.volume_serial
                    && cur.journal_id == cached_usn.journal_id
                    && cur.next_usn == cached_usn.next_usn
                {
                    return Ok(cached.clone());
                }
            }
        }

        observer.set_current_name("Reading MFT table...");
        let nodes = reader.read_all_nodes(
            |completed, total| {
                observer.set_progress(completed, total);
            },
            || observer.is_cancelled(),
        )?;

        let new_index = Arc::new(MftVolumeIndex::new(drive_letter, current_usn, nodes));
        write_guard.insert(drive_letter, new_index.clone());

        Ok(new_index)
    }

    pub fn invalidate_drive(&self, drive_letter: char) {
        if let Ok(mut guard) = self.cache.write() {
            guard.remove(&drive_letter);
        }
    }

    pub fn invalidate_all(&self) {
        if let Ok(mut guard) = self.cache.write() {
            guard.clear();
        }
    }
}

impl Default for MftIndexStore {
    fn default() -> Self {
        Self::new()
    }
}
