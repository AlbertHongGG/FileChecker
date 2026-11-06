use jwalk::WalkDir;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, UNIX_EPOCH};
use tauri::{Emitter, State, Window};

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
pub struct ScanProgressPayload {
    pub count: usize,
    pub current_path: String,
    pub phase: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncProgressPayload {
    pub current: usize,
    pub total: usize,
    pub current_file: String,
}

pub struct SessionManager {
    cancel_token: Mutex<Option<Arc<AtomicBool>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            cancel_token: Mutex::new(None),
        }
    }

    pub fn cancel_current(&self) {
        if let Ok(guard) = self.cancel_token.lock() {
            if let Some(token) = guard.as_ref() {
                token.store(true, Ordering::SeqCst);
            }
        }
    }

    pub fn new_token(&self) -> Arc<AtomicBool> {
        let mut guard = self.cancel_token.lock().unwrap();
        if let Some(token) = guard.as_ref() {
            token.store(true, Ordering::SeqCst);
        }
        let new_token = Arc::new(AtomicBool::new(false));
        *guard = Some(new_token.clone());
        new_token
    }
}

fn normalize_relative_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn get_prune_set() -> HashSet<&'static str> {
    let mut set = HashSet::new();
    set.insert("node_modules");
    set.insert("target");
    set.insert(".git");
    set.insert("dist");
    set.insert(".next");
    set.insert(".nuxt");
    set.insert(".venv");
    set.insert("venv");
    set.insert("__pycache__");
    set.insert(".idea");
    set.insert(".vscode");
    set.insert(".cache");
    set.insert("bin");
    set.insert("obj");
    set.insert(".turbo");
    set.insert(".output");
    set.insert(".gradle");
    set
}

fn scan_directory_tree(
    base_path: &Path,
    prune_ignored: bool,
    cancel_token: &Arc<AtomicBool>,
    window: &Window,
    phase: &str,
) -> Result<HashMap<String, FileMetadata>, String> {
    let mut map = HashMap::new();
    if !base_path.exists() || !base_path.is_dir() {
        return Ok(map);
    }

    let prune_set = if prune_ignored {
        Some(get_prune_set())
    } else {
        None
    };

    let walker = WalkDir::new(base_path)
        .skip_hidden(false)
        .process_read_dir(move |_depth, _path, _state, children| {
            if let Some(ref set) = prune_set {
                children.retain(|entry_res| {
                    if let Ok(entry) = entry_res {
                        if entry.file_type.is_dir() {
                            let name = entry.file_name.to_string_lossy();
                            return !set.contains(name.as_ref());
                        }
                    }
                    true
                });
            }
        });

    let mut last_emit = Instant::now();
    let mut count = 0;

    for entry_res in walker {
        if cancel_token.load(Ordering::Relaxed) {
            return Err("Scan cancelled by user".to_string());
        }

        if let Ok(entry) = entry_res {
            let path = entry.path();
            if path == base_path {
                continue;
            }

            if let Ok(rel) = path.strip_prefix(base_path) {
                let rel_str = normalize_relative_path(rel);
                if !rel_str.is_empty() {
                    let is_dir = entry.file_type.is_dir();
                    let size = if is_dir {
                        0
                    } else {
                        entry.metadata().map(|m| m.len()).unwrap_or(0)
                    };
                    let modified_timestamp = entry
                        .metadata()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);

                    map.insert(
                        rel_str.clone(),
                        FileMetadata {
                            size,
                            modified_timestamp,
                            is_dir,
                        },
                    );

                    count += 1;
                    if count % 100 == 0 && last_emit.elapsed().as_millis() > 60 {
                        let _ = window.emit(
                            "scan-progress",
                            ScanProgressPayload {
                                count,
                                current_path: rel_str,
                                phase: phase.to_string(),
                            },
                        );
                        last_emit = Instant::now();
                    }
                }
            }
        }
    }

    Ok(map)
}

#[tauri::command]
async fn compare_directories(
    dir_a: String,
    dir_b: String,
    prune_ignored: bool,
    window: Window,
    session: State<'_, Arc<SessionManager>>,
) -> Result<Vec<UnifiedEntry>, String> {
    let cancel_token = session.new_token();
    let win = window.clone();

    tokio::task::spawn_blocking(move || {
        let path_a = PathBuf::from(&dir_a);
        let path_b = PathBuf::from(&dir_b);

        if !path_a.exists() {
            return Err(format!("Directory A does not exist: {}", dir_a));
        }
        if !path_b.exists() {
            return Err(format!("Directory B does not exist: {}", dir_b));
        }

        let map_a = scan_directory_tree(&path_a, prune_ignored, &cancel_token, &win, "Directory A")?;
        let map_b = scan_directory_tree(&path_b, prune_ignored, &cancel_token, &win, "Directory B")?;

        if cancel_token.load(Ordering::Relaxed) {
            return Err("Scan cancelled by user".to_string());
        }

        let mut all_rel_paths: BTreeSet<String> = BTreeSet::new();
        for k in map_a.keys() {
            all_rel_paths.insert(k.clone());
        }
        for k in map_b.keys() {
            all_rel_paths.insert(k.clone());
        }

        let mut entries = Vec::with_capacity(all_rel_paths.len());

        for rel in all_rel_paths {
            if cancel_token.load(Ordering::Relaxed) {
                return Err("Scan cancelled by user".to_string());
            }

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
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?
}

#[tauri::command]
fn cancel_comparison(session: State<'_, Arc<SessionManager>>) {
    session.cancel_current();
}

#[tauri::command]
async fn get_file_content_pair(
    dir_a: String,
    dir_b: String,
    relative_path: String,
) -> Result<(String, String), String> {
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
    .map_err(|e| format!("Task failed: {}", e))?
}

#[tauri::command]
async fn execute_sync(
    dir_a: String,
    dir_b: String,
    tasks: Vec<SyncTask>,
    window: Window,
) -> Result<usize, String> {
    tokio::task::spawn_blocking(move || {
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
    })
    .await
    .map_err(|e| format!("Task failed: {}", e))?
}

#[tauri::command]
fn reveal_in_explorer(full_path: String) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        let mut cmd = Command::new("explorer");
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        cmd.args(["/select,", &full_path]);
        let _ = cmd.spawn();
    }
}

pub fn run() {
    let session_manager = Arc::new(SessionManager::new());

    tauri::Builder::default()
        .manage(session_manager)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            compare_directories,
            cancel_comparison,
            get_file_content_pair,
            execute_sync,
            reveal_in_explorer
        ])
        .run(tauri::generate_context!())
        .expect("error while running filechecker application");
}
