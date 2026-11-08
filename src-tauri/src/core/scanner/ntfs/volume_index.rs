use super::record_parser::{MftNode, MftRecordFragment};
use super::volume_reader::VolumeUsnState;
use crate::core::scanner::{ScanObserver, ScanPolicy, ScanSnapshot};
use crate::error::{AppError, AppResult};
use crate::models::FileMetadata;
use rustc_hash::FxHashMap;

pub struct MftVolumeIndex {
    pub drive_letter: char,
    pub usn_state: Option<VolumeUsnState>,
    pub parent_to_children: FxHashMap<u64, Vec<u32>>,
    pub nodes: Vec<MftNode>,
}

impl MftVolumeIndex {
    pub fn new(
        drive_letter: char,
        usn_state: Option<VolumeUsnState>,
        fragments: Vec<MftRecordFragment>,
    ) -> Self {
        let mut base_fragments: Vec<MftRecordFragment> = Vec::with_capacity(fragments.len());
        let mut extensions: FxHashMap<u64, Vec<MftRecordFragment>> = FxHashMap::default();

        for frag in fragments {
            if frag.is_extension {
                extensions.entry(frag.record_id).or_default().push(frag);
            } else {
                base_fragments.push(frag);
            }
        }

        let mut nodes = Vec::with_capacity(base_fragments.len());
        let mut parent_to_children: FxHashMap<u64, Vec<u32>> = FxHashMap::default();
        parent_to_children.reserve(base_fragments.len() / 2);

        for mut base in base_fragments {
            if let Some(ext_list) = extensions.remove(&base.record_id) {
                for ext in ext_list {
                    base.merge_from(ext);
                }
            }

            if let Some(node) = base.into_node() {
                let idx = nodes.len() as u32;
                parent_to_children
                    .entry(node.parent_id)
                    .or_default()
                    .push(idx);
                nodes.push(node);
            }
        }

        Self {
            drive_letter,
            usn_state,
            parent_to_children,
            nodes,
        }
    }

    /// Extract a subtree rooted at `target_frn` applying `policy` prune rules in O(1) per node.
    pub fn extract_subtree(
        &self,
        target_frn: u64,
        policy: &ScanPolicy,
        observer: &ScanObserver,
    ) -> AppResult<ScanSnapshot> {
        let mut snapshot = FxHashMap::default();
        let mut stack: Vec<(u64, String)> = vec![(target_frn, String::new())];
        let mut file_count = 0usize;

        observer.set_current_name("Extracting subtree...");

        while let Some((parent_id, prefix)) = stack.pop() {
            if observer.is_cancelled() {
                return Err(AppError::ScanCancelled);
            }

            if let Some(children_indices) = self.parent_to_children.get(&parent_id) {
                for &c_idx in children_indices {
                    let child = &self.nodes[c_idx as usize];
                    if child.record_id == parent_id {
                        continue; // Guard against cyclic self-reference
                    }

                    if child.is_dir {
                        if policy.should_prune(&child.name) {
                            continue;
                        }

                        let rel_path = if prefix.is_empty() {
                            child.name.to_string()
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
                            child.name.to_string()
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

    #[test]
    fn test_extract_subtree_with_pruning() {
        let frags = vec![
            MftRecordFragment {
                record_id: 10,
                is_extension: false,
                parent_id: 5,
                size: 0,
                has_data_attr: false,
                modified_timestamp: 0,
                name: Some("src".into()),
                chosen_namespace: 1,
                is_dir: true,
            },
            MftRecordFragment {
                record_id: 11,
                is_extension: false,
                parent_id: 10,
                size: 500,
                has_data_attr: true,
                modified_timestamp: 12345,
                name: Some("main.rs".into()),
                chosen_namespace: 1,
                is_dir: false,
            },
            MftRecordFragment {
                record_id: 12,
                is_extension: false,
                parent_id: 5,
                size: 0,
                has_data_attr: false,
                modified_timestamp: 0,
                name: Some("build".into()), // Should be pruned!
                chosen_namespace: 1,
                is_dir: true,
            },
            MftRecordFragment {
                record_id: 13,
                is_extension: false,
                parent_id: 12,
                size: 9999,
                has_data_attr: true,
                modified_timestamp: 12345,
                name: Some("app.apk".into()),
                chosen_namespace: 1,
                is_dir: false,
            },
            MftRecordFragment {
                record_id: 14,
                is_extension: false,
                parent_id: 5,
                size: 100,
                has_data_attr: true,
                modified_timestamp: 12345,
                name: Some("README.md".into()),
                chosen_namespace: 1,
                is_dir: false,
            },
        ];

        let index = MftVolumeIndex::new('C', None, frags);
        let policy = ScanPolicy::new(true); // prune_ignored = true
        let observer = ScanObserver::new(Arc::new(AtomicBool::new(false)));

        let snapshot = index.extract_subtree(5, &policy, &observer).expect("Extract subtree");

        assert!(snapshot.contains_key("src"));
        assert!(snapshot.contains_key("src/main.rs"));
        assert!(snapshot.contains_key("README.md"));
        // Pruned
        assert!(!snapshot.contains_key("build"));
        assert!(!snapshot.contains_key("build/app.apk"));
    }
}
