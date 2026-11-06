use crate::error::AppError;
use crate::platform::reveal_in_explorer as platform_reveal;
use std::fs;
use std::path::PathBuf;

#[tauri::command]
pub async fn get_file_content_pair(
    dir_a: String,
    dir_b: String,
    relative_path: String,
) -> Result<(String, String), AppError> {
    tokio::task::spawn_blocking(move || {
        let path_a = PathBuf::from(&dir_a).join(&relative_path);
        let path_b = PathBuf::from(&dir_b).join(&relative_path);

        let content_a = if path_a.exists() && path_a.is_file() {
            fs::read(&path_a)
                .map(|b| String::from_utf8_lossy(&b).to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        let content_b = if path_b.exists() && path_b.is_file() {
            fs::read(&path_b)
                .map(|b| String::from_utf8_lossy(&b).to_string())
                .unwrap_or_default()
        } else {
            String::new()
        };

        Ok((content_a, content_b))
    })
    .await
    .map_err(|e| AppError::TaskExecution(e.to_string()))?
}

#[tauri::command]
pub fn reveal_in_explorer(full_path: String) {
    platform_reveal(&full_path);
}
