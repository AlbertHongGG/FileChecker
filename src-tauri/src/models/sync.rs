use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncAction {
    Skip,
    CopyAtoB,
    CopyBtoA,
    DeleteA,
    DeleteB,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncTask {
    pub relative_path: String,
    pub action: SyncAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SyncResult {
    pub completed: usize,
    pub total: usize,
}
