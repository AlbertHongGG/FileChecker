use jwalk::WalkDir;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;
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
    pub action: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanProgressPayload {
    pub count_a: usize,
    pub count_b: usize,
    pub is_hdd_a: bool,
    pub is_hdd_b: bool,
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

// Hardware-Adaptive Storage Inspector: Detects if path is on a mechanical HDD / USB drive
#[cfg(target_os = "windows")]
fn is_slow_or_hdd(path: &Path) -> bool {
    use std::os::windows::ffi::OsStrExt;

    let path_str = path.to_string_lossy();
    let drive_letter = if path_str.len() >= 2 && path_str.as_bytes()[1] == b':' {
        path_str.chars().next().unwrap()
    } else {
        return false;
    };

    let drive_prefix = format!("{}:\\", drive_letter);
    let wide_root: Vec<u16> = std::ffi::OsStr::new(&drive_prefix)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    extern "system" {
        fn GetDriveTypeW(lpRootPathName: *const u16) -> u32;
    }

    let drive_type = unsafe { GetDriveTypeW(wide_root.as_ptr()) };
    if drive_type == 2 {
        // DRIVE_REMOVABLE (USB flash drive / card)
        return true;
    }

    // Check Seek Penalty via IOCTL_STORAGE_QUERY_PROPERTY
    let dev_path = format!("\\\\.\\{}:", drive_letter);
    let wide_dev: Vec<u16> = std::ffi::OsStr::new(&dev_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    extern "system" {
        fn CreateFileW(
            lpFileName: *const u16,
            dwDesiredAccess: u32,
            dwShareMode: u32,
            lpSecurityAttributes: *const std::ffi::c_void,
            dwCreationDisposition: u32,
            dwFlagsAndAttributes: u32,
            hTemplateFile: isize,
        ) -> isize;
        fn DeviceIoControl(
            hDevice: isize,
            dwIoControlCode: u32,
            lpInBuffer: *const std::ffi::c_void,
            nInBufferSize: u32,
            lpOutBuffer: *mut std::ffi::c_void,
            nOutBufferSize: u32,
            lpBytesReturned: *mut u32,
            lpOverlapped: *mut std::ffi::c_void,
        ) -> i32;
        fn CloseHandle(hObject: isize) -> i32;
    }

    const FILE_SHARE_READ: u32 = 0x00000001;
    const FILE_SHARE_WRITE: u32 = 0x00000002;
    const OPEN_EXISTING: u32 = 3;
    const IOCTL_STORAGE_QUERY_PROPERTY: u32 = 0x002D1400;

    unsafe {
        let handle = CreateFileW(
            wide_dev.as_ptr(),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            0,
        );

        if handle == -1 || handle == 0 {
            // Fallback: If not C:, consider external/secondary
            return drive_letter.to_ascii_uppercase() != 'C';
        }

        #[repr(C)]
        struct StoragePropertyQuery {
            property_id: u32,
            query_type: u32,
            additional_parameters: [u8; 1],
        }

        #[repr(C)]
        struct DeviceSeekPenaltyDescriptor {
            version: u32,
            size: u32,
            incurs_seek_penalty: u8,
        }

        let query = StoragePropertyQuery {
            property_id: 7, // StorageDeviceSeekPenaltyProperty
            query_type: 0,
            additional_parameters: [0],
        };

        let mut desc = DeviceSeekPenaltyDescriptor {
            version: 0,
            size: 0,
            incurs_seek_penalty: 0,
        };
        let mut bytes_returned = 0u32;

        let ok = DeviceIoControl(
            handle,
            IOCTL_STORAGE_QUERY_PROPERTY,
            &query as *const _ as *const _,
            std::mem::size_of::<StoragePropertyQuery>() as u32,
            &mut desc as *mut _ as *mut _,
            std::mem::size_of::<DeviceSeekPenaltyDescriptor>() as u32,
            &mut bytes_returned,
            std::ptr::null_mut(),
        );

        CloseHandle(handle);

        if ok != 0 {
            return desc.incurs_seek_penalty != 0;
        }

        drive_letter.to_ascii_uppercase() != 'C'
    }
}

#[cfg(not(target_os = "windows"))]
fn is_slow_or_hdd(_path: &Path) -> bool {
    false
}

// Sequential Zero-Overhead Scanner for Mechanical HDD / USB
// Uses single-threaded depth-first recursion with WIN32_FIND_DATAW zero-cost metadata
fn scan_hdd_sequential(
    current_dir: &Path,
    base_path: &Path,
    prune_set: &Option<HashSet<&'static str>>,
    cancel_token: &Arc<AtomicBool>,
    counter: &Arc<AtomicUsize>,
    map: &mut HashMap<String, FileMetadata>,
) -> Result<(), String> {
    if cancel_token.load(Ordering::Relaxed) {
        return Err("Scan cancelled by user".to_string());
    }

    let entries = match fs::read_dir(current_dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry_res in entries {
        if cancel_token.load(Ordering::Relaxed) {
            return Err("Scan cancelled by user".to_string());
        }

        let entry = match entry_res {
            Ok(e) => e,
            Err(_) => continue,
        };

        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();

        let is_dir = file_type.is_dir();

        // Pruning at directory boundary
        if is_dir {
            if let Some(ref set) = prune_set {
                if set.contains(name_str.as_ref()) {
                    continue; // Skip subtree completely!
                }
            }
        }

        let path = entry.path();
        if let Ok(rel) = path.strip_prefix(base_path) {
            let rel_str = normalize_relative_path(rel);
            if !rel_str.is_empty() {
                // In std::fs on Windows, entry.metadata() directly uses the cached WIN32_FIND_DATAW (0 extra syscalls!)
                let meta = entry.metadata().ok();
                let size = if is_dir {
                    0
                } else {
                    meta.as_ref().map(|m| m.len()).unwrap_or(0)
                };
                let modified_timestamp = meta
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);

                map.insert(
                    rel_str,
                    FileMetadata {
                        size,
                        modified_timestamp,
                        is_dir,
                    },
                );

                counter.fetch_add(1, Ordering::Relaxed);
            }
        }

        if is_dir {
            scan_hdd_sequential(&path, base_path, prune_set, cancel_token, counter, map)?;
        }
    }

    Ok(())
}

// Parallel Work-Stealing Scanner for Internal High-IOPS SSDs
fn scan_ssd_parallel(
    base_path: &Path,
    prune_set: Option<HashSet<&'static str>>,
    cancel_token: &Arc<AtomicBool>,
    counter: &Arc<AtomicUsize>,
) -> Result<HashMap<String, FileMetadata>, String> {
    let mut map = HashMap::new();
    if !base_path.exists() || !base_path.is_dir() {
        return Ok(map);
    }

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
                        rel_str,
                        FileMetadata {
                            size,
                            modified_timestamp,
                            is_dir,
                        },
                    );

                    counter.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    Ok(map)
}

// Adaptive Dispatcher: Selects SSD parallel vs HDD sequential based on hardware characteristics
fn scan_directory_adaptive(
    base_path: &Path,
    is_hdd: bool,
    prune_ignored: bool,
    cancel_token: &Arc<AtomicBool>,
    counter: &Arc<AtomicUsize>,
) -> Result<HashMap<String, FileMetadata>, String> {
    let prune_set = if prune_ignored {
        Some(get_prune_set())
    } else {
        None
    };

    if is_hdd {
        let mut map = HashMap::new();
        scan_hdd_sequential(
            base_path,
            base_path,
            &prune_set,
            cancel_token,
            counter,
            &mut map,
        )?;
        Ok(map)
    } else {
        scan_ssd_parallel(base_path, prune_set, cancel_token, counter)
    }
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

        // Detect Hardware Characteristics
        let is_hdd_a = is_slow_or_hdd(&path_a);
        let is_hdd_b = is_slow_or_hdd(&path_b);

        let counter_a = Arc::new(AtomicUsize::new(0));
        let counter_b = Arc::new(AtomicUsize::new(0));

        // Background progress ticker thread
        let win_ticker = win.clone();
        let cancel_ticker = cancel_token.clone();
        let c_a = counter_a.clone();
        let c_b = counter_b.clone();

        let ticker_handle = std::thread::spawn(move || {
            while !cancel_ticker.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(60));
                let count_a = c_a.load(Ordering::Relaxed);
                let count_b = c_b.load(Ordering::Relaxed);
                let _ = win_ticker.emit(
                    "scan-progress",
                    ScanProgressPayload {
                        count_a,
                        count_b,
                        is_hdd_a,
                        is_hdd_b,
                        phase: "Scanning".to_string(),
                    },
                );
            }
        });

        // DUAL CONCURRENCY: Run Directory A and Directory B simultaneously!
        let token_a = cancel_token.clone();
        let token_b = cancel_token.clone();
        let p_a = path_a.clone();
        let p_b = path_b.clone();
        let cnt_a = counter_a.clone();
        let cnt_b = counter_b.clone();

        let (res_a, res_b) = rayon::join(
            move || scan_directory_adaptive(&p_a, is_hdd_a, prune_ignored, &token_a, &cnt_a),
            move || scan_directory_adaptive(&p_b, is_hdd_b, prune_ignored, &token_b, &cnt_b),
        );

        // Stop progress ticker
        cancel_token.store(true, Ordering::Relaxed);
        let _ = ticker_handle.join();

        let map_a = res_a?;
        let map_b = res_b?;

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
        cmd.creation_flags(0x08000000);
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
