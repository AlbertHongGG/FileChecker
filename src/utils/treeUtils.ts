import { UnifiedEntry, TreeNode, PlannedAction } from '../types/compare';

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

  // Bubble up diff statuses to folders
  function propagateDiffs(node: TreeNode): boolean {
    let hasChildDiff = false;
    if (node.children && node.children.length > 0) {
      for (const child of node.children) {
        if (propagateDiffs(child)) {
          hasChildDiff = true;
        }
      }
    }
    if (node.is_dir && hasChildDiff && node.diff_kind.type === 'Identical') {
      node.diff_kind = { type: 'Modified' };
    }
    return node.diff_kind.type !== 'Identical' || hasChildDiff;
  }

  roots.forEach(propagateDiffs);

  // Sort: directories first, then alphabetical
  function sortNodes(nodes: TreeNode[]) {
    nodes.sort((a, b) => {
      if (a.is_dir !== b.is_dir) {
        return a.is_dir ? -1 : 1;
      }
      return a.name.localeCompare(b.name, undefined, { numeric: true, sensitivity: 'base' });
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

export function formatFileSize(bytes?: number | null): string {
  if (bytes === undefined || bytes === null) return '-';
  if (bytes === 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

export function formatTimestamp(ts?: number | null): string {
  if (!ts) return '-';
  const date = new Date(ts);
  return date.toLocaleString(undefined, {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false
  });
}

export interface FlattenedRow {
  node: TreeNode;
  depth: number;
  hasChildren: boolean;
  isExpanded: boolean;
}

export function flattenTree(
  nodes: TreeNode[],
  expandedPaths: Set<string>,
  onlyDiffs: boolean,
  actionMap: Map<string, PlannedAction>,
  depth = 0
): FlattenedRow[] {
  const result: FlattenedRow[] = [];

  for (const node of nodes) {
    const hasDiff = node.diff_kind.type !== 'Identical' || (actionMap.get(node.relative_path) && actionMap.get(node.relative_path) !== 'skip');
    if (onlyDiffs && !hasDiff && !node.is_dir) {
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
      result.push(...flattenTree(node.children, expandedPaths, onlyDiffs, actionMap, depth + 1));
    }
  }

  return result;
}
