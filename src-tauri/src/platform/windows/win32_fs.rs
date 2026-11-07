use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FindClose, FindFirstFileExW, FindNextFileW, GetFileInformationByHandle,
    BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FINDEX_INFO_LEVELS,
    FINDEX_SEARCH_OPS, OPEN_EXISTING, WIN32_FIND_DATAW,
};

// FindExInfoBasic = 1, FindExSearchNameMatch = 0, FIND_FIRST_EX_LARGE_FETCH = 2
const FIND_EX_INFO_BASIC: FINDEX_INFO_LEVELS = 1;
const FIND_EX_SEARCH_NAME_MATCH: FINDEX_SEARCH_OPS = 0;
const FIND_FIRST_EX_LARGE_FETCH: u32 = 0x00000002;

/// RAII wrapper for Win32 HANDLE to prevent resource leakage.
#[derive(Debug)]
pub struct SafeHandle(pub HANDLE);

impl SafeHandle {
    pub fn new(h: HANDLE) -> Self {
        Self(h)
    }

    pub fn is_valid(&self) -> bool {
        self.0 != INVALID_HANDLE_VALUE && !self.0.is_null()
    }

    pub fn handle(&self) -> HANDLE {
        self.0
    }
}

impl Drop for SafeHandle {
    fn drop(&mut self) {
        if self.is_valid() {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct RawDirEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified_timestamp: u64,
}

/// Fast Win32 directory iterator using FindFirstFileExW (LargeFetch).
/// Extracts file size and timestamp directly from WIN32_FIND_DATAW with ZERO extra syscalls.
pub fn read_dir_large_fetch(dir: &Path) -> std::io::Result<Vec<RawDirEntry>> {
    let mut search_path = dir.to_path_buf();
    search_path.push("*");

    let wide_path: Vec<u16> = search_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut find_data: WIN32_FIND_DATAW = unsafe { std::mem::zeroed() };
    let find_handle = unsafe {
        FindFirstFileExW(
            wide_path.as_ptr(),
            FIND_EX_INFO_BASIC,
            &mut find_data as *mut _ as *mut _,
            FIND_EX_SEARCH_NAME_MATCH,
            std::ptr::null(),
            FIND_FIRST_EX_LARGE_FETCH,
        )
    };

    if find_handle == INVALID_HANDLE_VALUE || find_handle.is_null() {
        return Ok(Vec::new());
    }

    struct FindGuard(HANDLE);
    impl Drop for FindGuard {
        fn drop(&mut self) {
            unsafe {
                FindClose(self.0);
            }
        }
    }
    let _guard = FindGuard(find_handle);

    let mut entries = Vec::with_capacity(32);

    loop {
        let name_len = find_data
            .cFileName
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(find_data.cFileName.len());

        let is_dot = name_len == 1 && find_data.cFileName[0] == b'.' as u16;
        let is_dotdot = name_len == 2
            && find_data.cFileName[0] == b'.' as u16
            && find_data.cFileName[1] == b'.' as u16;

        if !is_dot && !is_dotdot && name_len > 0 {
            let name = String::from_utf16_lossy(&find_data.cFileName[..name_len]);
            let is_dir = (find_data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) != 0;
            let size = if is_dir {
                0
            } else {
                ((find_data.nFileSizeHigh as u64) << 32) | (find_data.nFileSizeLow as u64)
            };

            let ft = ((find_data.ftLastWriteTime.dwHighDateTime as u64) << 32)
                | (find_data.ftLastWriteTime.dwLowDateTime as u64);

            // Convert Windows FILETIME (100-nanosecond intervals since January 1, 1601) to Unix timestamp (ms)
            let modified_timestamp = if ft >= 116_444_736_000_000_000 {
                (ft - 116_444_736_000_000_000) / 10_000
            } else {
                0
            };

            entries.push(RawDirEntry {
                name,
                is_dir,
                size,
                modified_timestamp,
            });
        }

        let next = unsafe { FindNextFileW(find_handle, &mut find_data) };
        if next == 0 {
            break;
        }
    }

    Ok(entries)
}

/// Retrieve the 48-bit NTFS File Reference Number (Record Index) of a directory.
pub fn get_directory_frn(path: &Path) -> Option<u64> {
    let wide_path: Vec<u16> = OsStr::new(path.as_os_str())
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let h = unsafe {
        CreateFileW(
            wide_path.as_ptr(),
            0,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            std::ptr::null_mut(),
        )
    };

    if h == INVALID_HANDLE_VALUE || h.is_null() {
        return None;
    }

    let safe_h = SafeHandle::new(h);
    let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
    let ok = unsafe { GetFileInformationByHandle(safe_h.handle(), &mut info) };
    if ok != 0 {
        let full_frn = ((info.nFileIndexHigh as u64) << 32) | (info.nFileIndexLow as u64);
        let record_num = full_frn & 0x0000_FFFF_FFFF_FFFF;
        Some(record_num)
    } else {
        None
    }
}
