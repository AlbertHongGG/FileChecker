use super::win32_engine::Win32LargeFetchEngine;
use super::{ScanEngine, ScanObserver, ScanPolicy, ScanSnapshot};
use crate::error::{AppError, AppResult};
use crate::models::FileMetadata;
use crate::platform::windows::win32_fs::{get_directory_frn, SafeHandle};
use rayon::prelude::*;
use rustc_hash::FxHashMap;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, SetFilePointerEx, FILE_BEGIN, FILE_SHARE_READ, FILE_SHARE_WRITE,
    OPEN_EXISTING,
};
use windows_sys::Win32::System::IO::DeviceIoControl;

const FSCTL_GET_NTFS_VOLUME_DATA: u32 = 0x00090064;
const FSCTL_GET_NTFS_FILE_RECORD: u32 = 0x00090068;

#[derive(Debug, Clone)]
struct ParsedRecord {
    record_id: u64,
    parent_id: u64,
    name: String,
    is_dir: bool,
    size: u64,
    modified_timestamp: u64,
}

pub struct NtfsMftEngine {
    pub is_hdd: bool,
}

impl NtfsMftEngine {
    pub fn new(is_hdd: bool) -> Self {
        Self { is_hdd }
    }
}

impl ScanEngine for NtfsMftEngine {
    fn scan(
        &self,
        root: &Path,
        policy: &ScanPolicy,
        observer: &ScanObserver,
    ) -> AppResult<ScanSnapshot> {
        match scan_ntfs_mft_internal(root, policy, observer) {
            Ok(snapshot) => Ok(snapshot),
            Err(e) => {
                if matches!(e, AppError::ScanCancelled) {
                    return Err(e);
                }
                // Robust Fallback: gracefully fallback to Win32 LargeFetch engine
                let fallback = Win32LargeFetchEngine::new(self.is_hdd);
                fallback.scan(root, policy, observer)
            }
        }
    }
}

fn scan_ntfs_mft_internal(
    root: &Path,
    policy: &ScanPolicy,
    observer: &ScanObserver,
) -> AppResult<ScanSnapshot> {
    let path_str = root.to_string_lossy();
    if path_str.len() < 2 || path_str.as_bytes()[1] != b':' {
        return Err(AppError::TaskExecution("Invalid drive path".into()));
    }
    let drive_letter = path_str.chars().next().unwrap().to_ascii_uppercase();

    // 1. Resolve Target Directory FRN
    let is_root = path_str.len() <= 3
        || (path_str.len() == 3 && path_str.ends_with('\\'))
        || (path_str.len() == 2 && path_str.ends_with(':'));

    let target_frn = if is_root {
        5u64 // NTFS Root Directory is always FRN 5
    } else {
        get_directory_frn(root)
            .ok_or_else(|| AppError::TaskExecution("Failed to get target folder FRN".into()))?
    };

    // 2. Open Volume handle
    let dev_path = format!("\\\\.\\{}:", drive_letter);
    let wide_dev: Vec<u16> = OsStr::new(&dev_path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let h_vol = unsafe {
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

    if h_vol == INVALID_HANDLE_VALUE || h_vol.is_null() {
        return Err(AppError::TaskExecution("Failed to open volume handle".into()));
    }
    let safe_vol = SafeHandle::new(h_vol);

    // 3. Query NTFS Volume Data
    let mut vol_buf = [0u8; 128];
    let mut bytes_returned = 0u32;
    let ok = unsafe {
        DeviceIoControl(
            safe_vol.handle(),
            FSCTL_GET_NTFS_VOLUME_DATA,
            std::ptr::null(),
            0,
            vol_buf.as_mut_ptr() as *mut _,
            vol_buf.len() as u32,
            &mut bytes_returned,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        return Err(AppError::TaskExecution("FSCTL_GET_NTFS_VOLUME_DATA failed".into()));
    }

    let bytes_per_cluster = u32::from_le_bytes(vol_buf[44..48].try_into().unwrap()) as u64;
    let mft_valid_data_length = u64::from_le_bytes(vol_buf[56..64].try_into().unwrap());
    let mut max_records = (mft_valid_data_length / 1024) as u64;

    // 4. Retrieve Record 0 ($MFT) to extract Data Runs & $BITMAP
    let mut in_frn = 0u64; // FRN 0 is $MFT
    let mut out_rec_buf = vec![0u8; 4096];
    let ok = unsafe {
        DeviceIoControl(
            safe_vol.handle(),
            FSCTL_GET_NTFS_FILE_RECORD,
            &mut in_frn as *mut _ as *mut _,
            8,
            out_rec_buf.as_mut_ptr() as *mut _,
            out_rec_buf.len() as u32,
            &mut bytes_returned,
            std::ptr::null_mut(),
        )
    };
    if ok == 0 || bytes_returned < 1036 {
        return Err(AppError::TaskExecution("Failed to read MFT Record 0".into()));
    }

    // Record 0 begins at out_rec_buf[12..1036]
    let rec0 = &out_rec_buf[12..1036];
    let (data_runs, highest_record) = parse_mft_record_zero(rec0, safe_vol.handle(), bytes_per_cluster);
    if let Some(hr) = highest_record {
        if hr + 1 < max_records {
            max_records = hr + 1;
        }
    }

    if data_runs.is_empty() {
        return Err(AppError::TaskExecution("No MFT data runs found".into()));
    }

    // 5. Read all in-use MFT records sequentially in 8MB chunks
    let chunk_size = 8 * 1024 * 1024; // 8 MB = 8,192 records per read
    let mut read_buf = vec![0u8; chunk_size];
    let mut records_parsed = Vec::with_capacity(max_records.min(1_500_000) as usize);

    let mut current_record_idx = 0u64;
    let total_records_to_scan = max_records;

    observer.set_progress(0, total_records_to_scan as usize);
    observer.set_current_name("Reading MFT table...");

    for (lcn, cluster_count) in data_runs {
        if current_record_idx >= total_records_to_scan {
            break;
        }

        let run_bytes = cluster_count * bytes_per_cluster;
        let mut run_offset = 0u64;

        while run_offset < run_bytes && current_record_idx < total_records_to_scan {
            if observer.is_cancelled() {
                return Err(AppError::ScanCancelled);
            }

            let disk_offset = (lcn * bytes_per_cluster) + run_offset;
            let to_read = ((run_bytes - run_offset) as usize).min(chunk_size);

            unsafe {
                let mut new_pos = 0i64;
                SetFilePointerEx(safe_vol.handle(), disk_offset as i64, &mut new_pos, FILE_BEGIN);
            }

            let mut bytes_read = 0u32;
            let read_ok = unsafe {
                ReadFile(
                    safe_vol.handle(),
                    read_buf.as_mut_ptr() as *mut _,
                    to_read as u32,
                    &mut bytes_read,
                    std::ptr::null_mut(),
                )
            };

            if read_ok == 0 || bytes_read < 1024 {
                break;
            }

            let valid_len = (bytes_read as usize) / 1024 * 1024;
            let start_id = current_record_idx;

            let parsed_chunk: Vec<ParsedRecord> = read_buf[..valid_len]
                .par_chunks_exact(1024)
                .enumerate()
                .filter_map(|(idx, chunk)| {
                    let record_id = start_id + idx as u64;
                    parse_single_file_record(chunk, record_id)
                })
                .collect();

            let records_in_chunk = valid_len / 1024;
            current_record_idx += records_in_chunk as u64;
            run_offset += valid_len as u64;

            records_parsed.extend(parsed_chunk);
            observer.set_progress(current_record_idx as usize, total_records_to_scan as usize);
        }
    }

    if observer.is_cancelled() {
        return Err(AppError::ScanCancelled);
    }

    // 6. In-Memory Subtree Construction from target_frn
    observer.set_current_name("Building directory tree...");

    let mut parent_to_children: FxHashMap<u64, Vec<usize>> = FxHashMap::default();
    for (idx, item) in records_parsed.iter().enumerate() {
        parent_to_children
            .entry(item.parent_id)
            .or_default()
            .push(idx);
    }

    let mut snapshot = FxHashMap::default();
    let mut stack: Vec<(u64, String)> = vec![(target_frn, String::new())];
    let mut file_count = 0usize;

    while let Some((parent_id, prefix)) = stack.pop() {
        if observer.is_cancelled() {
            return Err(AppError::ScanCancelled);
        }

        if let Some(children_indices) = parent_to_children.get(&parent_id) {
            for &c_idx in children_indices {
                let child = &records_parsed[c_idx];
                if child.record_id == parent_id {
                    continue; // Skip cyclic self-reference
                }

                if child.is_dir {
                    if policy.should_prune(&child.name) {
                        continue;
                    }
                    let rel_path = if prefix.is_empty() {
                        child.name.clone()
                    } else {
                        format!("{}/{}", prefix, child.name)
                    };

                    snapshot.insert(
                        rel_path.clone(),
                        FileMetadata {
                            size: 0,
                            modified_timestamp: 0,
                            is_dir: true,
                        },
                    );
                    stack.push((child.record_id, rel_path));
                } else {
                    let rel_path = if prefix.is_empty() {
                        child.name.clone()
                    } else {
                        format!("{}/{}", prefix, child.name)
                    };

                    snapshot.insert(
                        rel_path,
                        FileMetadata {
                            size: child.size,
                            modified_timestamp: child.modified_timestamp,
                            is_dir: false,
                        },
                    );
                    file_count += 1;
                    if file_count % 512 == 0 {
                        observer.inc_files(512);
                    }
                }
            }
        }
    }

    let remainder = file_count % 512;
    if remainder > 0 {
        observer.inc_files(remainder);
    }

    observer.set_current_name("");
    Ok(snapshot)
}

fn parse_mft_record_zero(
    rec0: &[u8],
    h_vol: windows_sys::Win32::Foundation::HANDLE,
    bytes_per_cluster: u64,
) -> (Vec<(u64, u64)>, Option<u64>) {
    if &rec0[0..4] != b"FILE" {
        return (Vec::new(), None);
    }

    let first_attr_offset = u16::from_le_bytes(rec0[20..22].try_into().unwrap()) as usize;
    let mut cur = first_attr_offset;
    let mut data_runs = Vec::new();
    let mut highest_record = None;

    while cur + 8 <= rec0.len() {
        let attr_type = u32::from_le_bytes(rec0[cur..cur + 4].try_into().unwrap());
        if attr_type == 0xFFFF_FFFF || attr_type == 0 {
            break;
        }
        let attr_len = u32::from_le_bytes(rec0[cur + 4..cur + 8].try_into().unwrap()) as usize;
        if attr_len == 0 || cur + attr_len > rec0.len() {
            break;
        }

        let non_resident = rec0[cur + 8];

        if attr_type == 0x80 && non_resident == 1 {
            // $DATA attribute: parse data runs
            let run_offset = u16::from_le_bytes(rec0[cur + 32..cur + 34].try_into().unwrap()) as usize;
            data_runs = parse_data_runs(&rec0[cur + run_offset..cur + attr_len]);
        } else if attr_type == 0xB0 && non_resident == 1 {
            // $BITMAP attribute: parse bitmap to determine active records
            let run_offset = u16::from_le_bytes(rec0[cur + 32..cur + 34].try_into().unwrap()) as usize;
            let bmp_runs = parse_data_runs(&rec0[cur + run_offset..cur + attr_len]);
            let real_size = u64::from_le_bytes(rec0[cur + 48..cur + 56].try_into().unwrap());

            if let Some(hr) = scan_mft_bitmap(h_vol, &bmp_runs, bytes_per_cluster, real_size) {
                highest_record = Some(hr);
            }
        }

        cur += attr_len;
    }

    (data_runs, highest_record)
}

fn parse_data_runs(data: &[u8]) -> Vec<(u64, u64)> {
    let mut runs = Vec::new();
    let mut r = 0;
    let mut current_lcn = 0i64;

    while r < data.len() {
        let header = data[r];
        r += 1;
        if header == 0 {
            break;
        }

        let len_bytes = (header & 0x0F) as usize;
        let off_bytes = ((header >> 4) & 0x0F) as usize;

        if r + len_bytes + off_bytes > data.len() {
            break;
        }

        let mut cluster_count = 0u64;
        for i in 0..len_bytes {
            cluster_count |= (data[r + i] as u64) << (i * 8);
        }
        r += len_bytes;

        let mut lcn_delta = 0i64;
        for i in 0..off_bytes {
            lcn_delta |= (data[r + i] as i64) << (i * 8);
        }
        r += off_bytes;

        // Sign extend lcn_delta if negative
        if off_bytes > 0 && (data[r - 1] & 0x80) != 0 {
            for i in off_bytes..8 {
                lcn_delta |= 0xFFi64 << (i * 8);
            }
        }

        current_lcn += lcn_delta;
        if current_lcn > 0 && cluster_count > 0 {
            runs.push((current_lcn as u64, cluster_count));
        }
    }

    runs
}

fn scan_mft_bitmap(
    h_vol: windows_sys::Win32::Foundation::HANDLE,
    bmp_runs: &[(u64, u64)],
    bytes_per_cluster: u64,
    real_size: u64,
) -> Option<u64> {
    let mut bit_index = 0u64;
    let mut highest = 0u64;
    let mut buf = vec![0u8; 64 * 1024];

    for &(lcn, cluster_count) in bmp_runs {
        let run_bytes = cluster_count * bytes_per_cluster;
        let mut run_offset = 0u64;

        while run_offset < run_bytes && (bit_index / 8) < real_size {
            let disk_offset = (lcn * bytes_per_cluster) + run_offset;
            let to_read = (run_bytes - run_offset).min(buf.len() as u64) as usize;

            unsafe {
                let mut new_pos = 0i64;
                SetFilePointerEx(h_vol, disk_offset as i64, &mut new_pos, FILE_BEGIN);
            }

            let mut bytes_read = 0u32;
            let ok = unsafe {
                ReadFile(
                    h_vol,
                    buf.as_mut_ptr() as *mut _,
                    to_read as u32,
                    &mut bytes_read,
                    std::ptr::null_mut(),
                )
            };

            if ok == 0 || bytes_read == 0 {
                break;
            }

            for b in 0..bytes_read as usize {
                if (bit_index / 8) >= real_size {
                    break;
                }
                let val = buf[b];
                if val != 0 {
                    for bit in 0..8 {
                        if (val & (1 << bit)) != 0 {
                            highest = bit_index + bit;
                        }
                    }
                }
                bit_index += 8;
            }

            run_offset += bytes_read as u64;
        }
    }

    if highest > 0 {
        Some(highest)
    } else {
        None
    }
}

fn parse_single_file_record(record_bytes: &[u8], record_id: u64) -> Option<ParsedRecord> {
    if &record_bytes[0..4] != b"FILE" {
        return None;
    }

    // Apply USA Fixup on a stack copy
    let mut rec = [0u8; 1024];
    rec.copy_from_slice(record_bytes);

    let usa_offset = u16::from_le_bytes(rec[4..6].try_into().unwrap()) as usize;
    let usa_size = u16::from_le_bytes(rec[6..8].try_into().unwrap()) as usize;

    if usa_offset + (usa_size * 2) <= 1024 && usa_size >= 2 {
        let update_seq = u16::from_le_bytes(rec[usa_offset..usa_offset + 2].try_into().unwrap());
        for i in 1..usa_size {
            let sector_end = i * 512 - 2;
            if sector_end + 2 <= 1024 {
                let disk_seq = u16::from_le_bytes(rec[sector_end..sector_end + 2].try_into().unwrap());
                if disk_seq == update_seq {
                    let b0 = rec[usa_offset + i * 2];
                    let b1 = rec[usa_offset + i * 2 + 1];
                    rec[sector_end] = b0;
                    rec[sector_end + 1] = b1;
                }
            }
        }
    }

    let flags = u16::from_le_bytes(rec[22..24].try_into().unwrap());
    if (flags & 0x0001) == 0 {
        return None; // Record deleted or inactive
    }
    let is_dir = (flags & 0x0002) != 0;

    let mut cur = u16::from_le_bytes(rec[20..22].try_into().unwrap()) as usize;
    let mut modified_timestamp = 0u64;
    let mut parent_id = 0u64;
    let mut file_size = 0u64;
    let mut name = String::new();
    let mut chosen_namespace = 255u8;

    while cur + 8 <= 1024 {
        let attr_type = u32::from_le_bytes(rec[cur..cur + 4].try_into().unwrap());
        if attr_type == 0xFFFF_FFFF || attr_type == 0 {
            break;
        }
        let attr_len = u32::from_le_bytes(rec[cur + 4..cur + 8].try_into().unwrap()) as usize;
        if attr_len == 0 || cur + attr_len > 1024 {
            break;
        }

        let non_resident = rec[cur + 8];

        if attr_type == 0x10 && non_resident == 0 {
            // $STANDARD_INFORMATION: extract Last Modified Time
            let val_offset = u16::from_le_bytes(rec[cur + 20..cur + 22].try_into().unwrap()) as usize;
            let val_start = cur + val_offset;
            if val_start + 16 <= cur + attr_len {
                let ft = u64::from_le_bytes(rec[val_start + 8..val_start + 16].try_into().unwrap());
                if ft >= 116_444_736_000_000_000 {
                    modified_timestamp = (ft - 116_444_736_000_000_000) / 10_000;
                }
            }
        } else if attr_type == 0x30 && non_resident == 0 {
            // $FILE_NAME
            let val_offset = u16::from_le_bytes(rec[cur + 20..cur + 22].try_into().unwrap()) as usize;
            let val_start = cur + val_offset;
            if val_start + 66 <= cur + attr_len {
                let p_frn = u64::from_le_bytes(rec[val_start..val_start + 8].try_into().unwrap())
                    & 0x0000_FFFF_FFFF_FFFF;
                let n_len = rec[val_start + 64] as usize;
                let ns = rec[val_start + 65];

                if val_start + 66 + (n_len * 2) <= cur + attr_len {
                    // Namespace preference: Win32 (1) or Win32&DOS (3) > POSIX (0) > DOS (2)
                    let should_take = match (chosen_namespace, ns) {
                        (255, _) => true,
                        (2, _) => true, // Replace DOS 8.3 name with any real name
                        (_, 1 | 3) => true,
                        _ => false,
                    };

                    if should_take {
                        let utf16_slice: &[u16] = unsafe {
                            std::slice::from_raw_parts(
                                rec[val_start + 66..].as_ptr() as *const u16,
                                n_len,
                            )
                        };
                        name = String::from_utf16_lossy(utf16_slice);
                        parent_id = p_frn;
                        chosen_namespace = ns;

                        // Fallback size from $FILE_NAME if $DATA not present yet
                        if file_size == 0 && !is_dir {
                            file_size = u64::from_le_bytes(
                                rec[val_start + 48..val_start + 56].try_into().unwrap(),
                            );
                        }
                    }
                }
            }
        } else if attr_type == 0x80 && !is_dir {
            // $DATA: extract exact file size
            let name_len = rec[cur + 9];
            if name_len == 0 {
                // Default unnamed $DATA stream
                if non_resident == 0 {
                    let val_len = u32::from_le_bytes(rec[cur + 16..cur + 20].try_into().unwrap());
                    file_size = val_len as u64;
                } else if cur + 56 <= 1024 {
                    file_size = u64::from_le_bytes(rec[cur + 48..cur + 56].try_into().unwrap());
                }
            }
        }

        cur += attr_len;
    }

    if name.is_empty() || name == "." || name == ".." {
        return None;
    }

    Some(ParsedRecord {
        record_id,
        parent_id,
        name,
        is_dir,
        size: if is_dir { 0 } else { file_size },
        modified_timestamp,
    })
}
