import { TreeNode, DescendantEntry, DirectoryActionType, PlannedAction } from '../../../types';
import { getExpectedEntryAction } from './subtreePolicy';

/**
 * On-demand stack-based traversal to collect all descendant entries
 * for a directory node when user triggers a directory-level action (Mirror / Delete).
 * Runs in < 1ms on demand without bloating TreeNode memory.
 */
export function collectSubtreeDescendants(rootNode: TreeNode): DescendantEntry[] {
  const result: DescendantEntry[] = [];
  const stack: TreeNode[] = [rootNode];

  while (stack.length > 0) {
    const node = stack.pop()!;
    result.push({
      path: node.relative_path,
      diffType: node.diff_kind.type,
      isDir: node.is_dir,
    });

    if (node.children && node.children.length > 0) {
      for (let i = node.children.length - 1; i >= 0; i--) {
        stack.push(node.children[i]);
      }
    }
  }

  return result;
}

/**
 * On-demand traversal to collect all modified file paths under a directory node for diff inspection.
 */
export function collectModifiableFiles(rootNode: TreeNode): string[] {
  const result: string[] = [];
  const stack: TreeNode[] = [rootNode];

  while (stack.length > 0) {
    const node = stack.pop()!;
    if (!node.is_dir && node.diff_kind.type === 'Modified') {
      result.push(node.relative_path);
    } else if (node.children && node.children.length > 0) {
      for (let i = node.children.length - 1; i >= 0; i--) {
        stack.push(node.children[i]);
      }
    }
  }

  return result;
}

/**
 * Checks whether all eligible entries in the subtree are already active for this directory action.
 */
export function isSubtreeActionActive(
  rootNode: TreeNode,
  actionMap: Map<string, PlannedAction> | undefined,
  dirAction: DirectoryActionType
): boolean {
  if (!actionMap || dirAction === 'skip') {
    return false;
  }

  const descendants = collectSubtreeDescendants(rootNode);
  const eligible = descendants.filter((d) => d.diffType !== 'Identical');
  if (eligible.length === 0) return false;

  return eligible.every((item) => {
    const expected = getExpectedEntryAction(item.diffType, dirAction);
    return expected ? actionMap.get(item.path) === expected : true;
  });
}
