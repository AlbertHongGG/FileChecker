use super::record_parser::{MftNode, MftRecordParser};
use crate::error::{AppError, AppResult};
use crate::platform::windows::win32_fs::SafeHandle;
use rayon::prelude::*;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, GetVolumeInformationW, ReadFile, SetFilePointerEx, FILE_BEGIN, FILE_SHARE_READ,
    FILE_SHARE_WRITE, OPEN_EXISTING,
};
use windows_sys::Win32::System::IO::DeviceIoControl;

const FSCTL_GET_NTFS_VOLUME_DATA: u32 = 0x00090064;
const FSCTL_GET_NTFS_FILE_RECORD: u32 = 0x00090068;
const FSCTL_QUERY_USN_JOURNAL: u32 = 0x000900f4;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
struct UsnJournalDataV0 {
    usn_journal_id: u64,
    lowest_usn: i64,
    next_usn: i64,
    maximum_size: u64,
    allocation_delta: u64,
}

#[derive(Debug, Clone)]
pub struct DataRun {
    pub lcn: u64,
    pub cluster_count: u64,
    pub start_record: u64,
    pub record_count: u64,
}

#[derive(Debug, Clone)]
struct MftReadSegment {
    disk_offset: u64,
    byte_len: usize,
    start_record: u64,
}

pub struct VolumeUsnState {
    pub volume_serial: u64,
    pub journal_id: u64,
    pub next_usn: i64,
}

pub struct NtfsVolumeReader {
    pub drive_letter: char,
    safe_handle: SafeHandle,
    pub bytes_per_cluster: u64,
    pub max_records: u64,
    pub volume_serial: u64,
}

impl NtfsVolumeReader {
    pub fn open(drive_letter: char) -> AppResult<Self> {
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
            return Err(AppError::TaskExecution(format!(
                "Failed to open volume handle for {}: (Admin privileges required)",
                drive_letter
            )));
        }

        let safe_handle = SafeHandle::new(h_vol);

        // Query NTFS volume data
        let mut vol_buf = [0u8; 128];
        let mut bytes_returned = 0u32;
        let ok = unsafe {
            DeviceIoControl(
                safe_handle.handle(),
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
            return Err(AppError::TaskExecution(format!(
                "FSCTL_GET_NTFS_VOLUME_DATA failed on drive {}:",
                drive_letter
            )));
        }

        let bytes_per_cluster = u32::from_le_bytes(vol_buf[44..48].try_into().unwrap()) as u64;
        let mft_valid_data_length = u64::from_le_bytes(vol_buf[56..64].try_into().unwrap());
        let max_records = mft_valid_data_length / 1024;

        // Query volume serial
        let root_path = format!("{}:\\", drive_letter);
        let wide_root: Vec<u16> = OsStr::new(&root_path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut serial = 0u32;
        unsafe {
            GetVolumeInformationW(
                wide_root.as_ptr(),
                std::ptr::null_mut(),
                0,
                &mut serial,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            );
        }

        Ok(Self {
            drive_letter,
            safe_handle,
            bytes_per_cluster,
            max_records,
            volume_serial: serial as u64,
        })
    }

    pub fn query_usn_state(&self) -> Option<VolumeUsnState> {
        let mut usn_data = UsnJournalDataV0::default();
        let mut bytes_returned = 0u32;
        let ok = unsafe {
            DeviceIoControl(
                self.safe_handle.handle(),
                FSCTL_QUERY_USN_JOURNAL,
                std::ptr::null(),
                0,
                &mut usn_data as *mut _ as *mut _,
                std::mem::size_of::<UsnJournalDataV0>() as u32,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };

        if ok != 0 && bytes_returned >= std::mem::size_of::<UsnJournalDataV0>() as u32 {
            Some(VolumeUsnState {
                volume_serial: self.volume_serial,
                journal_id: usn_data.usn_journal_id,
                next_usn: usn_data.next_usn,
            })
        } else {
            None
        }
    }

    /// Read Record 0 ($MFT), extract Data Runs and $BITMAP, skip empty 4MB chunks,
    /// order remaining chunks by LCN elevator schedule, and read into MftNode collection.
    pub fn read_all_nodes<F>(&mut self, mut on_progress: F, is_cancelled: impl Fn() -> bool) -> AppResult<Vec<MftNode>>
    where
        F: FnMut(usize, usize),
    {
        // 1. Read Record 0 ($MFT)
        let mut in_frn = 0u64;
        let mut out_rec_buf = vec![0u8; 4096];
        let mut bytes_returned = 0u32;

        let ok = unsafe {
            DeviceIoControl(
                self.safe_handle.handle(),
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

        let rec0 = &out_rec_buf[12..1036];
        let (raw_data_runs, raw_bitmap_runs, bitmap_real_size) =
            self.parse_mft_record_zero(rec0)?;

        if raw_data_runs.is_empty() {
            return Err(AppError::TaskExecution("No MFT data runs found".into()));
        }

        // 2. Read $BITMAP into memory with elevator sorting
        let bitmap_bytes = self.read_bitmap(&raw_bitmap_runs, bitmap_real_size)?;

        // Find true highest active record from bitmap
        let mut highest_active_record = None;
        for (byte_idx, &byte_val) in bitmap_bytes.iter().enumerate().rev() {
            if byte_val != 0 {
                for bit in (0..8).rev() {
                    if (byte_val & (1 << bit)) != 0 {
                        highest_active_record = Some((byte_idx as u64 * 8) + bit);
                        break;
                    }
                }
                break;
            }
        }

        if let Some(har) = highest_active_record {
            if har + 1 < self.max_records {
                self.max_records = har + 1;
            }
        }

        // 3. Convert raw Data Runs into indexed Data Runs
        let mut indexed_runs = Vec::new();
        let mut cur_rec = 0u64;
        for (lcn, cluster_count) in raw_data_runs {
            let records_in_run = (cluster_count * self.bytes_per_cluster) / 1024;
            indexed_runs.push(DataRun {
                lcn,
                cluster_count,
                start_record: cur_rec,
                record_count: records_in_run,
            });
            cur_rec += records_in_run;
        }

        // 4. Build 4MB (4,096 records) segments and filter out inactive ones using bitmap
        const CHUNK_RECORDS: u64 = 4096; // 4MB
        let mut active_segments: Vec<MftReadSegment> = Vec::new();

        for run in &indexed_runs {
            if run.start_record >= self.max_records {
                break;
            }

            let mut run_record_offset = 0u64;
            while run_record_offset < run.record_count {
                let seg_start_rec = run.start_record + run_record_offset;
                if seg_start_rec >= self.max_records {
                    break;
                }

                let seg_rec_count = (run.record_count - run_record_offset)
                    .min(CHUNK_RECORDS)
                    .min(self.max_records - seg_start_rec);

                // Bitmap check: does [seg_start_rec..seg_start_rec + seg_rec_count] have any active bits?
                let start_byte = (seg_start_rec / 8) as usize;
                let end_byte = (((seg_start_rec + seg_rec_count + 7) / 8) as usize)
                    .min(bitmap_bytes.len());

                let has_active = if start_byte < bitmap_bytes.len() {
                    bitmap_bytes[start_byte..end_byte].iter().any(|&b| b != 0)
                } else {
                    false
                };

                if has_active {
                    let disk_offset = (run.lcn * self.bytes_per_cluster) + (run_record_offset * 1024);
                    active_segments.push(MftReadSegment {
                        disk_offset,
                        byte_len: (seg_rec_count * 1024) as usize,
                        start_record: seg_start_rec,
                    });
                }

                run_record_offset += seg_rec_count;
            }
        }

        // 5. Elevator Scheduling: Sort active segments by physical LCN disk offset
        active_segments.sort_unstable_by_key(|s| s.disk_offset);

        // 6. Execute elevator reads with contiguous coalescing
        let total_active_segments = active_segments.len();
        let mut records_parsed = Vec::with_capacity(self.max_records.min(2_000_000) as usize);

        let mut read_buf = vec![0u8; 8 * 1024 * 1024]; // 8MB read buffer
        let mut seg_idx = 0;

        while seg_idx < total_active_segments {
            if is_cancelled() {
                return Err(AppError::ScanCancelled);
            }

            // Coalesce contiguous segments up to 8MB
            let first_seg = &active_segments[seg_idx];
            let start_offset = first_seg.disk_offset;
            let mut batch_bytes = 0usize;
            let batch_start_idx = seg_idx;

            while seg_idx < total_active_segments {
                let seg = &active_segments[seg_idx];
                let is_contiguous = seg_idx == batch_start_idx
                    || seg.disk_offset == active_segments[seg_idx - 1].disk_offset + active_segments[seg_idx - 1].byte_len as u64;

                if is_contiguous && (batch_bytes + seg.byte_len <= read_buf.len()) {
                    batch_bytes += seg.byte_len;
                    seg_idx += 1;
                } else {
                    break;
                }
            }

            // Read batch from disk
            unsafe {
                let mut new_pos = 0i64;
                SetFilePointerEx(self.safe_handle.handle(), start_offset as i64, &mut new_pos, FILE_BEGIN);
            }

            let mut bytes_read = 0u32;
            let read_ok = unsafe {
                ReadFile(
                    self.safe_handle.handle(),
                    read_buf.as_mut_ptr() as *mut _,
                    batch_bytes as u32,
                    &mut bytes_read,
                    std::ptr::null_mut(),
                )
            };

            if read_ok == 0 || bytes_read == 0 {
                continue;
            }

            // Parse each segment within the batch
            let mut batch_offset = 0usize;
            for s in &active_segments[batch_start_idx..seg_idx] {
                if batch_offset + s.byte_len > bytes_read as usize {
                    break;
                }

                let seg_slice = &read_buf[batch_offset..batch_offset + s.byte_len];
                let seg_start_rec = s.start_record;

                let parsed_chunk: Vec<MftNode> = seg_slice
                    .par_chunks_exact(1024)
                    .enumerate()
                    .filter_map(|(idx, rec)| {
                        let rec_id = seg_start_rec + idx as u64;
                        MftRecordParser::parse_record(rec, rec_id)
                    })
                    .collect();

                records_parsed.extend(parsed_chunk);
                batch_offset += s.byte_len;
            }

            on_progress(seg_idx, total_active_segments);
        }

        Ok(records_parsed)
    }

    fn parse_mft_record_zero(&self, rec0: &[u8]) -> AppResult<(Vec<(u64, u64)>, Vec<(u64, u64)>, u64)> {
        if &rec0[0..4] != b"FILE" {
            return Err(AppError::TaskExecution("Invalid Record 0 header".into()));
        }

        let first_attr_offset = u16::from_le_bytes(rec0[20..22].try_into().unwrap()) as usize;
        let mut cur = first_attr_offset;
        let mut data_runs = Vec::new();
        let mut bitmap_runs = Vec::new();
        let mut bitmap_real_size = 0u64;

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
                // $DATA runs
                let run_offset = u16::from_le_bytes(rec0[cur + 32..cur + 34].try_into().unwrap()) as usize;
                data_runs = parse_data_run_stream(&rec0[cur + run_offset..cur + attr_len]);
            } else if attr_type == 0xB0 && non_resident == 1 {
                // $BITMAP runs
                let run_offset = u16::from_le_bytes(rec0[cur + 32..cur + 34].try_into().unwrap()) as usize;
                bitmap_runs = parse_data_run_stream(&rec0[cur + run_offset..cur + attr_len]);
                bitmap_real_size = u64::from_le_bytes(rec0[cur + 48..cur + 56].try_into().unwrap());
            }

            cur += attr_len;
        }

        Ok((data_runs, bitmap_runs, bitmap_real_size))
    }

    fn read_bitmap(&self, bmp_runs: &[(u64, u64)], real_size: u64) -> AppResult<Vec<u8>> {
        if bmp_runs.is_empty() || real_size == 0 {
            return Ok(Vec::new());
        }

        let mut bitmap_buf = vec![0u8; real_size as usize];
        let mut cur_bitmap_offset = 0usize;

        // Sort bitmap runs by LCN to prevent random head movements
        let mut sorted_runs = bmp_runs.to_vec();
        sorted_runs.sort_unstable_by_key(|&(lcn, _)| lcn);

        for (lcn, cluster_count) in sorted_runs {
            if cur_bitmap_offset >= bitmap_buf.len() {
                break;
            }

            let run_bytes = (cluster_count * self.bytes_per_cluster) as usize;
            let disk_offset = lcn * self.bytes_per_cluster;
            let to_read = run_bytes.min(bitmap_buf.len() - cur_bitmap_offset);

            unsafe {
                let mut new_pos = 0i64;
                SetFilePointerEx(self.safe_handle.handle(), disk_offset as i64, &mut new_pos, FILE_BEGIN);
            }

            let mut bytes_read = 0u32;
            let ok = unsafe {
                ReadFile(
                    self.safe_handle.handle(),
                    bitmap_buf[cur_bitmap_offset..cur_bitmap_offset + to_read].as_mut_ptr() as *mut _,
                    to_read as u32,
                    &mut bytes_read,
                    std::ptr::null_mut(),
                )
            };

            if ok == 0 || bytes_read == 0 {
                break;
            }

            cur_bitmap_offset += bytes_read as usize;
        }

        Ok(bitmap_buf)
    }
}

fn parse_data_run_stream(data: &[u8]) -> Vec<(u64, u64)> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_data_run_stream() {
        // Run 1: len = 2 bytes (0x20 = 32 clusters), offset = 3 bytes (0x0186A0 = 100,000)
        // Header: (3 << 4) | 2 = 0x32
        let mut data = vec![0x32, 0x20, 0x00, 0xA0, 0x86, 0x01];
        // Run 2: len = 1 byte (0x10 = 16 clusters), offset = 2 bytes (0x0064 = +100 -> LCN 100,100)
        // Header: (2 << 4) | 1 = 0x21
        data.extend_from_slice(&[0x21, 0x10, 0x64, 0x00]);
        // End marker: 0x00
        data.push(0x00);

        let runs = parse_data_run_stream(&data);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0], (100_000, 32));
        assert_eq!(runs[1], (100_100, 16));
    }
}
