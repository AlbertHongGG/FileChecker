use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileMetadata {
    pub size: u64,
    pub modified_timestamp: u64,
    pub is_dir: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum DiffKind {
    Identical,
    OnlyInA,
    OnlyInB,
    Modified {
        a_newer: bool,
        b_newer: bool,
        same_time: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UnifiedEntry {
    pub relative_path: String,
    pub name: String,
    pub is_dir: bool,
    pub diff_kind: DiffKind,
    pub meta_a: Option<FileMetadata>,
    pub meta_b: Option<FileMetadata>,
}

#[derive(Debug, Clone)]
pub struct AnchorItem {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}
