/// Represents the NTFS $MFT $BITMAP attribute stream in memory.
/// Provides safe invariants:
/// 1. If bitmap is missing or empty, gracefully reports ranges as active (never falsely rejects records).
/// 2. Ensures cluster-boundary alignment for max record bounds to guarantee 4096-byte aligned I/O.
/// 3. Provides O(1) bit-level check for record pre-filtering.
#[derive(Debug, Clone, Default)]
pub struct MftBitmap {
    bytes: Vec<u8>,
}

impl MftBitmap {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    #[inline]
    pub fn len_bytes(&self) -> usize {
        self.bytes.len()
    }

    /// Calculate the aligned maximum record count based on the highest active record in the bitmap.
    /// Aligns UP to `records_per_cluster` (typically 4 for 4KB cluster and 1024-byte MFT record).
    /// Guarantees that total read size is always an exact multiple of the cluster size.
    pub fn aligned_max_records(&self, volume_max: u64, records_per_cluster: u64) -> u64 {
        let rpc = records_per_cluster.max(1);

        if self.bytes.is_empty() {
            // Align volume_max to cluster boundary
            return ((volume_max + rpc - 1) / rpc) * rpc;
        }

        // Search backwards for the highest active bit
        let mut highest_active_record = None;
        for (byte_idx, &byte_val) in self.bytes.iter().enumerate().rev() {
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
            let raw_max = har + 1;
            let aligned = ((raw_max + rpc - 1) / rpc) * rpc;
            aligned.min(volume_max)
        } else {
            // Bitmap is all zeros or empty, fall back to cluster-aligned volume_max
            ((volume_max + rpc - 1) / rpc) * rpc
        }
    }

    /// Check if a range of records contains any active bit.
    /// Invariant: If bitmap is empty or out of range, returns `true` (safety-first).
    #[inline]
    pub fn is_range_active(&self, start_rec: u64, count: u64) -> bool {
        if self.bytes.is_empty() {
            return true;
        }

        let start_byte = (start_rec / 8) as usize;
        let end_byte = (((start_rec + count + 7) / 8) as usize).min(self.bytes.len());

        if start_byte >= self.bytes.len() {
            return true;
        }

        self.bytes[start_byte..end_byte].iter().any(|&b| b != 0)
    }

    /// Fast O(1) bit test to check if an individual record is active.
    /// Invariant: If bitmap is empty or record is beyond bitmap, returns `true`.
    #[inline]
    pub fn is_record_active(&self, rec_id: u64) -> bool {
        if self.bytes.is_empty() {
            return true;
        }

        let byte_idx = (rec_id / 8) as usize;
        if byte_idx >= self.bytes.len() {
            return true;
        }

        (self.bytes[byte_idx] & (1 << (rec_id % 8))) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_bitmap_invariants() {
        let bmp = MftBitmap::new(Vec::new());
        assert!(bmp.is_empty());
        assert!(bmp.is_range_active(0, 100));
        assert!(bmp.is_record_active(50));
        // Aligned up to cluster boundary 4
        assert_eq!(bmp.aligned_max_records(1001, 4), 1004);
        assert_eq!(bmp.aligned_max_records(1004, 4), 1004);
    }

    #[test]
    fn test_aligned_max_records() {
        // Bitmap with bit 13 set (record 13)
        // byte 0: 0, byte 1: bit 5 set (8 + 5 = 13)
        let mut bytes = vec![0u8; 10];
        bytes[1] = 1 << 5; // record 13 active
        let bmp = MftBitmap::new(bytes);

        // har = 13 -> raw_max = 14
        // records_per_cluster = 4 -> aligned to 16
        assert_eq!(bmp.aligned_max_records(1000, 4), 16);
    }

    #[test]
    fn test_is_range_active_and_record_active() {
        let mut bytes = vec![0u8; 16]; // 128 records
        bytes[2] = 0b0000_0100; // record 18 active (2 * 8 + 2)

        let bmp = MftBitmap::new(bytes);

        // Range [0..16] (bytes 0..1) -> no active bits
        assert!(!bmp.is_range_active(0, 16));

        // Range [16..24] (byte 2) -> has bit 18
        assert!(bmp.is_range_active(16, 8));

        // Range [32..64] -> all zeros
        assert!(!bmp.is_range_active(32, 32));

        // Individual record checks
        assert!(!bmp.is_record_active(17));
        assert!(bmp.is_record_active(18));
        assert!(!bmp.is_record_active(19));

        // Beyond bitmap bounds -> returns true (safe invariant)
        assert!(bmp.is_record_active(200));
        assert!(bmp.is_range_active(200, 50));
    }
}
