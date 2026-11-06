use std::path::Path;

#[cfg(target_os = "windows")]
pub fn is_slow_or_hdd(path: &Path) -> bool {
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
        // DRIVE_REMOVABLE (e.g. USB flash drive or SD card)
        return true;
    }

    // Query Device Seek Penalty via IOCTL_STORAGE_QUERY_PROPERTY
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
            // Fallback: If not C: drive, treat as secondary/external
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
pub fn is_slow_or_hdd(_path: &Path) -> bool {
    false
}
