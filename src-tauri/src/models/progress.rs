use serde::{Deserialize, Serialize};

/// Detailed scan lifecycle stage for a single drive/engine
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "data")]
pub enum ScanStage {
    /// Preparing scan or determining drive parameters
    Initializing,

    /// MFT direct sector reads (raw disk DASD read)
    ReadingMftBlocks {
        completed_blocks: usize,
        total_blocks: usize,
    },

    /// Parsing raw MFT records and constructing CSR in RAM
    BuildingIndex {
        records_parsed: usize,
    },

    /// Extracting target subtree via in-memory DFS
    ExtractingSubtree {
        files_indexed: usize,
    },

    /// Win32 large fetch recursive directory scanning
    WalkingDirectories {
        completed_dirs: usize,
        total_dirs: usize,
        files_scanned: usize,
        current_dir: String,
    },

    /// Scan completed successfully
    Completed {
        total_files: usize,
    },

    /// Scan cancelled by user
    Cancelled,

    /// Scan failed with error
    Failed {
        error: String,
    },
}

impl Default for ScanStage {
    fn default() -> Self {
        Self::Initializing
    }
}

/// Independent progress state for one side (Dir A or Dir B)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SideScanProgress {
    pub engine: String,
    pub is_hdd: bool,
    pub percent: u8,
    pub stage: ScanStage,
}

impl SideScanProgress {
    pub fn new(engine: String, is_hdd: bool, stage: ScanStage) -> Self {
        let percent = match &stage {
            ScanStage::Initializing => 0,
            ScanStage::ReadingMftBlocks {
                completed_blocks,
                total_blocks,
            } => {
                if *total_blocks > 0 {
                    ((*completed_blocks as f64 / *total_blocks as f64) * 100.0).min(95.0) as u8
                } else {
                    0
                }
            }
            ScanStage::BuildingIndex { .. } => 96,
            ScanStage::ExtractingSubtree { .. } => 98,
            ScanStage::WalkingDirectories {
                completed_dirs,
                total_dirs,
                ..
            } => {
                if *total_dirs > 0 {
                    ((*completed_dirs as f64 / *total_dirs as f64) * 100.0).min(99.0) as u8
                } else {
                    0
                }
            }
            ScanStage::Completed { .. } => 100,
            ScanStage::Cancelled => 0,
            ScanStage::Failed { .. } => 0,
        };
        Self {
            engine,
            is_hdd,
            percent,
            stage,
        }
    }
}

/// Structured progress payload transmitted over Tauri IPC
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanProgressPayload {
    pub side_a: SideScanProgress,
    pub side_b: SideScanProgress,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncProgressPayload {
    pub current: usize,
    pub total: usize,
    pub current_file: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_stage_serde_reading_blocks() {
        let stage = ScanStage::ReadingMftBlocks {
            completed_blocks: 190,
            total_blocks: 225,
        };
        let progress = SideScanProgress::new("HDD (MFT)".to_string(), true, stage);
        assert_eq!(progress.percent, 84);

        let payload = ScanProgressPayload {
            side_a: SideScanProgress::new(
                "SSD (MFT)".to_string(),
                false,
                ScanStage::Completed {
                    total_files: 136047,
                },
            ),
            side_b: progress,
        };

        let json = serde_json::to_string(&payload).expect("Serialize to JSON");
        assert!(json.contains("\"kind\":\"ReadingMftBlocks\""));
        assert!(json.contains("\"completed_blocks\":190"));
        assert!(json.contains("\"total_blocks\":225"));
        assert!(!json.contains("files_count")); // Verified: zero files count field in reading stage

        let deserialized: ScanProgressPayload =
            serde_json::from_str(&json).expect("Deserialize from JSON");
        assert_eq!(payload, deserialized);
    }

    #[test]
    fn test_scan_stage_serde_walking_dirs() {
        let stage = ScanStage::WalkingDirectories {
            completed_dirs: 53,
            total_dirs: 54,
            files_scanned: 131146,
            current_dir: "Camera".to_string(),
        };
        let progress = SideScanProgress::new("SSD (Win32)".to_string(), false, stage);
        assert_eq!(progress.percent, 98);

        let json = serde_json::to_string(&progress).expect("Serialize to JSON");
        assert!(json.contains("\"kind\":\"WalkingDirectories\""));
        assert!(json.contains("\"current_dir\":\"Camera\""));
    }
}
