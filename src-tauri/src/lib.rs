use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use tauri::{Emitter, Window};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedEntry {
    pub relative_path: String,
    pub name: String,
    pub is_dir: bool,
    pub diff_kind: DiffKind,
    pub meta_a: Option<FileMetadata>,
    pub meta_b: Option<FileMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncTask {
    pub relative_path: String,
    pub action: String, // "copy_a_to_b" | "copy_b_to_a" | "delete_a" | "delete_b"
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncProgressPayload {
    pub current: usize,
    pub total: usize,
    pub current_file: String,
}

fn normalize_relative_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn extract_metadata(entry_path: &Path) -> Option<FileMetadata> {
    if let Ok(meta) = fs::symlink_metadata(entry_path) {
        let is_dir = meta.is_dir();
        let size = if is_dir { 0 } else { meta.len() };
        let modified_timestamp = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Some(FileMetadata {
            size,
            modified_timestamp,
            is_dir,
        })
    } else {
        None
    }
}

fn scan_directory_tree(base_path: &Path) -> HashMap<String, FileMetadata> {
    let mut map = HashMap::new();
    if !base_path.exists() || !base_path.is_dir() {
        return map;
    }

    for entry in WalkDir::new(base_path).min_depth(1).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if let Ok(rel) = path.strip_prefix(base_path) {
            let rel_str = normalize_relative_path(rel);
            if !rel_str.is_empty() {
                if let Some(meta) = extract_metadata(path) {
                    map.insert(rel_str, meta);
                }
            }
        }
    }
    map
}

#[tauri::command]
fn compare_directories(dir_a: String, dir_b: String) -> Result<Vec<UnifiedEntry>, String> {
    let path_a = PathBuf::from(&dir_a);
    let path_b = PathBuf::from(&dir_b);

    if !path_a.exists() {
        return Err(format!("Directory A does not exist: {}", dir_a));
    }
    if !path_b.exists() {
        return Err(format!("Directory B does not exist: {}", dir_b));
    }

    let map_a = scan_directory_tree(&path_a);
    let map_b = scan_directory_tree(&path_b);

    let mut all_rel_paths: BTreeSet<String> = BTreeSet::new();
    for k in map_a.keys() {
        all_rel_paths.insert(k.clone());
    }
    for k in map_b.keys() {
        all_rel_paths.insert(k.clone());
    }

    let mut entries = Vec::with_capacity(all_rel_paths.len());

    for rel in all_rel_paths {
        let meta_a = map_a.get(&rel).cloned();
        let meta_b = map_b.get(&rel).cloned();

        let name = Path::new(&rel)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| rel.clone());

        let is_dir = meta_a
            .as_ref()
            .map(|m| m.is_dir)
            .or_else(|| meta_b.as_ref().map(|m| m.is_dir))
            .unwrap_or(false);

        let diff_kind = match (&meta_a, &meta_b) {
            (Some(_), None) => DiffKind::OnlyInA,
            (None, Some(_)) => DiffKind::OnlyInB,
            (Some(a), Some(b)) => {
                if a.is_dir && b.is_dir {
                    DiffKind::Identical
                } else if a.size == b.size && a.modified_timestamp == b.modified_timestamp {
                    DiffKind::Identical
                } else {
                    DiffKind::Modified {
                        a_newer: a.modified_timestamp > b.modified_timestamp,
                        b_newer: b.modified_timestamp > a.modified_timestamp,
                        same_time: a.modified_timestamp == b.modified_timestamp,
                    }
                }
            }
            (None, None) => DiffKind::Identical,
        };

        entries.push(UnifiedEntry {
            relative_path: rel,
            name,
            is_dir,
            diff_kind,
            meta_a,
            meta_b,
        });
    }

    Ok(entries)
}

#[tauri::command]
fn get_file_content_pair(
    dir_a: String,
    dir_b: String,
    relative_path: String,
) -> Result<(String, String), String> {
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
}

#[tauri::command]
fn execute_sync(
    dir_a: String,
    dir_b: String,
    tasks: Vec<SyncTask>,
    window: Window,
) -> Result<usize, String> {
    let root_a = PathBuf::from(&dir_a);
    let root_b = PathBuf::from(&dir_b);
    let total = tasks.len();
    let mut completed = 0;

    for (index, task) in tasks.iter().enumerate() {
        let path_a = root_a.join(&task.relative_path);
        let path_b = root_b.join(&task.relative_path);

        match task.action.as_str() {
            "copy_a_to_b" => {
                if path_a.exists() {
                    if path_a.is_dir() {
                        let _ = fs::create_dir_all(&path_b);
                    } else {
                        if let Some(parent) = path_b.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        fs::copy(&path_a, &path_b)
                            .map_err(|e| format!("Failed to copy to B ({}): {}", task.relative_path, e))?;
                    }
                }
            }
            "copy_b_to_a" => {
                if path_b.exists() {
                    if path_b.is_dir() {
                        let _ = fs::create_dir_all(&path_a);
                    } else {
                        if let Some(parent) = path_a.parent() {
                            let _ = fs::create_dir_all(parent);
                        }
                        fs::copy(&path_b, &path_a)
                            .map_err(|e| format!("Failed to copy to A ({}): {}", task.relative_path, e))?;
                    }
                }
            }
            "delete_a" => {
                if path_a.exists() {
                    if path_a.is_dir() {
                        let _ = fs::remove_dir_all(&path_a);
                    } else {
                        let _ = fs::remove_file(&path_a);
                    }
                }
            }
            "delete_b" => {
                if path_b.exists() {
                    if path_b.is_dir() {
                        let _ = fs::remove_dir_all(&path_b);
                    } else {
                        let _ = fs::remove_file(&path_b);
                    }
                }
            }
            _ => {}
        }

        completed += 1;
        let _ = window.emit(
            "sync-progress",
            SyncProgressPayload {
                current: index + 1,
                total,
                current_file: task.relative_path.clone(),
            },
        );
    }

    Ok(completed)
}

#[tauri::command]
fn reveal_in_explorer(full_path: String) {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        use std::os::windows::process::CommandExt;
        let mut cmd = Command::new("explorer");
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        cmd.args(["/select,", &full_path]);
        let _ = cmd.spawn();
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            compare_directories,
            get_file_content_pair,
            execute_sync,
            reveal_in_explorer
        ])
        .run(tauri::generate_context!())
        .expect("error while running filechecker application");
}
