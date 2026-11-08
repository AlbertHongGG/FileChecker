use super::volume_index::MftVolumeIndex;
use super::volume_reader::NtfsVolumeReader;
use crate::core::scanner::ScanObserver;
use crate::error::AppResult;
use rustc_hash::FxHashMap;
use std::sync::{Arc, Mutex};

pub struct MftIndexStore {
    drive_slots: Mutex<FxHashMap<char, Arc<Mutex<Option<Arc<MftVolumeIndex>>>>>>,
}

impl MftIndexStore {
    pub fn new() -> Self {
        Self {
            drive_slots: Mutex::new(FxHashMap::default()),
        }
    }

    fn get_slot(&self, drive_letter: char) -> Arc<Mutex<Option<Arc<MftVolumeIndex>>>> {
        let mut map = self.drive_slots.lock().unwrap();
        map.entry(drive_letter)
            .or_insert_with(|| Arc::new(Mutex::new(None)))
            .clone()
    }

    pub fn get_or_build(
        &self,
        drive_letter: char,
        is_hdd: bool,
        observer: &ScanObserver,
    ) -> AppResult<Arc<MftVolumeIndex>> {
        let slot = self.get_slot(drive_letter);
        let mut slot_guard = slot.lock().unwrap();

        // 1. Open volume reader to check current USN state
        let mut reader = NtfsVolumeReader::open(drive_letter, is_hdd)?;
        let current_usn = reader.query_usn_state();

        // 2. Cache hit check under drive-specific lock
        if let Some(cached) = slot_guard.as_ref() {
            if let (Some(cur), Some(cached_usn)) = (&current_usn, &cached.usn_state) {
                if cur.volume_serial == cached_usn.volume_serial
                    && cur.journal_id == cached_usn.journal_id
                    && cur.next_usn == cached_usn.next_usn
                {
                    return Ok(cached.clone());
                }
            }
        }

        // 3. Cache miss: Read fragments and build index (runs in parallel across different drives!)
        observer.set_reading_mft(0, 1);
        let fragments = reader.read_all_fragments(
            |completed, total| {
                observer.set_reading_mft(completed, total);
            },
            || observer.is_cancelled(),
        )?;

        observer.set_building_index(fragments.len());
        let new_index = Arc::new(MftVolumeIndex::new(drive_letter, current_usn, fragments));
        *slot_guard = Some(new_index.clone());

        Ok(new_index)
    }

    pub fn invalidate_drive(&self, drive_letter: char) {
        let slot = {
            let map = self.drive_slots.lock().unwrap();
            map.get(&drive_letter).cloned()
        };
        if let Some(s) = slot {
            if let Ok(mut guard) = s.lock() {
                *guard = None;
            }
        }
    }

    pub fn invalidate_all(&self) {
        let map = self.drive_slots.lock().unwrap().clone();
        for (_, slot) in map {
            if let Ok(mut guard) = slot.lock() {
                *guard = None;
            }
        }
    }
}

impl Default for MftIndexStore {
    fn default() -> Self {
        Self::new()
    }
}
