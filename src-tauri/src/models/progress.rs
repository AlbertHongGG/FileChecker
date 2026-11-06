use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanProgressPayload {
    pub count_a: usize,
    pub count_b: usize,
    pub is_hdd_a: bool,
    pub is_hdd_b: bool,
    pub phase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncProgressPayload {
    pub current: usize,
    pub total: usize,
    pub current_file: String,
}
