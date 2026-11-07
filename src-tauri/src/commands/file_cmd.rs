use crate::error::AppError;
use crate::models::FileDiffPayload;
use crate::platform::reveal_in_explorer as platform_reveal;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_DIFF_SIZE: u64 = 2 * 1024 * 1024; // 2MB limit for text diff

fn is_binary_buffer(buf: &[u8]) -> bool {
    let check_len = buf.len().min(8192);
    buf[..check_len].contains(&0)
}

fn inspect_file(path: &Path) -> (String, bool, bool, u64) {
    if !path.exists() || !path.is_file() {
        return (String::new(), false, false, 0);
    }

    let meta = match fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return (String::new(), false, false, 0),
    };

    let size = meta.len();
    if size > MAX_DIFF_SIZE {
        return (String::new(), false, true, size);
    }

    match fs::read(path) {
        Ok(bytes) => {
            if is_binary_buffer(&bytes) {
                (String::new(), true, false, size)
            } else {
                (String::from_utf8_lossy(&bytes).to_string(), false, false, size)
            }
        }
        Err(_) => (String::new(), false, false, size),
    }
}

#[tauri::command]
pub async fn get_file_content_pair(
    dir_a: String,
    dir_b: String,
    relative_path: String,
) -> Result<FileDiffPayload, AppError> {
    tokio::task::spawn_blocking(move || {
        let path_a = PathBuf::from(&dir_a).join(&relative_path);
        let path_b = PathBuf::from(&dir_b).join(&relative_path);

        let (content_a, is_bin_a, is_large_a, size_a) = inspect_file(&path_a);
        let (content_b, is_bin_b, is_large_b, size_b) = inspect_file(&path_b);

        let is_binary = is_bin_a || is_bin_b;
        let is_too_large = is_large_a || is_large_b;

        Ok(FileDiffPayload {
            content_a,
            content_b,
            is_binary,
            is_too_large,
            size_a,
            size_b,
        })
    })
    .await
    .map_err(|e| AppError::TaskExecution(e.to_string()))?
}

#[tauri::command]
pub fn reveal_in_explorer(full_path: String) {
    platform_reveal(&full_path);
}
