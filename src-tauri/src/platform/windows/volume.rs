use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSystemKind {
    Ntfs,
    ExFat,
    Fat32,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    Ssd,
    Hdd,
    Removable,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    NtfsMft,
    Win32LargeFetchSsd,
    Win32LargeFetchHdd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Ssd,
    Hdd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanDriver {
    Mft,
    Win32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineBadge {
    pub media: MediaKind,
    pub driver: ScanDriver,
}

impl EngineBadge {
    pub fn new(media: MediaKind, driver: ScanDriver) -> Self {
        Self { media, driver }
    }

    pub fn label(&self) -> String {
        let media_str = match self.media {
            MediaKind::Ssd => "SSD",
            MediaKind::Hdd => "HDD",
        };
        let driver_str = match self.driver {
            ScanDriver::Mft => "MFT",
            ScanDriver::Win32 => "Win32",
        };
        format!("{} ({})", media_str, driver_str)
    }
}

#[derive(Debug, Clone)]
pub struct VolumeCapability {
    pub drive_letter: Option<char>,
    pub disk_number: Option<u32>,
    pub filesystem: FileSystemKind,
    pub media_type: MediaType,
    pub can_read_raw_volume: bool,
}

impl VolumeCapability {
    pub fn recommended_engine(&self) -> EngineKind {
        if self.filesystem == FileSystemKind::Ntfs && self.can_read_raw_volume {
            EngineKind::NtfsMft
        } else if self.media_type == MediaType::Hdd {
            EngineKind::Win32LargeFetchHdd
        } else {
            EngineKind::Win32LargeFetchSsd
        }
    }

    pub fn engine_badge(&self) -> EngineBadge {
        let media = if self.media_type == MediaType::Hdd {
            MediaKind::Hdd
        } else {
            MediaKind::Ssd
        };

        let driver = if self.filesystem == FileSystemKind::Ntfs && self.can_read_raw_volume {
            ScanDriver::Mft
        } else {
            ScanDriver::Win32
        };

        EngineBadge::new(media, driver)
    }
}

#[cfg(target_os = "windows")]
pub fn probe_volume(path: &Path) -> VolumeCapability {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, GetDriveTypeW, GetVolumeInformationW, FILE_SHARE_READ, FILE_SHARE_WRITE,
        OPEN_EXISTING,
    };
    use windows_sys::Win32::System::Ioctl::{
        PropertyStandardQuery, StorageDeviceSeekPenaltyProperty,
        DEVICE_SEEK_PENALTY_DESCRIPTOR, IOCTL_STORAGE_GET_DEVICE_NUMBER,
        IOCTL_STORAGE_QUERY_PROPERTY, STORAGE_DEVICE_NUMBER, STORAGE_PROPERTY_QUERY,
    };
    use windows_sys::Win32::System::IO::DeviceIoControl;

    let path_str = path.to_string_lossy();
    let drive_letter = if path_str.len() >= 2 && path_str.as_bytes()[1] == b':' {
        Some(path_str.chars().next().unwrap().to_ascii_uppercase())
    } else {
        None
    };

    let Some(dl) = drive_letter else {
        return VolumeCapability {
            drive_letter: None,
            disk_number: None,
            filesystem: FileSystemKind::Other,
            media_type: MediaType::Unknown,
            can_read_raw_volume: false,
        };
    };

    // 1. Query Volume Information (FileSystemName)
    let root_path = format!("{}:\\", dl);
    let wide_root: Vec<u16> = OsStr::new(&root_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut fs_name_buf = [0u16; 64];
    let mut flags = 0u32;
    let mut max_comp = 0u32;
    let mut serial = 0u32;

    let info_ok = unsafe {
        GetVolumeInformationW(
            wide_root.as_ptr(),
            std::ptr::null_mut(),
            0,
            &mut serial,
            &mut max_comp,
            &mut flags,
            fs_name_buf.as_mut_ptr(),
            fs_name_buf.len() as u32,
        )
    };

    let filesystem = if info_ok != 0 {
        let len = fs_name_buf
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(fs_name_buf.len());
        let fs_str = String::from_utf16_lossy(&fs_name_buf[..len]).to_uppercase();
        if fs_str.contains("NTFS") {
            FileSystemKind::Ntfs
        } else if fs_str.contains("EXFAT") {
            FileSystemKind::ExFat
        } else if fs_str.contains("FAT") {
            FileSystemKind::Fat32
        } else {
            FileSystemKind::Other
        }
    } else {
        FileSystemKind::Other
    };

    // 2. Query Drive Type
    let drive_type = unsafe { GetDriveTypeW(wide_root.as_ptr()) };
    let is_removable = drive_type == 2; // DRIVE_REMOVABLE

    // 3. Open Volume handle to test raw read & query storage properties
    let dev_path = format!("\\\\.\\{}:", dl);
    let wide_dev: Vec<u16> = OsStr::new(&dev_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let h_volume = unsafe {
        CreateFileW(
            wide_dev.as_ptr(),
            0x80000000, // GENERIC_READ
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            std::ptr::null(),
            OPEN_EXISTING,
            0,
            std::ptr::null_mut(),
        )
    };

    let can_read_raw_volume = h_volume != INVALID_HANDLE_VALUE && !h_volume.is_null();

    let mut disk_number = None;
    let mut media_type = if is_removable {
        MediaType::Removable
    } else {
        MediaType::Unknown
    };

    // If handle is opened, query device number & seek penalty
    let query_handle = if can_read_raw_volume {
        h_volume
    } else {
        // Fallback open with query metadata access (0)
        unsafe {
            CreateFileW(
                wide_dev.as_ptr(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        }
    };

    if query_handle != INVALID_HANDLE_VALUE && !query_handle.is_null() {
        // Query Physical Disk Number
        unsafe {
            let mut dev_num = std::mem::zeroed::<STORAGE_DEVICE_NUMBER>();
            let mut ret = 0u32;
            let ok = DeviceIoControl(
                query_handle,
                IOCTL_STORAGE_GET_DEVICE_NUMBER,
                std::ptr::null(),
                0,
                &mut dev_num as *mut _ as *mut _,
                std::mem::size_of::<STORAGE_DEVICE_NUMBER>() as u32,
                &mut ret,
                std::ptr::null_mut(),
            );
            if ok != 0 {
                disk_number = Some(dev_num.DeviceNumber);
            }
        }

        // Query Seek Penalty (SSD vs HDD)
        unsafe {
            let query = STORAGE_PROPERTY_QUERY {
                PropertyId: StorageDeviceSeekPenaltyProperty,
                QueryType: PropertyStandardQuery,
                AdditionalParameters: [0],
            };
            let mut desc = std::mem::zeroed::<DEVICE_SEEK_PENALTY_DESCRIPTOR>();
            let mut ret = 0u32;
            let ok = DeviceIoControl(
                query_handle,
                IOCTL_STORAGE_QUERY_PROPERTY,
                &query as *const _ as *const _,
                std::mem::size_of::<STORAGE_PROPERTY_QUERY>() as u32,
                &mut desc as *mut _ as *mut _,
                std::mem::size_of::<DEVICE_SEEK_PENALTY_DESCRIPTOR>() as u32,
                &mut ret,
                std::ptr::null_mut(),
            );
            if ok != 0 {
                if desc.IncursSeekPenalty {
                    media_type = MediaType::Hdd;
                } else {
                    media_type = MediaType::Ssd;
                }
            }
        }

        unsafe {
            CloseHandle(query_handle);
        }
    }

    if can_read_raw_volume && query_handle != h_volume {
        unsafe {
            CloseHandle(h_volume);
        }
    }

    // Default fallback if media type not detected: C: is usually SSD, others assume HDD if seek unknown
    if media_type == MediaType::Unknown {
        media_type = if dl == 'C' {
            MediaType::Ssd
        } else {
            MediaType::Hdd
        };
    }

    VolumeCapability {
        drive_letter: Some(dl),
        disk_number,
        filesystem,
        media_type,
        can_read_raw_volume,
    }
}

#[cfg(not(target_os = "windows"))]
pub fn probe_volume(_path: &Path) -> VolumeCapability {
    VolumeCapability {
        drive_letter: None,
        disk_number: None,
        filesystem: FileSystemKind::Other,
        media_type: MediaType::Ssd,
        can_read_raw_volume: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_badge_labels() {
        assert_eq!(
            EngineBadge::new(MediaKind::Ssd, ScanDriver::Mft).label(),
            "SSD (MFT)"
        );
        assert_eq!(
            EngineBadge::new(MediaKind::Ssd, ScanDriver::Win32).label(),
            "SSD (Win32)"
        );
        assert_eq!(
            EngineBadge::new(MediaKind::Hdd, ScanDriver::Mft).label(),
            "HDD (MFT)"
        );
        assert_eq!(
            EngineBadge::new(MediaKind::Hdd, ScanDriver::Win32).label(),
            "HDD (Win32)"
        );
    }

    #[test]
    fn test_volume_capability_engine_badge() {
        let cap_ssd_mft = VolumeCapability {
            drive_letter: Some('C'),
            disk_number: Some(0),
            filesystem: FileSystemKind::Ntfs,
            media_type: MediaType::Ssd,
            can_read_raw_volume: true,
        };
        assert_eq!(cap_ssd_mft.engine_badge().label(), "SSD (MFT)");

        let cap_ssd_win32 = VolumeCapability {
            drive_letter: Some('C'),
            disk_number: Some(0),
            filesystem: FileSystemKind::Ntfs,
            media_type: MediaType::Ssd,
            can_read_raw_volume: false,
        };
        assert_eq!(cap_ssd_win32.engine_badge().label(), "SSD (Win32)");

        let cap_hdd_mft = VolumeCapability {
            drive_letter: Some('D'),
            disk_number: Some(1),
            filesystem: FileSystemKind::Ntfs,
            media_type: MediaType::Hdd,
            can_read_raw_volume: true,
        };
        assert_eq!(cap_hdd_mft.engine_badge().label(), "HDD (MFT)");

        let cap_hdd_win32 = VolumeCapability {
            drive_letter: Some('D'),
            disk_number: Some(1),
            filesystem: FileSystemKind::ExFat,
            media_type: MediaType::Hdd,
            can_read_raw_volume: false,
        };
        assert_eq!(cap_hdd_win32.engine_badge().label(), "HDD (Win32)");
    }
}
