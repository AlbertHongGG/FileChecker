import { UnifiedEntry, SyncTask, PlannedAction } from '../../../types';

/**
 * Applies executed SyncTasks to entries to produce an updated UnifiedEntry list
 * without triggering a full filesystem re-scan.
 */
export function applySyncTasksToEntries(
  prevEntries: UnifiedEntry[],
  tasks: SyncTask[]
): UnifiedEntry[] {
  if (tasks.length === 0 || prevEntries.length === 0) {
    return prevEntries;
  }

  const taskMap = new Map<string, PlannedAction>();
  const copiedToBParents = new Set<string>();
  const copiedToAParents = new Set<string>();

  for (const t of tasks) {
    if (t.action !== 'skip') {
      taskMap.set(t.relative_path, t.action);

      if (t.action === 'copy_a_to_b') {
        const parts = t.relative_path.split('/');
        let cur = '';
        for (let i = 0; i < parts.length - 1; i++) {
          cur = cur ? `${cur}/${parts[i]}` : parts[i];
          copiedToBParents.add(cur);
        }
      } else if (t.action === 'copy_b_to_a') {
        const parts = t.relative_path.split('/');
        let cur = '';
        for (let i = 0; i < parts.length - 1; i++) {
          cur = cur ? `${cur}/${parts[i]}` : parts[i];
          copiedToAParents.add(cur);
        }
      }
    }
  }

  const nextEntries: UnifiedEntry[] = [];

  for (const entry of prevEntries) {
    const action = taskMap.get(entry.relative_path);

    if (!action) {
      // If this is a directory and children were copied into it, update directory presence
      if (entry.is_dir) {
        let updatedMetaA = entry.meta_a;
        let updatedMetaB = entry.meta_b;
        let changed = false;

        if (copiedToAParents.has(entry.relative_path) && !updatedMetaA) {
          updatedMetaA = { size: 0, modified_timestamp: 0, is_dir: true };
          changed = true;
        }
        if (copiedToBParents.has(entry.relative_path) && !updatedMetaB) {
          updatedMetaB = { size: 0, modified_timestamp: 0, is_dir: true };
          changed = true;
        }

        if (changed) {
          nextEntries.push({
            ...entry,
            meta_a: updatedMetaA,
            meta_b: updatedMetaB,
          });
          continue;
        }
      }

      nextEntries.push(entry);
      continue;
    }

    switch (action) {
      case 'copy_a_to_b': {
        const metaA = entry.meta_a;
        nextEntries.push({
          ...entry,
          diff_kind: { type: 'Identical' },
          meta_b: metaA
            ? { ...metaA }
            : entry.is_dir
            ? { size: 0, modified_timestamp: 0, is_dir: true }
            : entry.meta_b,
        });
        break;
      }
      case 'copy_b_to_a': {
        const metaB = entry.meta_b;
        nextEntries.push({
          ...entry,
          diff_kind: { type: 'Identical' },
          meta_a: metaB
            ? { ...metaB }
            : entry.is_dir
            ? { size: 0, modified_timestamp: 0, is_dir: true }
            : entry.meta_a,
        });
        break;
      }
      case 'delete_a': {
        if (entry.meta_b) {
          nextEntries.push({
            ...entry,
            diff_kind: { type: 'OnlyInB' },
            meta_a: null,
          });
        }
        break;
      }
      case 'delete_b': {
        if (entry.meta_a) {
          nextEntries.push({
            ...entry,
            diff_kind: { type: 'OnlyInA' },
            meta_b: null,
          });
        }
        break;
      }
      case 'delete_both': {
        // Completely removed from both sides, omitted from nextEntries
        break;
      }
      default:
        nextEntries.push(entry);
        break;
    }
  }

  return nextEntries;
}
