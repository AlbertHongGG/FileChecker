use super::bitmap::MftBitmap;
use super::record_parser::{apply_usa_fixup, MftRecordFragment, MftRecordParser};
use crate::error::{AppError, AppResult};
use crate::platform::windows::win32_fs::SafeHandle;
use rayon::prelude::*;
use rustc_hash::FxHashSet;
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
struct ActiveMicroBlock {
    disk_offset: u64,
    byte_len: usize,
    start_record: u64,
    record_count: u64,
}

#[derive(Debug, Clone)]
pub struct MftRecordSlice {
    pub slice_offset_in_buf: usize,
    pub slice_len: usize,
    pub start_record: u64,
    pub record_count: u64,
}

#[derive(Debug, Clone)]
pub struct MftReadBatch {
    pub disk_offset: u64,
    pub total_bytes: usize,
    pub active_slices: Vec<MftRecordSlice>,
}

struct ReadChunk {
    batch_idx: usize,
    buf: Vec<u8>,
    slices: Vec<MftRecordSlice>,
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
    pub is_hdd: bool,
}

impl NtfsVolumeReader {
    pub fn open(drive_letter: char, is_hdd: bool) -> AppResult<Self> {
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
            is_hdd,
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

    /// Read any file record by record index using FSCTL_GET_NTFS_FILE_RECORD and apply USA fixup.
    pub fn fetch_file_record(&self, frn: u64) -> AppResult<[u8; 1024]> {
        let mut in_frn = frn;
        let mut out_rec_buf = vec![0u8; 4096];
        let mut bytes_returned = 0u32;

        let ok = unsafe {
            DeviceIoControl(
                self.safe_handle.handle(),
                FSCTL_GET_NTFS_FILE_RECORD,
                &mut in_frn as *mut _ as *mut _,
                std::mem::size_of::<u64>() as u32,
                out_rec_buf.as_mut_ptr() as *mut _,
                out_rec_buf.len() as u32,
                &mut bytes_returned,
                std::ptr::null_mut(),
            )
        };

        if ok == 0 || bytes_returned < 1036 {
            return Err(AppError::TaskExecution(format!(
                "Failed to fetch NTFS File Record {} on drive {}:",
                frn, self.drive_letter
            )));
        }

        let mut rec = [0u8; 1024];
        rec.copy_from_slice(&out_rec_buf[12..1036]);
        if !apply_usa_fixup(&mut rec) {
            return Err(AppError::TaskExecution(format!(
                "USA fixup failed for File Record {} on drive {}:",
                frn, self.drive_letter
            )));
        }

        Ok(rec)
    }

    /// Read aligned data runs into memory (used for non-resident $BITMAP or $ATTRIBUTE_LIST).
    fn read_aligned_runs(&self, runs: &[(u64, u64)], real_size: u64) -> AppResult<Vec<u8>> {
        if runs.is_empty() || real_size == 0 {
            return Ok(Vec::new());
        }

        struct IndexedRun {
            lcn: u64,
            byte_len: usize,
            logical_offset: usize,
        }

        let mut total_alloc_bytes = 0usize;
        let mut indexed = Vec::with_capacity(runs.len());
        for &(lcn, count) in runs {
            let byte_len = (count * self.bytes_per_cluster) as usize;
            indexed.push(IndexedRun {
                lcn,
                byte_len,
                logical_offset: total_alloc_bytes,
            });
            total_alloc_bytes += byte_len;
        }

        let mut buf = vec![0u8; total_alloc_bytes];
        indexed.sort_unstable_by_key(|r| r.lcn);

        for r in indexed {
            if r.logical_offset >= real_size as usize {
                continue;
            }

            let disk_offset = r.lcn * self.bytes_per_cluster;
            unsafe {
                let mut new_pos = 0i64;
                SetFilePointerEx(
                    self.safe_handle.handle(),
                    disk_offset as i64,
                    &mut new_pos,
                    FILE_BEGIN,
                );
            }

            let mut bytes_read = 0u32;
            let ok = unsafe {
                ReadFile(
                    self.safe_handle.handle(),
                    buf[r.logical_offset..r.logical_offset + r.byte_len].as_mut_ptr() as *mut _,
                    r.byte_len as u32,
                    &mut bytes_read,
                    std::ptr::null_mut(),
                )
            };

            if ok == 0 || bytes_read as usize != r.byte_len {
                return Err(AppError::TaskExecution(format!(
                    "Failed to read aligned run on drive {}: at offset {} (read {} of {} bytes)",
                    self.drive_letter, disk_offset, bytes_read, r.byte_len
                )));
            }
        }

        buf.truncate(real_size as usize);
        Ok(buf)
    }

    /// Complete resolver for Record 0 and its attributes (including $ATTRIBUTE_LIST and Extension Records).
    /// Extracts all $DATA runs, retrieves the $BITMAP stream, and aligns max_records to cluster boundaries.
    fn resolve_mft_metadata(&mut self) -> AppResult<(Vec<DataRun>, MftBitmap)> {
        // 1. Fetch Record 0 ($MFT)
        let rec0 = self.fetch_file_record(0)?;

        let mut all_data_runs: Vec<(u64, Vec<(u64, u64)>)> = Vec::new(); // (starting_vcn, runs)
        let mut bitmap_runs: Vec<(u64, u64)> = Vec::new();
        let mut bitmap_real_size = 0u64;
        let mut ext_records_to_fetch: FxHashSet<u64> = FxHashSet::default();

        // 2. Scan Record 0 attributes
        let first_attr_offset = u16::from_le_bytes(rec0[20..22].try_into().unwrap()) as usize;
        let mut cur = first_attr_offset;

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
                // Non-resident $DATA in Record 0
                if cur + 34 <= rec0.len() {
                    let starting_vcn =
                        u64::from_le_bytes(rec0[cur + 16..cur + 24].try_into().unwrap());
                    let run_offset =
                        u16::from_le_bytes(rec0[cur + 32..cur + 34].try_into().unwrap()) as usize;
                    if cur + run_offset <= cur + attr_len {
                        let runs = parse_data_run_stream(&rec0[cur + run_offset..cur + attr_len]);
                        all_data_runs.push((starting_vcn, runs));
                    }
                }
            } else if attr_type == 0xB0 && non_resident == 1 {
                // Non-resident $BITMAP in Record 0
                if cur + 56 <= rec0.len() {
                    let run_offset =
                        u16::from_le_bytes(rec0[cur + 32..cur + 34].try_into().unwrap()) as usize;
                    let real_size =
                        u64::from_le_bytes(rec0[cur + 48..cur + 56].try_into().unwrap());
                    if cur + run_offset <= cur + attr_len {
                        bitmap_runs = parse_data_run_stream(&rec0[cur + run_offset..cur + attr_len]);
                        bitmap_real_size = real_size;
                    }
                }
            } else if attr_type == 0x20 {
                // $ATTRIBUTE_LIST in Record 0
                let list_data = if non_resident == 0 {
                    let val_len =
                        u32::from_le_bytes(rec0[cur + 16..cur + 20].try_into().unwrap()) as usize;
                    let val_offset =
                        u16::from_le_bytes(rec0[cur + 20..cur + 22].try_into().unwrap()) as usize;
                    if cur + val_offset + val_len <= rec0.len() {
                        rec0[cur + val_offset..cur + val_offset + val_len].to_vec()
                    } else {
                        Vec::new()
                    }
                } else if cur + 56 <= rec0.len() {
                    let run_offset =
                        u16::from_le_bytes(rec0[cur + 32..cur + 34].try_into().unwrap()) as usize;
                    let real_size =
                        u64::from_le_bytes(rec0[cur + 48..cur + 56].try_into().unwrap());
                    if cur + run_offset <= cur + attr_len {
                        let runs = parse_data_run_stream(&rec0[cur + run_offset..cur + attr_len]);
                        self.read_aligned_runs(&runs, real_size).unwrap_or_default()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                // Parse entries in list_data
                let mut p = 0usize;
                while p + 26 <= list_data.len() {
                    let entry_type =
                        u32::from_le_bytes(list_data[p..p + 4].try_into().unwrap());
                    let entry_len =
                        u16::from_le_bytes(list_data[p + 4..p + 6].try_into().unwrap()) as usize;
                    if entry_len == 0 || p + entry_len > list_data.len() {
                        break;
                    }
                    let base_ref =
                        u64::from_le_bytes(list_data[p + 16..p + 24].try_into().unwrap())
                            & 0x0000_FFFF_FFFF_FFFF;

                    if base_ref != 0 && (entry_type == 0x80 || entry_type == 0xB0) {
                        ext_records_to_fetch.insert(base_ref);
                    }
                    p += entry_len;
                }
            }

            cur += attr_len;
        }

        // 3. Fetch all referenced extension records to retrieve overflow $DATA and $BITMAP
        for ext_frn in ext_records_to_fetch {
            if let Ok(ext_rec) = self.fetch_file_record(ext_frn) {
                let first_attr =
                    u16::from_le_bytes(ext_rec[20..22].try_into().unwrap()) as usize;
                let mut e_cur = first_attr;

                while e_cur + 8 <= ext_rec.len() {
                    let attr_type =
                        u32::from_le_bytes(ext_rec[e_cur..e_cur + 4].try_into().unwrap());
                    if attr_type == 0xFFFF_FFFF || attr_type == 0 {
                        break;
                    }
                    let attr_len =
                        u32::from_le_bytes(ext_rec[e_cur + 4..e_cur + 8].try_into().unwrap())
                            as usize;
                    if attr_len == 0 || e_cur + attr_len > ext_rec.len() {
                        break;
                    }

                    let non_resident = ext_rec[e_cur + 8];

                    if attr_type == 0x80 && non_resident == 1 && e_cur + 34 <= ext_rec.len() {
                        let starting_vcn =
                            u64::from_le_bytes(ext_rec[e_cur + 16..e_cur + 24].try_into().unwrap());
                        let run_offset = u16::from_le_bytes(
                            ext_rec[e_cur + 32..e_cur + 34].try_into().unwrap(),
                        ) as usize;
                        if e_cur + run_offset <= e_cur + attr_len {
                            let runs = parse_data_run_stream(
                                &ext_rec[e_cur + run_offset..e_cur + attr_len],
                            );
                            all_data_runs.push((starting_vcn, runs));
                        }
                    } else if attr_type == 0xB0 && non_resident == 1 && e_cur + 56 <= ext_rec.len()
                    {
                        let run_offset = u16::from_le_bytes(
                            ext_rec[e_cur + 32..e_cur + 34].try_into().unwrap(),
                        ) as usize;
                        let real_size =
                            u64::from_le_bytes(ext_rec[e_cur + 48..e_cur + 56].try_into().unwrap());
                        if e_cur + run_offset <= e_cur + attr_len {
                            bitmap_runs = parse_data_run_stream(
                                &ext_rec[e_cur + run_offset..e_cur + attr_len],
                            );
                            bitmap_real_size = real_size;
                        }
                    }

                    e_cur += attr_len;
                }
            }
        }

        // 4. Sort and flatten $DATA runs by starting_vcn
        all_data_runs.sort_unstable_by_key(|(vcn, _)| *vcn);
        let raw_data_runs: Vec<(u64, u64)> = all_data_runs
            .into_iter()
            .flat_map(|(_, runs)| runs)
            .collect();

        if raw_data_runs.is_empty() {
            return Err(AppError::TaskExecution(format!(
                "No MFT data runs found on drive {}:",
                self.drive_letter
            )));
        }

        // 5. Read $BITMAP (if available) into MftBitmap
        let bitmap = if !bitmap_runs.is_empty() && bitmap_real_size > 0 {
            match self.read_aligned_runs(&bitmap_runs, bitmap_real_size) {
                Ok(bytes) => MftBitmap::new(bytes),
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to read MFT bitmap on drive {}: {:?}. Proceeding with safe fallback.",
                        self.drive_letter, e
                    );
                    MftBitmap::default()
                }
            }
        } else {
            MftBitmap::default()
        };

        // 6. Calculate cluster-aligned max_records (prevents unaligned ReadFile Error 87)
        let records_per_cluster = (self.bytes_per_cluster / 1024).max(1);
        self.max_records = bitmap.aligned_max_records(self.max_records, records_per_cluster);

        // 7. Convert raw runs into indexed DataRuns
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

        Ok((indexed_runs, bitmap))
    }

    /// Read active MFT fragments using 64KB micro-blocks, elevator scheduling,
    /// gap bridging, and a double-buffered concurrent reading pipeline.
    pub fn read_all_fragments<F>(
        &mut self,
        mut on_progress: F,
        is_cancelled: impl Fn() -> bool + Sync + Send,
    ) -> AppResult<Vec<MftRecordFragment>>
    where
        F: FnMut(usize, usize) + Send,
    {
        let (indexed_runs, bitmap) = self.resolve_mft_metadata()?;

        // Plan batches with 64KB micro-blocks and elevator sorting
        const MICRO_BLOCK_RECORDS: u64 = 64; // 64 records = 64 KB = 16 clusters of 4KB
        let mut active_micro_blocks: Vec<ActiveMicroBlock> = Vec::new();

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
                    .min(MICRO_BLOCK_RECORDS)
                    .min(self.max_records - seg_start_rec);

                if bitmap.is_range_active(seg_start_rec, seg_rec_count) {
                    let disk_offset =
                        (run.lcn * self.bytes_per_cluster) + (run_record_offset * 1024);
                    active_micro_blocks.push(ActiveMicroBlock {
                        disk_offset,
                        byte_len: (seg_rec_count * 1024) as usize,
                        start_record: seg_start_rec,
                        record_count: seg_rec_count,
                    });
                }

                run_record_offset += seg_rec_count;
            }
        }

        if active_micro_blocks.is_empty() {
            return Err(AppError::TaskExecution(format!(
                "MFT read on drive {}: returned 0 active micro-blocks",
                self.drive_letter
            )));
        }

        // Elevator sorting by physical LCN disk offset
        active_micro_blocks.sort_unstable_by_key(|b| b.disk_offset);

        // Coalesce into MftReadBatch (up to 4MB, bridging small gaps <= 128KB on HDD to prevent rotational delay)
        const MAX_BATCH_BYTES: usize = 4 * 1024 * 1024;
        let max_gap_to_bridge = if self.is_hdd { 128 * 1024 } else { 0 };

        let mut batches: Vec<MftReadBatch> = Vec::new();
        for block in active_micro_blocks {
            let can_coalesce = if let Some(last) = batches.last_mut() {
                let last_end = last.disk_offset + last.total_bytes as u64;
                if block.disk_offset >= last_end {
                    let gap = (block.disk_offset - last_end) as usize;
                    if gap <= max_gap_to_bridge
                        && last.total_bytes + gap + block.byte_len <= MAX_BATCH_BYTES
                    {
                        let slice_offset = last.total_bytes + gap;
                        last.total_bytes = slice_offset + block.byte_len;
                        last.active_slices.push(MftRecordSlice {
                            slice_offset_in_buf: slice_offset,
                            slice_len: block.byte_len,
                            start_record: block.start_record,
                            record_count: block.record_count,
                        });
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            if !can_coalesce {
                batches.push(MftReadBatch {
                    disk_offset: block.disk_offset,
                    total_bytes: block.byte_len,
                    active_slices: vec![MftRecordSlice {
                        slice_offset_in_buf: 0,
                        slice_len: block.byte_len,
                        start_record: block.start_record,
                        record_count: block.record_count,
                    }],
                });
            }
        }

        let total_batches = batches.len();
        if total_batches == 0 {
            return Err(AppError::TaskExecution(format!(
                "MFT read on drive {}: 0 batches planned",
                self.drive_letter
            )));
        }

        // Concurrent Double-Buffered Pipeline: Thread 1 (Disk Read) -> Channel -> Thread 2 (Rayon Parse)
        let (tx, rx) = std::sync::mpsc::sync_channel::<AppResult<ReadChunk>>(2);
        let safe_handle = &self.safe_handle;
        let drive_letter = self.drive_letter;

        let mut records_parsed = Vec::with_capacity(self.max_records.min(2_000_000) as usize);

        std::thread::scope(|s| {
            // Producer Thread: I/O Read stream
            let is_cancelled_ref = &is_cancelled;
            let batches_ref = &batches;

            s.spawn(move || {
                for (b_idx, batch) in batches_ref.iter().enumerate() {
                    if is_cancelled_ref() {
                        let _ = tx.send(Err(AppError::ScanCancelled));
                        break;
                    }

                    let mut buf = vec![0u8; batch.total_bytes];
                    unsafe {
                        let mut new_pos = 0i64;
                        SetFilePointerEx(
                            safe_handle.handle(),
                            batch.disk_offset as i64,
                            &mut new_pos,
                            FILE_BEGIN,
                        );
                    }

                    let mut bytes_read = 0u32;
                    let ok = unsafe {
                        ReadFile(
                            safe_handle.handle(),
                            buf.as_mut_ptr() as *mut _,
                            batch.total_bytes as u32,
                            &mut bytes_read,
                            std::ptr::null_mut(),
                        )
                    };

                    if ok == 0 || bytes_read as usize != batch.total_bytes {
                        let _ = tx.send(Err(AppError::TaskExecution(format!(
                            "Failed to read MFT batch on drive {}: at offset {} (expected {}, read {})",
                            drive_letter, batch.disk_offset, batch.total_bytes, bytes_read
                        ))));
                        break;
                    }

                    if tx
                        .send(Ok(ReadChunk {
                            batch_idx: b_idx,
                            buf,
                            slices: batch.active_slices.clone(),
                        }))
                        .is_err()
                    {
                        break;
                    }
                }
            });

            // Consumer Thread: CPU Rayon parsing with O(1) bit test
            while let Ok(res) = rx.recv() {
                let chunk = res?;
                if is_cancelled() {
                    return Err(AppError::ScanCancelled);
                }

                for slice in &chunk.slices {
                    if slice.slice_offset_in_buf + slice.slice_len > chunk.buf.len() {
                        continue;
                    }

                    let slice_bytes = &chunk.buf
                        [slice.slice_offset_in_buf..slice.slice_offset_in_buf + slice.slice_len];
                    let start_rec = slice.start_record;

                    let parsed_chunk: Vec<MftRecordFragment> = slice_bytes
                        .par_chunks_exact(1024)
                        .enumerate()
                        .filter_map(|(idx, rec)| {
                            let rec_id = start_rec + idx as u64;
                            if !bitmap.is_record_active(rec_id) {
                                return None;
                            }
                            MftRecordParser::parse_record_fragment(rec, rec_id)
                        })
                        .collect();

                    records_parsed.extend(parsed_chunk);
                }

                on_progress(chunk.batch_idx + 1, total_batches);
            }

            Ok(())
        })?;

        if records_parsed.is_empty() {
            return Err(AppError::TaskExecution(format!(
                "MFT read on drive {}: returned 0 valid records",
                self.drive_letter
            )));
        }

        Ok(records_parsed)
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
        let mut data = vec![0x32, 0x20, 0x00, 0xA0, 0x86, 0x01];
        data.extend_from_slice(&[0x21, 0x10, 0x64, 0x00]);
        data.push(0x00);

        let runs = parse_data_run_stream(&data);
        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0], (100_000, 32));
        assert_eq!(runs[1], (100_100, 16));
    }
}
