#[derive(Debug, Clone)]
pub struct MftNode {
    pub record_id: u64,
    pub parent_id: u64,
    pub size: u64,
    pub modified_timestamp: u64,
    pub name: Box<str>,
    pub is_dir: bool,
}

#[derive(Debug, Clone)]
pub struct MftRecordFragment {
    pub record_id: u64,
    pub is_extension: bool,
    pub parent_id: u64,
    pub size: u64,
    pub has_data_attr: bool,
    pub modified_timestamp: u64,
    pub name: Option<Box<str>>,
    pub chosen_namespace: u8,
    pub is_dir: bool,
}

impl MftRecordFragment {
    /// Merge an extension fragment into the base record fragment.
    pub fn merge_from(&mut self, other: MftRecordFragment) {
        if let Some(other_name) = other.name {
            let should_replace = match (self.name.is_none(), self.chosen_namespace, other.chosen_namespace) {
                (true, _, _) => true,
                (false, 255, _) => true,
                (false, 2, _) => true, // Replace DOS 8.3 short name with Win32/POSIX name
                (false, _, 1 | 3) => true, // Prefer Win32 / Win32+DOS
                _ => false,
            };

            if should_replace {
                self.name = Some(other_name);
                self.chosen_namespace = other.chosen_namespace;
                if other.parent_id != 0 {
                    self.parent_id = other.parent_id;
                }
            }
        }

        if other.has_data_attr {
            self.size = other.size;
            self.has_data_attr = true;
        } else if self.size == 0 && other.size > 0 {
            self.size = other.size;
        }

        if other.modified_timestamp > 0 {
            self.modified_timestamp = other.modified_timestamp;
        }

        if other.is_dir {
            self.is_dir = true;
        }

        if self.parent_id == 0 && other.parent_id != 0 {
            self.parent_id = other.parent_id;
        }
    }

    /// Convert into final MftNode, filtering invalid or special directories (. and ..).
    pub fn into_node(self) -> Option<MftNode> {
        let name = self.name?;
        if name.is_empty() || &*name == "." || &*name == ".." {
            return None;
        }

        Some(MftNode {
            record_id: self.record_id,
            parent_id: self.parent_id,
            size: if self.is_dir { 0 } else { self.size },
            modified_timestamp: self.modified_timestamp,
            name,
            is_dir: self.is_dir,
        })
    }
}

pub fn apply_usa_fixup(rec: &mut [u8]) -> bool {
    if rec.len() < 1024 || &rec[0..4] != b"FILE" {
        return false;
    }

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
        true
    } else {
        false
    }
}

pub struct MftRecordParser;

impl MftRecordParser {
    #[inline]
    pub fn parse_record_fragment(record_bytes: &[u8], slot_id: u64) -> Option<MftRecordFragment> {
        if record_bytes.len() < 1024 || &record_bytes[0..4] != b"FILE" {
            return None;
        }

        // Fast rejection: check in-use flag (bit 0)
        let flags = u16::from_le_bytes(record_bytes[22..24].try_into().unwrap());
        if (flags & 0x0001) == 0 {
            return None; // Inactive or deleted record
        }
        let is_dir = (flags & 0x0002) != 0;

        // Apply USA Fixup onto stack buffer
        let mut rec = [0u8; 1024];
        rec.copy_from_slice(&record_bytes[..1024]);
        if !apply_usa_fixup(&mut rec) {
            return None;
        }

        // Check if this record is an extension record (BaseFileRecordSegment != 0)
        let base_ref = u64::from_le_bytes(rec[32..40].try_into().unwrap()) & 0x0000_FFFF_FFFF_FFFF;
        let is_extension = base_ref != 0;
        let record_id = if is_extension { base_ref } else { slot_id };

        let first_attr_offset = u16::from_le_bytes(rec[20..22].try_into().unwrap()) as usize;
        let mut cur = first_attr_offset;
        let mut modified_timestamp = 0u64;
        let mut parent_id = 0u64;
        let mut file_size = 0u64;
        let mut has_data_attr = false;
        let mut name: Option<Box<str>> = None;
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
                        // Namespace priority: Win32 (1) or Win32&DOS (3) > POSIX (0) > DOS (2)
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
                            let parsed_name = String::from_utf16_lossy(utf16_slice);
                            name = Some(parsed_name.into_boxed_str());
                            parent_id = p_frn;
                            chosen_namespace = ns;

                            // Fallback size from $FILE_NAME if $DATA not encountered yet
                            if !has_data_attr && !is_dir {
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
                        has_data_attr = true;
                    } else if cur + 56 <= 1024 {
                        file_size = u64::from_le_bytes(rec[cur + 48..cur + 56].try_into().unwrap());
                        has_data_attr = true;
                    }
                }
            }

            cur += attr_len;
        }

        Some(MftRecordFragment {
            record_id,
            is_extension,
            parent_id,
            size: file_size,
            has_data_attr,
            modified_timestamp,
            name,
            chosen_namespace,
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
        assert!(MftRecordParser::parse_record_fragment(&empty, 100).is_none());

        let mut inactive = [0u8; 1024];
        inactive[0..4].copy_from_slice(b"FILE");
        inactive[22..24].copy_from_slice(&0u16.to_le_bytes()); // Inactive
        assert!(MftRecordParser::parse_record_fragment(&inactive, 100).is_none());
    }

    #[test]
    fn test_merge_extension_record_overwrites_dos_name() {
        let mut base = MftRecordFragment {
            record_id: 100,
            is_extension: false,
            parent_id: 5,
            size: 0,
            has_data_attr: false,
            modified_timestamp: 1000,
            name: Some("SYSTEM~2.INT".into()),
            chosen_namespace: 2, // DOS 8.3
            is_dir: true,
        };

        let ext = MftRecordFragment {
            record_id: 100,
            is_extension: true,
            parent_id: 5,
            size: 0,
            has_data_attr: false,
            modified_timestamp: 2000,
            name: Some("System.Runtime.InteropServices".into()),
            chosen_namespace: 1, // Win32
            is_dir: true,
        };

        base.merge_from(ext);
        let node = base.into_node().expect("Must produce valid MftNode");
        assert_eq!(&*node.name, "System.Runtime.InteropServices");
        assert_eq!(node.record_id, 100);
        assert_eq!(node.parent_id, 5);
        assert!(node.is_dir);
    }
}
