import { PlannedAction, DescendantEntry, DirectoryActionType, UnifiedEntry } from '../../../types';

/**
 * Resolves the expected PlannedAction for an entry when executing a directory-level mirror/delete.
 *
 * Mirror A -> B:
 *   - OnlyInA / Modified -> copy_a_to_b (bring or overwrite into B)
 *   - OnlyInB -> delete_b (purge extra legacy items from B)
 *
 * Mirror B -> A:
 *   - OnlyInB / Modified -> copy_b_to_a (bring or overwrite into A)
 *   - OnlyInA -> delete_a (purge extra legacy items from A)
 *
 * Delete:
 *   - OnlyInA -> delete_a
 *   - OnlyInB -> delete_b
 *   - Modified -> delete_both
 */
export function getExpectedEntryAction(
  diffType: UnifiedEntry['diff_kind']['type'],
  dirAction: DirectoryActionType
): PlannedAction | null {
  if (diffType === 'Identical' || dirAction === 'skip') {
    return null;
  }

  if (dirAction === 'mirror_a_to_b') {
    if (diffType === 'OnlyInA' || diffType === 'Modified') {
      return 'copy_a_to_b';
    }
    if (diffType === 'OnlyInB') {
      return 'delete_b';
    }
  }

  if (dirAction === 'mirror_b_to_a') {
    if (diffType === 'OnlyInB' || diffType === 'Modified') {
      return 'copy_b_to_a';
    }
    if (diffType === 'OnlyInA') {
      return 'delete_a';
    }
  }

  if (dirAction === 'delete') {
    if (diffType === 'OnlyInA') return 'delete_a';
    if (diffType === 'OnlyInB') return 'delete_b';
    if (diffType === 'Modified') return 'delete_both';
  }

  return null;
}

/**
 * Checks whether all eligible entries in the subtree are already active for this directory action.
 */
export function isDirectoryActionActive(
  descendants: DescendantEntry[],
  actionMap: Map<string, PlannedAction> | undefined,
  dirAction: DirectoryActionType
): boolean {
  if (!actionMap || descendants.length === 0 || dirAction === 'skip') {
    return false;
  }

  const eligible = descendants.filter((d) => d.diffType !== 'Identical');
  if (eligible.length === 0) return false;

  return eligible.every((item) => {
    const expected = getExpectedEntryAction(item.diffType, dirAction);
    return expected ? actionMap.get(item.path) === expected : true;
  });
}
