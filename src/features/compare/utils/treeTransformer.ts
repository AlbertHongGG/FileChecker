import { UnifiedEntry, TreeNode, FlattenedRow } from '../../../types';

const NATURAL_COLLATOR = new Intl.Collator(undefined, {
  numeric: true,
  sensitivity: 'base',
});

interface SubtreeSummary {
  hasA: boolean;
  hasB: boolean;
  hasDiff: boolean;
  childCount: number;
  hasModifiable: boolean;
  modifiableCount: number;
}

function propagateDirectoryState(node: TreeNode): SubtreeSummary {
  if (!node.is_dir) {
    const hasA = Boolean(node.meta_a);
    const hasB = Boolean(node.meta_b);
    const hasDiff = node.diff_kind.type !== 'Identical';
    const hasModifiable = node.diff_kind.type === 'Modified';

    return {
      hasA,
      hasB,
      hasDiff,
      childCount: 1,
      hasModifiable,
      modifiableCount: hasModifiable ? 1 : 0,
    };
  }

  let hasA = Boolean(node.meta_a);
  let hasB = Boolean(node.meta_b);
  let hasDiff = false;
  let childCount = 0;
  let hasModifiable = false;
  let modifiableCount = 0;

  if (node.children && node.children.length > 0) {
    for (const child of node.children) {
      const summary = propagateDirectoryState(child);
      if (summary.hasA) hasA = true;
      if (summary.hasB) hasB = true;
      if (summary.hasDiff) hasDiff = true;
      if (summary.hasModifiable) hasModifiable = true;
      modifiableCount += summary.modifiableCount;
      childCount += summary.childCount;
    }
  }

  // Update directory physical presence based on descendants
  if (hasA && !node.meta_a) {
    node.meta_a = { size: 0, modified_timestamp: 0, is_dir: true };
  } else if (!hasA && node.meta_a) {
    node.meta_a = null;
  }

  if (hasB && !node.meta_b) {
    node.meta_b = { size: 0, modified_timestamp: 0, is_dir: true };
  } else if (!hasB && node.meta_b) {
    node.meta_b = null;
  }

  // Derive directory diff_kind
  if (hasA && !hasB) {
    node.diff_kind = { type: 'OnlyInA' };
  } else if (!hasA && hasB) {
    node.diff_kind = { type: 'OnlyInB' };
  } else if (hasA && hasB) {
    if (hasDiff) {
      node.diff_kind = { type: 'Modified' };
    } else {
      node.diff_kind = { type: 'Identical' };
    }
  } else {
    node.diff_kind = { type: 'Identical' };
  }

  node.hasModifiable = hasModifiable;
  node.modifiableCount = modifiableCount;

  return {
    hasA,
    hasB,
    hasDiff: hasDiff || node.diff_kind.type !== 'Identical',
    childCount,
    hasModifiable,
    modifiableCount,
  };
}

export function buildTree(entries: UnifiedEntry[]): TreeNode[] {
  const rootMap = new Map<string, TreeNode>();

  for (const entry of entries) {
    const parts = entry.relative_path.split('/');
    let currentPath = '';

    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      const prevPath = currentPath;
      currentPath = prevPath ? `${prevPath}/${part}` : part;
      const isLeaf = i === parts.length - 1;

      if (!rootMap.has(currentPath)) {
        rootMap.set(currentPath, {
          relative_path: currentPath,
          name: part,
          is_dir: isLeaf ? entry.is_dir : true,
          diff_kind: isLeaf ? entry.diff_kind : { type: 'Identical' },
          meta_a: isLeaf ? entry.meta_a : null,
          meta_b: isLeaf ? entry.meta_b : null,
          children: isLeaf && !entry.is_dir ? undefined : [],
        });
      } else if (isLeaf) {
        const existing = rootMap.get(currentPath)!;
        existing.diff_kind = entry.diff_kind;
        existing.meta_a = entry.meta_a;
        existing.meta_b = entry.meta_b;
      }
    }
  }

  const roots: TreeNode[] = [];
  for (const [path, node] of rootMap.entries()) {
    const lastSlash = path.lastIndexOf('/');
    if (lastSlash === -1) {
      roots.push(node);
    } else {
      const parentPath = path.substring(0, lastSlash);
      const parent = rootMap.get(parentPath);
      if (parent && parent.children) {
        parent.children.push(node);
      }
    }
  }

  // Bottom-up propagation of directory presence and difference status
  roots.forEach(propagateDirectoryState);

  // Fast natural sort using singleton Intl.Collator
  function sortNodes(nodes: TreeNode[]) {
    nodes.sort((a, b) => {
      if (a.is_dir !== b.is_dir) {
        return a.is_dir ? -1 : 1;
      }
      return NATURAL_COLLATOR.compare(a.name, b.name);
    });
    for (const node of nodes) {
      if (node.children) {
        sortNodes(node.children);
      }
    }
  }

  sortNodes(roots);
  return roots;
}

export function flattenTree(
  nodes: TreeNode[],
  expandedPaths: Set<string>,
  onlyDiffs: boolean
): FlattenedRow[] {
  const result: FlattenedRow[] = [];

  function traverse(currentNodes: TreeNode[], depth: number) {
    for (const node of currentNodes) {
      // In onlyDiffs mode, exclude nodes (both files and folders) that are already identical
      if (onlyDiffs && node.diff_kind.type === 'Identical') {
        continue;
      }

      const hasChildren = Boolean(node.children && node.children.length > 0);
      const isExpanded = expandedPaths.has(node.relative_path);

      result.push({
        node,
        depth,
        hasChildren,
        isExpanded,
      });

      if (hasChildren && isExpanded && node.children) {
        traverse(node.children, depth + 1);
      }
    }
  }

  traverse(nodes, 0);
  return result;
}
