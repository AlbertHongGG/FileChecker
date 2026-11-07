#[derive(Debug, Clone)]
pub struct MftNode {
    pub record_id: u64,
    pub parent_id: u64,
    pub size: u64,
    pub modified_timestamp: u64,
    pub name: Box<str>,
    pub is_dir: bool,
}

pub struct MftRecordParser;

impl MftRecordParser {
    #[inline]
    pub fn parse_record(record_bytes: &[u8], record_id: u64) -> Option<MftNode> {
        if record_bytes.len() < 1024 || &record_bytes[0..4] != b"FILE" {
            return None;
        }

        // Fast rejection: In-use flag check before copying or fixup
        let flags = u16::from_le_bytes(record_bytes[22..24].try_into().unwrap());
        if (flags & 0x0001) == 0 {
            return None; // Inactive or deleted record
        }
        let is_dir = (flags & 0x0002) != 0;

        // Apply USA Fixup onto stack buffer
        let mut rec = [0u8; 1024];
        rec.copy_from_slice(&record_bytes[..1024]);

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

        let first_attr_offset = u16::from_le_bytes(rec[20..22].try_into().unwrap()) as usize;
        let mut cur = first_attr_offset;
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
                            (2, _) => true, // Replace DOS 8.3 name with real name
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

                            // Fallback size from $FILE_NAME if $DATA not encountered yet
                            if file_size == 0 && !is_dir {
                                file_size = u64::from_le_bytes(
                                    rec[val_start + 48..val_start + 56].try_into().unwrap(),
                                );
                            }
                        }
                    }
                }
            } else if attr_type == 0x80 && !is_dir {
                // $DATA: extract file size
                let name_len = rec[cur + 9];
                if name_len == 0 {
                    // Default unnamed stream
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

        Some(MftNode {
            record_id,
            parent_id,
            size: if is_dir { 0 } else { file_size },
            modified_timestamp,
            name: name.into_boxed_str(),
            is_dir,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_invalid_or_inactive_record() {
        let empty = [0u8; 1024];
        assert!(MftRecordParser::parse_record(&empty, 100).is_none());

        let mut inactive = [0u8; 1024];
        inactive[0..4].copy_from_slice(b"FILE");
        inactive[22..24].copy_from_slice(&0u16.to_le_bytes()); // Inactive
        assert!(MftRecordParser::parse_record(&inactive, 100).is_none());
    }

    #[test]
    fn test_parse_active_record_with_filename() {
        let mut rec = [0u8; 1024];
        rec[0..4].copy_from_slice(b"FILE");
        rec[4..6].copy_from_slice(&48u16.to_le_bytes()); // USA offset 48
        rec[6..8].copy_from_slice(&3u16.to_le_bytes()); // USA size 3 (1 update_seq + 2 sectors)
        rec[20..22].copy_from_slice(&56u16.to_le_bytes()); // first attr offset = 56
        rec[22..24].copy_from_slice(&1u16.to_le_bytes()); // flags = in-use file

        // USA table
        rec[48..50].copy_from_slice(&0x4242u16.to_le_bytes()); // sequence number
        rec[50..52].copy_from_slice(&[0x11, 0x22]); // sector 1 original end
        rec[52..54].copy_from_slice(&[0x33, 0x44]); // sector 2 original end

        // Sector ends with update_seq
        rec[510..512].copy_from_slice(&0x4242u16.to_le_bytes());
        rec[1022..1024].copy_from_slice(&0x4242u16.to_le_bytes());

        // Attr $FILE_NAME (0x30) at offset 56
        let attr_start = 56;
        rec[attr_start..attr_start + 4].copy_from_slice(&0x30u32.to_le_bytes()); // Type 0x30
        let name_utf16: Vec<u16> = "test.txt".encode_utf16().collect();
        let attr_len = 24 + 66 + (name_utf16.len() * 2);
        rec[attr_start + 4..attr_start + 8].copy_from_slice(&(attr_len as u32).to_le_bytes());
        rec[attr_start + 8] = 0; // Resident
        rec[attr_start + 20..attr_start + 22].copy_from_slice(&24u16.to_le_bytes()); // Value offset 24

        let val_start = attr_start + 24;
        rec[val_start..val_start + 8].copy_from_slice(&5u64.to_le_bytes()); // Parent FRN = 5
        rec[val_start + 48..val_start + 56].copy_from_slice(&1024u64.to_le_bytes()); // File size = 1024
        rec[val_start + 64] = name_utf16.len() as u8;
        rec[val_start + 65] = 1; // Win32 namespace

        for (i, &ch) in name_utf16.iter().enumerate() {
            let b = ch.to_le_bytes();
            rec[val_start + 66 + i * 2] = b[0];
            rec[val_start + 66 + i * 2 + 1] = b[1];
        }

        // End attribute marker
        let end_marker = attr_start + attr_len;
        rec[end_marker..end_marker + 4].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());

        let node = MftRecordParser::parse_record(&rec, 42).expect("Must parse valid node");
        assert_eq!(node.record_id, 42);
        assert_eq!(node.parent_id, 5);
        assert_eq!(&*node.name, "test.txt");
        assert_eq!(node.size, 1024);
        assert!(!node.is_dir);
    }
}
