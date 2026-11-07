use crate::models::AnchorItem;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Discovers top-level anchor directories and files in <1ms without recursing.
pub fn discover_anchors(
    base_path: &Path,
    prune_set: &Option<HashSet<&'static str>>,
) -> Vec<AnchorItem> {
    let mut items = Vec::new();
    if let Ok(entries) = fs::read_dir(base_path) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

            if is_dir {
                if let Some(ref set) = prune_set {
                    if set.contains(name_str.as_ref()) {
                        continue;
                    }
                }
            }

            items.push(AnchorItem {
                name: name_str.to_string(),
                path: entry.path(),
                is_dir,
            });
        }
    }

    // Sort: directories first, then alphabetical by name
    items.sort_by(|a, b| {
        if a.is_dir != b.is_dir {
            b.is_dir.cmp(&a.is_dir)
        } else {
            a.name.cmp(&b.name)
        }
    });

    items
}
