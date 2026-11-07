use crate::models::{DiffKind, FileMetadata, UnifiedEntry};
use rayon::prelude::*;
use rustc_hash::FxHashMap;

pub struct DiffComparator;

impl DiffComparator {
    pub fn compare(
        map_a: FxHashMap<String, FileMetadata>,
        mut map_b: FxHashMap<String, FileMetadata>,
    ) -> Vec<UnifiedEntry> {
        let mut entries = Vec::with_capacity(map_a.len() + map_b.len());

        for (rel, meta_a) in map_a {
            let meta_b = map_b.remove(&rel);

            let name = match rel.rfind('/') {
                Some(idx) => rel[idx + 1..].to_string(),
                None => rel.clone(),
            };

            let is_dir = meta_a.is_dir;

            let diff_kind = match &meta_b {
                None => DiffKind::OnlyInA,
                Some(b) => {
                    if meta_a.is_dir && b.is_dir {
                        DiffKind::Identical
                    } else if meta_a.size == b.size && meta_a.modified_timestamp == b.modified_timestamp {
                        DiffKind::Identical
                    } else {
                        DiffKind::Modified {
                            a_newer: meta_a.modified_timestamp > b.modified_timestamp,
                            b_newer: b.modified_timestamp > meta_a.modified_timestamp,
                            same_time: meta_a.modified_timestamp == b.modified_timestamp,
                        }
                    }
                }
            };

            entries.push(UnifiedEntry {
                relative_path: rel,
                name,
                is_dir,
                diff_kind,
                meta_a: Some(meta_a),
                meta_b,
            });
        }

        // Remaining elements in map_b exist only in B
        for (rel, meta_b) in map_b {
            let name = match rel.rfind('/') {
                Some(idx) => rel[idx + 1..].to_string(),
                None => rel.clone(),
            };

            let is_dir = meta_b.is_dir;

            entries.push(UnifiedEntry {
                relative_path: rel,
                name,
                is_dir,
                diff_kind: DiffKind::OnlyInB,
                meta_a: None,
                meta_b: Some(meta_b),
            });
        }

        // Parallel unstable sort in-place by relative_path
        entries.par_sort_unstable_by(|a, b| a.relative_path.cmp(&b.relative_path));
        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_files() {
        let mut a = FxHashMap::default();
        let mut b = FxHashMap::default();
        let meta = FileMetadata {
            size: 100,
            modified_timestamp: 1000,
            is_dir: false,
        };
        a.insert("file.txt".to_string(), meta.clone());
        b.insert("file.txt".to_string(), meta);

        let res = DiffComparator::compare(a, b);
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].diff_kind, DiffKind::Identical);
    }

    #[test]
    fn test_only_in_a_and_b() {
        let mut a = FxHashMap::default();
        let mut b = FxHashMap::default();
        a.insert(
            "a.txt".to_string(),
            FileMetadata {
                size: 10,
                modified_timestamp: 10,
                is_dir: false,
            },
        );
        b.insert(
            "b.txt".to_string(),
            FileMetadata {
                size: 20,
                modified_timestamp: 20,
                is_dir: false,
            },
        );

        let res = DiffComparator::compare(a, b);
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].diff_kind, DiffKind::OnlyInA);
        assert_eq!(res[1].diff_kind, DiffKind::OnlyInB);
    }
}
