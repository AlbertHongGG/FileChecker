use crate::models::{DiffKind, FileMetadata, UnifiedEntry};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

pub struct DiffComparator;

impl DiffComparator {
    pub fn compare(
        map_a: HashMap<String, FileMetadata>,
        map_b: HashMap<String, FileMetadata>,
    ) -> Vec<UnifiedEntry> {
        let mut all_paths: BTreeSet<String> = BTreeSet::new();
        for key in map_a.keys() {
            all_paths.insert(key.clone());
        }
        for key in map_b.keys() {
            all_paths.insert(key.clone());
        }

        let mut entries = Vec::with_capacity(all_paths.len());

        for rel in all_paths {
            let meta_a = map_a.get(&rel).cloned();
            let meta_b = map_b.get(&rel).cloned();

            let name = Path::new(&rel)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| rel.clone());

            let is_dir = meta_a
                .as_ref()
                .map(|m| m.is_dir)
                .or_else(|| meta_b.as_ref().map(|m| m.is_dir))
                .unwrap_or(false);

            let diff_kind = match (&meta_a, &meta_b) {
                (Some(_), None) => DiffKind::OnlyInA,
                (None, Some(_)) => DiffKind::OnlyInB,
                (Some(a), Some(b)) => {
                    if a.is_dir && b.is_dir {
                        DiffKind::Identical
                    } else if a.size == b.size && a.modified_timestamp == b.modified_timestamp {
                        DiffKind::Identical
                    } else {
                        DiffKind::Modified {
                            a_newer: a.modified_timestamp > b.modified_timestamp,
                            b_newer: b.modified_timestamp > a.modified_timestamp,
                            same_time: a.modified_timestamp == b.modified_timestamp,
                        }
                    }
                }
                (None, None) => DiffKind::Identical,
            };

            entries.push(UnifiedEntry {
                relative_path: rel,
                name,
                is_dir,
                diff_kind,
                meta_a,
                meta_b,
            });
        }

        entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_files() {
        let mut a = HashMap::new();
        let mut b = HashMap::new();
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
        let mut a = HashMap::new();
        let mut b = HashMap::new();
        a.insert("a.txt".to_string(), FileMetadata { size: 10, modified_timestamp: 10, is_dir: false });
        b.insert("b.txt".to_string(), FileMetadata { size: 20, modified_timestamp: 20, is_dir: false });

        let res = DiffComparator::compare(a, b);
        assert_eq!(res.len(), 2);
        assert_eq!(res[0].diff_kind, DiffKind::OnlyInA);
        assert_eq!(res[1].diff_kind, DiffKind::OnlyInB);
    }
}
