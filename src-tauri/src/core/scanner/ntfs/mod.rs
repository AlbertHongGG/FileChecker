pub mod index_store;
pub mod record_parser;
pub mod volume_index;
pub mod volume_reader;

pub use index_store::MftIndexStore;
pub use record_parser::{MftNode, MftRecordParser};
pub use volume_index::MftVolumeIndex;
pub use volume_reader::NtfsVolumeReader;

use super::win32_engine::Win32LargeFetchEngine;
use super::{ScanEngine, ScanObserver, ScanPolicy, ScanSnapshot};
use crate::error::{AppError, AppResult};
use crate::platform::windows::win32_fs::get_directory_frn;
use std::path::Path;
use std::sync::Arc;

pub struct NtfsMftEngine {
    store: Arc<MftIndexStore>,
    is_hdd: bool,
}

impl NtfsMftEngine {
    pub fn new(store: Arc<MftIndexStore>, is_hdd: bool) -> Self {
        Self { store, is_hdd }
    }
}

impl ScanEngine for NtfsMftEngine {
    fn scan(
        &self,
        root: &Path,
        policy: &ScanPolicy,
        observer: &ScanObserver,
    ) -> AppResult<ScanSnapshot> {
        let path_str = root.to_string_lossy();
        if path_str.len() < 2 || path_str.as_bytes()[1] != b':' {
            let fallback = Win32LargeFetchEngine::new(self.is_hdd);
            return fallback.scan(root, policy, observer);
        }

        let drive_letter = path_str.chars().next().unwrap().to_ascii_uppercase();

        let is_root = path_str.len() <= 3
            || (path_str.len() == 3 && path_str.ends_with('\\'))
            || (path_str.len() == 2 && path_str.ends_with(':'));

        let target_frn = if is_root {
            5u64 // NTFS Root Directory FRN is 5
        } else {
            match get_directory_frn(root) {
                Some(frn) => frn,
                None => {
                    let fallback = Win32LargeFetchEngine::new(self.is_hdd);
                    return fallback.scan(root, policy, observer);
                }
            }
        };

        // Attempt MFT retrieval / parsing from store
        match self.store.get_or_build(drive_letter, observer) {
            Ok(volume_index) => {
                let snapshot = volume_index.extract_subtree(target_frn, policy, observer)?;
                // Safety check: if target directory has files on disk but MFT subtree extraction returned empty,
                // fallback to Win32 LargeFetch to guarantee accurate results.
                if snapshot.is_empty() && !is_root {
                    if let Ok(mut entries) = std::fs::read_dir(root) {
                        if entries.next().is_some() {
                            let fallback = Win32LargeFetchEngine::new(self.is_hdd);
                            return fallback.scan(root, policy, observer);
                        }
                    }
                }
                Ok(snapshot)
            }
            Err(e) => {
                if matches!(e, AppError::ScanCancelled) {
                    return Err(e);
                }
                // Graceful fallback to Win32 large fetch
                let fallback = Win32LargeFetchEngine::new(self.is_hdd);
                fallback.scan(root, policy, observer)
            }
        }
    }
}
