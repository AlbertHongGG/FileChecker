use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Directory not found: {0}")]
    DirectoryNotFound(String),

    #[error("Scan cancelled by user")]
    ScanCancelled,

    #[error("Task failed: {0}")]
    TaskExecution(String),

    #[error("Sync operation failed for '{path}': {reason}")]
    SyncFailed { path: String, reason: String },

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
}

// Enable Tauri to serialize AppError directly to frontend as string or structured object
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
