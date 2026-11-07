use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnchorProgressPayload {
    pub completed_a: usize,
    pub total_a: usize,
    pub current_name_a: String,
    pub count_a: usize,
    pub is_hdd_a: bool,
    pub engine_a: String,

    pub completed_b: usize,
    pub total_b: usize,
    pub current_name_b: String,
    pub count_b: usize,
    pub is_hdd_b: bool,
    pub engine_b: String,

    pub phase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncProgressPayload {
    pub current: usize,
    pub total: usize,
    pub current_file: String,
}
