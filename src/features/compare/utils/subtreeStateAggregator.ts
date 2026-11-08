import { TreeNode, DirectoryActionType, PlannedAction } from '../../../types';
import { getExpectedEntryAction } from './subtreePolicy';

interface SubtreeActionSummary {
  eligibleCount: number;
  matchesMirrorAtoB: boolean;
  matchesMirrorBtoA: boolean;
  matchesDelete: boolean;
}

/**
 * Computes active directory action states across the entire tree in a single
 * Bottom-Up post-order traversal (O(N) time complexity).
 *
 * This completely eliminates the previous O(K * D) traversal during render in TreeActionButtons,
 * allowing each directory row to resolve its active button state via an O(1) map lookup.
 */
export function computeActiveDirectoryActions(
  roots: TreeNode[],
  actionMap: Map<string, PlannedAction>
): Map<string, DirectoryActionType> {
  const resultMap = new Map<string, DirectoryActionType>();

  // If there are no planned actions in the entire workspace, return empty map immediately in O(1)
  if (!roots || roots.length === 0 || !actionMap || actionMap.size === 0) {
    return resultMap;
  }

  function traverse(node: TreeNode): SubtreeActionSummary {
    if (!node.is_dir) {
      if (node.diff_kind.type === 'Identical') {
        return {
          eligibleCount: 0,
          matchesMirrorAtoB: true,
          matchesMirrorBtoA: true,
          matchesDelete: true,
        };
      }

      const currentAction = actionMap.get(node.relative_path);
      const expAtoB = getExpectedEntryAction(node.diff_kind.type, 'mirror_a_to_b');
      const expBtoA = getExpectedEntryAction(node.diff_kind.type, 'mirror_b_to_a');
      const expDelete = getExpectedEntryAction(node.diff_kind.type, 'delete');

      return {
        eligibleCount: 1,
        matchesMirrorAtoB: Boolean(expAtoB && currentAction === expAtoB),
        matchesMirrorBtoA: Boolean(expBtoA && currentAction === expBtoA),
        matchesDelete: Boolean(expDelete && currentAction === expDelete),
      };
    }

    // Directory node: evaluate own action if it has differences
    let eligibleCount = 0;
    let matchesMirrorAtoB = true;
    let matchesMirrorBtoA = true;
    let matchesDelete = true;

    if (node.diff_kind.type !== 'Identical') {
      eligibleCount++;
      const currentAction = actionMap.get(node.relative_path);
      const expAtoB = getExpectedEntryAction(node.diff_kind.type, 'mirror_a_to_b');
      const expBtoA = getExpectedEntryAction(node.diff_kind.type, 'mirror_b_to_a');
      const expDelete = getExpectedEntryAction(node.diff_kind.type, 'delete');

      if (!expAtoB || currentAction !== expAtoB) matchesMirrorAtoB = false;
      if (!expBtoA || currentAction !== expBtoA) matchesMirrorBtoA = false;
      if (!expDelete || currentAction !== expDelete) matchesDelete = false;
    }

    // Accumulate child subtrees
    if (node.children && node.children.length > 0) {
      for (const child of node.children) {
        const childSummary = traverse(child);
        eligibleCount += childSummary.eligibleCount;
        if (!childSummary.matchesMirrorAtoB) matchesMirrorAtoB = false;
        if (!childSummary.matchesMirrorBtoA) matchesMirrorBtoA = false;
        if (!childSummary.matchesDelete) matchesDelete = false;
      }
    }

    if (eligibleCount > 0) {
      if (matchesMirrorAtoB) {
        resultMap.set(node.relative_path, 'mirror_a_to_b');
      } else if (matchesMirrorBtoA) {
        resultMap.set(node.relative_path, 'mirror_b_to_a');
      } else if (matchesDelete) {
        resultMap.set(node.relative_path, 'delete');
      }
    }

    return {
      eligibleCount,
      matchesMirrorAtoB,
      matchesMirrorBtoA,
      matchesDelete,
    };
  }

  for (const root of roots) {
    traverse(root);
  }

  return resultMap;
}
